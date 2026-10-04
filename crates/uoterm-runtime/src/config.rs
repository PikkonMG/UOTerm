pub use crate::persona::Persona;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
pub use uoterm_protocol::crypto::EncryptionMode;
use uoterm_protocol::crypto::{for_mode, StreamCipher};
use uoterm_protocol::types::{ClientVersion, Era, LOGIN_NEXT_KEY_DEFAULT};
pub use uoterm_world::login::{CharacterChoices, CharacterRequest, NewCharacterWish};
use uoterm_world::login::{LoginAsk, LoginReply};

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

impl AppConfig {
    /// The era and the client version of a login: the ones a saved login or
    /// a page names, else the era of the config and its version.
    pub fn era_version(&self, era: Option<&str>, version: Option<&str>) -> (Era, ClientVersion) {
        let era = era.and_then(|era| era.parse().ok()).unwrap_or(self.era);
        (era, client_version(version, era, self.uopath.as_deref()))
    }
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

/// A saved login: where to log in and as whom. The password is never in
/// it; `password_env` may name the environment variable that holds it.
/// Each key but the account may be left out, so older files still load.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub account: String,
    /// The login server: an IP address or a DNS name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption: Option<EncryptionMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_env: Option<String>,
    /// Empty: the login asks, or plays the first of the account.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub character: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shard: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub era: Option<String>,
}

/// Where a login goes and how it speaks to the shard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginTarget {
    pub host: String,
    pub port: u16,
    pub encryption: EncryptionMode,
}

impl LoginTarget {
    /// Each part is the one the command line or the call names, else the
    /// one of the saved login, else the one of the config file. The config
    /// file names no encryption: then it is none.
    pub fn choose(
        host: Option<String>,
        port: Option<u16>,
        encryption: Option<EncryptionMode>,
        saved: Option<&Profile>,
        cfg: &AppConfig,
    ) -> Self {
        Self {
            host: host
                .or_else(|| saved.and_then(|p| p.host.clone()))
                .unwrap_or_else(|| cfg.host.clone()),
            port: port.or(saved.and_then(|p| p.port)).unwrap_or(cfg.port),
            encryption: encryption
                .or(saved.and_then(|p| p.encryption))
                .unwrap_or_default(),
        }
    }
}

/// What a login asks a human: which shard of the list, and what to do
/// with the characters of the account.
#[derive(Debug)]
pub enum LoginQuestion {
    /// The answer is the place of the pick in `names`.
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
}

impl LoginQuestion {
    /// The question as a screen across the wire reads it.
    pub fn ask(&self) -> LoginAsk {
        match self {
            Self::Shard { names, .. } => LoginAsk::Shard {
                names: names.clone(),
            },
            Self::Characters {
                names,
                refused,
                choices,
                ..
            } => LoginAsk::Characters {
                names: names.clone(),
                refused: refused.clone(),
                choices: choices.clone(),
            },
        }
    }

    /// Gives the reply to the login. A reply that does not fit gives the
    /// question back, still open: one of the wrong kind, a pick past the
    /// end of the list, or a slot to play or delete with no character in
    /// it. A login that stopped waiting takes the reply as well: there is
    /// nothing left to answer.
    pub fn answer(self, reply: LoginReply) -> Result<(), LoginQuestion> {
        match (self, reply) {
            (Self::Shard { names, reply }, LoginReply::Pick { index }) if index < names.len() => {
                let _ = reply.send(index);
                Ok(())
            }
            (
                Self::Characters {
                    names,
                    refused,
                    choices,
                    reply,
                },
                LoginReply::Request { request },
            ) => {
                let filled = |slot: usize| names.get(slot).is_some_and(|name| !name.is_empty());
                match request {
                    CharacterRequest::Play(slot) | CharacterRequest::Delete(slot)
                        if !filled(slot) =>
                    {
                        Err(Self::Characters {
                            names,
                            refused,
                            choices,
                            reply,
                        })
                    }
                    request => {
                        let _ = reply.send(request);
                        Ok(())
                    }
                }
            }
            (question, _) => Err(question),
        }
    }
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

    /// Asks the screen which shard of the list to play on, and waits for
    /// the pick. None when the screen is gone, or when its answer is not a
    /// place of the list.
    pub async fn shard(&self, names: Vec<String>) -> Option<usize> {
        let count = names.len();
        let (reply, answer) = tokio::sync::oneshot::channel();
        self.0.send(LoginQuestion::Shard { names, reply }).ok()?;
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

/// A login a person asks for on a screen: the login form of the window,
/// or the login link of a web page. The screen answers the questions of
/// the login; the config file gives the rest.
#[derive(Deserialize)]
pub struct ScreenLogin {
    pub host: String,
    pub port: u16,
    pub account: String,
    pub password: String,
    /// The shard of the list to play on. None or blank: the screen picks.
    #[serde(default)]
    pub shard: Option<String>,
    /// The character to play. None or blank: the screen picks.
    #[serde(default)]
    pub character: Option<String>,
    /// The era and the version of the login. None: the ones of the config.
    #[serde(default)]
    pub era: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub encryption: EncryptionMode,
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
    /// The options of a login a person asks for on a screen, which
    /// `picker` asks the questions of. The client files, the markers and
    /// the rules of play are the ones of the config file.
    pub fn for_screen(login: ScreenLogin, cfg: &AppConfig, picker: LoginPicker) -> Self {
        let (era, version) = cfg.era_version(login.era.as_deref(), login.version.as_deref());
        let shard = login.shard.as_deref().map(str::trim).unwrap_or_default();
        Self {
            host: login.host.trim().to_string(),
            port: login.port,
            account: login.account.trim().to_string(),
            password: login.password,
            shard: (!shard.is_empty()).then(|| shard.to_string()),
            character: login
                .character
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .to_string(),
            version,
            era,
            uopath: cfg.uopath.clone(),
            markers: cfg.markers.clone(),
            encryption: login.encryption,
            obey_shard_rules: cfg.obey_shard_rules,
            answer_when_named: cfg.answer_when_named,
            play_along: cfg.play_along,
            picker: Some(picker),
            reconnect: cfg.reconnect,
            proxy: cfg.proxy.clone(),
            ..Self::default()
        }
    }

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

/// The folder of the saved logins the login screen writes, in the config
/// folder.
pub const LOGINS_DIR: &str = "logins";
/// The older folder of saved logins, below the working directory. It is
/// read and never written.
pub const PROFILES_DIR: &str = "profiles";
pub const PROFILE_EXT: &str = "toml";
/// The variable that holds the password when nothing names one.
pub const DEFAULT_PASSWORD_ENV: &str = "UO_PASS";
/// Stands for any byte that may not be in a file name.
const ESCAPE: char = '%';
/// The name of a file that has no name left.
const EMPTY_NAME: &str = "%";
const HEX: u32 = 16;
/// The hex digits after each [`ESCAPE`].
const ESCAPE_DIGITS: usize = 2;

/// Words made safe for a file name: letters, digits, `-` and `_` stay, and
/// each other byte becomes `%` and two hex digits. A dot stays too, but not
/// at the start, so no name is hidden or means a parent folder.
pub fn file_safe(words: &str) -> String {
    let mut safe = String::with_capacity(words.len());
    for (at, byte) in words.bytes().enumerate() {
        let keep = byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'_'
            || (byte == b'.' && at > 0);
        if keep {
            safe.push(char::from(byte));
        } else {
            safe.push_str(&format!("{ESCAPE}{byte:02X}"));
        }
    }
    if safe.is_empty() {
        safe.push_str(EMPTY_NAME);
    }
    safe
}

/// The words a [`file_safe`] name was made from. A `%` with no two hex
/// digits after it stays as it is.
pub fn from_file_safe(safe: &str) -> String {
    if safe == EMPTY_NAME {
        return String::new();
    }
    let mut bytes = Vec::with_capacity(safe.len());
    let mut rest = safe.as_bytes();
    while let Some((&first, after)) = rest.split_first() {
        let escaped = (char::from(first) == ESCAPE)
            .then(|| after.get(..ESCAPE_DIGITS))
            .flatten()
            .and_then(|digits| std::str::from_utf8(digits).ok())
            .and_then(|digits| u8::from_str_radix(digits, HEX).ok());
        match escaped {
            Some(byte) => {
                bytes.push(byte);
                rest = &after[ESCAPE_DIGITS..];
            }
            None => {
                bytes.push(first);
                rest = after;
            }
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// One saved login of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredLogin {
    pub name: String,
    pub profile: Profile,
    /// It is kept in the config folder, so the screen may change or delete
    /// it. An older one is only read.
    pub kept: bool,
}

/// The saved logins: the files the login screen writes, in the config
/// folder, and the older files of the profiles folder in the working
/// directory, which are read and never written. When a name is in both,
/// the one of the config folder wins.
#[derive(Clone, Debug)]
pub struct LoginStore {
    kept: PathBuf,
    old: PathBuf,
}

impl LoginStore {
    /// The saved logins of this process: `<config_dir>/logins` and
    /// `profiles` in the working directory.
    pub fn standard() -> Self {
        Self::at(config_dir().join(LOGINS_DIR), PathBuf::from(PROFILES_DIR))
    }

    pub fn at(kept: PathBuf, old: PathBuf) -> Self {
        Self { kept, old }
    }

    fn kept_path(&self, name: &str) -> PathBuf {
        self.kept.join(format!("{}.{PROFILE_EXT}", file_safe(name)))
    }

    /// The older file of a name. A name that is not one plain file name,
    /// such as `../x`, has none.
    fn old_path(&self, name: &str) -> Option<PathBuf> {
        let file = format!("{name}.{PROFILE_EXT}");
        (Path::new(&file).file_name() == Some(std::ffi::OsStr::new(&file)))
            .then(|| self.old.join(file))
    }

    /// The file of the saved login of this name: the one of the config
    /// folder when there is one, else the older one.
    pub fn path(&self, name: &str) -> PathBuf {
        let kept = self.kept_path(name);
        if kept.is_file() {
            return kept;
        }
        self.old_path(name).unwrap_or(kept)
    }

    pub fn load(&self, name: &str) -> crate::error::Result<Profile> {
        read_profile(&self.path(name))
    }

    /// Every saved login that loads, by name.
    pub fn list(&self) -> Vec<StoredLogin> {
        let mut listed: Vec<StoredLogin> = Vec::new();
        for (dir, kept) in [(&self.kept, true), (&self.old, false)] {
            for path in profile_files(dir) {
                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                    continue;
                };
                let name = if kept {
                    from_file_safe(stem)
                } else {
                    stem.to_string()
                };
                if listed.iter().any(|known| known.name == name) {
                    continue;
                }
                if let Ok(profile) = read_profile(&path) {
                    listed.push(StoredLogin {
                        name,
                        profile,
                        kept,
                    });
                }
            }
        }
        listed.sort_by(|a, b| a.name.cmp(&b.name));
        listed
    }

    /// Writes a saved login to the config folder, over the one of that
    /// name. Gives the file.
    pub fn save(&self, name: &str, profile: &Profile) -> crate::error::Result<PathBuf> {
        let path = self.kept_path(name);
        let text = toml::to_string(profile)
            .map_err(|e| crate::error::RuntimeError::Usage(format!("profile: {e}")))?;
        std::fs::create_dir_all(&self.kept)
            .and_then(|()| std::fs::write(&path, text))
            .map_err(|e| {
                crate::error::RuntimeError::Usage(format!("profile {}: {e}", path.display()))
            })?;
        Ok(path)
    }

    /// Deletes the saved login of this name from the config folder. An
    /// older file is left as it is.
    pub fn delete(&self, name: &str) -> crate::error::Result<()> {
        let path = self.kept_path(name);
        std::fs::remove_file(&path).map_err(|e| {
            crate::error::RuntimeError::Usage(format!("profile {}: {e}", path.display()))
        })
    }
}

/// The saved login files of a folder.
fn profile_files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == PROFILE_EXT))
        .collect()
}

/// The file of the saved login of this name. See [`LoginStore::path`].
pub fn profile_path(name: &str) -> PathBuf {
    LoginStore::standard().path(name)
}

/// The saved login a path names. A plain name, such as `cedric` or
/// `cedric.toml`, is found as [`profile_path`] finds it. A path with a
/// folder is read as it is.
pub fn load_profile(path: &Path) -> crate::error::Result<Profile> {
    let plain = path.parent().is_none_or(|dir| dir.as_os_str().is_empty());
    let name = if path.extension().is_some_and(|ext| ext == PROFILE_EXT) {
        path.file_stem()
    } else {
        path.file_name()
    };
    match name.and_then(|name| name.to_str()).filter(|_| plain) {
        Some(name) => read_profile(&profile_path(name)),
        None => read_profile(path),
    }
}

fn read_profile(path: &Path) -> crate::error::Result<Profile> {
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

    /// A store of saved logins in folders of the test's own.
    fn test_store() -> (LoginStore, PathBuf) {
        let root = std::env::temp_dir().join(format!("uoterm-logins-{}", uuid::Uuid::new_v4()));
        (
            LoginStore::at(root.join(LOGINS_DIR), root.join(PROFILES_DIR)),
            root,
        )
    }

    fn full_profile() -> Profile {
        Profile {
            account: "acct".into(),
            host: Some("play.example.com".into()),
            port: Some(2594),
            encryption: Some(EncryptionMode::Osi),
            password_env: Some("MY_PASS".into()),
            character: "Mara".into(),
            shard: Some("Britannia".into()),
            version: Some("7.0.102.3".into()),
            era: Some("modern".into()),
        }
    }

    #[test]
    fn a_profile_goes_to_a_file_and_back_with_and_without_the_new_keys() {
        let full = full_profile();
        let text = toml::to_string(&full).unwrap();
        assert!(text.contains("encryption = \"osi\""), "{text}");
        assert_eq!(toml::from_str::<Profile>(&text).unwrap(), full);
        let bare = Profile {
            account: "acct".into(),
            ..Profile::default()
        };
        let text = toml::to_string(&bare).unwrap();
        assert_eq!(text.trim(), "account = \"acct\"");
        assert_eq!(toml::from_str::<Profile>(&text).unwrap(), bare);
    }

    #[test]
    fn the_older_profile_files_still_load() {
        for text in [
            include_str!("../../../profiles/cedric.toml"),
            include_str!("../../../profiles/aldreth.toml"),
        ] {
            let profile: Profile = toml::from_str(text).expect("an older file loads");
            assert_eq!(profile.password_env.as_deref(), Some(DEFAULT_PASSWORD_ENV));
            assert!(!profile.character.is_empty());
            assert_eq!(
                (profile.host, profile.port, profile.encryption),
                (None, None, None)
            );
        }
    }

    #[test]
    fn the_example_profile_names_every_new_key() {
        let example: Profile = toml::from_str(include_str!("../../../profiles/example.toml"))
            .expect("the example loads");
        assert_eq!(example.host.as_deref(), Some(DEFAULT_HOST));
        assert_eq!(example.port, Some(DEFAULT_LOGIN_PORT));
        assert_eq!(example.encryption, Some(EncryptionMode::None));
        assert_eq!(example.password_env.as_deref(), Some(DEFAULT_PASSWORD_ENV));
    }

    #[test]
    fn the_config_folder_wins_on_a_name_in_both_folders() {
        let (store, root) = test_store();
        std::fs::create_dir_all(root.join(PROFILES_DIR)).unwrap();
        for (name, account) in [("cedric", "old"), ("mara", "older")] {
            std::fs::write(
                root.join(PROFILES_DIR)
                    .join(format!("{name}.{PROFILE_EXT}")),
                format!("account = \"{account}\"\n"),
            )
            .unwrap();
        }
        let kept = Profile {
            account: "new".into(),
            ..Profile::default()
        };
        store.save("cedric", &kept).unwrap();
        store.save("x y", &kept).unwrap();
        let listed = store.list();
        let names: Vec<_> = listed.iter().map(|l| (l.name.as_str(), l.kept)).collect();
        assert_eq!(names, [("cedric", true), ("mara", false), ("x y", true)]);
        assert_eq!(store.load("cedric").unwrap().account, "new");
        assert_eq!(store.load("mara").unwrap().account, "older");
        // Deleting the kept one shows the older one again, which stays.
        store.delete("cedric").unwrap();
        assert_eq!(store.load("cedric").unwrap().account, "old");
        assert!(
            store.delete("mara").is_err(),
            "an older file is never deleted"
        );
        assert!(root.join(PROFILES_DIR).join("mara.toml").is_file());
        assert!(store.load("../mara").is_err(), "no way out of the folders");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn load_profile_finds_a_plain_name_as_profile_path_does() {
        let name = format!("test-{}", uuid::Uuid::new_v4());
        let store = LoginStore::standard();
        let path = store.save(&name, &full_profile()).unwrap();
        assert!(path.starts_with(config_dir().join(LOGINS_DIR)));
        assert_eq!(profile_path(&name), path);
        for plain in [name.clone(), format!("{name}.{PROFILE_EXT}")] {
            assert_eq!(load_profile(Path::new(&plain)).unwrap(), full_profile());
        }
        // A path with a folder is read as it is.
        assert_eq!(load_profile(&path).unwrap(), full_profile());
        assert!(load_profile(&Path::new(PROFILES_DIR).join(&name)).is_err());
        store.delete(&name).unwrap();
        assert!(load_profile(Path::new(&name)).is_err());
    }

    #[test]
    fn names_become_safe_file_names_and_come_back() {
        assert_eq!(file_safe("Mara"), "Mara");
        assert_eq!(
            file_safe("acct@play.example.com:2593"),
            "acct%40play.example.com%3A2593"
        );
        assert_eq!(file_safe("../x"), "%2E.%2Fx");
        assert_eq!(file_safe(""), EMPTY_NAME);
        assert_ne!(file_safe("a b"), file_safe("a_b"));
        for name in ["Mara", "acct@host:2593", "../x", "", "a b", "100%", "Élan"] {
            assert_eq!(from_file_safe(&file_safe(name)), name);
        }
        assert_eq!(from_file_safe("50%zz"), "50%zz");
    }

    #[test]
    fn a_saved_login_holds_no_password() {
        let (store, root) = test_store();
        let path = store.save("mara", &full_profile()).unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        let keys: Vec<&str> = text
            .lines()
            .filter_map(|line| line.split_once(" = ").map(|(key, _)| key))
            .collect();
        assert!(keys
            .iter()
            .all(|key| *key == "password_env" || !key.contains("pass")));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_command_line_wins_then_the_saved_login_then_the_config() {
        let cfg = AppConfig::default();
        let saved = full_profile();
        let named = LoginTarget::choose(
            Some("10.0.0.9".into()),
            Some(3000),
            Some(EncryptionMode::None),
            Some(&saved),
            &cfg,
        );
        assert_eq!(
            named,
            LoginTarget {
                host: "10.0.0.9".into(),
                port: 3000,
                encryption: EncryptionMode::None,
            }
        );
        let from_saved = LoginTarget::choose(None, None, None, Some(&saved), &cfg);
        assert_eq!(
            (
                from_saved.host.as_str(),
                from_saved.port,
                from_saved.encryption
            ),
            ("play.example.com", 2594, EncryptionMode::Osi)
        );
        let from_config = LoginTarget::choose(None, None, None, Some(&Profile::default()), &cfg);
        assert_eq!(
            from_config,
            LoginTarget {
                host: cfg.host.clone(),
                port: cfg.port,
                encryption: EncryptionMode::None,
            }
        );
        assert_eq!(
            LoginTarget::choose(None, None, None, None, &cfg),
            from_config
        );
    }

    fn typed_login() -> ScreenLogin {
        ScreenLogin {
            host: " play.example.com ".into(),
            port: DEFAULT_LOGIN_PORT,
            account: " acct ".into(),
            password: "pw".into(),
            shard: Some(" ".into()),
            character: Some(" Mara ".into()),
            era: None,
            version: None,
            encryption: EncryptionMode::Osi,
        }
    }

    fn no_screen() -> LoginPicker {
        LoginPicker(tokio::sync::mpsc::unbounded_channel().0)
    }

    #[test]
    fn a_screen_login_takes_the_files_and_the_rules_of_the_config() {
        let cfg = AppConfig {
            uopath: Some(PathBuf::from("/uo")),
            markers: Some(PathBuf::from("/uo/Waypoints.lua")),
            era: Era::T2a,
            obey_shard_rules: false,
            answer_when_named: false,
            play_along: true,
            reconnect: false,
            proxy: Some("socks5://10.0.0.2:1080".parse().unwrap()),
            ..AppConfig::default()
        };
        let opts = ConnectOptions::for_screen(typed_login(), &cfg, no_screen());
        assert_eq!(opts.host, "play.example.com");
        assert_eq!(opts.account, "acct");
        assert_eq!(opts.password, "pw");
        assert_eq!(opts.shard, None, "a blank shard lets the screen pick");
        assert_eq!(opts.character, "Mara");
        assert_eq!(opts.encryption, EncryptionMode::Osi);
        assert_eq!((opts.era, opts.version), (Era::T2a, ClientVersion::T2A));
        assert_eq!(opts.uopath, cfg.uopath);
        assert_eq!(opts.markers, cfg.markers);
        assert!(!opts.obey_shard_rules && !opts.answer_when_named);
        assert!(opts.play_along && !opts.reconnect);
        assert_eq!(opts.proxy, cfg.proxy);
        assert!(opts.picker.is_some());
    }

    #[test]
    fn a_screen_login_names_its_era_and_version_over_the_config() {
        let named = ScreenLogin {
            shard: Some(" Atlantic ".into()),
            character: None,
            era: Some("modern".into()),
            version: Some("7.0.50.0".into()),
            ..typed_login()
        };
        let cfg = AppConfig {
            era: Era::T2a,
            ..AppConfig::default()
        };
        let opts = ConnectOptions::for_screen(named, &cfg, no_screen());
        assert_eq!(opts.shard.as_deref(), Some("Atlantic"));
        assert_eq!(opts.character, "");
        assert_eq!(
            (opts.era, opts.version),
            (Era::Modern, ClientVersion::new(7, 0, 50, 0))
        );
    }

    #[test]
    fn a_web_page_login_reads_with_the_optional_fields_left_out() {
        let login: ScreenLogin = serde_json::from_value(serde_json::json!({
            "host": "127.0.0.1", "port": 2593, "account": "a", "password": "p",
            "shard": null, "character": null, "era": "t2a", "version": null,
        }))
        .unwrap();
        assert_eq!((login.port, login.era.as_deref()), (2593, Some("t2a")));
        assert_eq!((login.shard, login.character), (None, None));
        assert_eq!(login.encryption, EncryptionMode::None);
    }

    #[tokio::test]
    async fn a_question_takes_only_a_reply_of_its_kind() {
        let (reply, answer) = tokio::sync::oneshot::channel();
        let question = LoginQuestion::Characters {
            names: vec!["Mara".into()],
            refused: Some("taken".into()),
            choices: CharacterChoices::default(),
            reply,
        };
        assert_eq!(
            question.ask(),
            LoginAsk::Characters {
                names: vec!["Mara".into()],
                refused: Some("taken".into()),
                choices: CharacterChoices::default(),
            }
        );
        let Err(question) = question.answer(LoginReply::Pick { index: 0 }) else {
            panic!("a pick does not answer the character list");
        };
        let play = LoginReply::Request {
            request: CharacterRequest::Play(0),
        };
        assert!(question.answer(play).is_ok());
        assert_eq!(answer.await.unwrap(), CharacterRequest::Play(0));

        let (reply, answer) = tokio::sync::oneshot::channel();
        let shard = LoginQuestion::Shard {
            names: vec!["Atlantic".into()],
            reply,
        };
        assert_eq!(
            shard.ask(),
            LoginAsk::Shard {
                names: vec!["Atlantic".into()]
            }
        );
        let leave = LoginReply::Request {
            request: CharacterRequest::Leave,
        };
        let Err(shard) = shard.answer(leave) else {
            panic!("a request does not answer the shard list");
        };
        assert!(shard.answer(LoginReply::Pick { index: 0 }).is_ok());
        assert_eq!(answer.await.unwrap(), 0);
    }

    #[test]
    fn a_question_the_login_stopped_waiting_for_takes_its_reply() {
        let (reply, answer) = tokio::sync::oneshot::channel();
        drop(answer);
        let question = LoginQuestion::Shard {
            names: vec!["Atlantic".into()],
            reply,
        };
        assert!(question.answer(LoginReply::Pick { index: 0 }).is_ok());
    }

    #[test]
    fn a_pick_past_the_list_or_an_empty_slot_is_no_answer() {
        let (reply, _answer) = tokio::sync::oneshot::channel();
        let shard = LoginQuestion::Shard {
            names: vec!["Atlantic".into()],
            reply,
        };
        assert!(shard.answer(LoginReply::Pick { index: 1 }).is_err());
        let (reply, _answer) = tokio::sync::oneshot::channel();
        let mut question = LoginQuestion::Characters {
            names: vec!["Mara".into(), String::new()],
            refused: None,
            choices: CharacterChoices::default(),
            reply,
        };
        for slot in [1, 2] {
            for request in [CharacterRequest::Play(slot), CharacterRequest::Delete(slot)] {
                question = question
                    .answer(LoginReply::Request { request })
                    .expect_err("an empty slot is no answer");
            }
        }
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
