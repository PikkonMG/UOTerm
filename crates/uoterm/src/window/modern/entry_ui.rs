//! The dialog of the Modern style for words the shard waits for: a text
//! entry dialog (0xAB) with its title and description, or a prompt (0x9A,
//! 0xC2) whose question is in the journal. The field keeps the rules of the
//! shard (digits only, at most so many chars); Okay or Enter answers, and
//! Cancel or Esc says no when the shard lets the player. It shows whether
//! the chat line shows or not, and while the agent has control, so the
//! operator sees what the shard asks; the clicks work only while the human
//! has control.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::model::asked::{asked_dialog, kept_words, AskedDialog};
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Vec2};

pub const ENTRY_ID: &str = "modern:entry";
const WIDTH: f32 = 400.0;
const FIELD_ROW: f32 = 30.0;
const FOOT_ROW: f32 = 40.0;
const FIELD_ID: &str = "modern-entry-field";

const WORDS_TITLE: &str = "The shard asks";
const WORDS_OKAY: &str = "Okay";
const WORDS_CANCEL: &str = "Cancel";
const WORDS_TAKE_CONTROL: &str = "Take control to answer.";
const HINT_WORDS: &str = "Type the answer and press Enter.";
const HINT_DIGITS: &str = "Digits only. Press Enter.";

/// The dialog, and the words typed for the question it shows.
#[derive(Default)]
pub struct EntryUi {
    /// The question the field was made for. A new one gets an empty field.
    shown: Option<AskedDialog>,
    words: String,
    /// The field has had the keys once.
    focused: bool,
}

impl EntryUi {
    /// Draws the dialog while the shard waits for words. Gives its place.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        let Some(dialog) = asked_dialog(frame) else {
            self.shown = None;
            return None;
        };
        if self.shown.as_ref() != Some(&dialog) {
            self.words.clear();
            self.focused = false;
            self.shown = Some(dialog.clone());
        }
        let live = frame.human_control;
        let title = if dialog.title.is_empty() {
            WORDS_TITLE
        } else {
            dialog.title.as_str()
        };
        let text_width = WIDTH - theme::PANEL_PAD * 2.0;
        let description = ui.painter().layout(
            dialog.description.clone(),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
            text_width,
        );
        let height = theme::PANEL_PAD * 2.0
            + frame::TITLE_ROW
            + description.size().y
            + theme::ROW_GAP
            + FIELD_ROW
            + FOOT_ROW;
        let spec = PanelSpec {
            id: ENTRY_ID,
            title,
            default: layout::first_place(rect, Spot::Middle(0), Vec2::new(WIDTH, height)),
            min_size: None,
            closable: live && dialog.can_cancel,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, title);
        let description_height = description.size().y;
        ui.painter()
            .galley(body.left_top(), description, theme::TEXT_DIM);
        let field = Rect::from_min_size(
            body.left_top() + Vec2::new(0.0, description_height + theme::ROW_GAP),
            Vec2::new(body.width(), FIELD_ROW),
        );
        let answer = self.field(ui, field, &dialog, live);
        let foot = Pos2::new(body.left(), field.bottom() + theme::ROW_GAP);
        let mut cancel = false;
        let mut okay = answer;
        if live {
            let (okay_area, pressed) = theme::button(ui, foot, WORDS_OKAY, theme::GOAL);
            okay |= pressed;
            if dialog.can_cancel {
                let at = Pos2::new(okay_area.right() + theme::ROW_GAP, foot.y);
                let (_, pressed) = theme::button(ui, at, WORDS_CANCEL, theme::TEXT_DIM);
                cancel = pressed || ui.input(|i| i.key_pressed(Key::Escape));
            }
        } else {
            ui.painter().text(
                Pos2::new(foot.x, foot.y + FOOT_ROW / 2.0),
                Align2::LEFT_CENTER,
                WORDS_TAKE_CONTROL,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
        }
        let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if okay && live {
            tools.hand.act(dialog.commands.answer_act(&self.words));
        } else if (cancel || closed) && live {
            tools.hand.act(dialog.commands.cancel_act());
        }
        Some(panel)
    }

    /// The field of the answer. It takes the keys once as the dialog
    /// opens. True when Enter answered.
    fn field(&mut self, ui: &mut egui::Ui, area: Rect, dialog: &AskedDialog, live: bool) -> bool {
        ui.painter()
            .rect_filled(area, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let id = Id::new(FIELD_ID);
        let hint = if dialog.numeric {
            HINT_DIGITS
        } else {
            HINT_WORDS
        };
        let response = ui
            .add_enabled_ui(live, |ui| {
                ui.put(
                    area,
                    egui::TextEdit::singleline(&mut self.words)
                        .id(id)
                        .frame(false)
                        .hint_text(hint)
                        .margin(egui::Margin::symmetric(8, 6))
                        .font(text_font(theme::SIZE_BODY))
                        .text_color(theme::TEXT),
                )
            })
            .inner;
        if response.changed() {
            self.words = kept_words(&self.words, dialog.numeric, dialog.max_chars);
        }
        if live && !self.focused {
            response.request_focus();
            self.focused = true;
        }
        response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter))
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{draw_frames, typing};
    use super::*;
    use crate::view::{WatchTextEntry, TEXT_ENTRY_STYLE_NUMERIC};

    #[test]
    fn the_field_takes_the_keys_and_keeps_the_rules_of_the_shard() {
        let asking = WatchFrame {
            human_control: true,
            text_entry: Some(WatchTextEntry {
                title: "How many?".into(),
                style: TEXT_ENTRY_STYLE_NUMERIC,
                max_length: 3,
                ..WatchTextEntry::default()
            }),
            ..WatchFrame::default()
        };
        let mut entry = EntryUi::default();
        let mut profile = Profile::default();
        let mut shown = None;
        draw_frames(
            &mut profile,
            &[Vec::new(), typing("12a345")],
            |ui, rect, tools, profile| shown = entry.draw(ui, rect, &asking, tools, profile),
        );
        assert!(shown.is_some());
        assert_eq!(entry.words, "123", "digits only, three at most");
        let prompt = WatchFrame {
            human_control: true,
            prompt: true,
            ..WatchFrame::default()
        };
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = entry.draw(ui, rect, &prompt, tools, profile);
        });
        assert!(entry.words.is_empty(), "a new question has an empty field");
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = entry.draw(ui, rect, &WatchFrame::default(), tools, profile);
        });
        assert!(shown.is_none(), "no question, no dialog");
    }
}
