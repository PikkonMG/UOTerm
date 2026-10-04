//! The question of the Modern style: one question with Yes and No, as the
//! classic client asks before the game quits, and before a journal tab is
//! deleted. It stands in the middle until it is answered; Escape and its
//! close mark answer No.

use super::super::boxes_ui::Tools;
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use eframe::egui::{self, Id, Key, Pos2, Rect, Vec2};
use uoterm_view::ui::question::{
    question_height, QUESTION_BUTTON_ROW as BUTTON_ROW, QUESTION_BUTTON_WIDTH as BUTTON_WIDTH,
    QUESTION_GAP as GAP, QUESTION_ID, QUESTION_WIDTH as WIDTH, WORDS_NO,
    WORDS_QUESTION as WORDS_TITLE, WORDS_YES,
};

pub use uoterm_view::ui::question::{Asked, Question};

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
    let size = Vec2::new(WIDTH, question_height(galley.size().y));
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
