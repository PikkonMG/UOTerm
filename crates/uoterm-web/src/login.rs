//! The rules of the screens before the game, for the page: the words of
//! the login screens, the checks of the form, the names of a saved login,
//! and whether the account has room for a new character. They are the
//! rules of the window's login screens.

use crate::to_js;
use uoterm_view::model::creation::can_make;
use uoterm_view::model::login::{login_fault, save_name, saved_detail, LOGIN_WORDS};
use wasm_bindgen::prelude::*;

/// The words of the login screens: `LoginWords`.
#[wasm_bindgen(js_name = loginWords)]
pub fn login_words() -> JsValue {
    to_js(&LOGIN_WORDS)
}

/// Why a form cannot log in, or undefined when it can. The page has no
/// password variables, so a login needs a typed password; a form saved,
/// not logged in with, gives none (undefined).
#[wasm_bindgen(js_name = loginFault)]
pub fn login_fault_js(
    host: &str,
    port: &str,
    account: &str,
    password: Option<String>,
) -> Option<String> {
    login_fault(host, port, account, password.as_deref()).map(str::to_string)
}

/// The name "Save login" offers: account@host.
#[wasm_bindgen(js_name = saveName)]
pub fn save_name_js(account: &str, host: &str) -> String {
    save_name(account, host)
}

/// The line under the name of a saved login: "account @ host:port".
#[wasm_bindgen(js_name = savedDetail)]
pub fn saved_detail_js(account: &str, host: &str, port: u16) -> String {
    saved_detail(account, host, port)
}

/// True when an account with the characters of `names_json` (a JSON list
/// of names, empty for a free slot) and the flags of its list may make one
/// more.
#[wasm_bindgen(js_name = canMake)]
pub fn can_make_js(names_json: &str, list_flags: u32) -> bool {
    let names: Vec<String> = serde_json::from_str(names_json).unwrap_or_default();
    can_make(&names, list_flags)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_checks_the_form_and_the_room_by_the_rules_of_the_window() {
        let fault = |password: Option<&str>| {
            login_fault_js("h", "2593", "mara", password.map(str::to_string))
        };
        assert_eq!(fault(Some("")).as_deref(), Some("Type the password."));
        assert_eq!(fault(Some("pw")), None);
        assert_eq!(fault(None), None);
        assert_eq!(save_name_js("mara", "h"), "mara@h");
        assert_eq!(saved_detail_js("mara", "h", 2593), "mara @ h:2593");
        assert!(can_make_js(r#"["Mara",""]"#, 0));
        assert!(!can_make_js(r#"["A","B","C","D","E"]"#, 0));
    }
}
