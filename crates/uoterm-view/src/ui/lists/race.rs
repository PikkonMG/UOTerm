//! The words and the preview of the race change panel.

use crate::frame::{WatchEquip, WatchLook};
use crate::model::race_change::{doll_body, RacePicks};
use uoterm_protocol::types::{LAYER_BEARD, LAYER_HAIR};
use uoterm_world::{Race, RaceChange};

/// A worn hair or beard of the preview has no serial.
const NO_SERIAL: u32 = 0;
const NO_STYLE: u16 = 0;
pub const WORDS_RACE_CHANGE: &str = "Race change";
const WORDS_HUMAN: &str = "Human";
const WORDS_ELF: &str = "Elf";
const WORDS_GARGOYLE: &str = "Gargoyle";
const WORDS_MAN: &str = "man";
const WORDS_WOMAN: &str = "woman";

/// The words of the race and the sex the shard asks the player to be.
pub fn race_change_words(change: RaceChange) -> String {
    let race = match change.race {
        Race::Human => WORDS_HUMAN,
        Race::Elf => WORDS_ELF,
        Race::Gargoyle => WORDS_GARGOYLE,
    };
    let sex = if change.female {
        WORDS_WOMAN
    } else {
        WORDS_MAN
    };
    format!("{WORDS_RACE_CHANGE}: {race} {sex}")
}

/// The look of the figure with the new looks.
pub fn race_preview_look(change: RaceChange, picks: &RacePicks) -> WatchLook {
    let looks = picks.looks(change);
    let worn = [
        (looks.hair, looks.hair_hue, LAYER_HAIR),
        (looks.beard, looks.beard_hue, LAYER_BEARD),
    ];
    WatchLook {
        body: doll_body(change),
        hue: looks.skin_hue,
        equipment: worn
            .into_iter()
            .filter(|(graphic, ..)| *graphic != NO_STYLE)
            .map(|(graphic, hue, layer)| WatchEquip {
                serial: NO_SERIAL,
                graphic,
                layer,
                hue,
            })
            .collect(),
        ..WatchLook::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_world::BODY_HUMAN_MALE;

    const HUMAN_MAN: RaceChange = RaceChange {
        race: Race::Human,
        female: false,
    };

    #[test]
    fn the_preview_wears_the_picked_hair_and_beard_on_the_new_body() {
        let mut picks = RacePicks::default();
        picks.follow(HUMAN_MAN);
        assert!(
            race_preview_look(HUMAN_MAN, &picks).equipment.is_empty(),
            "bald"
        );
        picks.hair = 1;
        picks.beard = 1;
        let look = race_preview_look(HUMAN_MAN, &picks);
        assert_eq!(look.body, BODY_HUMAN_MALE);
        let layers: Vec<u8> = look.equipment.iter().map(|worn| worn.layer).collect();
        assert_eq!(layers, vec![LAYER_HAIR, LAYER_BEARD]);
        assert_eq!(race_change_words(HUMAN_MAN), "Race change: Human man");
    }
}
