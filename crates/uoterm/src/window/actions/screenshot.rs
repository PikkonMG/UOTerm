//! Screenshots the player takes with a key, and the one the window takes
//! when the character dies (an option). Each is a PNG file in the
//! `screenshots` folder of the config folder.

use crate::view::WatchFrame;
use eframe::egui::{self, Event, UserData, ViewportCommand};
use std::path::{Path, PathBuf};
use uoterm_runtime::config::config_dir;

const SCREENSHOTS_DIR: &str = "screenshots";
const FILE_PREFIX: &str = "Screenshot_";
const FILE_TIME: &str = "%Y-%m-%d_%H-%M-%S%.3f";
const FILE_EXTENSION: &str = "png";

/// Marks the pictures the player asked for, so the window's own
/// snapshot does not take them.
struct PlayerScreenshot;

#[derive(Default)]
pub struct Screenshots {
    /// A picture was asked for and has not come yet.
    asked: bool,
    /// The character was dead in the last picture.
    was_dead: bool,
}

/// The file of a screenshot taken now.
fn new_file(folder: &Path) -> PathBuf {
    let time = chrono::Local::now().format(FILE_TIME);
    folder.join(format!("{FILE_PREFIX}{time}.{FILE_EXTENSION}"))
}

/// The words the journal shows for a saved screenshot.
pub fn stored_words(path: &Path) -> String {
    format!("Screenshot stored in: {}", path.display())
}

impl Screenshots {
    /// Asks the window for a picture of itself. It comes in a later frame.
    pub fn ask(&mut self, ctx: &egui::Context) {
        self.asked = true;
        ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::new(PlayerScreenshot)));
    }

    /// True when the character died since the last picture.
    pub fn died(&mut self, frame: &WatchFrame) -> bool {
        let died = frame.dead && !self.was_dead;
        self.was_dead = frame.dead;
        died
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
        let folder = config_dir().join(SCREENSHOTS_DIR);
        let path = new_file(&folder);
        let saved = std::fs::create_dir_all(&folder)
            .map_err(|e| e.to_string())
            .and_then(|()| super::super::save_png(&path, &image));
        Some(saved.map(|()| path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_death_is_seen_once() {
        let mut shots = Screenshots::default();
        let mut frame = WatchFrame::default();
        assert!(!shots.died(&frame));
        frame.dead = true;
        assert!(shots.died(&frame));
        assert!(!shots.died(&frame));
        frame.dead = false;
        assert!(!shots.died(&frame));
    }

    #[test]
    fn a_screenshot_file_is_a_png_in_the_folder() {
        let file = new_file(Path::new("shots"));
        assert_eq!(file.parent(), Some(Path::new("shots")));
        assert_eq!(
            file.extension().and_then(|e| e.to_str()),
            Some(FILE_EXTENSION)
        );
        assert!(stored_words(&file).starts_with("Screenshot stored in: shots"));
    }
}
