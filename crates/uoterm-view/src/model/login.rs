//! The screens before the game, with no drawing: the words of the login
//! form, of the shard list and of the character list, the checks of the
//! form, and the names a saved login takes. The window and the web client
//! both log in with these.

use serde::Serialize;
use uoterm_protocol::crypto::EncryptionMode;

pub const WORDS_TITLE: &str = "UOTerm";
pub const WORDS_SAVED: &str = "Saved logins";
pub const WORDS_NO_SAVED: &str = "No saved logins yet. Fill in the form and press Save login.";
pub const WORDS_SAVE_LOGIN: &str = "Save login";
pub const WORDS_SAVE: &str = "Save";
pub const WORDS_CANCEL: &str = "Cancel";
pub const WORDS_SAVE_AS: &str = "Save as";
pub const WORDS_SAVED_AS: &str = "Saved as";
pub const WORDS_NOT_SAVED: &str = "The password is not saved.";
pub const WORDS_CONNECT: &str = "Connect";
pub const WORDS_CONNECTING: &str = "Connecting...";
pub const WORDS_PICK_SHARD: &str = "Pick a shard";
pub const WORDS_PICK_CHARACTER: &str = "Pick a character";
pub const WORDS_MAKE: &str = "New character";
pub const WORDS_DELETE: &str = "Delete";
/// A delete asks twice: the second click is on these words.
pub const WORDS_DELETE_SURE: &str = "Delete?";
pub const WORDS_EMPTY_SLOT: &str = "(empty)";
pub const WORDS_NO_ROOM: &str = "The account has no room for another character.";
/// Leaves the character list with no character: the login ends.
pub const WORDS_LEAVE: &str = "Leave";
pub const WORDS_ENCRYPTION: &str = "Encryption";
/// The fields of the form, in their order.
pub const LABELS: [&str; 6] = ["Host", "Port", "Account", "Password", "Shard", "Character"];
/// The encryption choices of the form, as `play --encryption` names them.
pub const ENCRYPTIONS: [(EncryptionMode, &str); 2] = [
    (EncryptionMode::None, "None (most free shards)"),
    (EncryptionMode::Osi, "OSI (encrypted shards)"),
];
/// The slots the character list shows: the most an account has.
pub const CHARACTER_SLOTS: usize = 7;
pub const NEEDS_ACCOUNT: &str = "Type the account.";
pub const NEEDS_HOST: &str = "Type the host.";
pub const BAD_PORT: &str = "The port must be a number from 1 to 65535.";
pub const NEEDS_NAME: &str = "Type a name for the saved login.";
pub const NEEDS_PASSWORD: &str = "Type the password.";

/// The words of the login screens, for a page that draws them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LoginWords {
    pub title: &'static str,
    pub saved: &'static str,
    pub no_saved: &'static str,
    pub save_login: &'static str,
    pub save: &'static str,
    pub cancel: &'static str,
    pub save_as: &'static str,
    pub saved_as: &'static str,
    pub not_saved: &'static str,
    pub connect: &'static str,
    pub connecting: &'static str,
    pub pick_shard: &'static str,
    pub pick_character: &'static str,
    pub make: &'static str,
    pub delete: &'static str,
    pub delete_sure: &'static str,
    pub empty_slot: &'static str,
    pub no_room: &'static str,
    pub leave: &'static str,
    pub labels: [&'static str; 6],
    pub encryption: &'static str,
    pub encryptions: [(EncryptionMode, &'static str); 2],
    pub character_slots: usize,
}

/// The words of the login screens.
pub const LOGIN_WORDS: LoginWords = LoginWords {
    title: WORDS_TITLE,
    saved: WORDS_SAVED,
    no_saved: WORDS_NO_SAVED,
    save_login: WORDS_SAVE_LOGIN,
    save: WORDS_SAVE,
    cancel: WORDS_CANCEL,
    save_as: WORDS_SAVE_AS,
    saved_as: WORDS_SAVED_AS,
    not_saved: WORDS_NOT_SAVED,
    connect: WORDS_CONNECT,
    connecting: WORDS_CONNECTING,
    pick_shard: WORDS_PICK_SHARD,
    pick_character: WORDS_PICK_CHARACTER,
    make: WORDS_MAKE,
    delete: WORDS_DELETE,
    delete_sure: WORDS_DELETE_SURE,
    empty_slot: WORDS_EMPTY_SLOT,
    no_room: WORDS_NO_ROOM,
    leave: WORDS_LEAVE,
    labels: LABELS,
    encryption: WORDS_ENCRYPTION,
    encryptions: ENCRYPTIONS,
    character_slots: CHARACTER_SLOTS,
};

/// The account, trimmed, or words for the human when there is none.
pub fn account_name(account: &str) -> Result<&str, &'static str> {
    Some(account.trim())
        .filter(|account| !account.is_empty())
        .ok_or(NEEDS_ACCOUNT)
}

/// The host, trimmed, or words for the human when there is none.
pub fn host_name(host: &str) -> Result<&str, &'static str> {
    Some(host.trim())
        .filter(|host| !host.is_empty())
        .ok_or(NEEDS_HOST)
}

/// The port, or words for the human when it is not one.
pub fn port_number(port: &str) -> Result<u16, &'static str> {
    port.trim()
        .parse()
        .ok()
        .filter(|port| *port != 0)
        .ok_or(BAD_PORT)
}

/// Why a form cannot log in, or None when it can: the account, the host,
/// the port and the password, in that order. A form saved, not logged in
/// with, gives no password.
pub fn login_fault(
    host: &str,
    port: &str,
    account: &str,
    password: Option<&str>,
) -> Option<&'static str> {
    let typed = password.is_none_or(|password| !password.is_empty());
    account_name(account)
        .and(host_name(host))
        .and(port_number(port))
        .and(Some(()).filter(|()| typed).ok_or(NEEDS_PASSWORD))
        .err()
}

/// The name "Save login" offers for a new saved login: account@host.
pub fn save_name(account: &str, host: &str) -> String {
    format!("{}@{}", account.trim(), host.trim())
}

/// The server of a saved login, as `host:port`.
pub fn server_words(host: &str, port: u16) -> String {
    format!("{host}:{port}")
}

/// The line under the name of a saved login: "account @ host:port".
pub fn saved_detail(account: &str, host: &str, port: u16) -> String {
    format!("{account} @ {}", server_words(host, port))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_login_needs_an_account_a_host_a_port_and_a_password_in_that_order() {
        assert_eq!(login_fault("", "x", " ", Some("")), Some(NEEDS_ACCOUNT));
        assert_eq!(login_fault(" ", "x", "mara", Some("")), Some(NEEDS_HOST));
        assert_eq!(login_fault("h", "0", "mara", Some("")), Some(BAD_PORT));
        assert_eq!(login_fault("h", "70000", "mara", Some("")), Some(BAD_PORT));
        assert_eq!(
            login_fault("h", " 2593 ", "mara", Some("")),
            Some(NEEDS_PASSWORD)
        );
        assert_eq!(login_fault("h", "2593", "mara", Some("pw")), None);
        assert_eq!(login_fault("h", "2593", "mara", None), None, "a save");
        assert_eq!(port_number(" 2593 "), Ok(2593));
        assert_eq!(account_name(" mara "), Ok("mara"));
    }

    #[test]
    fn a_saved_login_is_named_and_described_by_its_account_and_server() {
        assert_eq!(save_name(" mara ", " 10.0.0.7 "), "mara@10.0.0.7");
        assert_eq!(
            saved_detail("acct2", "play.example.com", 2594),
            "acct2 @ play.example.com:2594"
        );
    }

    #[test]
    fn the_words_go_to_a_page_with_the_encryptions_by_their_wire_names() {
        let words = serde_json::to_value(LOGIN_WORDS).unwrap();
        assert_eq!(words["encryptions"][1][0], "osi");
        assert_eq!(words["labels"][3], "Password");
        assert_eq!(words["character_slots"], CHARACTER_SLOTS);
    }
}
