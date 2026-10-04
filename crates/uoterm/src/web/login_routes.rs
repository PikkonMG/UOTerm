//! The saved logins a web page lists and saves: the same files the login
//! screen of `uoterm play` reads and writes. A saved login never holds a
//! password, and no route takes or gives one.

use super::{on_blocking, refused, WebState};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uoterm_protocol::crypto::EncryptionMode;
use uoterm_runtime::{LoginStore, LoginTarget, Profile, StoredLogin};
use uoterm_view::model::login::{account_name, host_name, NEEDS_NAME};

pub(super) fn routes() -> Router<WebState> {
    Router::new()
        .route("/v1/logins", get(list))
        .route("/v1/logins/{name}", put(save))
}

/// One saved login as the page lists it. The host, the port and the
/// encryption are the saved login's, else those of the config file.
#[derive(Debug, PartialEq, Eq, Serialize)]
struct LoginRow {
    name: String,
    host: String,
    port: u16,
    account: String,
    shard: String,
    character: String,
    encryption: EncryptionMode,
    /// The era and the version the login speaks; None: those of the config.
    era: Option<String>,
    version: Option<String>,
}

/// The saved logins, and where a login with none goes: the server of the
/// config file.
#[derive(Debug, Serialize)]
struct Logins {
    host: String,
    port: u16,
    logins: Vec<LoginRow>,
}

/// A login form as a page saves it. It has no password field: a body that
/// names one is refused.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedForm {
    host: String,
    port: u16,
    account: String,
    #[serde(default)]
    shard: String,
    #[serde(default)]
    character: String,
    #[serde(default)]
    encryption: EncryptionMode,
}

impl SavedForm {
    /// The form as a saved login, or the words that refuse it.
    fn profile(self) -> Result<Profile, &'static str> {
        let shard = self.shard.trim();
        Ok(Profile {
            account: account_name(&self.account)?.to_string(),
            host: Some(host_name(&self.host)?.to_string()),
            port: Some(self.port),
            encryption: Some(self.encryption),
            password_env: None,
            character: self.character.trim().to_string(),
            shard: (!shard.is_empty()).then(|| shard.to_string()),
            version: None,
            era: None,
        })
    }
}

fn row(stored: &StoredLogin, state: &WebState) -> LoginRow {
    let profile = &stored.profile;
    let target = LoginTarget::choose(None, None, None, Some(profile), &state.login_config);
    LoginRow {
        name: stored.name.clone(),
        host: target.host,
        port: target.port,
        account: profile.account.clone(),
        shard: profile.shard.clone().unwrap_or_default(),
        character: profile.character.clone(),
        encryption: target.encryption,
        era: profile.era.clone(),
        version: profile.version.clone(),
    }
}

fn logins(store: &LoginStore, state: &WebState) -> Logins {
    let blank = LoginTarget::choose(None, None, None, None, &state.login_config);
    Logins {
        host: blank.host,
        port: blank.port,
        logins: store
            .list()
            .iter()
            .map(|stored| row(stored, state))
            .collect(),
    }
}

async fn list(State(state): State<WebState>) -> Response {
    on_blocking(move || Json(logins(&state.logins, &state)).into_response()).await
}

/// Saves the form under `name`, over a saved login of that name, as the
/// login screen of `uoterm play` saves it. Gives the list after.
async fn save(
    State(state): State<WebState>,
    Path(name): Path<String>,
    Json(form): Json<SavedForm>,
) -> Response {
    let name = name.trim().to_string();
    if name.is_empty() {
        return refused(StatusCode::BAD_REQUEST, NEEDS_NAME);
    }
    let profile = match form.profile() {
        Ok(profile) => profile,
        Err(words) => return refused(StatusCode::BAD_REQUEST, words),
    };
    on_blocking(move || match state.logins.save_over(&name, profile) {
        Ok(_) => Json(logins(&state.logins, &state)).into_response(),
        Err(error) => refused(StatusCode::INTERNAL_SERVER_ERROR, &error.to_string()),
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::super::tests::{send, state_in, temp_folder};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::{json, Value};
    use uoterm_runtime::config::LOGINS_DIR;
    use uoterm_runtime::{AppConfig, LoginStore, Profile};

    const SECRET: &str = "hunter2";

    fn put_json(path: &str, value: &Value) -> Request<Body> {
        Request::put(path)
            .header("content-type", "application/json")
            .body(Body::from(value.to_string()))
            .unwrap()
    }

    async fn json_of(answer: axum::response::Response) -> Value {
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn form() -> Value {
        json!({"host": " 10.0.0.7 ", "port": 2593, "account": "mara", "shard": "",
            "character": "Mara", "encryption": "osi"})
    }

    #[tokio::test]
    async fn a_saved_login_lists_with_the_server_of_the_config_and_no_password() {
        let home = temp_folder();
        let store = LoginStore::at(home.path().join(LOGINS_DIR), home.path().join("old"));
        let cedric = Profile {
            account: "acct2".into(),
            password_env: Some("CEDRIC_PASS".into()),
            character: "Cedric".into(),
            shard: Some("Britannia".into()),
            era: Some("t2a".into()),
            ..Profile::default()
        };
        store.save("cedric", &cedric).unwrap();
        let config = AppConfig {
            host: "play.example.com".into(),
            port: 2594,
            ..AppConfig::default()
        };
        let state = state_in(None, home.path().to_path_buf()).with_login_config(config);
        let answer = send(
            state,
            Request::get("/v1/logins").body(Body::empty()).unwrap(),
        )
        .await;
        assert_eq!(answer.status(), StatusCode::OK);
        let listed = json_of(answer).await;
        assert_eq!(
            listed,
            json!({"host": "play.example.com", "port": 2594, "logins": [{
                "name": "cedric", "host": "play.example.com", "port": 2594, "account": "acct2",
                "shard": "Britannia", "character": "Cedric", "encryption": "none",
                "era": "t2a", "version": null}]})
        );
        assert!(!listed.to_string().contains("CEDRIC_PASS"));
    }

    #[tokio::test]
    async fn a_save_writes_the_saved_login_play_reads_and_gives_the_list() {
        let home = temp_folder();
        let state = state_in(None, home.path().to_path_buf());
        let answer = send(state, put_json("/v1/logins/mara@10.0.0.7", &form())).await;
        assert_eq!(answer.status(), StatusCode::OK);
        let listed = json_of(answer).await;
        assert_eq!(listed["logins"][0]["name"], "mara@10.0.0.7");
        assert_eq!(listed["logins"][0]["host"], "10.0.0.7");
        let store = LoginStore::at(home.path().join(LOGINS_DIR), home.path().join("old"));
        let saved = store.load("mara@10.0.0.7").unwrap();
        assert_eq!(saved.account, "mara");
        assert_eq!(saved.shard, None, "a blank shard is none");
        assert_eq!(saved.encryption, Some(uoterm_runtime::EncryptionMode::Osi));
    }

    #[tokio::test]
    async fn a_save_with_a_password_or_with_no_account_or_name_is_refused() {
        let home = temp_folder();
        let state = state_in(None, home.path().to_path_buf());
        let mut with_password = form();
        with_password["password"] = json!(SECRET);
        let answer = send(state.clone(), put_json("/v1/logins/mara", &with_password)).await;
        assert_eq!(answer.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let mut no_account = form();
        no_account["account"] = json!(" ");
        let answer = send(state.clone(), put_json("/v1/logins/mara", &no_account)).await;
        assert_eq!(answer.status(), StatusCode::BAD_REQUEST);
        assert_eq!(json_of(answer).await["error"], "Type the account.");
        let answer = send(state, put_json("/v1/logins/%20", &form())).await;
        assert_eq!(answer.status(), StatusCode::BAD_REQUEST);
        assert!(!home.path().join(LOGINS_DIR).exists(), "nothing was saved");
    }
}
