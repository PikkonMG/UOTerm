//! Who the API answers: the loopback rule and the token rule.
//!
//! A caller shows the token in an `Authorization: Bearer` header, or in a
//! cookie. A web page cannot put headers on a WebSocket, so it trades the
//! token once for the cookie at [`TOKEN_PATH`], and the browser sends the
//! cookie from then on.

use super::{ApiState, HEALTH_PATH};
use crate::config::{BEARER_PREFIX, ENV_API_TOKEN};
use axum::extract::{Request, State};
use axum::http::{
    header::{AUTHORIZATION, COOKIE, HOST, SET_COOKIE},
    HeaderMap, StatusCode,
};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;

const LOOPBACK_HOSTS: &[&str] = &["127.0.0.1", "localhost", "::1", "[::1]"];
/// Where a web page trades the token for the cookie.
pub(super) const TOKEN_PATH: &str = "/v1/web/token";
/// The cookie that carries the token.
const TOKEN_COOKIE: &str = "uoterm_token";
/// The browser keeps the cookie from the scripts of the page, and sends it
/// only on calls the page of this API makes, to every path of it.
const TOKEN_COOKIE_RULES: &str = "HttpOnly; SameSite=Strict; Path=/";
const COOKIE_SEPARATOR: char = ';';
const COOKIE_NAME_END: char = '=';

/// Who may call: the token a caller must show, if any, and whether only a
/// caller on this machine is answered.
#[derive(Clone)]
pub struct Guard {
    pub token: Option<String>,
    /// The API listens on this machine only, so it answers only a caller
    /// that names this machine. A web page that points a name it owns at
    /// this machine names itself, and is refused.
    pub local_only: bool,
}

/// Puts the guard in front of every route of `router`.
pub fn guard_layer<S>(router: Router<S>, guard: Guard) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router.layer(middleware::from_fn_with_state(guard, require_bearer))
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

async fn require_bearer(State(guard): State<Guard>, req: Request, next: Next) -> Response {
    if guard.local_only {
        let named = req.headers().get(HOST).and_then(|v| v.to_str().ok());
        if named.is_some_and(|host| !host_is_loopback(host)) {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "error": "this API answers callers on this machine only" })),
            )
                .into_response();
        }
    }
    // The token page checks the token itself, and gives the cookie.
    if [HEALTH_PATH, TOKEN_PATH].contains(&req.uri().path()) {
        return next.run(req).await;
    }
    let Some(expected) = guard.token.as_deref() else {
        return next.run(req).await;
    };
    let bearer = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix(BEARER_PREFIX));
    let presented = [bearer, cookie_token(req.headers())];
    if presented
        .into_iter()
        .flatten()
        .any(|token| same_secret(token, expected))
    {
        next.run(req).await
    } else {
        unauthorized()
    }
}

/// The token in the `uoterm_token` cookie, among every cookie the caller
/// sent.
fn cookie_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|cookies| cookies.split(COOKIE_SEPARATOR))
        .filter_map(|cookie| cookie.trim().split_once(COOKIE_NAME_END))
        .find(|(name, _)| *name == TOKEN_COOKIE)
        .map(|(_, token)| token)
}

#[derive(Deserialize)]
pub(super) struct TokenBody {
    token: String,
}

/// Gives the cookie for the right token, so the browser shows the token on
/// every later call, the live link too.
pub(super) async fn give_token(
    State(st): State<Arc<ApiState>>,
    Json(body): Json<TokenBody>,
) -> Response {
    let right = st
        .guard
        .token
        .as_deref()
        .is_some_and(|expected| same_secret(&body.token, expected));
    if !right {
        return unauthorized();
    }
    let cookie = format!("{TOKEN_COOKIE}={}; {TOKEN_COOKIE_RULES}", body.token);
    (StatusCode::NO_CONTENT, [(SET_COOKIE, cookie)]).into_response()
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(json!({ "error": "unauthorized" })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::live::tests::{serve_with_mock_session_token, try_connect_live};
    use crate::api::{router_for, ApiState};
    use crate::manager::Runtime;
    use axum::body::Body;
    use axum::http::header::CONTENT_TYPE;
    use axum::http::Method;
    use std::sync::Arc;
    use tower::ServiceExt;

    const TOKEN: &str = "s3cret-token";
    const TEST_SESSIONS: usize = 1;
    const LOCAL_HOST: &str = "127.0.0.1:7733";
    const FOREIGN_HOST: &str = "evil.example:7733";
    const SESSIONS_PATH: &str = "/v1/sessions";
    const JSON_TYPE: &str = "application/json";

    fn state_with_token(token: &str) -> Arc<ApiState> {
        Arc::new(ApiState {
            runtime: Runtime::new(TEST_SESSIONS),
            guard: Guard {
                token: Some(token.to_string()),
                local_only: true,
            },
        })
    }

    /// The status a local API wanting `TOKEN` gives one request.
    async fn status_of(
        method: Method,
        path: &str,
        host: &str,
        headers: &[(&str, &str)],
        body: Body,
    ) -> StatusCode {
        let mut request = axum::http::Request::builder()
            .method(method)
            .uri(path)
            .header(HOST, host);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        router_for(Runtime::new(TEST_SESSIONS), Some(TOKEN.to_string()), true)
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap()
            .status()
    }

    fn token_body(token: &str) -> Body {
        Body::from(json!({ "token": token }).to_string())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cookie_token_opens_the_live_link() {
        let server = serve_with_mock_session_token(TOKEN).await;
        let cookie = format!("{TOKEN_COOKIE}={TOKEN}");
        let ws = try_connect_live(server.addr, &server.id, Some(&cookie)).await;
        assert!(ws.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_wrong_cookie_is_refused() {
        let server = serve_with_mock_session_token(TOKEN).await;
        let cookie = format!("{TOKEN_COOKIE}=wrong");
        let refused = try_connect_live(server.addr, &server.id, Some(&cookie)).await;
        assert_eq!(refused.err(), Some(StatusCode::UNAUTHORIZED.as_u16()));
    }

    #[tokio::test]
    async fn the_token_page_sets_the_cookie_only_for_the_right_token() {
        let st = state_with_token(TOKEN);
        let good = give_token(
            State(st.clone()),
            Json(TokenBody {
                token: TOKEN.into(),
            }),
        )
        .await
        .into_response();
        assert_eq!(good.status(), StatusCode::NO_CONTENT);
        let cookie = good.headers()[SET_COOKIE].to_str().unwrap();
        assert!(cookie.starts_with(&format!("{TOKEN_COOKIE}={TOKEN};")));
        assert!(cookie.contains("HttpOnly"));
        assert!(cookie.contains("SameSite=Strict"));
        let bad = give_token(State(st), Json(TokenBody { token: "no".into() }))
            .await
            .into_response();
        assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
    }

    /// The page asks for its cookie with no token yet, so the token page
    /// needs none. The loopback rule still holds for it.
    #[tokio::test]
    async fn the_token_page_needs_no_token_but_a_local_caller() {
        let json = [(CONTENT_TYPE.as_str(), JSON_TYPE)];
        let local = status_of(
            Method::POST,
            TOKEN_PATH,
            LOCAL_HOST,
            &json,
            token_body(TOKEN),
        )
        .await;
        assert_eq!(local, StatusCode::NO_CONTENT);
        let wrong = status_of(
            Method::POST,
            TOKEN_PATH,
            LOCAL_HOST,
            &json,
            token_body("no"),
        )
        .await;
        assert_eq!(wrong, StatusCode::UNAUTHORIZED);
        let foreign = status_of(
            Method::POST,
            TOKEN_PATH,
            FOREIGN_HOST,
            &json,
            token_body(TOKEN),
        )
        .await;
        assert_eq!(foreign, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_route_opens_to_the_header_or_the_cookie() {
        let bearer = format!("{BEARER_PREFIX}{TOKEN}");
        let among_others = format!("theme=dark; {TOKEN_COOKIE}={TOKEN} ; lang=en");
        let wrong = format!("{TOKEN_COOKIE}=wrong");
        let cases: [(&[(&str, &str)], StatusCode); 4] = [
            (&[], StatusCode::UNAUTHORIZED),
            (&[(AUTHORIZATION.as_str(), &bearer)], StatusCode::OK),
            (&[(COOKIE.as_str(), &among_others)], StatusCode::OK),
            (&[(COOKIE.as_str(), &wrong)], StatusCode::UNAUTHORIZED),
        ];
        for (headers, expected) in cases {
            let status = status_of(
                Method::GET,
                SESSIONS_PATH,
                LOCAL_HOST,
                headers,
                Body::empty(),
            )
            .await;
            assert_eq!(status, expected, "{headers:?}");
        }
    }

    #[test]
    fn the_token_cookie_is_found_among_others() {
        let mut headers = axum::http::HeaderMap::new();
        headers.append(COOKIE, "theme=dark".parse().unwrap());
        headers.append(
            COOKIE,
            format!("a=1; {TOKEN_COOKIE}={TOKEN}").parse().unwrap(),
        );
        assert_eq!(cookie_token(&headers), Some(TOKEN));
        assert_eq!(cookie_token(&axum::http::HeaderMap::new()), None);
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
