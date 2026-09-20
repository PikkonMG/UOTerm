//! What the window keeps between runs, as small TOML files in the config
//! folder: the sound volumes, the hotbar.

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

pub fn save_to<T: Serialize>(path: &Path, value: &T) {
    let Ok(text) = toml::to_string(value) else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(path, text) {
        tracing::warn!(error = %e, path = %path.display(), "settings not saved");
    }
}
