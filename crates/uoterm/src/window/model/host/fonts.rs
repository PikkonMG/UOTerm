//! The `Fonts` folder of the config folder: the font files the player put
//! there, and the bytes of the one the Fonts page names, by its file name
//! or by a whole path.

use std::path::{Path, PathBuf};
use uoterm_view::model::fonts;

const FONTS_DIR: &str = "Fonts";

/// The folder of the player's fonts in the config folder `config`.
pub fn fonts_dir(config: &Path) -> PathBuf {
    config.join(FONTS_DIR)
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name()?.to_str()
}

fn is_font(path: &Path) -> bool {
    file_name(path).is_some_and(fonts::is_font)
}

/// The font files of a folder, by name.
pub fn fonts_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_font(path))
        .collect();
    found.sort();
    found
}

/// The file names of the font files of a folder, by name.
pub fn font_names(dir: &Path) -> Vec<String> {
    fonts_in(dir)
        .iter()
        .filter_map(|path| file_name(path).map(str::to_string))
        .collect()
}

/// The bytes of the chosen font. A bare file name lies in `dir`. None when
/// it is not a font file that reads.
pub fn load(dir: &Path, chosen: &Path) -> Option<Vec<u8>> {
    let path = if chosen.is_absolute() || chosen.components().count() > 1 {
        chosen.to_path_buf()
    } else {
        dir.join(fonts::resolve(&font_names(dir), chosen.to_str()?)?)
    };
    is_font(&path).then(|| std::fs::read(path).ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fonts_of_the_folder_are_found_by_name_or_by_path() {
        let dir = std::env::temp_dir().join(format!("uoterm-fonts-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Avadonian.TTF"), b"font").unwrap();
        std::fs::write(dir.join("readme.txt"), b"words").unwrap();
        let found = fonts_in(&dir);
        assert_eq!(found, vec![dir.join("Avadonian.TTF")]);
        assert_eq!(font_names(&dir), ["Avadonian.TTF"]);
        assert_eq!(
            load(&dir, Path::new("Avadonian.TTF")),
            Some(b"font".to_vec())
        );
        assert_eq!(
            load(&dir, &dir.join("Avadonian.TTF")),
            Some(b"font".to_vec())
        );
        assert_eq!(load(&dir, Path::new("readme.txt")), None);
        assert_eq!(load(&dir, Path::new("gone.ttf")), None);
        assert!(fonts_in(&dir.join("none")).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
