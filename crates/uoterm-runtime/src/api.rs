use crate::config::{era_from_str, version_from_str, ConnectOptions, BEARER_PREFIX, ENV_API_TOKEN};
use crate::manager::Runtime;
use crate::tools::{ToolCall, ToolResult, TOOL_OBSERVE};
use axum::extract::{Path, Request, State};
use axum::http::{header::AUTHORIZATION, StatusCode};
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
    let host = bind
        .rsplit_once(':')
        .map(|(h, _)| h.trim_matches(|c| c == '[' || c == ']'))
        .unwrap_or(bind);
    LOOPBACK_HOSTS.contains(&host)
}

fn router_with_token(runtime: Runtime, token: Option<String>) -> Router {
    let state = Arc::new(ApiState { runtime, token });
    Router::new()
        .route("/v1/sessions", get(list_sessions).post(create_session))
        .route("/v1/sessions/{id}/state", get(session_state))
        .route("/v1/sessions/{id}/tools/{name}", post(call_tool))
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
    axum::serve(listener, router_with_token(runtime, token))
        .await
        .map_err(|e| crate::error::RuntimeError::Network(e.to_string()))
}

async fn require_bearer(
    State(st): State<Arc<ApiState>>,
    req: Request,
    next: Next,
) -> axum::response::Response {
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
    if presented == Some(expected) {
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
}

async fn create_session(
    State(st): State<Arc<ApiState>>,
    Json(body): Json<CreateBody>,
) -> impl IntoResponse {
    let era: Era = era_from_str(body.era.as_deref().unwrap_or("modern"));
    let version = version_from_str(body.version.as_deref(), era);
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
        persona: None,
        stay_on_socket: true,
        next_login_key: uoterm_protocol::types::LOGIN_NEXT_KEY_DEFAULT,
        encryption: Default::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_binds_are_detected() {
        assert!(bind_is_loopback("127.0.0.1:7733"));
        assert!(bind_is_loopback("localhost:7733"));
        assert!(!bind_is_loopback("0.0.0.0:7733"));
        assert!(!bind_is_loopback("10.0.0.1:7733"));
    }
}
