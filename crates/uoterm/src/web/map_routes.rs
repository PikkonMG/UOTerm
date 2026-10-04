//! One block of the map: its 64 tiles, row by row from its north west
//! corner. An UltimaLive shard changes the map while it runs, so the
//! browser does not keep a block; it sends the live map of its pictures
//! here, and asks again for the blocks that changed.

use super::{kept, on_art_mut, WebState, CONTENT_PNG};
use crate::art::png::encode;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::header::CACHE_CONTROL;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::Value;
use uoterm_view::frame::watch_live_map;
use uoterm_view::map_lay::{MAP_PICTURE_PREFIX, NEAR_MAP_PREFIX};
use uoterm_view::model::map_item::{map_of_path, MAP_ITEM_PREFIX};

const KEEP_NOTHING: &str = "no-store";
pub(super) const LIVE_MAP_PATH: &str = "/v1/map/live";
/// The live map of a picture holds the blocks round the character only:
/// a few hundred kilobytes at most. This leaves room.
pub(super) const LIVE_MAP_MAX_BYTES: usize = 8 * 1024 * 1024;

pub(super) fn routes() -> Router<WebState> {
    Router::new()
        .route("/v1/map/{map}/{block_x}/{block_y}", get(block))
        .route(&format!("{NEAR_MAP_PREFIX}/{{map}}/{{x}}/{{y}}"), get(near))
        .route(
            &format!("{MAP_PICTURE_PREFIX}/{{map}}/{{tx}}/{{ty}}"),
            get(map_picture),
        )
        .route(
            &format!("{MAP_ITEM_PREFIX}/{{facet}}/{{start_x}}/{{start_y}}/{{end_x}}/{{end_y}}"),
            get(map_item),
        )
        .route(
            LIVE_MAP_PATH,
            post(live_map).layer(DefaultBodyLimit::max(LIVE_MAP_MAX_BYTES)),
        )
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

/// The land round a tile in its radar colors, as a PNG with the tile in
/// the middle: the map of a start town. Not found when no tile round it
/// has a color. The browser keeps it for this version of the client files.
async fn near(State(state): State<WebState>, Path((map, x, y)): Path<(u8, u16, u16)>) -> Response {
    let etag = format!("{}-near-{map}-{x}-{y}", state.files_tag);
    on_art_mut(
        &state,
        move |art| art.near_picture(map, (x, y)),
        move |picture| match picture.as_ref().and_then(encode) {
            Some(png) => kept(&etag, CONTENT_PNG, png),
            None => StatusCode::NOT_FOUND.into_response(),
        },
    )
    .await
}

/// One tile of the whole-world picture of a map, in its radar colors, as a
/// PNG. Not found past the picture or with no known land. The browser
/// keeps it for this version of the client files.
async fn map_picture(
    State(state): State<WebState>,
    Path((map, tx, ty)): Path<(u8, u16, u16)>,
) -> Response {
    let etag = format!("{}-world-{map}-{tx}-{ty}", state.files_tag);
    let tile = (usize::from(tx), usize::from(ty));
    on_art_mut(
        &state,
        move |art| art.map_tile_picture(map, tile),
        move |picture| match picture.as_ref().and_then(encode) {
            Some(png) => kept(&etag, CONTENT_PNG, png),
            None => StatusCode::NOT_FOUND.into_response(),
        },
    )
    .await
}

/// The land of a map item between its corners, in its radar colors, as a
/// PNG. A bad request when its end is not past its start; not found with
/// no known land.
async fn map_item(
    State(state): State<WebState>,
    Path((facet, start_x, start_y, end_x, end_y)): Path<(u8, u16, u16, u16, u16)>,
) -> Response {
    let Some(map) = map_of_path(facet, (start_x, start_y), (end_x, end_y)) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let etag = format!(
        "{}-map-item-{facet}-{start_x}-{start_y}-{end_x}-{end_y}",
        state.files_tag
    );
    on_art_mut(
        &state,
        move |art| art.map_item_picture(&map),
        move |picture| match picture.as_ref().and_then(encode) {
            Some(png) => kept(&etag, CONTENT_PNG, png),
            None => StatusCode::NOT_FOUND.into_response(),
        },
    )
    .await
}

/// Lays the live map of a picture, the `live_map` value of the `watch`
/// tool, over the map files. Gives the blocks that changed, for the page
/// to ask for again.
async fn live_map(State(state): State<WebState>, Json(live): Json<Value>) -> Response {
    let live = watch_live_map(&live);
    on_art_mut(
        &state,
        move |art| art.take_live_map(&live),
        |changed| Json(changed).into_response(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::super::tests::{fixture_uopath, send, state_in, test_state};
    use super::LIVE_MAP_PATH;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use uoterm_nav::fixtures::{FIXTURE_WALL_CX, FIXTURE_WALL_CY, FIXTURE_WALL_GRAPHIC};
    use uoterm_view::art::{Cell, MapBlockAt};
    use uoterm_view::map_lay::SPAN;

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

    /// The land round a place is a PNG of its radar colors, the place in
    /// the middle; a place with no known land round it has none.
    #[tokio::test]
    async fn the_land_near_a_place_is_a_picture_of_its_radar_colors() {
        const PNG_SIGNATURE: &[u8] = b"\x89PNG";
        let files = green_fixture();
        let state = state_in(Some(files.path()), files.path().join("config"));
        let answer = send(
            state.clone(),
            Request::get("/v1/map/near/0/4/4")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(answer.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        assert!(bytes.starts_with(PNG_SIGNATURE));
        let picture = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(picture.dimensions(), (SPAN as u32, SPAN as u32));
        let middle = (SPAN / 2) as u32;
        assert_eq!(picture.get_pixel(middle, middle).0, [0, 255, 0, 255]);
        let nowhere = send(
            state,
            Request::get("/v1/map/near/0/60000/60000")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(nowhere.status(), StatusCode::NOT_FOUND);
    }

    /// The radar colors of a fixture: every land tile green.
    fn green_fixture() -> super::super::tests::TempFolder {
        const RADARCOL_NAME: &str = "radarcol.mul";
        const ITEM_BASE: usize = 0x4000;
        const ALL_LAND_GREEN: u16 = 31 << 5;
        let files = fixture_uopath();
        let colors: Vec<u8> = (0..ITEM_BASE * 2)
            .flat_map(|_| ALL_LAND_GREEN.to_le_bytes())
            .collect();
        std::fs::write(files.path().join(RADARCOL_NAME), colors).unwrap();
        files
    }

    async fn picture_at(state: super::super::WebState, path: &str) -> (StatusCode, Vec<u8>) {
        let answer = send(state, Request::get(path).body(Body::empty()).unwrap()).await;
        let status = answer.status();
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, bytes.to_vec())
    }

    /// A tile of the world picture holds the sampled radar colors; one past
    /// the picture is not found.
    #[tokio::test]
    async fn a_tile_of_the_world_picture_is_a_png_of_its_radar_colors() {
        use uoterm_view::map_lay::{map_picture_path, map_picture_tiles, MAP_TILE_SIDE};
        let files = green_fixture();
        let state = state_in(Some(files.path()), files.path().join("config"));
        let (status, bytes) = picture_at(state.clone(), &map_picture_path(0, 0, 0)).await;
        assert_eq!(status, StatusCode::OK);
        let picture = image::load_from_memory(&bytes).unwrap().to_rgba8();
        let side = MAP_TILE_SIDE as u32;
        assert_eq!(picture.dimensions(), (side, side));
        assert_eq!(picture.get_pixel(0, 0).0, [0, 255, 0, 255]);
        let (across, _) = map_picture_tiles(0);
        let (status, _) = picture_at(state, &map_picture_path(0, across, 0)).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    /// The land of a map item is a PNG of its radar colors; corners whose
    /// end is not past their start are a bad request.
    #[tokio::test]
    async fn the_land_of_a_map_item_is_a_png_between_its_corners() {
        let files = green_fixture();
        let state = state_in(Some(files.path()), files.path().join("config"));
        let (status, bytes) = picture_at(state.clone(), "/v1/map-item/0/0/0/8/4").await;
        assert_eq!(status, StatusCode::OK);
        let picture = image::load_from_memory(&bytes).unwrap().to_rgba8();
        assert_eq!(picture.dimensions(), (8, 4));
        let (status, _) = picture_at(state, "/v1/map-item/0/8/0/8/4").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
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

    /// The live map of a watch picture, with the land of block `block` all
    /// of land `land_id`.
    fn live_map(block: u32, land_id: u16) -> serde_json::Value {
        let cell: String = land_id
            .to_le_bytes()
            .iter()
            .chain(&[0u8])
            .map(|byte| format!("{byte:02x}"))
            .collect();
        serde_json::json!({
            "map": 0,
            "revision": 1,
            "blocks": [{ "block": block, "changed": 1, "land": cell.repeat(BLOCK_SIDE * BLOCK_SIDE) }],
        })
    }

    fn post_live(body: Vec<u8>) -> Request<Body> {
        Request::post(LIVE_MAP_PATH)
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    #[tokio::test]
    async fn a_live_map_changes_the_blocks_it_names() {
        const NEW_LAND: u16 = 3;
        const SECOND_BLOCK: u32 = 1;
        let (state, _files) = test_state();
        let body = serde_json::to_vec(&live_map(SECOND_BLOCK, NEW_LAND)).unwrap();
        let answer = send(state.clone(), post_live(body.clone())).await;
        assert_eq!(answer.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        let changed: Vec<MapBlockAt> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            changed,
            [MapBlockAt {
                map: 0,
                bx: 0,
                by: 1
            }],
            "block 1 of a map two blocks high"
        );
        let block = send(
            state.clone(),
            Request::get("/v1/map/0/0/1").body(Body::empty()).unwrap(),
        )
        .await;
        let bytes = axum::body::to_bytes(block.into_body(), usize::MAX)
            .await
            .unwrap();
        let cells: Vec<Cell> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(cells[0].land_id, Some(NEW_LAND));
        let again = send(state, post_live(body)).await;
        let bytes = axum::body::to_bytes(again.into_body(), usize::MAX)
            .await
            .unwrap();
        let changed: Vec<MapBlockAt> = serde_json::from_slice(&bytes).unwrap();
        assert!(changed.is_empty(), "the same change is laid once");
    }

    /// A block the map files could not take is not named as changed: here
    /// its land holds one tile, not sixty four.
    #[tokio::test]
    async fn a_live_block_the_map_cannot_take_is_not_named() {
        const ONE_TILE_OF_LAND: &str = "010000";
        let (state, _files) = test_state();
        let short = serde_json::json!({
            "map": 0,
            "revision": 1,
            "blocks": [{ "block": 1, "changed": 1, "land": ONE_TILE_OF_LAND }],
        });
        let body = serde_json::to_vec(&short).unwrap();
        let answer = send(state, post_live(body)).await;
        assert_eq!(answer.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        let changed: Vec<MapBlockAt> = serde_json::from_slice(&bytes).unwrap();
        assert!(changed.is_empty(), "{changed:?}");
    }

    #[tokio::test]
    async fn a_live_map_larger_than_the_limit_is_refused() {
        let (state, _files) = test_state();
        let answer = send(state, post_live(vec![b' '; super::LIVE_MAP_MAX_BYTES + 1])).await;
        assert_eq!(answer.status(), StatusCode::PAYLOAD_TOO_LARGE);
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
