//! The words of the ability panels: a slot of the weapon in hand, and the
//! weapons that have an ability.

use crate::frame::WatchFrame;
use crate::model::abilities::{ability_of, AbilitySlot};
use uoterm_assist::abilities::{ability_name, weapons_with};

/// The name of an ability, or its number when the table has none.
fn ability_name_of(ability: u8) -> String {
    ability_name(ability).map_or_else(|| ability.to_string(), str::to_string)
}

/// The words of a slot: which it is, and the ability of the weapon in hand.
pub fn ability_slot_words(frame: &WatchFrame, slot: AbilitySlot) -> String {
    format!(
        "{}: {}",
        slot.title(),
        ability_name_of(ability_of(frame, slot))
    )
}

/// The names of the weapons that have an ability, each once, as the client
/// files name their graphics. `tile_name` gives the name of a graphic.
pub fn ability_weapon_names(ability: u8, tile_name: impl Fn(u16) -> Option<String>) -> String {
    let mut names: Vec<String> = Vec::new();
    for graphic in weapons_with(ability) {
        let Some(name) = tile_name(graphic) else {
            continue;
        };
        let name = name.trim().to_string();
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
    names.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_names_the_ability_of_the_weapon_in_hand() {
        let frame = WatchFrame::default();
        assert_eq!(
            ability_slot_words(&frame, AbilitySlot::Primary),
            "Primary: Paralyzing Blow"
        );
        assert_eq!(
            ability_slot_words(&frame, AbilitySlot::Secondary),
            "Secondary: Disarm"
        );
        assert_eq!(ability_name_of(200), "200");
    }

    #[test]
    fn each_weapon_name_shows_once() {
        const ARMOR_IGNORE: u8 = 1;
        let same = ability_weapon_names(ARMOR_IGNORE, |_| Some(" axe ".to_string()));
        assert_eq!(same, "axe");
        assert_eq!(ability_weapon_names(ARMOR_IGNORE, |_| None), "");
    }
}
