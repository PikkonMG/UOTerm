//! The rules of screenshots, apart from how a window takes the picture
//! and where it keeps the file: the file name, the words of the journal,
//! and the death that takes one when the Interface page asks for it.

use crate::frame::WatchFrame;

const FILE_PREFIX: &str = "Screenshot_";
pub const FILE_EXTENSION: &str = "png";

/// The name of a screenshot file, from the time it was taken in words.
pub fn file_name(stamp: &str) -> String {
    format!("{FILE_PREFIX}{stamp}.{FILE_EXTENSION}")
}

/// The words the journal shows for a saved screenshot.
pub fn stored_words(place: &str) -> String {
    format!("Screenshot stored in: {place}")
}

/// The words the journal shows when a screenshot was not saved.
pub fn failed_words(why: &str) -> String {
    format!("The screenshot was not saved: {why}")
}

/// Sees the death of the character once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DeathWatch {
    /// The character was dead in the last picture.
    was_dead: bool,
}

impl DeathWatch {
    /// True when the character died since the last picture.
    pub fn died(&mut self, frame: &WatchFrame) -> bool {
        let died = frame.dead && !self.was_dead;
        self.was_dead = frame.dead;
        died
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_death_is_seen_once() {
        let mut deaths = DeathWatch::default();
        let mut frame = WatchFrame::default();
        assert!(!deaths.died(&frame));
        frame.dead = true;
        assert!(deaths.died(&frame));
        assert!(!deaths.died(&frame));
        frame.dead = false;
        assert!(!deaths.died(&frame));
    }

    #[test]
    fn a_screenshot_file_is_a_png_named_by_its_time() {
        let name = file_name("2026-10-04_01-02-03.456");
        assert_eq!(name, "Screenshot_2026-10-04_01-02-03.456.png");
        assert!(stored_words("shots/a.png").starts_with("Screenshot stored in: shots"));
        assert!(failed_words("no room").ends_with("no room"));
    }
}
