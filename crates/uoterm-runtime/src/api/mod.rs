//! The HTTP API of the runtime: sessions, their tools, and the live link
//! a web page keeps to one session.

mod auth;
mod live;

pub use auth::{api_token_from_env, bind_is_loopback, guard_layer, Guard};
pub use live::LIVE_POLL_MS;

use crate::config::{client_version, era_from_str, ConnectOptions, ENV_API_TOKEN};
use crate::manager::Runtime;
use crate::tools::{ToolCall, ToolResult, TOOL_OBSERVE};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uoterm_protocol::types::Era;

const HEALTH_PATH: &str = "/health";

/// What every route of the API reads: the sessions and who may call.
#[derive(Clone)]
pub struct ApiState {
    pub runtime: Runtime,
    pub guard: Guard,
}

/// Every route of the API on `runtime`, behind the guard. `uoterm web`
/// merges its own routes with these and puts the same guard on them with
/// [`guard_layer`].
pub fn router_for(runtime: Runtime, token: Option<String>, local_only: bool) -> Router {
    let guard = Guard { token, local_only };
    let state = Arc::new(ApiState {
        runtime,
        guard: guard.clone(),
    });
    let routes = Router::new()
        .route("/v1/sessions", get(list_sessions).post(create_session))
        .route("/v1/sessions/{id}/state", get(session_state))
        .route("/v1/sessions/{id}/tools/{name}", post(call_tool))
        .route("/v1/tools/{name}", post(call_runtime_tool))
        .route("/v1/sessions/{id}/live", get(live::live))
        .route(auth::TOKEN_PATH, post(auth::give_token))
        .route(HEALTH_PATH, get(|| async { "ok" }))
        .with_state(state);
    guard_layer(routes, guard)
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
    axum::serve(listener, router_for(runtime, token, local_only))
        .await
        .map_err(|e| crate::error::RuntimeError::Network(e.to_string()))
}

/// The answer to a call on a session the runtime does not have.
fn session_not_found() -> Response {
    (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response()
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
        None => session_not_found(),
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
            guard: Guard {
                token: None,
                local_only: true,
            },
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
}
