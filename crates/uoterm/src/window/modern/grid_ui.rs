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
    ask_waiting_name, scrolled, single_or_double, Tools, CELL, CELL_GAP, CELL_RADIUS,
};
use super::super::control::{Act, DropTo};
use super::super::desk::Zone;
use super::super::hud::capitalized;
use super::super::model::clicks::ClickDelay;
use super::super::model::compare::{self, Difference};
use super::super::model::grid::{self, Look, Selection};
use super::super::model::loot::{amount_at, share_of, LootAmounts};
use super::super::model::{highlight, loot, properties};
use super::super::ring_ui::Subject;
use super::super::settings::GridLayout;
use super::super::settings::Profile;
use super::super::theme::{self, number_font, text_font};
use super::frame::{self, FrameEvent, PanelSpec, TITLE_ROW};
use super::layout::{self, Spot};
use crate::view::{WatchContainer, WatchFrame, WatchPackItem};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::{HashMap, HashSet};

const CORPSE_PLACE_ID: &str = "grid:corpse:";
const PERCENT: f32 = 100.0;
/// The smallest grid shows this many cells across and down.
const MIN_COLUMNS: f32 = 2.0;
const MIN_ROWS: f32 = 1.0;
const SEARCH_ROW: f32 = 22.0;
const TITLE_BUTTON_WIDTH: f32 = 44.0;
const TITLE_GAP: f32 = 6.0;
const STRIP_ROW: f32 = 26.0;
const STRIP_BUTTON_WIDTH: f32 = 96.0;
const MARK_WIDTH: f32 = 2.0;
const LOCK_DOT: f32 = 3.0;
const DIMMED_ALPHA: f32 = 0.3;
/// A bag's preview names this many of its items.
const PREVIEW_ITEMS: usize = 8;
/// Pictures in a cell grow no more than this when the Containers page
/// scales them, and not at all when it does not.
const SCALED_ART: f32 = 2.0;
const NATURAL_ART: f32 = 1.0;

/// The slider of a pile of a grid loot cell: its height at the foot of the
/// cell.
const PILE_SLIDER: f32 = 6.0;

const WORDS_FAVORITE: &str = "Fav";
const WORDS_LOOT_ALL: &str = "Loot";
const WORDS_LOOT_BAG: &str = "Bag";
const WORDS_MOVE_HERE: &str = "Move here";
const WORDS_TO_FAVORITE: &str = "To favorite";
const WORDS_CLEAR: &str = "Clear";
const WORDS_COMPARED: &str = "Against the worn one:";
const WORDS_SAME: &str = "No change against the worn one.";
const WORDS_HOLDS: &str = "Holds:";
/// The title of a container the shard and the client files do not name.
const WORDS_CONTAINER: &str = "Container";
const HINT_SEARCH: &str = "search";
const HINT_ITEM: &str = "Click: name.  Double-click: use.  Drag: move.  Ctrl+click: choose.  \
     Shift+click: lock the slot.";
const HINT_LOOT_ITEM: &str =
    "Click: grab into the grab bag.  Drag the bar: how many.  Ctrl+click: choose.";
const HINT_FAVORITE: &str = "Make this bag the favorite bag.";
const HINT_LOOT_ALL: &str = "Loot this corpse by the loot list.";
const HINT_LOOT_BAG: &str = "Set the bag grabbed items go into.";

/// What the player did in one grid that changes another.
#[derive(Default)]
pub struct GridUi {
    search: HashMap<u32, String>,
    first_row: HashMap<u32, usize>,
    chosen: Selection,
    clicks: ClickDelay,
    /// How many of each pile of a grid loot a click grabs.
    amounts: LootAmounts,
}

/// What one grid needs to know of the frame and the page.
struct GridView<'a> {
    frame: &'a WatchFrame,
    container: &'a WatchContainer,
    corpse: bool,
    grid_loot: bool,
    /// The item property lines are needed: a rule, a search or the
    /// highlight words.
    wants_lines: bool,
}

fn cell_side(profile: &Profile) -> f32 {
    CELL * f32::from(profile.containers.grid_scale) / PERCENT
}

/// The size of a grid of `columns` by `rows` cells.
fn grid_size(side: f32, columns: f32, rows: f32) -> Vec2 {
    Vec2::new(
        columns * (side + CELL_GAP) - CELL_GAP,
        rows * (side + CELL_GAP) - CELL_GAP + TITLE_ROW + SEARCH_ROW + CELL_GAP,
    ) + Vec2::splat(theme::PANEL_PAD * 2.0)
}

/// The size a grid first opens at: the columns and the rows of the
/// Containers page.
pub(super) fn first_size(profile: &Profile) -> Vec2 {
    let options = &profile.containers;
    grid_size(
        cell_side(profile),
        f32::from(options.grid_columns),
        f32::from(options.grid_rows),
    )
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
        closed: &mut HashSet<u32>,
    ) -> Vec<Rect> {
        self.chosen.keep_present(frame);
        self.search
            .retain(|serial, _| frame.containers.iter().any(|c| c.serial == *serial));
        self.amounts.keep_only(|serial| {
            frame
                .containers
                .iter()
                .any(|c| c.items.iter().any(|item| item.serial == serial))
        });
        let options = &profile.containers;
        let wants_lines = !options.grid_highlight_rules.is_empty()
            || !options.grid_highlight_properties.is_empty();
        let corpse_look = loot::corpse_look(profile.general.grid_loot);
        let mut covered = Vec::new();
        // Each container first opens beside the character, a step from the
        // one before.
        let mut opened = 0;
        let mut corpses = 0;
        let mut shut = Vec::new();
        for container in frame
            .containers
            .iter()
            .filter(|c| !closed.contains(&c.serial))
        {
            let corpse = loot::is_corpse(frame, container.serial);
            if corpse && !loot::shows_corpse(&profile.general, container.items.len()) {
                continue;
            }
            // A grid loot closes when the corpse is out of reach.
            if corpse && corpse_look.grid && !loot::grid_loot_alive(frame, container.serial) {
                shut.push(container.serial);
                continue;
            }
            let searching = self
                .search
                .get(&container.serial)
                .is_some_and(|words| !words.trim().is_empty());
            let view = GridView {
                frame,
                container,
                corpse,
                grid_loot: corpse && corpse_look.grid,
                wants_lines: wants_lines || searching,
            };
            let side = cell_side(profile);
            let default = layout::first_place(rect, Spot::Container(opened), first_size(profile));
            opened += 1;
            // The first corpse opens where the last first corpse was left,
            // the second where the last second one was, and so on, so the
            // profile does not keep a place for each corpse.
            let id = if corpse {
                corpses += 1;
                format!("{CORPSE_PLACE_ID}{corpses}")
            } else {
                grid::place_id(container.serial)
            };
            let tile_name = tools
                .scene
                .item_tile(container.graphic)
                .map(|tile| tile.name.clone());
            let title = grid_title(&container.name, tile_name.as_deref());
            let spec = PanelSpec {
                id: &id,
                title: &title,
                default,
                min_size: Some(grid_size(side, MIN_COLUMNS, MIN_ROWS)),
                closable: true,
            };
            let panel = frame::place(rect, &spec, profile);
            covered.push(panel);
            if self.grid(ui, panel, &spec, &view, tools, profile) == Some(FrameEvent::Closed) {
                shut.push(container.serial);
            }
        }
        closed.extend(shut);
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
        tools.desk.zone(panel, Zone::Into(serial));
        let options = &profile.containers;
        let fill = theme::with_alpha(theme::GLASS, f32::from(options.grid_opacity) / PERCENT);
        let edge = if options.grid_border_hue == 0 {
            theme::GLASS_EDGE
        } else {
            tools.scene.words_color(options.grid_border_hue)
        };
        // The grid draws its own title, cut short before its buttons.
        let mut body = frame::draw_tinted(ui.painter(), panel, "", fill, edge);
        self.title_row(ui, panel, spec, view, tools, profile);
        let search = Rect::from_min_size(body.min, Vec2::new(body.width(), SEARCH_ROW));
        body.set_top(search.bottom() + CELL_GAP);
        self.search_row(ui, search, view);
        if !self.chosen.is_empty() && frame.human_control {
            let strip = Rect::from_min_max(
                Pos2::new(body.left(), body.bottom() - STRIP_ROW),
                body.right_bottom(),
            );
            body.set_bottom(strip.top() - CELL_GAP);
            self.selection_strip(ui, strip, view, tools, profile);
        }
        self.cells(ui, panel, body, view, tools, profile);
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
        if frame.human_control && !view.corpse {
            let favorite = profile.containers.favorite_bag == Some(serial);
            let area = button(&mut right, TITLE_BUTTON_WIDTH);
            let color = if favorite {
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
                profile.containers.favorite_bag = (!favorite).then_some(serial);
                tools.keep_profile(profile);
            }
            if ui.rect_contains_pointer(area) {
                super::super::tips::label(ui, HINT_FAVORITE, "");
            }
        }
        if frame.human_control && view.grid_loot {
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
                super::super::tips::label(ui, HINT_LOOT_ALL, "");
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
                super::super::tips::label(ui, HINT_LOOT_BAG, "");
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
        let words = self.search.entry(serial).or_default();
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
        profile: &mut Profile,
    ) {
        let serial = view.container.serial;
        let count = self.chosen.len();
        let favorite = profile.containers.favorite_bag;
        let buttons: [(&str, Option<u32>); 3] = [
            (WORDS_MOVE_HERE, Some(serial)),
            (WORDS_TO_FAVORITE, favorite),
            (WORDS_CLEAR, None),
        ];
        let mut left = strip.left();
        for (index, (words, bag)) in buttons.into_iter().enumerate() {
            if words == WORDS_TO_FAVORITE && bag.is_none() {
                continue;
            }
            let area = Rect::from_min_size(
                Pos2::new(left, strip.top()),
                Vec2::new(STRIP_BUTTON_WIDTH, strip.height()),
            );
            left = area.right() + TITLE_GAP;
            let key = Id::new(("grid-chosen", serial, index));
            if theme::segment_keyed(ui, area, key, words, theme::TEXT) {
                match bag {
                    Some(bag) => {
                        for act in self.chosen.moves_into(bag, view.frame) {
                            tools.hand.act(act);
                        }
                        self.chosen.clear();
                    }
                    None => self.chosen.clear(),
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

    fn cells(
        &mut self,
        ui: &egui::Ui,
        panel: Rect,
        body: Rect,
        view: &GridView<'_>,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let serial = view.container.serial;
        let side = cell_side(profile);
        let (columns, rows) = grid::fits(
            body.width() + CELL_GAP,
            body.height() + CELL_GAP,
            side + CELL_GAP,
        );
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
        let first_row = {
            let kept = self.first_row.entry(serial).or_default();
            *kept = scrolled(ui, panel, *kept, all_rows.saturating_sub(rows));
            *kept
        };
        let search = self.search.get(&serial).cloned().unwrap_or_default();
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
            if frame.human_control && response.clicked() && shift && !view.corpse {
                self.set_lock(tools, profile, serial, slot, None);
            }
            return;
        };
        let lines = if view.wants_lines {
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
        let max_scale = if profile.containers.scale_items {
            SCALED_ART
        } else {
            NATURAL_ART
        };
        if let Some((texture, sprite)) = tools.scene.item_picture(frame.map, item.graphic, item.hue)
        {
            let area = theme::fit_up_to(cell, sprite.width, sprite.height, max_scale);
            painter.image(
                texture,
                area,
                sprite.uv,
                Color32::WHITE.gamma_multiply(alpha),
            );
        }
        let pile_slider = view.grid_loot && item.amount > 1;
        let most = i32::from(item.amount);
        let taken = if pile_slider {
            *self.amounts.slot(item.serial, most)
        } else {
            most
        };
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
        let rules = &profile.containers.grid_highlight_rules;
        let marked = highlight::first_passed(rules, &lines, view.corpse)
            .map(|rule| tools.scene.words_color(rule.hue))
            .or_else(|| {
                highlight::has_any_words(&profile.containers.grid_highlight_properties, &lines)
                    .then_some(theme::GOAL)
            })
            .or_else(|| (look == Look::Marked).then_some(theme::GOAL));
        let chosen = self.chosen.contains(item.serial);
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
            let extra = self.hover_lines(item, &lines, view.frame, tools, profile);
            let footer = match (frame.human_control, view.grid_loot) {
                (false, _) => "",
                (true, true) => HINT_LOOT_ITEM,
                (true, false) => HINT_ITEM,
            };
            tools.tips.point_at_with(
                ui,
                tools.hand,
                item.serial,
                &item.name,
                &extra,
                footer,
                tools.time,
            );
        }
        if !frame.human_control {
            return;
        }
        if pile_slider {
            self.pile_slider(ui, cell, item.serial, most, taken);
        }
        let modifiers = ui.input(|i| i.modifiers);
        if response.clicked() && modifiers.command {
            self.chosen.toggle(item.serial);
        } else if response.clicked() && modifiers.shift && !view.corpse {
            let lock = (!slot_locked).then_some(item.serial);
            self.set_lock(tools, profile, serial, slot, lock);
        } else if response.clicked() && view.grid_loot && !frame.target_cursor {
            if let Some(bag) = tools.hand.grab_bag() {
                tools.hand.act(Act::Move {
                    item: item.serial,
                    amount: self.amounts.taken(item.serial),
                    to: DropTo::Into(bag),
                });
            }
        } else if response.drag_started_by(egui::PointerButton::Primary) {
            tools.desk.pick_up(item);
        } else if single_or_double(
            &response,
            &mut self.clicks,
            frame,
            tools.hand,
            item.serial,
            tools.time,
        ) {
            tools.hand.act(Act::Use(item.serial));
        } else if response.secondary_clicked() {
            tools.ring.open_at(
                cell.center(),
                item.serial,
                &item.name,
                Subject::Packed,
                tools.hand,
            );
        }
        // An item dropped on a bag goes in, and on a pile of its kind joins.
        tools.desk.zone(cell, Zone::Into(item.serial));
    }

    /// The bar at the foot of a pile of a grid loot: a click or a drag on
    /// it sets how many of the pile a click grabs.
    fn pile_slider(&mut self, ui: &egui::Ui, cell: Rect, serial: u32, most: i32, taken: i32) {
        let bar = Rect::from_min_max(
            Pos2::new(cell.left(), cell.bottom() - PILE_SLIDER),
            cell.right_bottom(),
        );
        let slider = ui.interact(
            bar,
            Id::new(("grid-loot-amount", serial)),
            Sense::click_and_drag(),
        );
        let painter = ui.painter();
        painter.rect_filled(bar, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let filled = Rect::from_min_size(
            bar.min,
            Vec2::new(bar.width() * share_of(taken, most), bar.height()),
        );
        painter.rect_filled(filled, CornerRadius::same(CELL_RADIUS), theme::GOAL);
        let set = slider.dragged() || slider.clicked();
        if let Some(at) = slider.interact_pointer_pos().filter(|_| set) {
            let share = (at.x - bar.left()) / bar.width().max(1.0);
            *self.amounts.slot(serial, most) = amount_at(share, most);
        }
    }

    /// Locks an item into a slot, or frees the slot with None.
    fn set_lock(
        &mut self,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        container: u32,
        slot: usize,
        item: Option<u32>,
    ) {
        let layout: &mut GridLayout = profile
            .containers
            .grid_layouts
            .entry(grid::layout_key(container))
            .or_default();
        match item {
            Some(item) => grid::lock(layout, slot, item),
            None => grid::unlock(layout, slot),
        }
        tools.keep_profile(profile);
    }

    /// The lines under the name in the tooltip: the compare with the worn
    /// item, and what a bag holds.
    fn hover_lines(
        &self,
        item: &WatchPackItem,
        lines: &[String],
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Vec<String> {
        let mut extra = Vec::new();
        let compare_on = profile.containers.grid_compare_tooltip;
        let worn = tools
            .layers
            .layer(item.graphic)
            .and_then(|layer| compare::worn_on(frame, layer))
            .filter(|worn| worn.serial != item.serial);
        if let (true, Some(worn)) = (compare_on, worn) {
            let item_lines = if lines.is_empty() {
                properties::lines_of(tools.readings, item.serial)
            } else {
                lines.to_vec()
            };
            let worn_lines = properties::lines_of(tools.readings, worn.serial);
            let differences: Vec<Difference> = compare::differences(&item_lines, &worn_lines);
            if differences.is_empty() {
                extra.push(WORDS_SAME.to_string());
            } else {
                extra.push(WORDS_COMPARED.to_string());
                extra.extend(differences.iter().map(Difference::words_for_player));
            }
        }
        if profile.containers.grid_preview {
            if let Some(bag) = grid::open_bag(frame, item.serial) {
                extra.push(format!("{WORDS_HOLDS} {}", bag.total));
                extra.extend(bag.items.iter().take(PREVIEW_ITEMS).map(preview_words));
            }
        }
        extra
    }
}

/// The title of a grid: the name the shard gives the container, or with
/// none, the name the client files give its graphic, as "Backpack".
fn grid_title(name: &str, tile_name: Option<&str>) -> String {
    [Some(name), tile_name]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|name| !name.is_empty())
        .map_or_else(|| WORDS_CONTAINER.to_string(), capitalized)
}

/// One item of a bag's preview: its amount and its name.
fn preview_words(item: &WatchPackItem) -> String {
    if item.amount > 1 {
        format!("{} {}", item.amount, item.name)
    } else {
        item.name.clone()
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

    fn draw(grids: &mut GridUi, frame: &WatchFrame, closed: &mut HashSet<u32>) -> usize {
        let mut profile = Profile::default();
        profile.general.grid_loot = GridLoot::GridOnly;
        let mut shown = 0;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = grids.draw(ui, rect, frame, tools, profile, closed).len();
        });
        shown
    }

    #[test]
    fn a_grid_is_titled_by_the_shard_or_else_by_the_client_files() {
        assert_eq!(grid_title("Mara's chest", Some("chest")), "Mara's chest");
        assert_eq!(grid_title("", Some("backpack")), "Backpack");
        assert_eq!(grid_title("  ", Some(" pouch ")), "Pouch");
        assert_eq!(grid_title("", None), WORDS_CONTAINER);
        assert_eq!(grid_title("", Some("")), WORDS_CONTAINER);
    }

    #[test]
    fn a_grid_loot_offers_the_whole_pile_and_closes_out_of_reach() {
        let mut grids = GridUi::default();
        let mut closed = HashSet::new();
        assert_eq!(draw(&mut grids, &corpse_at(2), &mut closed), 1);
        assert_eq!(grids.amounts.taken(COINS), 30, "the slider starts at all");
        assert_eq!(draw(&mut grids, &corpse_at(9), &mut closed), 0);
        assert!(closed.contains(&CORPSE), "out of reach, the grid closed");
    }
}
