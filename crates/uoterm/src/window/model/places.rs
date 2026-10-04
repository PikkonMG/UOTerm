//! Where each panel stands: the place and size the player gave it, kept
//! in the profile, held inside the window however the window changed, and
//! whether it is locked, open or folded to its title. The gump places of
//! the profile hold the places, the Interface page the panels that are
//! open, and the folded list the folded ones, as the Classic gumps keep
//! theirs. A panel that shows until the player shuts it, as the journal
//! does, keeps that it is shut among the open panels.

pub use uoterm_view::model::places::*;

use crate::window::settings::{GumpPlace, Profile};
use eframe::egui::{Pos2, Rect, Vec2};

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
}
