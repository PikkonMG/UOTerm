//! Plain orders and wishes from the human, sent to TypeSafe's Jev model.
//! The questions and the reading of the answers are
//! `uoterm_view::orders`; this sends them, for the window and for the web
//! client, so the key never leaves UOTerm.
//!
//! The order and the names of the things near go to the TypeSafe service.
//! That happens only when the operator gave a key. Without a key the order
//! box is off, and the rest of the window works the same.

use serde_json::Value;
use uoterm_view::act::Act;
use uoterm_view::frame::WatchFrame;
use uoterm_view::orders::{
    decide, group_request, hotkey_choices, hotkey_groups, hotkey_lines, pick_request, picked,
    picked_group, request, ASK_HOTKEY, NO_SUCH_HOTKEY,
};

pub use uoterm_view::orders::{
    ASK_CHANNEL, ASK_HOUSE_PART, ASK_LANDMARK, ASK_PROFILE, ASK_SHARD, ASK_WEAR,
};

const KEY_ENV: &str = "TYPESAFE_API_KEY";
/// The words for the human when there is no key.
pub const ORDER_OFF: &str = "Orders need a TypeSafe key. Put TYPESAFE_API_KEY in the environment.";
const ENV_FILE: &str = ".env";
const API_URL: &str = "https://api.typesafe.ai/v1/systemone";

/// The key from the environment, or from a `.env` file in this directory.
pub fn api_key() -> Option<String> {
    let from_file = || {
        let text = std::fs::read_to_string(ENV_FILE).ok()?;
        text.lines().find_map(|line| {
            let value = line
                .trim()
                .strip_prefix(KEY_ENV)?
                .trim()
                .strip_prefix('=')?;
            Some(value.trim().trim_matches(['"', '\'']).to_string())
        })
    };
    std::env::var(KEY_ENV)
        .ok()
        .or_else(from_file)
        .filter(|key| !key.is_empty())
}

/// Sends one request to Jev. The error is words for the human.
async fn post(key: &str, request: &Value) -> Result<Value, String> {
    let response = reqwest::Client::new()
        .post(API_URL)
        .bearer_auth(key)
        .json(request)
        .send()
        .await
        .map_err(|e| format!("TypeSafe did not answer: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("TypeSafe refused the request: HTTP {status}"));
    }
    response
        .json()
        .await
        .map_err(|e| format!("TypeSafe gave a bad answer: {e}"))
}

/// Asks Jev about one order. The error is words for the human.
pub async fn ask(key: &str, order: &str, frame: &WatchFrame) -> Result<Act, String> {
    decide(&post(key, &request(order, frame)).await?, frame)
}

/// Asks Jev which of `options` the wish names. None when Jev is not sure,
/// or when the wish names none of them.
pub async fn pick(
    key: &str,
    instructions: &str,
    wish: &str,
    options: &[&str],
) -> Result<Option<usize>, String> {
    let response = post(key, &pick_request(instructions, wish, options)).await?;
    Ok(picked(&response, options.len()))
}

/// The script lines for a wish in plain words. `hotkeys` asks the session:
/// with no name for the groups, and with a name for the lines of one hotkey.
pub async fn lines_for<F, Fut>(key: &str, wish: &str, hotkeys: F) -> Result<String, String>
where
    F: Fn(Option<String>) -> Fut,
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    let groups = hotkey_groups(&hotkeys(None).await?);
    let group_answer = post(key, &group_request(wish, &groups)).await?;
    let names = picked_group(&group_answer, &groups).ok_or(NO_SUCH_HOTKEY)?;
    let choices = hotkey_choices(wish, names);
    let place = pick(key, ASK_HOTKEY, wish, &choices)
        .await?
        .ok_or(NO_SUCH_HOTKEY)?;
    let name = choices[place];
    hotkey_lines(&hotkeys(Some(name.to_string())).await?)
}
