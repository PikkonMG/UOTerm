//! What the window and the web client keep between runs, as small TOML
//! files in the config folder: the profiles of the options, the hotbar.

use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use uoterm_runtime::config::{config_dir, write_whole};

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

/// Writes `value` to `path` as one change, and its folder when there is
/// none: a window and a web page that save at one time never leave half a
/// file. False when it was not written; the reason goes to the log.
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
    match write_whole(path, text) {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!(error = %e, path = %path.display(), "settings not saved");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn a_saved_value_is_read_back_and_leaves_no_part_file() {
        let dir = std::env::temp_dir().join(format!("uoterm-kept-{}", uuid::Uuid::new_v4()));
        let path = dir.join("hotbar.toml");
        let value = BTreeMap::from([("slot".to_string(), 1)]);
        assert!(save_to(&path, &value));
        assert_eq!(load_from::<BTreeMap<String, i32>>(&path), value);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
