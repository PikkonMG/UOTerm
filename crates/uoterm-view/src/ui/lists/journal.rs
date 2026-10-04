//! The rules of the journal panel: its sizes, the words of the people who
//! wait for an answer, the turns of the wheel, the color of a line, and
//! the filters under the search.

use crate::geom::Vector;
use crate::model::journal::{Entry, Origin};
use crate::settings::JournalOptions;

/// The journal stands at the bottom right, as wide as this.
pub const JOURNAL_WIDTH: f32 = 400.0;
pub const JOURNAL_HEIGHT: f32 = 330.0;
pub const JOURNAL_LEAST: Vector = Vector::new(260.0, 200.0);

pub const WORDS_JOURNAL: &str = "Journal";
pub const WORDS_SAVE: &str = "Save";
pub const WORDS_NO_LINES: &str = "No lines yet.";
pub const WORDS_SAVED: &str = "Saved to";
pub const WORDS_BACK: &str = "lines back";
pub const WORDS_NEW_TAB: &str = "+";
pub const WORDS_RENAME: &str = "Rename";
pub const WORDS_DELETE_TAB: &str = "Delete tab";
pub const HINT_SEARCH: &str = "search the journal";
pub const HINT_NEW_TAB: &str = "Add a tab.";
pub const HINT_TAB: &str = "Right-click: rename, kinds of lines, delete.";
pub const HINT_TAB_NAME: &str = "tab name";
/// How long the words about a save stay over the search, in seconds.
pub const JOURNAL_NOTE_SECONDS: f64 = 6.0;

/// The option of the Journal page one filter flips.
pub type FilterOption = fn(&mut JournalOptions) -> &mut bool;

/// The filters under the search: their words, and the option each flips.
pub const JOURNAL_FILTERS: [(&str, FilterOption); 4] = [
    ("Shard", |options| &mut options.show_system_lines),
    ("Things", |options| &mut options.show_object_lines),
    ("Client", |options| &mut options.show_client_lines),
    ("Guild", |options| &mut options.show_guild_and_alliance),
];

/// The color of the words of a journal line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordsColor {
    /// The dim words of the window.
    Dim,
    /// The plain words of the window.
    Plain,
    /// The hue the shard gave the line.
    Hue(u16),
}

pub fn journal_waiting_words(persons: usize) -> String {
    if persons == 1 {
        "1 person waits for an answer".to_string()
    } else {
        format!("{persons} persons wait for an answer")
    }
}

/// The words after a save of the journal to `place`.
pub fn saved_words(place: &str) -> String {
    format!("{WORDS_SAVED} {place}")
}

/// The words over the lines while the player reads back.
pub fn back_words(back: usize) -> String {
    format!("{back} {WORDS_BACK}")
}

/// The whole turns of the wheel: up reads back.
pub fn journal_wheel_turns(notches: f32) -> i32 {
    (notches.signum() * notches.abs().ceil()) as i32
}

/// The color of a line's words: the hue the shard gave it, or the plain
/// colors of the window.
pub fn journal_words_color(entry: &Entry) -> WordsColor {
    match entry.origin {
        Origin::System | Origin::Client => WordsColor::Dim,
        Origin::Mobile | Origin::Object if entry.hue == 0 => WordsColor::Plain,
        Origin::Mobile | Origin::Object => WordsColor::Hue(entry.hue),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::JournalKind;

    #[test]
    fn one_person_waits_and_two_persons_wait() {
        assert_eq!(journal_waiting_words(1), "1 person waits for an answer");
        assert_eq!(journal_waiting_words(2), "2 persons wait for an answer");
    }

    #[test]
    fn a_part_of_a_notch_is_a_whole_turn_each_way() {
        assert_eq!(journal_wheel_turns(0.2), 1);
        assert_eq!(journal_wheel_turns(-1.5), -2);
        assert_eq!(journal_wheel_turns(0.0), 0);
    }

    #[test]
    fn each_filter_flips_its_own_option() {
        let mut options = JournalOptions::default();
        for (_, option) in JOURNAL_FILTERS {
            let before = *option(&mut options);
            *option(&mut options) = !before;
            assert_ne!(*option(&mut options), before);
        }
        assert_ne!(options, JournalOptions::default());
    }

    #[test]
    fn the_shard_lines_are_dim_and_a_hued_line_keeps_its_hue() {
        let line = |origin, hue| Entry {
            serial: 0,
            name: String::new(),
            text: String::new(),
            hue,
            kind: JournalKind::Speech,
            origin,
            stamp: String::new(),
        };
        assert_eq!(
            journal_words_color(&line(Origin::System, 5)),
            WordsColor::Dim
        );
        assert_eq!(
            journal_words_color(&line(Origin::Mobile, 0)),
            WordsColor::Plain
        );
        assert_eq!(
            journal_words_color(&line(Origin::Object, 5)),
            WordsColor::Hue(5)
        );
    }
}
