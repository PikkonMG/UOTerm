//! The deck of the player: the sheet with what the character wears, his
//! status, his skills, his spellbooks and his party; the panels of his
//! combat and racial abilities; the box of a party invite; and the hotbar
//! at the bottom of the window. The sheet is a panel the player moves,
//! sizes and locks, and it shows at all times it is open. Its clicks, and
//! the hotbar, work only while the human has control.

use super::actions::modern::wanted;
use super::actions::GumpOp;
use super::atlas::Sprite;
use super::boxes_ui::{scrolled, Tools, CELL, CELL_GAP, CELL_RADIUS};
use super::control::{Act, Answer, Ask, Asker};
use super::desk::Zone;
use super::figure::WORN_LAYERS;
use super::kept;
use super::model::abilities::{
    ability_of, icon_of, race_of, racial_command, slot_hue, toggle_command, AbilitySlot,
};
use super::model::clicks::ClickDelay;
use super::model::durability::{worn_wear, Wear};
use super::model::key_macros;
use super::model::places;
use super::model::spell_data::{book_spell, icon_hue};
use super::model::status::{StatLocks, STAT_NAMES};
use super::modern::abilities_ui::{self, ABILITIES_ID, RACIAL_ID};
use super::modern::frame::{self, FrameEvent, PanelSpec};
use super::modern::layout::{self, Spot};
use super::modern::party_ui::{self, PartyTab};
use super::modern::skills_ui::SkillsTab;
use super::modern::spells_ui::SpellsTab;
use super::modern::{status_ui, wear_color};
use super::ring_ui::Subject;
use super::settings::{MacroStep, Profile};
use super::theme::{self, number_font, text_font, title_font};
use crate::view::{WatchEquip, WatchFrame, WatchPackItem};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uoterm_assist::spells::School;
use uoterm_protocol::types::{LAYER_BACKPACK, LAYER_BEARD, LAYER_HAIR, LAYER_LEGS};

const SHEET_ID: &str = "modern:sheet";
/// The hotbar is a panel of its own the player moves and locks.
const HOTBAR_ID: &str = "modern:hotbar";
const SHEET_WIDTH: f32 = 430.0;
/// The rows of the sheet as it first opens, and the fewest it takes.
const SHEET_ROWS: usize = 14;
const SHEET_MIN_ROWS: usize = 10;
pub(super) const ROW: f32 = 24.0;
const TAB_HEIGHT: f32 = 28.0;
pub(super) const TAB_GAP: f32 = 6.0;
pub(super) const LOCK_SIDE: f32 = 16.0;
const LOCK_MARK: f32 = 5.0;
pub(super) const USE_WIDTH: f32 = 40.0;
const ROW_BUTTON_INSET: f32 = 2.0;
pub(super) const PIN_WIDTH: f32 = 36.0;

/// The last layer that is clothes or arms. Above it are the mount and the
/// boxes of a shopkeeper and the bank.
const LAYER_LAST_WORN: u8 = LAYER_LEGS;

const SKILL_LOCK_UP: u8 = 0;
const SKILL_LOCK_DOWN: u8 = 1;

pub const HOTBAR_SLOTS: usize = 10;
const HOTBAR_GAP: f32 = 10.0;
const HOTBAR_FILE: &str = "watch-hotbar.toml";
const HOTBAR_KEYS: [Key; HOTBAR_SLOTS] = [
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Num5,
    Key::Num6,
    Key::Num7,
    Key::Num8,
    Key::Num9,
    Key::Num0,
];
const HOTBAR_KEY_WORDS: [&str; HOTBAR_SLOTS] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"];
/// The dragged picture on its way to the hotbar.
const CARRY_SIDE: f32 = 40.0;
const CARRY_ALPHA: f32 = 0.85;
/// The picker of an empty slot: its columns and the height of a choice.
const PICKER_COLUMNS: usize = 2;
const PICKER_ROW: f32 = 28.0;

/// How many letters of a skill or a command a slot shows.
const SLOT_WORD_CHARS: usize = 6;

pub(super) const WORDS_USE: &str = "Use";
const WORDS_PIN: &str = "Pin";
const WORDS_SHEET: &str = "Character";
const WORDS_HOTBAR: &str = "Hotbar";
const WORDS_WORN_VIEW: &str = "Worn";
const WORDS_STATUS_VIEW: &str = "Status";
const WORDS_ABILITIES: &str = "Abilities";
const WORDS_RACIAL: &str = "Racial";
const WORDS_BAR_FULL: &str = "The hotbar is full. Right-click a slot to clear it.";
const WORDS_PICK_FOR: &str = "Put on slot";
const WORDS_NO_MACROS: &str = "No macros yet: make them on the Macros page of the Options.";
const WORDS_MACRO_GONE: &str = "That macro is no longer in the profile.";
const VIEW_WIDTH: f32 = 72.0;
const PANEL_BUTTON_WIDTH: f32 = 84.0;
/// The durability bar under a worn item.
const DURABILITY_BAR: f32 = 3.0;
const HINT_WORN: &str =
    "Click: name.  Double-click: use.  Drag or x: take off.  Right-click: more.";
const HINT_SLOT: &str = "Click or press the key: use.  Right-click: clear.";
const HINT_EMPTY_SLOT: &str = "Click: choose a macro or an ability for it.";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Character,
    Skills,
    Spells,
    Party,
}

const TABS: [(Tab, &str); 4] = [
    (Tab::Character, "Character"),
    (Tab::Skills, "Skills"),
    (Tab::Spells, "Spells"),
    (Tab::Party, "Party"),
];

/// What the character tab shows: the worn items, or the whole status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CharacterView {
    #[default]
    Worn,
    Status,
}

/// The ability panels a window command opens and closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilityPanel {
    Combat,
    Racial,
}

impl AbilityPanel {
    fn id(self) -> &'static str {
        match self {
            AbilityPanel::Combat => ABILITIES_ID,
            AbilityPanel::Racial => RACIAL_ID,
        }
    }
}

/// What one slot of the hotbar does.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Slot {
    Item {
        serial: u32,
        graphic: u16,
        hue: u16,
        name: String,
    },
    Skill {
        id: u16,
        name: String,
    },
    Spell {
        id: u16,
        name: String,
    },
    Command {
        text: String,
    },
    /// A macro of the profile, by its name.
    Macro {
        name: String,
    },
    /// The primary or the secondary ability of the weapon in hand.
    Ability {
        slot: AbilitySlot,
    },
    /// A racial ability, by its icon.
    Racial {
        icon: u16,
        name: String,
    },
}

/// What a slot does when the player presses it.
#[derive(Clone, Debug, PartialEq)]
enum Press {
    Act(Act),
    /// The steps of a macro of the profile, for the window to run.
    Macro(Vec<MacroStep>),
}

impl Slot {
    /// What the slot does now. None for a macro no longer in the profile
    /// and for a racial ability the character cannot use.
    fn press(&self, frame: &WatchFrame, profile: &Profile) -> Option<Press> {
        Some(match self {
            Self::Item { serial, .. } => Press::Act(Act::Use(*serial)),
            Self::Skill { id, .. } => Press::Act(Act::UseSkill(*id)),
            Self::Spell { id, .. } => Press::Act(Act::Cast(*id)),
            Self::Command { text } => Press::Act(Act::Command(text.clone())),
            Self::Macro { name } => {
                Press::Macro(key_macros::steps_of(&profile.macros.key_bindings, name)?)
            }
            Self::Ability { slot } => Press::Act(Act::Command(toggle_command(frame, *slot))),
            Self::Racial { icon, .. } => {
                Press::Act(Act::Command(racial_command(frame, *icon)?.to_string()))
            }
        })
    }

    fn words(&self, frame: &WatchFrame) -> String {
        match self {
            Self::Item { name, .. }
            | Self::Skill { name, .. }
            | Self::Spell { name, .. }
            | Self::Macro { name }
            | Self::Racial { name, .. } => name.clone(),
            Self::Command { text } => text.clone(),
            Self::Ability { slot } => abilities_ui::slot_words(frame, *slot),
        }
    }

    /// The picture of the slot: the item, or the icon of the spell or the
    /// ability, in the hue it has now. Words stand for the others.
    fn picture(
        &self,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Option<(egui::TextureId, Sprite)> {
        match self {
            Self::Item { graphic, hue, .. } => tools.scene.item_picture(frame.map, *graphic, *hue),
            Self::Spell { id, .. } => {
                let (_, spell) = book_spell(*id)?;
                tools
                    .scene
                    .gump_picture(spell.small_icon, icon_hue(frame, *id))
            }
            Self::Ability { slot } => tools
                .scene
                .gump_picture(icon_of(ability_of(frame, *slot)), slot_hue(frame, *slot)),
            Self::Racial { icon, .. } => tools.scene.gump_picture(*icon, 0),
            Self::Skill { .. } | Self::Command { .. } | Self::Macro { .. } => None,
        }
    }
}

/// What a row of the sheet or a panel gives the hotbar: a slot to put on
/// the first free place, or one the player drags onto a place.
pub enum Offer {
    Pin(Slot),
    Drag(Slot),
}

/// The hotbar of each character, by his name. The key of a slot is its
/// number as text, because a TOML list cannot hold an empty place.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct KeptHotbars {
    #[serde(default)]
    characters: BTreeMap<String, BTreeMap<String, Slot>>,
}

impl KeptHotbars {
    fn slot(&self, character: &str, slot: usize) -> Option<&Slot> {
        self.characters.get(character)?.get(&slot.to_string())
    }

    fn set(&mut self, character: &str, slot: usize, what: Option<Slot>) {
        let bar = self.characters.entry(character.to_string()).or_default();
        match what {
            Some(what) => bar.insert(slot.to_string(), what),
            None => bar.remove(&slot.to_string()),
        };
    }

    fn first_free(&self, character: &str) -> Option<usize> {
        (0..HOTBAR_SLOTS).find(|slot| self.slot(character, *slot).is_none())
    }
}

/// What an empty slot may take beside the rows of the sheet: each macro of
/// the profile, the two weapon abilities, and the racial abilities the
/// character uses.
fn slot_choices(frame: &WatchFrame, profile: &Profile) -> Vec<Slot> {
    let mut choices: Vec<Slot> = profile
        .macros
        .key_bindings
        .iter()
        .filter(|binding| !binding.name.trim().is_empty())
        .map(|binding| Slot::Macro {
            name: binding.name.clone(),
        })
        .collect();
    choices.extend(AbilitySlot::BOTH.map(|slot| Slot::Ability { slot }));
    if let Some(race) = race_of(frame) {
        for (index, name) in race.names.iter().enumerate() {
            let icon = race.icon(index);
            if racial_command(frame, icon).is_some() {
                choices.push(Slot::Racial {
                    icon,
                    name: (*name).to_string(),
                });
            }
        }
    }
    choices
}

pub struct DeckUi {
    open: bool,
    tab: Tab,
    view: CharacterView,
    /// The first row of the worn list that shows.
    first_worn: usize,
    /// The first line of the status view that shows.
    first_status: usize,
    /// The first row of the list of every weapon ability that shows.
    first_ability: usize,
    /// The words the human typed about what to wear, and the things Jev
    /// was asked about, in the order it was asked.
    wear_wish: String,
    wear_asked: Vec<WearChoice>,
    wear_note: Option<(String, bool, f64)>,
    hotbars: KeptHotbars,
    stat_locks: StatLocks,
    /// A click on a worn item asks its name once no double click follows.
    worn_clicks: ClickDelay,
    skills: SkillsTab,
    spells: SpellsTab,
    party: PartyTab,
    /// The slot the player drags toward the hotbar.
    dragging: Option<Slot>,
    /// The empty slot whose picker is open.
    picking: Option<usize>,
    /// The ability panels window commands opened or closed, until the
    /// next draw.
    panel_ops: Vec<(AbilityPanel, GumpOp)>,
    /// The macros of the profile the hotbar ran, for the window to run.
    macros: Vec<Vec<MacroStep>>,
}

impl DeckUi {
    pub fn starting(open: bool) -> Self {
        Self {
            open,
            tab: Tab::default(),
            view: CharacterView::default(),
            first_worn: 0,
            first_status: 0,
            first_ability: 0,
            wear_wish: String::new(),
            wear_asked: Vec::new(),
            wear_note: None,
            hotbars: kept::load(HOTBAR_FILE),
            stat_locks: StatLocks::default(),
            worn_clicks: ClickDelay::default(),
            skills: SkillsTab::default(),
            spells: SpellsTab::default(),
            party: PartyTab::default(),
            dragging: None,
            picking: None,
            panel_ops: Vec::new(),
            macros: Vec::new(),
        }
    }
}

pub(super) fn is_worn_layer(layer: u8) -> bool {
    layer != LAYER_BACKPACK
        && layer != LAYER_HAIR
        && layer != LAYER_BEARD
        && (1..=LAYER_LAST_WORN).contains(&layer)
}

/// What the human pressed at the right end of a row.
pub(super) enum RowPress {
    Go,
    Pin,
}

/// The two small buttons at the right end of a row: the act, and the pin
/// that puts the act on the hotbar.
pub(super) fn row_buttons(
    ui: &egui::Ui,
    row: Rect,
    key: (&'static str, u16),
    go_words: &str,
) -> Option<RowPress> {
    let mut pressed = None;
    let mut right = row.right();
    for (words, width, press) in [
        (WORDS_PIN, PIN_WIDTH, RowPress::Pin),
        (go_words, USE_WIDTH, RowPress::Go),
    ] {
        let area = Rect::from_min_max(
            Pos2::new(right - width, row.top() + ROW_BUTTON_INSET),
            Pos2::new(right, row.bottom() - ROW_BUTTON_INSET),
        );
        right -= width + TAB_GAP;
        let response = ui.interact(
            area,
            Id::new(("row-button", key, width as u32)),
            Sense::click(),
        );
        let fill = if response.hovered() {
            theme::BUTTON_HOVER
        } else {
            theme::BUTTON
        };
        ui.painter()
            .rect_filled(area, CornerRadius::same(CELL_RADIUS), fill);
        ui.painter().text(
            area.center(),
            Align2::CENTER_CENTER,
            words,
            text_font(theme::SIZE_SMALL),
            theme::TEXT,
        );
        if response.clicked() {
            pressed = Some(press);
        }
    }
    pressed
}

/// An arrow up, an arrow down, or a block for a locked skill or stat.
pub(super) fn lock_mark(painter: &egui::Painter, area: Rect, lock: u8, color: Color32) {
    let center = area.center();
    let point = |x: f32, y: f32| center + Vec2::new(x, y) * LOCK_MARK;
    let shape = match lock {
        SKILL_LOCK_UP => vec![point(0.0, -1.0), point(1.0, 0.8), point(-1.0, 0.8)],
        SKILL_LOCK_DOWN => vec![point(0.0, 1.0), point(-1.0, -0.8), point(1.0, -0.8)],
        _ => vec![
            point(-0.8, -0.8),
            point(0.8, -0.8),
            point(0.8, 0.8),
            point(-0.8, 0.8),
        ],
    };
    painter.add(egui::Shape::convex_polygon(
        shape,
        color,
        egui::Stroke::NONE,
    ));
}

/// The height of the sheet with room for `rows` rows.
fn sheet_height(rows: usize) -> f32 {
    frame::TITLE_ROW + theme::PANEL_PAD * 2.0 + TAB_HEIGHT + TAB_GAP + rows as f32 * ROW
}

/// Where the sheet first opens: the middle of the window.
fn sheet_first_place(window: Rect) -> Rect {
    layout::first_place(
        window,
        Spot::Middle(0),
        Vec2::new(SHEET_WIDTH, sheet_height(SHEET_ROWS)),
    )
}

/// The side of a hotbar cell, and the width of the row of cells, for a
/// bar as wide as a pack panel of `pack_width`, so it never lies on the
/// panels at the sides of the pack.
fn hotbar_cells(pack_width: f32) -> (f32, f32) {
    let room = pack_width - theme::PANEL_PAD * 2.0;
    let side = ((room + CELL_GAP) / HOTBAR_SLOTS as f32 - CELL_GAP).min(CELL);
    (side, HOTBAR_SLOTS as f32 * (side + CELL_GAP) - CELL_GAP)
}

/// The size of the hotbar panel over a pack panel of `pack_width`.
pub(super) fn hotbar_size(pack_width: f32) -> Vec2 {
    let (side, width) = hotbar_cells(pack_width);
    frame::with_title_room(Vec2::new(
        width + theme::PANEL_PAD * 2.0,
        frame::TITLE_ROW + side + theme::PANEL_PAD * 2.0,
    ))
}

impl DeckUi {
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the deck on a tab.
    pub fn show(&mut self, tab: Tab) {
        self.open = true;
        self.tab = tab;
    }

    /// The deck is open on this tab.
    pub fn shows(&self, tab: Tab) -> bool {
        self.open && self.tab == tab
    }

    /// Opens the character tab on a view.
    pub fn show_view(&mut self, view: CharacterView) {
        self.show(Tab::Character);
        self.view = view;
    }

    /// The deck is open on the character tab with this view.
    pub fn shows_view(&self, view: CharacterView) -> bool {
        self.shows(Tab::Character) && self.view == view
    }

    /// The deck is open on a spellbook of this school.
    pub fn shows_school(&self, frame: &WatchFrame, school: School) -> bool {
        self.shows(Tab::Spells) && self.spells.school(frame) == Some(school)
    }

    /// Turns the spells tab to a book of this school. False when the
    /// character has none yet; the tab turns to one when it comes.
    pub fn choose_school(&mut self, frame: &WatchFrame, school: School) -> bool {
        self.spells.choose_school(frame, school)
    }

    /// Opens or closes an ability panel at the next draw.
    pub fn switch_panel(&mut self, panel: AbilityPanel, op: GumpOp) {
        self.panel_ops.push((panel, op));
    }

    /// The macros of the profile the hotbar ran since the last call.
    pub fn take_macros(&mut self) -> Vec<Vec<MacroStep>> {
        std::mem::take(&mut self.macros)
    }

    /// Puts a script line on the first free slot. False when the bar is full.
    pub fn pin_command(&mut self, character: &str, text: &str) -> bool {
        self.pin(
            character,
            Slot::Command {
                text: text.to_string(),
            },
        )
    }

    fn pin(&mut self, character: &str, what: Slot) -> bool {
        let Some(free) = self.hotbars.first_free(character) else {
            return false;
        };
        self.set_slot(character, free, Some(what));
        true
    }

    fn set_slot(&mut self, character: &str, slot: usize, what: Option<Slot>) {
        self.hotbars.set(character, slot, what);
        kept::save(HOTBAR_FILE, &self.hotbars);
    }

    /// Takes what a row gave the hotbar.
    fn take_offer(&mut self, offer: Offer, frame: &WatchFrame, tools: &Tools<'_>) {
        match offer {
            Offer::Pin(slot) => {
                if !self.pin(&frame.name, slot) {
                    tools.hand.report(WORDS_BAR_FULL);
                }
            }
            Offer::Drag(slot) => self.dragging = Some(slot),
        }
    }

    /// Opens and closes the ability panels as window commands asked.
    fn apply_panel_ops(&mut self, profile: &mut Profile, tools: &Tools<'_>) {
        for (panel, op) in std::mem::take(&mut self.panel_ops) {
            let open = places::is_open(profile, panel.id());
            let want = wanted(op, open);
            if want != open {
                places::set_open(profile, panel.id(), want);
                tools.keep_profile(profile);
            }
        }
    }

    /// Draws the sheet, the ability panels, the box of a party invite and
    /// the hotbar. Gives the places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        pack_panel: Rect,
    ) -> Vec<Rect> {
        self.apply_panel_ops(profile, tools);
        let mut covered = Vec::new();
        let mut offers = Vec::new();
        if self.open {
            let (panel, offer) = self.sheet(ui, rect, frame, tools, profile);
            covered.push(panel);
            offers.extend(offer);
        }
        if places::is_open(profile, ABILITIES_ID) {
            let (panel, closed, offer) = abilities_ui::abilities_panel(
                ui,
                rect,
                frame,
                tools,
                profile,
                &mut self.first_ability,
            );
            covered.push(panel);
            offers.extend(offer);
            if closed {
                places::set_open(profile, ABILITIES_ID, false);
                tools.keep_profile(profile);
            }
        }
        if places::is_open(profile, RACIAL_ID) {
            let (panel, closed, offer) =
                abilities_ui::racial_panel(ui, rect, frame, tools, profile);
            covered.push(panel);
            offers.extend(offer);
            if closed {
                places::set_open(profile, RACIAL_ID, false);
                tools.keep_profile(profile);
            }
        }
        if !self.shows(Tab::Party) {
            covered.extend(party_ui::invite_panel(ui, rect, frame, tools, profile));
        }
        for offer in offers {
            self.take_offer(offer, frame, tools);
        }
        if frame.human_control {
            let bar = self.hotbar(ui, rect, frame, tools, profile, pack_panel);
            covered.push(bar);
            covered.extend(self.picker(ui, bar, frame, profile));
        } else {
            self.dragging = None;
            self.picking = None;
        }
        covered
    }

    fn sheet(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> (Rect, Option<Offer>) {
        let spec = PanelSpec {
            id: SHEET_ID,
            title: WORDS_SHEET,
            default: sheet_first_place(rect),
            min_size: Some(Vec2::new(SHEET_WIDTH, sheet_height(SHEET_MIN_ROWS))),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let title = if frame.name.is_empty() {
            WORDS_SHEET
        } else {
            frame.name.as_str()
        };
        let inner = frame::draw(ui.painter(), panel, title);
        let tab_width = (inner.width() - TAB_GAP * (TABS.len() - 1) as f32) / TABS.len() as f32;
        for (i, (tab, words)) in TABS.into_iter().enumerate() {
            let area = Rect::from_min_size(
                inner.left_top() + Vec2::new(i as f32 * (tab_width + TAB_GAP), 0.0),
                Vec2::new(tab_width, TAB_HEIGHT),
            );
            let color = if tab == self.tab {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            if theme::segment(ui, area, words, color) {
                self.tab = tab;
            }
        }
        let body = Rect::from_min_max(
            inner.left_top() + Vec2::new(0.0, TAB_HEIGHT + TAB_GAP),
            inner.right_bottom(),
        );
        let offer = match self.tab {
            Tab::Character => {
                self.character_tab(ui, body, frame, tools, profile);
                None
            }
            Tab::Skills => self.skills.draw(ui, body, frame, tools, profile),
            Tab::Spells => self.spells.draw(ui, body, frame, tools, profile),
            Tab::Party => {
                self.party.draw(ui, body, frame, tools, &profile.speech);
                None
            }
        };
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            self.open = false;
        }
        (panel, offer)
    }

    /// The character tab: the worn view or the status view, and the
    /// buttons of the ability panels.
    fn character_tab(
        &mut self,
        ui: &mut egui::Ui,
        body: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let top = Rect::from_min_size(body.min, Vec2::new(body.width(), ROW - TAB_GAP));
        for (at, (view, words)) in [
            (CharacterView::Worn, WORDS_WORN_VIEW),
            (CharacterView::Status, WORDS_STATUS_VIEW),
        ]
        .into_iter()
        .enumerate()
        {
            let area = Rect::from_min_size(
                top.left_top() + Vec2::new(at as f32 * (VIEW_WIDTH + TAB_GAP), 0.0),
                Vec2::new(VIEW_WIDTH, top.height()),
            );
            let color = if view == self.view {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            if theme::segment_keyed(ui, area, Id::new(("sheet-view", at)), words, color) {
                self.view = view;
            }
        }
        for (at, (panel, words)) in [
            (AbilityPanel::Racial, WORDS_RACIAL),
            (AbilityPanel::Combat, WORDS_ABILITIES),
        ]
        .into_iter()
        .enumerate()
        {
            let area = Rect::from_min_size(
                Pos2::new(
                    top.right() - (at + 1) as f32 * (PANEL_BUTTON_WIDTH + TAB_GAP) + TAB_GAP,
                    top.top(),
                ),
                Vec2::new(PANEL_BUTTON_WIDTH, top.height()),
            );
            let open = places::is_open(profile, panel.id());
            let color = if open { theme::GOAL } else { theme::TEXT_DIM };
            if theme::segment_keyed(ui, area, Id::new(("sheet-panel", at)), words, color) {
                places::set_open(profile, panel.id(), !open);
                tools.keep_profile(profile);
            }
        }
        let rest = Rect::from_min_max(Pos2::new(body.left(), body.top() + ROW), body.max);
        match self.view {
            CharacterView::Worn => self.worn_view(ui, rest, frame, tools, profile),
            CharacterView::Status => status_ui::draw(
                ui,
                rest,
                frame,
                tools,
                &mut self.stat_locks,
                &mut self.first_status,
            ),
        }
    }
}

const DOLL_WIDTH: f32 = 96.0;
const DOLL_HEIGHT: f32 = 130.0;
const SLOT_ROW: f32 = 22.0;
const TAKE_OFF_WIDTH: f32 = 26.0;
/// The worn list stands in two columns, so a full suit fits.
const WORN_COLUMNS: usize = 2;
const WORDS_MORE_WORN: &str = "Wheel: more";
const WEAR_WIDTH: f32 = 54.0;
const WORDS_WORN: &str = "Worn";
const WORDS_TAKE_OFF: &str = "x";
const WORDS_WEAR: &str = "Wear";
const WORDS_WEIGHT: &str = "Weight";
const WORDS_GOLD: &str = "Gold";
const HINT_WEAR: &str = "Say what to wear or take off, for example: my viking sword";
const HINT_WEAR_OFF: &str = "Plain words need a TypeSafe key. Set TYPESAFE_API_KEY.";
const WORDS_LOOKING: &str = "Jev looks in your bag...";
const WORDS_NOTHING_WORN: &str = "Nothing worn. Drag an item onto the figure.";

/// One thing Jev may pick: an item of the bag to wear, or a worn item to
/// take off.
#[derive(Clone, Debug, PartialEq, Eq)]
enum WearChoice {
    Wear(u32),
    TakeOff(u8),
}

/// The words for a layer, or its number when it has no name here.
pub(super) fn layer_words(layer: u8) -> String {
    WORN_LAYERS
        .iter()
        .find(|(known, _)| *known == layer)
        .map_or_else(
            || format!("layer {layer}"),
            |(_, words)| words.to_lowercase(),
        )
}

/// What Jev reads about one thing the character could wear or take off.
fn wear_words(name: &str, where_it_is: &str) -> String {
    format!("{name} ({where_it_is})")
}

/// How many rows of the worn list fit in `height`, and the last first row
/// the wheel may scroll to, for `count` worn items in their columns.
fn worn_rows(height: f32, count: usize) -> (usize, usize) {
    let rows = ((height / SLOT_ROW).floor() as usize).max(1);
    let needed = count.div_ceil(WORN_COLUMNS);
    (rows, needed.saturating_sub(rows))
}

impl DeckUi {
    /// The worn view: the figure, the stats with their locks, the weight
    /// and the gold, the worn list and the field of what to wear.
    fn worn_view(
        &mut self,
        ui: &mut egui::Ui,
        body: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &Profile,
    ) {
        let wears = if profile.interface.durability_bars {
            worn_wear(frame, tools.readings)
        } else {
            Vec::new()
        };
        let warning = profile.interface.durability_warning;
        // An item dropped anywhere on this view is put on.
        tools.desk.zone(body, Zone::Wear);
        let painter = ui.painter();
        let doll = Rect::from_min_size(body.left_top(), Vec2::new(DOLL_WIDTH, DOLL_HEIGHT));
        painter.rect_filled(doll, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        match tools.scene.doll_picture(frame.map, &frame.look) {
            Some((texture, sprite)) => {
                let area = theme::fit(doll, sprite.width, sprite.height);
                painter.image(texture, area, sprite.uv, Color32::WHITE);
            }
            None => {
                painter.text(
                    doll.center(),
                    Align2::CENTER_CENTER,
                    &frame.name,
                    title_font(theme::SIZE_PLATE),
                    theme::TEXT_DIM,
                );
            }
        }
        let facts_left = doll.right() + theme::ROW_GAP * 2.0;
        let fact_row = |at: usize| {
            Rect::from_min_max(
                Pos2::new(facts_left, body.top() + at as f32 * ROW),
                Pos2::new(body.right(), body.top() + (at + 1) as f32 * ROW),
            )
        };
        for stat in 0..STAT_NAMES.len() {
            status_ui::stat_row(
                ui,
                fact_row(stat),
                stat,
                frame,
                &mut self.stat_locks,
                tools.hand,
            );
        }
        for (at, (words, value)) in [
            (WORDS_WEIGHT, frame.carried()),
            (WORDS_GOLD, frame.gold.to_string()),
        ]
        .into_iter()
        .enumerate()
        {
            let row = fact_row(STAT_NAMES.len() + at);
            let words_left = row.left() + LOCK_SIDE + theme::ROW_GAP;
            ui.painter().text(
                Pos2::new(words_left, row.center().y),
                Align2::LEFT_CENTER,
                words,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_DIM,
            );
            ui.painter().text(
                row.right_center(),
                Align2::RIGHT_CENTER,
                value,
                number_font(theme::SIZE_SMALL),
                theme::TEXT,
            );
        }
        let worn: Vec<&WatchEquip> = frame
            .look
            .equipment
            .iter()
            .filter(|item| is_worn_layer(item.layer))
            .collect();
        let label_top = doll.bottom() + theme::ROW_GAP;
        let painter = ui.painter();
        painter.text(
            Pos2::new(body.left(), label_top),
            Align2::LEFT_TOP,
            format!("{WORDS_WORN} ({})", worn.len()),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let field = Rect::from_min_max(
            Pos2::new(body.left(), body.bottom() - ROW),
            body.right_bottom(),
        );
        // The list fills the room between its label and the field, and no more.
        let list = Rect::from_min_max(
            Pos2::new(body.left(), label_top + SLOT_ROW),
            Pos2::new(body.right(), field.top() - theme::ROW_GAP),
        );
        let (rows, last_first) = worn_rows(list.height(), worn.len());
        self.first_worn = scrolled(ui, list, self.first_worn.min(last_first), last_first);
        if last_first > 0 {
            painter.text(
                Pos2::new(body.right(), label_top),
                Align2::RIGHT_TOP,
                WORDS_MORE_WORN,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
        }
        if worn.is_empty() {
            painter.text(
                list.left_top(),
                Align2::LEFT_TOP,
                WORDS_NOTHING_WORN,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
        }
        let column_width = (list.width() - theme::ROW_GAP) / WORN_COLUMNS as f32;
        let shown = worn
            .iter()
            .skip(self.first_worn * WORN_COLUMNS)
            .take(rows * WORN_COLUMNS);
        for (i, item) in shown.enumerate() {
            let (row, column) = (i / WORN_COLUMNS, i % WORN_COLUMNS);
            let slot = Rect::from_min_size(
                list.left_top()
                    + Vec2::new(
                        column as f32 * (column_width + theme::ROW_GAP),
                        row as f32 * SLOT_ROW,
                    ),
                Vec2::new(column_width, SLOT_ROW - 2.0),
            );
            let wear = wears.iter().find(|wear| wear.serial == item.serial);
            self.worn_row(
                ui,
                slot,
                item,
                frame,
                tools,
                wear.map(|wear| (wear, warning)),
            );
        }
        if let Some(act) = self.worn_clicks.due_look(tools.time) {
            tools.hand.act(act);
        }
        if self.worn_clicks.is_waiting() {
            ui.ctx().request_repaint();
        }
        self.wear_field(ui, field, frame, tools);
    }

    /// One worn item: its picture, the words for its layer, a cross that
    /// takes it off, and its durability under the warning of the Interface
    /// page when it has one. As on the classic paperdoll, a click names it,
    /// or targets it while the shard waits for a target, a double click
    /// uses it, and a drag takes it off.
    fn worn_row(
        &mut self,
        ui: &egui::Ui,
        row: Rect,
        item: &WatchEquip,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        wear: Option<(&Wear, u8)>,
    ) {
        let art = Rect::from_min_size(row.min, Vec2::splat(row.height()));
        if let Some((wear, warning)) = wear {
            let track = Rect::from_min_max(
                Pos2::new(art.right() + theme::ROW_GAP, row.bottom() - DURABILITY_BAR),
                Pos2::new(row.right() - TAKE_OFF_WIDTH - theme::ROW_GAP, row.bottom()),
            );
            theme::bar(ui.painter(), track, wear.share(), wear_color(wear, warning));
        }
        if let Some((texture, sprite)) = tools.scene.item_picture(frame.map, item.graphic, item.hue)
        {
            let area = theme::fit(art, sprite.width, sprite.height);
            ui.painter().image(texture, area, sprite.uv, Color32::WHITE);
        }
        let response = ui.interact(
            row,
            Id::new(("worn-row", item.serial, item.layer)),
            Sense::click_and_drag(),
        );
        ui.painter().text(
            Pos2::new(art.right() + theme::ROW_GAP, row.center().y),
            Align2::LEFT_CENTER,
            layer_words(item.layer),
            text_font(theme::SIZE_SMALL),
            if response.hovered() {
                theme::TEXT
            } else {
                theme::TEXT_DIM
            },
        );
        if response.hovered() && !tools.desk.carries() && !tools.ring.is_open() {
            let footer = if frame.human_control { HINT_WORN } else { "" };
            tools
                .tips
                .point_at(ui, tools.hand, item.serial, "", footer, tools.time);
        }
        if !frame.human_control {
            return;
        }
        let cross = Rect::from_min_size(
            Pos2::new(row.right() - TAKE_OFF_WIDTH, row.top()),
            Vec2::new(TAKE_OFF_WIDTH, row.height()),
        );
        if theme::segment_keyed(
            ui,
            cross,
            Id::new(("take-off", item.layer)),
            WORDS_TAKE_OFF,
            theme::ALARM,
        ) {
            tools.hand.act(Act::TakeOff(item.layer));
        } else if response.drag_started_by(egui::PointerButton::Primary) {
            tools.desk.pick_up(&WatchPackItem {
                serial: item.serial,
                graphic: item.graphic,
                hue: item.hue,
                amount: 1,
                ..WatchPackItem::default()
            });
        } else if response.double_clicked() {
            self.worn_clicks.double_clicked();
            tools.hand.act(Act::Use(item.serial));
        } else if response.clicked() {
            if let Some(act) = self
                .worn_clicks
                .single_click(frame, item.serial, tools.time)
            {
                tools.hand.act(act);
            }
        } else if response.secondary_clicked() {
            tools
                .ring
                .open_at(row.center(), item.serial, "", Subject::Packed, tools.hand);
        }
    }

    /// The things the words could mean: each item of the backpack to put
    /// on, and each worn item to take off.
    fn wear_choices(&self, frame: &WatchFrame) -> (Vec<WearChoice>, Vec<String>) {
        let mut choices = Vec::new();
        let mut words = Vec::new();
        let bag = frame.backpack();
        let in_bag = frame
            .containers
            .iter()
            .filter(|container| Some(container.serial) == bag)
            .flat_map(|container| container.items.iter());
        for item in in_bag {
            choices.push(WearChoice::Wear(item.serial));
            words.push(wear_words(&item.name, "in your bag"));
        }
        for item in frame
            .look
            .equipment
            .iter()
            .filter(|item| is_worn_layer(item.layer))
        {
            choices.push(WearChoice::TakeOff(item.layer));
            words.push(wear_words(&layer_words(item.layer), "worn now"));
        }
        (choices, words)
    }

    /// Takes the answer of Jev about what to wear or take off.
    fn take_wear_answers(&mut self, tools: &Tools<'_>) {
        for answer in tools.hand.new_answers(Asker::Deck) {
            match answer {
                Answer::Picked(Ok(place)) => {
                    match self.wear_asked.get(place) {
                        Some(WearChoice::Wear(serial)) => tools.hand.act(Act::Wear(*serial)),
                        Some(WearChoice::TakeOff(layer)) => tools.hand.act(Act::TakeOff(*layer)),
                        None => continue,
                    }
                    self.wear_wish.clear();
                    self.wear_note = None;
                }
                Answer::Picked(Err(words)) => {
                    self.wear_note = Some((words, true, tools.time));
                }
                // The deck asks only for a pick.
                _ => {}
            }
        }
    }

    /// The field that takes what to wear or take off in plain words.
    fn wear_field(
        &mut self,
        ui: &mut egui::Ui,
        row: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) {
        self.take_wear_answers(tools);
        if !frame.human_control {
            return;
        }
        let (choices, words) = self.wear_choices(frame);
        let on = tools.hand.orders_on && !choices.is_empty();
        let field = Rect::from_min_max(
            row.min,
            Pos2::new(row.right() - WEAR_WIDTH - theme::ROW_GAP, row.bottom()),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let hint = if tools.hand.orders_on {
            HINT_WEAR
        } else {
            HINT_WEAR_OFF
        };
        let typed = ui.put(
            field,
            egui::TextEdit::singleline(&mut self.wear_wish)
                .frame(false)
                .margin(egui::Margin::symmetric(8, 4))
                .hint_text(hint)
                .font(text_font(theme::SIZE_SMALL))
                .text_color(theme::TEXT),
        );
        let (_, pressed) = theme::button(
            ui,
            Pos2::new(field.right() + theme::ROW_GAP, row.top()),
            WORDS_WEAR,
            if on { theme::GOAL } else { theme::TEXT_FAINT },
        );
        let asked = pressed || (typed.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
        if asked && on && !self.wear_wish.trim().is_empty() {
            tools.hand.ask(
                Asker::Deck,
                Ask::WearItem {
                    wish: self.wear_wish.trim().to_string(),
                    options: words,
                },
            );
            self.wear_asked = choices;
            self.wear_note = Some((WORDS_LOOKING.into(), false, tools.time));
        }
        if let Some((note, failed, since)) = &self.wear_note {
            if tools.time - since > NOTE_SECONDS {
                self.wear_note = None;
            } else {
                let color = if *failed {
                    theme::ALARM
                } else {
                    theme::WAITING
                };
                ui.painter().text(
                    Pos2::new(row.left(), row.top() - theme::ROW_GAP),
                    Align2::LEFT_BOTTOM,
                    note,
                    text_font(theme::SIZE_SMALL),
                    color,
                );
            }
        }
    }
}

/// How long the words about what Jev did stay on the sheet.
const NOTE_SECONDS: f64 = 6.0;

impl DeckUi {
    /// The hotbar panel: its ten slots in a row, as wide as the pack
    /// panel. It first stands where the plan puts it, over the pack; the
    /// player moves and locks it, and the profile keeps its place. Gives
    /// its place.
    fn hotbar(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        pack_panel: Rect,
    ) -> Rect {
        if let Some((slot, item)) = tools.desk.slotted.take() {
            let what = Slot::Item {
                serial: item.serial,
                graphic: item.graphic,
                hue: item.hue,
                name: item.name,
            };
            self.set_slot(&frame.name, slot, Some(what));
        }
        let (side, width) = hotbar_cells(pack_panel.width());
        let spec = PanelSpec {
            id: HOTBAR_ID,
            title: WORDS_HOTBAR,
            default: layout::first_place(rect, Spot::Hotbar, hotbar_size(pack_panel.width())),
            min_size: None,
            closable: false,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_HOTBAR);
        let bar = Rect::from_min_size(body.left_top(), Vec2::new(width, side));
        let typing = ui.ctx().wants_keyboard_input();
        let mut cells = Vec::with_capacity(HOTBAR_SLOTS);
        for slot in 0..HOTBAR_SLOTS {
            let cell = Rect::from_min_size(
                bar.left_top() + Vec2::new(slot as f32 * (side + CELL_GAP), 0.0),
                Vec2::splat(side),
            );
            cells.push(cell);
            tools.desk.zone(cell, Zone::Slot(slot));
            let response = ui.interact(cell, Id::new(("hotbar", slot)), Sense::click());
            let fill = if response.hovered() || self.picking == Some(slot) {
                theme::BUTTON_HOVER
            } else {
                theme::GLASS
            };
            ui.painter()
                .rect_filled(cell, CornerRadius::same(CELL_RADIUS), fill);
            ui.painter().text(
                cell.left_top() + Vec2::splat(theme::CELL_ART_PAD),
                Align2::LEFT_TOP,
                HOTBAR_KEY_WORDS[slot],
                number_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            let Some(what) = self.hotbars.slot(&frame.name, slot).cloned() else {
                if response.hovered() && !tools.desk.carries() && self.dragging.is_none() {
                    super::tips::label(ui, HOTBAR_KEY_WORDS[slot], HINT_EMPTY_SLOT);
                }
                if response.clicked() {
                    self.picking = if self.picking == Some(slot) {
                        None
                    } else {
                        Some(slot)
                    };
                }
                continue;
            };
            slot_face(ui, cell, &what, frame, tools);
            if response.hovered() && !tools.desk.carries() && self.dragging.is_none() {
                match &what {
                    Slot::Item { serial, name, .. } => {
                        tools
                            .tips
                            .point_at(ui, tools.hand, *serial, name, HINT_SLOT, tools.time);
                    }
                    other => super::tips::label(ui, &other.words(frame), HINT_SLOT),
                }
            }
            let key = !typing && ui.input(|i| i.key_pressed(HOTBAR_KEYS[slot]));
            if response.clicked() || key {
                self.press(&what, frame, tools, profile);
            } else if response.secondary_clicked() {
                self.set_slot(&frame.name, slot, None);
            }
        }
        self.follow_drag(ui, frame, tools, &cells);
        frame::controls(ui, panel, &spec, profile, tools);
        panel
    }

    /// Does what a slot does.
    fn press(&mut self, what: &Slot, frame: &WatchFrame, tools: &Tools<'_>, profile: &Profile) {
        match what.press(frame, profile) {
            Some(Press::Act(act)) => tools.hand.act(act),
            Some(Press::Macro(steps)) => self.macros.push(steps),
            None if matches!(what, Slot::Macro { .. }) => tools.hand.report(WORDS_MACRO_GONE),
            None => {}
        }
    }

    /// Draws the slot the player drags at the mouse, and puts it on the
    /// slot it is let go over.
    fn follow_drag(
        &mut self,
        ui: &egui::Ui,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        cells: &[Rect],
    ) {
        let Some(what) = self.dragging.clone() else {
            return;
        };
        let (mouse, down) = ui.input(|i| (i.pointer.hover_pos(), i.pointer.primary_down()));
        let Some(mouse) = mouse else {
            self.dragging = None;
            return;
        };
        if down {
            let painter = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                Id::new("hotbar-carry"),
            ));
            let area = Rect::from_center_size(mouse, Vec2::splat(CARRY_SIDE));
            match what.picture(frame, tools) {
                Some((texture, sprite)) => {
                    let tint = theme::with_alpha(Color32::WHITE, CARRY_ALPHA);
                    painter.image(
                        texture,
                        theme::fit(area, sprite.width, sprite.height),
                        sprite.uv,
                        tint,
                    );
                }
                None => {
                    painter.text(
                        area.center(),
                        Align2::CENTER_CENTER,
                        what.words(frame),
                        title_font(theme::SIZE_SMALL),
                        theme::GOAL,
                    );
                }
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            return;
        }
        self.dragging = None;
        if let Some(slot) = cells.iter().position(|cell| cell.contains(mouse)) {
            self.set_slot(&frame.name, slot, Some(what));
        }
    }

    /// The choices of the empty slot the player clicked: the macros of the
    /// profile and the abilities. Gives its place when it shows.
    fn picker(
        &mut self,
        ui: &egui::Ui,
        bar: Rect,
        frame: &WatchFrame,
        profile: &Profile,
    ) -> Option<Rect> {
        let slot = self.picking?;
        let choices = slot_choices(frame, profile);
        let has_macros = choices
            .iter()
            .any(|choice| matches!(choice, Slot::Macro { .. }));
        let rows = choices.len().div_ceil(PICKER_COLUMNS) + usize::from(!has_macros);
        let height = frame::TITLE_ROW + rows as f32 * PICKER_ROW + theme::PANEL_PAD * 2.0;
        let panel = Rect::from_min_size(
            Pos2::new(bar.left(), bar.top() - HOTBAR_GAP - height),
            Vec2::new(bar.width(), height),
        );
        let body = frame::draw(
            ui.painter(),
            panel,
            &format!("{WORDS_PICK_FOR} {}", HOTBAR_KEY_WORDS[slot]),
        );
        let mut top = body.top();
        if !has_macros {
            ui.painter().text(
                Pos2::new(body.left(), top),
                Align2::LEFT_TOP,
                WORDS_NO_MACROS,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            top += PICKER_ROW;
        }
        let width = (body.width() - TAB_GAP * (PICKER_COLUMNS - 1) as f32) / PICKER_COLUMNS as f32;
        let mut picked = None;
        for (at, choice) in choices.iter().enumerate() {
            let (row, column) = (at / PICKER_COLUMNS, at % PICKER_COLUMNS);
            let area = Rect::from_min_size(
                Pos2::new(
                    body.left() + column as f32 * (width + TAB_GAP),
                    top + row as f32 * PICKER_ROW,
                ),
                Vec2::new(width, PICKER_ROW - TAB_GAP),
            );
            if theme::segment_keyed(
                ui,
                area,
                Id::new(("slot-choice", at)),
                &choice.words(frame),
                theme::TEXT,
            ) {
                picked = Some(choice.clone());
            }
        }
        // A click on the bar picks another slot; one anywhere else closes.
        let clicked_away = ui.input(|i| {
            i.pointer.any_click()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|at| !panel.contains(at) && !bar.contains(at))
        });
        if let Some(choice) = picked {
            self.set_slot(&frame.name, slot, Some(choice));
            self.picking = None;
        } else if clicked_away || ui.input(|i| i.key_pressed(Key::Escape)) {
            self.picking = None;
        }
        Some(panel)
    }
}

/// The face of a filled slot: its picture, or its first letters.
fn slot_face(ui: &egui::Ui, cell: Rect, what: &Slot, frame: &WatchFrame, tools: &mut Tools<'_>) {
    match what.picture(frame, tools) {
        Some((texture, sprite)) => {
            let area = theme::fit(cell, sprite.width, sprite.height);
            ui.painter().image(texture, area, sprite.uv, Color32::WHITE);
        }
        None => {
            let short: String = what.words(frame).chars().take(SLOT_WORD_CHARS).collect();
            ui.painter().text(
                cell.center(),
                Align2::CENTER_CENTER,
                short,
                title_font(theme::SIZE_SMALL),
                theme::TEXT,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::settings::KeyBinding;

    /// The worn list takes only the rows that fit, and the wheel reaches the
    /// rest: twelve worn items in two columns of four rows scroll two rows.
    #[test]
    fn the_worn_list_fits_its_room_and_scrolls_for_the_rest() {
        const FOUR_ROWS: f32 = SLOT_ROW * 4.5;
        assert_eq!(worn_rows(FOUR_ROWS, 12), (4, 2));
        assert_eq!(
            worn_rows(FOUR_ROWS, 8),
            (4, 0),
            "a full page does not scroll"
        );
        assert_eq!(worn_rows(0.0, 3), (1, 1), "one row shows at the least");
    }

    const MARA: &str = "Mara";

    #[test]
    fn a_worn_place_has_words_a_player_knows() {
        assert_eq!(layer_words(1), "right hand");
        assert_eq!(layer_words(13), "chest");
        assert_eq!(layer_words(uoterm_protocol::types::LAYER_CLOAK), "cloak");
        assert_eq!(layer_words(uoterm_protocol::types::LAYER_ROBE), "robe");
        assert_eq!(layer_words(99), "layer 99");
    }

    #[test]
    fn jev_is_asked_about_the_bag_and_the_body() {
        use crate::view::{WatchContainer, WatchLook};
        const BAG: u32 = 0x4000_0100;
        let deck = DeckUi::starting(true);
        let frame = WatchFrame {
            look: WatchLook {
                equipment: vec![
                    WatchEquip {
                        serial: BAG,
                        layer: LAYER_BACKPACK,
                        ..WatchEquip::default()
                    },
                    WatchEquip {
                        serial: 9,
                        layer: 1,
                        ..WatchEquip::default()
                    },
                ],
                ..WatchLook::default()
            },
            containers: vec![WatchContainer {
                serial: BAG,
                items: vec![WatchPackItem {
                    serial: 7,
                    name: "a viking sword".into(),
                    ..WatchPackItem::default()
                }],
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        };
        let (choices, words) = deck.wear_choices(&frame);
        assert_eq!(choices, vec![WearChoice::Wear(7), WearChoice::TakeOff(1)]);
        assert_eq!(words[0], "a viking sword (in your bag)");
        assert_eq!(words[1], "right hand (worn now)");
        // The backpack itself is no part of the body.
        assert!(!words.iter().any(|w| w.starts_with("layer 21")));
    }

    #[test]
    fn hair_a_backpack_and_a_mount_are_not_on_the_sheet() {
        assert!(is_worn_layer(1));
        assert!(is_worn_layer(LAYER_LAST_WORN));
        assert!(!is_worn_layer(LAYER_BACKPACK));
        assert!(!is_worn_layer(LAYER_HAIR));
        assert!(!is_worn_layer(0x19));
    }

    #[test]
    fn a_hotbar_is_kept_for_each_character_and_comes_back_from_the_file() {
        let mut bars = KeptHotbars::default();
        let skill = Slot::Skill {
            id: 21,
            name: "Hiding".into(),
        };
        assert_eq!(bars.first_free(MARA), Some(0));
        bars.set(MARA, 0, Some(skill.clone()));
        bars.set(
            MARA,
            3,
            Some(Slot::Command {
                text: "bandageself".into(),
            }),
        );
        bars.set(
            MARA,
            4,
            Some(Slot::Ability {
                slot: AbilitySlot::Secondary,
            }),
        );
        bars.set(
            MARA,
            5,
            Some(Slot::Macro {
                name: "Heal".into(),
            }),
        );
        assert_eq!(bars.first_free(MARA), Some(1));
        assert_eq!(bars.slot("Cedric", 0), None);
        let dir = std::env::temp_dir().join(format!("uoterm-hotbar-{}", uuid::Uuid::new_v4()));
        let path = dir.join(HOTBAR_FILE);
        kept::save_to(&path, &bars);
        let back: KeptHotbars = kept::load_from(&path);
        assert_eq!(back, bars);
        let frame = WatchFrame::default();
        let profile = Profile::default();
        assert_eq!(
            back.slot(MARA, 0)
                .and_then(|slot| slot.press(&frame, &profile)),
            Some(Press::Act(Act::UseSkill(21)))
        );
        std::fs::remove_dir_all(&dir).unwrap();
        bars.set(MARA, 0, None);
        assert_eq!(bars.first_free(MARA), Some(0));
    }

    #[test]
    fn a_macro_slot_runs_the_macro_of_the_profile_and_an_ability_slot_arms() {
        let frame = WatchFrame::default();
        let mut profile = Profile::default();
        let heal = Slot::Macro {
            name: "Heal".into(),
        };
        assert_eq!(heal.press(&frame, &profile), None, "no such macro yet");
        let steps = vec![MacroStep::new("cast", "Heal")];
        profile.macros.key_bindings.push(KeyBinding {
            name: "Heal".into(),
            steps: steps.clone(),
            ..KeyBinding::default()
        });
        assert_eq!(heal.press(&frame, &profile), Some(Press::Macro(steps)));
        let primary = Slot::Ability {
            slot: AbilitySlot::Primary,
        };
        assert_eq!(
            primary.press(&frame, &profile),
            Some(Press::Act(Act::Command("setability 'primary' 'on'".into())))
        );
        let passive = Slot::Racial {
            icon: 0x5DD0,
            name: "Strong Back".into(),
        };
        assert_eq!(passive.press(&frame, &profile), None);
    }

    #[test]
    fn an_empty_slot_offers_the_macros_the_abilities_and_the_flight() {
        const RACE_GARGOYLE: u8 = 3;
        let mut frame = WatchFrame::default();
        let mut profile = Profile::default();
        profile.macros.key_bindings.push(KeyBinding {
            name: "Heal".into(),
            ..KeyBinding::default()
        });
        let choices = slot_choices(&frame, &profile);
        assert_eq!(choices.len(), 3, "the macro and the two abilities");
        frame.status.race = RACE_GARGOYLE;
        let choices = slot_choices(&frame, &profile);
        assert_eq!(
            choices.last(),
            Some(&Slot::Racial {
                icon: super::super::model::abilities::FLIGHT_ICON,
                name: "Flying".into()
            })
        );
    }

    #[test]
    fn the_sheet_opens_on_a_view_and_a_command_opens_a_panel_at_the_next_draw() {
        let mut deck = DeckUi::starting(false);
        deck.show_view(CharacterView::Status);
        assert!(deck.shows_view(CharacterView::Status));
        assert!(!deck.shows_view(CharacterView::Worn));
        deck.switch_panel(AbilityPanel::Combat, GumpOp::Toggle);
        assert_eq!(deck.panel_ops, vec![(AbilityPanel::Combat, GumpOp::Toggle)]);
        assert!(sheet_height(SHEET_MIN_ROWS) < sheet_height(SHEET_ROWS));
    }

    /// A frame of a gargoyle with a book, two skills, a party and an invite.
    fn full_frame() -> WatchFrame {
        use crate::view::{WatchPartyMember, WatchSkill, WatchSpellbook};
        const RACE_GARGOYLE: u8 = 3;
        let mut frame = WatchFrame {
            serial: 1,
            name: MARA.into(),
            human_control: true,
            skills: vec![WatchSkill {
                id: 21,
                name: "Hiding".into(),
                usable: true,
                ..WatchSkill::default()
            }],
            spellbooks: vec![WatchSpellbook {
                serial: 0x4000_0200,
                school: "magery".into(),
                spells: vec![(1, "Clumsy".into())],
                ..WatchSpellbook::default()
            }],
            party_members: vec![WatchPartyMember {
                serial: 1,
                name: MARA.into(),
                hits_percent: Some(80),
                ..WatchPartyMember::default()
            }],
            party_invite: Some(2),
            ..WatchFrame::default()
        };
        frame.status.race = RACE_GARGOYLE;
        frame
    }

    #[test]
    fn the_hotbar_is_a_panel_that_stands_where_the_profile_keeps_it() {
        use super::super::modern::testing::{draw_frames, SCREEN};
        let frame = full_frame();
        let mut deck = DeckUi::starting(false);
        let mut profile = Profile::default();
        let pack = Rect::from_min_size(Pos2::new(400.0, 700.0), Vec2::new(480.0, 80.0));
        let mut first = Rect::NOTHING;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            // The hotbar is the last place the deck covers.
            first = *deck
                .draw(ui, rect, &frame, tools, profile, pack)
                .last()
                .unwrap();
        });
        assert_eq!(
            first,
            layout::first_place(
                Rect::from_min_size(Pos2::ZERO, SCREEN),
                Spot::Hotbar,
                hotbar_size(pack.width())
            ),
            "it first stands where the plan puts it"
        );
        let moved = first.translate(Vec2::new(-100.0, -200.0));
        places::remember(&mut profile, HOTBAR_ID, moved, false);
        let mut shown = Rect::NOTHING;
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            shown = *deck
                .draw(ui, rect, &frame, tools, profile, pack)
                .last()
                .unwrap();
        });
        assert_eq!(shown.min, moved.min, "the kept place");
        assert_eq!(shown.size(), first.size());
    }

    #[test]
    fn every_tab_and_panel_of_the_deck_draws_and_the_invite_shows_off_the_party_tab() {
        use super::super::modern::testing::draw_frames;
        let frame = full_frame();
        let mut deck = DeckUi::starting(true);
        let mut profile = Profile::default();
        places::set_open(&mut profile, ABILITIES_ID, true);
        places::set_open(&mut profile, RACIAL_ID, true);
        let pack = Rect::from_min_size(Pos2::new(400.0, 700.0), Vec2::new(480.0, 80.0));
        for (tab, _) in TABS {
            for view in [CharacterView::Worn, CharacterView::Status] {
                deck.show(tab);
                deck.view = view;
                let mut covered = Vec::new();
                draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
                    covered = deck.draw(ui, rect, &frame, tools, profile, pack);
                });
                // The sheet, two panels, the hotbar, and the invite off
                // the party tab.
                let invite = usize::from(tab != Tab::Party);
                assert_eq!(covered.len(), 4 + invite, "{tab:?} {view:?}");
            }
        }
    }

    #[test]
    fn a_click_turns_the_character_tab_to_the_status() {
        use super::super::modern::testing::{click, draw_frames, SCREEN};
        let frame = full_frame();
        let mut deck = DeckUi::starting(true);
        let mut profile = Profile::default();
        let pack = Rect::from_min_size(Pos2::new(400.0, 700.0), Vec2::new(480.0, 80.0));
        let window = Rect::from_min_size(Pos2::ZERO, SCREEN);
        let inner = sheet_first_place(window).min + Vec2::splat(theme::PANEL_PAD);
        let body_top = inner.y + frame::TITLE_ROW + TAB_HEIGHT + TAB_GAP;
        let status = Pos2::new(
            inner.x + VIEW_WIDTH + TAB_GAP + VIEW_WIDTH / 2.0,
            body_top + (ROW - TAB_GAP) / 2.0,
        );
        draw_frames(&mut profile, &click(status), |ui, rect, tools, profile| {
            deck.draw(ui, rect, &frame, tools, profile, pack);
        });
        assert!(deck.shows_view(CharacterView::Status));
    }
}
