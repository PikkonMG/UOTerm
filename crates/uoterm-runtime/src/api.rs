use crate::config::{client_version, era_from_str, ConnectOptions, BEARER_PREFIX, ENV_API_TOKEN};
use crate::manager::Runtime;
use crate::tools::{ToolCall, ToolResult, TOOL_OBSERVE};
use axum::extract::{Path, Request, State};
use axum::http::{
    header::{AUTHORIZATION, HOST},
    StatusCode,
};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use uoterm_protocol::types::Era;

const HEALTH_PATH: &str = "/health";
const LOOPBACK_HOSTS: &[&str] = &["127.0.0.1", "localhost", "::1", "[::1]"];

#[derive(Clone)]
pub struct ApiState {
    pub runtime: Runtime,
    pub token: Option<String>,
    /// The API listens on this machine only, so it answers only a caller
    /// that names this machine. A web page that points a name it owns at
    /// this machine names itself, and is refused.
    pub local_only: bool,
}

pub fn api_token_from_env() -> Option<String> {
    std::env::var(ENV_API_TOKEN)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn bind_is_loopback(bind: &str) -> bool {
    if let Ok(addr) = bind.parse::<SocketAddr>() {
        return addr.ip().is_loopback();
    }
    host_is_loopback(bind)
}

/// True when a host, with or without its port, names this machine.
fn host_is_loopback(host_and_port: &str) -> bool {
    if let Ok(addr) = host_and_port.parse::<SocketAddr>() {
        return addr.ip().is_loopback();
    }
    if let Ok(ip) = host_and_port
        .trim_matches(|c| c == '[' || c == ']')
        .parse::<std::net::IpAddr>()
    {
        return ip.is_loopback();
    }
    let host = match host_and_port.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => host,
        _ => host_and_port,
    };
    LOOPBACK_HOSTS.contains(&host.trim_matches(|c| c == '[' || c == ']'))
}

/// Compares a token presented with the one expected in a time that does not
/// depend on where they first differ, so the answer times give nothing away.
fn same_secret(presented: &str, expected: &str) -> bool {
    let (a, b) = (presented.as_bytes(), expected.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn router_with_token(runtime: Runtime, token: Option<String>, local_only: bool) -> Router {
    let state = Arc::new(ApiState {
        runtime,
        token,
        local_only,
    });
    Router::new()
        .route("/v1/sessions", get(list_sessions).post(create_session))
        .route("/v1/sessions/{id}/state", get(session_state))
        .route("/v1/sessions/{id}/tools/{name}", post(call_tool))
        .route("/v1/tools/{name}", post(call_runtime_tool))
        .route(HEALTH_PATH, get(|| async { "ok" }))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer,
        ))
        .with_state(state)
}

pub async fn serve(bind: &str, runtime: Runtime) -> crate::error::Result<()> {
    let token = api_token_from_env();
    if !bind_is_loopback(bind) && token.is_none() {
        return Err(crate::error::RuntimeError::Usage(format!(
            "non-loopback --api-bind requires {ENV_API_TOKEN}"
        )));
    }
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|e| crate::error::RuntimeError::Network(e.to_string()))?;
    tracing::info!(bind, "HTTP API listening");
    let local_only = bind_is_loopback(bind);
    axum::serve(listener, router_with_token(runtime, token, local_only))
        .await
        .map_err(|e| crate::error::RuntimeError::Network(e.to_string()))
}

async fn require_bearer(
    State(st): State<Arc<ApiState>>,
    req: Request,
    next: Next,
) -> axum::response::Response {
    if st.local_only {
        let named = req.headers().get(HOST).and_then(|v| v.to_str().ok());
        if named.is_some_and(|host| !host_is_loopback(host)) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "this API answers callers on this machine only" })),
            )
                .into_response();
        }
    }
    if req.uri().path() == HEALTH_PATH {
        return next.run(req).await;
    }
    let Some(expected) = st.token.as_deref() else {
        return next.run(req).await;
    };
    let presented = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix(BEARER_PREFIX));
    if presented.is_some_and(|token| same_secret(token, expected)) {
        next.run(req).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "unauthorized" })),
        )
            .into_response()
    }
}

async fn list_sessions(State(st): State<Arc<ApiState>>) -> Json<Value> {
    Json(json!({ "sessions": st.runtime.list() }))
}

#[derive(Deserialize)]
struct CreateBody {
    host: String,
    port: u16,
    account: String,
    password: String,
    character: String,
    #[serde(default)]
    shard: Option<String>,
    #[serde(default)]
    era: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default = "crate::config::obey_shard_rules_default")]
    obey_shard_rules: bool,
    #[serde(default = "crate::config::answer_when_named_default")]
    answer_when_named: bool,
    #[serde(default)]
    play_along: bool,
    #[serde(default = "crate::config::reconnect_default")]
    reconnect: bool,
    /// `socks5://` or `http://` proxy to reach the shard through.
    #[serde(default)]
    proxy: Option<crate::proxy::Proxy>,
}

async fn create_session(
    State(st): State<Arc<ApiState>>,
    Json(body): Json<CreateBody>,
) -> impl IntoResponse {
    let era: Era = era_from_str(body.era.as_deref());
    let version = client_version(body.version.as_deref(), era, None);
    let opts = ConnectOptions {
        host: body.host,
        port: body.port,
        account: body.account,
        password: body.password,
        shard: body.shard,
        character: body.character,
        version,
        era,
        uopath: None,
        markers: None,
        persona: None,
        next_login_key: uoterm_protocol::types::LOGIN_NEXT_KEY_DEFAULT,
        encryption: Default::default(),
        obey_shard_rules: body.obey_shard_rules,
        answer_when_named: body.answer_when_named,
        play_along: body.play_along,
        picker: None,
        reconnect: body.reconnect,
        proxy: body.proxy,
    };
    match st.runtime.connect(opts).await {
        Ok(h) => (StatusCode::CREATED, Json(json!({ "id": h.id }))).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn session_state(
    State(st): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match st.runtime.get(&id) {
        Some(h) => {
            let obs = h
                .call(ToolCall {
                    name: TOOL_OBSERVE.into(),
                    args: json!({}),
                })
                .await;
            if obs.ok {
                (StatusCode::OK, Json(obs.result)).into_response()
            } else {
                (StatusCode::OK, Json(h.snapshot())).into_response()
            }
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
    }
}

async fn call_tool(
    State(st): State<Arc<ApiState>>,
    Path((id, name)): Path<(String, String)>,
    Json(args): Json<Value>,
) -> impl IntoResponse {
    match st.runtime.call(Some(&id), ToolCall { name, args }).await {
        Ok(r) if r.ok => (StatusCode::OK, Json(r)).into_response(),
        Ok(r) => (StatusCode::CONFLICT, Json(r)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ToolResult::err(e.to_string())),
        )
            .into_response(),
    }
}

/// A tool of the runtime itself, which needs no session: the character list
/// of an account, a new character, a login. See [`crate::characters`].
async fn call_runtime_tool(
    State(st): State<Arc<ApiState>>,
    Path(name): Path<String>,
    Json(args): Json<Value>,
) -> axum::response::Response {
    if !crate::characters::is_runtime_tool(&name) {
        return (
            StatusCode::NOT_FOUND,
            Json(ToolResult::err(format!("{name} is no tool of the runtime"))),
        )
            .into_response();
    }
    let answer = crate::characters::call(&st.runtime, &name, &args).await;
    let status = if answer.ok {
        StatusCode::OK
    } else {
        StatusCode::CONFLICT
    };
    (status, Json(answer)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_runtime_tool_is_answered_without_a_session() {
        let st = Arc::new(ApiState {
            runtime: Runtime::new(1),
            token: None,
            local_only: true,
        });
        let unknown =
            call_runtime_tool(State(st.clone()), Path("observe".into()), Json(json!({}))).await;
        assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
        let refused = call_runtime_tool(
            State(st),
            Path(crate::characters::TOOL_CHARACTERS.into()),
            Json(json!({})),
        )
        .await;
        assert_eq!(refused.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn loopback_binds_are_detected() {
        assert!(bind_is_loopback("127.0.0.1:7733"));
        assert!(bind_is_loopback("localhost:7733"));
        assert!(!bind_is_loopback("0.0.0.0:7733"));
        assert!(!bind_is_loopback("10.0.0.1:7733"));
    }

    /// The Host a caller names decides a local API's answer: this machine by
    /// any of its names passes, and a name a web page owns does not, even
    /// when that name points here.
    #[test]
    fn only_a_caller_that_names_this_machine_is_local() {
        for host in [
            "127.0.0.1:7733",
            "localhost:7733",
            "localhost",
            "[::1]:7733",
            "::1",
        ] {
            assert!(host_is_loopback(host), "{host}");
        }
        for host in ["evil.example:7733", "localhost.evil.example", "10.0.0.1"] {
            assert!(!host_is_loopback(host), "{host}");
        }
    }

    #[test]
    fn a_token_matches_only_itself() {
        assert!(same_secret("s3cret", "s3cret"));
        assert!(!same_secret("s3cres", "s3cret"));
        assert!(!same_secret("s3cret-and-more", "s3cret"));
        assert!(!same_secret("", "s3cret"));
    }
}
