//! The deck of the player: the sheet with what the character wears, his
//! skills and his party, and the hotbar at the bottom of the window. The
//! sheet shows at all times it is open. Its clicks, and the hotbar, work
//! only while the human has control.

use super::boxes_ui::{scrolled, Tools, CELL, CELL_GAP, CELL_RADIUS};
use super::control::Act;
use super::desk::Zone;
use super::kept;
use super::ring_ui::Subject;
use super::theme::{self, number_font, text_font, title_font};
use crate::view::{WatchFrame, WatchPackItem, WatchSkill, LAYER_BACKPACK};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uoterm_assist::spells::SpellBook;

const SHEET_LEFT: f32 = 356.0;
const SHEET_TOP: f32 = 150.0;
const SHEET_WIDTH: f32 = 324.0;
const SHEET_ROWS: usize = 12;
const ROW: f32 = 24.0;
const TAB_HEIGHT: f32 = 28.0;
const TAB_GAP: f32 = 6.0;
const WORN_COLUMNS: usize = 6;
const LOCK_SIDE: f32 = 16.0;
const LOCK_MARK: f32 = 5.0;
const USE_WIDTH: f32 = 40.0;
const ROW_BUTTON_INSET: f32 = 2.0;
const PIN_WIDTH: f32 = 36.0;
const SKILL_TENTHS: u16 = 10;
const PERCENT: f32 = 100.0;

const LAYER_HAIR: u8 = 0x0B;
const LAYER_BEARD: u8 = 0x10;
/// The last layer that is clothes or arms. Above it are the mount and the
/// boxes of a shopkeeper and the bank.
const LAYER_LAST_WORN: u8 = 0x18;

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
    /// The spells of the standard schools. A shard's own spells go through
    /// the command box.
    spells: SpellBook,
    hotbars: KeptHotbars,
}

impl DeckUi {
    pub fn starting(open: bool) -> Self {
        Self {
            open,
            tab: Tab::default(),
            first_skill: 0,
            first_spell: 0,
            spells: SpellBook::standard(),
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

fn is_worn_layer(layer: u8) -> bool {
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
        ui: &egui::Ui,
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
        ui: &egui::Ui,
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
            Tab::Character => character_tab(ui, body, frame, tools),
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

fn character_tab(ui: &egui::Ui, body: Rect, frame: &WatchFrame, tools: &mut Tools<'_>) {
    tools.desk.zone(body, Zone::Wear);
    let painter = ui.painter();
    let facts = [
        ("Strength", frame.stats.strength.to_string()),
        ("Dexterity", frame.stats.dexterity.to_string()),
        ("Intelligence", frame.stats.intelligence.to_string()),
        ("Weight", format!("{} / {}", frame.weight, frame.weight_max)),
        ("Gold", frame.gold.to_string()),
    ];
    for (i, (words, value)) in facts.iter().enumerate() {
        let y = body.top() + (i as f32 + 0.5) * ROW;
        painter.text(
            Pos2::new(body.left(), y),
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
    let worn_top = body.top() + (facts.len() as f32 + 0.5) * ROW;
    let worn = frame
        .look
        .equipment
        .iter()
        .filter(|item| is_worn_layer(item.layer));
    for (i, item) in worn.enumerate() {
        let cell = Rect::from_min_size(
            Pos2::new(
                body.left() + (i % WORN_COLUMNS) as f32 * (CELL + CELL_GAP),
                worn_top + (i / WORN_COLUMNS) as f32 * (CELL + CELL_GAP),
            ),
            Vec2::splat(CELL),
        );
        let response = ui.interact(
            cell,
            Id::new(("worn-item", item.serial, item.layer)),
            Sense::click_and_drag(),
        );
        let fill = if response.hovered() {
            theme::BUTTON_HOVER
        } else {
            theme::TRACK
        };
        painter.rect_filled(cell, CornerRadius::same(CELL_RADIUS), fill);
        if let Some((texture, sprite)) = tools.scene.item_picture(frame.map, item.graphic, item.hue)
        {
            let area = theme::fit(cell, sprite.width, sprite.height);
            painter.image(texture, area, sprite.uv, Color32::WHITE);
        }
        if response.hovered() && !tools.desk.carries() && !tools.ring.is_open() {
            let footer = if frame.human_control { HINT_WORN } else { "" };
            tools
                .tips
                .point_at(ui, tools.hand, item.serial, "", footer, tools.time);
        }
        if !frame.human_control {
            continue;
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
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
                .open_at(cell.center(), item.serial, "", Subject::Packed, tools.hand);
        }
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
