//! What the window and the web client keep between runs, as small TOML
//! files in the config folder: the profiles of the options, the hotbar.

use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use uoterm_runtime::config::config_dir;

/// The kept value of `file`. A missing or bad file gives the default.
pub fn load<T: DeserializeOwned + Default>(file: &str) -> T {
    load_from(&config_dir().join(file))
}

pub fn load_from<T: DeserializeOwned + Default>(path: &Path) -> T {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| toml::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save<T: Serialize>(file: &str, value: &T) {
    save_to(&config_dir().join(file), value);
}

/// Writes `value` to `path`, and its folder when there is none. False when
/// it was not written; the reason goes to the log.
pub fn save_to<T: Serialize>(path: &Path, value: &T) -> bool {
    let text = match toml::to_string(value) {
        Ok(text) => text,
        Err(e) => {
            tracing::warn!(error = %e, path = %path.display(), "settings not saved");
            return false;
        }
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match std::fs::write(path, text) {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(error = %e, path = %path.display(), "settings not saved");
            false
        }
    }
}
