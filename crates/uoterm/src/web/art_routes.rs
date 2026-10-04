//! One picture of the client files as a PNG, with the point of it that
//! goes on the tile in a header. And what a page needs to lay out what it
//! draws: the lines of words in a UO font, and the drawn pixels of a gump.

use super::{kept, on_art, WebState, CONTENT_JSON, CONTENT_PNG};
use crate::art::client_art::ArtTooLarge;
use crate::art::png::encode;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uoterm_view::art::{ArtRequest, TextLook};

pub(super) const ART_PATH: &str = "/v1/art";
pub(super) const MEASURE_PATH: &str = "/v1/text/measure";
/// The point of the picture that goes on the tile, as `x,y` from its top
/// left.
const ANCHOR_HEADER: &str = "x-uoterm-anchor";

pub(super) fn routes() -> Router<WebState> {
    Router::new()
        .route(ART_PATH, post(art))
        .route(MEASURE_PATH, post(measure))
        .route("/v1/gump-mask/{id}", get(gump_mask))
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

/// Words in a UO font, as a picture of them would ask.
#[derive(Deserialize)]
struct MeasureRequest {
    text: String,
    look: TextLook,
}

/// The lines words break into and the height of one line. A bad request
/// for words a picture would refuse; not found with no UO fonts.
async fn measure(State(state): State<WebState>, Json(request): Json<MeasureRequest>) -> Response {
    on_art(
        &state,
        move |art| art.measured_text(&request.text, &request.look),
        |measured| match measured {
            Ok(Some(measured)) => Json(measured).into_response(),
            Ok(None) => StatusCode::NOT_FOUND.into_response(),
            Err(ArtTooLarge) => StatusCode::BAD_REQUEST.into_response(),
        },
    )
    .await
}

/// The drawn pixels of a gump: its width, its height, and the bits of its
/// mask in base64. The browser keeps it for this version of the client
/// files.
async fn gump_mask(State(state): State<WebState>, Path(id): Path<u16>) -> Response {
    let files_tag = state.files_tag.clone();
    on_art(
        &state,
        move |art| art.gump_mask(id),
        move |mask| {
            let Some(mask) = mask else {
                return StatusCode::NOT_FOUND.into_response();
            };
            let body = json!({
                "width": mask.width,
                "height": mask.height,
                "bits": BASE64.encode(&mask.bits),
            });
            kept(&files_tag, CONTENT_JSON, body.to_string().into_bytes())
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::super::tests::{fixture_uopath, send, state_in, temp_folder, test_state};
    use super::super::PICTURES_AT_ONCE;
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::sync::Arc;
    use uoterm_view::art::{ArtRequest, GumpMask, TextLook, TextMeasure};

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

    fn post_measure(text: &str, look: TextLook) -> Request<Body> {
        Request::post(MEASURE_PATH)
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({ "text": text, "look": look })).unwrap(),
            ))
            .unwrap()
    }

    #[tokio::test]
    async fn words_are_measured_as_their_picture_is_drawn() {
        use uoterm_nav::fixtures::{write_small_fonts, ASCII_FIXTURE_HEIGHT};
        let files = fixture_uopath();
        write_small_fonts(files.path());
        let state = state_in(Some(files.path()), files.path().join("config"));
        let look = TextLook::ascii(0, 0);
        let answer = send(state.clone(), post_measure("Hail\nfriend", look)).await;
        assert_eq!(answer.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        let measured: TextMeasure = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(measured.lines, ["Hail", "friend"]);
        assert_eq!(measured.line_height, u32::from(ASCII_FIXTURE_HEIGHT));
        let too_long = "a".repeat(crate::art::client_art::TEXT_MAX_CHARS + 1);
        let refused = send(state, post_measure(&too_long, look)).await;
        assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn with_no_fonts_words_are_not_measured() {
        let (state, _files) = test_state();
        let answer = send(state, post_measure("Hail", TextLook::ascii(0, 0))).await;
        assert_eq!(answer.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_gump_mask_marks_the_drawn_pixels() {
        use base64::Engine;
        use uoterm_nav::fixtures::{FIXTURE_GUMP_DRAWN, FIXTURE_GUMP_ID, FIXTURE_GUMP_SIZE};
        let (state, _files) = test_state();
        let path = format!("/v1/gump-mask/{FIXTURE_GUMP_ID}");
        let answer = send(
            state.clone(),
            Request::get(&path).body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(answer.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        let mask: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let (width, height) = FIXTURE_GUMP_SIZE;
        assert_eq!(mask["width"], width);
        assert_eq!(mask["height"], height);
        let bits = base64::engine::general_purpose::STANDARD
            .decode(mask["bits"].as_str().unwrap())
            .unwrap();
        let mask = GumpMask {
            width,
            height,
            bits,
        };
        let drawn: Vec<(usize, usize)> = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|(x, y)| mask.drawn_at(*x, *y))
            .collect();
        assert_eq!(drawn, FIXTURE_GUMP_DRAWN);
        let path = format!("/v1/gump-mask/{}", FIXTURE_GUMP_ID + 1);
        let missing = send(state, Request::get(&path).body(Body::empty()).unwrap()).await;
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn with_no_client_files_no_picture_is_made() {
        let config = temp_folder();
        let state = state_in(None, config.path().to_path_buf());
        let answer = send(state, post_art(&item(1))).await;
        assert_eq!(answer.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
