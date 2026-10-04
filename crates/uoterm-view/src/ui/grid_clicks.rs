//! The rules of the grid containers of the Modern style: what a click on
//! a cell does by its button and its keys, the sizes of a grid, the place
//! id of a corpse, the title of a grid, and the lines of an item's tip.

use super::gumps::{CELL, CELL_GAP};
use super::hud::capitalized;
use super::places::TITLE_ROW;
use super::theme::PANEL_PAD;
use crate::frame::{WatchFrame, WatchPackItem};
use crate::geom::Vector;
use crate::input::{Mods, PointerButton};
use crate::model::compare::{self, Difference, ItemLayers};
use crate::model::grid;
use crate::model::properties;
use crate::model::reads::ReadCache;
use crate::settings::Profile;

const CORPSE_PLACE_ID: &str = "grid:corpse:";
const PERCENT: f32 = 100.0;
/// The smallest grid shows this many cells across and down.
const MIN_COLUMNS: f32 = 2.0;
const MIN_ROWS: f32 = 1.0;
/// The height of the search row under the title.
pub const SEARCH_ROW: f32 = 22.0;
/// A bag's preview names this many of its items.
const PREVIEW_ITEMS: usize = 8;
const WORDS_COMPARED: &str = "Against the worn one:";
const WORDS_SAME: &str = "No change against the worn one.";
const WORDS_HOLDS: &str = "Holds:";
/// The title of a container the shard and the client files do not name.
const WORDS_CONTAINER: &str = "Container";

/// What the pointer did on a cell of a grid in one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CellPress {
    /// The button of a click, when one ended on the cell.
    pub click: Option<PointerButton>,
    /// The click was the second of a double click.
    pub double: bool,
    /// The primary button started a drag on the cell.
    pub drag_started: bool,
    pub mods: Mods,
}

/// What a press on a cell with an item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridClick {
    /// Ctrl or Command and a click: choose the item, or leave it.
    Choose,
    /// Shift and a click: lock the item in its slot, or free the slot.
    Lock,
    /// A click on a grid loot: grab the amount of the pile into the grab bag.
    Grab,
    /// A drag: pick the item up.
    PickUp,
    /// A double click: use the item.
    Use,
    /// A single click: target it, or ask its name once the double click
    /// time is over.
    Name,
    /// A right click: open the ring.
    Ring,
    Nothing,
}

/// What a press on a cell with an item does, in a corpse, in a grid loot,
/// and while the shard waits for a target.
pub fn grid_click(
    press: CellPress,
    corpse: bool,
    grid_loot: bool,
    target_cursor: bool,
) -> GridClick {
    let primary = press.click == Some(PointerButton::Primary);
    if primary && press.mods.command {
        GridClick::Choose
    } else if primary && press.mods.shift && !corpse {
        GridClick::Lock
    } else if primary && grid_loot && !target_cursor {
        GridClick::Grab
    } else if press.drag_started {
        GridClick::PickUp
    } else if press.double {
        GridClick::Use
    } else if primary {
        GridClick::Name
    } else if press.click == Some(PointerButton::Secondary) {
        GridClick::Ring
    } else {
        GridClick::Nothing
    }
}

pub fn cell_side(profile: &Profile) -> f32 {
    CELL * f32::from(profile.containers.grid_scale) / PERCENT
}

/// The size of a grid of `columns` by `rows` cells.
pub fn grid_size(side: f32, columns: f32, rows: f32) -> Vector {
    Vector::new(
        columns * (side + CELL_GAP) - CELL_GAP + PANEL_PAD * 2.0,
        rows * (side + CELL_GAP) - CELL_GAP + TITLE_ROW + SEARCH_ROW + CELL_GAP + PANEL_PAD * 2.0,
    )
}

/// The size a grid first opens at: the columns and the rows of the
/// Containers page.
pub fn first_size(profile: &Profile) -> Vector {
    let options = &profile.containers;
    grid_size(
        cell_side(profile),
        f32::from(options.grid_columns),
        f32::from(options.grid_rows),
    )
}

/// The least size the player may make a grid.
pub fn least_size(profile: &Profile) -> Vector {
    grid_size(cell_side(profile), MIN_COLUMNS, MIN_ROWS)
}

/// The place id of the `nth` corpse open now, from 1. The first corpse
/// opens where the last first corpse was left, the second where the last
/// second one was, and so on, so the profile does not keep a place for
/// each corpse.
pub fn corpse_place_id(nth: usize) -> String {
    format!("{CORPSE_PLACE_ID}{nth}")
}

/// The title of a grid: the name the shard gives the container, or with
/// none, the name the client files give its graphic, as "Backpack".
pub fn grid_title(name: &str, tile_name: Option<&str>) -> String {
    [Some(name), tile_name]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|name| !name.is_empty())
        .map_or_else(|| WORDS_CONTAINER.to_string(), capitalized)
}

/// One item of a bag's preview: its amount and its name.
pub fn preview_words(item: &WatchPackItem) -> String {
    if item.amount > 1 {
        format!("{} {}", item.amount, item.name)
    } else {
        item.name.clone()
    }
}

/// The lines under the name in the tip of an item: the compare with the
/// worn item, and what a bag holds. `lines` are the property lines of the
/// item already read, or none.
pub fn hover_lines(
    item: &WatchPackItem,
    lines: &[String],
    frame: &WatchFrame,
    (layers, readings): (&ItemLayers, &mut ReadCache),
    profile: &Profile,
) -> Vec<String> {
    let mut extra = Vec::new();
    let compare_on = profile.containers.grid_compare_tooltip;
    let worn = layers
        .layer(item.graphic)
        .and_then(|layer| compare::worn_on(frame, layer))
        .filter(|worn| worn.serial != item.serial);
    if let (true, Some(worn)) = (compare_on, worn) {
        let item_lines = if lines.is_empty() {
            properties::lines_of(readings, item.serial)
        } else {
            lines.to_vec()
        };
        let worn_lines = properties::lines_of(readings, worn.serial);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::WatchContainer;

    const BAG: u32 = 0x4000_0300;

    fn press(click: Option<PointerButton>, mods: Mods) -> CellPress {
        CellPress {
            click,
            mods,
            ..CellPress::default()
        }
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
    fn the_keys_of_a_click_choose_lock_or_grab() {
        let primary = Some(PointerButton::Primary);
        let command = Mods {
            command: true,
            ..Mods::default()
        };
        let shift = Mods {
            shift: true,
            ..Mods::default()
        };
        let plain = Mods::default();
        assert_eq!(
            grid_click(press(primary, command), true, true, false),
            GridClick::Choose
        );
        assert_eq!(
            grid_click(press(primary, shift), false, false, false),
            GridClick::Lock
        );
        assert_eq!(
            grid_click(press(primary, shift), true, true, false),
            GridClick::Grab,
            "a corpse slot does not lock"
        );
        assert_eq!(
            grid_click(press(primary, plain), true, true, true),
            GridClick::Name,
            "a target cursor takes the click"
        );
        let double = CellPress {
            double: true,
            ..press(primary, plain)
        };
        assert_eq!(grid_click(double, false, false, false), GridClick::Use);
        let drag = CellPress {
            drag_started: true,
            ..CellPress::default()
        };
        assert_eq!(grid_click(drag, false, false, false), GridClick::PickUp);
        assert_eq!(
            grid_click(
                press(Some(PointerButton::Secondary), plain),
                false,
                false,
                false
            ),
            GridClick::Ring
        );
        assert_eq!(
            grid_click(CellPress::default(), false, false, false),
            GridClick::Nothing
        );
    }

    #[test]
    fn a_grid_opens_at_the_page_size_and_holds_at_least_two_cells() {
        let profile = Profile::default();
        assert!(least_size(&profile).x < first_size(&profile).x);
        assert_eq!(corpse_place_id(2), "grid:corpse:2");
    }

    #[test]
    fn a_bag_preview_names_its_items_with_their_amounts() {
        let coins = WatchPackItem {
            name: "gold coins".into(),
            amount: 30,
            ..WatchPackItem::default()
        };
        assert_eq!(preview_words(&coins), "30 gold coins");
        let frame = WatchFrame {
            containers: vec![WatchContainer {
                serial: BAG,
                total: 1,
                items: vec![coins],
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        };
        let mut profile = Profile::default();
        profile.containers.grid_preview = true;
        profile.containers.grid_compare_tooltip = false;
        let bag = WatchPackItem {
            serial: BAG,
            ..WatchPackItem::default()
        };
        let lines = hover_lines(
            &bag,
            &[],
            &frame,
            (&ItemLayers::default(), &mut ReadCache::default()),
            &profile,
        );
        assert_eq!(
            lines,
            vec!["Holds: 1".to_string(), "30 gold coins".to_string()]
        );
    }
}
