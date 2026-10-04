//! The dye panel of the Modern style, for the dye tub that asks for a
//! colour (0x95) when the client files have no gump art for the color
//! picker gump: the grid of hues with the slider that shifts it, the tub in
//! the picked hue, the eyedropper that takes the hue of a thing the player
//! clicks, and Okay. The grid is the Modern hue grid (`hue_ui`). The shard
//! waits for the colour, so the panel does not close.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::control::Act;
use super::super::settings::Profile;
use super::super::theme::{self, text_font};
use super::frame::{self, PanelSpec};
use super::hue_ui::HueGridUi;
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Align2, Color32, CornerRadius, Pos2, Rect, Vec2};
use uoterm_view::ui::hues::{dye_first_place, DyePanel, DYE_ID, TUB_SIDE, WORDS_DYE, WORDS_OKAY};

/// The tub whose colour is picked, the pick, and the grid.
#[derive(Default)]
pub struct DyeUi {
    tub: DyePanel,
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
        let pick = self
            .tub
            .follow(frame.dye.as_ref(), &mut self.grid.eyedropper)?;
        let dye = frame.dye.as_ref()?;
        let live = frame.human_control;
        let spec = PanelSpec {
            id: DYE_ID,
            title: WORDS_DYE,
            default: bridge::rect(dye_first_place(bridge::area(rect))),
            min_size: None,
            closable: false,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_DYE);
        let picker = self.grid.draw(ui, body.min, DYE_ID, pick, tools, live);
        self.grid.take_picked(pick, frame, tools);
        let tub = Rect::from_min_size(
            Pos2::new(picker.right() + theme::ROW_GAP * 2.0, picker.top()),
            Vec2::splat(TUB_SIDE),
        );
        ui.painter()
            .rect_filled(tub, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        if let Some((texture, sprite)) = tools.scene.item_picture(dye.graphic, pick.hue()) {
            let shown = theme::fit(tub, sprite.width, sprite.height);
            ui.painter()
                .image(texture, shown, bridge::rect(sprite.uv), Color32::WHITE);
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
