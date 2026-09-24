//! Where profiles are kept: one global default, and one for each character
//! of each shard. A character with no profile of its own starts from the
//! global default.

use super::super::kept;
use super::Profile;
use serde::Deserialize;
use std::fmt::Display;
use std::path::PathBuf;
use uoterm_runtime::config::config_dir;

const PROFILES_DIR: &str = "profiles";
const DEFAULT_PROFILE_FILE: &str = "default.toml";
const PROFILE_EXTENSION: &str = "toml";
/// The sound settings of older windows. They are read one time, when there
/// is no global profile yet.
const OLD_AUDIO_FILE: &str = "watch-audio.toml";
/// Stands for any byte that may not be in a file name.
const ESCAPE: char = '%';
/// The name of a file that has no name left.
const EMPTY_NAME: &str = "%";

/// The shard a profile belongs to, as the login server's `host:port`.
pub fn shard_address(host: &str, port: impl Display) -> String {
    format!("{}:{}", host.trim().to_ascii_lowercase(), port)
}

/// One character of one shard.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterKey {
    shard: String,
    name: String,
}

impl CharacterKey {
    /// None when the shard or the name is empty.
    pub fn new(shard: &str, name: &str) -> Option<Self> {
        let (shard, name) = (shard.trim(), name.trim());
        (!shard.is_empty() && !name.is_empty()).then(|| Self {
            shard: shard.to_string(),
            name: name.to_string(),
        })
    }
}

/// Words made safe for a file name: letters, digits, `-` and `_` stay, and
/// each other byte becomes `%` and two hex digits. A dot stays too, but not
/// at the start, so no name is hidden or means a parent folder.
fn file_safe(words: &str) -> String {
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

/// The sound settings of older windows, before profiles.
#[derive(Default, Deserialize)]
struct OldAudio {
    muted: Option<bool>,
    master: Option<f32>,
    music: Option<f32>,
    effects: Option<f32>,
    footsteps: Option<f32>,
}

/// The profile files in one folder.
pub struct ProfileStore {
    dir: PathBuf,
    old_audio: PathBuf,
}

impl ProfileStore {
    /// The profiles in the config folder of UOTerm.
    pub fn in_config() -> Self {
        let config = config_dir();
        Self::at(config.join(PROFILES_DIR), config.join(OLD_AUDIO_FILE))
    }

    fn at(dir: PathBuf, old_audio: PathBuf) -> Self {
        Self { dir, old_audio }
    }

    fn default_path(&self) -> PathBuf {
        self.dir.join(DEFAULT_PROFILE_FILE)
    }

    fn character_path(&self, key: &CharacterKey) -> PathBuf {
        self.dir
            .join(file_safe(&key.shard))
            .join(format!("{}.{PROFILE_EXTENSION}", file_safe(&key.name)))
    }

    /// The global default profile. With none kept yet, it is the defaults
    /// with the volumes of the older sound settings, when there are some.
    pub fn load_default(&self) -> Profile {
        let path = self.default_path();
        if path.exists() {
            return kept::load_from(&path);
        }
        let mut profile = Profile::default();
        if self.old_audio.exists() {
            let old: OldAudio = kept::load_from(&self.old_audio);
            let sound = &mut profile.sound;
            sound.muted = old.muted.unwrap_or(sound.muted);
            sound.master_volume = old.master.unwrap_or(sound.master_volume);
            sound.music_volume = old.music.unwrap_or(sound.music_volume);
            sound.sound_volume = old.effects.unwrap_or(sound.sound_volume);
            sound.footsteps_volume = old.footsteps.unwrap_or(sound.footsteps_volume);
        }
        profile
    }

    /// The profile of one character, or the global default when the
    /// character has none yet.
    pub fn load_character(&self, key: &CharacterKey) -> Profile {
        let path = self.character_path(key);
        if path.exists() {
            kept::load_from(&path)
        } else {
            self.load_default()
        }
    }

    pub fn save_default(&self, profile: &Profile) {
        kept::save_to(&self.default_path(), profile);
    }

    pub fn save_character(&self, key: &CharacterKey, profile: &Profile) {
        kept::save_to(&self.character_path(key), profile);
    }
}

/// Where the window's profile comes from and goes to. Before the first
/// frame names the character, and on a shard of unknown address, it is the
/// global default profile.
pub struct ProfileHome {
    store: ProfileStore,
    shard: Option<String>,
    character: Option<CharacterKey>,
    /// The window asked for the character's profile already.
    followed: bool,
}

impl ProfileHome {
    /// `shard` is the login server's address, from [`shard_address`].
    pub fn new(shard: Option<String>) -> Self {
        Self::with_store(ProfileStore::in_config(), shard)
    }

    /// A home whose profiles lie in `dir`, so a test never writes the
    /// player's own profiles.
    #[cfg(test)]
    pub fn in_dir(dir: &std::path::Path) -> Self {
        Self::with_store(
            ProfileStore::at(dir.join(PROFILES_DIR), dir.join(OLD_AUDIO_FILE)),
            None,
        )
    }

    fn with_store(store: ProfileStore, shard: Option<String>) -> Self {
        Self {
            store,
            shard,
            character: None,
            followed: false,
        }
    }

    /// The profile the window starts with: the global default.
    pub fn first_profile(&self) -> Profile {
        self.store.load_default()
    }

    /// The character's own profile, the first time a frame names the
    /// character. None before a name, after the first time, and when the
    /// shard is not known: the global profile stays then.
    pub fn follow(&mut self, character_name: &str) -> Option<Profile> {
        if self.followed || character_name.trim().is_empty() {
            return None;
        }
        self.followed = true;
        let key = CharacterKey::new(self.shard.as_deref()?, character_name)?;
        let profile = self.store.load_character(&key);
        self.character = Some(key);
        Some(profile)
    }

    /// Keeps the profile: the character's own, or the global default.
    pub fn save(&self, profile: &Profile) {
        match &self.character {
            Some(key) => self.store.save_character(key, profile),
            None => self.store.save_default(profile),
        }
    }

    /// Makes the profile the global default, the start of each new
    /// character.
    pub fn save_as_default(&self, profile: &Profile) {
        self.store.save_default(profile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::settings::{GumpPlace, KeyBinding, MacroStep, UiStyle};
    use std::path::Path;

    const SHARD: &str = "play.example.com:2593";

    /// A new folder for one test's profiles.
    fn test_dir() -> PathBuf {
        std::env::temp_dir().join(format!("uoterm-profiles-{}", uuid::Uuid::new_v4()))
    }

    fn test_store(dir: &Path) -> ProfileStore {
        ProfileStore::at(dir.join(PROFILES_DIR), dir.join(OLD_AUDIO_FILE))
    }

    #[test]
    fn a_profile_comes_back_from_its_file() {
        let dir = test_dir();
        let store = test_store(&dir);
        let key = CharacterKey::new(SHARD, "Mara").unwrap();
        let mut profile = Profile::default();
        profile.general.always_run = true;
        profile.sound.music_volume = 0.25;
        // The style that is not the default, so the file must carry it.
        profile.interface.ui_style = UiStyle::Classic;
        profile.folded.insert("paperdoll:5".into());
        profile.looks.insert("buffs".into(), 2);
        profile.macros.key_bindings.push(KeyBinding {
            chord: Some("Ctrl+H".parse().unwrap()),
            steps: vec![MacroStep::new("cast", "Heal")],
            ..KeyBinding::default()
        });
        profile.gumps.insert(
            "paperdoll".into(),
            GumpPlace {
                x: 40.0,
                y: 60.0,
                size: Some((260.0, 300.0)),
                locked: true,
            },
        );
        assert_ne!(
            profile.interface.ui_style,
            Profile::default().interface.ui_style
        );
        store.save_character(&key, &profile);
        assert_eq!(store.load_character(&key), profile);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_file_that_names_few_options_gives_the_defaults_for_the_rest() {
        let dir = test_dir();
        let store = test_store(&dir);
        std::fs::create_dir_all(dir.join(PROFILES_DIR)).unwrap();
        std::fs::write(
            store.default_path(),
            "[general]\nalways_run = true\n[sound]\nmuted = true\n",
        )
        .unwrap();
        let mut expected = Profile::default();
        expected.general.always_run = true;
        expected.sound.muted = true;
        assert_eq!(store.load_default(), expected);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_new_character_starts_from_the_global_default_and_keeps_its_own() {
        let dir = test_dir();
        let mut home = ProfileHome::with_store(test_store(&dir), Some(SHARD.into()));
        let mut global = home.first_profile();
        assert_eq!(global, Profile::default());
        global.video.fps = 144;
        home.save_as_default(&global);
        assert!(home.follow("").is_none());
        let mut own = home.follow("Mara").unwrap();
        assert_eq!(own, global);
        assert!(home.follow("Mara").is_none());
        own.video.fps = 30;
        home.save(&own);
        let again = ProfileHome::with_store(test_store(&dir), Some(SHARD.into()));
        assert_eq!(again.first_profile().video.fps, 144);
        let mut again = again;
        assert_eq!(again.follow("Mara").unwrap().video.fps, 30);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn with_no_shard_address_the_global_profile_stays() {
        let dir = test_dir();
        let mut home = ProfileHome::with_store(test_store(&dir), None);
        assert!(home.follow("Mara").is_none());
        let mut profile = home.first_profile();
        profile.general.hide_roofs = true;
        home.save(&profile);
        assert!(home.first_profile().general.hide_roofs);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_older_sound_settings_are_read_when_there_is_no_profile() {
        let dir = test_dir();
        let store = test_store(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(OLD_AUDIO_FILE), "muted = true\nmusic = 0.1\n").unwrap();
        let sound = store.load_default().sound;
        assert!(sound.muted);
        assert_eq!(sound.music_volume, 0.1);
        assert_eq!(sound.master_volume, Profile::default().sound.master_volume);
        let mut profile = store.load_default();
        profile.sound.music_volume = 0.9;
        store.save_default(&profile);
        assert_eq!(store.load_default().sound.music_volume, 0.9);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn names_become_safe_file_names_that_do_not_meet() {
        assert_eq!(file_safe("Mara"), "Mara");
        let store = test_store(Path::new("x"));
        let key = CharacterKey::new(SHARD, "Mara.Jr").unwrap();
        assert!(store.character_path(&key).ends_with("Mara.Jr.toml"));
        assert_eq!(
            file_safe("play.example.com:2593"),
            "play.example.com%3A2593"
        );
        assert_eq!(file_safe("../x"), "%2E.%2Fx");
        assert_eq!(file_safe(""), EMPTY_NAME);
        assert_ne!(file_safe("a b"), file_safe("a_b"));
        assert_eq!(
            shard_address(" Play.Example.com ", 2593),
            "play.example.com:2593"
        );
    }
}
