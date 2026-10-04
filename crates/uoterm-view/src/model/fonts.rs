//! The TrueType fonts the player can give the Modern style: the files of
//! the `Fonts` folder of the config folder. The Fonts page names one by
//! its file name. The host reads the folder and the files; the rules of
//! which file is a font and which one the page names are here.

const FONT_EXTENSIONS: [&str; 2] = ["ttf", "otf"];
const EXTENSION_MARK: char = '.';

/// True when a file name ends in the extension of a font.
pub fn is_font(name: &str) -> bool {
    name.rsplit_once(EXTENSION_MARK)
        .is_some_and(|(stem, extension)| {
            !stem.is_empty()
                && FONT_EXTENSIONS
                    .iter()
                    .any(|known| extension.eq_ignore_ascii_case(known))
        })
}

/// The font of the folder that the Fonts page names, from the file names
/// of the folder. None when no font has that name.
pub fn resolve(names: &[String], wanted: &str) -> Option<String> {
    names
        .iter()
        .find(|name| name.as_str() == wanted && is_font(name))
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_font_of_the_folder_is_found_by_its_name() {
        let names = vec!["Avadonian.TTF".to_string(), "readme.txt".to_string()];
        assert_eq!(
            resolve(&names, "Avadonian.TTF").as_deref(),
            Some("Avadonian.TTF")
        );
        assert_eq!(resolve(&names, "readme.txt"), None);
        assert_eq!(resolve(&names, "gone.ttf"), None);
        assert!(is_font("serif.otf") && !is_font("ttf") && !is_font(".ttf"));
    }
}
