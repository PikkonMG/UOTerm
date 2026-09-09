//! MCP stdio JSON-RPC. Proxies tools and resources to a running HTTP API.

use crate::remote;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use uoterm_protocol::types::EXIT_OK;
use uoterm_runtime::error::{Result, RuntimeError};

const PROTOCOL_VERSION: &str = "2024-11-05";
const HEADER_CONTENT_LENGTH: &str = "content-length";
const MAX_RPC_BODY: usize = 1_048_576;
const JSONRPC_PARSE_ERROR: i32 = -32700;

pub async fn run_stdio(api: Option<&str>) -> Result<u8> {
    let base = remote::api_base(api);
    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    loop {
        let Some((msg, framed)) = read_rpc_message(&mut stdin).await? else {
            break;
        };
        if is_parse_error(&msg) {
            write_rpc_message(&mut stdout, &msg, framed).await?;
            continue;
        }
        if let Some(resp) = handle(&base, msg).await {
            write_rpc_message(&mut stdout, &resp, framed).await?;
        }
    }
    Ok(EXIT_OK as u8)
}

fn parse_error_response() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": Value::Null,
        "error": { "code": JSONRPC_PARSE_ERROR, "message": "parse error" }
    })
}

fn decode_rpc_json(raw: &[u8]) -> std::result::Result<Value, Value> {
    serde_json::from_slice(raw).map_err(|_| parse_error_response())
}

async fn read_rpc_message(
    stdin: &mut BufReader<tokio::io::Stdin>,
) -> Result<Option<(Value, bool)>> {
    let mut first = String::new();
    loop {
        first.clear();
        let n = stdin
            .read_line(&mut first)
            .await
            .map_err(|e| RuntimeError::Network(e.to_string()))?;
        if n == 0 {
            return Ok(None);
        }
        if !first.trim().is_empty() {
            break;
        }
    }
    let trimmed = first.trim();
    if trimmed.starts_with('{') {
        return match serde_json::from_str::<Value>(trimmed) {
            Ok(msg) => Ok(Some((msg, false))),
            Err(_) => Ok(Some((parse_error_response(), false))),
        };
    }
    let mut content_len = parse_content_length(&first);
    loop {
        let mut line = String::new();
        let n = stdin
            .read_line(&mut line)
            .await
            .map_err(|e| RuntimeError::Network(e.to_string()))?;
        if n == 0 {
            return Ok(None);
        }
        if line.trim().is_empty() {
            break;
        }
        if content_len.is_none() {
            content_len = parse_content_length(&line);
        }
    }
    let Some(len) = content_len else {
        return Ok(None);
    };
    if !content_length_allowed(len) {
        return Ok(Some((parse_error_response(), true)));
    }
    let mut buf = vec![0u8; len];
    stdin
        .read_exact(&mut buf)
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))?;
    match decode_rpc_json(&buf) {
        Ok(msg) => Ok(Some((msg, true))),
        Err(err) => Ok(Some((err, true))),
    }
}

fn content_length_allowed(len: usize) -> bool {
    len <= MAX_RPC_BODY
}

fn parse_content_length(line: &str) -> Option<usize> {
    let (key, value) = line.split_once(':')?;
    if !key.trim().eq_ignore_ascii_case(HEADER_CONTENT_LENGTH) {
        return None;
    }
    value.trim().parse().ok()
}

fn is_parse_error(msg: &Value) -> bool {
    msg.get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_i64())
        == Some(i64::from(JSONRPC_PARSE_ERROR))
}

async fn write_rpc_message(
    stdout: &mut tokio::io::Stdout,
    value: &Value,
    framed: bool,
) -> Result<()> {
    let body = serde_json::to_string(value)?;
    if framed {
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        stdout
            .write_all(header.as_bytes())
            .await
            .map_err(|e| RuntimeError::Network(e.to_string()))?;
        stdout
            .write_all(body.as_bytes())
            .await
            .map_err(|e| RuntimeError::Network(e.to_string()))?;
    } else {
        stdout
            .write_all(format!("{body}\n").as_bytes())
            .await
            .map_err(|e| RuntimeError::Network(e.to_string()))?;
    }
    stdout
        .flush()
        .await
        .map_err(|e| RuntimeError::Network(e.to_string()))?;
    Ok(())
}

async fn handle(base: &str, msg: Value) -> Option<Value> {
    let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = msg.get("id").cloned();
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {}, "resources": {} },
            "serverInfo": {
                "name": "uoterm",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
        "notifications/initialized" | "initialized" | "notifications/cancelled" => return None,
        "ping" => json!({}),
        "tools/list" => uoterm_runtime::tools::mcp_tool_list(),
        "tools/call" => match tools_call(base, &params).await {
            Ok(v) => json!({
                "content": [{ "type": "text", "text": v.to_string() }],
                "structuredContent": v
            }),
            Err(e) => json!({
                "content": [{ "type": "text", "text": e.to_string() }],
                "isError": true
            }),
        },
        "resources/list" => match remote::list_sessions(base).await {
            Ok(ids) => {
                let resources: Vec<Value> = ids
                    .into_iter()
                    .map(|sid| {
                        json!({
                            "uri": format!("uo://session/{sid}/state"),
                            "name": format!("session {sid} state"),
                            "mimeType": "application/json"
                        })
                    })
                    .collect();
                json!({ "resources": resources })
            }
            Err(e) => return id.map(|i| rpc_err(i, e.to_string())),
        },
        "resources/read" => {
            let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
            match read_resource(base, uri).await {
                Ok(text) => json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": text
                    }]
                }),
                Err(e) => return id.map(|i| rpc_err(i, e.to_string())),
            }
        }
        _ => {
            return id.map(|i| rpc_err(i, format!("unknown method {method}")));
        }
    };
    id.map(|i| json!({"jsonrpc": "2.0", "id": i, "result": result}))
}

fn rpc_err(id: Value, msg: String) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32000, "message": msg}})
}

async fn tools_call(base: &str, params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| RuntimeError::Usage("tools/call needs name".into()))?;
    let mut args = params.get("arguments").cloned().unwrap_or(json!({}));
    let wanted = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let sid = remote::resolve_session(base, wanted.as_deref()).await?;
    if let Some(obj) = args.as_object_mut() {
        obj.entry("session_id").or_insert(json!(sid.clone()));
    }
    remote::call_tool(base, &sid, name, args).await
}

async fn read_resource(base: &str, uri: &str) -> Result<String> {
    let rest = uri
        .strip_prefix("uo://session/")
        .ok_or_else(|| RuntimeError::Usage("uri must be uo://session/{id}/state".into()))?;
    let id = rest.trim_end_matches("/state");
    let state = remote::session_state(base, id).await?;
    Ok(serde_json::to_string_pretty(&state)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_uses_jsonrpc_code() {
        let err = parse_error_response();
        assert!(is_parse_error(&err));
        assert_eq!(
            err["error"]["code"].as_i64(),
            Some(i64::from(JSONRPC_PARSE_ERROR))
        );
        assert!(decode_rpc_json(b"{not json").is_err());
    }

    #[test]
    fn content_length_cap_is_named() {
        assert!(content_length_allowed(MAX_RPC_BODY));
        assert!(!content_length_allowed(MAX_RPC_BODY + 1));
        assert!(parse_content_length("Content-Length: 12").is_some());
    }
}
