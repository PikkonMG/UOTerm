//! Which panels are open, shut or folded, and the places the profile
//! keeps for them. The window holds each panel inside itself; that part
//! stays with the drawing.

use crate::settings::{GumpPlace, Profile};
use std::borrow::Cow;

/// The mark of a shut panel among the open panels, after its id.
const SHUT_SUFFIX: &str = ":shut";

/// The place the player gave a panel in this run or before.
pub fn kept<'a>(profile: &'a Profile, id: &str) -> Option<&'a GumpPlace> {
    profile.gumps.get(id)
}

/// The profile as it goes to its file: with no places and no joined gumps
/// when the Interface page does not keep them, so each panel starts at its
/// first place again.
pub fn for_saving(profile: &Profile) -> Cow<'_, Profile> {
    if profile.interface.remember_gump_places {
        Cow::Borrowed(profile)
    } else {
        let mut kept = profile.clone();
        kept.gumps.clear();
        kept.anchored.clear();
        Cow::Owned(kept)
    }
}

/// True when the player locked the panel.
pub fn is_locked(profile: &Profile, id: &str) -> bool {
    profile.gumps.get(id).is_some_and(|place| place.locked)
}

/// Forgets where a panel was, so it goes back to its first place.
pub fn forget(profile: &mut Profile, id: &str) {
    profile.gumps.remove(id);
}

/// True when the panel is open.
pub fn is_open(profile: &Profile, id: &str) -> bool {
    profile.interface.open_panels.iter().any(|open| open == id)
}

/// Opens or closes a panel. The profile keeps it, so it opens again with
/// the window.
pub fn set_open(profile: &mut Profile, id: &str, open: bool) {
    let panels = &mut profile.interface.open_panels;
    panels.retain(|kept| kept != id);
    if open {
        panels.push(id.to_string());
    }
}

fn shut_key(id: &str) -> String {
    format!("{id}{SHUT_SUFFIX}")
}

/// True when the player shut a panel that shows until he shuts it.
pub fn is_shut(profile: &Profile, id: &str) -> bool {
    is_open(profile, &shut_key(id))
}

/// Shuts a panel that shows until the player shuts it, or shows it again.
pub fn set_shut(profile: &mut Profile, id: &str, shut: bool) {
    set_open(profile, &shut_key(id), shut);
}

/// True when the panel is folded to its title.
pub fn is_folded(profile: &Profile, id: &str) -> bool {
    profile.folded.contains(id)
}

/// Folds a panel to its title, or unfolds it.
pub fn set_folded(profile: &mut Profile, id: &str, folded: bool) {
    if folded {
        profile.folded.insert(id.to_string());
    } else {
        profile.folded.remove(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "journal";

    #[test]
    fn a_shut_panel_and_a_folded_one_are_kept_apart_from_the_open_ones() {
        let mut profile = Profile::default();
        assert!(!is_shut(&profile, ID) && !is_folded(&profile, ID));
        set_shut(&mut profile, ID, true);
        assert!(is_shut(&profile, ID));
        assert!(!is_open(&profile, ID), "shut is not open");
        set_shut(&mut profile, ID, false);
        assert!(!is_shut(&profile, ID));
        set_folded(&mut profile, ID, true);
        assert!(is_folded(&profile, ID));
        set_folded(&mut profile, ID, false);
        assert!(profile.folded.is_empty());
    }

    #[test]
    fn open_panels_are_kept_once() {
        let mut profile = Profile::default();
        set_open(&mut profile, ID, true);
        set_open(&mut profile, ID, true);
        assert!(is_open(&profile, ID));
        assert_eq!(profile.interface.open_panels.len(), 1);
        set_open(&mut profile, ID, false);
        assert!(!is_open(&profile, ID));
    }
}
