use crate::characters;
use crate::manager::Runtime;
use crate::playbooks;
use crate::tools::{mcp_tool_list, ToolCall, TOOL_OBSERVE};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const PROTOCOL_VERSION: &str = "2024-11-05";
/// The argument that names the session a tool of a session acts in.
const ARG_SESSION_ID: &str = "session_id";

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
            let session = args
                .get(ARG_SESSION_ID)
                .and_then(Value::as_str)
                .map(str::to_string);
            let answered = if characters::is_runtime_tool(name) {
                Ok(characters::call(runtime, name, &args).await)
            } else {
                runtime
                    .call(
                        session.as_deref(),
                        ToolCall {
                            name: name.into(),
                            args,
                        },
                    )
                    .await
            };
            match answered {
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
            json!({ "resources": playbooks::mcp_resources(runtime.list()) })
        }
        "resources/read" => {
            let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
            if let Some((mime, text)) = playbooks::read_playbook(uri) {
                json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": mime,
                        "text": text
                    }]
                })
            } else {
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
                                "mimeType": playbooks::SESSION_MIME,
                                "text": serde_json::to_string(&obs.result).unwrap_or_default()
                            }]
                        })
                    }
                    None => json!({ "contents": [] }),
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_runtime_tool_needs_no_session_and_a_session_tool_does() {
        let rt = Runtime::new(1);
        let listed = handle_line(&rt, r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#)
            .await
            .unwrap();
        let tools = listed["result"]["tools"].as_array().unwrap();
        assert!(tools
            .iter()
            .any(|t| t["name"] == characters::TOOL_CHARACTERS));
        let asked = handle_line(
            &rt,
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"characters","arguments":{"account":"a"}}}"#,
        )
        .await
        .unwrap();
        let words = asked["result"]["content"][0]["text"].as_str().unwrap();
        assert!(words.contains("password_env"), "{words}");
        let observe = handle_line(
            &rt,
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"observe","arguments":{}}}"#,
        )
        .await
        .unwrap();
        assert_eq!(observe["result"]["isError"], json!(true));
    }

    #[tokio::test]
    async fn lists_and_reads_playbook_resources() {
        let rt = Runtime::new(1);
        let listed = handle_line(&rt, r#"{"jsonrpc":"2.0","id":1,"method":"resources/list"}"#)
            .await
            .unwrap();
        let resources = listed["result"]["resources"].as_array().unwrap();
        assert!(
            resources
                .iter()
                .any(|r| r["uri"] == "uo://playbook/hunt"
                    && r["mimeType"] == playbooks::PLAYBOOK_MIME)
        );
        let read = handle_line(
            &rt,
            r#"{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"uo://playbook/driver"}}"#,
        )
        .await
        .unwrap();
        let text = read["result"]["contents"][0]["text"].as_str().unwrap();
        assert!(text.contains("next_event"));
        assert_eq!(
            read["result"]["contents"][0]["mimeType"],
            playbooks::PLAYBOOK_MIME
        );
    }
}
