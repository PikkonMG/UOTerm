//! The counter bar of the Modern style, as both windows show it: a grid of
//! the items the Counters page names, each with how many the backpack
//! holds. A low amount turns to the alarm, and a changed one flashes. A
//! click, or a double click without "Single-click UI buttons", uses one of
//! the items. An item dropped on a cell counts from then on, and Alt with
//! a right click empties a cell, unless the switch in the title fixes the
//! cells, as a double click on the classic bar makes it read only. What
//! each cell counts is `model::counters`'.

use super::layout::{first_place, Spot};
use super::places::{with_title_room, TITLE_ROW};
use super::theme::{ALARM, GOAL, PANEL_PAD, TEXT, TEXT_FAINT};
use crate::geom::{Area, Rgba, Vector};
use crate::model::counters::{self, Change};
use crate::settings::CounterOptions;

pub const COUNTERS_ID: &str = "modern:counters";
pub const COUNTER_GAP: f32 = 4.0;
/// A cell whose amount changed flashes this long.
const FLASH_SECONDS: f64 = 1.5;
pub const WORDS_COUNTERS: &str = "Counters";
pub const WORDS_FIXED: &str = "Fixed";
pub const HINT_EMPTY: &str = "Drop an item here to count it.";
const HINT_USE: &str = "Click: use one. Alt+right-click: empty the cell.";
const HINT_USE_DOUBLE: &str = "Double-click: use one. Alt+right-click: empty the cell.";
pub const HINT_FIXED: &str = "Fixed cells take no dropped items and do not empty.";

/// Where the bar first stands in `window`, by the cells of the page.
pub fn counters_first_place(window: Area, options: &CounterOptions) -> Area {
    let side = f32::from(options.cell_size);
    let cells = Vector::new(
        f32::from(options.columns.max(1)),
        f32::from(options.rows.max(1)),
    );
    let size = with_title_room(
        Vector::new(cells.x, cells.y) * (side + COUNTER_GAP) - Vector::splat(COUNTER_GAP)
            + Vector::new(0.0, TITLE_ROW)
            + Vector::splat(PANEL_PAD * 2.0),
    );
    first_place(window, Spot::OverVitals, size)
}

/// The hint of a counted cell: how it uses an item, while the human has
/// control.
pub fn cell_hint(live: bool, single_click: bool) -> &'static str {
    match (live, single_click) {
        (false, _) => "",
        (true, true) => HINT_USE,
        (true, false) => HINT_USE_DOUBLE,
    }
}

/// True when a click on a cell uses one of its items: a single click with
/// "Single-click UI buttons", else a double click.
pub fn uses_item(single_click: bool, double: bool) -> bool {
    single_click != double
}

/// True while a cell whose amount last changed by `change` flashes.
pub fn flashing(options: &CounterOptions, change: Option<Change>, time: f64) -> bool {
    options.highlight_on_change && change.is_some_and(|change| time - change.at < FLASH_SECONDS)
}

/// The color of the amount of a cell: the alarm when it runs low.
pub fn amount_color(amount: u32, options: &CounterOptions) -> Rgba {
    if counters::is_low(amount, options) {
        ALARM
    } else {
        TEXT
    }
}

/// The color of the words of the switch that fixes the cells.
pub fn fixed_color(fixed: bool) -> Rgba {
    if fixed {
        GOAL
    } else {
        TEXT_FAINT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cell_uses_by_the_click_option_and_flashes_after_a_change() {
        assert!(uses_item(true, false) && !uses_item(true, true));
        assert!(uses_item(false, true) && !uses_item(false, false));
        assert_eq!(cell_hint(false, true), "");
        assert_eq!(cell_hint(true, false), HINT_USE_DOUBLE);
        let mut options = CounterOptions {
            highlight_on_change: true,
            ..CounterOptions::default()
        };
        let change = Some(Change { at: 1.0, up: true });
        assert!(flashing(&options, change, 2.0));
        assert!(!flashing(&options, change, 3.0));
        assert!(!flashing(&options, None, 1.0));
        options.highlight_on_change = false;
        assert!(!flashing(&options, change, 1.0));
    }
}
