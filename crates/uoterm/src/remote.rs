//! HTTP client for a running `uoterm connect` / `uoterm populate` process.

use chrono::{Duration as ChronoDuration, Utc};
use serde_json::{json, Value};
use std::path::PathBuf;
use uoterm_runtime::config::{
    data_dir, load_app_config, BEARER_PREFIX, ENV_API_TOKEN, JOURNAL_HARVEST_NAME,
};
use uoterm_runtime::error::{Result, RuntimeError};

const HTTP_TIMEOUT_SECS: u64 = 15;
const ENV_API: &str = "UOTERM_API";
const ENV_SESSION: &str = "UOTERM_SESSION";

pub fn api_base(cli_api: Option<&str>) -> String {
    let raw = cli_api
        .map(str::to_string)
        .or_else(|| std::env::var(ENV_API).ok())
        .unwrap_or_else(|| load_app_config(None).api_bind);
    normalize_base(&raw)
}

pub fn normalize_base(raw: &str) -> String {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else {
        format!("http://{trimmed}")
    }
}

pub fn session_hint(cli_session: Option<&str>) -> Option<String> {
    cli_session
        .map(str::to_string)
        .or_else(|| std::env::var(ENV_SESSION).ok())
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| RuntimeError::Network(e.to_string()))
}

fn bearer_value(token: &str) -> Option<String> {
    let token = token.trim();
    if token.is_empty() {
        None
    } else {
        Some(format!("{BEARER_PREFIX}{token}"))
    }
}

fn apply_auth(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    match std::env::var(ENV_API_TOKEN)
        .ok()
        .and_then(|token| bearer_value(&token))
    {
        Some(value) => req.header(reqwest::header::AUTHORIZATION, value),
        None => req,
    }
}

pub async fn get_json(url: &str) -> Result<Value> {
    let resp = apply_auth(client()?.get(url))
        .send()
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))?;
    if !status.is_success() {
        return Err(status_err(status.as_u16(), &body));
    }
    serde_json::from_str(&body).map_err(|e| RuntimeError::Protocol(e.to_string()))
}

pub async fn post_json(url: &str, body: &Value) -> Result<Value> {
    let resp = apply_auth(client()?.post(url).json(body))
        .send()
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))?;
    if !status.is_success() {
        return Err(status_err(status.as_u16(), &text));
    }
    if text.trim().is_empty() {
        return Ok(json!({ "ok": true }));
    }
    serde_json::from_str(&text).map_err(|e| RuntimeError::Protocol(e.to_string()))
}

fn status_err(code: u16, body: &str) -> RuntimeError {
    if (400..500).contains(&code) {
        RuntimeError::World(format!("http {code}: {body}"))
    } else {
        RuntimeError::Network(format!("http {code}: {body}"))
    }
}

pub async fn list_sessions(base: &str) -> Result<Vec<String>> {
    let v = get_json(&format!("{base}/v1/sessions")).await?;
    if let Some(arr) = v.get("sessions").and_then(|s| s.as_array()) {
        return Ok(arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect());
    }
    Ok(Vec::new())
}

pub async fn resolve_session(base: &str, wanted: Option<&str>) -> Result<String> {
    if let Some(id) = wanted {
        return Ok(id.to_string());
    }
    let ids = list_sessions(base).await?;
    ids.into_iter()
        .next()
        .ok_or_else(|| RuntimeError::World("no active session; run uoterm connect first".into()))
}

pub async fn session_state(base: &str, id: &str) -> Result<Value> {
    let v = get_json(&format!("{base}/v1/sessions/{id}/state")).await?;
    if let Some(err) = v.get("error").and_then(|e| e.as_str()) {
        return Err(RuntimeError::World(err.to_string()));
    }
    Ok(v)
}

pub async fn call_tool(base: &str, id: &str, name: &str, args: Value) -> Result<Value> {
    let v = post_json(&format!("{base}/v1/sessions/{id}/tools/{name}"), &args).await?;
    if v.get("ok") == Some(&json!(false)) {
        let msg = v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("tool failed");
        return Err(RuntimeError::World(msg.to_string()));
    }
    Ok(v)
}

pub fn parse_since(s: &str) -> ChronoDuration {
    let s = s.trim();
    if let Some(h) = s.strip_suffix('h').and_then(|n| n.parse::<i64>().ok()) {
        return ChronoDuration::hours(h);
    }
    if let Some(m) = s.strip_suffix('m').and_then(|n| n.parse::<i64>().ok()) {
        return ChronoDuration::minutes(m);
    }
    if let Some(sec) = s.strip_suffix('s').and_then(|n| n.parse::<i64>().ok()) {
        return ChronoDuration::seconds(sec);
    }
    ChronoDuration::hours(1)
}

pub fn harvest_paths() -> Vec<PathBuf> {
    let dir = data_dir();
    let mut out = vec![dir.join(JOURNAL_HARVEST_NAME)];
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("jsonl") && !out.contains(&p) {
                out.push(p);
            }
        }
    }
    out
}

pub fn read_harvest(since: &str) -> Result<Vec<Value>> {
    let cutoff = Utc::now() - parse_since(since);
    let mut rows = Vec::new();
    for path in harvest_paths() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if let Some(at) = v.get("at").and_then(|a| a.as_str()) {
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(at) {
                    if ts.with_timezone(&Utc) < cutoff {
                        continue;
                    }
                }
            }
            rows.push(v);
        }
    }
    Ok(rows)
}

pub fn goal_for_class(class: &str) -> &'static str {
    uoterm_runtime::tools::Goal::for_class(class).name()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_adds_http() {
        assert_eq!(normalize_base("127.0.0.1:7733"), "http://127.0.0.1:7733");
        assert_eq!(
            normalize_base("http://127.0.0.1:7733/"),
            "http://127.0.0.1:7733"
        );
    }

    #[test]
    fn since_hours_and_minutes() {
        assert_eq!(parse_since("2h"), ChronoDuration::hours(2));
        assert_eq!(parse_since("30m"), ChronoDuration::minutes(30));
        assert_eq!(parse_since("bogus"), ChronoDuration::hours(1));
    }

    #[test]
    fn class_maps_to_goal() {
        assert_eq!(goal_for_class("lumberjack"), "gather");
        assert_eq!(goal_for_class("traveler"), "travel");
        assert_eq!(goal_for_class("sitter"), "social");
    }

    #[test]
    fn bearer_value_trims_token() {
        assert_eq!(bearer_value("  secret  ").as_deref(), Some("Bearer secret"));
        assert!(bearer_value("   ").is_none());
        assert!(bearer_value("").is_none());
    }
}
