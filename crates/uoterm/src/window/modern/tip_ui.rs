//! The tip of the day or the notice of the shard (`0xA6`) in the Modern
//! style: the words in a panel the player moves and sizes, with a scroll
//! for long words. A tip has the buttons that ask the shard for the tip
//! before or after it (`0xA7`). The close mark hides the words until the
//! shard sends others.

use super::super::boxes_ui::Tools;
use super::super::control::Act;
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, FrameEvent, PanelSpec};
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Pos2, Rect};
use uoterm_view::ui::places::FOOT_ROW;
use uoterm_view::ui::shard_asks::{
    tip_first_place, NoticePanel, TIP_ID, TIP_LEAST_SIZE, WORDS_NEXT, WORDS_PREVIOUS,
};

/// The words the player closed, so they stay hidden until the shard sends
/// others.
#[derive(Default)]
pub struct TipUi {
    notice: NoticePanel,
}

impl TipUi {
    /// Draws the words of the shard while they show. Gives its place.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        self.notice.follow(frame);
        let shown = self.notice.shown(frame)?;
        let (title, words, tip) = (shown.title, shown.words, shown.tip);
        let spec = PanelSpec {
            id: TIP_ID,
            title,
            default: bridge::rect(tip_first_place(bridge::area(rect))),
            min_size: Some(bridge::vec2(TIP_LEAST_SIZE)),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, title);
        let foot_room = if tip { FOOT_ROW } else { 0.0 };
        let text = Rect::from_min_max(body.min, Pos2::new(body.right(), body.bottom() - foot_room));
        ui.scope_builder(egui::UiBuilder::new().max_rect(text), |ui| {
            egui::ScrollArea::vertical()
                .id_salt(TIP_ID)
                .auto_shrink(false)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(words)
                                .font(text_font(theme::SIZE_BODY))
                                .color(theme::TEXT),
                        )
                        .wrap(),
                    );
                });
        });
        if tip && frame.human_control {
            let foot = Pos2::new(body.left(), text.bottom() + theme::ROW_GAP);
            let (previous, back) = theme::button(ui, foot, WORDS_PREVIOUS, theme::TEXT);
            let at = Pos2::new(previous.right() + theme::ROW_GAP, foot.y);
            let (_, on) = theme::button(ui, at, WORDS_NEXT, theme::TEXT);
            if back || on {
                tools.hand.act(Act::Tip { next: on });
            }
        }
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            self.notice.close(words);
        }
        Some(panel)
    }
}
