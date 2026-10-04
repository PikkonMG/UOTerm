//! The grid containers of the Modern style as both windows show them:
//! which open containers show as grids and where they first stand, the
//! containers the player closed, what he keeps of each grid between frames
//! (its search, its first row, the chosen items and the amounts of a grid
//! loot), how many cells fit, how a cell is marked, and the words of the
//! buttons and the tips. What a click on a cell does is `grid_clicks`'.

use super::grid_clicks::{corpse_place_id, first_size, SEARCH_ROW};
use super::gumps::{next_first_row, CELL_GAP};
use super::layout::{first_place, Spot};
use super::places::TITLE_ROW;
use super::theme::PANEL_PAD;
use crate::act::{Act, DropTo};
use crate::frame::{WatchFrame, WatchPackItem};
use crate::geom::{Area, Vector};
use crate::model::grid::{self, Look, Selection};
use crate::model::highlight;
use crate::model::loot::{self, LootAmounts};
use crate::settings::Profile;
use std::collections::{HashMap, HashSet};

pub const WORDS_FAVORITE: &str = "Fav";
pub const WORDS_LOOT_ALL: &str = "Loot";
pub const WORDS_LOOT_BAG: &str = "Bag";
pub const WORDS_MOVE_HERE: &str = "Move here";
pub const WORDS_TO_FAVORITE: &str = "To favorite";
pub const WORDS_CLEAR: &str = "Clear";
pub const HINT_SEARCH: &str = "search";
pub const HINT_ITEM: &str = "Click: name.  Double-click: use.  Drag: move.  Ctrl+click: choose.  \
     Shift+click: lock the slot.";
pub const HINT_LOOT_ITEM: &str =
    "Click: grab into the grab bag.  Drag the bar: how many.  Ctrl+click: choose.";
pub const HINT_FAVORITE: &str = "Make this bag the favorite bag.";
pub const HINT_LOOT_ALL: &str = "Loot this corpse by the loot list.";
pub const HINT_LOOT_BAG: &str = "Set the bag grabbed items go into.";

/// An item the search does not match shows this opaque.
pub const DIMMED_ALPHA: f32 = 0.3;
/// Pictures in a cell grow no more than this when the Containers page
/// scales them, and not at all when it does not.
pub const SCALED_ART: f32 = 2.0;
pub const NATURAL_ART: f32 = 1.0;
/// The height of the row of the chosen items at the foot of a grid.
pub const STRIP_ROW: f32 = 26.0;
const PERCENT: f32 = 100.0;

/// How much a picture in a cell may grow, by the Containers page.
pub fn art_most_scale(profile: &Profile) -> f32 {
    if profile.containers.scale_items {
        SCALED_ART
    } else {
        NATURAL_ART
    }
}

/// The opacity of the glass of a grid, 0 to 1.
pub fn grid_opacity(profile: &Profile) -> f32 {
    f32::from(profile.containers.grid_opacity) / PERCENT
}

/// The hue of the edge of a grid, when the Containers page sets one.
pub fn border_hue(profile: &Profile) -> Option<u16> {
    let hue = profile.containers.grid_border_hue;
    (hue != 0).then_some(hue)
}

/// What a click on an item of a grid does, as the tip says.
pub fn item_footer(live: bool, grid_loot: bool) -> &'static str {
    match (live, grid_loot) {
        (false, _) => "",
        (true, true) => HINT_LOOT_ITEM,
        (true, false) => HINT_ITEM,
    }
}

/// The containers the player closed. The game keeps no "closed" state for
/// a container, so the window keeps it. A use of the container shows it
/// again.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClosedBoxes {
    closed: HashSet<u32>,
}

impl ClosedBoxes {
    /// True when the window shows this container now.
    pub fn shows(&self, frame: &WatchFrame, container: u32) -> bool {
        !self.closed.contains(&container) && frame.containers.iter().any(|c| c.serial == container)
    }

    pub fn is_closed(&self, container: u32) -> bool {
        self.closed.contains(&container)
    }

    pub fn close(&mut self, container: u32) {
        self.closed.insert(container);
    }

    /// Call this when the human uses a thing. A closed container shows again.
    pub fn used(&mut self, thing: u32) {
        self.closed.remove(&thing);
    }

    /// Forgets the containers that are no longer open.
    pub fn keep_open(&mut self, frame: &WatchFrame) {
        self.closed
            .retain(|serial| frame.containers.iter().any(|c| c.serial == *serial));
    }
}

/// What the player keeps of the grids between frames: the words of each
/// search, the first row each shows, the items he chose to move together,
/// and how many of each pile of a grid loot a click grabs.
#[derive(Default)]
pub struct GridMemory {
    pub search: HashMap<u32, String>,
    pub first_rows: HashMap<u32, usize>,
    pub chosen: Selection,
    pub amounts: LootAmounts,
}

impl GridMemory {
    /// Forgets what belongs to containers and items that are gone.
    pub fn keep_present(&mut self, frame: &WatchFrame) {
        self.chosen.keep_present(frame);
        let open = |serial: &u32| frame.containers.iter().any(|c| c.serial == *serial);
        self.search.retain(|serial, _| open(serial));
        self.first_rows.retain(|serial, _| open(serial));
        self.amounts.keep_only(|serial| {
            frame
                .containers
                .iter()
                .any(|c| c.items.iter().any(|item| item.serial == serial))
        });
    }

    /// The words of the search of a grid.
    pub fn search_words(&self, container: u32) -> &str {
        self.search.get(&container).map_or("", String::as_str)
    }

    /// True while the search of a grid has words.
    pub fn searching(&self, container: u32) -> bool {
        !self.search_words(container).trim().is_empty()
    }

    /// The first row of a grid after the wheel turned `turned` over it.
    pub fn scroll(&mut self, container: u32, turned: f32, last_first_row: usize) -> usize {
        let row = self.first_rows.entry(container).or_default();
        *row = next_first_row(*row, turned, last_first_row);
        *row
    }

    /// The amount a cell shows: how many a click grabs of a pile with a
    /// slider, else the whole pile.
    pub fn shown_amount(&mut self, item: &WatchPackItem, slider: bool) -> i32 {
        let most = i32::from(item.amount);
        if slider {
            *self.amounts.slot(item.serial, most)
        } else {
            most
        }
    }

    /// Sets how many of a pile a click grabs, from the share of its slider.
    pub fn set_amount(&mut self, item: &WatchPackItem, share: f32) {
        let most = i32::from(item.amount);
        *self.amounts.slot(item.serial, most) = loot::amount_at(share, most);
    }

    /// The act of a click on an item of a grid loot: the amount of its
    /// slider goes into the grab bag.
    pub fn grab_act(&self, item: u32, bag: u32) -> Act {
        Act::Move {
            item,
            amount: self.amounts.taken(item),
            to: DropTo::Into(bag),
        }
    }

    /// Does a button of the row of the chosen items. Gives the acts.
    pub fn strip_press(&mut self, press: StripPress, frame: &WatchFrame) -> Vec<Act> {
        let acts = match press {
            StripPress::MoveInto(bag) => self.chosen.moves_into(bag, frame),
            StripPress::Clear => Vec::new(),
        };
        self.chosen.clear();
        acts
    }
}

/// A pile of a grid loot has a slider of how many a click grabs.
pub fn has_pile_slider(grid_loot: bool, item: &WatchPackItem) -> bool {
    grid_loot && item.amount > 1
}

/// One open container that shows as a grid now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShownGrid {
    pub serial: u32,
    pub corpse: bool,
    /// The corpse loots with one click.
    pub grid_loot: bool,
    /// The property lines of its items are needed: a rule, a search or the
    /// highlight words.
    pub wants_lines: bool,
    /// The id that keeps its place in the profile.
    pub place_id: String,
    /// It is the `nth` grid open, from 0: each first opens a step from the
    /// one before.
    pub nth: usize,
}

/// The containers that show as grids now, and the ones to close: a grid
/// loot whose corpse is out of reach closes. A closed container and an
/// empty corpse the General page skips do not show.
pub fn shown_grids(
    frame: &WatchFrame,
    profile: &Profile,
    closed: &ClosedBoxes,
    memory: &GridMemory,
) -> (Vec<ShownGrid>, Vec<u32>) {
    let options = &profile.containers;
    let wants_lines =
        !options.grid_highlight_rules.is_empty() || !options.grid_highlight_properties.is_empty();
    let corpse_look = loot::corpse_look(profile.general.grid_loot);
    let mut shown = Vec::new();
    let mut shut = Vec::new();
    let mut corpses = 0;
    for container in frame
        .containers
        .iter()
        .filter(|c| !closed.is_closed(c.serial))
    {
        let corpse = loot::is_corpse(frame, container.serial);
        if corpse && !loot::shows_corpse(&profile.general, container.items.len()) {
            continue;
        }
        if corpse && corpse_look.grid && !loot::grid_loot_alive(frame, container.serial) {
            shut.push(container.serial);
            continue;
        }
        let place_id = if corpse {
            corpses += 1;
            corpse_place_id(corpses)
        } else {
            grid::place_id(container.serial)
        };
        shown.push(ShownGrid {
            serial: container.serial,
            corpse,
            grid_loot: corpse && corpse_look.grid,
            wants_lines: wants_lines || memory.searching(container.serial),
            place_id,
            nth: shown.len(),
        });
    }
    (shown, shut)
}

/// Where the `nth` grid first stands in `window`: beside the character, a
/// step from the one before, at the size of the Containers page.
pub fn grid_first_place(window: Area, nth: usize, profile: &Profile) -> Area {
    first_place(window, Spot::Container(nth), first_size(profile))
}

/// The room of the cells of a grid of `panel` size: under the title and
/// the search, over the row of the chosen items when it shows.
pub fn cells_room(panel: Vector, strip: bool) -> Vector {
    let strip_room = if strip { STRIP_ROW + CELL_GAP } else { 0.0 };
    Vector::new(
        panel.x - PANEL_PAD * 2.0,
        panel.y - PANEL_PAD * 2.0 - TITLE_ROW - SEARCH_ROW - CELL_GAP - strip_room,
    )
}

/// How many columns and rows of cells of `side` fit a room of cells.
pub fn cells_fit(room: Vector, side: f32) -> (usize, usize) {
    grid::fits(room.x + CELL_GAP, room.y + CELL_GAP, side + CELL_GAP)
}

/// How a cell is marked: by the hue of the first rule its item passes, or
/// in the goal color for the property words of the Containers page or a
/// match of the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellMark {
    Hue(u16),
    Goal,
}

pub fn cell_mark(
    profile: &Profile,
    lines: &[String],
    corpse: bool,
    look: Look,
) -> Option<CellMark> {
    let options = &profile.containers;
    highlight::first_passed(&options.grid_highlight_rules, lines, corpse)
        .map(|rule| CellMark::Hue(rule.hue))
        .or_else(|| {
            highlight::has_any_words(&options.grid_highlight_properties, lines)
                .then_some(CellMark::Goal)
        })
        .or_else(|| (look == Look::Marked).then_some(CellMark::Goal))
}

/// A button of the row of the chosen items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripPress {
    /// Moves the chosen items into a bag.
    MoveInto(u32),
    Clear,
}

/// The buttons of the row of the chosen items of `container`: move here,
/// to the favorite bag when there is one, and clear.
pub fn strip_buttons(container: u32, favorite: Option<u32>) -> Vec<(&'static str, StripPress)> {
    let mut buttons = vec![(WORDS_MOVE_HERE, StripPress::MoveInto(container))];
    if let Some(bag) = favorite {
        buttons.push((WORDS_TO_FAVORITE, StripPress::MoveInto(bag)));
    }
    buttons.push((WORDS_CLEAR, StripPress::Clear));
    buttons
}

/// True when the bag is the favorite bag.
pub fn is_favorite(profile: &Profile, container: u32) -> bool {
    profile.containers.favorite_bag == Some(container)
}

/// Makes the bag the favorite bag, or no bag when it was.
pub fn toggle_favorite(profile: &mut Profile, container: u32) {
    let favorite = is_favorite(profile, container);
    profile.containers.favorite_bag = (!favorite).then_some(container);
}

/// Shift and a click on a slot: an item locks into its slot, and a locked
/// slot or an empty one is freed.
pub fn toggle_lock(profile: &mut Profile, container: u32, slot: usize, item: Option<u32>) {
    let layout = profile
        .containers
        .grid_layouts
        .entry(grid::layout_key(container))
        .or_default();
    match item.filter(|_| !grid::is_locked(layout, slot)) {
        Some(item) => grid::lock(layout, slot, item),
        None => grid::unlock(layout, slot),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchContainer, WatchItem};
    use crate::model::loot::CORPSE_GRAPHIC;
    use crate::settings::GridLoot;
    use crate::ui::grid_clicks::cell_side;

    const BAG: u32 = 0x4000_0100;
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
            containers: vec![
                WatchContainer {
                    serial: BAG,
                    ..WatchContainer::default()
                },
                WatchContainer {
                    serial: CORPSE,
                    items: vec![WatchPackItem {
                        serial: COINS,
                        graphic: 0x0EED,
                        amount: 30,
                        ..WatchPackItem::default()
                    }],
                    ..WatchContainer::default()
                },
            ],
            ..WatchFrame::default()
        }
    }

    #[test]
    fn a_closed_container_shows_again_when_it_is_used() {
        let frame = corpse_at(2);
        let mut closed = ClosedBoxes::default();
        assert!(closed.shows(&frame, BAG));
        closed.close(BAG);
        assert!(!closed.shows(&frame, BAG));
        closed.used(BAG);
        assert!(closed.shows(&frame, BAG));
        closed.close(BAG);
        closed.keep_open(&WatchFrame::default());
        assert!(!closed.is_closed(BAG), "a container gone is forgotten");
    }

    #[test]
    fn a_grid_loot_shows_near_and_closes_out_of_reach() {
        let mut profile = Profile::default();
        profile.general.grid_loot = GridLoot::GridOnly;
        let memory = GridMemory::default();
        let closed = ClosedBoxes::default();
        let (shown, shut) = shown_grids(&corpse_at(2), &profile, &closed, &memory);
        assert_eq!(shown.len(), 2);
        assert!(shut.is_empty());
        let corpse = &shown[1];
        assert!(corpse.corpse && corpse.grid_loot);
        assert_eq!((corpse.place_id.as_str(), corpse.nth), ("grid:corpse:1", 1));
        assert_eq!(shown[0].place_id, grid::place_id(BAG));
        let (shown, shut) = shown_grids(&corpse_at(9), &profile, &closed, &memory);
        assert_eq!((shown.len(), shut), (1, vec![CORPSE]));
    }

    #[test]
    fn a_grid_first_holds_the_cells_of_the_containers_page() {
        let profile = Profile::default();
        let size = first_size(&profile);
        let fit = cells_fit(cells_room(size, false), cell_side(&profile));
        let options = &profile.containers;
        assert_eq!(
            fit,
            (
                usize::from(options.grid_columns),
                usize::from(options.grid_rows)
            )
        );
        let (_, rows) = cells_fit(cells_room(size, true), cell_side(&profile));
        assert!(
            rows < usize::from(options.grid_rows),
            "the strip takes room"
        );
    }

    #[test]
    fn the_chosen_items_move_together_and_the_choice_clears() {
        let frame = corpse_at(2);
        let mut memory = GridMemory::default();
        memory.chosen.toggle(COINS);
        let buttons = strip_buttons(CORPSE, Some(BAG));
        assert_eq!(buttons.len(), 3);
        assert_eq!(strip_buttons(CORPSE, None).len(), 2, "no favorite bag");
        let acts = memory.strip_press(StripPress::MoveInto(BAG), &frame);
        assert_eq!(acts.len(), 1);
        assert!(memory.chosen.is_empty());
    }

    #[test]
    fn a_shift_click_locks_and_frees_a_slot_and_the_favorite_turns() {
        let mut profile = Profile::default();
        toggle_lock(&mut profile, BAG, 3, Some(COINS));
        let key = grid::layout_key(BAG);
        assert!(grid::is_locked(&profile.containers.grid_layouts[&key], 3));
        toggle_lock(&mut profile, BAG, 3, Some(COINS));
        assert!(!grid::is_locked(&profile.containers.grid_layouts[&key], 3));
        toggle_favorite(&mut profile, BAG);
        assert!(is_favorite(&profile, BAG));
        toggle_favorite(&mut profile, BAG);
        assert_eq!(profile.containers.favorite_bag, None);
    }

    #[test]
    fn a_pile_of_a_grid_loot_starts_whole_and_its_slider_sets_the_grab() {
        let mut memory = GridMemory::default();
        let coins = &corpse_at(2).containers[1].items[0];
        assert!(has_pile_slider(true, coins) && !has_pile_slider(false, coins));
        assert_eq!(memory.shown_amount(coins, true), 30);
        memory.set_amount(coins, 0.0);
        assert_eq!(memory.shown_amount(coins, true), 1);
        assert_eq!(
            memory.grab_act(COINS, BAG),
            Act::Move {
                item: COINS,
                amount: 1,
                to: DropTo::Into(BAG)
            }
        );
        assert_eq!(item_footer(true, true), HINT_LOOT_ITEM);
        assert_eq!(item_footer(false, true), "");
    }
}
