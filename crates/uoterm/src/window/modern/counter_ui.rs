//! The counter bar: a grid of the items the Counters page names, each with
//! how many the backpack holds. A low amount turns to the alarm, and a
//! changed one flashes. A click, or a double click without "Single-click
//! UI buttons", uses one of the items. An item dropped on a cell counts
//! from then on, and Alt with a right click empties a cell, unless the
//! switch in the title fixes the cells, as a double click on the classic
//! bar makes it read only.

use super::super::boxes_ui::{Tools, CELL_RADIUS};
use super::super::control::Act;
use super::super::model::counters;
use super::super::settings::Profile;
use super::super::settings::NO_HUE;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, PanelSpec};
use crate::view::WatchFrame;
use crate::window::bridge;
use crate::window::desk;
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, Vec2};
use uoterm_view::ui::counter_bar::{
    amount_color, cell_hint, counters_first_place, fixed_color, flashing, uses_item, COUNTERS_ID,
    COUNTER_GAP as CELL_GAP, HINT_EMPTY, HINT_FIXED, WORDS_COUNTERS as WORDS_TITLE, WORDS_FIXED,
};

const FLASH_WIDTH: f32 = 2.0;
/// The switch that fixes the cells takes the room of this many marks.
const FIXED_MARKS: usize = 3;

#[derive(Default)]
pub struct CounterUi {
    /// When the amount of each cell last changed.
    changes: counters::Changes,
}

impl CounterUi {
    /// Draws the bar. Gives its place.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let options = profile.counters.clone();
        let side = f32::from(options.cell_size);
        let spec = PanelSpec {
            id: COUNTERS_ID,
            title: WORDS_TITLE,
            default: bridge::rect(counters_first_place(bridge::area(rect), &options)),
            min_size: None,
            closable: false,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_TITLE);
        let bags = counters::backpack_containers(frame);
        let per_row = usize::from(options.columns.max(1));
        let single_click = profile.combat.single_click_buttons;
        let mut changed = false;
        for index in 0..counters::cells(&options) {
            let (column, row) = (index % per_row, index / per_row);
            let cell = Rect::from_min_size(
                body.left_top() + Vec2::new(column as f32, row as f32) * (side + CELL_GAP),
                Vec2::splat(side),
            );
            let response = ui.interact(cell, Id::new(("counter", index)), Sense::click());
            ui.painter()
                .rect_filled(cell, CornerRadius::same(CELL_RADIUS), theme::TRACK);
            if response.hovered() && !options.read_only {
                changed |= take_drop(ui, cell, index, tools, profile);
            }
            let Some(item) = options.items.get(index) else {
                if response.hovered() {
                    super::super::tips::label(ui, HINT_EMPTY, "");
                }
                continue;
            };
            let amount = counters::count(&bags, item);
            let change = self.changes.observe(index, amount, tools.time);
            let hue = if item.hue == NO_HUE { 0 } else { item.hue };
            if let Some((texture, sprite)) = tools.scene.item_picture(item.graphic, hue) {
                ui.painter().image(
                    texture,
                    theme::fit(cell, sprite.width, sprite.height),
                    bridge::rect(sprite.uv),
                    Color32::WHITE,
                );
            }
            theme::shadowed_text(
                ui.painter(),
                cell.right_bottom() - Vec2::splat(theme::CELL_ART_PAD),
                Align2::RIGHT_BOTTOM,
                &counters::amount_words(amount, &options),
                number_font(theme::SIZE_SMALL),
                bridge::color(amount_color(amount, &options)),
            );
            if flashing(&options, change, tools.time) {
                ui.painter().rect_stroke(
                    cell,
                    CornerRadius::same(CELL_RADIUS),
                    Stroke::new(FLASH_WIDTH, theme::WAITING),
                    egui::StrokeKind::Inside,
                );
                ui.ctx().request_repaint();
            }
            if response.hovered() {
                let hint = cell_hint(frame.human_control, single_click);
                super::super::tips::label(ui, &item.label, hint);
            }
            let used = (response.clicked() && uses_item(single_click, false))
                || (response.double_clicked() && uses_item(single_click, true));
            if frame.human_control && used {
                if let Some(pack) = counters::first_counted(&bags, item) {
                    tools.hand.act(Act::Use(pack.serial));
                }
            }
            let alt = ui.input(|i| i.modifiers.alt);
            if response.secondary_clicked() && alt && !options.read_only {
                changed |= counters::clear_cell(&mut profile.counters.items, index);
            }
        }
        changed |= fixed_switch(ui, panel, &spec, profile);
        if changed {
            tools.keep_profile(profile);
        }
        frame::controls_leaving(ui, panel, &spec, FIXED_MARKS, profile, tools);
        panel
    }
}

/// Counts the item the player drops on a cell from then on. True when one
/// was dropped.
fn take_drop(
    ui: &egui::Ui,
    cell: Rect,
    index: usize,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> bool {
    if tools.desk.carried().is_none() {
        return false;
    }
    ui.painter().rect_stroke(
        cell,
        CornerRadius::same(CELL_RADIUS),
        Stroke::new(FLASH_WIDTH, theme::GOAL),
        egui::StrokeKind::Inside,
    );
    let Some((item, _)) = desk::land(tools.desk, ui) else {
        return false;
    };
    counters::put_in_cell(&mut profile.counters.items, index, counters::counted(&item));
    true
}

/// The switch in the title that fixes the cells. True when it was flipped.
fn fixed_switch(ui: &egui::Ui, panel: Rect, spec: &PanelSpec<'_>, profile: &mut Profile) -> bool {
    let first = frame::marks(spec);
    let right = frame::mark_area(panel, first);
    let left = frame::mark_area(panel, first + FIXED_MARKS - 1);
    let area = Rect::from_min_max(left.min, right.max);
    let fixed = profile.counters.read_only;
    let color = bridge::color(fixed_color(fixed));
    let response = ui.interact(area, Id::new("counters-fixed"), Sense::click());
    ui.painter().text(
        area.center(),
        Align2::CENTER_CENTER,
        WORDS_FIXED,
        text_font(theme::SIZE_SMALL),
        color,
    );
    if response.hovered() {
        super::super::tips::label(ui, HINT_FIXED, "");
    }
    if response.clicked() {
        profile.counters.read_only = !fixed;
    }
    response.clicked()
}

#[cfg(test)]
mod tests {
    use super::super::testing::draw_frames;
    use super::*;
    use crate::view::WatchPackItem;
    use eframe::egui::{Event, Modifiers, PointerButton};

    #[test]
    fn an_item_dropped_on_a_cell_is_counted_unless_the_cells_are_fixed() {
        let frame = WatchFrame::default();
        let mut bar = CounterUi::default();
        let bandages = WatchPackItem {
            serial: 0x4000_0001,
            graphic: 0x0E21,
            name: "bandages".into(),
            ..WatchPackItem::default()
        };
        for fixed in [true, false] {
            let mut profile = Profile::default();
            profile.counters.read_only = fixed;
            let mut place = None;
            draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
                place = Some(bar.draw(ui, rect, &frame, tools, profile));
            });
            let on_cell = place.unwrap().min
                + Vec2::splat(theme::PANEL_PAD + CELL_GAP)
                + Vec2::new(0.0, frame::TITLE_ROW);
            let frames = vec![
                vec![Event::PointerMoved(on_cell)],
                vec![Event::PointerButton {
                    pos: on_cell,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::default(),
                }],
            ];
            let mut picked = false;
            draw_frames(&mut profile, &frames, |ui, rect, tools, profile| {
                if !picked {
                    tools.desk.pick_up(&bandages);
                    tools.desk.begin();
                    picked = true;
                }
                bar.draw(ui, rect, &frame, tools, profile);
            });
            assert_eq!(
                profile.counters.items.len(),
                usize::from(!fixed),
                "fixed: {fixed}"
            );
        }
    }
}
