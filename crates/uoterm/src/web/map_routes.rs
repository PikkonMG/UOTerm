//! One block of the map: its 64 tiles, row by row from its north west
//! corner, and the pictures of the map in its radar colors. An UltimaLive
//! shard changes the map while it runs, so the browser does not keep a
//! block; it sends the live map of its pictures under its session, and
//! asks again for the blocks that changed. A page names its session in
//! the `session` query, and gets the map with the changes of that session
//! only.

use super::{kept, on_art, on_art_mut, WebState, CONTENT_PNG};
use crate::art::client_art::ClientArt;
use crate::art::facet_maps::FacetMaps;
use crate::art::png::encode;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{MutexGuard, PoisonError};
use uoterm_runtime::api::not_found;
use uoterm_view::art::Picture;
use uoterm_view::frame::watch_live_map;
use uoterm_view::map_lay::{MAP_PICTURE_PREFIX, NEAR_MAP_PREFIX};
use uoterm_view::model::map_item::{map_of_path, MAP_ITEM_PREFIX};

const KEEP_NOTHING: &str = "no-store";
pub(super) const LIVE_MAP_PATH: &str = "/v1/sessions/{id}/map/live";
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

/// The session whose live map lies over the map files. With none, the map
/// files alone.
#[derive(Deserialize)]
struct SessionQuery {
    session: Option<String>,
}

/// The live maps of the sessions the runtime still has. Those of the
/// sessions that ended are let go.
fn live_maps(state: &WebState) -> MutexGuard<'_, HashMap<String, FacetMaps>> {
    let mut maps = state
        .live_maps
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    maps.retain(|id, _| state.runtime.get(id).is_some());
    maps
}

/// Runs `read` with the live map of `session`, when the runtime still has
/// that session.
fn under_live<T>(
    state: &WebState,
    session: Option<&str>,
    read: impl FnOnce(Option<&FacetMaps>) -> T,
) -> T {
    let maps = live_maps(state);
    read(session.and_then(|id| maps.get(id)))
}

/// True when the live map of `session` changed facet `map`.
fn changed_live(state: &WebState, session: Option<&str>, map: u8) -> bool {
    under_live(state, session, |live| {
        live.is_some_and(|live| live.has_live(map))
    })
}

/// A PNG of the map. The browser keeps it for this version of the client
/// files, unless a live map changed its facet: then it keeps nothing.
fn map_png(etag: &str, png: Vec<u8>, changed: bool) -> Response {
    if changed {
        (
            [(CONTENT_TYPE, CONTENT_PNG), (CACHE_CONTROL, KEEP_NOTHING)],
            png,
        )
            .into_response()
    } else {
        kept(etag, CONTENT_PNG, png)
    }
}

/// The picture `make` makes of facet `map` with the live map of
/// `session`, as a PNG. Not found when it makes none.
async fn live_picture(
    state: &WebState,
    session: Option<String>,
    map: u8,
    etag: String,
    make: impl FnOnce(&mut ClientArt, Option<&FacetMaps>) -> Option<Picture> + Send + 'static,
) -> Response {
    let live_state = state.clone();
    on_art_mut(
        state,
        move |art| {
            under_live(&live_state, session.as_deref(), |live| {
                let changed = live.is_some_and(|live| live.has_live(map));
                (changed, make(art, live))
            })
        },
        move |(changed, picture)| match picture.as_ref().and_then(encode) {
            Some(png) => map_png(&etag, png, changed),
            None => StatusCode::NOT_FOUND.into_response(),
        },
    )
    .await
}

/// The tiles of one block. Not found past the edge of the map.
async fn block(
    State(state): State<WebState>,
    Path((map, block_x, block_y)): Path<(u8, u16, u16)>,
    Query(query): Query<SessionQuery>,
) -> Response {
    let live_state = state.clone();
    on_art_mut(
        &state,
        move |art| {
            under_live(&live_state, query.session.as_deref(), |live| {
                art.cell_block(live, map, block_x, block_y)
            })
        },
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
/// has a color.
async fn near(
    State(state): State<WebState>,
    Path((map, x, y)): Path<(u8, u16, u16)>,
    Query(query): Query<SessionQuery>,
) -> Response {
    let etag = format!("{}-near-{map}-{x}-{y}", state.files_tag);
    live_picture(&state, query.session, map, etag, move |art, live| {
        art.near_picture(live, map, (x, y))
    })
    .await
}

/// One tile of the whole-world picture of a map, in its radar colors, as a
/// PNG. Not found past the picture or with no known land. A tile of the
/// map files alone is kept here too.
async fn map_picture(
    State(state): State<WebState>,
    Path((map, tx, ty)): Path<(u8, u16, u16)>,
    Query(query): Query<SessionQuery>,
) -> Response {
    let etag = format!("{}-world-{map}-{tx}-{ty}", state.files_tag);
    let tile = (usize::from(tx), usize::from(ty));
    if changed_live(&state, query.session.as_deref(), map) {
        return live_picture(&state, query.session, map, etag, move |art, live| {
            art.map_tile_picture(live, map, tile)
        })
        .await;
    }
    let key = (map, tx, ty);
    let made = state
        .map_tiles
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
        .cloned();
    if let Some(png) = made {
        return kept(&etag, CONTENT_PNG, png);
    }
    let tiles = state.map_tiles.clone();
    on_art_mut(
        &state,
        move |art| art.map_tile_picture(None, map, tile),
        move |picture| match picture.as_ref().and_then(encode) {
            Some(png) => {
                tiles
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(key, png.clone());
                kept(&etag, CONTENT_PNG, png)
            }
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
    Query(query): Query<SessionQuery>,
) -> Response {
    let Some(map) = map_of_path(facet, (start_x, start_y), (end_x, end_y)) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let etag = format!(
        "{}-map-item-{facet}-{start_x}-{start_y}-{end_x}-{end_y}",
        state.files_tag
    );
    live_picture(&state, query.session, facet, etag, move |art, live| {
        art.map_item_picture(live, &map)
    })
    .await
}

/// Lays the live map of a picture of session `id`, the `live_map` value
/// of the `watch` tool, over the map files of that session. Gives the
/// blocks that changed, for the page to ask for again. Not found for a
/// session the runtime does not have.
async fn live_map(
    State(state): State<WebState>,
    Path(id): Path<String>,
    Json(live): Json<Value>,
) -> Response {
    if state.runtime.get(&id).is_none() {
        return not_found();
    }
    let live = watch_live_map(&live);
    let live_state = state.clone();
    on_art(
        &state,
        move |art| {
            live_maps(&live_state)
                .entry(id)
                .or_default()
                .take_live_map(art.uopath(), &live)
        },
        |changed| Json(changed).into_response(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::super::tests::{fixture_uopath, send, state_in, test_state, TempFolder};
    use super::super::WebState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use uoterm_nav::fixtures::{FIXTURE_WALL_CX, FIXTURE_WALL_CY, FIXTURE_WALL_GRAPHIC};
    use uoterm_protocol::types::{ClientVersion, Era};
    use uoterm_runtime::config::ConnectOptions;
    use uoterm_runtime::mock::{MockServer, MOCK_CHAR};
    use uoterm_runtime::Runtime;
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
        assert!(
            state.map_tiles.lock().unwrap().get(&(0, 0, 0)).is_some(),
            "the tile is kept"
        );
        let (status, again) = picture_at(state.clone(), &map_picture_path(0, 0, 0)).await;
        assert_eq!((status, again), (StatusCode::OK, bytes));
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

    fn post_live(session: &str, body: Vec<u8>) -> Request<Body> {
        Request::post(format!("/v1/sessions/{session}/map/live"))
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    }

    async fn json_of<T: serde::de::DeserializeOwned>(answer: axum::response::Response) -> T {
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// The routes on the fixture files with `count` sessions on a mock
    /// shard. The shard and the folder must outlive the state.
    async fn state_with_sessions(count: usize) -> (WebState, TempFolder, MockServer, Vec<String>) {
        let shard = MockServer::start().await.unwrap();
        let runtime = Runtime::new(count);
        let mut ids = Vec::new();
        for _ in 0..count {
            let options = ConnectOptions {
                host: shard.addr.ip().to_string(),
                port: shard.addr.port(),
                account: "test".into(),
                password: "test".into(),
                character: MOCK_CHAR.into(),
                version: ClientVersion::T2A,
                era: Era::T2a,
                ..ConnectOptions::default()
            };
            ids.push(runtime.connect(options).await.unwrap().id);
        }
        let files = fixture_uopath();
        let state = WebState {
            runtime,
            ..state_in(Some(files.path()), files.path().join("config"))
        };
        (state, files, shard, ids)
    }

    /// The land of the first tile of block 0, 1 as `session` sees it.
    async fn first_land(state: &WebState, session: Option<&str>) -> Option<u16> {
        let query = session.map_or(String::new(), |id| format!("?session={id}"));
        let answer = send(
            state.clone(),
            Request::get(format!("/v1/map/0/0/1{query}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(answer.status(), StatusCode::OK);
        json_of::<Vec<Cell>>(answer).await[0].land_id
    }

    #[tokio::test]
    async fn a_live_map_changes_the_blocks_it_names() {
        const NEW_LAND: u16 = 3;
        const SECOND_BLOCK: u32 = 1;
        let (state, _files, _shard, ids) = state_with_sessions(1).await;
        let id = &ids[0];
        let body = serde_json::to_vec(&live_map(SECOND_BLOCK, NEW_LAND)).unwrap();
        let answer = send(state.clone(), post_live(id, body.clone())).await;
        assert_eq!(answer.status(), StatusCode::OK);
        let changed: Vec<MapBlockAt> = json_of(answer).await;
        assert_eq!(
            changed,
            [MapBlockAt {
                map: 0,
                bx: 0,
                by: 1
            }],
            "block 1 of a map two blocks high"
        );
        assert_eq!(first_land(&state, Some(id)).await, Some(NEW_LAND));
        let again = send(state, post_live(id, body)).await;
        let changed: Vec<MapBlockAt> = json_of(again).await;
        assert!(changed.is_empty(), "the same change is laid once");
    }

    /// The live map of one session never shows in the map of another, nor
    /// in the map files alone, and goes when its session ends.
    #[tokio::test]
    async fn a_live_map_belongs_to_its_session_and_ends_with_it() {
        const NEW_LAND: u16 = 3;
        const SECOND_BLOCK: u32 = 1;
        let (state, _files, _shard, ids) = state_with_sessions(2).await;
        let (changer, other) = (&ids[0], &ids[1]);
        let before = first_land(&state, None).await;
        assert_ne!(before, Some(NEW_LAND));
        let body = serde_json::to_vec(&live_map(SECOND_BLOCK, NEW_LAND)).unwrap();
        let answer = send(state.clone(), post_live(changer, body)).await;
        assert_eq!(answer.status(), StatusCode::OK);
        assert_eq!(first_land(&state, Some(changer)).await, Some(NEW_LAND));
        assert_eq!(first_land(&state, Some(other)).await, before);
        assert_eq!(first_land(&state, None).await, before);
        state.runtime.stop(changer).await.unwrap();
        assert_eq!(first_land(&state, Some(changer)).await, before);
        assert!(state.live_maps.lock().unwrap().is_empty(), "let go");
    }

    #[tokio::test]
    async fn a_live_map_of_an_unknown_session_is_not_found() {
        let (state, _files) = test_state();
        let body = serde_json::to_vec(&live_map(1, 3)).unwrap();
        let answer = send(state.clone(), post_live("s999", body)).await;
        assert_eq!(answer.status(), StatusCode::NOT_FOUND);
        assert!(state.live_maps.lock().unwrap().is_empty());
    }

    /// A block the map files could not take is not named as changed: here
    /// its land holds one tile, not sixty four.
    #[tokio::test]
    async fn a_live_block_the_map_cannot_take_is_not_named() {
        const ONE_TILE_OF_LAND: &str = "010000";
        let (state, _files, _shard, ids) = state_with_sessions(1).await;
        let short = serde_json::json!({
            "map": 0,
            "revision": 1,
            "blocks": [{ "block": 1, "changed": 1, "land": ONE_TILE_OF_LAND }],
        });
        let body = serde_json::to_vec(&short).unwrap();
        let answer = send(state, post_live(&ids[0], body)).await;
        assert_eq!(answer.status(), StatusCode::OK);
        let changed: Vec<MapBlockAt> = json_of(answer).await;
        assert!(changed.is_empty(), "{changed:?}");
    }

    #[tokio::test]
    async fn a_live_map_larger_than_the_limit_is_refused() {
        let (state, _files) = test_state();
        let answer = send(
            state,
            post_live("s1", vec![b' '; super::LIVE_MAP_MAX_BYTES + 1]),
        )
        .await;
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
