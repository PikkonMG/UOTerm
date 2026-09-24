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
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Pos2, Rect, Vec2};

pub const TIP_ID: &str = "modern:tip";
const SIZE: Vec2 = Vec2::new(360.0, 320.0);
const LEAST_SIZE: Vec2 = Vec2::new(240.0, 180.0);
const FOOT_ROW: f32 = 40.0;

const WORDS_TIP: &str = "Tip of the day";
const WORDS_NOTICE: &str = "Notice";
const WORDS_PREVIOUS: &str = "Previous";
const WORDS_NEXT: &str = "Next";

/// The words the player closed, so they stay hidden until the shard sends
/// others.
#[derive(Default)]
pub struct TipUi {
    closed: Option<String>,
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
        let Some(words) = frame.shard_notice.as_deref() else {
            self.closed = None;
            return None;
        };
        if self.closed.as_deref() == Some(words) {
            return None;
        }
        let tip = frame.shard_tip.is_some();
        let title = if tip { WORDS_TIP } else { WORDS_NOTICE };
        let spec = PanelSpec {
            id: TIP_ID,
            title,
            default: layout::first_place(rect, Spot::Middle(0), SIZE),
            min_size: Some(LEAST_SIZE),
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
            self.closed = Some(words.to_string());
        }
        Some(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::draw_frames;
    use super::*;

    #[test]
    fn the_words_show_until_closed_and_new_words_show_again() {
        let notice = |words: &str| WatchFrame {
            shard_notice: Some(words.into()),
            ..WatchFrame::default()
        };
        let mut tip = TipUi::default();
        let mut profile = Profile::default();
        let mut shown = None;
        let mut draw = |tip: &mut TipUi, frame: &WatchFrame, profile: &mut Profile| {
            draw_frames(profile, &[Vec::new()], |ui, rect, tools, profile| {
                shown = tip.draw(ui, rect, frame, tools, profile);
            });
            shown
        };
        assert!(draw(&mut tip, &notice("Hail"), &mut profile).is_some());
        tip.closed = Some("Hail".into());
        assert!(
            draw(&mut tip, &notice("Hail"), &mut profile).is_none(),
            "closed"
        );
        assert!(draw(&mut tip, &notice("Welcome"), &mut profile).is_some());
        assert!(draw(&mut tip, &WatchFrame::default(), &mut profile).is_none());
        assert_eq!(tip.closed, None, "no words, nothing closed");
    }
}
