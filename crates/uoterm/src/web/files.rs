//! The page of the web client: the files `npm run build` makes in the web
//! folder. A path that names no file is a path of the page itself, so it
//! gets the page.

use axum::Router;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};

/// The file of the page, in the web folder.
pub(super) const INDEX_FILE: &str = "index.html";

/// The files of `web_dir`, and its page for every other path.
pub fn serve_page(web_dir: PathBuf) -> Router {
    let page = ServeFile::new(web_dir.join(INDEX_FILE));
    Router::new().fallback_service(ServeDir::new(web_dir).fallback(page))
}

#[cfg(test)]
mod tests {
    use super::super::tests::temp_folder;
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    const PAGE: &str = "<!doctype html><title>t</title>";
    const SCRIPT: &str = "console.log(1)";

    async fn text(page: axum::Router, path: &str) -> (StatusCode, String) {
        let answer = page
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = answer.status();
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn the_page_and_its_files_come_from_the_web_folder() {
        let web = temp_folder();
        std::fs::create_dir_all(web.path().join("assets")).unwrap();
        std::fs::write(web.path().join(INDEX_FILE), PAGE).unwrap();
        std::fs::write(web.path().join("assets/app.js"), SCRIPT).unwrap();
        let page = serve_page(web.path().to_path_buf());
        assert_eq!(text(page.clone(), "/").await, (StatusCode::OK, PAGE.into()));
        assert_eq!(
            text(page.clone(), "/assets/app.js").await,
            (StatusCode::OK, SCRIPT.into())
        );
        assert_eq!(
            text(page, "/play/s1").await,
            (StatusCode::OK, PAGE.into()),
            "a path of the page itself"
        );
    }
}
