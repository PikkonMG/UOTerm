//! The nearby-loot window: where it first stands, its words, what each
//! corpse row says, and the corpses it opens by itself.

use crate::frame::WatchFrame;
use crate::geom::{Area, Vector};
use crate::model::loot::{self, NearbyCorpse};
use crate::settings::Profile;
use crate::ui::layout::{first_place, Spot};
use crate::ui::places::TITLE_ROW;
use crate::ui::theme::PANEL_PAD;
use std::collections::HashSet;

/// The id that keeps the place of the nearby-loot window in the profile.
pub const LOOT_ID: &str = "modern:loot";
const LOOT_WIDTH: f32 = 300.0;
/// The height of a corpse row.
pub const LOOT_ROW: f32 = 26.0;
/// The window shows at most this many corpses.
pub const LOOT_MAX_ROWS: usize = 8;
pub const WORDS_LOOT_TITLE: &str = "Nearby loot";
pub const WORDS_LOOT_OPEN: &str = "Open";
pub const WORDS_LOOT_ONE: &str = "Loot";
pub const WORDS_LOOT_ALL_NEAR: &str = "Loot all in reach";
pub const WORDS_NO_CORPSE: &str = "No corpse near.";
const WORDS_EMPTY: &str = "empty";
const WORDS_SHUT: &str = "shut";

/// Where the nearby-loot window first stands in `window` with `corpses`
/// rows: at the top of the right column, a row for each corpse and one for
/// the button that loots them all.
pub fn loot_first_place(window: Area, corpses: usize) -> Area {
    let rows = corpses.clamp(1, LOOT_MAX_ROWS) + 1;
    let height = TITLE_ROW + rows as f32 * LOOT_ROW + PANEL_PAD * 2.0;
    first_place(
        window,
        Spot::RightColumn(0),
        Vector::new(LOOT_WIDTH, height),
    )
}

/// What a row says of a corpse: shut, empty or how many items it holds,
/// and how far it is.
pub fn corpse_state_words(corpse: &NearbyCorpse) -> String {
    let state = match (corpse.open, corpse.items) {
        (false, _) => WORDS_SHUT.to_string(),
        (true, 0) => WORDS_EMPTY.to_string(),
        (true, items) => items.to_string(),
    };
    format!("{state}  {}", corpse.distance)
}

/// The corpses to open now by the corpse options, each once: `opened`
/// keeps the ones opened before, and forgets those that are gone. It runs
/// whether the window shows or not, while the human has control.
pub fn open_corpses(opened: &mut HashSet<u32>, frame: &WatchFrame, profile: &Profile) -> Vec<u32> {
    opened.retain(|serial| frame.items.iter().any(|item| item.serial == *serial));
    if !frame.human_control {
        return Vec::new();
    }
    let corpses = loot::corpses_to_open(&profile.general, frame, opened);
    opened.extend(corpses.iter().copied());
    corpses
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_tells_whether_the_corpse_is_open_and_how_far_it_is() {
        let corpse = |open, items| NearbyCorpse {
            serial: 1,
            name: "a corpse".into(),
            distance: 3,
            open,
            items,
        };
        assert_eq!(corpse_state_words(&corpse(false, 0)), "shut  3");
        assert_eq!(corpse_state_words(&corpse(true, 0)), "empty  3");
        assert_eq!(corpse_state_words(&corpse(true, 4)), "4  3");
        let window = Area::from_min_size(
            crate::geom::Point::new(0.0, 0.0),
            Vector::new(1600.0, 900.0),
        );
        let few = loot_first_place(window, 1);
        let many = loot_first_place(window, LOOT_MAX_ROWS * 2);
        assert_eq!(
            many.height() - few.height(),
            (LOOT_MAX_ROWS - 1) as f32 * LOOT_ROW
        );
    }

    #[test]
    fn a_corpse_that_is_gone_is_forgotten_and_none_opens_without_control() {
        let mut opened = HashSet::from([9]);
        assert!(open_corpses(&mut opened, &WatchFrame::default(), &Profile::default()).is_empty());
        assert!(opened.is_empty());
    }
}
