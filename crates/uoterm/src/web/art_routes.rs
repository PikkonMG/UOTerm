//! One picture of the client files as a PNG, with the point of it that
//! goes on the tile in a header.

use super::{kept, on_art, WebState};
use crate::art::client_art::ArtTooLarge;
use crate::art::png::encode;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use std::sync::Arc;
use uoterm_view::art::ArtRequest;

pub(super) const ART_PATH: &str = "/v1/art";
/// The point of the picture that goes on the tile, as `x,y` from its top
/// left.
const ANCHOR_HEADER: &str = "x-uoterm-anchor";
const CONTENT_PNG: &str = "image/png";

pub(super) fn routes() -> Router<WebState> {
    Router::new().route(ART_PATH, post(art))
}

/// The picture a request asks for. Its ETag holds the key of the request,
/// which names the picture only on this server. A bad request for a
/// picture too large. It waits while other pictures are made, and keeps
/// its permit until its PNG is made, even when the page goes away.
async fn art(State(state): State<WebState>, Json(request): Json<ArtRequest>) -> Response {
    let etag = format!("{}-{:x}", state.files_tag, request.key());
    let Ok(permit) = Arc::clone(&state.pictures).acquire_owned().await else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    on_art(
        &state,
        move |art| art.checked_picture(&request),
        move |picture| {
            let _making = permit;
            let picture = match picture {
                Ok(Some(picture)) => picture,
                Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                Err(ArtTooLarge) => return StatusCode::BAD_REQUEST.into_response(),
            };
            let anchor = format!("{},{}", picture.anchor.x, picture.anchor.y);
            let (Some(png), Ok(anchor)) = (encode(&picture), HeaderValue::from_str(&anchor)) else {
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            };
            let mut answer = kept(&etag, CONTENT_PNG, png);
            answer.headers_mut().insert(ANCHOR_HEADER, anchor);
            answer
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::super::tests::{send, test_state};
    use super::super::PICTURES_AT_ONCE;
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use uoterm_view::art::ArtRequest;

    fn post_art(request: &ArtRequest) -> Request<Body> {
        Request::post(ART_PATH)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(request).unwrap()))
            .unwrap()
    }

    fn item(graphic: u16) -> ArtRequest {
        ArtRequest::Item {
            graphic,
            hue: 0,
            whole_hue: false,
            border: false,
        }
    }

    #[tokio::test]
    async fn an_item_picture_comes_as_png_with_its_anchor() {
        let (state, _files) = test_state();
        let tag = state.files_tag.clone();
        let answer = send(state, post_art(&item(1))).await;
        assert_eq!(answer.status(), StatusCode::OK);
        assert_eq!(answer.headers()["content-type"], "image/png");
        assert_eq!(answer.headers()["x-uoterm-anchor"], "0,0");
        assert_eq!(
            answer.headers()["cache-control"],
            "public, max-age=31536000, immutable"
        );
        let etag = answer.headers()["etag"].to_str().unwrap().to_string();
        assert!(etag.contains(&tag), "{etag}");
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[tokio::test]
    async fn two_pictures_have_two_etags() {
        let (state, _files) = test_state();
        let one = send(state.clone(), post_art(&item(0))).await;
        let two = send(state, post_art(&item(1))).await;
        assert_ne!(one.headers()["etag"], two.headers()["etag"]);
    }

    #[tokio::test]
    async fn words_too_long_for_a_picture_are_a_bad_request() {
        let (state, _files) = test_state();
        let request = ArtRequest::Text {
            text: "a".repeat(crate::art::client_art::TEXT_MAX_CHARS + 1),
            look: uoterm_view::art::TextLook::unicode(1, 0),
        };
        let answer = send(state, post_art(&request)).await;
        assert_eq!(answer.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_figure_with_too_many_worn_items_is_a_bad_request() {
        let (state, _files) = test_state();
        let request = ArtRequest::Figure {
            look: uoterm_view::frame::WatchLook {
                equipment: vec![
                    uoterm_view::frame::WatchEquip::default();
                    crate::art::client_art::FIGURE_MAX_EQUIPMENT + 1
                ],
                ..Default::default()
            },
            pose: uoterm_view::art::Pose {
                action: uoterm_nav::Action::Stand,
                tick: 0,
            },
            paint: uoterm_view::art::Paint {
                outline: [0; 4],
                whole_hue: None,
            },
        };
        let answer = send(state, post_art(&request)).await;
        assert_eq!(answer.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_picture_waits_while_others_are_made() {
        const WAIT: std::time::Duration = std::time::Duration::from_millis(100);
        let (state, _files) = test_state();
        let busy = Arc::clone(&state.pictures)
            .acquire_many_owned(PICTURES_AT_ONCE as u32)
            .await
            .unwrap();
        let waiting = tokio::time::timeout(WAIT, send(state.clone(), post_art(&item(1)))).await;
        assert!(waiting.is_err(), "no picture is made while all are busy");
        drop(busy);
        let answer = send(state, post_art(&item(1))).await;
        assert_eq!(answer.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn a_missing_item_is_not_found() {
        let (state, _files) = test_state();
        let answer = send(state, post_art(&item(2))).await;
        assert_eq!(answer.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn with_no_client_files_no_picture_is_made() {
        let answer = send(WebState::open(None), post_art(&item(1))).await;
        assert_eq!(answer.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
