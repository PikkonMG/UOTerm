//! The questions a web page asks Jev: an order in plain words, a pick
//! from a list of names, and the script lines of a wish. They run here, so
//! the key of TypeSafe never goes to the browser; the page learns only
//! whether Jev can answer.

use super::{refused, WebState};
use crate::orders;
use crate::window::Link;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use uoterm_runtime::tools::TOOL_HOTKEYS;
use uoterm_view::frame::WatchFrame;
use uoterm_view::orders::PickQuestion;

const NO_SESSION: &str = "not found";
/// An order reads only what is near in the picture, not its time.
const ORDER_FRAME_TIME: f64 = 0.0;

/// Where the page reads whether Jev can answer: `{"on": bool}`.
pub(super) const JEV_STATE_PATH: &str = "/v1/jev";

pub(super) fn routes() -> Router<WebState> {
    Router::new()
        .route(JEV_STATE_PATH, get(jev_state))
        .route("/v1/sessions/{id}/jev/order", post(order))
        .route("/v1/sessions/{id}/jev/pick", post(pick))
        .route("/v1/sessions/{id}/jev/lines", post(lines))
}

/// The key, when Jev may be asked about session `id`, or the status and
/// the words that refuse.
fn jev_key(state: &WebState, id: &str) -> Result<String, (StatusCode, &'static str)> {
    let key = state
        .jev_key
        .clone()
        .ok_or((StatusCode::SERVICE_UNAVAILABLE, orders::ORDER_OFF))?;
    if state.runtime.get(id).is_none() {
        return Err((StatusCode::NOT_FOUND, NO_SESSION));
    }
    Ok(key)
}

/// Whether Jev can answer: a TypeSafe key is set. The key itself stays
/// here.
async fn jev_state(State(state): State<WebState>) -> Response {
    Json(json!({ "on": state.jev_key.is_some() })).into_response()
}

#[derive(Deserialize)]
struct OrderBody {
    words: String,
    /// The picture of the `watch` tool the page shows.
    frame: Value,
}

/// The act the order asks for, as the page sends it on its live link, or
/// the words that say why there is none.
async fn order(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(body): Json<OrderBody>,
) -> Response {
    let key = match jev_key(&state, &id) {
        Ok(key) => key,
        Err((status, words)) => return refused(status, words),
    };
    let frame = WatchFrame::from_observe(&body.frame, ORDER_FRAME_TIME);
    match orders::ask(&key, &body.words, &frame).await {
        Ok(act) => Json(json!({ "act": act.for_page() })).into_response(),
        Err(words) => refused(StatusCode::CONFLICT, &words),
    }
}

#[derive(Deserialize)]
struct PickBody {
    question: PickQuestion,
    names: Vec<String>,
    wish: String,
}

/// The place in `names` of the one the wish names, or null when Jev is not
/// sure.
async fn pick(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(body): Json<PickBody>,
) -> Response {
    let key = match jev_key(&state, &id) {
        Ok(key) => key,
        Err((status, words)) => return refused(status, words),
    };
    let names: Vec<&str> = body.names.iter().map(String::as_str).collect();
    match orders::pick(&key, body.question.instructions(), &body.wish, &names).await {
        Ok(index) => Json(json!({ "index": index })).into_response(),
        Err(words) => refused(StatusCode::CONFLICT, &words),
    }
}

#[derive(Deserialize)]
struct LinesBody {
    wish: String,
}

/// The script lines of the hotkey the wish names. The hotkeys come from
/// the session, as for the window.
async fn lines(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(body): Json<LinesBody>,
) -> Response {
    let key = match jev_key(&state, &id) {
        Ok(key) => key,
        Err((status, words)) => return refused(status, words),
    };
    let link = Link::SameProgram {
        runtime: state.runtime.clone(),
        session: id,
    };
    let hotkeys = |name: Option<String>| {
        let link = link.clone();
        async move {
            let args = name.map_or_else(|| json!({}), |name| json!({ "name": name }));
            link.call(TOOL_HOTKEYS, args).await
        }
    };
    match orders::lines_for(&key, &body.wish, hotkeys).await {
        Ok(text) => Json(json!({ "lines": text.lines().collect::<Vec<_>>() })).into_response(),
        Err(words) => refused(StatusCode::CONFLICT, &words),
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{send, temp_folder};
    use super::super::WebState;
    use super::JEV_STATE_PATH;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{json, Value};
    use uoterm_runtime::Runtime;

    const TEST_KEY: &str = "test-key";

    fn post(path: &str, body: &Value) -> Request<Body> {
        Request::post(path)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(body).unwrap()))
            .unwrap()
    }

    fn asks() -> [(&'static str, Value); 3] {
        [
            (
                "/v1/sessions/s1/jev/order",
                json!({ "words": "attack the orc", "frame": {} }),
            ),
            (
                "/v1/sessions/s1/jev/pick",
                json!({ "question": "shard", "names": ["Atlantic"], "wish": "atlantic" }),
            ),
            ("/v1/sessions/s1/jev/lines", json!({ "wish": "heal me" })),
        ]
    }

    #[tokio::test]
    async fn with_no_typesafe_key_jev_is_unavailable() {
        let config = temp_folder();
        let state = WebState::open(None, config.path().to_path_buf(), Runtime::new(1), None);
        for (path, body) in asks() {
            let answer = send(state.clone(), post(path, &body)).await;
            assert_eq!(answer.status(), StatusCode::SERVICE_UNAVAILABLE, "{path}");
        }
    }

    #[tokio::test]
    async fn the_page_learns_whether_jev_can_answer_and_never_the_key() {
        let config = temp_folder();
        for key in [None, Some(TEST_KEY.to_string())] {
            let on = key.is_some();
            let state = WebState::open(None, config.path().to_path_buf(), Runtime::new(1), key);
            let request = axum::http::Request::get(JEV_STATE_PATH)
                .body(axum::body::Body::empty())
                .unwrap();
            let answer = send(state, request).await;
            assert_eq!(answer.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
                .await
                .unwrap();
            let text = String::from_utf8(bytes.to_vec()).unwrap();
            assert_eq!(
                serde_json::from_str::<Value>(&text).unwrap(),
                json!({ "on": on })
            );
            assert!(!text.contains(TEST_KEY));
        }
    }

    #[tokio::test]
    async fn jev_answers_only_for_a_session_of_the_runtime() {
        let config = temp_folder();
        let state = WebState::open(
            None,
            config.path().to_path_buf(),
            Runtime::new(1),
            Some(TEST_KEY.into()),
        );
        for (path, body) in asks() {
            let answer = send(state.clone(), post(path, &body)).await;
            assert_eq!(answer.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[tokio::test]
    async fn a_question_jev_does_not_know_is_refused() {
        let config = temp_folder();
        let state = WebState::open(
            None,
            config.path().to_path_buf(),
            Runtime::new(1),
            Some(TEST_KEY.into()),
        );
        let body = json!({ "question": "anything", "names": [], "wish": "x" });
        let answer = send(state, post("/v1/sessions/s1/jev/pick", &body)).await;
        assert_eq!(answer.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
