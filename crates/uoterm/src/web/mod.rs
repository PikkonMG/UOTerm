//! The routes the web client reads the client files through: each picture
//! as a PNG, each block of the map, and the tables the scene builds with.
//! `uoterm web` merges them with the routes of the runtime API, behind the
//! same guard.

// The `uoterm web` command of a later step of the web client serves these
// routes. Until it does, only the tests call them. The expectation fails
// once it does: then remove it.
#![cfg_attr(not(test), expect(dead_code))]

mod art_routes;
mod data_routes;
mod map_routes;

use crate::art::client_art::ClientArt;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Router;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;
use uoterm_nav::SEASONS_NAME;

/// The browser keeps a picture or a table for a year and never asks again:
/// other client files have another tag, so another ETag.
const KEEP_FOREVER: &str = "public, max-age=31536000, immutable";
const CONTENT_JSON: &str = "application/json";

/// What the routes read: the client files, and the tag of their version.
#[derive(Clone)]
pub struct WebState {
    /// None when there are no client files: every route answers that it is
    /// unavailable. The lock is held only on a blocking thread.
    pub art: Option<Arc<Mutex<ClientArt>>>,
    /// Names the version of the client files in every ETag.
    pub files_tag: String,
}

impl WebState {
    /// The client files of `uopath`, and the season table of the config
    /// folder.
    pub fn open(uopath: Option<&Path>) -> Self {
        let art = uopath.and_then(|path| {
            ClientArt::open(path)
                .inspect_err(|error| tracing::warn!(%error, "the web client has no client files"))
                .ok()
        });
        let config_dir = uoterm_runtime::config::config_dir();
        Self {
            art: art.map(|art| Arc::new(Mutex::new(art))),
            files_tag: uopath
                .map(|path| files_tag(path, &config_dir))
                .unwrap_or_default(),
        }
    }
}

/// Every route that reads the client files.
pub fn router(state: WebState) -> Router {
    Router::new()
        .merge(art_routes::routes())
        .merge(map_routes::routes())
        .merge(data_routes::routes())
        .with_state(state)
}

/// A hex number that changes when a file of the client files changes, or
/// the season table of the config folder: the name and the time each was
/// last changed.
fn files_tag(uopath: &Path, config_dir: &Path) -> String {
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
    let mut hasher = DefaultHasher::new();
    files.hash(&mut hasher);
    seasons.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Runs `read` on the client files on a blocking thread, then `answer` on
/// what it read once the files are unlocked. Unavailable with no client
/// files.
async fn on_art<T: 'static>(
    state: &WebState,
    read: impl FnOnce(&mut ClientArt) -> T + Send + 'static,
    answer: impl FnOnce(T) -> Response + Send + 'static,
) -> Response {
    let Some(art) = state.art.clone() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    tokio::task::spawn_blocking(move || {
        let read = read(&mut art.lock().unwrap_or_else(PoisonError::into_inner));
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::path::PathBuf;
    use tower::ServiceExt;
    use uoterm_nav::fixtures::{write_mini_client, write_two_items};

    /// A folder of client files written by hand, taken away when it drops.
    pub(super) struct FixtureUopath(pub(super) PathBuf);

    impl Drop for FixtureUopath {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Two items of art and a map of two by two blocks with one wall.
    pub(super) fn fixture_uopath() -> FixtureUopath {
        let dir = std::env::temp_dir().join(format!("uoterm-web-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        write_two_items(&dir);
        write_mini_client(&dir);
        FixtureUopath(dir)
    }

    /// The routes on the fixture files. The folder must outlive the state:
    /// the map files open when a block is first asked for.
    pub(super) fn test_state() -> (WebState, FixtureUopath) {
        let uopath = fixture_uopath();
        (WebState::open(Some(&uopath.0)), uopath)
    }

    pub(super) async fn send(state: WebState, request: Request<Body>) -> axum::response::Response {
        router(state).oneshot(request).await.unwrap()
    }

    #[tokio::test]
    async fn with_no_client_files_every_route_is_unavailable() {
        for path in ["/v1/map/0/0/0", "/v1/data/tiledata", "/v1/data/seasons"] {
            let answer = send(
                WebState::open(None),
                Request::get(path).body(Body::empty()).unwrap(),
            )
            .await;
            assert_eq!(answer.status(), StatusCode::SERVICE_UNAVAILABLE, "{path}");
        }
    }

    #[test]
    fn the_files_tag_changes_when_a_file_changes() {
        let uopath = fixture_uopath();
        let config = fixture_uopath();
        let before = files_tag(&uopath.0, &config.0);
        assert_eq!(before, files_tag(&uopath.0, &config.0), "the same files");
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(1);
        let file = std::fs::File::options()
            .write(true)
            .open(uopath.0.join(uoterm_nav::TILEDATA_NAME))
            .unwrap();
        file.set_modified(later).unwrap();
        assert_ne!(before, files_tag(&uopath.0, &config.0));
    }

    #[test]
    fn a_season_table_of_the_shard_changes_the_files_tag() {
        let uopath = fixture_uopath();
        let config = fixture_uopath();
        let before = files_tag(&uopath.0, &config.0);
        std::fs::write(config.0.join(uoterm_nav::SEASONS_NAME), "").unwrap();
        assert_ne!(before, files_tag(&uopath.0, &config.0));
    }
}
