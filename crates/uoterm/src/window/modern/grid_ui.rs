//! The grid containers of the Modern style: each open container is a
//! panel of cells the player moves and sizes. Items keep the slots the
//! player locked, a search marks or hides items, rules mark items by
//! their properties, several items move together, and a corpse loots with
//! one click when the corpse options ask for grid loot: a click grabs the
//! amount the slider of a pile takes into the grab bag, the title sets the
//! grab bag, and the grid closes when the corpse is out of reach.
//!
//! A click on an item targets it while the shard waits for a target, and
//! else asks its name once the double click time is over; a double click
//! uses it.

use super::super::actions::guard::AIM_GRAB_BAG;
use super::super::actions::LocalAim;
use super::super::boxes_ui::{
    ask_waiting_name, single_or_double, wheel_over, Tools, CELL_GAP, CELL_RADIUS,
};
use super::super::control::Act;
use super::super::desk::Zone;
use super::super::model::clicks::ClickDelay;
use super::super::model::grid::{self, Look};
use super::super::model::loot::share_of;
use super::super::model::properties;
use super::super::ring_ui::Subject;
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::super::tips;
use super::frame::{self, FrameEvent, PanelSpec};
use crate::view::{WatchContainer, WatchFrame, WatchPackItem};
use crate::window::bridge;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, Id, PointerButton, Pos2, Rect, Sense, Stroke, Vec2,
};
use uoterm_view::input;
use uoterm_view::ui::grid_clicks::{
    cell_side, grid_click, grid_title, hover_lines, least_size, CellPress, GridClick, SEARCH_ROW,
};
use uoterm_view::ui::grids::{
    art_most_scale, border_hue, cell_mark, cells_fit, cells_room, grid_first_place, grid_opacity,
    has_pile_slider, is_favorite, item_footer, shown_grids, strip_buttons, toggle_favorite,
    toggle_lock, CellMark, ClosedBoxes, GridMemory, ShownGrid, DIMMED_ALPHA, HINT_FAVORITE,
    HINT_LOOT_ALL, HINT_LOOT_BAG, HINT_SEARCH, STRIP_ROW, WORDS_FAVORITE, WORDS_LOOT_ALL,
    WORDS_LOOT_BAG,
};

const TITLE_BUTTON_WIDTH: f32 = 44.0;
const TITLE_GAP: f32 = 6.0;
const STRIP_BUTTON_WIDTH: f32 = 96.0;
const MARK_WIDTH: f32 = 2.0;
const LOCK_DOT: f32 = 3.0;

/// The slider of a pile of a grid loot cell: its height at the foot of the
/// cell.
const PILE_SLIDER: f32 = 6.0;

/// What the player did in one grid that changes another.
#[derive(Default)]
pub struct GridUi {
    memory: GridMemory,
    clicks: ClickDelay,
}

/// What one grid needs to know of the frame and the page.
struct GridView<'a> {
    frame: &'a WatchFrame,
    container: &'a WatchContainer,
    shown: &'a ShownGrid,
}

impl GridUi {
    /// Draws each open container that is not closed. Gives the places it
    /// covered.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        closed: &mut ClosedBoxes,
    ) -> Vec<Rect> {
        self.memory.keep_present(frame);
        let (grids, shut) = shown_grids(frame, profile, closed, &self.memory);
        let mut covered = Vec::new();
        for shown in &grids {
            let Some(container) = frame.containers.iter().find(|c| c.serial == shown.serial) else {
                continue;
            };
            let view = GridView {
                frame,
                container,
                shown,
            };
            let default = bridge::rect(grid_first_place(bridge::area(rect), shown.nth, profile));
            let tile_name = tools
                .scene
                .item_tile(container.graphic)
                .map(|tile| tile.name.clone());
            let title = grid_title(&container.name, tile_name.as_deref());
            let spec = PanelSpec {
                id: &shown.place_id,
                title: &title,
                default,
                min_size: Some(bridge::vec2(least_size(profile))),
                closable: true,
            };
            let panel = frame::place(rect, &spec, profile);
            covered.push(panel);
            if self.grid(ui, panel, &spec, &view, tools, profile) == Some(FrameEvent::Closed) {
                closed.close(shown.serial);
            }
        }
        for serial in shut {
            closed.close(serial);
        }
        ask_waiting_name(ui, &mut self.clicks, tools.hand, tools.time);
        covered
    }

    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        panel: Rect,
        spec: &PanelSpec<'_>,
        view: &GridView<'_>,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<FrameEvent> {
        let frame = view.frame;
        let serial = view.container.serial;
        tools.desk.zone(bridge::area(panel), Zone::Into(serial));
        let fill = theme::with_alpha(theme::GLASS, grid_opacity(profile));
        let edge =
            border_hue(profile).map_or(theme::GLASS_EDGE, |hue| tools.scene.words_color(hue));
        // The grid draws its own title, cut short before its buttons.
        let mut body = frame::draw_tinted(ui.painter(), panel, "", fill, edge);
        self.title_row(ui, panel, spec, view, tools, profile);
        let search = Rect::from_min_size(body.min, Vec2::new(body.width(), SEARCH_ROW));
        body.set_top(search.bottom() + CELL_GAP);
        self.search_row(ui, search, view);
        let strip = !self.memory.chosen.is_empty() && frame.human_control;
        if strip {
            let strip = Rect::from_min_max(
                Pos2::new(body.left(), body.bottom() - STRIP_ROW),
                body.right_bottom(),
            );
            body.set_bottom(strip.top() - CELL_GAP);
            self.selection_strip(ui, strip, view, tools, profile);
        }
        self.cells(ui, panel, body, strip, view, tools, profile);
        frame::controls(ui, panel, spec, profile, tools)
    }

    /// The count, the search field and the buttons of the title, left of
    /// the lock and close marks.
    fn title_row(
        &mut self,
        ui: &mut egui::Ui,
        panel: Rect,
        spec: &PanelSpec<'_>,
        view: &GridView<'_>,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let frame = view.frame;
        let serial = view.container.serial;
        let marks_left = frame::mark_area(panel, frame::marks(spec) - 1).left();
        let row_top = frame::mark_area(panel, 0).top();
        let mut right = marks_left - TITLE_GAP;
        let button = |right: &mut f32, width: f32| {
            let area = Rect::from_min_size(
                Pos2::new(*right - width, row_top),
                Vec2::new(width, frame::mark_area(panel, 0).height()),
            );
            *right = area.left() - TITLE_GAP;
            area
        };
        if frame.human_control && !view.shown.corpse {
            let area = button(&mut right, TITLE_BUTTON_WIDTH);
            let color = if is_favorite(profile, serial) {
                theme::WAITING
            } else {
                theme::TEXT_DIM
            };
            if theme::segment_keyed(
                ui,
                area,
                Id::new(("grid-favorite", serial)),
                WORDS_FAVORITE,
                color,
            ) {
                toggle_favorite(profile, serial);
                tools.keep_profile(profile);
            }
            if ui.rect_contains_pointer(area) {
                tips::label(ui, HINT_FAVORITE, "");
            }
        }
        if frame.human_control && view.shown.grid_loot {
            let area = button(&mut right, TITLE_BUTTON_WIDTH);
            if theme::segment_keyed(
                ui,
                area,
                Id::new(("grid-loot", serial)),
                WORDS_LOOT_ALL,
                theme::GOAL,
            ) {
                tools.hand.act(Act::Loot(serial));
            }
            if ui.rect_contains_pointer(area) {
                tips::label(ui, HINT_LOOT_ALL, "");
            }
            let area = button(&mut right, TITLE_BUTTON_WIDTH);
            if theme::segment_keyed(
                ui,
                area,
                Id::new(("grid-loot-bag", serial)),
                WORDS_LOOT_BAG,
                theme::TEXT,
            ) {
                tools.hand.report(AIM_GRAB_BAG);
                tools.hand.aim(LocalAim::SetGrabBag);
            }
            if ui.rect_contains_pointer(area) {
                tips::label(ui, HINT_LOOT_BAG, "");
            }
        }
        let whole = frame::title_room(panel, frame::marks(spec));
        let title = Rect::from_min_max(
            whole.min,
            Pos2::new(right.max(whole.left()), whole.bottom()),
        );
        frame::draw_title(ui.painter(), title, spec.title);
    }

    /// The search field, and how many items the container holds.
    fn search_row(&mut self, ui: &mut egui::Ui, row: Rect, view: &GridView<'_>) {
        let serial = view.container.serial;
        let count = ui.painter().text(
            row.right_center(),
            Align2::RIGHT_CENTER,
            view.container.total.to_string(),
            number_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let field = Rect::from_min_max(row.min, Pos2::new(count.left() - TITLE_GAP, row.bottom()));
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let words = self.memory.search.entry(serial).or_default();
        ui.put(
            field,
            egui::TextEdit::singleline(words)
                .id(Id::new(("grid-search", serial)))
                .frame(false)
                .hint_text(HINT_SEARCH)
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
    }

    /// The row that moves the chosen items together.
    fn selection_strip(
        &mut self,
        ui: &egui::Ui,
        strip: Rect,
        view: &GridView<'_>,
        tools: &mut Tools<'_>,
        profile: &Profile,
    ) {
        let serial = view.container.serial;
        let count = self.memory.chosen.len();
        let mut left = strip.left();
        let buttons = strip_buttons(serial, profile.containers.favorite_bag);
        for (index, (words, press)) in buttons.into_iter().enumerate() {
            let area = Rect::from_min_size(
                Pos2::new(left, strip.top()),
                Vec2::new(STRIP_BUTTON_WIDTH, strip.height()),
            );
            left = area.right() + TITLE_GAP;
            let key = Id::new(("grid-chosen", serial, index));
            if theme::segment_keyed(ui, area, key, words, theme::TEXT) {
                for act in self.memory.strip_press(press, view.frame) {
                    tools.hand.act(act);
                }
            }
        }
        ui.painter().text(
            Pos2::new(strip.right(), strip.center().y),
            Align2::RIGHT_CENTER,
            count.to_string(),
            number_font(theme::SIZE_BODY),
            theme::WAITING,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn cells(
        &mut self,
        ui: &egui::Ui,
        panel: Rect,
        body: Rect,
        strip: bool,
        view: &GridView<'_>,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let serial = view.container.serial;
        let side = cell_side(profile);
        let (columns, rows) = cells_fit(cells_room(bridge::vector(panel.size()), strip), side);
        let key = grid::layout_key(serial);
        let layout = profile
            .containers
            .grid_layouts
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let items: Vec<u32> = view
            .container
            .items
            .iter()
            .map(|item| item.serial)
            .collect();
        let slots = grid::arrange(&items, &layout, columns * rows);
        let all_rows = slots.len().div_ceil(columns);
        let turned = wheel_over(ui, panel);
        let first_row = self
            .memory
            .scroll(serial, turned, all_rows.saturating_sub(rows));
        let search = self.memory.search_words(serial).to_string();
        for (at, slot) in slots
            .iter()
            .enumerate()
            .skip(first_row * columns)
            .take(columns * rows)
        {
            let shown = at - first_row * columns;
            let cell = Rect::from_min_size(
                body.left_top()
                    + Vec2::new(
                        (shown % columns) as f32 * (side + CELL_GAP),
                        (shown / columns) as f32 * (side + CELL_GAP),
                    ),
                Vec2::splat(side),
            );
            let item = slot.and_then(|item| view.container.items.iter().find(|i| i.serial == item));
            let slot_locked = grid::is_locked(&layout, at);
            self.cell(
                ui,
                cell,
                at,
                slot_locked,
                item,
                &search,
                view,
                tools,
                profile,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn cell(
        &mut self,
        ui: &egui::Ui,
        cell: Rect,
        slot: usize,
        slot_locked: bool,
        item: Option<&WatchPackItem>,
        search: &str,
        view: &GridView<'_>,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let frame = view.frame;
        let painter = ui.painter();
        let serial = view.container.serial;
        let corpse = view.shown.corpse;
        let grid_loot = view.shown.grid_loot;
        let response = ui.interact(
            cell,
            Id::new(("grid-cell", serial, slot)),
            Sense::click_and_drag(),
        );
        let fill = if response.hovered() {
            theme::BUTTON_HOVER
        } else {
            theme::TRACK
        };
        painter.rect_filled(cell, CornerRadius::same(CELL_RADIUS), fill);
        if slot_locked {
            painter.circle_filled(
                cell.left_top() + Vec2::splat(theme::CELL_ART_PAD + LOCK_DOT),
                LOCK_DOT,
                theme::WAITING,
            );
        }
        let Some(item) = item else {
            let shift = ui.input(|i| i.modifiers.shift);
            if frame.human_control && response.clicked() && shift && !corpse {
                toggle_lock(profile, serial, slot, None);
                tools.keep_profile(profile);
            }
            return;
        };
        let lines = if view.shown.wants_lines {
            properties::lines_of(tools.readings, item.serial)
        } else {
            Vec::new()
        };
        let look = grid::search_look(profile.containers.grid_search, search, &item.name, &lines);
        if look == Look::Hidden {
            return;
        }
        let alpha = if look == Look::Dimmed {
            DIMMED_ALPHA
        } else {
            1.0
        };
        if let Some((texture, sprite)) = tools.scene.item_picture(item.graphic, item.hue) {
            let area = theme::fit_up_to(cell, sprite.width, sprite.height, art_most_scale(profile));
            painter.image(
                texture,
                area,
                bridge::rect(sprite.uv),
                Color32::WHITE.gamma_multiply(alpha),
            );
        }
        let pile_slider = has_pile_slider(grid_loot, item);
        let most = i32::from(item.amount);
        let taken = self.memory.shown_amount(item, pile_slider);
        if item.amount > 1 {
            let color = if taken < most {
                theme::GOAL
            } else {
                theme::TEXT
            };
            theme::shadowed_text(
                painter,
                cell.right_bottom() - Vec2::splat(theme::CELL_ART_PAD),
                Align2::RIGHT_BOTTOM,
                &taken.to_string(),
                number_font(theme::SIZE_SMALL),
                theme::with_alpha(color, alpha),
            );
        }
        let marked = cell_mark(profile, &lines, corpse, look).map(|mark| match mark {
            CellMark::Hue(hue) => tools.scene.words_color(hue),
            CellMark::Goal => theme::GOAL,
        });
        let chosen = self.memory.chosen.contains(item.serial);
        for (on, color) in [
            (marked.is_some(), marked.unwrap_or(theme::GOAL)),
            (chosen, theme::WAITING),
        ] {
            if on {
                painter.rect_stroke(
                    cell,
                    CornerRadius::same(CELL_RADIUS),
                    Stroke::new(MARK_WIDTH, color),
                    egui::StrokeKind::Inside,
                );
            }
        }
        if response.hovered() && !tools.desk.carries() && !tools.ring.is_open() {
            let extra = hover_lines(
                item,
                &lines,
                view.frame,
                (tools.layers, tools.readings),
                profile,
            );
            tips::point_at_with(
                tools.tips,
                ui,
                tools.hand,
                item.serial,
                &item.name,
                &extra,
                item_footer(frame.human_control, grid_loot),
                tools.time,
            );
        }
        if !frame.human_control {
            return;
        }
        if pile_slider {
            self.pile_slider(ui, cell, item, taken);
        }
        let press = CellPress {
            click: if response.clicked() {
                Some(input::PointerButton::Primary)
            } else if response.secondary_clicked() {
                Some(input::PointerButton::Secondary)
            } else {
                None
            },
            double: response.double_clicked(),
            drag_started: response.drag_started_by(PointerButton::Primary),
            mods: bridge::mods(ui.input(|i| i.modifiers)),
        };
        match grid_click(press, corpse, grid_loot, frame.target_cursor) {
            GridClick::Choose => self.memory.chosen.toggle(item.serial),
            GridClick::Lock => {
                toggle_lock(profile, serial, slot, Some(item.serial));
                tools.keep_profile(profile);
            }
            GridClick::Grab => {
                if let Some(bag) = tools.hand.grab_bag() {
                    tools.hand.act(self.memory.grab_act(item.serial, bag));
                }
            }
            GridClick::PickUp => tools.desk.pick_up(item),
            GridClick::Use | GridClick::Name => {
                if single_or_double(
                    &response,
                    &mut self.clicks,
                    frame,
                    tools.hand,
                    item.serial,
                    tools.time,
                ) {
                    tools.hand.act(Act::Use(item.serial));
                }
            }
            GridClick::Ring => tools.ring.open_at(
                cell.center(),
                item.serial,
                &item.name,
                Subject::Packed,
                tools.hand,
            ),
            GridClick::Nothing => {}
        }
        // An item dropped on a bag goes in, and on a pile of its kind joins.
        tools.desk.zone(bridge::area(cell), Zone::Into(item.serial));
    }

    /// The bar at the foot of a pile of a grid loot: a click or a drag on
    /// it sets how many of the pile a click grabs.
    fn pile_slider(&mut self, ui: &egui::Ui, cell: Rect, item: &WatchPackItem, taken: i32) {
        let bar = Rect::from_min_max(
            Pos2::new(cell.left(), cell.bottom() - PILE_SLIDER),
            cell.right_bottom(),
        );
        let slider = ui.interact(
            bar,
            Id::new(("grid-loot-amount", item.serial)),
            Sense::click_and_drag(),
        );
        let painter = ui.painter();
        painter.rect_filled(bar, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let filled = Rect::from_min_size(
            bar.min,
            Vec2::new(
                bar.width() * share_of(taken, i32::from(item.amount)),
                bar.height(),
            ),
        );
        painter.rect_filled(filled, CornerRadius::same(CELL_RADIUS), theme::GOAL);
        let set = slider.dragged() || slider.clicked();
        if let Some(at) = slider.interact_pointer_pos().filter(|_| set) {
            let share = (at.x - bar.left()) / bar.width().max(1.0);
            self.memory.set_amount(item, share);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::draw_frames;
    use super::*;
    use crate::view::WatchItem;
    use crate::window::model::loot::CORPSE_GRAPHIC;
    use crate::window::settings::GridLoot;

    const CORPSE: u32 = 0x4000_0200;
    const COINS: u32 = 0x4000_0201;

    fn corpse_at(x: u16) -> WatchFrame {
        WatchFrame {
            human_control: true,
            items: vec![WatchItem {
                serial: CORPSE,
                graphic: CORPSE_GRAPHIC,
                x,
                ..WatchItem::default()
            }],
            containers: vec![WatchContainer {
                serial: CORPSE,
                items: vec![WatchPackItem {
                    serial: COINS,
                    graphic: 0x0EED,
                    amount: 30,
                    ..WatchPackItem::default()
                }],
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        }
    }

    fn draw(grids: &mut GridUi, frame: &WatchFrame, closed: &mut ClosedBoxes) -> usize {
        let mut profile = Profile::default();
        profile.general.grid_loot = GridLoot::GridOnly;
        let mut shown = 0;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = grids.draw(ui, rect, frame, tools, profile, closed).len();
        });
        shown
    }

    #[test]
    fn a_grid_loot_offers_the_whole_pile_and_closes_out_of_reach() {
        let mut grids = GridUi::default();
        let mut closed = ClosedBoxes::default();
        assert_eq!(draw(&mut grids, &corpse_at(2), &mut closed), 1);
        assert_eq!(
            grids.memory.amounts.taken(COINS),
            30,
            "the slider starts at all"
        );
        assert_eq!(draw(&mut grids, &corpse_at(9), &mut closed), 0);
        assert!(closed.is_closed(CORPSE), "out of reach, the grid closed");
    }
}
