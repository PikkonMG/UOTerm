//! The two ability panels of the Modern style, as both windows show them,
//! with what the classic combat book and racial book do. The combat panel
//! shows the primary and the secondary ability of the weapon in hand, red
//! while armed: a click arms one or lets it go, and each pins or drags
//! onto the hotbar. Under them it lists every weapon ability with the
//! weapons that have it. The racial panel shows the abilities of the
//! character's race; the gargoyle's flight flies or lands, and pins onto
//! the hotbar. What each ability is and does is `model::abilities`'.

use super::deck::Slot;
use super::layout::{first_place, Spot};
use super::places::TITLE_ROW;
use super::theme::{PANEL_PAD, ROW_GAP};
use crate::act::Act;
use crate::frame::WatchFrame;
use crate::geom::{Area, Vector};
use crate::model::abilities::{
    ability_of, icon_of, racial_command, toggle_command, AbilitySlot, Race,
};
use uoterm_assist::abilities::ABILITIES;

pub const ABILITIES_ID: &str = "modern:abilities";
pub const RACIAL_ID: &str = "modern:racial";
pub const ABILITIES_SIZE: Vector = Vector::new(300.0, 390.0);
pub const ABILITIES_MIN: Vector = Vector::new(260.0, 260.0);
const RACIAL_WIDTH: f32 = 250.0;
/// Where the panels first open: the combat panel in the left column a
/// step from the box of a party invite, the racial panel in the right
/// column, beside the sheet in the middle.
const ABILITIES_SPOT: Spot = Spot::LeftColumn(1);
const RACIAL_SPOT: Spot = Spot::RightColumn(0);
pub const ABILITY_ICON: f32 = 44.0;
pub const WORDS_ABILITIES: &str = "Combat abilities";
pub const WORDS_RACIAL: &str = "Racial abilities";
pub const WORDS_ALL: &str = "Every weapon ability";
const WORDS_ARM: &str = "Arm";
const WORDS_LET_GO: &str = "Let go";
pub const WORDS_ARMED: &str = "Armed";
pub const WORDS_RACIAL_USE: &str = "Use";
pub const WORDS_PASSIVE: &str = "Passive";
pub const WORDS_NO_RACE: &str = "The shard names no race for the character.";
pub const WORDS_WEAPONS: &str = "Weapons: ";
pub const HINT_SLOT: &str = "Click: arm or let go.  Drag: onto the hotbar.";
pub const HINT_FLIGHT: &str = "Click: fly or land.  Drag: onto the hotbar.";

/// The buttons of the character tab of the sheet that open and close the
/// two panels, right to left: their place ids and words.
pub const SHEET_PANEL_BUTTONS: [(&str, &str); 2] =
    [(RACIAL_ID, "Racial"), (ABILITIES_ID, "Abilities")];

/// Where the combat panel first stands in `window`.
pub fn abilities_first_place(window: Area) -> Area {
    first_place(window, ABILITIES_SPOT, ABILITIES_SIZE)
}

/// The height of the racial panel of `race`: a row for each ability, or
/// one for the words that name no race.
pub fn racial_height(race: Option<&Race>) -> f32 {
    let rows = race.map_or(1, |race| race.names.len()) as f32;
    TITLE_ROW + rows * (ABILITY_ICON + ROW_GAP) + PANEL_PAD * 2.0
}

/// Where the racial panel first stands in `window`, for `race`.
pub fn racial_first_place(window: Area, race: Option<&Race>) -> Area {
    first_place(
        window,
        RACIAL_SPOT,
        Vector::new(RACIAL_WIDTH, racial_height(race)),
    )
}

/// The words of the button that arms a slot or lets it go.
pub fn arm_words(armed: bool) -> &'static str {
    if armed {
        WORDS_LET_GO
    } else {
        WORDS_ARM
    }
}

/// The act of a click on a slot, or on its arm button: it arms the
/// ability or lets it go.
pub fn slot_act(frame: &WatchFrame, slot: AbilitySlot) -> Act {
    Act::Command(toggle_command(frame, slot))
}

/// The hotbar slot of the primary or the secondary ability.
pub fn slot_of(slot: AbilitySlot) -> Slot {
    Slot::Ability { slot }
}

/// One weapon ability of the list under the slots: its number, name and
/// icon, and the slot of the weapon in hand that has it.
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityRow {
    pub ability: u8,
    pub name: &'static str,
    pub icon: u16,
    pub in_hand: Option<AbilitySlot>,
}

/// Every weapon ability, with the ones of the weapon in hand marked.
pub fn ability_rows(frame: &WatchFrame) -> Vec<AbilityRow> {
    ABILITIES
        .iter()
        .map(|(ability, name)| AbilityRow {
            ability: *ability,
            name,
            icon: icon_of(*ability),
            in_hand: AbilitySlot::BOTH
                .into_iter()
                .find(|slot| ability_of(frame, *slot) == *ability),
        })
        .collect()
}

/// The act of a click on the racial ability at `index`, or on its Use
/// button: the flight flies or lands. None for a passive one, or without
/// control.
pub fn racial_act(frame: &WatchFrame, race: &Race, index: usize) -> Option<Act> {
    if race.passive(index) || !frame.human_control {
        return None;
    }
    racial_command(frame, race.icon(index)).map(|command| Act::Command(command.into()))
}

/// The hotbar slot of the racial ability at `index`, when it is one that
/// acts.
pub fn racial_slot(frame: &WatchFrame, race: &Race, index: usize) -> Option<Slot> {
    racial_act(frame, race, index)?;
    Some(Slot::Racial {
        icon: race.icon(index),
        name: race.names.get(index)?.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::abilities::{ELF, GARGOYLE};

    #[test]
    fn only_the_flight_of_a_gargoyle_acts_and_only_with_control() {
        const RACE_GARGOYLE: u8 = 3;
        let mut frame = WatchFrame::default();
        frame.status.race = RACE_GARGOYLE;
        assert_eq!(racial_act(&frame, &GARGOYLE, 0), None, "no control");
        frame.human_control = true;
        assert!(racial_act(&frame, &GARGOYLE, 0).is_some());
        assert!(racial_slot(&frame, &GARGOYLE, 0).is_some());
        assert_eq!(racial_act(&frame, &GARGOYLE, 1), None, "passive");
        assert!(racial_height(Some(&ELF)) > racial_height(None));
        assert_eq!(ability_rows(&frame).len(), ABILITIES.len());
    }
}
