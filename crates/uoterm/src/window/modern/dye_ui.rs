//! The dye panel of the Modern style, for the dye tub that asks for a
//! colour (0x95) when the client files have no gump art for the color
//! picker gump: the grid of hues with the slider that shifts it, the tub in
//! the picked hue, the eyedropper that takes the hue of a thing the player
//! clicks, and Okay. The grid is the Modern hue grid (`hue_ui`). The shard
//! waits for the colour, so the panel does not close.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::control::Act;
use super::super::model::hue_grid::HuePick;
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, PanelSpec};
use super::hue_ui::{self, HueGridUi};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, Color32, CornerRadius, Pos2, Rect, Vec2};

pub const DYE_ID: &str = "modern:dye";
const TUB_SIDE: f32 = 64.0;
const FOOT_ROW: f32 = 40.0;
const NO_HUE: u16 = 0;

const WORDS_TITLE: &str = "Dye";
const WORDS_OKAY: &str = "Okay";

/// The size of the panel: the grid with its slider, and the tub beside it.
fn panel_size() -> Vec2 {
    let picker = hue_ui::picker_size();
    Vec2::new(
        picker.x + theme::ROW_GAP * 2.0 + TUB_SIDE + theme::PANEL_PAD * 2.0,
        frame::TITLE_ROW + picker.y + FOOT_ROW + theme::PANEL_PAD * 2.0,
    )
}

/// The tub whose colour is picked, and the pick.
#[derive(Default)]
pub struct DyeUi {
    tub: Option<(u32, HuePick)>,
    grid: HueGridUi,
}

impl DyeUi {
    /// Draws the panel while a dye tub waits for its colour. Gives its
    /// place.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Rect> {
        let Some(dye) = frame.dye.as_ref() else {
            self.tub = None;
            return None;
        };
        if self.tub.is_none_or(|(tub, _)| tub != dye.serial) {
            self.tub = Some((dye.serial, HuePick::of(NO_HUE)));
            self.grid.stop();
        }
        let (_, pick) = self.tub.as_mut()?;
        let live = frame.human_control;
        let spec = PanelSpec {
            id: DYE_ID,
            title: WORDS_TITLE,
            default: layout::first_place(rect, Spot::Middle(0), panel_size()),
            min_size: None,
            closable: false,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
        let picker = self.grid.draw(ui, body.min, DYE_ID, pick, tools, live);
        self.grid.take_picked(pick, frame, tools);
        let tub = Rect::from_min_size(
            Pos2::new(picker.right() + theme::ROW_GAP * 2.0, picker.top()),
            Vec2::splat(TUB_SIDE),
        );
        ui.painter()
            .rect_filled(tub, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        if let Some((texture, sprite)) =
            tools.scene.item_picture(frame.map, dye.graphic, pick.hue())
        {
            let shown = theme::fit(tub, sprite.width, sprite.height);
            ui.painter()
                .image(texture, shown, sprite.uv, Color32::WHITE);
        }
        ui.painter().text(
            tub.center_bottom() + Vec2::new(0.0, theme::ROW_GAP),
            Align2::CENTER_TOP,
            pick.hue().to_string(),
            text_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        if live {
            let foot = Pos2::new(body.left(), picker.bottom() + theme::ROW_GAP);
            let (okay_area, okay) = theme::button(ui, foot, WORDS_OKAY, theme::GOAL);
            let at = Pos2::new(okay_area.right() + theme::ROW_GAP, foot.y);
            self.grid.eyedropper(ui, at, DYE_ID, frame, tools);
            if okay {
                tools.hand.act(Act::Dye(pick.hue()));
            }
        }
        frame::controls(ui, panel, &spec, profile, tools);
        Some(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::draw_frames;
    use super::*;
    use crate::view::WatchDye;

    const TUB: u32 = 0x4000_0100;

    #[test]
    fn the_panel_shows_for_a_tub_and_a_new_tub_starts_over() {
        let frame = |serial| WatchFrame {
            human_control: true,
            dye: Some(WatchDye {
                serial,
                graphic: 0x0FAB,
            }),
            ..WatchFrame::default()
        };
        let mut dye = DyeUi::default();
        let mut profile = Profile::default();
        let mut shown = None;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = dye.draw(ui, rect, &frame(TUB), tools, profile);
        });
        assert!(shown.is_some());
        dye.tub = Some((TUB, HuePick::of(1001)));
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = dye.draw(ui, rect, &frame(TUB), tools, profile);
        });
        assert_eq!(
            dye.tub.map(|(_, pick)| pick.hue()),
            Some(1001),
            "the same tub"
        );
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = dye.draw(ui, rect, &frame(TUB + 1), tools, profile);
        });
        assert_eq!(dye.tub, Some((TUB + 1, HuePick::of(NO_HUE))), "another tub");
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = dye.draw(ui, rect, &WatchFrame::default(), tools, profile);
        });
        assert!(shown.is_none() && dye.tub.is_none());
        assert!(panel_size().x > hue_ui::picker_size().x);
    }
}
