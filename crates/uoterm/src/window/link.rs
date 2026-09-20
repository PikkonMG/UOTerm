//! How the window reaches the session. When `connect` opens the window, both
//! are one program, and a tool call is a function call with no delay. The
//! `watch` command is a different program, so it calls over HTTP.
//!
//! Both ways make the same tool calls and get the same answers, so the rest
//! of the window does not know which one it has.

use crate::remote;
use serde_json::Value;
use uoterm_runtime::tools::ToolCall;
use uoterm_runtime::Runtime;

/// How often the window asks for the state of the session. A call inside one
/// program costs little, so that window asks at the pace of the screen.
const POLL_MS_SAME_PROGRAM: u64 = 33;
const POLL_MS_OVER_HTTP: u64 = 100;
const TOOL_FAILED: &str = "tool failed";

#[derive(Clone)]
pub enum Link {
    SameProgram { runtime: Runtime, session: String },
    Http { api: String, session: String },
}

impl Link {
    pub fn poll_ms(&self) -> u64 {
        match self {
            Self::SameProgram { .. } => POLL_MS_SAME_PROGRAM,
            Self::Http { .. } => POLL_MS_OVER_HTTP,
        }
    }

    /// Makes one tool call. The answer is the `result` of the tool. An
    /// error is words for the human.
    pub async fn call(&self, tool: &str, args: Value) -> Result<Value, String> {
        match self {
            Self::Http { api, session } => remote::call_tool(api, session, tool, args)
                .await
                .map(|answer| answer.get("result").cloned().unwrap_or(answer))
                .map_err(|e| e.to_string()),
            Self::SameProgram { runtime, session } => {
                let call = ToolCall {
                    name: tool.to_string(),
                    args,
                };
                let answer = runtime
                    .call(Some(session), call)
                    .await
                    .map_err(|e| e.to_string())?;
                if answer.ok {
                    Ok(answer.result)
                } else {
                    Err(answer.error.unwrap_or_else(|| TOOL_FAILED.to_string()))
                }
            }
        }
    }
}
