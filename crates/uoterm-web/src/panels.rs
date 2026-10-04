//! The Modern panels as the page draws them: the data of each panel, built
//! by the shared rules, and the small actions its buttons send, which turn
//! into the same acts the panels of the Rust window make. Each panel has
//! one field of [`PanelData`] and one action type in its module.
//!
//! Every panel the player moves stands in a frame: the page reports where
//! he dragged or sized it, and that he locked, folded, closed or put it
//! back; the view keeps it by the rules of `uoterm_view::ui::places`, as
//! the Rust window does, and saves the profile.

mod asks;
mod bar;
mod bars;
mod deals;
mod deck;
mod desk;
mod doll;
mod grids;
mod hud;
mod journal;
mod pages;
mod radar;
mod ring;
mod sheet;

pub use asks::{DyeData, EntryData, RaceData, TipData};
pub use bar::{ChatData, ControlBarData, LauncherData, QuestionData, ReportData, WaitingData};
pub use bars::{HealthBarData, NearData};
pub use deals::{ShopData, TradeData};
pub use deck::{HotbarAction, HotbarData, HotbarSlot, PickerData};
pub use desk::{CarriedData, DropZone, SplitData};
pub use doll::PaperdollData;
pub use grids::{GridData, LootData};
pub use hud::{ActivityData, PackData, VitalsData};
pub use journal::JournalData;
pub use pages::{BoardData, BookData, OldMenuData};
pub use radar::RadarData;
pub use ring::{RingData, TipKey};
pub use sheet::SheetData;

pub(crate) use asks::AsksState;
pub(crate) use bar::BarState;
pub(crate) use bars::BarsState;
pub(crate) use deals::DealsState;
pub(crate) use deck::DeckState;
pub(crate) use doll::DollState;
pub(crate) use grids::GridsState;
pub(crate) use journal::JournalState;
pub(crate) use pages::PagesState;
pub(crate) use radar::RadarState;
pub(crate) use ring::RingState;
pub(crate) use sheet::SheetState;

use crate::out::Hand;
use crate::{kept, TooltipData, WebView};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point, Vector};
use uoterm_view::model::clicks::ClickDelay;
use uoterm_view::model::places;
use uoterm_view::scene::WHEEL_POINTS_PER_NOTCH;
use uoterm_view::ui::gumps::single_or_double;
use uoterm_view::ui::places::{
    place, HINT_CLOSE, HINT_FOLD, HINT_LOCK, HINT_MOVE, HINT_SIZE, PANEL_WHEEL_POINTS,
};

/// The names the page gives its panels in a `Panel` event.
pub const PANEL_BAR: &str = "bar";
pub const PANEL_CHAT: &str = "chat";
pub const PANEL_QUESTION: &str = "question";
pub const PANEL_LAUNCHER: &str = "launcher";
pub const PANEL_ACTIVITY: &str = "activity";
pub const PANEL_VITALS: &str = "vitals";
pub const PANEL_PACK: &str = "pack";
pub const PANEL_HOTBAR: &str = "hotbar";
pub const PANEL_NEAR: &str = "near";
pub const PANEL_JOURNAL: &str = "journal";
pub const PANEL_RADAR: &str = "radar";
pub const PANEL_SHEET: &str = "sheet";
pub const PANEL_SPLIT: &str = "split";
pub const PANEL_RING: &str = "ring";
pub const PANEL_TIPS: &str = "tips";
pub const PANEL_DESK: &str = "desk";
pub const PANEL_SHOP: &str = "shop";
pub const PANEL_LOOT: &str = "loot";
pub const PANEL_OLD_MENU: &str = "old_menu";
pub const PANEL_BOOK: &str = "book";
pub const PANEL_BOARD: &str = "board";
pub const PANEL_PAPERDOLL: &str = "paperdoll";
pub const PANEL_ENTRY: &str = "entry";
pub const PANEL_RACE: &str = "race";
pub const PANEL_TIP: &str = "tip";
pub const PANEL_DYE: &str = "dye";
/// A grid container is the panel `"grid:{serial}"`.
pub const PANEL_GRID_PREFIX: &str = "grid:";
/// A trade is the panel `"trade:{serial}"`, by the character's box of it.
pub const PANEL_TRADE_PREFIX: &str = "trade:";
/// A health bar of its own is the panel `"health:{serial}"`; the target
/// bar is `"health:target"`.
pub const PANEL_HEALTH_PREFIX: &str = "health:";
const PERCENT: f32 = 100.0;

/// What the page draws of the panels this frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PanelData {
    /// The UI scale and the opacity of the panels, from the profile.
    pub look: Look,
    /// What the title and the marks of a frame do.
    pub hints: FrameHints,
    /// The title of the page.
    pub title: String,
    /// The message in the middle while no picture of the session shows.
    pub waiting: Option<WaitingData>,
    /// How strong the alarm color is over the edge of the map, 0 to 1.
    pub alarm: f32,
    pub bar: Option<ControlBarData>,
    pub launcher: Option<Framed<LauncherData>>,
    pub activity: Option<Framed<ActivityData>>,
    pub vitals: Option<Framed<VitalsData>>,
    pub pack: Option<Framed<PackData>>,
    pub near: Option<Framed<NearData>>,
    /// The health bars of their own, and the target bar.
    pub bars: Vec<Framed<HealthBarData>>,
    pub journal: Option<Framed<JournalData>>,
    pub radar: Option<Framed<RadarData>>,
    /// The hotbar, while the human has control.
    pub hotbar: Option<Framed<HotbarData>>,
    /// The choices of the empty slot the player clicked.
    pub picker: Option<PickerData>,
    pub sheet: Option<Framed<SheetData>>,
    /// The open containers, each a grid.
    pub grids: Vec<Framed<GridData>>,
    /// The corpses near the character.
    pub loot: Option<Framed<LootData>>,
    /// The box that asks how many of a pile to move.
    pub split: Option<Framed<SplitData>>,
    /// The list of a shopkeeper.
    pub shop: Option<Framed<ShopData>>,
    pub trades: Vec<Framed<TradeData>>,
    pub old_menu: Option<Framed<OldMenuData>>,
    pub book: Option<Framed<BookData>>,
    pub board: Option<Framed<BoardData>>,
    pub paperdoll: Option<Framed<PaperdollData>>,
    /// The dialog for words the shard waits for.
    pub entry: Option<Framed<EntryData>>,
    pub race: Option<Framed<RaceData>>,
    /// The tip of the day or the notice of the shard.
    pub tip: Option<Framed<TipData>>,
    /// The dye panel, when the client files have no gump art.
    pub dye: Option<Framed<DyeData>>,
    pub ring: Option<RingData>,
    /// The words of the last act, while they show.
    pub report: Option<ReportData>,
    /// The question that waits for Yes or No.
    pub question: Option<QuestionData>,
    pub chat: ChatData,
    /// The tooltip of the thing under the mouse, on the map or on a panel.
    pub tooltip: Option<TooltipData>,
    /// What the player carries on the mouse.
    pub carried: Option<CarriedData>,
}

/// How the panels look, from the profile.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Look {
    /// The UI scale of the Video page: the panels grow by it.
    pub ui_scale: f32,
    /// The opacity of the glass of the panels, 0 to 1.
    pub opacity: f32,
}

/// The tips of the title and the marks of every frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct FrameHints {
    pub drag: &'static str,
    pub lock: &'static str,
    pub close: &'static str,
    pub size: &'static str,
    pub fold: &'static str,
}

const HINTS: FrameHints = FrameHints {
    drag: HINT_MOVE,
    lock: HINT_LOCK,
    close: HINT_CLOSE,
    size: HINT_SIZE,
    fold: HINT_FOLD,
};

/// A place on the page, in the points of the panel layer: its left, its
/// top, its width and its height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl From<Area> for Place {
    fn from(area: Area) -> Self {
        Self {
            x: area.min.x,
            y: area.min.y,
            w: area.width(),
            h: area.height(),
        }
    }
}

impl Place {
    pub fn area(self) -> Area {
        Area::from_min_size(Point::new(self.x, self.y), Vector::new(self.w, self.h))
    }
}

/// Words and their color, as CSS: a theme token or a hue of the shard.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Colored {
    pub words: String,
    pub color: String,
}

/// The frame of a panel the player moves.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FrameData {
    /// The name of the panel in its `Panel` events.
    pub panel: String,
    pub title: String,
    /// The color of the title, when it is not the plain one.
    pub title_color: Option<String>,
    /// Words beside the marks of the title, such as how many are near.
    pub aside: Option<Colored>,
    /// The color of the edge, when it is not the glass edge.
    pub edge: Option<String>,
    /// The whole panel, unfolded.
    pub area: Place,
    pub locked: bool,
    pub folded: bool,
    pub foldable: bool,
    pub closable: bool,
    /// The player may size it by its corner.
    pub sizable: bool,
}

/// A panel in its frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Framed<T> {
    pub frame: FrameData,
    pub body: T,
}

/// What a panel is: its id in the profile, its title, where it stands
/// before the player moves it, and how he may change its frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FrameSpec {
    pub id: String,
    pub title: String,
    pub default: Area,
    /// The least size of a panel the player sizes. None for a panel of a
    /// fixed size.
    pub min_size: Option<Vector>,
    pub closable: bool,
    pub foldable: bool,
}

impl FrameSpec {
    /// A panel of a fixed size that moves and locks only.
    pub fn fixed(id: &str, title: &str, default: Area) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            default,
            min_size: None,
            closable: false,
            foldable: false,
        }
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }

    /// Closable only while `live`: the human has control.
    pub fn closable_if(mut self, live: bool) -> Self {
        self.closable = live;
        self
    }

    pub fn foldable(mut self) -> Self {
        self.foldable = true;
        self
    }

    pub fn sized(mut self, least: Vector) -> Self {
        self.min_size = Some(least);
        self
    }
}

/// What the player did to the frame of a panel: `{"place": {x, y, w, h}}`
/// after a drag of its title or its corner, `{"lock": true}`,
/// `{"fold": true}`, `{"close": true}`, or `{"reset": true}` after a double
/// click on its title.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameAction {
    Place(Place),
    Lock(bool),
    Fold(bool),
    Close(bool),
    Reset(bool),
}

/// The panels the view keeps open and how each stands, apart from the
/// profile.
#[derive(Default)]
pub(crate) struct PanelState {
    pub bar: BarState,
    pub bars: BarsState,
    pub deck: DeckState,
    pub journal: JournalState,
    pub radar: RadarState,
    pub ring: RingState,
    pub sheet: SheetState,
    pub desk: desk::DeskState,
    pub deals: DealsState,
    pub grids: GridsState,
    pub pages: PagesState,
    pub doll: DollState,
    pub asks: AsksState,
    pub hud: hud::HudBars,
}

impl WebView {
    /// The room the panels stand in: the view, in the points of the panel
    /// layer, which the UI scale grows.
    pub(crate) fn panel_room(&self) -> Area {
        let scale = self.profile.video.ui_scale.max(f32::EPSILON);
        Area::from_min_size(Point::new(0.0, 0.0), self.view.size() / scale)
    }

    /// The data of every panel at `time`.
    pub fn panel_data(&mut self, time: f64) -> PanelData {
        let look = Look {
            ui_scale: self.profile.video.ui_scale,
            opacity: f32::from(self.profile.interface.gump_opacity) / PERCENT,
        };
        let chat = self.chat_data();
        let report = self.report_data(time);
        let question = self.question_data();
        let tooltip = self.panel_tooltip(time).or_else(|| self.tooltip.clone());
        let carried = self.carried_data();
        let Some(frame) = self.frame.clone().filter(|frame| frame.error.is_empty()) else {
            return PanelData {
                look,
                hints: HINTS,
                title: uoterm_view::model::info_bar::WINDOW_TITLE.to_string(),
                waiting: Some(self.waiting_data()),
                alarm: 0.0,
                bar: None,
                launcher: None,
                activity: None,
                vitals: None,
                pack: None,
                near: None,
                bars: Vec::new(),
                journal: None,
                radar: None,
                hotbar: None,
                picker: None,
                sheet: None,
                grids: Vec::new(),
                loot: None,
                split: None,
                shop: None,
                trades: Vec::new(),
                old_menu: None,
                book: None,
                board: None,
                paperdoll: None,
                entry: None,
                race: None,
                tip: None,
                dye: None,
                ring: None,
                report,
                question,
                chat,
                tooltip,
                carried,
            };
        };
        PanelData {
            look,
            hints: HINTS,
            title: uoterm_view::model::info_bar::window_title(&frame, &self.profile.interface),
            waiting: None,
            alarm: uoterm_view::ui::hud::alarm_share(frame.danger(), time),
            bar: Some(self.control_bar_data(&frame)),
            launcher: self.launcher_data(),
            activity: Some(self.activity_data(&frame)),
            vitals: Some(self.vitals_data(&frame)),
            pack: Some(self.pack_data(&frame)),
            near: Some(self.near_data(&frame)),
            bars: self.health_bars_data(&frame),
            journal: self.journal_data(&frame, time),
            radar: self.radar_data(&frame),
            hotbar: frame.human_control.then(|| self.hotbar_data(&frame)),
            picker: self.picker_data(&frame),
            sheet: self.sheet_data(&frame),
            grids: self.grids_data(&frame),
            loot: self.loot_data(&frame),
            split: self.split_data(),
            shop: self.shop_data(&frame),
            trades: self.trades_data(&frame),
            old_menu: self.menu_data(&frame),
            book: self.book_data(&frame),
            board: self.board_data(&frame),
            paperdoll: self.doll_data(&frame),
            entry: self.entry_data(&frame),
            race: self.race_data(&frame),
            tip: self.tip_data(&frame),
            dye: self.dye_data(&frame),
            ring: self.ring_data(&frame),
            report,
            question,
            chat,
            tooltip,
            carried,
        }
    }

    /// The frame of panel `panel` by its spec, around `body`.
    pub(crate) fn framed<T>(&self, panel: &str, spec: &FrameSpec, body: T) -> Framed<T> {
        Framed {
            frame: self.frame_data(panel, spec),
            body,
        }
    }

    /// The frame of panel `panel` as the profile keeps it.
    pub(crate) fn frame_data(&self, panel: &str, spec: &FrameSpec) -> FrameData {
        FrameData {
            panel: panel.to_string(),
            title: spec.title.clone(),
            title_color: None,
            aside: None,
            edge: None,
            area: Place::from(self.panel_area(spec)),
            locked: places::is_locked(&self.profile, &spec.id),
            folded: spec.foldable && places::is_folded(&self.profile, &spec.id),
            foldable: spec.foldable,
            closable: spec.closable,
            sizable: spec.min_size.is_some(),
        }
    }

    /// Where a panel stands now: where the player left it, or at its first
    /// place, inside the room of the panels.
    pub(crate) fn panel_area(&self, spec: &FrameSpec) -> Area {
        place(
            self.panel_room(),
            &spec.id,
            spec.default,
            spec.min_size,
            &self.profile,
        )
    }

    /// The spec of the frame of panel `panel` now, when it shows one.
    fn frame_spec(&self, panel: &str) -> Option<FrameSpec> {
        let frame = self.frame.as_ref()?;
        match panel {
            PANEL_LAUNCHER => Some(self.launcher_spec()),
            PANEL_ACTIVITY => Some(self.activity_spec(frame)),
            PANEL_VITALS => Some(self.vitals_spec(frame)),
            PANEL_PACK => Some(self.pack_spec(frame)),
            PANEL_HOTBAR => Some(self.hotbar_spec()),
            PANEL_NEAR => Some(self.near_spec(frame)),
            PANEL_JOURNAL => Some(self.journal_spec()),
            PANEL_RADAR => Some(self.radar_spec()),
            PANEL_SHEET => Some(self.sheet_spec(frame)),
            PANEL_SPLIT => self.split_spec(),
            PANEL_QUESTION => self.question_spec(),
            PANEL_SHOP => self.shop_spec(frame),
            PANEL_LOOT => self.loot_spec(frame),
            PANEL_OLD_MENU => self.menu_spec(frame),
            PANEL_BOOK => self.book_spec(frame),
            PANEL_BOARD => self.board_spec(frame),
            PANEL_PAPERDOLL => self.doll_spec(),
            PANEL_ENTRY => self.entry_spec(frame),
            PANEL_RACE => self.race_spec(frame),
            PANEL_TIP => self.tip_spec(frame),
            PANEL_DYE => self.dye_spec(frame),
            grid if grid.starts_with(PANEL_GRID_PREFIX) => self.grid_panel_spec(frame, grid),
            trade if trade.starts_with(PANEL_TRADE_PREFIX) => self.trade_spec(frame, trade),
            health => self.health_bar_spec(frame, health),
        }
    }

    /// Takes the action of a panel. An action of a panel this view does not
    /// know, or one that does not read, does nothing.
    pub(crate) fn panel_action(&mut self, panel: &str, action: Value) {
        if let Some(spec) = self.frame_spec(panel) {
            if let Ok(done) = serde_json::from_value::<FrameAction>(action.clone()) {
                self.frame_action(panel, &spec, done);
                return;
            }
        }
        match panel {
            PANEL_BAR => self.bar_action(action),
            PANEL_CHAT => self.chat_action(action),
            PANEL_QUESTION => self.question_action(action),
            PANEL_LAUNCHER => self.launcher_action(action),
            PANEL_HOTBAR => self.hotbar_action(action),
            PANEL_NEAR => self.near_action(action),
            PANEL_JOURNAL => self.journal_action(action),
            PANEL_RADAR => self.radar_action(action),
            PANEL_SHEET => self.sheet_action(action),
            PANEL_SPLIT => self.split_action(action),
            PANEL_RING => self.ring_action(action),
            PANEL_TIPS => self.tips_action(action),
            PANEL_DESK => self.desk_action(action),
            PANEL_SHOP => self.shop_action(action),
            PANEL_LOOT => self.loot_action(action),
            PANEL_OLD_MENU => self.menu_action(action),
            PANEL_BOOK => self.book_action(action),
            PANEL_BOARD => self.board_action(action),
            PANEL_PAPERDOLL => self.doll_action(action),
            PANEL_ENTRY => self.entry_action(action),
            PANEL_RACE => self.race_action(action),
            PANEL_TIP => self.tip_action(action),
            PANEL_DYE => self.dye_action(action),
            grid if grid.starts_with(PANEL_GRID_PREFIX) => self.grid_action(grid, action),
            trade if trade.starts_with(PANEL_TRADE_PREFIX) => self.trade_action(trade, action),
            health => self.health_bar_action(health, action),
        }
    }

    /// Does what the player did to the frame of a panel, by the rules of
    /// the places, and keeps the profile.
    fn frame_action(&mut self, panel: &str, spec: &FrameSpec, action: FrameAction) {
        let sized = spec.min_size.is_some();
        let locked = places::is_locked(&self.profile, &spec.id);
        let now = self.panel_area(spec);
        match action {
            FrameAction::Place(moved) if !locked => {
                let area = if sized {
                    moved.area()
                } else {
                    Area::from_min_size(moved.area().min, now.size())
                };
                places::remember(&mut self.profile, &spec.id, area, sized);
            }
            FrameAction::Lock(lock) => {
                places::set_locked(&mut self.profile, &spec.id, now, sized, lock);
            }
            FrameAction::Fold(folded) if spec.foldable => {
                places::set_folded(&mut self.profile, &spec.id, folded);
            }
            FrameAction::Reset(true) if !locked => places::forget(&mut self.profile, &spec.id),
            FrameAction::Close(true) if spec.closable => return self.close_panel(panel, spec),
            _ => return,
        }
        self.keep_profile();
    }

    /// Closes a panel the player closed by its mark.
    fn close_panel(&mut self, panel: &str, spec: &FrameSpec) {
        match panel {
            PANEL_LAUNCHER => self.panels.bar.launcher_open = false,
            PANEL_JOURNAL | PANEL_RADAR => {
                places::set_shut(&mut self.profile, &spec.id, true);
                self.keep_profile();
            }
            PANEL_SHEET => self.panels.sheet.open = false,
            PANEL_SPLIT => self.panels.desk.split = None,
            PANEL_QUESTION => self.answer_asked(false),
            PANEL_SHOP => self.close_deal(panel),
            PANEL_LOOT => self.close_loot(),
            PANEL_OLD_MENU => self.menu_action(serde_json::json!({ "cancel": true })),
            PANEL_BOOK => self.close_book(),
            PANEL_BOARD => self.close_board(),
            PANEL_PAPERDOLL => self.close_doll(),
            PANEL_ENTRY => self.close_entry(),
            PANEL_RACE => self.close_race(),
            PANEL_TIP => self.close_tip(),
            grid if grid.starts_with(PANEL_GRID_PREFIX) => self.close_grid(grid),
            trade if trade.starts_with(PANEL_TRADE_PREFIX) => self.close_deal(trade),
            health => self.close_health_bar(health),
        }
    }

    /// Keeps the profile where the page keeps it: with the places only
    /// when the Interface page keeps them, in the UI style of the file.
    pub(crate) fn keep_profile(&mut self) {
        let saving = places::for_saving(&self.profile);
        if let Ok(profile) = serde_json::to_value(kept(&saving, self.kept_style)) {
            self.hand.push(crate::out::OutCall::SaveProfile { profile });
        }
    }
}

/// A click on an item of a panel, as the panels of the Rust window take
/// it: a single click targets the item while the shard waits for a
/// target, and else asks its name once the double click time is over. A
/// `double` click gives true: its act is the panel's.
pub(crate) fn click_item(
    clicks: &mut ClickDelay,
    hand: &mut Hand,
    frame: &WatchFrame,
    serial: u32,
    double: bool,
) -> bool {
    let time = hand.time();
    let (doubled, act) = single_or_double((!double, double), clicks, frame, serial, time);
    if let Some(act) = act {
        hand.act(act);
    }
    doubled
}

/// The notches of a wheel over a panel's lines or map, from the notches
/// of the mouse wheel the page counts, as the Rust window counts its
/// points.
pub(crate) fn panel_notches(notches: f32) -> f32 {
    notches * WHEEL_POINTS_PER_NOTCH / PANEL_WHEEL_POINTS
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::out::OutCall;
    use crate::tests::{settled, VIEW};
    use serde_json::json;
    use uoterm_view::act::PageAct;
    use uoterm_view::settings::Profile;
    use uoterm_view::ui::hud::VITALS_ID;
    use uoterm_view::ui::launch::JOURNAL_ID;

    /// The acts among the calls for the page.
    pub fn out_acts(out: &[OutCall]) -> Vec<PageAct> {
        out.iter()
            .filter_map(|call| match call {
                OutCall::Act { act, .. } => Some(act.clone()),
                _ => None,
            })
            .collect()
    }

    /// The profiles among the calls for the page.
    pub fn saved_profiles(out: &[OutCall]) -> Vec<Profile> {
        out.iter()
            .filter_map(|call| match call {
                OutCall::SaveProfile { profile } => serde_json::from_value(profile.clone()).ok(),
                _ => None,
            })
            .collect()
    }

    /// A settled view whose picture holds `value` at `key`, with or
    /// without control, after a frame and with what it sent taken.
    pub fn view_with(key: &str, value: Value, control: bool) -> WebView {
        let mut watch: Value =
            serde_json::from_str(&crate::tests::fixture_watch_with_backpack()).unwrap();
        watch[key] = value;
        watch["human_control"] = json!(control);
        let mut view = settled();
        view.frame(&watch.to_string(), 0.1);
        view.tick_native(0.1, VIEW, None);
        view.take_out_native();
        view
    }

    /// A `Panel` event of `panel` with `action`, given to the view.
    pub fn press(view: &mut WebView, panel: &str, action: Value) -> Vec<OutCall> {
        view.input_native(
            &json!({"kind": "Panel", "panel": panel, "action": action}).to_string(),
            0.0,
        )
    }

    #[test]
    fn a_drag_of_the_title_keeps_the_new_place_as_the_window_does() {
        let mut view = settled();
        let vitals = view.panel_data(0.0).vitals.unwrap().frame.area;
        let moved = Place {
            x: vitals.x + 40.0,
            y: vitals.y - 30.0,
            ..vitals
        };
        let out = press(&mut view, PANEL_VITALS, json!({ "place": moved }));
        let saved = saved_profiles(&out);
        assert_eq!(saved.len(), 1, "the move is kept once");
        let kept = places::kept(&saved[0], VITALS_ID).unwrap();
        assert_eq!((kept.x, kept.y), (moved.x, moved.y));
        assert_eq!(kept.size, None, "the vitals have a size of their own");
        assert_eq!(view.panel_data(0.0).vitals.unwrap().frame.area, moved);
    }

    #[test]
    fn a_locked_panel_stays_and_a_double_click_puts_a_free_one_back() {
        let mut view = settled();
        let first = view.panel_data(0.0).vitals.unwrap().frame.area;
        press(&mut view, PANEL_VITALS, json!({ "lock": true }));
        assert!(view.panel_data(0.0).vitals.unwrap().frame.locked);
        let away = Place { x: 0.0, ..first };
        assert!(press(&mut view, PANEL_VITALS, json!({ "place": away })).is_empty());
        assert_eq!(view.panel_data(0.0).vitals.unwrap().frame.area, first);
        press(&mut view, PANEL_VITALS, json!({ "lock": false }));
        press(&mut view, PANEL_VITALS, json!({ "place": away }));
        press(&mut view, PANEL_VITALS, json!({ "reset": true }));
        assert_eq!(view.panel_data(0.0).vitals.unwrap().frame.area, first);
    }

    #[test]
    fn a_place_held_off_the_window_comes_back_inside_it() {
        let mut view = settled();
        let far = Place {
            x: VIEW.max.x * 4.0,
            y: VIEW.max.y * 4.0,
            w: 10.0,
            h: 10.0,
        };
        press(&mut view, PANEL_VITALS, json!({ "place": far }));
        let shown = view.panel_data(0.0).vitals.unwrap().frame.area.area();
        assert!(view
            .panel_room()
            .contains(shown.max - Vector::new(1.0, 1.0)));
    }

    #[test]
    fn the_journal_folds_and_shuts_by_its_marks() {
        let mut view = settled();
        press(&mut view, PANEL_JOURNAL, json!({ "fold": true }));
        assert!(view.panel_data(0.0).journal.unwrap().frame.folded);
        let out = press(&mut view, PANEL_JOURNAL, json!({ "close": true }));
        assert!(places::is_shut(&saved_profiles(&out)[0], JOURNAL_ID));
        assert!(view.panel_data(0.0).journal.is_none());
    }

    #[test]
    fn the_panels_grow_by_the_ui_scale() {
        let mut view = settled();
        let small = view.panel_room();
        let mut profile = Profile::default();
        profile.video.ui_scale = 2.0;
        view.set_profile(&serde_json::to_string(&profile).unwrap());
        assert_eq!(view.panel_room().width(), small.width() / 2.0);
        assert_eq!(view.panel_data(0.0).look.ui_scale, 2.0);
    }
}
