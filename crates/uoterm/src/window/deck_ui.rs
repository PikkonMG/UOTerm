//! The deck of the player: the sheet with what the character wears, his
//! skills and his party, and the hotbar at the bottom of the window. The
//! sheet shows at all times it is open. Its clicks, and the hotbar, work
//! only while the human has control.

use super::boxes_ui::{scrolled, Tools, CELL, CELL_GAP, CELL_RADIUS};
use super::control::{Act, Answer, Ask};
use super::desk::Zone;
use super::kept;
use super::ring_ui::Subject;
use super::theme::{self, number_font, text_font, title_font};
use crate::view::{WatchFrame, WatchPackItem, WatchSkill};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uoterm_assist::spells::SpellBook;
use uoterm_protocol::types::{LAYER_BACKPACK, LAYER_BEARD, LAYER_HAIR, LAYER_LEGS};

const SHEET_LEFT: f32 = 356.0;
const SHEET_TOP: f32 = 150.0;
const SHEET_WIDTH: f32 = 430.0;
const SHEET_ROWS: usize = 12;
const ROW: f32 = 24.0;
const TAB_HEIGHT: f32 = 28.0;
const TAB_GAP: f32 = 6.0;
const LOCK_SIDE: f32 = 16.0;
const LOCK_MARK: f32 = 5.0;
const USE_WIDTH: f32 = 40.0;
const ROW_BUTTON_INSET: f32 = 2.0;
const PIN_WIDTH: f32 = 36.0;
const SKILL_TENTHS: u16 = 10;
const PERCENT: f32 = 100.0;

/// The last layer that is clothes or arms. Above it are the mount and the
/// boxes of a shopkeeper and the bank.
const LAYER_LAST_WORN: u8 = LAYER_LEGS;

const SKILL_LOCK_UP: u8 = 0;
const SKILL_LOCK_DOWN: u8 = 1;
const LOCK_WORDS: [&str; 3] = ["up", "down", "locked"];

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

/// How many letters of a skill or a command a slot shows.
const SLOT_WORD_CHARS: usize = 6;

const WORDS_USE: &str = "Use";
const WORDS_CAST: &str = "Cast";
const WORDS_PIN: &str = "Pin";
const WORDS_NO_PARTY: &str = "No party.";
const HINT_WORN: &str = "Drag: take off.  Right-click: more.";
const HINT_SLOT: &str = "Click or press the key: use.  Right-click: clear.";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Tab {
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
}

impl Slot {
    fn act(&self) -> Act {
        match self {
            Self::Item { serial, .. } => Act::Use(*serial),
            Self::Skill { id, .. } => Act::UseSkill(*id),
            Self::Spell { id, .. } => Act::Cast(*id),
            Self::Command { text } => Act::Command(text.clone()),
        }
    }

    fn words(&self) -> &str {
        match self {
            Self::Item { name, .. } | Self::Skill { name, .. } | Self::Spell { name, .. } => name,
            Self::Command { text } => text,
        }
    }
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

pub struct DeckUi {
    open: bool,
    tab: Tab,
    first_skill: usize,
    first_spell: usize,
    /// The first row of the worn list that shows.
    first_worn: usize,
    /// The spells of the standard schools. A shard's own spells go through
    /// the command box.
    spells: SpellBook,
    /// The words the human typed about what to wear, and the things Jev
    /// was asked about, in the order it was asked.
    wear_wish: String,
    wear_asked: Vec<WearChoice>,
    wear_note: Option<(String, bool, f64)>,
    hotbars: KeptHotbars,
}

impl DeckUi {
    pub fn starting(open: bool) -> Self {
        Self {
            open,
            tab: Tab::default(),
            first_skill: 0,
            first_spell: 0,
            first_worn: 0,
            spells: SpellBook::standard(),
            wear_wish: String::new(),
            wear_asked: Vec::new(),
            wear_note: None,
            hotbars: kept::load(HOTBAR_FILE),
        }
    }
}

/// A skill value in points, from the tenths the shard sends.
fn points(tenths: u16) -> String {
    format!("{}.{}", tenths / SKILL_TENTHS, tenths % SKILL_TENTHS)
}

/// The script line that sets the lock after the one a skill has now.
fn next_lock_command(skill: &WatchSkill) -> String {
    let next = (usize::from(skill.lock) + 1) % LOCK_WORDS.len();
    format!("setskill '{}' {}", skill.name, LOCK_WORDS[next])
}

pub(super) fn is_worn_layer(layer: u8) -> bool {
    layer != LAYER_BACKPACK
        && layer != LAYER_HAIR
        && layer != LAYER_BEARD
        && (1..=LAYER_LAST_WORN).contains(&layer)
}

/// What the human pressed at the right end of a row.
enum RowPress {
    Go,
    Pin,
}

/// The two small buttons at the right end of a row: the act, and the pin
/// that puts the act on the hotbar.
fn row_buttons(
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

/// An arrow up, an arrow down, or a block for a locked skill.
fn lock_mark(painter: &egui::Painter, area: Rect, lock: u8, color: Color32) {
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

impl DeckUi {
    pub fn toggle(&mut self) {
        self.open = !self.open;
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
        self.hotbars.set(character, free, Some(what));
        kept::save(HOTBAR_FILE, &self.hotbars);
        true
    }

    /// Draws the sheet and the hotbar. Gives the places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        pack_panel: Rect,
    ) -> Vec<Rect> {
        let mut covered = Vec::new();
        if self.open {
            covered.push(self.sheet(ui, rect, frame, tools));
        }
        if frame.human_control {
            covered.push(self.hotbar(ui, frame, tools, pack_panel));
        }
        covered
    }

    fn sheet(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Rect {
        let panel = Rect::from_min_size(
            rect.left_top() + Vec2::new(SHEET_LEFT, SHEET_TOP),
            Vec2::new(
                SHEET_WIDTH,
                theme::PANEL_PAD * 2.0 + TAB_HEIGHT + TAB_GAP + SHEET_ROWS as f32 * ROW,
            ),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
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
        match self.tab {
            Tab::Character => character_tab(ui, body, frame, tools, self),
            Tab::Skills => self.skills_tab(ui, panel, body, frame, tools),
            Tab::Spells => self.spells_tab(ui, panel, body, frame, tools),
            Tab::Party => party_tab(ui, body, frame, tools),
        }
        panel
    }

    fn skills_tab(
        &mut self,
        ui: &egui::Ui,
        panel: Rect,
        body: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) {
        let mut skills: Vec<&WatchSkill> = frame.skills.iter().collect();
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        let last_first = skills.len().saturating_sub(SHEET_ROWS);
        self.first_skill = scrolled(ui, panel, self.first_skill, last_first);
        let live = frame.human_control;
        let painter = ui.painter();
        let shown = skills.into_iter().skip(self.first_skill).take(SHEET_ROWS);
        for (i, skill) in shown.enumerate() {
            let row = Rect::from_min_size(
                body.left_top() + Vec2::new(0.0, i as f32 * ROW),
                Vec2::new(body.width(), ROW),
            );
            let lock = Rect::from_center_size(
                Pos2::new(row.left() + LOCK_SIDE / 2.0, row.center().y),
                Vec2::splat(LOCK_SIDE),
            );
            let lock_response =
                ui.interact(lock, Id::new(("skill-lock", skill.id)), Sense::click());
            let lock_color = if live && lock_response.hovered() {
                theme::TEXT
            } else {
                theme::TEXT_DIM
            };
            lock_mark(painter, lock, skill.lock, lock_color);
            if live && lock_response.clicked() {
                tools.hand.act(Act::Command(next_lock_command(skill)));
            }
            painter.text(
                Pos2::new(lock.right() + theme::ROW_GAP, row.center().y),
                Align2::LEFT_CENTER,
                &skill.name,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            let value_right = row.right() - PIN_WIDTH - USE_WIDTH - TAB_GAP * 2.0;
            if skill.usable && live {
                match row_buttons(ui, row, ("skill", skill.id), WORDS_USE) {
                    Some(RowPress::Pin) => {
                        self.pin(
                            &frame.name,
                            Slot::Skill {
                                id: skill.id,
                                name: skill.name.clone(),
                            },
                        );
                    }
                    Some(RowPress::Go) => tools.hand.act(Act::UseSkill(skill.id)),
                    None => {}
                }
            }
            painter.text(
                Pos2::new(value_right, row.center().y),
                Align2::RIGHT_CENTER,
                points(skill.value),
                number_font(theme::SIZE_BODY),
                theme::TEXT_DIM,
            );
        }
    }

    fn spells_tab(
        &mut self,
        ui: &egui::Ui,
        panel: Rect,
        body: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) {
        let mut spells: Vec<_> = self.spells.iter().collect();
        spells.sort_by_key(|spell| spell.id);
        let last_first = spells.len().saturating_sub(SHEET_ROWS);
        self.first_spell = scrolled(ui, panel, self.first_spell, last_first);
        let mut pinned = None;
        let shown = spells.into_iter().skip(self.first_spell).take(SHEET_ROWS);
        for (i, spell) in shown.enumerate() {
            let row = Rect::from_min_size(
                body.left_top() + Vec2::new(0.0, i as f32 * ROW),
                Vec2::new(body.width(), ROW),
            );
            ui.painter().text(
                row.left_center(),
                Align2::LEFT_CENTER,
                &spell.name,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            if !frame.human_control {
                continue;
            }
            match row_buttons(ui, row, ("spell", spell.id), WORDS_CAST) {
                Some(RowPress::Pin) => {
                    pinned = Some(Slot::Spell {
                        id: spell.id,
                        name: spell.name.clone(),
                    });
                }
                Some(RowPress::Go) => tools.hand.act(Act::Cast(spell.id)),
                None => {}
            }
        }
        if let Some(what) = pinned {
            self.pin(&frame.name, what);
        }
    }

    fn hotbar(
        &mut self,
        ui: &egui::Ui,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        pack_panel: Rect,
    ) -> Rect {
        if let Some((slot, item)) = tools.desk.slotted.take() {
            let what = Slot::Item {
                serial: item.serial,
                graphic: item.graphic,
                hue: item.hue,
                name: item.name,
            };
            self.hotbars.set(&frame.name, slot, Some(what));
            kept::save(HOTBAR_FILE, &self.hotbars);
        }
        // The bar is as wide as the pack panel under it, so it never lies on
        // the panels at its sides.
        let side = ((pack_panel.width() + CELL_GAP) / HOTBAR_SLOTS as f32 - CELL_GAP).min(CELL);
        let width = HOTBAR_SLOTS as f32 * (side + CELL_GAP) - CELL_GAP;
        let bar = Rect::from_min_size(
            Pos2::new(
                pack_panel.center().x - width / 2.0,
                pack_panel.top() - HOTBAR_GAP - side,
            ),
            Vec2::new(width, side),
        );
        let typing = ui.ctx().wants_keyboard_input();
        for slot in 0..HOTBAR_SLOTS {
            let cell = Rect::from_min_size(
                bar.left_top() + Vec2::new(slot as f32 * (side + CELL_GAP), 0.0),
                Vec2::splat(side),
            );
            tools.desk.zone(cell, Zone::Slot(slot));
            let response = ui.interact(cell, Id::new(("hotbar", slot)), Sense::click());
            let painter = ui.painter();
            let fill = if response.hovered() {
                theme::BUTTON_HOVER
            } else {
                theme::GLASS
            };
            painter.rect_filled(cell, CornerRadius::same(CELL_RADIUS), fill);
            painter.text(
                cell.left_top() + Vec2::splat(theme::CELL_ART_PAD),
                Align2::LEFT_TOP,
                HOTBAR_KEY_WORDS[slot],
                number_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            let Some(what) = self.hotbars.slot(&frame.name, slot).cloned() else {
                continue;
            };
            match &what {
                Slot::Item { graphic, hue, .. } => {
                    if let Some((texture, sprite)) =
                        tools.scene.item_picture(frame.map, *graphic, *hue)
                    {
                        let area = theme::fit(cell, sprite.width, sprite.height);
                        painter.image(texture, area, sprite.uv, Color32::WHITE);
                    }
                }
                Slot::Skill { name, .. }
                | Slot::Spell { name, .. }
                | Slot::Command { text: name } => {
                    let short: String = name.chars().take(SLOT_WORD_CHARS).collect();
                    painter.text(
                        cell.center(),
                        Align2::CENTER_CENTER,
                        short,
                        title_font(theme::SIZE_SMALL),
                        theme::TEXT,
                    );
                }
            }
            if response.hovered() && !tools.desk.carries() {
                match &what {
                    Slot::Item { serial, name, .. } => {
                        tools
                            .tips
                            .point_at(ui, tools.hand, *serial, name, HINT_SLOT, tools.time);
                    }
                    other => super::tips::label(ui, other.words(), HINT_SLOT),
                }
            }
            let key = !typing && ui.input(|i| i.key_pressed(HOTBAR_KEYS[slot]));
            if response.clicked() || key {
                tools.hand.act(what.act());
            } else if response.secondary_clicked() {
                self.hotbars.set(&frame.name, slot, None);
                kept::save(HOTBAR_FILE, &self.hotbars);
            }
        }
        bar
    }
}

/// The layers a person wears, in the order a paperdoll lists them, with
/// the words for each.
const WORN_LAYERS: [(u8, &str); 19] = [
    (1, "right hand"),
    (2, "left hand"),
    (3, "shoes"),
    (4, "trousers"),
    (5, "shirt"),
    (6, "head"),
    (7, "gloves"),
    (8, "ring"),
    (9, "talisman"),
    (10, "neck"),
    (12, "waist"),
    (13, "chest"),
    (14, "bracelet"),
    (16, "arms"),
    (17, "cloak"),
    (19, "robe"),
    (20, "skirt"),
    (21, "legs"),
    (22, "earrings"),
];
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
            |(_, words)| (*words).to_string(),
        )
}

/// What Jev reads about one thing the character could wear or take off.
fn wear_words(name: &str, where_it_is: &str) -> String {
    format!("{name} ({where_it_is})")
}

fn character_tab(
    ui: &mut egui::Ui,
    body: Rect,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    deck: &mut DeckUi,
) {
    // An item dropped anywhere on this tab is put on.
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
    let facts = [
        ("Strength", frame.stats.strength.to_string()),
        ("Dexterity", frame.stats.dexterity.to_string()),
        ("Intelligence", frame.stats.intelligence.to_string()),
        ("Weight", frame.carried()),
        ("Gold", frame.gold.to_string()),
    ];
    let facts_left = doll.right() + theme::ROW_GAP * 2.0;
    for (i, (words, value)) in facts.iter().enumerate() {
        let y = body.top() + (i as f32 + 0.5) * ROW;
        painter.text(
            Pos2::new(facts_left, y),
            Align2::LEFT_CENTER,
            words,
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        painter.text(
            Pos2::new(body.right(), y),
            Align2::RIGHT_CENTER,
            value,
            number_font(theme::SIZE_BODY),
            theme::TEXT,
        );
    }
    let worn: Vec<&crate::view::WatchEquip> = frame
        .look
        .equipment
        .iter()
        .filter(|item| is_worn_layer(item.layer))
        .collect();
    let label_top = doll.bottom() + theme::ROW_GAP;
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
    deck.first_worn = scrolled(ui, list, deck.first_worn.min(last_first), last_first);
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
        .skip(deck.first_worn * WORN_COLUMNS)
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
        worn_row(ui, slot, item, frame, tools);
    }
    deck.wear_field(ui, field, frame, tools);
}

/// How many rows of the worn list fit in `height`, and the last first row
/// the wheel may scroll to, for `count` worn items in their columns.
fn worn_rows(height: f32, count: usize) -> (usize, usize) {
    let rows = ((height / SLOT_ROW).floor() as usize).max(1);
    let needed = count.div_ceil(WORN_COLUMNS);
    (rows, needed.saturating_sub(rows))
}

impl DeckUi {
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
        for answer in tools.hand.new_answers() {
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
                // The other panels take the rest.
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
            tools.hand.ask(Ask::WearItem {
                wish: self.wear_wish.trim().to_string(),
                options: words,
            });
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

/// One worn item: its picture, the words for its layer, its name, and a
/// cross that takes it off.
fn worn_row(
    ui: &egui::Ui,
    row: Rect,
    item: &crate::view::WatchEquip,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
) {
    let art = Rect::from_min_size(row.min, Vec2::splat(row.height()));
    if let Some((texture, sprite)) = tools.scene.item_picture(frame.map, item.graphic, item.hue) {
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
            name: String::new(),
        });
    } else if response.double_clicked() {
        tools.hand.act(Act::TakeOff(item.layer));
    } else if response.secondary_clicked() {
        tools
            .ring
            .open_at(row.center(), item.serial, "", Subject::Packed, tools.hand);
    }
}

fn party_tab(ui: &egui::Ui, body: Rect, frame: &WatchFrame, tools: &mut Tools<'_>) {
    let painter = ui.painter();
    if frame.party_members.is_empty() {
        painter.text(
            body.left_top(),
            Align2::LEFT_TOP,
            WORDS_NO_PARTY,
            text_font(theme::SIZE_BODY),
            theme::TEXT_FAINT,
        );
        return;
    }
    for (i, member) in frame.party_members.iter().take(SHEET_ROWS / 2).enumerate() {
        let row = Rect::from_min_size(
            body.left_top() + Vec2::new(0.0, i as f32 * ROW * 2.0),
            Vec2::new(body.width(), ROW * 2.0 - theme::ROW_GAP),
        );
        let response = ui.interact(row, Id::new(("party", member.serial)), Sense::click());
        painter.text(
            row.left_top(),
            Align2::LEFT_TOP,
            &member.name,
            text_font(theme::SIZE_BODY),
            if response.hovered() {
                theme::TEXT
            } else {
                theme::TEXT_DIM
            },
        );
        let track = Rect::from_min_max(
            Pos2::new(row.left(), row.bottom() - theme::BAR_HEIGHT),
            row.right_bottom(),
        );
        painter.rect_filled(track, CornerRadius::same(theme::BAR_RADIUS), theme::TRACK);
        if let Some(percent) = member.hits_percent {
            let mut fill = track;
            fill.set_width(track.width() * f32::from(percent) / PERCENT);
            painter.rect_filled(fill, CornerRadius::same(theme::BAR_RADIUS), theme::HITS);
        }
        if frame.human_control && response.clicked() {
            tools.hand.act(if frame.target_cursor {
                Act::Target(member.serial)
            } else {
                Act::Look(member.serial)
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn hiding(lock: u8) -> WatchSkill {
        WatchSkill {
            id: 21,
            name: "Hiding".into(),
            lock,
            ..WatchSkill::default()
        }
    }

    #[test]
    fn a_worn_place_has_words_a_player_knows() {
        assert_eq!(layer_words(1), "right hand");
        assert_eq!(layer_words(13), "chest");
        assert_eq!(layer_words(99), "layer 99");
    }

    #[test]
    fn jev_is_asked_about_the_bag_and_the_body() {
        use crate::view::{WatchContainer, WatchEquip, WatchLook, WatchPackItem};
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
    fn a_click_on_a_lock_sets_the_next_lock() {
        assert_eq!(next_lock_command(&hiding(0)), "setskill 'Hiding' down");
        assert_eq!(next_lock_command(&hiding(1)), "setskill 'Hiding' locked");
        assert_eq!(next_lock_command(&hiding(2)), "setskill 'Hiding' up");
    }

    #[test]
    fn a_skill_value_shows_in_points() {
        assert_eq!(points(702), "70.2");
        assert_eq!(points(5), "0.5");
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
        assert_eq!(bars.first_free(MARA), Some(1));
        assert_eq!(bars.slot("Cedric", 0), None);
        let dir = std::env::temp_dir().join(format!("uoterm-hotbar-{}", uuid::Uuid::new_v4()));
        let path = dir.join(HOTBAR_FILE);
        kept::save_to(&path, &bars);
        let back: KeptHotbars = kept::load_from(&path);
        assert_eq!(back, bars);
        assert_eq!(back.slot(MARA, 0).map(Slot::act), Some(Act::UseSkill(21)));
        std::fs::remove_dir_all(&dir).unwrap();
        bars.set(MARA, 0, None);
        assert_eq!(bars.first_free(MARA), Some(0));
    }
}
