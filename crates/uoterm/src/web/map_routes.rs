//! One block of the map: its 64 tiles, row by row from its north west
//! corner. An UltimaLive shard changes the map while it runs, so the
//! browser does not keep a block.

use super::{on_art_mut, WebState};
use axum::extract::{Path, State};
use axum::http::header::CACHE_CONTROL;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};

const KEEP_NOTHING: &str = "no-store";

pub(super) fn routes() -> Router<WebState> {
    Router::new().route("/v1/map/{map}/{block_x}/{block_y}", get(block))
}

/// The tiles of one block. Not found past the edge of the map.
async fn block(
    State(state): State<WebState>,
    Path((map, block_x, block_y)): Path<(u8, u16, u16)>,
) -> Response {
    on_art_mut(
        &state,
        move |art| art.cell_block(map, block_x, block_y),
        |cells| {
            if cells.is_empty() {
                return StatusCode::NOT_FOUND.into_response();
            }
            ([(CACHE_CONTROL, KEEP_NOTHING)], Json(cells)).into_response()
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::super::tests::{send, test_state};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use uoterm_nav::fixtures::{FIXTURE_WALL_CX, FIXTURE_WALL_CY, FIXTURE_WALL_GRAPHIC};
    use uoterm_view::art::Cell;

    const BLOCK_SIDE: usize = 8;

    #[tokio::test]
    async fn a_map_block_has_sixty_four_cells() {
        let (state, _files) = test_state();
        let answer = send(
            state,
            Request::get("/v1/map/0/0/0").body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(answer.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        let cells: Vec<Cell> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(cells.len(), BLOCK_SIDE * BLOCK_SIDE);
        let walls: Vec<usize> = (0..cells.len())
            .filter(|at| !cells[*at].statics.is_empty())
            .collect();
        let wall_at = usize::from(FIXTURE_WALL_CY) * BLOCK_SIDE + usize::from(FIXTURE_WALL_CX);
        assert_eq!(walls, [wall_at], "row by row from the north west");
        assert_eq!(cells[wall_at].statics[0].graphic, FIXTURE_WALL_GRAPHIC);
    }

    #[tokio::test]
    async fn a_block_past_the_edge_of_the_map_is_not_found() {
        const FAR_EAST: u16 = u16::MAX / 8;
        const FIRST_TOO_FAR: u16 = FAR_EAST + 1;
        let (state, _files) = test_state();
        for (x, y) in [
            (FAR_EAST, 0),
            (FIRST_TOO_FAR, 0),
            (0, FIRST_TOO_FAR),
            (u16::MAX, u16::MAX),
        ] {
            let path = format!("/v1/map/0/{x}/{y}");
            let answer = send(
                state.clone(),
                Request::get(&path).body(Body::empty()).unwrap(),
            )
            .await;
            assert_eq!(answer.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[tokio::test]
    async fn a_map_block_is_not_kept_by_the_browser() {
        let (state, _files) = test_state();
        let answer = send(
            state,
            Request::get("/v1/map/0/0/0").body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(answer.headers()["cache-control"], "no-store");
    }
}
