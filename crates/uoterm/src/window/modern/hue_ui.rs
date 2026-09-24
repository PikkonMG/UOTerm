//! The grid of hues of the Modern style, for every panel that picks a hue:
//! the cells whose hues step by five from the shade the slider sets, the
//! mark on the picked cell, and the eyedropper that takes the hue of a
//! thing the player clicks in the world or in a panel. The dye panel and
//! the color picker of the Options pick with it; the hues are
//! `model::hue_grid`'s, as the classic color picker has them.

use super::super::actions::LocalAim;
use super::super::boxes_ui::Tools;
use super::super::control::Act;
use super::super::model::hue_grid::{
    grid_hue, hue_of, HuePick, BAD_HUE_WORDS, GRADUATION_MAX, GRADUATION_MIN, GRID_COLUMNS,
    GRID_ROWS,
};
use super::super::theme;
use crate::view::WatchFrame;
use eframe::egui::{self, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2};

pub const CELL: f32 = 14.0;
/// The shade slider under the grid.
pub const SLIDER_ROW: f32 = 28.0;
/// The eyedropper button.
pub const EYEDROPPER_SIZE: Vec2 = Vec2::new(110.0, 28.0);
const MARK_WIDTH: f32 = 2.0;
const WORDS_SHADE: &str = "Shade";
const WORDS_EYEDROPPER: &str = "Eyedropper";

/// The size of the grid.
pub fn grid_size() -> Vec2 {
    Vec2::new(GRID_COLUMNS as f32 * CELL, GRID_ROWS as f32 * CELL)
}

/// The size of the grid with the slider under it.
pub fn picker_size() -> Vec2 {
    grid_size() + Vec2::new(0.0, SLIDER_ROW)
}

/// Whether the eyedropper waits for a click on a thing.
#[derive(Default)]
pub struct HueGridUi {
    picking: bool,
}

impl HueGridUi {
    /// Draws the grid at `left_top` and the shade slider under it. The
    /// player changes the pick while `live`. `key` tells the grids of
    /// two panels apart. Gives the place they take.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        left_top: Pos2,
        key: &str,
        pick: &mut HuePick,
        tools: &Tools<'_>,
        live: bool,
    ) -> Rect {
        let grid = Rect::from_min_size(left_top, grid_size());
        for index in 0..GRID_ROWS * GRID_COLUMNS {
            let cell = Rect::from_min_size(
                grid.min
                    + Vec2::new(
                        (index % GRID_COLUMNS) as f32 * CELL,
                        (index / GRID_COLUMNS) as f32 * CELL,
                    ),
                Vec2::splat(CELL),
            );
            let hue = grid_hue(pick.graduation, index);
            ui.painter()
                .rect_filled(cell, CornerRadius::ZERO, tools.scene.words_color(hue));
            let clicked = ui
                .interact(cell, Id::new((key, "hue-cell", index)), Sense::click())
                .clicked();
            if clicked && live {
                pick.index = index;
            }
            if index == pick.index {
                ui.painter().rect_stroke(
                    cell,
                    CornerRadius::ZERO,
                    Stroke::new(MARK_WIDTH, theme::TEXT),
                    egui::StrokeKind::Inside,
                );
            }
        }
        let slider = Rect::from_min_size(
            Pos2::new(grid.left(), grid.bottom() + theme::ROW_GAP),
            Vec2::new(grid.width(), SLIDER_ROW - theme::ROW_GAP),
        );
        ui.add_enabled_ui(live, |ui| {
            ui.put(
                slider,
                egui::Slider::new(&mut pick.graduation, GRADUATION_MIN..=GRADUATION_MAX)
                    .text(WORDS_SHADE)
                    .text_color(theme::TEXT),
            );
        });
        Rect::from_min_size(left_top, picker_size())
    }

    /// The eyedropper button with its top left at `at`: the next click on
    /// a thing gives its hue. Gives its place.
    pub fn eyedropper(
        &mut self,
        ui: &egui::Ui,
        at: Pos2,
        key: &str,
        frame: &WatchFrame,
        tools: &Tools<'_>,
    ) -> Rect {
        let area = Rect::from_min_size(at, EYEDROPPER_SIZE);
        let color = if self.picking {
            theme::WAITING
        } else {
            theme::TEXT
        };
        if theme::segment_keyed(
            ui,
            area,
            Id::new((key, "eyedropper")),
            WORDS_EYEDROPPER,
            color,
        ) {
            if frame.target_cursor {
                tools.hand.act(Act::CancelTarget);
            }
            tools.hand.aim(LocalAim::PickThing);
            self.picking = true;
        }
        area
    }

    /// Takes the hue of the thing the eyedropper clicked into the pick. A
    /// hue the grid cannot show is told to the player.
    pub fn take_picked(&mut self, pick: &mut HuePick, frame: &WatchFrame, tools: &Tools<'_>) {
        if !self.picking {
            return;
        }
        if tools.hand.aiming() != Some(LocalAim::PickThing) {
            self.picking = false;
        }
        if let Some(serial) = tools.hand.take_picked(LocalAim::PickThing) {
            self.picking = false;
            if !pick.take(hue_of(frame, serial)) {
                tools.hand.report(BAD_HUE_WORDS);
            }
        }
    }

    /// Stops waiting for a click, as a new pick starts.
    pub fn stop(&mut self) {
        self.picking = false;
    }
}

/// A swatch of a hue, as the words of the hue show in the client.
pub fn swatch(ui: &egui::Ui, area: Rect, hue: u16, tools: &Tools<'_>) {
    ui.painter().rect_filled(
        area,
        CornerRadius::same(theme::BAR_RADIUS),
        tools.scene.words_color(hue),
    );
    ui.painter().rect_stroke(
        area,
        CornerRadius::same(theme::BAR_RADIUS),
        Stroke::new(MARK_WIDTH / 2.0, Color32::from_black_alpha(u8::MAX)),
        egui::StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_picker_holds_the_grid_and_the_slider() {
        assert_eq!(grid_size().x, GRID_COLUMNS as f32 * CELL);
        assert_eq!(picker_size().y, grid_size().y + SLIDER_ROW);
        let mut grid = HueGridUi { picking: true };
        grid.stop();
        assert!(!grid.picking);
    }
}
