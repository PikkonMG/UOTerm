//! The corpses the nearby-loot window opens by itself.

use crate::frame::WatchFrame;
use crate::model::loot;
use crate::settings::Profile;
use std::collections::HashSet;

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
    fn a_corpse_that_is_gone_is_forgotten_and_none_opens_without_control() {
        let mut opened = HashSet::from([9]);
        assert!(open_corpses(&mut opened, &WatchFrame::default(), &Profile::default()).is_empty());
        assert!(opened.is_empty());
    }
}
