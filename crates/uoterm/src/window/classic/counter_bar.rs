//! The counter bar of the classic client: a
//! dark bar of cells, each with the picture of an item the Counters page
//! names and how many of it the backpack holds. A cell flashes its hue when
//! the amount goes up or down, and its frame turns red when the amount is
//! low. A click, or a double click without "Single-click UI buttons", uses
//! one of the items. An item dropped on a cell counts from then on; Alt and
//! a right click empty a cell; a double click on the bar makes it read
//! only, which takes no drops. The bar shows while the Counters page has it
//! on. The counts come from `model::counters`, as the Modern bar has them.

use super::canvas::Canvas;
use super::manager::GumpManager;
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use crate::window::control::Act;
use crate::window::model::counters::{self, Changes};
use crate::window::settings::{CounterOptions, Profile, NO_HUE};
use eframe::egui::{Color32, Vec2};

pub const COUNTER_BAR: GumpKind = GumpKind {
    id: well_known::COUNTERS,
    rules: GumpRules {
        right_click_closes: false,
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(CounterBar::default()),
};

/// Shows the bar while the Counters page has it on, and hides it else.
pub fn sync(manager: &mut GumpManager, profile: &mut Profile) {
    let id = GumpId::one(COUNTER_BAR.id);
    match (profile.counters.enabled, manager.is_open(&id)) {
        (true, false) => {
            manager.open(id, profile);
        }
        (false, true) => manager.close(&id, profile),
        _ => {}
    }
}

const BORDER: i32 = 2;
const BACK_ALPHA: f32 = 0.7;
const LEAST_CELL: u8 = 30;
const MOST_CELL: u8 = 80;
const FLASH_SECONDS: f64 = 5.0;
const FLASH_UP_HUE: u16 = 1165;
const FLASH_DOWN_HUE: u16 = 1166;
const AMOUNT_FONT: u8 = 1;
const AMOUNT_HUE: u16 = 0x0035;
const AMOUNT_X: i32 = 2;
const AMOUNT_UP: i32 = 15;
const HELP_HUE: u16 = 0x0032;
const HELP_FONT: u8 = 1;
const HELP_AT: i32 = BORDER * 4;
/// An empty bar is wide enough for its help words.
const HELP_CELLS: i32 = 6;
const WORDS_HELP: &str = "Drop an item here to\nstart your first counter";
const HOVER_LINE: Color32 = Color32::YELLOW;
const LOW_LINE: Color32 = Color32::RED;
const PLAIN_LINE: Color32 = Color32::GRAY;
const HALF: i32 = 2;

/// The words of an amount on a cell: none for a single item, as the
/// classic bar leaves it out.
fn shown_amount(amount: u32, options: &CounterOptions) -> String {
    if amount == 1 {
        String::new()
    } else {
        counters::amount_words(amount, options)
    }
}

/// The size of the bar for its cells, and the side of one cell.
fn bar_size(options: &CounterOptions, empty: bool) -> (i32, i32, i32) {
    let side = i32::from(options.cell_size.clamp(LEAST_CELL, MOST_CELL));
    let columns = i32::from(options.columns.max(1));
    let rows = i32::from(options.rows.max(1));
    let width = if empty {
        (columns * side).max(HELP_CELLS * side)
    } else {
        columns * side
    };
    (width + BORDER * HALF, rows * side + BORDER * HALF, side)
}

#[derive(Default)]
pub struct CounterBar {
    changes: Changes,
}

impl CounterBar {
    /// Takes the item on the mouse into a cell, when the bar takes drops.
    fn take_drop(&self, g: &Canvas<'_>, cx: &mut GumpContext<'_>, cell: usize, hovered: bool) {
        if !hovered || cx.profile.counters.read_only || cx.desk.carried().is_none() {
            return;
        }
        let Some((item, _)) = cx.desk.land(g.ui()) else {
            return;
        };
        counters::put_in_cell(
            &mut cx.profile.counters.items,
            cell,
            counters::counted(&item),
        );
        cx.profile_changed();
    }
}

impl GumpBody for CounterBar {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let options = cx.profile.counters.clone();
        let empty = options.items.is_empty();
        let (width, height, side) = bar_size(&options, empty);
        g.shade(
            BORDER,
            BORDER,
            width - BORDER * HALF,
            height - BORDER * HALF,
            0,
            BACK_ALPHA,
        );
        if g.body_double_click() {
            cx.profile.counters.read_only = !options.read_only;
            cx.profile_changed();
        }
        if empty {
            let look = TextLook::unicode(HELP_FONT, HELP_HUE);
            g.label(HELP_AT, HELP_AT, WORDS_HELP, &look);
            self.take_drop(g, cx, 0, g.hovered(0, 0, width, height));
            return;
        }
        let bags = counters::backpack_containers(cx.frame);
        let per_row = usize::from(options.columns.max(1));
        let now = g.ctx().input(|i| i.time);
        let alt = g.ui().input(|i| i.modifiers.alt);
        let right = g.right_click();
        for cell in 0..counters::cells(&options) {
            let x = BORDER + (cell % per_row) as i32 * side;
            let y = BORDER + (cell / per_row) as i32 * side;
            let inner = side - BORDER * HALF;
            let (left, top) = (x + BORDER, y + BORDER);
            let hovered = g.hovered(left, top, inner, inner);
            let response = g.click_area(("cell", cell), left, top, inner, inner);
            let Some(item) = options.items.get(cell) else {
                let line = if hovered { HOVER_LINE } else { PLAIN_LINE };
                g.outline(left, top, inner, inner, line);
                self.take_drop(g, cx, cell, hovered);
                continue;
            };
            let amount = counters::count(&bags, item);
            if let Some(change) = self.changes.observe(cell, amount, now) {
                let fading = 1.0 - ((now - change.at) / FLASH_SECONDS) as f32;
                if fading > 0.0 && options.highlight_on_change {
                    let hue = if change.up {
                        FLASH_UP_HUE
                    } else {
                        FLASH_DOWN_HUE
                    };
                    g.shade(left, top, inner, inner, hue, fading);
                    g.ctx().request_repaint();
                }
            }
            let hue = if item.hue == NO_HUE { 0 } else { item.hue };
            let picture = g.item_size(item.graphic);
            let fit = (inner as f32 / picture.x.max(1.0))
                .min(inner as f32 / picture.y.max(1.0))
                .min(1.0);
            let shown = picture * fit;
            let inset = (Vec2::splat(inner as f32) - shown) / HALF as f32;
            g.scaled(fit, |g| {
                g.item(
                    ((left as f32 + inset.x) / fit) as i32,
                    ((top as f32 + inset.y) / fit) as i32,
                    item.graphic,
                    hue,
                );
            });
            let look = TextLook::unicode(AMOUNT_FONT, AMOUNT_HUE).bordered();
            g.label(
                left + AMOUNT_X,
                top + inner - AMOUNT_UP,
                &shown_amount(amount, &options),
                &look,
            );
            let line = if hovered {
                HOVER_LINE
            } else if counters::is_low(amount, &options) {
                LOW_LINE
            } else {
                PLAIN_LINE
            };
            g.outline(left, top, inner, inner, line);
            if hovered {
                g.tooltip(&item.label);
            }
            let used = if cx.profile.combat.single_click_buttons {
                response.clicked()
            } else {
                response.double_clicked()
            };
            if used {
                if let Some(pack) = counters::first_counted(&bags, item) {
                    cx.act(Act::Use(pack.serial));
                }
            }
            if hovered
                && alt
                && right
                && !options.read_only
                && counters::clear_cell(&mut cx.profile.counters.items, cell)
            {
                cx.profile_changed();
                return;
            }
            self.take_drop(g, cx, cell, hovered);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_fits_its_cells_and_an_empty_bar_its_help() {
        let options = CounterOptions {
            rows: 2,
            columns: 3,
            cell_size: 40,
            ..CounterOptions::default()
        };
        assert_eq!(bar_size(&options, false), (124, 84, 40));
        assert_eq!(bar_size(&options, true), (244, 84, 40));
        let tiny = CounterOptions {
            cell_size: 5,
            ..options.clone()
        };
        assert_eq!(bar_size(&tiny, false).2, i32::from(LEAST_CELL));
        assert_eq!(shown_amount(1, &options), "");
        assert_eq!(shown_amount(12, &options), "12");
    }

    #[test]
    fn the_bar_follows_its_option_and_counts_an_item_dropped_on_it() {
        use crate::view::{WatchFrame, WatchPackItem};
        use crate::window::classic::testing::draw_with_input;
        use crate::window::desk::Desk;
        use eframe::egui::{Event, Modifiers, PointerButton, Pos2};
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let id = GumpId::one(COUNTER_BAR.id);
        sync(&mut manager, &mut profile);
        assert!(!manager.is_open(&id));
        profile.counters.enabled = true;
        sync(&mut manager, &mut profile);
        assert!(manager.is_open(&id));
        let place = Pos2::new(200.0, 200.0);
        manager.open_at(id, place, &mut profile);
        let mut desk = Desk::default();
        desk.pick_up(&WatchPackItem {
            serial: 0x4000_0001,
            graphic: 0x0E21,
            hue: 0,
            name: "bandages".into(),
            ..WatchPackItem::default()
        });
        let on_bar = place + eframe::egui::Vec2::new(10.0, 10.0);
        let frames = vec![
            vec![Event::PointerMoved(on_bar)],
            vec![Event::PointerMoved(on_bar)],
            vec![Event::PointerButton {
                pos: on_bar,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::default(),
            }],
        ];
        if !draw_with_input(
            &mut manager,
            &mut profile,
            &WatchFrame::default(),
            &mut desk,
            &frames,
        ) {
            return;
        }
        assert_eq!(profile.counters.items.len(), 1);
        assert_eq!(profile.counters.items[0].graphic, 0x0E21);
        assert!(desk.carried().is_none());
    }
}
