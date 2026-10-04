//! The question of the Modern style: one question with Yes and No, as the
//! classic client asks before the game quits, and before a journal tab is
//! deleted, and what its Yes does. It stands in the middle until it is
//! answered; Escape and its close mark answer No.

use super::layout::{first_place, Spot};
use super::places::TITLE_ROW;
use super::theme::PANEL_PAD;
use crate::geom::{Area, Vector};
use crate::model::journal;

/// The id that keeps the place of the question in the profile.
pub const QUESTION_ID: &str = "modern:question";
pub const QUESTION_WIDTH: f32 = 320.0;
pub const QUESTION_BUTTON_ROW: f32 = 30.0;
pub const QUESTION_BUTTON_WIDTH: f32 = 90.0;
pub const QUESTION_GAP: f32 = 10.0;
pub const WORDS_QUESTION: &str = "Question";
pub const WORDS_YES: &str = "Yes";
pub const WORDS_NO: &str = "No";
/// The reference client's words for the question before the game quits.
pub const QUIT_WORDS: &str = "Quit\nUltima Online?";
/// The height of one line of the words of a question, as a page lays
/// them.
const WORDS_LINE: f32 = 18.0;
const LINE_BREAK: char = '\n';

/// What a Yes does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    Quit,
    /// Deletes the journal tab of this name.
    DeleteJournalTab(String),
}

/// One question and what its Yes does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub words: String,
    pub asked: Asked,
}

impl Question {
    /// The question before the game quits, in the words of the classic
    /// client.
    pub fn quit() -> Self {
        Self {
            words: QUIT_WORDS.to_string(),
            asked: Asked::Quit,
        }
    }

    /// The question before a journal tab is deleted.
    pub fn delete_journal_tab(name: &str) -> Self {
        Self {
            words: journal::delete_question(name),
            asked: Asked::DeleteJournalTab(name.to_string()),
        }
    }
}

/// The height of a question panel whose words are `words_height` high.
pub fn question_height(words_height: f32) -> f32 {
    TITLE_ROW + words_height + QUESTION_GAP + QUESTION_BUTTON_ROW + PANEL_PAD * 2.0
}

/// Where a question first stands in `window`, for a page that lays its
/// words one line for each line they hold.
pub fn question_first_place(window: Area, words: &str) -> Area {
    let lines = words.split(LINE_BREAK).count() as f32;
    let size = Vector::new(QUESTION_WIDTH, question_height(lines * WORDS_LINE));
    first_place(window, Spot::Middle(0), size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;

    #[test]
    fn the_questions_carry_their_words_and_what_yes_does() {
        assert_eq!(Question::quit().words, QUIT_WORDS);
        assert_eq!(Question::quit().asked, Asked::Quit);
        let delete = Question::delete_journal_tab("Chat");
        assert_eq!(delete.words, "Delete [Chat] tab?");
        assert_eq!(delete.asked, Asked::DeleteJournalTab("Chat".into()));
    }

    #[test]
    fn a_question_of_two_lines_is_taller_than_one_of_one() {
        let window = Area::from_min_size(Point::default(), Vector::new(1280.0, 800.0));
        let two = question_first_place(window, QUIT_WORDS);
        let one = question_first_place(window, "Delete [Chat] tab?");
        assert!(two.height() > one.height());
        assert_eq!(two.width(), QUESTION_WIDTH);
    }
}
