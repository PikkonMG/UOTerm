//! The routes of `uoterm web` that are not the runtime API: the pictures,
//! the map and the tables of the client files, the sounds and the music,
//! the files of the config folder (profiles, kept files, fonts,
//! screenshots), the questions to Jev, and the page itself. [`app`] merges
//! them with the routes of the runtime API, behind the same guard.

mod art_routes;
mod data_routes;
mod files;
mod jev_routes;
mod map_routes;
mod profile_routes;
mod sound_routes;

pub use files::serve_page;

use crate::art::client_art::ClientArt;
use axum::body::Body;
use axum::extract::Request;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG};
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, PoisonError, RwLock};
use std::time::SystemTime;
use tokio::sync::Semaphore;
use tower_http::services::ServeFile;
use uoterm_nav::{MusicList, SoundData, SEASONS_NAME};
use uoterm_runtime::api::{guard_layer, not_found, router_for, Guard};
use uoterm_runtime::{AppConfig, Runtime};

/// The browser keeps a picture or a table for a year and never asks again:
/// other client files, or another build of UOTerm, have another tag, so
/// another ETag.
const KEEP_FOREVER: &str = "public, max-age=31536000, immutable";
const CONTENT_JSON: &str = "application/json";
/// How many pictures are made and turned into PNG at one time. Each can
/// take tens of megabytes while it is made.
const PICTURES_AT_ONCE: usize = 4;

/// What the routes read: the client files and the tag of their version,
/// the config folder, the sessions, and the key of TypeSafe.
#[derive(Clone)]
pub struct WebState {
    /// None when there are no client files: every route answers that it is
    /// unavailable. The lock is held only on a blocking thread: pictures
    /// and tables share it, a map block or a light shape takes it alone.
    pub art: Option<Arc<RwLock<ClientArt>>>,
    /// Names the version of the client files and of UOTerm in every ETag.
    pub files_tag: String,
    /// One permit for each picture made at one time.
    pub pictures: Arc<Semaphore>,
    /// None when the client files hold no sounds.
    pub sounds: Option<Arc<SoundData>>,
    /// None when the client files hold no music.
    pub music: Option<Arc<MusicList>>,
    /// Where the profiles, the kept files, the fonts and the screenshots
    /// are. `uoterm web` gives the config folder of UOTerm; a test gives
    /// a folder of its own.
    pub config_dir: PathBuf,
    /// The sessions the questions to Jev ask about.
    pub runtime: Runtime,
    /// The key of TypeSafe. None turns the questions to Jev off. It never
    /// goes to the browser.
    pub jev_key: Option<String>,
}

impl WebState {
    /// The client files of `uopath`, and the season table of the config
    /// folder. The tag also names this build of UOTerm by the time its
    /// program file was last changed. It reads files, so it runs before
    /// the server does or on a blocking thread.
    pub fn open(
        uopath: Option<&Path>,
        config_dir: PathBuf,
        runtime: Runtime,
        jev_key: Option<String>,
    ) -> Self {
        let art = uopath.and_then(|path| {
            ClientArt::open(path)
                .inspect_err(|error| tracing::warn!(%error, "the web client has no client files"))
                .ok()
        });
        let program = std::env::current_exe().unwrap_or_default();
        Self {
            art: art.map(|art| Arc::new(RwLock::new(art))),
            files_tag: uopath
                .map(|path| files_tag(path, &config_dir, &program))
                .unwrap_or_default(),
            pictures: Arc::new(Semaphore::new(PICTURES_AT_ONCE)),
            sounds: uopath
                .and_then(|path| SoundData::open(path).ok())
                .map(Arc::new),
            music: uopath
                .and_then(|path| MusicList::open(path).ok())
                .map(Arc::new),
            config_dir,
            runtime,
            jev_key,
        }
    }
}

/// Every path of the API. A path no route has is not found, whatever the
/// method, so it never gets the page.
const API_PATHS: &str = "/v1/{*rest}";

/// Every route of this module, and not found for any other path of the
/// API.
pub fn router(state: WebState) -> Router {
    Router::new()
        .merge(art_routes::routes())
        .merge(map_routes::routes())
        .merge(data_routes::routes())
        .merge(sound_routes::routes())
        .merge(profile_routes::routes())
        .merge(jev_routes::routes())
        .route(API_PATHS, any(|| async { not_found() }))
        .with_state(state)
}

/// The whole server of `uoterm web`: the runtime API on `runtime`, the
/// routes of `state`, and the page of `web_dir`, each behind `guard`. The
/// page itself needs no token, so a page on another machine can ask the
/// player for it; the loopback rule and the origin rule hold for it too.
pub fn app(
    runtime: Runtime,
    config: AppConfig,
    state: WebState,
    web_dir: PathBuf,
    guard: Guard,
) -> Router {
    let page_guard = Guard {
        token: None,
        local_only: guard.local_only,
    };
    router_for(runtime, config, guard.token.clone(), guard.local_only)
        .merge(guard_layer(router(state), guard))
        .merge(guard_layer(serve_page(web_dir), page_guard))
}

/// The version of UOTerm and a hex number that changes when a file of the
/// client files changes, the season table of the config folder, or the
/// program file of UOTerm: the name and the time each was last changed.
fn files_tag(uopath: &Path, config_dir: &Path, program: &Path) -> String {
    let changed = |path: &Path| {
        std::fs::metadata(path)
            .ok()
            .filter(std::fs::Metadata::is_file)
            .map(|meta| meta.modified().ok())
    };
    let mut files: Vec<_> = std::fs::read_dir(uopath)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| Some((entry.file_name(), changed(&entry.path())?)))
        .collect();
    files.sort();
    let seasons: Option<Option<SystemTime>> = changed(&config_dir.join(SEASONS_NAME));
    let build: Option<Option<SystemTime>> = changed(program);
    let mut hasher = DefaultHasher::new();
    files.hash(&mut hasher);
    seasons.hash(&mut hasher);
    build.hash(&mut hasher);
    format!("{}-{:x}", env!("CARGO_PKG_VERSION"), hasher.finish())
}

/// Runs `read` with a shared borrow of the client files on a blocking
/// thread, then `answer` on what it read once the files are unlocked.
async fn on_art<T: 'static>(
    state: &WebState,
    read: impl FnOnce(&ClientArt) -> T + Send + 'static,
    answer: impl FnOnce(T) -> Response + Send + 'static,
) -> Response {
    on_locked_art(
        state,
        |art| read(&art.read().unwrap_or_else(PoisonError::into_inner)),
        answer,
    )
    .await
}

/// [`on_art`] for a read that changes the client art: a map block or a
/// light shape read for the first time.
async fn on_art_mut<T: 'static>(
    state: &WebState,
    read: impl FnOnce(&mut ClientArt) -> T + Send + 'static,
    answer: impl FnOnce(T) -> Response + Send + 'static,
) -> Response {
    on_locked_art(
        state,
        |art| read(&mut art.write().unwrap_or_else(PoisonError::into_inner)),
        answer,
    )
    .await
}

/// Runs `read` on the lock of the client files on a blocking thread, then
/// `answer` on what it read. Unavailable with no client files.
async fn on_locked_art<T: 'static>(
    state: &WebState,
    read: impl FnOnce(&RwLock<ClientArt>) -> T + Send + 'static,
    answer: impl FnOnce(T) -> Response + Send + 'static,
) -> Response {
    let Some(art) = state.art.clone() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    tokio::task::spawn_blocking(move || {
        let read = read(&art);
        answer(read)
    })
    .await
    .unwrap_or_else(|error| {
        tracing::warn!(%error, "a web route failed");
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    })
}

/// An answer the browser keeps for as long as `etag` names it.
fn kept(etag: &str, content_type: &str, body: Vec<u8>) -> Response {
    (
        [
            (CONTENT_TYPE, content_type.to_string()),
            (CACHE_CONTROL, KEEP_FOREVER.to_string()),
            (ETAG, format!("\"{etag}\"")),
        ],
        body,
    )
        .into_response()
}

/// A table as JSON the browser keeps for this version of the client files.
/// Not found when the client files do not hold it.
fn table<T: Serialize>(value: Option<T>, files_tag: &str) -> Response {
    let Some(value) = value else {
        return StatusCode::NOT_FOUND.into_response();
    };
    match serde_json::to_vec(&value) {
        Ok(body) => kept(files_tag, CONTENT_JSON, body),
        Err(error) => {
            tracing::warn!(%error, "a table did not turn into JSON");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// Runs `work` on a blocking thread and gives its answer: for the files
/// of the config folder and the sounds, which are read from disk.
async fn on_blocking(work: impl FnOnce() -> Response + Send + 'static) -> Response {
    tokio::task::spawn_blocking(work)
        .await
        .unwrap_or_else(|error| {
            tracing::warn!(%error, "a web route failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        })
}

/// The file at `path` as the answer to `request`, read a part at a time,
/// with the content type `content_type` when one is given.
async fn send_file(path: &Path, request: Request, content_type: Option<&'static str>) -> Response {
    let mut answer = match ServeFile::new(path).try_call(request).await {
        Ok(answer) => answer.map(Body::new),
        Err(error) => {
            tracing::warn!(%error, path = %path.display(), "a file was not sent");
            return StatusCode::NOT_FOUND.into_response();
        }
    };
    if let Some(content_type) = content_type.filter(|_| answer.status().is_success()) {
        answer
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static(content_type));
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::path::PathBuf;
    use tower::ServiceExt;
    use uoterm_nav::fixtures::{
        write_mini_client, write_one_gump, write_one_sound, write_two_items,
    };
    use uoterm_runtime::api::Guard;

    /// The sessions a test runtime may hold.
    const TEST_SESSIONS: usize = 1;
    /// The config folder of a test, inside its folder of client files.
    const TEST_CONFIG: &str = "config";

    /// A folder of a test, taken away when it drops.
    pub(super) struct TempFolder(pub(super) PathBuf);

    impl TempFolder {
        pub(super) fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempFolder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A new empty folder.
    pub(super) fn temp_folder() -> TempFolder {
        let dir = std::env::temp_dir().join(format!("uoterm-web-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        TempFolder(dir)
    }

    /// Two items of art, a map of two by two blocks with one wall, one
    /// sound and one gump.
    pub(super) fn fixture_uopath() -> TempFolder {
        let folder = temp_folder();
        write_two_items(folder.path());
        write_mini_client(folder.path());
        write_one_sound(folder.path());
        write_one_gump(folder.path());
        folder
    }

    /// The routes on client files and a config folder, with no TypeSafe
    /// key.
    pub(super) fn state_in(uopath: Option<&Path>, config_dir: PathBuf) -> WebState {
        WebState::open(uopath, config_dir, Runtime::new(TEST_SESSIONS), None)
    }

    /// The routes on the fixture files. The folder must outlive the state:
    /// the map files open when a block is first asked for.
    pub(super) fn test_state() -> (WebState, TempFolder) {
        let uopath = fixture_uopath();
        let state = state_in(Some(uopath.path()), uopath.path().join(TEST_CONFIG));
        (state, uopath)
    }

    pub(super) async fn send(state: WebState, request: Request<Body>) -> axum::response::Response {
        router(state).oneshot(request).await.unwrap()
    }

    /// With a token, the page loads with none, so it can ask for the
    /// token; every route of the API and of the client files wants it.
    #[tokio::test]
    async fn the_page_needs_no_token_and_every_route_does() {
        const TOKEN: &str = "secret";
        let web = temp_folder();
        std::fs::write(web.path().join(files::INDEX_FILE), "page").unwrap();
        let config = temp_folder();
        let guard = Guard {
            token: Some(TOKEN.into()),
            local_only: true,
        };
        let state = state_in(None, config.path().to_path_buf());
        let app = app(
            state.runtime.clone(),
            uoterm_runtime::AppConfig::default(),
            state,
            web.path().to_path_buf(),
            guard,
        );
        let call = |path: &str, token: Option<&str>| {
            let mut request = Request::get(path).header("host", "127.0.0.1:7733");
            if let Some(token) = token {
                request = request.header("authorization", format!("Bearer {token}"));
            }
            app.clone().oneshot(request.body(Body::empty()).unwrap())
        };
        assert_eq!(call("/", None).await.unwrap().status(), StatusCode::OK);
        assert_eq!(
            call("/health", None).await.unwrap().status(),
            StatusCode::OK
        );
        for path in ["/v1/fonts", "/v1/sessions", "/v1/profiles/default"] {
            let refused = call(path, None).await.unwrap();
            assert_eq!(refused.status(), StatusCode::UNAUTHORIZED, "{path}");
            let answered = call(path, Some(TOKEN)).await.unwrap();
            assert_eq!(answered.status(), StatusCode::OK, "{path}");
        }
        let elsewhere = app
            .clone()
            .oneshot(
                Request::get("/")
                    .header("host", "example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            elsewhere.status(),
            StatusCode::FORBIDDEN,
            "the loopback rule"
        );
        let other_site = app
            .oneshot(
                Request::get("/v1/fonts")
                    .header("host", "127.0.0.1:7733")
                    .header("origin", "http://example.com")
                    .header("authorization", format!("Bearer {TOKEN}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            other_site.status(),
            StatusCode::FORBIDDEN,
            "the origin rule"
        );
    }

    /// A wrong path of the API never looks like a page that answered.
    #[tokio::test]
    async fn an_unknown_api_path_is_not_found_and_other_paths_are_the_page() {
        const PAGE: &str = "page";
        let web = temp_folder();
        std::fs::write(web.path().join(files::INDEX_FILE), PAGE).unwrap();
        let config = temp_folder();
        let state = state_in(None, config.path().to_path_buf());
        let guard = Guard {
            token: None,
            local_only: true,
        };
        let app = app(
            state.runtime.clone(),
            uoterm_runtime::AppConfig::default(),
            state,
            web.path().to_path_buf(),
            guard,
        );
        let call = |method: &str, path: &str| {
            let request = Request::builder()
                .method(method)
                .uri(path)
                .header("host", "127.0.0.1:7733")
                .body(Body::empty())
                .unwrap();
            app.clone().oneshot(request)
        };
        let text = |answer: axum::response::Response| async move {
            let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
                .await
                .unwrap();
            String::from_utf8(bytes.to_vec()).unwrap()
        };
        for (method, path) in [
            ("GET", "/v1/typo"),
            ("POST", "/v1/typo"),
            ("GET", "/v1/sessions/s1/nothing"),
            ("DELETE", "/v1/map/0/0/0/0"),
        ] {
            let answer = call(method, path).await.unwrap();
            assert_eq!(answer.status(), StatusCode::NOT_FOUND, "{method} {path}");
            let body: serde_json::Value = serde_json::from_str(&text(answer).await).unwrap();
            assert_eq!(body, serde_json::json!({ "error": "not found" }), "{path}");
        }
        let page = call("GET", "/play/x").await.unwrap();
        assert_eq!(page.status(), StatusCode::OK);
        assert_eq!(text(page).await, PAGE);
        let wrong_method = call("GET", "/v1/art").await.unwrap();
        assert_eq!(wrong_method.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn with_no_client_files_every_route_is_unavailable() {
        for path in ["/v1/map/0/0/0", "/v1/data/tiledata", "/v1/data/seasons"] {
            let config = temp_folder();
            let answer = send(
                state_in(None, config.path().to_path_buf()),
                Request::get(path).body(Body::empty()).unwrap(),
            )
            .await;
            assert_eq!(answer.status(), StatusCode::SERVICE_UNAVAILABLE, "{path}");
        }
    }

    /// Marks a file as changed a second from now.
    fn touch(path: &std::path::Path) {
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(1);
        let file = std::fs::File::options().write(true).open(path).unwrap();
        file.set_modified(later).unwrap();
    }

    #[test]
    fn the_files_tag_changes_when_a_file_changes() {
        let uopath = fixture_uopath();
        let config = fixture_uopath();
        let program_folder = fixture_uopath();
        let program = program_folder.0.join(uoterm_nav::TILEDATA_NAME);
        let tag = || files_tag(&uopath.0, &config.0, &program);
        let before = tag();
        assert_eq!(before, tag(), "the same files");
        touch(&uopath.0.join(uoterm_nav::TILEDATA_NAME));
        assert_ne!(before, tag());
    }

    #[test]
    fn a_season_table_of_the_shard_changes_the_files_tag() {
        let uopath = fixture_uopath();
        let config = fixture_uopath();
        let program = uopath.0.join(uoterm_nav::TILEDATA_NAME);
        let before = files_tag(&uopath.0, &config.0, &program);
        std::fs::write(config.0.join(uoterm_nav::SEASONS_NAME), "").unwrap();
        assert_ne!(before, files_tag(&uopath.0, &config.0, &program));
    }

    /// A browser that kept answers of an older UOTerm asks again.
    #[test]
    fn the_files_tag_names_the_build_of_uoterm() {
        let uopath = fixture_uopath();
        let config = fixture_uopath();
        let program_folder = fixture_uopath();
        let program = program_folder.0.join(uoterm_nav::TILEDATA_NAME);
        let before = files_tag(&uopath.0, &config.0, &program);
        assert!(before.starts_with(env!("CARGO_PKG_VERSION")), "{before}");
        touch(&program);
        assert_ne!(before, files_tag(&uopath.0, &config.0, &program));
    }
}
