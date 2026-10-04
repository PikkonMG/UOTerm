//! The rules of the deck of the player: the slots of the hotbar and what
//! each does, the hotbar each character keeps, what an empty slot may take,
//! the words of a worn place, what Jev may pick to wear or take off, and
//! the rows of the worn list. Loading and saving the kept hotbars stays
//! with each window: the Rust window keeps `HOTBAR_FILE`, the browser asks
//! the session for it.

use super::gumps::{CELL, CELL_GAP};
use super::lists::ability_slot_words;
use super::places::{with_title_room, TITLE_ROW};
use super::theme::PANEL_PAD;
use crate::act::Act;
use crate::frame::WatchFrame;
use crate::geom::Vector;
use crate::input::KeyName;
use crate::model::abilities::{
    ability_of, icon_of, race_of, racial_command, slot_hue, toggle_command, AbilitySlot,
};
use crate::model::durability::is_worn_layer;
use crate::model::key_macros;
use crate::model::spell_data::{book_spell, icon_hue};
use crate::settings::{MacroStep, Profile, WORN_LAYERS};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const HOTBAR_SLOTS: usize = 10;
/// The keys that press the slots while no field takes the keys, by their
/// egui names. Each slot shows the name of its key.
pub const HOTBAR_KEYS: [&str; HOTBAR_SLOTS] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"];
/// A slot with no picture shows this many letters of its words.
pub const SLOT_WORD_CHARS: usize = 6;
/// The file of the config folder that keeps the hotbars.
pub const HOTBAR_FILE: &str = "watch-hotbar.toml";
/// The height of a row of the worn list.
pub const SLOT_ROW: f32 = 22.0;
/// The worn list stands in two columns, so a full suit fits.
pub const WORN_COLUMNS: usize = 2;

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

/// The words for a slot of a macro the profile no longer has.
pub const WORDS_MACRO_GONE: &str = "That macro is no longer in the profile.";

/// What a slot does when the player presses it.
#[derive(Clone, Debug, PartialEq)]
pub enum Press {
    Act(Act),
    /// The steps of a macro of the profile, for the window to run.
    Macro(Vec<MacroStep>),
    /// Words for the player, as the report of an act.
    Report(&'static str),
}

impl Slot {
    /// What the slot does now. A macro no longer in the profile tells the
    /// player so. None for a racial ability the character cannot use.
    pub fn press(&self, frame: &WatchFrame, profile: &Profile) -> Option<Press> {
        Some(match self {
            Self::Item { serial, .. } => Press::Act(Act::Use(*serial)),
            Self::Skill { id, .. } => Press::Act(Act::UseSkill(*id)),
            Self::Spell { id, .. } => Press::Act(Act::Cast(*id)),
            Self::Command { text } => Press::Act(Act::Command(text.clone())),
            Self::Macro { name } => key_macros::steps_of(&profile.macros.key_bindings, name)
                .map_or(Press::Report(WORDS_MACRO_GONE), Press::Macro),
            Self::Ability { slot } => Press::Act(Act::Command(toggle_command(frame, *slot))),
            Self::Racial { icon, .. } => {
                Press::Act(Act::Command(racial_command(frame, *icon)?.to_string()))
            }
        })
    }

    /// The first letters of its words, which a slot with no picture shows.
    pub fn face_words(&self, frame: &WatchFrame) -> String {
        self.words(frame).chars().take(SLOT_WORD_CHARS).collect()
    }

    /// The words the slot shows when it has no picture, and in its tip.
    pub fn words(&self, frame: &WatchFrame) -> String {
        match self {
            Self::Item { name, .. }
            | Self::Skill { name, .. }
            | Self::Spell { name, .. }
            | Self::Macro { name }
            | Self::Racial { name, .. } => name.clone(),
            Self::Command { text } => text.clone(),
            Self::Ability { slot } => ability_slot_words(frame, *slot),
        }
    }
}

/// The slot a key presses, while no field takes the keys.
pub fn hotbar_key_slot(key: &KeyName) -> Option<usize> {
    HOTBAR_KEYS.iter().position(|name| *name == key.0)
}

/// The picture a slot shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotPicture {
    Item { graphic: u16, hue: u16 },
    Gump { gump: u16, hue: u16 },
}

/// The picture of a slot: its item, the icon of its spell or ability.
/// None shows its words.
pub fn slot_picture(slot: &Slot, frame: &WatchFrame) -> Option<SlotPicture> {
    match slot {
        Slot::Item { graphic, hue, .. } => Some(SlotPicture::Item {
            graphic: *graphic,
            hue: *hue,
        }),
        Slot::Spell { id, .. } => {
            let (_, spell) = book_spell(*id)?;
            Some(SlotPicture::Gump {
                gump: spell.small_icon,
                hue: icon_hue(frame, *id),
            })
        }
        Slot::Ability { slot } => Some(SlotPicture::Gump {
            gump: icon_of(ability_of(frame, *slot)),
            hue: slot_hue(frame, *slot),
        }),
        Slot::Racial { icon, .. } => Some(SlotPicture::Gump {
            gump: *icon,
            hue: 0,
        }),
        Slot::Skill { .. } | Slot::Command { .. } | Slot::Macro { .. } => None,
    }
}

/// The hotbar of each character, by his name. The key of a slot is its
/// number as text, because a TOML list cannot hold an empty place.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptHotbars {
    #[serde(default)]
    characters: BTreeMap<String, BTreeMap<String, Slot>>,
}

impl KeptHotbars {
    pub fn slot(&self, character: &str, slot: usize) -> Option<&Slot> {
        self.characters.get(character)?.get(&slot.to_string())
    }

    pub fn set(&mut self, character: &str, slot: usize, what: Option<Slot>) {
        let bar = self.characters.entry(character.to_string()).or_default();
        match what {
            Some(what) => bar.insert(slot.to_string(), what),
            None => bar.remove(&slot.to_string()),
        };
    }

    pub fn first_free(&self, character: &str) -> Option<usize> {
        (0..HOTBAR_SLOTS).find(|slot| self.slot(character, *slot).is_none())
    }
}

/// What an empty slot may take beside the rows of the sheet: each macro of
/// the profile, the two weapon abilities, and the racial abilities the
/// character uses.
pub fn slot_choices(frame: &WatchFrame, profile: &Profile) -> Vec<Slot> {
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

/// The side of a hotbar cell, and the width of the row of cells, for a
/// bar as wide as a pack panel of `pack_width`, so it never lies on the
/// panels at the sides of the pack.
pub fn hotbar_cells(pack_width: f32) -> (f32, f32) {
    let room = pack_width - PANEL_PAD * 2.0;
    let side = ((room + CELL_GAP) / HOTBAR_SLOTS as f32 - CELL_GAP).min(CELL);
    (side, HOTBAR_SLOTS as f32 * (side + CELL_GAP) - CELL_GAP)
}

/// The size of the hotbar panel over a pack panel of `pack_width`.
pub fn hotbar_size(pack_width: f32) -> Vector {
    let (side, width) = hotbar_cells(pack_width);
    with_title_room(Vector::new(
        width + PANEL_PAD * 2.0,
        TITLE_ROW + side + PANEL_PAD * 2.0,
    ))
}

/// One thing Jev may pick: an item of the bag to wear, or a worn item to
/// take off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WearChoice {
    Wear(u32),
    TakeOff(u8),
}

/// The words for a layer, or its number when it has no name here.
pub fn layer_words(layer: u8) -> String {
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

/// The things the words of the player could mean: each item of the
/// backpack to put on, and each worn item to take off, with the words Jev
/// reads about each.
pub fn wear_choices(frame: &WatchFrame) -> (Vec<WearChoice>, Vec<String>) {
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

/// How many rows of the worn list fit in `height`, and the last first row
/// the wheel may scroll to, for `count` worn items in their columns.
pub fn worn_rows(height: f32, count: usize) -> (usize, usize) {
    let rows = ((height / SLOT_ROW).floor() as usize).max(1);
    let needed = count.div_ceil(WORN_COLUMNS);
    (rows, needed.saturating_sub(rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchContainer, WatchEquip, WatchLook, WatchPackItem};
    use crate::model::abilities::FLIGHT_ICON;
    use crate::settings::KeyBinding;
    use uoterm_protocol::types::{LAYER_BACKPACK, LAYER_CLOAK, LAYER_ROBE};

    const MARA: &str = "Mara";

    #[test]
    fn the_number_keys_press_the_slots_and_a_slot_shows_its_picture() {
        assert_eq!(hotbar_key_slot(&KeyName("1".into())), Some(0));
        assert_eq!(
            hotbar_key_slot(&KeyName("0".into())),
            Some(HOTBAR_SLOTS - 1)
        );
        assert_eq!(hotbar_key_slot(&KeyName("A".into())), None);
        let frame = WatchFrame::default();
        let axe = Slot::Item {
            serial: 1,
            graphic: 0x0F43,
            hue: 2,
            name: "hatchet".into(),
        };
        assert_eq!(
            slot_picture(&axe, &frame),
            Some(SlotPicture::Item {
                graphic: 0x0F43,
                hue: 2
            })
        );
        let racial = Slot::Racial {
            icon: FLIGHT_ICON,
            name: "Flying".into(),
        };
        assert_eq!(
            slot_picture(&racial, &frame),
            Some(SlotPicture::Gump {
                gump: FLIGHT_ICON,
                hue: 0
            })
        );
        let command = Slot::Command {
            text: "bandageself".into(),
        };
        assert_eq!(slot_picture(&command, &frame), None);
        assert_eq!(command.face_words(&frame), "bandag");
    }

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

    #[test]
    fn a_worn_place_has_words_a_player_knows() {
        assert_eq!(layer_words(1), "right hand");
        assert_eq!(layer_words(13), "chest");
        assert_eq!(layer_words(LAYER_CLOAK), "cloak");
        assert_eq!(layer_words(LAYER_ROBE), "robe");
        assert_eq!(layer_words(99), "layer 99");
    }

    #[test]
    fn jev_is_asked_about_the_bag_and_the_body() {
        const BAG: u32 = 0x4000_0100;
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
        let (choices, words) = wear_choices(&frame);
        assert_eq!(choices, vec![WearChoice::Wear(7), WearChoice::TakeOff(1)]);
        assert_eq!(words[0], "a viking sword (in your bag)");
        assert_eq!(words[1], "right hand (worn now)");
        // The backpack itself is no part of the body.
        assert!(!words.iter().any(|w| w.starts_with("layer 21")));
    }

    #[test]
    fn a_hotbar_is_kept_for_each_character_and_survives_its_file_format() {
        let mut bars = KeptHotbars::default();
        assert_eq!(bars.first_free(MARA), Some(0));
        bars.set(
            MARA,
            0,
            Some(Slot::Skill {
                id: 21,
                name: "Hiding".into(),
            }),
        );
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
        assert_eq!(bars.first_free(MARA), Some(1));
        assert_eq!(bars.slot("Cedric", 0), None);
        let text = toml::to_string(&bars).unwrap();
        let back: KeptHotbars = toml::from_str(&text).unwrap();
        assert_eq!(back, bars);
        assert_eq!(
            back.slot(MARA, 0)
                .and_then(|slot| slot.press(&WatchFrame::default(), &Profile::default())),
            Some(Press::Act(Act::UseSkill(21)))
        );
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
        assert_eq!(
            heal.press(&frame, &profile),
            Some(Press::Report(WORDS_MACRO_GONE)),
            "no such macro yet"
        );
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
                icon: FLIGHT_ICON,
                name: "Flying".into()
            })
        );
    }

    #[test]
    fn the_hotbar_fits_over_the_pack_and_holds_its_title() {
        const PACK_WIDTH: f32 = 340.0;
        let (side, width) = hotbar_cells(PACK_WIDTH);
        assert!(side <= CELL);
        assert!(width + PANEL_PAD * 2.0 <= PACK_WIDTH);
        assert!(hotbar_size(PACK_WIDTH).x >= width);
    }
}
