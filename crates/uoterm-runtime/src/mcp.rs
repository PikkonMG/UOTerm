use crate::manager::Runtime;
use crate::tools::{mcp_tool_list, ToolCall, TOOL_OBSERVE};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const PROTOCOL_VERSION: &str = "2024-11-05";

pub async fn serve_stdio(runtime: Runtime) -> crate::error::Result<()> {
    let mut lines = BufReader::new(tokio::io::stdin()).lines();
    let mut stdout = tokio::io::stdout();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(v) = handle_line(&runtime, &line).await {
            let mut s = serde_json::to_string(&v)?;
            s.push('\n');
            stdout.write_all(s.as_bytes()).await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}

async fn handle_line(runtime: &Runtime, line: &str) -> Option<Value> {
    let msg: Value = serde_json::from_str(line).ok()?;
    let method = msg.get("method")?.as_str()?;
    let id = msg.get("id").cloned();
    let params = msg.get("params").cloned().unwrap_or(json!({}));
    let result = match method {
        "initialize" => json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {}, "resources": {} },
            "serverInfo": { "name": "uoterm", "version": env!("CARGO_PKG_VERSION") }
        }),
        "notifications/initialized" => return None,
        "tools/list" => mcp_tool_list(),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match runtime
                .call(
                    None,
                    ToolCall {
                        name: name.into(),
                        args,
                    },
                )
                .await
            {
                Ok(r) => json!({
                    "content": [{ "type": "text", "text": serde_json::to_string(&r).unwrap_or_default() }],
                    "isError": !r.ok
                }),
                Err(e) => json!({
                    "content": [{ "type": "text", "text": e.to_string() }],
                    "isError": true
                }),
            }
        }
        "resources/list" => {
            let resources: Vec<Value> = runtime
                .list()
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
        "resources/read" => {
            let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
            let sid = uri
                .strip_prefix("uo://session/")
                .and_then(|s| s.strip_suffix("/state"))
                .unwrap_or("");
            match runtime.get(sid) {
                Some(h) => {
                    let obs = h
                        .call(ToolCall {
                            name: TOOL_OBSERVE.into(),
                            args: json!({}),
                        })
                        .await;
                    json!({
                        "contents": [{
                            "uri": uri,
                            "mimeType": "application/json",
                            "text": serde_json::to_string(&obs.result).unwrap_or_default()
                        }]
                    })
                }
                None => json!({ "contents": [] }),
            }
        }
        "ping" => json!({}),
        other => {
            return id.map(|id| {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": { "code": -32601, "message": format!("unknown method {other}") }
                })
            });
        }
    };
    id.map(|id| json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}
