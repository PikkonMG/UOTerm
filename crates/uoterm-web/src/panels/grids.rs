//! The grid containers of the Modern style and the nearby-loot window.
//! Each open container is a panel of cells; which ones show, what each
//! cell shows and what a press on it does are the rules of
//! `uoterm_view::ui::grids` and `ui::grid_clicks`, as in the Rust window.
//! The page sends the raw facts of a press (the slot, the button, the
//! keys); the view decides. The clicks work only while the human has
//! control.

use super::{Colored, DropZone, FrameSpec, Framed, TipKey, PANEL_GRID_PREFIX, PANEL_LOOT};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use uoterm_view::act::Act;
use uoterm_view::art::WorldArt;
use uoterm_view::frame::{WatchContainer, WatchFrame, WatchPackItem};
use uoterm_view::geom::{Point, Rgba};
use uoterm_view::guard::{LocalAim, AIM_GRAB_BAG};
use uoterm_view::input::{Mods, PointerButton};
use uoterm_view::model::agents::AUTOLOOT;
use uoterm_view::model::clicks::ClickDelay;
use uoterm_view::model::compare::ItemLayers;
use uoterm_view::model::grid::{self, Look};
use uoterm_view::model::loot::{self, share_of, NEARBY_LOOT_TILES};
use uoterm_view::model::properties;
use uoterm_view::ui::grid_clicks::{
    cell_side, grid_click, grid_title, hover_lines, least_size, CellPress, GridClick,
};
use uoterm_view::ui::grids::{
    art_most_scale, border_hue, cell_mark, cells_fit, cells_room, grid_first_place, grid_opacity,
    has_pile_slider, is_favorite, item_footer, shown_grids, strip_buttons, toggle_favorite,
    toggle_lock, CellMark, ClosedBoxes, GridMemory, ShownGrid, DIMMED_ALPHA, HINT_FAVORITE,
    HINT_LOOT_ALL, HINT_LOOT_BAG, HINT_SEARCH, WORDS_FAVORITE, WORDS_LOOT_ALL, WORDS_LOOT_BAG,
};
use uoterm_view::ui::gumps::{single_or_double, CELL_GAP};
use uoterm_view::ui::launch::Launch;
use uoterm_view::ui::lists::{
    corpse_state_words, loot_first_place, open_corpses, LOOT_ID, LOOT_MAX_ROWS,
    WORDS_LOOT_ALL_NEAR, WORDS_LOOT_ONE, WORDS_LOOT_OPEN, WORDS_LOOT_TITLE, WORDS_NO_CORPSE,
};
use uoterm_view::ui::ring::Subject as RingSubject;
use uoterm_view::ui::theme::{css_color, GLASS, GOAL, TEXT, TEXT_DIM, WAITING};

/// A click event of the page whose count is this or more is the second
/// click of a double click, which comes as its own press.
const DOUBLE_CLICK_COUNT: u32 = 2;
const PLAIN_ALPHA: f32 = 1.0;

/// What the grids and the nearby loot keep between frames.
#[derive(Default)]
pub(crate) struct GridsState {
    pub closed: ClosedBoxes,
    memory: GridMemory,
    /// A click on an item asks its name once no double click follows.
    clicks: ClickDelay,
    /// The corpses the nearby loot opened, so each opens once.
    loot_opened: HashSet<u32>,
    /// The layer each wearable graphic is worn on, once the tiles came.
    layers: Option<ItemLayers>,
}

/// One grid container.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GridData {
    pub serial: u32,
    pub live: bool,
    /// The glass of the grid and how opaque it is, by the Containers page.
    pub glass: String,
    pub glass_opacity: f32,
    pub search: String,
    pub search_hint: &'static str,
    /// How many items it holds.
    pub count: String,
    pub favorite: Option<TitleButton>,
    pub loot_all: Option<TitleButton>,
    pub loot_bag: Option<TitleButton>,
    pub columns: usize,
    /// The side of a cell and the gap between two, and how much a picture
    /// in it may grow.
    pub side: f32,
    pub gap: f32,
    pub art_scale: f32,
    pub cells: Vec<GridCell>,
    /// The row that moves the chosen items together.
    pub strip: Option<StripData>,
    /// An item dropped on the grid goes into the container.
    pub zone: DropZone,
}

/// A button of the head of a grid.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TitleButton {
    pub words: &'static str,
    pub color: String,
    pub hint: &'static str,
}

/// One cell of a grid.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GridCell {
    pub slot: usize,
    /// The player locked an item into this slot.
    pub locked: bool,
    pub item: Option<GridItem>,
}

/// The item of a cell.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GridItem {
    pub serial: u32,
    pub picture: Option<String>,
    /// How opaque it shows: a search it does not match dims it.
    pub alpha: f32,
    /// How many a click takes, or the whole pile.
    pub amount: Option<Colored>,
    /// The edge of a rule, a property word or a match of the search.
    pub mark: Option<String>,
    pub chosen: bool,
    /// The share of the slider of a pile of a grid loot.
    pub slider: Option<f32>,
    pub hover: TipKey,
    /// An item dropped on a bag goes in, and on a pile of its kind joins.
    pub zone: DropZone,
}

/// The row of the chosen items.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StripData {
    pub buttons: Vec<&'static str>,
    pub count: String,
}

/// The nearby-loot window.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LootData {
    pub live: bool,
    pub rows: Vec<LootRow>,
    pub none: Option<&'static str>,
    pub open: &'static str,
    pub loot: &'static str,
    pub loot_all: Option<&'static str>,
}

/// One corpse near the character.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LootRow {
    pub serial: u32,
    pub name: String,
    pub state: String,
}

/// A press on a cell: its slot, the second press of a double click, how
/// many clicks the page counted, the right button, and the keys held.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
struct CellAction {
    slot: usize,
    double: bool,
    count: u32,
    secondary: bool,
    mods: Mods,
    x: f32,
    y: f32,
}

/// A slot and a share of its slider.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
struct SlotShare {
    slot: usize,
    share: f32,
}

/// What the player does on a grid.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GridAction {
    Cell(CellAction),
    /// A drag started on the slot.
    Drag(usize),
    Search(String),
    Favorite(bool),
    LootAll(bool),
    LootBag(bool),
    /// A button of the row of the chosen items, by its place.
    Strip(usize),
    Amount(SlotShare),
}

/// What the player does on the nearby loot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LootAction {
    Open(u32),
    Loot(u32),
    LootAll(bool),
}

/// The cells of a grid as they stand: the item in each slot.
struct Arranged {
    columns: usize,
    slots: Vec<Option<u32>>,
}

fn title_button(words: &'static str, color: Rgba, hint: &'static str) -> TitleButton {
    TitleButton {
        words,
        color: css_color(color),
        hint,
    }
}

impl WebView {
    /// The grids and the nearby loot in one frame, as the Rust window
    /// draws them: what is gone is forgotten, a grid loot out of reach
    /// closes, the corpse options open corpses, and a click that waited
    /// long enough asks the name.
    pub(crate) fn follow_grids(&mut self, frame: &WatchFrame, time: f64) {
        let grids = &mut self.panels.grids;
        grids.memory.keep_present(frame);
        grids.closed.keep_open(frame);
        let (_, shut) = shown_grids(frame, &self.profile, &grids.closed, &grids.memory);
        for serial in shut {
            grids.closed.close(serial);
        }
        if grids.layers.is_none() {
            grids.layers = self.art.tile_data().map(ItemLayers::of_tiles);
        }
        let corpses = open_corpses(&mut grids.loot_opened, frame, &self.profile);
        if let Some(act) = grids.clicks.due_look(time) {
            self.hand.act(act);
        }
        for corpse in corpses {
            self.hand.act(Act::Use(corpse));
        }
    }

    /// The grid of panel `panel` that shows now.
    fn shown_grid(&self, frame: &WatchFrame, panel: &str) -> Option<ShownGrid> {
        let serial: u32 = panel.strip_prefix(PANEL_GRID_PREFIX)?.parse().ok()?;
        let grids = &self.panels.grids;
        let (shown, _) = shown_grids(frame, &self.profile, &grids.closed, &grids.memory);
        shown.into_iter().find(|grid| grid.serial == serial)
    }

    fn grid_spec(&self, frame: &WatchFrame, shown: &ShownGrid) -> Option<FrameSpec> {
        let container = frame.containers.iter().find(|c| c.serial == shown.serial)?;
        let tile_name = self
            .art
            .item_tile(container.graphic)
            .map(|tile| tile.name.as_str());
        let title = grid_title(&container.name, tile_name);
        let default = grid_first_place(self.panel_room(), shown.nth, &self.profile);
        Some(
            FrameSpec::fixed(&shown.place_id, &title, default)
                .sized(least_size(&self.profile))
                .closable(),
        )
    }

    pub(super) fn grid_panel_spec(&self, frame: &WatchFrame, panel: &str) -> Option<FrameSpec> {
        self.grid_spec(frame, &self.shown_grid(frame, panel)?)
    }

    /// The row of the chosen items shows.
    fn strip_shows(&self, frame: &WatchFrame) -> bool {
        !self.panels.grids.memory.chosen.is_empty() && frame.human_control
    }

    /// The slots of a grid as the panel stands now.
    fn arranged(
        &self,
        frame: &WatchFrame,
        spec: &FrameSpec,
        container: &WatchContainer,
    ) -> Arranged {
        let room = cells_room(self.panel_area(spec).size(), self.strip_shows(frame));
        let (columns, rows) = cells_fit(room, cell_side(&self.profile));
        let layout = self
            .profile
            .containers
            .grid_layouts
            .get(&grid::layout_key(container.serial))
            .cloned()
            .unwrap_or_default();
        let items: Vec<u32> = container.items.iter().map(|item| item.serial).collect();
        Arranged {
            columns,
            slots: grid::arrange(&items, &layout, columns * rows),
        }
    }

    /// The property lines of an item, when the grid needs them.
    fn wanted_lines(&mut self, shown: &ShownGrid, item: u32) -> Vec<String> {
        if shown.wants_lines {
            properties::lines_of(self.hand.reads(), item)
        } else {
            Vec::new()
        }
    }

    pub(super) fn grids_data(&mut self, frame: &WatchFrame) -> Vec<Framed<GridData>> {
        let grids = &self.panels.grids;
        let (shown, _) = shown_grids(frame, &self.profile, &grids.closed, &grids.memory);
        let mut data = Vec::new();
        for grid in shown {
            let Some(container) = frame.containers.iter().find(|c| c.serial == grid.serial) else {
                continue;
            };
            let Some(spec) = self.grid_spec(frame, &grid) else {
                continue;
            };
            let body = self.grid_body(frame, &grid, &spec, container);
            let panel = format!("{PANEL_GRID_PREFIX}{}", grid.serial);
            let mut framed = self.framed(&panel, &spec, body);
            framed.frame.edge = border_hue(&self.profile)
                .map(|hue| css_color(uoterm_view::art::hue_color(&self.art, hue)));
            data.push(framed);
        }
        data
    }

    fn grid_body(
        &mut self,
        frame: &WatchFrame,
        shown: &ShownGrid,
        spec: &FrameSpec,
        container: &WatchContainer,
    ) -> GridData {
        let live = frame.human_control;
        let serial = container.serial;
        let arranged = self.arranged(frame, spec, container);
        let layout = self
            .profile
            .containers
            .grid_layouts
            .get(&grid::layout_key(serial))
            .cloned()
            .unwrap_or_default();
        let search = self.panels.grids.memory.search_words(serial).to_string();
        let mut cells = Vec::new();
        for (slot, held) in arranged.slots.iter().enumerate() {
            let item = held.and_then(|held| container.items.iter().find(|i| i.serial == held));
            let item = item.and_then(|item| self.grid_item(frame, shown, item, &search));
            cells.push(GridCell {
                slot,
                locked: grid::is_locked(&layout, slot),
                item,
            });
        }
        let favorite = (live && !shown.corpse).then(|| {
            let color = if is_favorite(&self.profile, serial) {
                WAITING
            } else {
                TEXT_DIM
            };
            title_button(WORDS_FAVORITE, color, HINT_FAVORITE)
        });
        let grid_loot = live && shown.grid_loot;
        let memory = &self.panels.grids.memory;
        GridData {
            serial,
            live,
            glass: css_color(GLASS),
            glass_opacity: grid_opacity(&self.profile),
            search,
            search_hint: HINT_SEARCH,
            count: container.total.to_string(),
            favorite,
            loot_all: grid_loot.then(|| title_button(WORDS_LOOT_ALL, GOAL, HINT_LOOT_ALL)),
            loot_bag: grid_loot.then(|| title_button(WORDS_LOOT_BAG, TEXT, HINT_LOOT_BAG)),
            columns: arranged.columns,
            side: cell_side(&self.profile),
            gap: CELL_GAP,
            art_scale: art_most_scale(&self.profile),
            cells,
            strip: self.strip_shows(frame).then(|| StripData {
                buttons: strip_buttons(serial, self.profile.containers.favorite_bag)
                    .into_iter()
                    .map(|(words, _)| words)
                    .collect(),
                count: memory.chosen.len().to_string(),
            }),
            zone: DropZone::Into(serial),
        }
    }

    /// What a cell shows of its item. None for an item the search hides.
    fn grid_item(
        &mut self,
        frame: &WatchFrame,
        shown: &ShownGrid,
        item: &WatchPackItem,
        search: &str,
    ) -> Option<GridItem> {
        let lines = self.wanted_lines(shown, item.serial);
        let look = grid::search_look(
            self.profile.containers.grid_search,
            search,
            &item.name,
            &lines,
        );
        if look == Look::Hidden {
            return None;
        }
        let alpha = if look == Look::Dimmed {
            DIMMED_ALPHA
        } else {
            PLAIN_ALPHA
        };
        let slider = has_pile_slider(shown.grid_loot, item);
        let memory = &mut self.panels.grids.memory;
        let taken = memory.shown_amount(item, slider);
        let most = i32::from(item.amount);
        let amount = (item.amount > 1).then(|| Colored {
            words: taken.to_string(),
            color: css_color(if taken < most { GOAL } else { TEXT }),
        });
        let chosen = memory.chosen.contains(item.serial);
        let mark = cell_mark(&self.profile, &lines, shown.corpse, look).map(|mark| match mark {
            CellMark::Hue(hue) => css_color(uoterm_view::art::hue_color(&self.art, hue)),
            CellMark::Goal => css_color(GOAL),
        });
        let mut hover = TipKey::thing(
            item.serial,
            &item.name,
            item_footer(frame.human_control, shown.grid_loot),
        );
        hover.in_grid = true;
        Some(GridItem {
            serial: item.serial,
            picture: self.item_picture(item),
            alpha,
            amount,
            mark,
            chosen,
            slider: (slider && frame.human_control).then(|| share_of(taken, most)),
            hover,
            zone: DropZone::Into(item.serial),
        })
    }

    /// The lines under the name in the tip of an item of a grid: the
    /// compare with the worn item, and what a bag holds.
    pub(crate) fn grid_tip_lines(&mut self, serial: u32) -> Vec<String> {
        let Some(frame) = self.frame.clone() else {
            return Vec::new();
        };
        let Some(item) = frame
            .containers
            .iter()
            .flat_map(|container| &container.items)
            .find(|item| item.serial == serial)
        else {
            return Vec::new();
        };
        let layers = self.panels.grids.layers.clone().unwrap_or_default();
        hover_lines(
            item,
            &[],
            &frame,
            (&layers, self.hand.reads()),
            &self.profile,
        )
    }

    pub(super) fn grid_action(&mut self, panel: &str, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Some(shown) = self.shown_grid(&frame, panel) else {
            return;
        };
        let Some(spec) = self.grid_spec(&frame, &shown) else {
            return;
        };
        let Some(container) = frame.containers.iter().find(|c| c.serial == shown.serial) else {
            return;
        };
        let Ok(action) = serde_json::from_value::<GridAction>(action) else {
            return;
        };
        let serial = container.serial;
        match action {
            GridAction::Cell(press) if press.double || press.count < DOUBLE_CLICK_COUNT => {
                let button = if press.secondary {
                    PointerButton::Secondary
                } else {
                    PointerButton::Primary
                };
                let cell = CellPress {
                    click: Some(button),
                    double: press.double,
                    drag_started: false,
                    mods: press.mods,
                };
                let at = Point::new(press.x, press.y);
                self.cell_press(&frame, &shown, &spec, container, press.slot, cell, at);
            }
            GridAction::Cell(_) => {}
            GridAction::Drag(slot) => {
                let cell = CellPress {
                    drag_started: true,
                    ..CellPress::default()
                };
                let at = Point::default();
                self.cell_press(&frame, &shown, &spec, container, slot, cell, at);
            }
            GridAction::Search(words) => {
                self.panels.grids.memory.search.insert(serial, words);
            }
            GridAction::Favorite(_) if !shown.corpse => {
                toggle_favorite(&mut self.profile, serial);
                self.keep_profile();
            }
            GridAction::LootAll(_) if shown.grid_loot => self.hand.act(Act::Loot(serial)),
            GridAction::LootBag(_) if shown.grid_loot => {
                self.hand.report(AIM_GRAB_BAG);
                self.hand.aim(LocalAim::SetGrabBag);
            }
            GridAction::Strip(at) => {
                let buttons = strip_buttons(serial, self.profile.containers.favorite_bag);
                if let Some((_, press)) = buttons.get(at).copied() {
                    for act in self.panels.grids.memory.strip_press(press, &frame) {
                        self.hand.act(act);
                    }
                }
            }
            GridAction::Amount(set) => {
                let arranged = self.arranged(&frame, &spec, container);
                let item = arranged
                    .slots
                    .get(set.slot)
                    .copied()
                    .flatten()
                    .and_then(|held| container.items.iter().find(|i| i.serial == held));
                if let Some(item) = item.filter(|item| has_pile_slider(shown.grid_loot, item)) {
                    self.panels.grids.memory.set_amount(item, set.share);
                }
            }
            GridAction::Favorite(_) | GridAction::LootAll(_) | GridAction::LootBag(_) => {}
        }
    }

    /// A press on a cell, as the grid of the Rust window takes it.
    #[allow(clippy::too_many_arguments)]
    fn cell_press(
        &mut self,
        frame: &WatchFrame,
        shown: &ShownGrid,
        spec: &FrameSpec,
        container: &WatchContainer,
        slot: usize,
        press: CellPress,
        at: Point,
    ) {
        let serial = container.serial;
        let arranged = self.arranged(frame, spec, container);
        let item = arranged
            .slots
            .get(slot)
            .copied()
            .flatten()
            .and_then(|held| container.items.iter().find(|i| i.serial == held));
        let Some(item) = item else {
            let lock = press.click == Some(PointerButton::Primary) && press.mods.shift;
            if lock && !shown.corpse {
                toggle_lock(&mut self.profile, serial, slot, None);
                self.keep_profile();
            }
            return;
        };
        let search = self.panels.grids.memory.search_words(serial).to_string();
        let lines = self.wanted_lines(shown, item.serial);
        let look = grid::search_look(
            self.profile.containers.grid_search,
            &search,
            &item.name,
            &lines,
        );
        if look == Look::Hidden {
            return;
        }
        let time = self.hand.time();
        match grid_click(press, shown.corpse, shown.grid_loot, frame.target_cursor) {
            GridClick::Choose => self.panels.grids.memory.chosen.toggle(item.serial),
            GridClick::Lock => {
                toggle_lock(&mut self.profile, serial, slot, Some(item.serial));
                self.keep_profile();
            }
            GridClick::Grab => {
                if let Some(bag) = self.hand.grab_bag() {
                    let act = self.panels.grids.memory.grab_act(item.serial, bag);
                    self.hand.act(act);
                }
            }
            GridClick::PickUp => self.pick_up(item),
            GridClick::Use | GridClick::Name => {
                let clicks = &mut self.panels.grids.clicks;
                let (double, act) =
                    single_or_double((true, press.double), clicks, frame, item.serial, time);
                if let Some(act) = act {
                    self.hand.act(act);
                }
                if double {
                    self.hand.act(Act::Use(item.serial));
                }
            }
            GridClick::Ring => self.open_ring(at, item.serial, &item.name, RingSubject::Packed),
            GridClick::Nothing => {}
        }
    }

    /// Closes a grid by its close mark.
    pub(super) fn close_grid(&mut self, panel: &str) {
        if let Some(serial) = panel
            .strip_prefix(PANEL_GRID_PREFIX)
            .and_then(|serial| serial.parse().ok())
        {
            self.panels.grids.closed.close(serial);
        }
    }

    /// The bag button: a bag that shows closes; else it shows and opens.
    pub(crate) fn toggle_bag(&mut self, frame: &WatchFrame, bag: u32) {
        if let Some(act) = self.panels.grids.closed.toggle(frame, bag) {
            self.hand.act(act);
        }
    }

    pub(super) fn loot_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        if !self.profile.interface.nearby_loot_window {
            return None;
        }
        let corpses = loot::nearby_corpses(frame, NEARBY_LOOT_TILES).len();
        let default = loot_first_place(self.panel_room(), corpses);
        Some(FrameSpec::fixed(LOOT_ID, WORDS_LOOT_TITLE, default).closable())
    }

    pub(super) fn loot_data(&self, frame: &WatchFrame) -> Option<Framed<LootData>> {
        let spec = self.loot_spec(frame)?;
        let live = frame.human_control;
        let corpses = loot::nearby_corpses(frame, NEARBY_LOOT_TILES);
        let body = LootData {
            live,
            rows: corpses
                .iter()
                .take(LOOT_MAX_ROWS)
                .map(|corpse| LootRow {
                    serial: corpse.serial,
                    name: corpse.name.clone(),
                    state: corpse_state_words(corpse),
                })
                .collect(),
            none: corpses.is_empty().then_some(WORDS_NO_CORPSE),
            open: WORDS_LOOT_OPEN,
            loot: WORDS_LOOT_ONE,
            loot_all: (live && !corpses.is_empty()).then_some(WORDS_LOOT_ALL_NEAR),
        };
        Some(self.framed(PANEL_LOOT, &spec, body))
    }

    pub(super) fn loot_action(&mut self, action: Value) {
        if !self.frame.as_ref().is_some_and(|frame| frame.human_control) {
            return;
        }
        let Ok(action) = serde_json::from_value::<LootAction>(action) else {
            return;
        };
        match action {
            LootAction::Open(corpse) => {
                self.panels.grids.loot_opened.insert(corpse);
                self.hand.act(Act::Use(corpse));
            }
            LootAction::Loot(corpse) => self.hand.act(Act::Loot(corpse)),
            LootAction::LootAll(_) => self.hand.act(Act::AgentRun {
                agent: AUTOLOOT.to_string(),
                list: None,
            }),
        }
    }

    /// Closes the nearby loot by its close mark.
    pub(super) fn close_loot(&mut self) {
        Launch::Loot.set(&mut self.profile, false);
        self.keep_profile();
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, saved_profiles};
    use super::super::{PANEL_BAR, PANEL_GRID_PREFIX, PANEL_LOOT, PANEL_TIPS};
    use super::*;
    use crate::tests::{fixture_watch_with_backpack, settled, HATCHET};
    use serde_json::json;
    use uoterm_view::act::DropTo;
    use uoterm_view::settings::GridLoot;

    const BACKPACK: u32 = 0x4000_0001;
    const CORPSE: u32 = 0x4000_0200;
    const COINS: u32 = 0x4000_0201;
    const SHIFT: Mods = Mods {
        ctrl: false,
        alt: false,
        shift: true,
        command: false,
    };
    const COMMAND: Mods = Mods {
        ctrl: true,
        alt: false,
        shift: false,
        command: true,
    };

    /// Mara with her backpack open, and a corpse beside her, open too.
    fn watch(control: bool) -> serde_json::Value {
        let mut watch: serde_json::Value =
            serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["human_control"] = json!(control);
        watch["self_state"]["equipment"] =
            json!([{ "serial": BACKPACK, "graphic": 0x0E75, "layer": 21 }]);
        watch["items"] = json!([{ "serial": CORPSE, "graphic": loot::CORPSE_GRAPHIC,
            "name": "a corpse", "location": { "x": 1001, "y": 1000, "z": 0 } }]);
        watch["containers"] = json!([
            { "serial": BACKPACK, "name": "backpack", "total": 1, "contents": [
                { "serial": HATCHET, "graphic": 0x0F43, "amount": 1, "name": "hatchet" }] },
            { "serial": CORPSE, "gump": loot::CORPSE_GUMP, "total": 1, "contents": [
                { "serial": COINS, "graphic": 0x0EED, "amount": 30, "name": "gold" }] }
        ]);
        watch
    }

    fn view_with_grids() -> WebView {
        let mut view = settled();
        view.frame(&watch(true).to_string(), 0.1);
        view.take_out_native();
        view
    }

    fn pack() -> String {
        format!("{PANEL_GRID_PREFIX}{BACKPACK}")
    }

    fn corpse() -> String {
        format!("{PANEL_GRID_PREFIX}{CORPSE}")
    }

    fn slot_of(view: &mut WebView, panel: &str, serial: u32) -> usize {
        let grids = view.panel_data(0.0).grids;
        let grid = grids.iter().find(|grid| grid.frame.panel == panel).unwrap();
        grid.body
            .cells
            .iter()
            .find(|cell| cell.item.as_ref().is_some_and(|item| item.serial == serial))
            .unwrap()
            .slot
    }

    #[test]
    fn the_backpack_shows_as_a_grid_and_a_double_click_uses_an_item() {
        let mut view = view_with_grids();
        let grids = view.panel_data(0.0).grids;
        assert_eq!(grids.len(), 2);
        assert_eq!(grids[0].frame.title, "Backpack");
        let slot = slot_of(&mut view, &pack(), HATCHET);
        let click = json!({"cell": {"slot": slot}});
        assert!(out_acts(&press(&mut view, &pack(), click.clone())).is_empty());
        let second = json!({"cell": {"slot": slot, "count": 2}});
        assert!(out_acts(&press(&mut view, &pack(), second)).is_empty());
        let double = json!({"cell": {"slot": slot, "double": true, "count": 2}});
        let out = press(&mut view, &pack(), double);
        assert_eq!(out_acts(&out), vec![Act::Use(HATCHET).for_page()]);
        // A single click asks the name once the double click time is over.
        press(&mut view, &pack(), click);
        view.follow_grids(&watch_frame(&view), 5.0);
        assert_eq!(
            out_acts(&view.take_out_native()),
            vec![Act::Look(HATCHET).for_page()]
        );
    }

    fn watch_frame(view: &WebView) -> WatchFrame {
        view.frame_ref().unwrap().clone()
    }

    #[test]
    fn a_press_makes_the_act_of_the_window_by_grid_click() {
        let mut view = view_with_grids();
        let slot = slot_of(&mut view, &pack(), HATCHET);
        let frame = watch_frame(&view);
        let cases = [
            (json!({"slot": slot, "mods": COMMAND}), GridClick::Choose),
            (
                json!({"slot": slot, "secondary": true, "x": 5, "y": 5}),
                GridClick::Ring,
            ),
        ];
        for (press_json, expected) in cases {
            let cell = CellPress {
                click: Some(if press_json["secondary"] == json!(true) {
                    PointerButton::Secondary
                } else {
                    PointerButton::Primary
                }),
                mods: serde_json::from_value(press_json["mods"].clone()).unwrap_or_default(),
                ..CellPress::default()
            };
            assert_eq!(
                grid_click(cell, false, false, frame.target_cursor),
                expected
            );
            press(&mut view, &pack(), json!({ "cell": press_json }));
        }
        let data = view.panel_data(0.0).grids;
        let chosen = data[0].body.cells.iter().filter_map(|c| c.item.as_ref());
        assert!(chosen.clone().any(|item| item.chosen), "Ctrl chose it");
        assert_eq!(data[0].body.strip.as_ref().unwrap().count, "1");
        assert!(
            view.panel_data(0.0).ring.is_some(),
            "a right click opens the ring"
        );
        let out = press(&mut view, &pack(), json!({"strip": 0}));
        assert_eq!(out_acts(&out), Selection_moves(&frame, HATCHET, BACKPACK));
        press(&mut view, &pack(), json!({"drag": slot}));
        assert!(view.carries(), "a drag picks it up");
    }

    #[allow(non_snake_case)]
    fn Selection_moves(frame: &WatchFrame, item: u32, bag: u32) -> Vec<uoterm_view::act::PageAct> {
        let mut chosen = grid::Selection::default();
        chosen.toggle(item);
        chosen
            .moves_into(bag, frame)
            .into_iter()
            .map(|act| act.for_page())
            .collect()
    }

    #[test]
    fn a_shift_click_locks_the_slot_and_the_close_mark_closes_the_grid() {
        let mut view = view_with_grids();
        let out = press(
            &mut view,
            &pack(),
            json!({"cell": {"slot": 0, "mods": SHIFT}}),
        );
        let saved = saved_profiles(&out);
        let layout = &saved[0].containers.grid_layouts[&grid::layout_key(BACKPACK)];
        assert!(grid::is_locked(layout, 0));
        press(&mut view, &pack(), json!({"close": true}));
        assert_eq!(view.panel_data(0.0).grids.len(), 1, "the corpse stays");
        let out = press(&mut view, PANEL_BAR, json!({"press": "Bag"}));
        assert_eq!(out_acts(&out), vec![Act::Use(BACKPACK).for_page()]);
        assert_eq!(view.panel_data(0.0).grids.len(), 2);
        press(&mut view, PANEL_BAR, json!({"press": "Bag"}));
        assert_eq!(
            view.panel_data(0.0).grids.len(),
            1,
            "the bag button closes it"
        );
    }

    #[test]
    fn a_grid_loot_grabs_the_amount_of_its_slider_into_the_grab_bag() {
        let mut view = view_with_grids();
        let mut profile = view.profile.clone();
        profile.general.grid_loot = GridLoot::GridOnly;
        view.set_profile(&serde_json::to_string(&profile).unwrap());
        // The guard learns the backpack, the grab bag at first, in a frame.
        view.tick_native(0.2, crate::tests::VIEW, None);
        view.take_out_native();
        let slot = slot_of(&mut view, &corpse(), COINS);
        let grids = view.panel_data(0.0).grids;
        let loot = grids
            .iter()
            .find(|grid| grid.frame.panel == corpse())
            .unwrap();
        assert!(loot.body.loot_all.is_some() && loot.body.favorite.is_none());
        press(
            &mut view,
            &corpse(),
            json!({"amount": {"slot": slot, "share": 0.0}}),
        );
        let out = press(&mut view, &corpse(), json!({"cell": {"slot": slot}}));
        let grab = Act::Move {
            item: COINS,
            amount: 1,
            to: DropTo::Into(BACKPACK),
        };
        assert_eq!(out_acts(&out), vec![grab.for_page()]);
        let out = press(&mut view, &corpse(), json!({"loot_all": true}));
        assert_eq!(out_acts(&out), vec![Act::Loot(CORPSE).for_page()]);
    }

    #[test]
    fn an_item_of_a_grid_has_the_tip_of_the_window() {
        let mut view = view_with_grids();
        let hover = view.panel_data(0.0).grids[0].body.cells[0]
            .item
            .as_ref()
            .unwrap()
            .hover
            .clone();
        assert!(hover.in_grid);
        assert_eq!(hover.footer, item_footer(true, false));
        press(&mut view, PANEL_TIPS, json!({ "over": hover }));
        assert_eq!(view.panel_tooltip(0.0).unwrap().lines[0], "hatchet");
    }

    #[test]
    fn the_nearby_loot_opens_loots_and_closes_as_the_window() {
        let mut view = view_with_grids();
        assert!(view.panel_data(0.0).loot.is_none());
        let mut profile = view.profile.clone();
        profile.interface.nearby_loot_window = true;
        view.set_profile(&serde_json::to_string(&profile).unwrap());
        let loot = view.panel_data(0.0).loot.unwrap();
        assert_eq!(loot.body.rows[0].serial, CORPSE);
        let out = press(&mut view, PANEL_LOOT, json!({"open": CORPSE}));
        assert_eq!(out_acts(&out), vec![Act::Use(CORPSE).for_page()]);
        let out = press(&mut view, PANEL_LOOT, json!({"loot": CORPSE}));
        assert_eq!(out_acts(&out), vec![Act::Loot(CORPSE).for_page()]);
        let out = press(&mut view, PANEL_LOOT, json!({"loot_all": true}));
        let run = Act::AgentRun {
            agent: AUTOLOOT.to_string(),
            list: None,
        };
        assert_eq!(out_acts(&out), vec![run.for_page()]);
        let out = press(&mut view, PANEL_LOOT, json!({"close": true}));
        assert!(!saved_profiles(&out)[0].interface.nearby_loot_window);
    }

    #[test]
    fn no_grid_or_loot_acts_without_control() {
        let mut view = view_with_grids();
        view.frame(&watch(false).to_string(), 0.2);
        let slot = slot_of(&mut view, &pack(), HATCHET);
        for action in [
            json!({"cell": {"slot": slot, "double": true}}),
            json!({"cell": {"slot": slot, "mods": SHIFT}}),
            json!({"favorite": true}),
            json!({"drag": slot}),
        ] {
            let out = press(&mut view, &pack(), action);
            assert!(out_acts(&out).is_empty() && saved_profiles(&out).is_empty());
        }
        assert!(!view.carries());
        assert!(out_acts(&press(&mut view, PANEL_LOOT, json!({"loot": CORPSE}))).is_empty());
    }
}
