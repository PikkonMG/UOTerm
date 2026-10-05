//! The files of the config folder a web page reads and writes: the
//! profiles of the options, the other kept files, the player fonts, and the
//! screenshots. They are the same files the window reads, so a player has
//! the same options in both.

use super::{on_blocking, send_file, WebState};
use crate::kept;
use crate::window::fonts::{font_names, fonts_dir};
use crate::window::map_files::{change_user_markers, map_dir_in, map_folder, MarkerFault};
use crate::window::screenshot::{create_new_file, screenshots_dir};
use crate::window::{shard_address, CharacterKey, ProfileStore};
use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, Request, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::Write;
use uoterm_view::guard::{KeptGrabBags, GRAB_BAGS_FILE};
use uoterm_view::model::world_map::{
    MarkerChange, MAP_FILES_KEPT, MARKER_CHANGE_PATH, MARKER_FULL_STATUS, MARKER_INVALID_STATUS,
    MARKER_STALE_STATUS,
};
use uoterm_view::settings::Profile;
use uoterm_view::ui::deck::{KeptHotbars, HOTBAR_FILE};

/// A change of the own marker file holds two markers: far less than this.
pub(super) const MARKER_CHANGE_MAX_BYTES: usize = 64 * 1024;
/// A screenshot is never larger than this.
pub(super) const SCREENSHOT_MAX_BYTES: usize = 32 * 1024 * 1024;
/// Every PNG file starts with these bytes.
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
/// The shard of a profile is `host:port`.
const PORT_SEPARATOR: char = ':';

pub(super) fn routes() -> Router<WebState> {
    Router::new()
        .route("/v1/profiles/default", get(read_default).put(write_default))
        .route(
            "/v1/profiles/{shard}/{character}",
            get(read_character).put(write_character),
        )
        .route("/v1/kept/{name}", get(read_kept).put(write_kept))
        .route(
            MARKER_CHANGE_PATH,
            post(change_markers).layer(DefaultBodyLimit::max(MARKER_CHANGE_MAX_BYTES)),
        )
        .route("/v1/fonts", get(list_fonts))
        .route("/v1/fonts/{name}", get(font))
        .route(
            "/v1/screenshots",
            post(save_screenshot).layer(DefaultBodyLimit::max(SCREENSHOT_MAX_BYTES)),
        )
}

/// One character of one shard, with the shard named as the window names
/// it. None for a shard with no port, or an empty name.
fn character_key(shard: &str, character: &str) -> Option<CharacterKey> {
    let (host, port) = shard.rsplit_once(PORT_SEPARATOR)?;
    let port: u16 = port.parse().ok()?;
    CharacterKey::new(&shard_address(host, port), character)
}

/// The profile of the character of a shard, or the default profile with
/// neither. A character with no profile of its own has the default one,
/// as in the window. A bad request for one of the two alone, or for a
/// shard with no port.
pub(super) fn profile_of(
    config_dir: &std::path::Path,
    shard: Option<&str>,
    character: Option<&str>,
) -> Result<Profile, StatusCode> {
    let store = ProfileStore::in_folder(config_dir);
    match (shard, character) {
        (None, None) => Ok(store.load_default()),
        (Some(shard), Some(character)) => character_key(shard, character)
            .map(|key| store.load_character(&key))
            .ok_or(StatusCode::BAD_REQUEST),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// No content when the file was written.
fn written(saved: bool) -> Response {
    if saved {
        StatusCode::NO_CONTENT.into_response()
    } else {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn read_default(State(state): State<WebState>) -> Response {
    on_blocking(move || {
        Json(ProfileStore::in_folder(&state.config_dir).load_default()).into_response()
    })
    .await
}

async fn write_default(State(state): State<WebState>, Json(profile): Json<Profile>) -> Response {
    on_blocking(move || written(ProfileStore::in_folder(&state.config_dir).save_default(&profile)))
        .await
}

async fn read_character(
    State(state): State<WebState>,
    Path((shard, character)): Path<(String, String)>,
) -> Response {
    on_blocking(
        move || match profile_of(&state.config_dir, Some(&shard), Some(&character)) {
            Ok(profile) => Json(profile).into_response(),
            Err(status) => status.into_response(),
        },
    )
    .await
}

async fn write_character(
    State(state): State<WebState>,
    Path((shard, character)): Path<(String, String)>,
    Json(profile): Json<Profile>,
) -> Response {
    let Some(key) = character_key(&shard, &character) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    on_blocking(move || {
        written(ProfileStore::in_folder(&state.config_dir).save_character(&key, &profile))
    })
    .await
}

/// The kept files a web page may read, besides the profiles. It writes
/// the hotbars and the grab bags; the marker and zone files of the map
/// folder it only reads.
#[derive(Clone, Copy)]
enum KeptFile {
    Hotbars,
    GrabBags,
    MapFiles,
}

impl KeptFile {
    fn named(name: &str) -> Option<Self> {
        match name {
            HOTBAR_FILE => Some(Self::Hotbars),
            GRAB_BAGS_FILE => Some(Self::GrabBags),
            MAP_FILES_KEPT => Some(Self::MapFiles),
            _ => None,
        }
    }

    /// The file as JSON, from the config folder. A missing or bad file
    /// gives the default.
    fn read(self, config_dir: &std::path::Path) -> Value {
        match self {
            Self::Hotbars => as_json(kept::load_from::<KeptHotbars>(
                &config_dir.join(HOTBAR_FILE),
            )),
            Self::GrabBags => as_json(kept::load_from::<KeptGrabBags>(
                &config_dir.join(GRAB_BAGS_FILE),
            )),
            Self::MapFiles => as_json(map_folder(&map_dir_in(config_dir))),
        }
    }

    /// Writes the JSON of a page as the file. A bad request when the JSON
    /// is not what the file holds; not allowed for a file the page only
    /// reads.
    fn write(self, config_dir: &std::path::Path, value: Value) -> Response {
        match self {
            Self::Hotbars => write_as::<KeptHotbars>(&config_dir.join(HOTBAR_FILE), value),
            Self::GrabBags => write_as::<KeptGrabBags>(&config_dir.join(GRAB_BAGS_FILE), value),
            Self::MapFiles => StatusCode::METHOD_NOT_ALLOWED.into_response(),
        }
    }
}

fn as_json(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or_default()
}

fn write_as<T: DeserializeOwned + Serialize>(path: &std::path::Path, value: Value) -> Response {
    match serde_json::from_value::<T>(value) {
        Ok(value) => written(kept::save_to(path, &value)),
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

async fn read_kept(State(state): State<WebState>, Path(name): Path<String>) -> Response {
    let Some(file) = KeptFile::named(&name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    on_blocking(move || Json(file.read(&state.config_dir)).into_response()).await
}

async fn write_kept(
    State(state): State<WebState>,
    Path(name): Path<String>,
    Json(value): Json<Value>,
) -> Response {
    let Some(file) = KeptFile::named(&name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    on_blocking(move || file.write(&state.config_dir, value)).await
}

/// Changes the player's own marker file, the one file of the map folder a
/// page may write. A bad request for a marker that is not valid, a
/// conflict when the file no longer holds the marker the change expects.
async fn change_markers(
    State(state): State<WebState>,
    Json(change): Json<MarkerChange>,
) -> Response {
    on_blocking(
        move || match change_user_markers(&map_dir_in(&state.config_dir), &change) {
            Ok(()) => Json(json!({ "changed": true })).into_response(),
            Err(MarkerFault::Invalid) => marker_status(MARKER_INVALID_STATUS),
            Err(MarkerFault::Stale) => marker_status(MARKER_STALE_STATUS),
            Err(MarkerFault::Full) => marker_status(MARKER_FULL_STATUS),
            Err(MarkerFault::Write(error)) => {
                tracing::warn!(%error, "the own marker file was not written");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        },
    )
    .await
}

/// The answer of a change of the own marker file that was not made, by
/// the status the page reads its words from.
fn marker_status(status: u16) -> Response {
    StatusCode::from_u16(status)
        .unwrap_or(StatusCode::BAD_REQUEST)
        .into_response()
}

/// The file names of the player fonts in the `Fonts` folder.
async fn list_fonts(State(state): State<WebState>) -> Response {
    on_blocking(move || Json(font_names(&fonts_dir(&state.config_dir))).into_response()).await
}

/// One player font. Only a name the list gives is read.
async fn font(
    State(state): State<WebState>,
    Path(name): Path<String>,
    request: Request,
) -> Response {
    let dir = fonts_dir(&state.config_dir);
    let listed = {
        let dir = dir.clone();
        tokio::task::spawn_blocking(move || font_names(&dir).contains(&name).then_some(name))
            .await
            .ok()
            .flatten()
    };
    match listed {
        Some(name) => send_file(&dir.join(name), request, None).await,
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Writes a new screenshot file in `folder`, and the folder when there is
/// none. A file `write` did not write whole is taken away again, so no
/// broken picture stays in the folder.
fn write_new_file(
    folder: &std::path::Path,
    write: impl FnOnce(&mut std::fs::File) -> std::io::Result<()>,
) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(folder)?;
    let (path, mut file) = create_new_file(folder)?;
    if let Err(error) = write(&mut file) {
        drop(file);
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    Ok(path)
}

/// Saves a PNG of the page in the `screenshots` folder, as the window
/// saves its own. Gives the name of the file.
async fn save_screenshot(State(state): State<WebState>, picture: Bytes) -> Response {
    if !picture.starts_with(PNG_SIGNATURE) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    on_blocking(move || {
        let folder = screenshots_dir(&state.config_dir);
        let saved = write_new_file(&folder, |file| file.write_all(&picture));
        match saved {
            Ok(path) => {
                let file = path.file_name().map(|name| name.to_string_lossy());
                (StatusCode::CREATED, Json(json!({ "file": file }))).into_response()
            }
            Err(error) => {
                tracing::warn!(%error, folder = %folder.display(), "a screenshot was not saved");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::super::router;
    use super::super::tests::{state_in, temp_folder, TempFolder};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use serde_json::{json, Value};
    use tower::ServiceExt;
    use uoterm_runtime::config::file_safe;
    use uoterm_view::model::world_map::{MapFolder, Marker, MarkerChange, MARKER_CHANGE_PATH};
    use uoterm_view::settings::Profile;

    const PNG_START: &[u8] = b"\x89PNG\r\n\x1a\nrest";

    fn test_config_dir() -> TempFolder {
        temp_folder()
    }

    /// The routes on a config folder of a test, with no client files.
    fn profile_router(config_dir: &std::path::Path) -> Router {
        router(state_in(None, config_dir.to_path_buf()))
    }

    fn put_json(path: &str, value: &impl serde::Serialize) -> Request<Body> {
        Request::put(path)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(value).unwrap()))
            .unwrap()
    }

    fn get(path: &str) -> Request<Body> {
        Request::get(path).body(Body::empty()).unwrap()
    }

    async fn body(answer: axum::response::Response) -> Vec<u8> {
        axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    #[tokio::test]
    async fn a_saved_profile_reads_back_the_same() {
        let home = test_config_dir();
        let app = profile_router(home.path());
        let mut profile = Profile::default();
        profile.video.default_zoom = 1.5;
        let put = app
            .clone()
            .oneshot(put_json("/v1/profiles/127.0.0.1:2593/Mara", &profile))
            .await
            .unwrap();
        assert_eq!(put.status(), StatusCode::NO_CONTENT);
        let get = app
            .oneshot(get("/v1/profiles/127.0.0.1:2593/Mara"))
            .await
            .unwrap();
        let back: Profile = serde_json::from_slice(&body(get).await).unwrap();
        assert_eq!(back, profile);
        assert!(home
            .path()
            .join("profiles")
            .join(file_safe("127.0.0.1:2593"))
            .join("Mara.toml")
            .exists());
    }

    #[tokio::test]
    async fn a_character_with_no_profile_gets_the_default_one() {
        let home = test_config_dir();
        let app = profile_router(home.path());
        let mut profile = Profile::default();
        profile.general.always_run = true;
        let put = app
            .clone()
            .oneshot(put_json("/v1/profiles/default", &profile))
            .await
            .unwrap();
        assert_eq!(put.status(), StatusCode::NO_CONTENT);
        for path in [
            "/v1/profiles/default",
            "/v1/profiles/Play.Example.com:2593/Mara",
        ] {
            let answer = app.clone().oneshot(get(path)).await.unwrap();
            let back: Profile = serde_json::from_slice(&body(answer).await).unwrap();
            assert_eq!(back, profile, "{path}");
        }
    }

    /// The window names a shard by its address in small letters.
    #[tokio::test]
    async fn the_shard_of_a_profile_is_named_as_the_window_names_it() {
        let home = test_config_dir();
        let app = profile_router(home.path());
        let mut profile = Profile::default();
        profile.general.always_run = true;
        let put = app
            .clone()
            .oneshot(put_json(
                "/v1/profiles/Play.Example.com:2593/Mara",
                &profile,
            ))
            .await
            .unwrap();
        assert_eq!(put.status(), StatusCode::NO_CONTENT);
        assert!(home
            .path()
            .join("profiles")
            .join(file_safe("play.example.com:2593"))
            .join("Mara.toml")
            .exists());
        let no_port = app.oneshot(get("/v1/profiles/example/Mara")).await.unwrap();
        assert_eq!(no_port.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn only_known_kept_files_are_served() {
        let home = test_config_dir();
        let app = profile_router(home.path());
        let answer = app.oneshot(get("/v1/kept/uoterm.toml")).await.unwrap();
        assert_eq!(answer.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_kept_file_is_json_to_the_page_and_toml_on_disk() {
        const BAG: u32 = 0x4000_0001;
        let home = test_config_dir();
        let app = profile_router(home.path());
        let empty = app
            .clone()
            .oneshot(get("/v1/kept/watch-grab-bags.toml"))
            .await
            .unwrap();
        assert_eq!(empty.status(), StatusCode::OK);
        let bags = json!({ "characters": { "Mara": BAG } });
        let put = app
            .clone()
            .oneshot(put_json("/v1/kept/watch-grab-bags.toml", &bags))
            .await
            .unwrap();
        assert_eq!(put.status(), StatusCode::NO_CONTENT);
        let text = std::fs::read_to_string(home.path().join("watch-grab-bags.toml")).unwrap();
        assert!(text.contains("Mara"), "{text}");
        let back = app
            .clone()
            .oneshot(get("/v1/kept/watch-grab-bags.toml"))
            .await
            .unwrap();
        let back: Value = serde_json::from_slice(&body(back).await).unwrap();
        assert_eq!(back, bags);
        let wrong = app
            .oneshot(put_json(
                "/v1/kept/watch-hotbar.toml",
                &json!({ "characters": 3 }),
            ))
            .await
            .unwrap();
        assert_eq!(wrong.status(), StatusCode::BAD_REQUEST);
    }

    /// The marker and zone files of the map folder read as one kept file,
    /// which the page may not write.
    #[tokio::test]
    async fn the_map_files_are_read_only_to_the_page() {
        let home = test_config_dir();
        let map = home.path().join("map");
        std::fs::create_dir_all(&map).unwrap();
        std::fs::write(map.join("towns.csv"), "1434,1699,1,Bank\n").unwrap();
        std::fs::write(
            map.join("land.zones.json"),
            r#"{ "MapIndex": 1, "Zones": [ { "Label": "Britain", "Color": "red",
                "Polygon": [[0, 0], [10, 0], [10, 10]] } ] }"#,
        )
        .unwrap();
        let app = profile_router(home.path());
        let answer = app.clone().oneshot(get("/v1/kept/markers")).await.unwrap();
        assert_eq!(answer.status(), StatusCode::OK);
        let folder: MapFolder = serde_json::from_slice(&body(answer).await).unwrap();
        assert_eq!(folder.markers[0].name, "towns");
        assert_eq!(folder.markers[0].markers[0].x, 1434);
        assert_eq!(folder.zones[0].zones[0].label, "Britain");
        let put = app
            .oneshot(put_json("/v1/kept/markers", &json!({})))
            .await
            .unwrap();
        assert_eq!(put.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert!(map.join("towns.csv").exists(), "the files stay");
    }

    fn post_change(change: &MarkerChange) -> Request<Body> {
        Request::post(MARKER_CHANGE_PATH)
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(change).unwrap()))
            .unwrap()
    }

    /// The own marker file changes by operations only, each checked: a
    /// marker that is not valid is a bad request, and a change of a marker
    /// the file no longer holds is a conflict.
    #[tokio::test]
    async fn the_own_marker_file_changes_by_checked_operations() {
        let home = test_config_dir();
        let app = profile_router(home.path());
        let camp = Marker {
            name: "Camp".into(),
            map: 0,
            x: 10,
            y: 20,
            icon: String::new(),
            color: "red".into(),
        };
        let send = |change: MarkerChange| {
            let app = app.clone();
            async move { app.oneshot(post_change(&change)).await.unwrap() }
        };
        let added = send(MarkerChange::Add(camp.clone())).await;
        assert_eq!(added.status(), StatusCode::OK);
        let bad = Marker {
            x: u16::MAX,
            ..camp.clone()
        };
        assert_eq!(
            send(MarkerChange::Add(bad)).await.status(),
            StatusCode::BAD_REQUEST
        );
        let line_break = Marker {
            name: "Camp\n9,9,0,Fake".into(),
            ..camp.clone()
        };
        assert_eq!(
            send(MarkerChange::Add(line_break)).await.status().as_u16(),
            uoterm_view::model::world_map::MARKER_INVALID_STATUS,
            "a name never starts a line of its own"
        );
        let mine = Marker {
            name: "Mine".into(),
            ..camp.clone()
        };
        let stale = MarkerChange::Remove {
            at: 0,
            expected: mine.clone(),
        };
        assert_eq!(
            send(stale).await.status().as_u16(),
            uoterm_view::model::world_map::MARKER_STALE_STATUS
        );
        let keep = MarkerChange::Keep {
            at: 0,
            marker: mine.clone(),
            expected: camp,
        };
        assert_eq!(send(keep).await.status(), StatusCode::OK);
        let read = app.clone().oneshot(get("/v1/kept/markers")).await.unwrap();
        let folder: MapFolder = serde_json::from_slice(&body(read).await).unwrap();
        assert_eq!(folder.markers[0].markers, vec![mine.clone()]);
        let remove = MarkerChange::Remove {
            at: 0,
            expected: mine,
        };
        assert_eq!(send(remove).await.status(), StatusCode::OK);
        let too_big = Request::post(MARKER_CHANGE_PATH)
            .header("content-type", "application/json")
            .body(Body::from(vec![b' '; super::MARKER_CHANGE_MAX_BYTES + 1]))
            .unwrap();
        let answer = app.oneshot(too_big).await.unwrap();
        assert_eq!(answer.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn the_player_fonts_are_listed_and_only_those_are_read() {
        let home = test_config_dir();
        let fonts = home.path().join("Fonts");
        std::fs::create_dir_all(&fonts).unwrap();
        std::fs::write(fonts.join("Avadonian.ttf"), b"font").unwrap();
        std::fs::write(fonts.join("readme.txt"), b"words").unwrap();
        std::fs::write(home.path().join("secret.ttf"), b"secret").unwrap();
        let app = profile_router(home.path());
        let listed = app.clone().oneshot(get("/v1/fonts")).await.unwrap();
        let listed: Value = serde_json::from_slice(&body(listed).await).unwrap();
        assert_eq!(listed, json!(["Avadonian.ttf"]));
        let font = app
            .clone()
            .oneshot(get("/v1/fonts/Avadonian.ttf"))
            .await
            .unwrap();
        assert_eq!(font.status(), StatusCode::OK);
        assert_eq!(body(font).await, b"font");
        for path in [
            "/v1/fonts/readme.txt",
            "/v1/fonts/..%2Fsecret.ttf",
            "/v1/fonts/gone.ttf",
        ] {
            let answer = app.clone().oneshot(get(path)).await.unwrap();
            assert_eq!(answer.status(), StatusCode::NOT_FOUND, "{path}");
        }
    }

    #[tokio::test]
    async fn a_screenshot_must_be_a_png() {
        let home = test_config_dir();
        let app = profile_router(home.path());
        let answer = app
            .oneshot(
                Request::post("/v1/screenshots")
                    .body(Body::from("not a picture"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(answer.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn a_screenshot_is_saved_in_the_screenshots_folder() {
        let home = test_config_dir();
        let app = profile_router(home.path());
        let answer = app
            .oneshot(
                Request::post("/v1/screenshots")
                    .header("content-type", "image/png")
                    .body(Body::from(PNG_START))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(answer.status(), StatusCode::CREATED);
        let saved: Value = serde_json::from_slice(&body(answer).await).unwrap();
        let file = saved["file"].as_str().unwrap();
        assert!(
            file.starts_with("Screenshot_") && file.ends_with(".png"),
            "{file}"
        );
        let written = std::fs::read(home.path().join("screenshots").join(file)).unwrap();
        assert_eq!(written, PNG_START);
    }

    #[test]
    fn a_screenshot_not_written_whole_is_taken_away() {
        let home = test_config_dir();
        let folder = home.path().join("screenshots");
        let failed = super::write_new_file(&folder, |_| {
            Err(std::io::Error::new(std::io::ErrorKind::StorageFull, "full"))
        });
        assert!(failed.is_err());
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn a_screenshot_larger_than_the_limit_is_refused() {
        let home = test_config_dir();
        let app = profile_router(home.path());
        let mut huge = PNG_START.to_vec();
        huge.resize(super::SCREENSHOT_MAX_BYTES + 1, 0);
        let answer = app
            .oneshot(
                Request::post("/v1/screenshots")
                    .body(Body::from(huge))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(answer.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
