//! The tables the scene of the web client builds with, read from the
//! client files. The browser keeps each one for as long as the client
//! files do not change.

use super::{on_art, on_art_mut, table, WebState};
use crate::art::client_art::ClientArt;
use crate::creation_files::read_creation_tables;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use std::collections::BTreeMap;
use uoterm_nav::{frames_question_fits, Action};
use uoterm_view::model::compare::ItemLayers;
use uoterm_view::model::creation::{
    every_palette_hue, CreationFiles, CREATION_FILES_PATH, TOWNS_SEPARATOR,
};

pub(super) fn routes() -> Router<WebState> {
    Router::new()
        .route("/v1/data/tiledata", get(tiledata))
        .route("/v1/data/multis/{id}", get(multi))
        .route("/v1/data/animdata", get(art_cycles))
        .route("/v1/data/anim-rules", get(anim_rules))
        .route("/v1/data/radarcol", get(radar))
        .route("/v1/data/seasons", get(seasons))
        .route("/v1/data/lights/{id}", get(light))
        .route("/v1/data/cliloc", get(cliloc))
        .route(
            "/v1/data/frames/{body}/{action}/{direction}/{mounted}",
            get(frames),
        )
        .route(CREATION_FILES_PATH, get(creation))
        .route("/v1/data/item-layers", get(item_layers))
        .route("/v1/data/hues-text/{hue}", get(text_rgb))
}

/// Reads with `read` from the client files on a blocking thread, then
/// makes the answer with `answer` once they are unlocked. It gets the files
/// tag for the ETag.
async fn from_files<T: Send + 'static>(
    state: &WebState,
    read: impl FnOnce(&ClientArt) -> T + Send + 'static,
    answer: impl FnOnce(T, &str) -> Response + Send + 'static,
) -> Response {
    let files_tag = state.files_tag.clone();
    on_art(state, read, move |value| answer(value, &files_tag)).await
}

async fn tiledata(State(state): State<WebState>) -> Response {
    from_files(&state, ClientArt::tiledata_tables, |tiles, tag| {
        table(tiles.as_deref(), tag)
    })
    .await
}

/// The pieces of a house or a boat. Empty for a multi the files do not
/// hold.
async fn multi(State(state): State<WebState>, Path(id): Path<u16>) -> Response {
    from_files(
        &state,
        move |art| Some(art.multi_pieces(id).to_vec()),
        table,
    )
    .await
}

/// The picture cycle of each item graphic that has one, by graphic.
async fn art_cycles(State(state): State<WebState>) -> Response {
    from_files(&state, ClientArt::art_cycles_table, |cycles, tag| {
        let by_graphic = cycles
            .as_deref()
            .map(|cycles| cycles.entries().collect::<BTreeMap<_, _>>());
        table(by_graphic, tag)
    })
    .await
}

async fn anim_rules(State(state): State<WebState>) -> Response {
    from_files(&state, |art| art.anim_rules().cloned(), table).await
}

async fn radar(State(state): State<WebState>) -> Response {
    from_files(&state, ClientArt::radar_tables, table).await
}

async fn seasons(State(state): State<WebState>) -> Response {
    from_files(&state, ClientArt::season_tables, |seasons, tag| {
        table(Some(&*seasons), tag)
    })
    .await
}

/// A light shape is read from the files the first time it is asked for, so
/// it takes the client art alone.
async fn light(State(state): State<WebState>, Path(id): Path<u8>) -> Response {
    let files_tag = state.files_tag.clone();
    on_art_mut(
        &state,
        move |art| art.light_shape(id).cloned(),
        move |shape| table(shape, &files_tag),
    )
    .await
}

/// Every message of the text database by its number.
async fn cliloc(State(state): State<WebState>) -> Response {
    from_files(&state, ClientArt::cliloc_table, |words, tag| {
        let by_number = words
            .as_deref()
            .map(|words| words.entries().collect::<BTreeMap<_, _>>());
        table(by_number, tag)
    })
    .await
}

/// How many frames a body has for an action, facing a direction, on a
/// mount or not. The action is `stand`, `walk`, `run` or a group number. A
/// bad request for a question the animation files cannot hold.
async fn frames(
    State(state): State<WebState>,
    Path((body, action, direction, mounted)): Path<(u16, String, u8, bool)>,
) -> Response {
    let action = action
        .parse::<Action>()
        .ok()
        .filter(|action| frames_question_fits(body, direction, *action));
    let Some(action) = action else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    from_files(
        &state,
        move |art| art.body_frame_count(body, direction, action, mounted),
        table,
    )
    .await
}

#[derive(Deserialize)]
struct CreationQuery {
    /// The text numbers of the words about the start towns the shard
    /// offers, comma separated.
    #[serde(default)]
    towns: String,
}

/// What the character creation reads, with only the words it reads, and
/// the color of each hue of its palettes. The small creation files are
/// read once the client art is unlocked.
async fn creation(State(state): State<WebState>, Query(query): Query<CreationQuery>) -> Response {
    let towns: Result<Vec<u32>, _> = query
        .towns
        .split(TOWNS_SEPARATOR)
        .filter(|number| !number.is_empty())
        .map(str::parse)
        .collect();
    let Ok(towns) = towns else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    from_files(
        &state,
        |art| {
            let hue_colors = every_palette_hue()
                .into_iter()
                .filter_map(|hue| Some((hue, art.text_rgb(hue)?)))
                .collect();
            (art.uopath().to_path_buf(), art.cliloc_table(), hue_colors)
        },
        move |(uopath, words, hue_colors), tag| {
            let files = CreationFiles {
                hue_colors,
                ..read_creation_tables(&uopath).with_needed_words(words.as_deref(), &towns)
            };
            table(Some(files), tag)
        },
    )
    .await
}

async fn item_layers(State(state): State<WebState>) -> Response {
    from_files(&state, ClientArt::tiledata_tables, |tiles, tag| {
        table(tiles.as_deref().map(ItemLayers::of_tiles), tag)
    })
    .await
}

/// The color of words written in a hue, as red, green and blue.
async fn text_rgb(State(state): State<WebState>, Path(hue): Path<u16>) -> Response {
    from_files(&state, move |art| art.text_rgb(hue), table).await
}

#[cfg(test)]
mod tests {
    use super::super::tests::{fixture_uopath, send, state_in, test_state};
    use super::super::WebState;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use uoterm_nav::fixtures::{write_cliloc, FIXTURE_LAND_NAMES, FIXTURE_WALL_NAME};
    use uoterm_view::model::creation::{every_palette_hue, CreationFiles};

    const CLILOC_NAME: &str = "Cliloc.enu";
    const RADARCOL_NAME: &str = "radarcol.mul";
    const HUES_NAME: &str = "hues.mul";
    /// The text numbers of the name of Advanced, the profession every
    /// client has, of a start town, and of a message the creation does not
    /// read.
    const ADVANCED_NAME_ID: u32 = 1_061_176;
    const BRITAIN_WORDS: u32 = 1_075_074;
    const REFUSAL: u32 = 1_001_018;

    fn get(path: &str) -> Request<Body> {
        Request::get(path).body(Body::empty()).unwrap()
    }

    async fn json(state: WebState, path: &str) -> Value {
        let answer = send(state, get(path)).await;
        assert_eq!(answer.status(), StatusCode::OK, "{path}");
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn tables_are_cached_by_the_files_tag() {
        let (state, _files) = test_state();
        let tag = state.files_tag.clone();
        let answer = send(state, get("/v1/data/tiledata")).await;
        assert!(answer.headers()["etag"].to_str().unwrap().contains(&tag));
        assert_eq!(
            answer.headers()["cache-control"],
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn the_tiledata_holds_the_land_and_the_items() {
        let (state, _files) = test_state();
        let tiles = json(state, "/v1/data/tiledata").await;
        assert_eq!(tiles["land"][0]["name"], FIXTURE_LAND_NAMES[0]);
        assert_eq!(tiles["items"][0]["name"], FIXTURE_WALL_NAME);
    }

    #[tokio::test]
    async fn tables_the_client_files_do_not_hold_are_not_found() {
        let (state, _files) = test_state();
        for path in [
            "/v1/data/animdata",
            "/v1/data/anim-rules",
            "/v1/data/radarcol",
            "/v1/data/lights/1",
            "/v1/data/cliloc",
            "/v1/data/frames/400/walk/2/false",
            "/v1/data/hues-text/33",
        ] {
            let answer = send(state.clone(), get(path)).await;
            assert_eq!(answer.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[tokio::test]
    async fn a_frame_question_the_files_cannot_hold_is_a_bad_request() {
        let (state, _files) = test_state();
        for path in [
            "/v1/data/frames/400/fly/2/false",
            "/v1/data/frames/2048/walk/2/false",
            "/v1/data/frames/400/walk/8/false",
            "/v1/data/frames/400/80/2/false",
        ] {
            let answer = send(state.clone(), get(path)).await;
            assert_eq!(answer.status(), StatusCode::BAD_REQUEST, "{path}");
        }
    }

    #[tokio::test]
    async fn the_seasons_multis_and_item_layers_come_as_tables() {
        const GRASS: &str = "196";
        const SNOW: u64 = 282;
        const WINTER: usize = 3;
        let (state, _files) = test_state();
        let seasons = json(state.clone(), "/v1/data/seasons").await;
        assert_eq!(seasons["seasons"][WINTER]["land"][GRASS], SNOW);
        let pieces = json(state.clone(), "/v1/data/multis/1").await;
        assert_eq!(pieces, Value::Array(Vec::new()), "no houses in the files");
        let layers = json(state, "/v1/data/item-layers").await;
        assert_eq!(layers["layers"], serde_json::json!({}), "nothing is worn");
    }

    #[tokio::test]
    async fn the_radar_colors_come_by_land_and_by_item() {
        const ITEM_BASE: usize = 0x4000;
        const ITEMS: usize = 2;
        const PURE_RED: u16 = 31 << 10;
        let files = fixture_uopath();
        let mut colors = vec![0u8; (ITEM_BASE + ITEMS) * 2];
        colors[..2].copy_from_slice(&PURE_RED.to_le_bytes());
        std::fs::write(files.0.join(RADARCOL_NAME), colors).unwrap();
        let radar = json(
            state_in(Some(files.path()), files.path().join("config")),
            "/v1/data/radarcol",
        )
        .await;
        assert_eq!(radar["land"][0], serde_json::json!([255, 0, 0]));
        assert_eq!(radar["items"].as_array().unwrap().len(), ITEMS);
    }

    #[tokio::test]
    async fn the_text_database_comes_by_number() {
        let files = fixture_uopath();
        write_cliloc(&files.0.join(CLILOC_NAME), &[(REFUSAL, "No.")]);
        let words = json(
            state_in(Some(files.path()), files.path().join("config")),
            "/v1/data/cliloc",
        )
        .await;
        assert_eq!(words, serde_json::json!({ REFUSAL.to_string(): "No." }));
    }

    #[tokio::test]
    async fn the_creation_gets_the_color_of_every_palette_hue() {
        const HUE_GROUP_BYTES: usize = 708;
        const HUE_GROUPS: usize = 512;
        const WHITE: [u8; 3] = [255, 255, 255];
        let files = fixture_uopath();
        let all_white = vec![u8::MAX; HUE_GROUP_BYTES * HUE_GROUPS];
        std::fs::write(files.0.join(HUES_NAME), all_white).unwrap();
        let state = state_in(Some(files.path()), files.path().join("config"));
        let creation: CreationFiles =
            serde_json::from_value(json(state, "/v1/data/creation?towns=").await).unwrap();
        let hues = every_palette_hue();
        assert_eq!(
            creation.hue_colors.keys().copied().collect::<Vec<_>>(),
            hues
        );
        assert!(creation.hue_colors.values().all(|rgb| *rgb == WHITE));
    }

    #[tokio::test]
    async fn the_creation_gets_only_the_words_it_reads() {
        let files = fixture_uopath();
        write_cliloc(
            &files.0.join(CLILOC_NAME),
            &[
                (ADVANCED_NAME_ID, "Advanced"),
                (BRITAIN_WORDS, "Britain"),
                (REFUSAL, "No."),
            ],
        );
        let state = state_in(Some(files.path()), files.path().join("config"));
        let path = format!("/v1/data/creation?towns={BRITAIN_WORDS}");
        let creation: CreationFiles =
            serde_json::from_value(json(state.clone(), &path).await).unwrap();
        assert_eq!(creation.words(ADVANCED_NAME_ID, ""), "Advanced");
        assert_eq!(creation.words(BRITAIN_WORDS, ""), "Britain");
        assert_eq!(creation.words(REFUSAL, "left out"), "left out");
        let bad = send(state, get("/v1/data/creation?towns=Britain")).await;
        assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    }
}
