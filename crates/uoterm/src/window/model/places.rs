//! Where each panel stands: the place and size the player gave it, kept
//! in the profile, held inside the window however the window changed, and
//! whether it is locked, open or folded to its title. The gump places of
//! the profile hold the places, the Interface page the panels that are
//! open, and the folded list the folded ones, as the Classic gumps keep
//! theirs. A panel that shows until the player shuts it, as the journal
//! does, keeps that it is shut among the open panels.

use crate::window::settings::{GumpPlace, Profile};
use eframe::egui::{Pos2, Rect, Vec2};
use std::borrow::Cow;

/// The mark of a shut panel among the open panels, after its id.
const SHUT_SUFFIX: &str = ":shut";

/// The place of a panel now: the kept one, or else `default`, sized by the
/// kept size of a panel the player can size, and held inside `window`.
pub fn placed_rect(
    window: Rect,
    default: Rect,
    kept: Option<&GumpPlace>,
    min_size: Option<Vec2>,
) -> Rect {
    let size = match (kept.and_then(|place| place.size), min_size) {
        (Some((width, height)), Some(min)) => Vec2::new(width, height).max(min),
        _ => default.size(),
    };
    let left_top = kept.map_or(default.min, |place| Pos2::new(place.x, place.y));
    held_inside(Rect::from_min_size(left_top, size), window)
}

/// `rect` moved, and made smaller when it must be, so it lies inside
/// `room`.
pub fn held_inside(rect: Rect, room: Rect) -> Rect {
    let size = rect.size().min(room.size());
    let left = rect
        .left()
        .clamp(room.left(), (room.right() - size.x).max(room.left()));
    let top = rect
        .top()
        .clamp(room.top(), (room.bottom() - size.y).max(room.top()));
    Rect::from_min_size(Pos2::new(left, top), size)
}

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

/// Keeps where a panel is and its size, with its lock.
pub fn remember(profile: &mut Profile, id: &str, rect: Rect, sized: bool) {
    let locked = is_locked(profile, id);
    profile.gumps.insert(
        id.to_string(),
        GumpPlace {
            x: rect.left(),
            y: rect.top(),
            size: sized.then(|| (rect.width(), rect.height())),
            locked,
        },
    );
}

/// Locks a panel where it is, or frees it.
pub fn set_locked(profile: &mut Profile, id: &str, rect: Rect, sized: bool, locked: bool) {
    remember(profile, id, rect, sized);
    if let Some(place) = profile.gumps.get_mut(id) {
        place.locked = locked;
    }
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
    use crate::window::settings::AnchorCell;

    const ID: &str = "journal";

    fn window() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0))
    }

    fn default() -> Rect {
        Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(300.0, 200.0))
    }

    #[test]
    fn a_panel_stands_where_it_was_kept_and_inside_the_window() {
        let mut profile = Profile::default();
        assert_eq!(
            placed_rect(window(), default(), kept(&profile, ID), None),
            default()
        );
        let moved = Rect::from_min_size(Pos2::new(900.0, 700.0), Vec2::new(400.0, 300.0));
        remember(&mut profile, ID, moved, true);
        let min = Some(Vec2::splat(100.0));
        let placed = placed_rect(window(), default(), kept(&profile, ID), min);
        assert_eq!(
            placed.size(),
            Vec2::new(400.0, 300.0),
            "a sized panel keeps its size"
        );
        assert!(window().contains_rect(placed), "and stays in the window");
        let fixed = placed_rect(window(), default(), kept(&profile, ID), None);
        assert_eq!(
            fixed.size(),
            default().size(),
            "a fixed panel keeps its own size"
        );
        assert!(for_saving(&profile).gumps.contains_key(ID));
        profile.anchored.insert(
            ID.into(),
            AnchorCell {
                group: 1,
                column: 0,
                row: 0,
            },
        );
        assert!(!for_saving(&profile).anchored.is_empty());
        profile.interface.remember_gump_places = false;
        assert!(kept(&profile, ID).is_some(), "the place stays for this run");
        assert!(for_saving(&profile).gumps.is_empty(), "but it is not kept");
        assert!(for_saving(&profile).anchored.is_empty(), "nor its group");
    }

    #[test]
    fn a_lock_stays_with_the_place_and_forgetting_clears_both() {
        let mut profile = Profile::default();
        set_locked(&mut profile, ID, default(), false, true);
        assert!(is_locked(&profile, ID));
        remember(&mut profile, ID, default(), false);
        assert!(is_locked(&profile, ID), "a move keeps the lock");
        forget(&mut profile, ID);
        assert!(!is_locked(&profile, ID) && profile.gumps.is_empty());
    }

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
