//! The dialog of the Modern style for words the shard waits for: a text
//! entry dialog (0xAB) with its title and description, or a prompt (0x9A,
//! 0xC2) whose question is in the journal. The field keeps the rules of the
//! shard (digits only, at most so many chars); Okay or Enter answers, and
//! Cancel or Esc says no when the shard lets the player. It shows whether
//! the chat line shows or not, and while the agent has control, so the
//! operator sees what the shard asks; the clicks work only while the human
//! has control.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::model::asked::{asked_dialog, AskedDialog};
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Vec2};
use uoterm_view::ui::places::FOOT_ROW;
use uoterm_view::ui::shard_asks::{
    entry_first_place, entry_hint, entry_text_room, entry_title, AskedField, ENTRY_ID, FIELD_ROW,
    WORDS_CANCEL, WORDS_OKAY, WORDS_TAKE_CONTROL,
};

const FIELD_ID: &str = "modern-entry-field";

/// The dialog, and the words typed for the question it shows.
#[derive(Default)]
pub struct EntryUi {
    field: AskedField,
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
        let dialog = asked_dialog(frame);
        self.field.follow(dialog.as_ref());
        let dialog = dialog?;
        let live = frame.human_control;
        let title = entry_title(&dialog);
        let description = ui.painter().layout(
            dialog.description.clone(),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
            entry_text_room(),
        );
        let spec = PanelSpec {
            id: ENTRY_ID,
            title,
            default: bridge::rect(entry_first_place(bridge::area(rect), description.size().y)),
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
            tools
                .hand
                .act(dialog.commands.answer_act(&self.field.words));
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
        let hint = entry_hint(dialog);
        let mut words = self.field.words.clone();
        let response = ui
            .add_enabled_ui(live, |ui| {
                ui.put(
                    area,
                    egui::TextEdit::singleline(&mut words)
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
            self.field.typed(&words, dialog);
        }
        if live && !self.field.focused {
            response.request_focus();
            self.field.focused = true;
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
        assert_eq!(entry.field.words, "123", "digits only, three at most");
        let prompt = WatchFrame {
            human_control: true,
            prompt: true,
            ..WatchFrame::default()
        };
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = entry.draw(ui, rect, &prompt, tools, profile);
        });
        assert!(
            entry.field.words.is_empty(),
            "a new question has an empty field"
        );
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = entry.draw(ui, rect, &WatchFrame::default(), tools, profile);
        });
        assert!(shown.is_none(), "no question, no dialog");
    }
}
