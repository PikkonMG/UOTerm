//! The question of the Modern style: one question with Yes and No, as the
//! classic client asks before the game quits, and before a journal tab is
//! deleted. It stands in the middle until it is answered; Escape and its
//! close mark answer No.

use super::super::boxes_ui::Tools;
use super::super::classic::message_box::QUIT_WORDS;
use super::super::model::journal;
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use eframe::egui::{self, Id, Key, Pos2, Rect, Vec2};

const QUESTION_ID: &str = "modern:question";
const WIDTH: f32 = 320.0;
const BUTTON_ROW: f32 = 30.0;
const BUTTON_WIDTH: f32 = 90.0;
const GAP: f32 = 10.0;
const WORDS_TITLE: &str = "Question";
const WORDS_YES: &str = "Yes";
const WORDS_NO: &str = "No";

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

/// Draws the question. Gives its place, and the answer when the player
/// gave one.
pub fn draw(
    ui: &egui::Ui,
    rect: Rect,
    question: &Question,
    tools: &Tools<'_>,
    profile: &mut Profile,
) -> (Rect, Option<bool>) {
    let galley = ui.painter().layout(
        question.words.clone(),
        text_font(theme::SIZE_BODY),
        theme::TEXT,
        WIDTH - theme::PANEL_PAD * 2.0,
    );
    let size = Vec2::new(
        WIDTH,
        frame::TITLE_ROW + galley.size().y + GAP + BUTTON_ROW + theme::PANEL_PAD * 2.0,
    );
    let spec = PanelSpec {
        id: QUESTION_ID,
        title: WORDS_TITLE,
        default: layout::first_place(rect, Spot::Middle(0), size),
        min_size: None,
        closable: true,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
    ui.painter().galley(body.min, galley, theme::TEXT);
    let buttons_top = body.bottom() - BUTTON_ROW;
    let yes_area = Rect::from_min_size(
        Pos2::new(body.center().x - GAP / 2.0 - BUTTON_WIDTH, buttons_top),
        Vec2::new(BUTTON_WIDTH, BUTTON_ROW),
    );
    let no_area = yes_area.translate(Vec2::new(BUTTON_WIDTH + GAP, 0.0));
    let yes = theme::segment_keyed(
        ui,
        yes_area,
        Id::new("question-yes"),
        WORDS_YES,
        theme::GOAL,
    );
    let no = theme::segment_keyed(ui, no_area, Id::new("question-no"), WORDS_NO, theme::TEXT);
    let escaped = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape));
    let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
    let answer = if yes {
        Some(true)
    } else if no || escaped || closed {
        Some(false)
    } else {
        None
    };
    (panel, answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_questions_carry_their_words_and_what_yes_does() {
        assert_eq!(Question::quit().words, QUIT_WORDS);
        assert_eq!(Question::quit().asked, Asked::Quit);
        let delete = Question::delete_journal_tab("Chat");
        assert_eq!(delete.words, "Delete [Chat] tab?");
        assert_eq!(delete.asked, Asked::DeleteJournalTab("Chat".into()));
    }
}
