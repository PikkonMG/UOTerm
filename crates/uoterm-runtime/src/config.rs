pub use crate::persona::Persona;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
pub use uoterm_protocol::crypto::EncryptionMode;
use uoterm_protocol::crypto::{for_mode, StreamCipher};
use uoterm_protocol::types::{ClientVersion, Era, LOGIN_NEXT_KEY_DEFAULT};

pub const APP_NAME: &str = "uoterm";
pub const DEFAULT_API_PORT: u16 = 7733;
pub const DEFAULT_LOGIN_PORT: u16 = 2593;
pub const DEFAULT_MAX_SESSIONS: usize = 32;
/// How long one step on foot takes: a walking step and a running one.
pub const STEP_WALK_MS: u64 = 400;
pub const STEP_RUN_MS: u64 = 200;
/// How long one step on a mount takes. A mount carries the character at twice
/// the pace of his own legs at both a walk and a run, which three independent
/// sources agree on.
pub const STEP_MOUNT_WALK_MS: u64 = STEP_WALK_MS / MOUNT_PACE_SHARE;
pub const STEP_MOUNT_RUN_MS: u64 = STEP_RUN_MS / MOUNT_PACE_SHARE;
/// How many times faster a mount is than the person on it.
const MOUNT_PACE_SHARE: u64 = 2;
pub const JITTER_PCT: u32 = 15;
pub const REFLEX_TICK_MS: u64 = 100;
pub const PING_INTERVAL_MS: u64 = 30_000;
pub const JOURNAL_HARVEST_NAME: &str = "harvest.jsonl";
pub const ENCRYPTION_NONE: &str = "none";
pub const ENCRYPTION_OSI: &str = "osi";
pub const ENV_API_TOKEN: &str = "UOTERM_API_TOKEN";
pub const BEARER_PREFIX: &str = "Bearer ";
/// Some shards send a list of assistant features they forbid. By default the
/// character obeys it. A user can switch this off to ignore the list.
pub const OBEY_SHARD_RULES_DEFAULT: bool = true;

/// For serde: a config file without the setting obeys the shard's list.
pub fn obey_shard_rules_default() -> bool {
    OBEY_SHARD_RULES_DEFAULT
}
/// When another character says this one's name, the agent hears of it so
/// it can answer. A user can switch this off.
pub const ANSWER_WHEN_NAMED_DEFAULT: bool = true;

/// For serde: a config file without the setting tells the agent.
pub fn answer_when_named_default() -> bool {
    ANSWER_WHEN_NAMED_DEFAULT
}
/// With `answer_when_named`, the agent may also party up with, follow and
/// fight beside a player who spoke to the character. Off unless the user
/// switches it on; then the agent only answers and says no to plans.
pub const PLAY_ALONG_DEFAULT: bool = false;

pub fn parse_encryption_mode(s: &str) -> crate::error::Result<EncryptionMode> {
    match s.trim().to_ascii_lowercase().as_str() {
        ENCRYPTION_NONE => Ok(EncryptionMode::None),
        ENCRYPTION_OSI => Ok(EncryptionMode::Osi),
        _ => Err(crate::error::RuntimeError::Usage(format!(
            "encryption must be {ENCRYPTION_NONE} or {ENCRYPTION_OSI}"
        ))),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub uopath: Option<PathBuf>,
    /// A marker file of named places to travel to (UO Auto Map `.map` or
    /// Ultima Mapper `Waypoints.lua`). The user supplies it; none ships.
    #[serde(default)]
    pub markers: Option<PathBuf>,
    pub era: Era,
    pub log_level: String,
    pub api_bind: String,
    pub max_sessions: usize,
    #[serde(default = "obey_shard_rules_default")]
    pub obey_shard_rules: bool,
    #[serde(default = "answer_when_named_default")]
    pub answer_when_named: bool,
    #[serde(default)]
    pub play_along: bool,
    /// `connect` opens the watch window by itself, as `--view` does.
    #[serde(default)]
    pub view: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: DEFAULT_LOGIN_PORT,
            uopath: None,
            markers: None,
            era: Era::Modern,
            log_level: "info".into(),
            api_bind: format!("127.0.0.1:{DEFAULT_API_PORT}"),
            max_sessions: DEFAULT_MAX_SESSIONS,
            obey_shard_rules: OBEY_SHARD_RULES_DEFAULT,
            answer_when_named: ANSWER_WHEN_NAMED_DEFAULT,
            play_along: PLAY_ALONG_DEFAULT,
            view: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub account: String,
    pub password_env: String,
    pub character: String,
    pub shard: Option<String>,
    pub version: Option<String>,
    pub era: Option<String>,
}

/// What a login asks a human: which shard of the list, or which character.
/// The answer is the place of the pick in `names`.
#[derive(Debug)]
pub enum LoginQuestion {
    Shard {
        names: Vec<String>,
        reply: tokio::sync::oneshot::Sender<usize>,
    },
    /// The characters of the account. The screen may play one, delete one
    /// or make a new one. The shard answers a new list, or a refusal.
    Characters {
        names: Vec<String>,
        /// The words of the last refusal of the shard, when there was one.
        refused: Option<String>,
        reply: tokio::sync::oneshot::Sender<CharacterRequest>,
    },
    Character {
        names: Vec<String>,
        reply: tokio::sync::oneshot::Sender<usize>,
    },
}

/// What a login screen may ask the shard to do with the characters of the
/// account, before it plays one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CharacterRequest {
    /// Play the character in this slot.
    Play(usize),
    Delete(usize),
    Make(Box<NewCharacterWish>),
}

/// What a player picked for a new character, in the words of a screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewCharacterWish {
    pub name: String,
    pub female: bool,
    pub race: u8,
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
    pub skills: Vec<(u8, u8)>,
    pub skin_hue: u16,
    pub hair: u16,
    pub hair_hue: u16,
    pub start_city: u16,
    pub slot: u16,
}

/// The way from a login to the screen that answers its questions. With no
/// picker, the login picks by the names in the options, as an agent needs.
#[derive(Clone, Debug)]
pub struct LoginPicker(pub tokio::sync::mpsc::UnboundedSender<LoginQuestion>);

impl LoginPicker {
    /// Asks the screen what to do with the characters of the account.
    /// None when the screen is gone.
    pub async fn characters(
        &self,
        names: Vec<String>,
        refused: Option<String>,
    ) -> Option<CharacterRequest> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        self.0
            .send(LoginQuestion::Characters {
                names,
                refused,
                reply,
            })
            .ok()?;
        answer.await.ok()
    }

    /// Asks the screen, and waits for the pick. None when the screen is
    /// gone, or when its answer is not a place of the list.
    pub async fn pick(
        &self,
        names: Vec<String>,
        question: fn(Vec<String>, tokio::sync::oneshot::Sender<usize>) -> LoginQuestion,
    ) -> Option<usize> {
        let count = names.len();
        let (reply, answer) = tokio::sync::oneshot::channel();
        self.0.send(question(names, reply)).ok()?;
        answer.await.ok().filter(|place| *place < count)
    }
}

#[derive(Clone, Debug)]
pub struct ConnectOptions {
    pub host: String,
    pub port: u16,
    pub account: String,
    pub password: String,
    pub shard: Option<String>,
    pub character: String,
    pub version: ClientVersion,
    pub era: Era,
    pub uopath: Option<PathBuf>,
    /// A marker file of named places to travel to. See [`AppConfig::markers`].
    pub markers: Option<PathBuf>,
    pub persona: Option<Persona>,
    pub next_login_key: u8,
    pub encryption: EncryptionMode,
    /// Obey the shard's list of forbidden assistant features.
    pub obey_shard_rules: bool,
    /// Tell the agent when another character says this one's name.
    pub answer_when_named: bool,
    /// Let the agent party up with, follow and help a player who spoke.
    pub play_along: bool,
    /// A screen that picks the shard and the character. None for a login
    /// that picks by name.
    pub picker: Option<LoginPicker>,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: DEFAULT_LOGIN_PORT,
            account: String::new(),
            password: String::new(),
            shard: None,
            character: String::new(),
            version: ClientVersion::MODERN,
            era: Era::Modern,
            uopath: None,
            markers: None,
            persona: None,
            next_login_key: LOGIN_NEXT_KEY_DEFAULT,
            encryption: EncryptionMode::None,
            obey_shard_rules: OBEY_SHARD_RULES_DEFAULT,
            answer_when_named: ANSWER_WHEN_NAMED_DEFAULT,
            play_along: PLAY_ALONG_DEFAULT,
            picker: None,
        }
    }
}

impl ConnectOptions {
    pub fn client_flag(&self) -> u32 {
        match self.era {
            Era::T2a => 0x00,
            Era::Modern => 0x20,
        }
    }

    pub fn cipher(&self, seed: u32) -> Box<dyn StreamCipher> {
        for_mode(self.encryption, seed, self.version)
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

pub fn data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

pub fn load_app_config(path: Option<&PathBuf>) -> AppConfig {
    let candidates = [
        path.cloned(),
        Some(PathBuf::from("uoterm.toml")),
        Some(config_dir().join("uoterm.toml")),
    ];
    for c in candidates.into_iter().flatten() {
        if let Ok(text) = std::fs::read_to_string(&c) {
            if let Ok(cfg) = toml::from_str(&text) {
                return cfg;
            }
        }
    }
    AppConfig::default()
}

pub fn load_persona(path: &std::path::Path) -> crate::error::Result<Persona> {
    Persona::load(path)
        .map_err(|e| crate::error::RuntimeError::Usage(format!("persona {}: {e}", path.display())))
}

pub fn era_from_str(s: &str) -> Era {
    s.parse().unwrap_or(Era::Modern)
}

pub fn version_from_str(s: Option<&str>, era: Era) -> ClientVersion {
    s.and_then(|v| v.parse().ok())
        .unwrap_or_else(|| era.default_version())
}

pub fn load_profile(path: &std::path::Path) -> crate::error::Result<Profile> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        crate::error::RuntimeError::Usage(format!("profile {}: {e}", path.display()))
    })?;
    toml::from_str(&text)
        .map_err(|e| crate::error::RuntimeError::Usage(format!("profile parse: {e}")))
}

pub fn password_from_env(var: &str) -> crate::error::Result<String> {
    std::env::var(var)
        .map_err(|_| crate::error::RuntimeError::Usage(format!("password env {var} is not set")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_default_is_none() {
        assert_eq!(EncryptionMode::default(), EncryptionMode::None);
        assert_eq!(ConnectOptions::default().encryption, EncryptionMode::None);
        assert_eq!(
            parse_encryption_mode(ENCRYPTION_NONE).unwrap(),
            EncryptionMode::None
        );
    }

    #[test]
    fn osi_mode_is_stored() {
        assert_eq!(
            parse_encryption_mode(ENCRYPTION_OSI).unwrap(),
            EncryptionMode::Osi
        );
        assert_eq!(parse_encryption_mode("OSI").unwrap(), EncryptionMode::Osi);
        let opts = ConnectOptions {
            encryption: EncryptionMode::Osi,
            ..ConnectOptions::default()
        };
        assert_eq!(opts.encryption, EncryptionMode::Osi);
        assert_eq!(opts.cipher(1).name(), ENCRYPTION_OSI);
        assert_eq!(
            for_mode(EncryptionMode::None, 1, ClientVersion::MODERN).name(),
            ENCRYPTION_NONE
        );
    }

    /// A config file written before the setting existed still loads, and
    /// obeys the shard's list. The `stay_on_socket` key of an old file is
    /// read past: the client always opens a new socket to the game server.
    #[test]
    fn a_config_without_the_setting_obeys_the_shard() {
        const OLD_CONFIG: &str = r#"
host = "127.0.0.1"
port = 2593
era = "t2a"
log_level = "info"
api_bind = "127.0.0.1:7733"
max_sessions = 32
stay_on_socket = true
"#;
        let cfg: AppConfig = toml::from_str(OLD_CONFIG).expect("an old config loads");
        assert!(cfg.obey_shard_rules);
        assert!(AppConfig::default().obey_shard_rules);
        assert!(ConnectOptions::default().obey_shard_rules);
    }

    #[test]
    fn the_setting_can_ignore_the_shard() {
        const IGNORING: &str = r#"
host = "127.0.0.1"
port = 2593
era = "t2a"
log_level = "info"
api_bind = "127.0.0.1:7733"
max_sessions = 32
obey_shard_rules = false
"#;
        let cfg: AppConfig = toml::from_str(IGNORING).expect("the config loads");
        assert!(!cfg.obey_shard_rules);
    }

    /// The watch window stays shut until the file asks for it.
    #[test]
    fn view_is_off_until_switched_on() {
        const ON: &str = r#"
host = "127.0.0.1"
port = 2593
era = "t2a"
log_level = "info"
api_bind = "127.0.0.1:7733"
max_sessions = 32
view = true
"#;
        assert!(!AppConfig::default().view);
        let cfg: AppConfig = toml::from_str(ON).unwrap();
        assert!(cfg.view);
    }

    /// Telling the agent when someone says the character's name is on
    /// unless the file switches it off.
    #[test]
    fn answer_when_named_is_on_until_switched_off() {
        const OFF: &str = r#"
host = "127.0.0.1"
port = 2593
era = "t2a"
log_level = "info"
api_bind = "127.0.0.1:7733"
max_sessions = 32
answer_when_named = false
"#;
        assert!(AppConfig::default().answer_when_named);
        assert!(ConnectOptions::default().answer_when_named);
        let cfg: AppConfig = toml::from_str(OFF).expect("the config loads");
        assert!(!cfg.answer_when_named);
        assert!(
            !AppConfig::default().play_along,
            "play along is off at first"
        );
    }

    #[test]
    fn encryption_parse_rejects_unknown() {
        assert!(parse_encryption_mode("blowfish").is_err());
    }

    #[test]
    fn example_config_loads_every_app_setting() {
        const EXAMPLE: &str = include_str!("../../../uoterm.toml.example");
        let cfg: AppConfig = toml::from_str(EXAMPLE).expect("example loads");
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, DEFAULT_LOGIN_PORT);
        assert_eq!(cfg.era, Era::Modern);
        assert_eq!(cfg.log_level, "info");
        assert_eq!(cfg.api_bind, format!("127.0.0.1:{DEFAULT_API_PORT}"));
        assert_eq!(cfg.max_sessions, DEFAULT_MAX_SESSIONS);
        assert!(cfg.obey_shard_rules);
        assert!(cfg.answer_when_named);
        assert!(!cfg.play_along);
        assert_eq!(
            cfg.uopath.as_deref(),
            Some(std::path::Path::new("/path/to/uo"))
        );
        assert_eq!(
            cfg.markers.as_deref(),
            Some(std::path::Path::new("/path/to/Waypoints.lua"))
        );
        assert!(!EXAMPLE.contains("stay_on_socket"), "old key must not ship");
    }
}
