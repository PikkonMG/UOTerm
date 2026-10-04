//! One picture of the client files as a PNG, with the point of it that
//! goes on the tile in a header.

use super::{kept, on_art, WebState};
use crate::art::png::encode;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
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
/// which names the picture only on this server.
async fn art(State(state): State<WebState>, Json(request): Json<ArtRequest>) -> Response {
    let etag = format!("{}-{:x}", state.files_tag, request.key());
    on_art(
        &state,
        move |art| art.picture(&request),
        move |picture| {
            let Some(picture) = picture else {
                return StatusCode::NOT_FOUND.into_response();
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
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
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
