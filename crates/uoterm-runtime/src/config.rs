pub use crate::persona::Persona;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
pub use uoterm_protocol::crypto::EncryptionMode;
use uoterm_protocol::crypto::{for_mode, StreamCipher};
use uoterm_protocol::types::{ClientVersion, Era, LOGIN_NEXT_KEY_DEFAULT};

pub const APP_NAME: &str = "uoterm";
pub const DEFAULT_API_PORT: u16 = 7733;
pub const DEFAULT_LOGIN_PORT: u16 = 2593;
/// The host a session and the demo shard use when none is named: this
/// machine.
pub const DEFAULT_HOST: &str = "127.0.0.1";
/// Where the demo shard listens when no bind is named: [`DEFAULT_HOST`] on
/// [`DEFAULT_LOGIN_PORT`]. A test holds the two together.
pub const DEFAULT_MOCK_BIND: &str = "127.0.0.1:2593";
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

/// A session whose link to the shard drops logs in again by itself, as a
/// player whose connection broke would, unless it logged out.
pub const RECONNECT_DEFAULT: bool = true;

pub fn reconnect_default() -> bool {
    RECONNECT_DEFAULT
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
    /// Log in again when the link to the shard drops.
    #[serde(default = "reconnect_default")]
    pub reconnect: bool,
    /// Reach the shard through this proxy: `socks5://host:port` or
    /// `http://host:port`, with `user:password@` before the host when the
    /// proxy asks for one.
    #[serde(default)]
    pub proxy: Option<crate::proxy::Proxy>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.into(),
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
            reconnect: RECONNECT_DEFAULT,
            proxy: None,
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
        /// What a new character may be: the start towns and the flags.
        choices: CharacterChoices,
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
    /// Play none: the login ends at the character list.
    Leave,
}

/// What the shard lets a new character be: the towns he may start in, the
/// features of the account (`0xB9`) and the flags of the character list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharacterChoices {
    pub towns: Vec<uoterm_protocol::StartTown>,
    pub features: u32,
    pub list_flags: u32,
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
    pub beard: u16,
    pub beard_hue: u16,
    pub shirt_hue: u16,
    pub pants_hue: u16,
    pub profession: u8,
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
        choices: CharacterChoices,
    ) -> Option<CharacterRequest> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        self.0
            .send(LoginQuestion::Characters {
                names,
                refused,
                choices,
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
    /// Log in again when the link to the shard drops.
    pub reconnect: bool,
    /// The proxy the session reaches the shard through. See
    /// [`AppConfig::proxy`].
    pub proxy: Option<crate::proxy::Proxy>,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.into(),
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
            reconnect: RECONNECT_DEFAULT,
            proxy: None,
        }
    }
}

impl ConnectOptions {
    /// The expansion bits the play-character request carries. They come from
    /// the client version, the same way the reference Classic Client builds
    /// them, so a shard gives the session every map and rule that version has
    /// and still holds it as a Classic Client.
    pub fn client_flag(&self) -> u32 {
        self.version.expansion_flags()
    }

    pub fn cipher(&self, seed: u32) -> Box<dyn StreamCipher> {
        for_mode(self.encryption, seed, self.version)
    }
}

/// The folders a process keeps its files in: settings and data.
struct Folders {
    config: PathBuf,
    data: PathBuf,
}

/// The folders of this process, set on first use. Only a program run by a
/// person claims the person's own folders, with [`use_user_folders`]. Any
/// other process, such as a test, gets folders of its own under the temp
/// folder, so it can never change the person's settings or data.
static FOLDERS: OnceLock<Folders> = OnceLock::new();

/// The name of the folder a process without the person's folders uses.
const SCRATCH_FOLDER_PREFIX: &str = "uoterm-scratch-";
const SCRATCH_CONFIG_FOLDER: &str = "config";
const SCRATCH_DATA_FOLDER: &str = "data";

impl Folders {
    fn user() -> Self {
        let under =
            |base: Option<PathBuf>| base.unwrap_or_else(|| PathBuf::from(".")).join(APP_NAME);
        Self {
            config: under(dirs::config_dir()),
            data: under(dirs::data_local_dir()),
        }
    }

    fn scratch() -> Self {
        let root =
            std::env::temp_dir().join(format!("{SCRATCH_FOLDER_PREFIX}{}", std::process::id()));
        Self {
            config: root.join(SCRATCH_CONFIG_FOLDER),
            data: root.join(SCRATCH_DATA_FOLDER),
        }
    }
}

/// Makes this process keep its files in the person's own folders. The
/// program calls it first, before anything reads a folder; a test never
/// does.
pub fn use_user_folders() {
    // A second call finds the folders set already, and keeps them.
    let _ = FOLDERS.set(Folders::user());
}

fn folders() -> &'static Folders {
    FOLDERS.get_or_init(Folders::scratch)
}

pub fn config_dir() -> PathBuf {
    folders().config.clone()
}

pub fn data_dir() -> PathBuf {
    folders().data.clone()
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

/// The era named, or the modern one when none is named or the name is not an
/// era.
pub fn era_from_str(s: Option<&str>) -> Era {
    s.and_then(|name| name.parse().ok()).unwrap_or(Era::Modern)
}

/// The version a session says it is: the one named; else, in the modern
/// era, the version of the client program in the client folder, which a
/// shard that checks versions compares with its own copy; else the default
/// of the era.
pub fn client_version(named: Option<&str>, era: Era, uopath: Option<&Path>) -> ClientVersion {
    named
        .and_then(|v| v.parse().ok())
        .or_else(|| {
            uopath
                .filter(|_| era == Era::Modern)
                .and_then(uoterm_nav::client_program_version)
        })
        .unwrap_or_else(|| era.default_version())
}

/// The folder the saved logins are kept in, below the working directory,
/// and the extension of each file.
pub const PROFILES_DIR: &str = "profiles";
pub const PROFILE_EXT: &str = "toml";

/// The file of the saved login of this name.
pub fn profile_path(name: &str) -> PathBuf {
    PathBuf::from(PROFILES_DIR).join(format!("{name}.{PROFILE_EXT}"))
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
    fn a_named_version_is_said_as_it_is() {
        let folder = uoterm_nav::client_data_dir_from_env();
        let named = client_version(Some("7.0.50.0"), Era::Modern, folder.as_deref());
        assert_eq!(named, ClientVersion::new(7, 0, 50, 0));
    }

    #[test]
    fn with_no_client_program_the_era_names_the_version() {
        let empty = std::env::temp_dir().join("uoterm-no-client-program");
        assert_eq!(
            client_version(None, Era::Modern, Some(&empty)),
            Era::Modern.default_version()
        );
        assert_eq!(
            client_version(None, Era::Modern, None),
            Era::Modern.default_version()
        );
    }

    #[test]
    fn a_modern_session_says_the_version_of_its_client_program() {
        let Some(folder) = uoterm_nav::client_data_dir_from_env() else {
            eprintln!("skipped: UOTERM_TEST_UOPATH is not set");
            return;
        };
        let Some(program) = uoterm_nav::client_program_version(&folder) else {
            eprintln!("skipped: the client folder has no client program");
            return;
        };
        assert_eq!(client_version(None, Era::Modern, Some(&folder)), program);
        // An old-era session keeps the version of its era.
        assert_eq!(
            client_version(None, Era::T2a, Some(&folder)),
            Era::T2a.default_version()
        );
    }

    #[test]
    fn the_demo_shard_listens_on_the_default_host_and_port() {
        assert_eq!(
            DEFAULT_MOCK_BIND,
            format!("{DEFAULT_HOST}:{DEFAULT_LOGIN_PORT}")
        );
    }

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
    fn a_proxy_is_read_from_the_config_file() {
        const WITH_PROXY: &str = r#"
host = "127.0.0.1"
port = 2593
era = "t2a"
log_level = "info"
api_bind = "127.0.0.1:7733"
max_sessions = 32
proxy = "socks5://10.0.0.2:1080"
"#;
        assert!(AppConfig::default().proxy.is_none());
        let cfg: AppConfig = toml::from_str(WITH_PROXY).expect("the config loads");
        let proxy = cfg.proxy.expect("the proxy is read");
        assert_eq!(proxy.kind, crate::proxy::ProxyKind::Socks5);
        assert_eq!(proxy.port, 1080);
        let bad = WITH_PROXY.replace("socks5", "gopher");
        assert!(toml::from_str::<AppConfig>(&bad).is_err());
    }

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
    fn the_play_flags_come_from_the_client_version() {
        let modern = ConnectOptions {
            version: ClientVersion::MODERN,
            ..ConnectOptions::default()
        };
        assert_eq!(
            modern.client_flag(),
            ClientVersion::MODERN.expansion_flags()
        );
        let old = ConnectOptions {
            version: ClientVersion::T2A,
            ..ConnectOptions::default()
        };
        assert_eq!(old.client_flag(), ClientVersion::T2A.expansion_flags());
        assert!(old.version.is_classic());
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

    #[test]
    fn a_process_that_does_not_claim_the_user_folders_keeps_files_in_temp() {
        let user = Folders::user();
        let temp = std::env::temp_dir();
        for folder in [config_dir(), data_dir()] {
            assert!(
                folder.starts_with(&temp),
                "{} is not a temp folder",
                folder.display()
            );
            assert_ne!(folder, user.config);
            assert_ne!(folder, user.data);
        }
        assert_ne!(config_dir(), data_dir());
    }
}
