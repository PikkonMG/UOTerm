//! Screenshots the player takes with a key, and the one the window takes
//! when the character dies (an option). Each is a PNG file in the
//! `screenshots` folder of the config folder. The file name and the words
//! are `uoterm_view::actions::screenshot`.

use eframe::egui::{self, Event, UserData, ViewportCommand};
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use uoterm_runtime::config::config_dir;
use uoterm_view::actions::screenshot::file_name;

const SCREENSHOTS_DIR: &str = "screenshots";
const FILE_TIME: &str = "%Y-%m-%d_%H-%M-%S%.3f";
/// Screenshots of one moment get these numbers after their time, at most.
const SAME_MOMENT_MAX: u32 = 1000;
const NUMBER_SEPARATOR: char = '_';

/// Marks the pictures the player asked for, so the window's own
/// snapshot does not take them.
struct PlayerScreenshot;

#[derive(Default)]
pub struct Screenshots {
    /// A picture was asked for and has not come yet.
    asked: bool,
}

/// The folder of the screenshots in the config folder `config`.
pub fn screenshots_dir(config: &Path) -> PathBuf {
    config.join(SCREENSHOTS_DIR)
}

/// The time of a screenshot taken now, in words.
fn stamp_now() -> String {
    chrono::Local::now().format(FILE_TIME).to_string()
}

/// A new file for a screenshot taken now, that no other screenshot has:
/// one of the same moment gets a number after its time.
pub fn create_new_file(folder: &Path) -> std::io::Result<(PathBuf, File)> {
    create_numbered(folder, &stamp_now())
}

/// A new file for the screenshot of `stamp`: the plain name, else the
/// first free one of `stamp_1`, `stamp_2` and on.
fn create_numbered(folder: &Path, stamp: &str) -> std::io::Result<(PathBuf, File)> {
    let mut last_error = None;
    for number in 0..=SAME_MOMENT_MAX {
        let stamp = match number {
            0 => stamp.to_string(),
            _ => format!("{stamp}{NUMBER_SEPARATOR}{number}"),
        };
        let path = folder.join(file_name(&stamp));
        match File::options().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => last_error = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or_else(|| ErrorKind::AlreadyExists.into()))
}

impl Screenshots {
    /// Asks the window for a picture of itself. It comes in a later frame.
    pub fn ask(&mut self, ctx: &egui::Context) {
        self.asked = true;
        ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::new(PlayerScreenshot)));
    }

    /// Saves the picture when it came. Gives where it went, or why not.
    pub fn take(&mut self, ctx: &egui::Context) -> Option<Result<PathBuf, String>> {
        if !self.asked {
            return None;
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                Event::Screenshot {
                    image, user_data, ..
                } if user_data
                    .data
                    .as_ref()
                    .is_some_and(|data| data.is::<PlayerScreenshot>()) =>
                {
                    Some(std::sync::Arc::clone(image))
                }
                _ => None,
            })
        })?;
        self.asked = false;
        let folder = screenshots_dir(&config_dir());
        let saved = std::fs::create_dir_all(&folder)
            .and_then(|()| create_new_file(&folder))
            .map_err(|e| e.to_string())
            .and_then(|(path, _)| super::super::save_png(&path, &image).map(|()| path));
        Some(saved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_view::actions::screenshot::FILE_EXTENSION;

    #[test]
    fn two_screenshots_of_one_moment_get_two_files() {
        const STAMP: &str = "2026-10-04_01-02-03.456";
        let folder = std::env::temp_dir().join(format!("uoterm-shots-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        let (first, _) = create_numbered(&folder, STAMP).unwrap();
        let (second, _) = create_numbered(&folder, STAMP).unwrap();
        assert_eq!(first, folder.join(file_name(STAMP)));
        assert_eq!(second, folder.join(file_name(&format!("{STAMP}_1"))));
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_screenshot_file_is_a_png_in_the_folder() {
        let folder = std::env::temp_dir().join(format!("uoterm-shots-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&folder).unwrap();
        let (file, _) = create_new_file(&folder).unwrap();
        assert_eq!(file.parent(), Some(folder.as_path()));
        assert_eq!(
            file.extension().and_then(|e| e.to_str()),
            Some(FILE_EXTENSION)
        );
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
