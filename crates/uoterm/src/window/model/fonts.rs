//! The TrueType fonts the player can give the Modern style: the files of
//! the `Fonts` folder of the config folder. The Fonts page names one by
//! its file name, or by a whole path.

use std::path::{Path, PathBuf};

const FONTS_DIR: &str = "Fonts";
const FONT_EXTENSIONS: [&str; 2] = ["ttf", "otf"];

/// The folder of the player's fonts.
pub fn fonts_dir() -> PathBuf {
    uoterm_runtime::config::config_dir().join(FONTS_DIR)
}

fn is_font(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        FONT_EXTENSIONS
            .iter()
            .any(|known| e.eq_ignore_ascii_case(known))
    })
}

/// The font files of a folder, by name.
pub fn fonts_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut fonts: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| is_font(path))
        .collect();
    fonts.sort();
    fonts
}

/// The file a chosen font names: a bare file name lies in `dir`.
pub fn resolve(dir: &Path, chosen: &Path) -> PathBuf {
    if chosen.is_absolute() || chosen.components().count() > 1 {
        chosen.to_path_buf()
    } else {
        dir.join(chosen)
    }
}

/// The bytes of the chosen font. None when it is not a font file that
/// reads.
pub fn load(dir: &Path, chosen: &Path) -> Option<Vec<u8>> {
    let path = resolve(dir, chosen);
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
