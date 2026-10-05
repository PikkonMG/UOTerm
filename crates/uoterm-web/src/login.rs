//! The rules of the screens before the game, for the page: the words of
//! the login screens, the checks of the form, the names of a saved login,
//! whether the account has room for a new character, and the Delete that
//! asks once more. They are the rules of the window's login screens.

use crate::to_js;
use uoterm_view::model::login::{
    login_fault, no_room_note, save_name, saved_detail, CharacterList, LOGIN_WORDS,
};
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

/// The note New character gives when the account of `names_json` (a JSON
/// list of names, empty for a free slot) has no room for one more, or
/// undefined when the making may begin.
#[wasm_bindgen(js_name = noRoomNote)]
pub fn no_room_note_js(names_json: &str, list_flags: u32) -> Option<String> {
    let names: Vec<String> = serde_json::from_str(names_json).unwrap_or_default();
    no_room_note(&names, list_flags).map(str::to_string)
}

/// The character list of the page: the Delete that asks once more. The
/// page makes a new one for each list the shard sends.
#[wasm_bindgen(js_name = CharacterList)]
#[derive(Default)]
pub struct CharacterListJs(CharacterList);

#[wasm_bindgen(js_class = CharacterList)]
impl CharacterListJs {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// A press on the Delete of a slot. True when the page sends the
    /// delete now; false when the press only asked.
    #[wasm_bindgen(js_name = pressDelete)]
    pub fn press_delete(&mut self, slot: usize) -> bool {
        self.0.press_delete(slot)
    }

    /// The words of the Delete button of a slot.
    #[wasm_bindgen(js_name = deleteWords)]
    pub fn delete_words(&self, slot: usize) -> String {
        self.0.delete_words(slot).to_string()
    }

    /// The slot whose Delete was pressed once, or undefined.
    #[wasm_bindgen(js_name = deleteAsked)]
    pub fn delete_asked(&self) -> Option<usize> {
        self.0.delete_asked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_checks_the_form_by_the_rules_of_the_window() {
        let fault = |password: Option<&str>| {
            login_fault_js("h", "2593", "mara", password.map(str::to_string))
        };
        assert_eq!(fault(Some("")).as_deref(), Some("Type the password."));
        assert_eq!(fault(Some("pw")), None);
        assert_eq!(fault(None), None);
        assert_eq!(save_name_js("mara", "h"), "mara@h");
        assert_eq!(saved_detail_js("mara", "h", 2593), "mara @ h:2593");
    }

    #[test]
    fn the_page_deletes_on_the_second_press_and_hears_of_no_room() {
        let mut list = CharacterListJs::new();
        assert!(!list.press_delete(2));
        assert_eq!(list.delete_asked(), Some(2));
        assert_eq!(list.delete_words(2), LOGIN_WORDS.delete_sure);
        assert!(list.press_delete(2));
        assert_eq!(list.delete_asked(), None);
        assert_eq!(
            no_room_note_js(r#"["A","B","C","D","E"]"#, 0).as_deref(),
            Some(LOGIN_WORDS.no_room)
        );
        assert_eq!(no_room_note_js(r#"["Mara",""]"#, 0), None);
    }
}
