//! MCP stdio JSON-RPC. Proxies tools and resources to a running HTTP API.

use crate::remote;
use base64::Engine;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use uoterm_protocol::types::EXIT_OK;
use uoterm_runtime::error::{Result, RuntimeError};
use uoterm_runtime::playbooks;

const PROTOCOL_VERSION: &str = "2024-11-05";
const HEADER_CONTENT_LENGTH: &str = "content-length";
const MAX_RPC_BODY: usize = 1_048_576;
const JSONRPC_PARSE_ERROR: i32 = -32700;

/// This tool lives here and not in the session, because only this program
/// can draw the watch window.
const TOOL_SCREENSHOT: &str = "screenshot";
const SCREENSHOT_ABOUT: &str = "A picture of the watch window: the real map round the character, the mobiles with their names, and the panels. Use it when the text radar is not enough, for example in a crowd or in a dungeon. It needs a desktop and takes a few seconds. Precondition: session exists";
/// The window needs time to open, load the art and come to rest.
const SCREENSHOT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
/// A vision model reads a picture of this width well, and it costs few tokens.
const SCREENSHOT_MAX_SIDE: u32 = 1024;
const SCREENSHOT_JPEG_QUALITY: u8 = 82;
const SCREENSHOT_MIME: &str = "image/jpeg";

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
        "tools/list" => tool_list(),
        "tools/call" if params.get("name").and_then(Value::as_str) == Some(TOOL_SCREENSHOT) => {
            match screenshot(base, &params).await {
                Ok(jpeg) => json!({
                    "content": [{
                        "type": "image",
                        "data": base64::engine::general_purpose::STANDARD.encode(jpeg),
                        "mimeType": SCREENSHOT_MIME
                    }]
                }),
                Err(e) => json!({
                    "content": [{ "type": "text", "text": e.to_string() }],
                    "isError": true
                }),
            }
        }
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
        "resources/list" => {
            let ids = remote::list_sessions(base).await.unwrap_or_default();
            json!({ "resources": playbooks::mcp_resources(ids) })
        }
        "resources/read" => {
            let uri = params.get("uri").and_then(|u| u.as_str()).unwrap_or("");
            match read_resource(base, uri).await {
                Ok((mime, text)) => json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": mime,
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

/// The tools of the session, and the tools of this program after them.
fn tool_list() -> Value {
    let mut list = uoterm_runtime::tools::mcp_tool_list();
    if let Some(tools) = list.get_mut("tools").and_then(Value::as_array_mut) {
        tools.push(json!({
            "name": TOOL_SCREENSHOT,
            "description": SCREENSHOT_ABOUT,
            "inputSchema": {
                "type": "object",
                "properties": { "session_id": {"type": "string"} }
            }
        }));
    }
    list
}

/// Opens the watch window for one picture, and gives the picture as a JPEG.
async fn screenshot(base: &str, params: &Value) -> Result<Vec<u8>> {
    let wanted = params
        .get("arguments")
        .and_then(|a| a.get("session_id"))
        .and_then(Value::as_str);
    let sid = remote::resolve_session(base, wanted).await?;
    let exe = std::env::current_exe().map_err(|e| RuntimeError::Usage(e.to_string()))?;
    let png = std::env::temp_dir().join(format!("uoterm-{}.png", uuid::Uuid::new_v4()));
    let mut child = tokio::process::Command::new(exe)
        .env("UOTERM_API", base)
        .env("UOTERM_SESSION", sid)
        .arg("watch")
        .arg("--snapshot")
        .arg(&png)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| RuntimeError::Usage(e.to_string()))?;
    let finished = tokio::time::timeout(SCREENSHOT_TIMEOUT, child.wait()).await;
    let picture = image::open(&png);
    let _ = std::fs::remove_file(&png);
    if finished.is_err() {
        return Err(RuntimeError::World(
            "the watch window did not draw in time; it needs a desktop where it is visible".into(),
        ));
    }
    let picture = picture.map_err(|_| {
        RuntimeError::World("the watch window made no picture; it needs a desktop".into())
    })?;
    jpeg_of(&picture)
}

fn jpeg_of(picture: &image::DynamicImage) -> Result<Vec<u8>> {
    let small = picture
        .thumbnail(SCREENSHOT_MAX_SIDE, SCREENSHOT_MAX_SIDE)
        .to_rgb8();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, SCREENSHOT_JPEG_QUALITY)
        .encode_image(&small)
        .map_err(|e| RuntimeError::Usage(e.to_string()))?;
    Ok(jpeg)
}

async fn tools_call(base: &str, params: &Value) -> Result<Value> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| RuntimeError::Usage("tools/call needs name".into()))?;
    let mut args = params.get("arguments").cloned().unwrap_or(json!({}));
    if uoterm_runtime::characters::is_runtime_tool(name) {
        return remote::call_runtime_tool(base, name, args).await;
    }
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

async fn read_resource(base: &str, uri: &str) -> Result<(String, String)> {
    if let Some(name) = playbooks::parse_uri(uri) {
        return match playbooks::get(name) {
            Some(book) => Ok((playbooks::PLAYBOOK_MIME.into(), book.body.to_string())),
            None => Err(RuntimeError::Usage(format!("unknown playbook {name}"))),
        };
    }
    let rest = uri.strip_prefix("uo://session/").ok_or_else(|| {
        RuntimeError::Usage("uri must be uo://session/{id}/state or uo://playbook/{name}".into())
    })?;
    let id = rest.trim_end_matches("/state");
    let state = remote::session_state(base, id).await?;
    Ok((
        playbooks::SESSION_MIME.into(),
        serde_json::to_string_pretty(&state)?,
    ))
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
    fn screenshot_is_listed_and_is_a_small_jpeg() {
        const JPEG_MAGIC: [u8; 2] = [0xFF, 0xD8];
        let listed = tool_list();
        let names: Vec<&str> = listed["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        assert!(names.contains(&TOOL_SCREENSHOT));
        assert!(names.contains(&"observe"));
        let wide = image::DynamicImage::new_rgba8(SCREENSHOT_MAX_SIDE * 2, SCREENSHOT_MAX_SIDE);
        let jpeg = jpeg_of(&wide).unwrap();
        assert_eq!(jpeg[..2], JPEG_MAGIC);
        let back = image::load_from_memory(&jpeg).unwrap();
        assert_eq!(back.width(), SCREENSHOT_MAX_SIDE);
    }

    #[test]
    fn content_length_cap_is_named() {
        assert!(content_length_allowed(MAX_RPC_BODY));
        assert!(!content_length_allowed(MAX_RPC_BODY + 1));
        assert!(parse_content_length("Content-Length: 12").is_some());
    }

    #[test]
    fn playbook_resources_are_listed() {
        let listed = playbooks::mcp_resources(Vec::<String>::new());
        assert!(
            listed
                .iter()
                .any(|r| r["uri"] == "uo://playbook/hunt"
                    && r["mimeType"] == playbooks::PLAYBOOK_MIME)
        );
        assert!(playbooks::read_playbook("uo://playbook/driver")
            .unwrap()
            .1
            .contains("next_event"));
    }
}
