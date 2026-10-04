//! The files of the config folder a web page reads and writes: the
//! profiles of the options, the other kept files, the player fonts, and the
//! screenshots. They are the same files the window reads, so a player has
//! the same options in both.

use super::{on_blocking, send_file, WebState};
use crate::kept;
use crate::window::fonts::{font_names, fonts_dir};
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
use uoterm_view::settings::Profile;
use uoterm_view::ui::deck::{KeptHotbars, HOTBAR_FILE};

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

/// The kept files a web page may read and write, besides the profiles.
#[derive(Clone, Copy)]
enum KeptFile {
    Hotbars,
    GrabBags,
}

impl KeptFile {
    fn named(name: &str) -> Option<Self> {
        match name {
            HOTBAR_FILE => Some(Self::Hotbars),
            GRAB_BAGS_FILE => Some(Self::GrabBags),
            _ => None,
        }
    }

    fn file(self) -> &'static str {
        match self {
            Self::Hotbars => HOTBAR_FILE,
            Self::GrabBags => GRAB_BAGS_FILE,
        }
    }

    /// The file as JSON. A missing or bad file gives the default.
    fn read(self, path: &std::path::Path) -> Value {
        match self {
            Self::Hotbars => as_json(kept::load_from::<KeptHotbars>(path)),
            Self::GrabBags => as_json(kept::load_from::<KeptGrabBags>(path)),
        }
    }

    /// Writes the JSON of a page as the file. A bad request when the JSON
    /// is not what the file holds.
    fn write(self, path: &std::path::Path, value: Value) -> Response {
        match self {
            Self::Hotbars => write_as::<KeptHotbars>(path, value),
            Self::GrabBags => write_as::<KeptGrabBags>(path, value),
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
    on_blocking(move || Json(file.read(&state.config_dir.join(file.file()))).into_response()).await
}

async fn write_kept(
    State(state): State<WebState>,
    Path(name): Path<String>,
    Json(value): Json<Value>,
) -> Response {
    let Some(file) = KeptFile::named(&name) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    on_blocking(move || file.write(&state.config_dir.join(file.file()), value)).await
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

/// Saves a PNG of the page in the `screenshots` folder, as the window
/// saves its own. Gives the name of the file.
async fn save_screenshot(State(state): State<WebState>, picture: Bytes) -> Response {
    if !picture.starts_with(PNG_SIGNATURE) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    on_blocking(move || {
        let folder = screenshots_dir(&state.config_dir);
        let saved = std::fs::create_dir_all(&folder)
            .and_then(|()| create_new_file(&folder))
            .and_then(|(path, mut file)| file.write_all(&picture).map(|()| path));
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
