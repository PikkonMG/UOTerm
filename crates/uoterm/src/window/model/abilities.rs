//! The weapon abilities and the racial abilities of the character, apart
//! from how a window draws them: the primary and the secondary ability of
//! the weapon in hand, which one is armed, the script line that arms one,
//! the icon of each, and the abilities of each race with the gargoyle's
//! flight, the one a player uses. The Classic books and buttons and the
//! Modern panels and hotbar all read them.

use crate::view::WatchFrame;
use uoterm_assist::abilities::weapon_moves;
use uoterm_protocol::types::{LAYER_ONE_HANDED, LAYER_TWO_HANDED};

/// The icon of the first weapon ability; the rest follow it.
pub const FIRST_ABILITY_ICON: u16 = 0x5200;
/// The tooltip of a weapon ability, and its name, from these text numbers
/// on.
pub const FIRST_ABILITY_TOOLTIP: u32 = 1_061_693;
pub const FIRST_ABILITY_NAME: u32 = 1_028_838;
/// The hue of an armed ability's icon.
pub const ARMED_HUE: u16 = 38;

/// The primary or the secondary ability of the weapon in hand.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbilitySlot {
    Primary,
    Secondary,
}

impl AbilitySlot {
    pub const BOTH: [AbilitySlot; 2] = [AbilitySlot::Primary, AbilitySlot::Secondary];

    /// The slot a serial names: 0 the primary, any other the secondary.
    pub fn of(serial: u32) -> Self {
        if serial == 0 {
            AbilitySlot::Primary
        } else {
            AbilitySlot::Secondary
        }
    }

    pub fn serial(self) -> u32 {
        match self {
            AbilitySlot::Primary => 0,
            AbilitySlot::Secondary => 1,
        }
    }

    /// The slot as the ability command names it.
    pub fn word(self) -> &'static str {
        match self {
            AbilitySlot::Primary => "primary",
            AbilitySlot::Secondary => "secondary",
        }
    }

    /// The slot as a panel names it.
    pub fn title(self) -> &'static str {
        match self {
            AbilitySlot::Primary => "Primary",
            AbilitySlot::Secondary => "Secondary",
        }
    }
}

/// The weapon in hand, as the session finds it for its ability commands:
/// the first item in either hand.
pub fn weapon(frame: &WatchFrame) -> Option<u16> {
    frame
        .look
        .equipment
        .iter()
        .find(|item| item.layer == LAYER_ONE_HANDED || item.layer == LAYER_TWO_HANDED)
        .map(|item| item.graphic)
}

/// The ability number of a slot of the weapon in hand.
pub fn ability_of(frame: &WatchFrame, slot: AbilitySlot) -> u8 {
    let (primary, secondary) = weapon_moves(weapon(frame));
    match slot {
        AbilitySlot::Primary => primary,
        AbilitySlot::Secondary => secondary,
    }
}

/// True when the ability of the slot is armed now.
pub fn armed(frame: &WatchFrame, slot: AbilitySlot) -> bool {
    frame
        .abilities
        .weapon
        .as_ref()
        .is_some_and(|(number, _)| *number == ability_of(frame, slot))
}

/// The hue of the icon of a slot: red while armed.
pub fn slot_hue(frame: &WatchFrame, slot: AbilitySlot) -> u16 {
    if armed(frame, slot) {
        ARMED_HUE
    } else {
        0
    }
}

/// The script line that arms the ability of a slot, or lets it go when it
/// is armed.
pub fn toggle_command(frame: &WatchFrame, slot: AbilitySlot) -> String {
    let on = if armed(frame, slot) { "off" } else { "on" };
    format!("setability '{}' '{on}'", slot.word())
}

/// The icon of an ability number, from one.
pub fn icon_of(ability: u8) -> u16 {
    FIRST_ABILITY_ICON + u16::from(ability.saturating_sub(1))
}

/// The races as the status names them.
const RACE_HUMAN: u8 = 1;
const RACE_ELF: u8 = 2;
const RACE_GARGOYLE: u8 = 3;
/// The gargoyle's flight, the one racial ability a player uses.
pub const FLIGHT_ICON: u16 = 0x5DDA;
const FLIGHT_COMMAND: &str = "togglefly";

/// The abilities of one race: their names, their first icon, and the text
/// number of the first tooltip.
#[derive(Debug, PartialEq, Eq)]
pub struct Race {
    pub names: &'static [&'static str],
    pub first_icon: u16,
    pub first_tooltip: u32,
}

pub const HUMAN: Race = Race {
    names: &["Strong Back", "Tough", "Workhorse", "Jack of All Trades"],
    first_icon: 0x5DD0,
    first_tooltip: 1_112_198,
};

pub const ELF: Race = Race {
    names: &[
        "Night Sight",
        "Infused with Magic",
        "Knowledge of Nature",
        "Difficult to Track",
        "Perception",
        "Wisdom",
    ],
    first_icon: 0x5DD4,
    first_tooltip: 1_112_202,
};

pub const GARGOYLE: Race = Race {
    names: &[
        "Flying",
        "Berserk",
        "Master Artisan",
        "Deadly Aim",
        "Mystic Insight",
    ],
    first_icon: FLIGHT_ICON,
    first_tooltip: 1_112_208,
};

/// The race of the character, by the status. An older shard names none.
pub fn race_of(frame: &WatchFrame) -> Option<&'static Race> {
    match frame.status.race {
        RACE_HUMAN => Some(&HUMAN),
        RACE_ELF => Some(&ELF),
        RACE_GARGOYLE => Some(&GARGOYLE),
        _ => None,
    }
}

impl Race {
    /// Only a gargoyle's flight is no passive ability.
    pub fn passive(&self, index: usize) -> bool {
        !(self.first_icon == FLIGHT_ICON && index == 0)
    }

    /// The icon of the ability at `index`, from zero.
    pub fn icon(&self, index: usize) -> u16 {
        self.first_icon + index as u16
    }
}

/// The text number of the tooltip of a racial icon. The tooltips of every
/// race count from the humans' first icon.
pub fn racial_tooltip(icon: u16) -> u32 {
    HUMAN.first_tooltip + u32::from(icon.saturating_sub(HUMAN.first_icon))
}

/// The script line a racial icon runs: the flight flies or lands, for a
/// gargoyle. The passive abilities run nothing.
pub fn racial_command(frame: &WatchFrame, icon: u16) -> Option<&'static str> {
    (icon == FLIGHT_ICON && frame.status.race == RACE_GARGOYLE).then_some(FLIGHT_COMMAND)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchAbilities, WatchEquip, WatchLook};

    const KATANA: u16 = 0x13FF;
    const DOUBLE_STRIKE: u8 = 7;
    const ARMOR_IGNORE: u8 = 1;

    fn armed_frame(armed: Option<u8>) -> WatchFrame {
        WatchFrame {
            look: WatchLook {
                equipment: vec![WatchEquip {
                    serial: 5,
                    graphic: KATANA,
                    layer: LAYER_ONE_HANDED,
                    hue: 0,
                }],
                ..WatchLook::default()
            },
            abilities: WatchAbilities {
                weapon: armed.map(|number| (number, String::new())),
                spells: Vec::new(),
            },
            ..WatchFrame::default()
        }
    }

    #[test]
    fn the_slots_follow_the_weapon_in_hand_and_the_armed_one_is_known() {
        let frame = armed_frame(Some(DOUBLE_STRIKE));
        assert_eq!(ability_of(&frame, AbilitySlot::Primary), DOUBLE_STRIKE);
        assert_eq!(ability_of(&frame, AbilitySlot::Secondary), ARMOR_IGNORE);
        assert!(armed(&frame, AbilitySlot::Primary) && !armed(&frame, AbilitySlot::Secondary));
        assert!(!armed(&armed_frame(None), AbilitySlot::Primary));
        assert_eq!(slot_hue(&frame, AbilitySlot::Primary), ARMED_HUE);
        assert_eq!(icon_of(ARMOR_IGNORE), FIRST_ABILITY_ICON);
        assert_eq!(AbilitySlot::of(1), AbilitySlot::Secondary);
        assert_eq!(
            toggle_command(&frame, AbilitySlot::Primary),
            "setability 'primary' 'off'"
        );
        assert_eq!(
            toggle_command(&frame, AbilitySlot::Secondary),
            "setability 'secondary' 'on'"
        );
    }

    #[test]
    fn each_race_has_its_abilities_and_only_a_gargoyle_flies() {
        let mut frame = WatchFrame::default();
        assert!(race_of(&frame).is_none());
        frame.status.race = RACE_ELF;
        let elf = race_of(&frame).unwrap();
        assert_eq!(elf.names.len(), 6);
        assert!(elf.passive(0));
        assert_eq!(racial_command(&frame, FLIGHT_ICON), None);
        frame.status.race = RACE_GARGOYLE;
        let gargoyle = race_of(&frame).unwrap();
        assert!(!gargoyle.passive(0) && gargoyle.passive(1));
        assert_eq!(gargoyle.icon(1), FLIGHT_ICON + 1);
        assert_eq!(racial_command(&frame, FLIGHT_ICON), Some(FLIGHT_COMMAND));
        assert_eq!(racial_command(&frame, FLIGHT_ICON + 1), None);
        assert_eq!(racial_tooltip(ELF.first_icon), ELF.first_tooltip);
    }
}
