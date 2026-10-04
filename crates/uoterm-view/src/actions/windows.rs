//! The window commands of the Modern style apart from its windows: whether
//! a window should be open after a command, and which tab of the deck or
//! panel of the profile shows each kind of window.

use super::{GumpKind, GumpOp};
use crate::model::places;
use crate::settings::Profile;

/// A tab of the deck.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Character,
    Skills,
    Spells,
    Party,
}

/// What the character tab shows: the worn items, or the whole status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CharacterView {
    #[default]
    Worn,
    Status,
}

/// Whether a window that is `open` now should be open after `op`.
/// Minimize closes a Modern panel and maximize opens it.
pub fn wanted(op: GumpOp, open: bool) -> bool {
    match op {
        GumpOp::Open | GumpOp::Maximize => true,
        GumpOp::Close | GumpOp::Minimize => false,
        GumpOp::Toggle => !open,
    }
}

/// Opens, shuts, folds or unfolds a panel that shows until the player
/// shuts it, as the journal and the radar do: minimize folds it to its
/// title, and maximize unfolds it.
pub fn shown_panel(op: GumpOp, id: &str, profile: &mut Profile) {
    let shows = !places::is_shut(profile, id);
    match op {
        GumpOp::Open => places::set_shut(profile, id, false),
        GumpOp::Close => places::set_shut(profile, id, true),
        GumpOp::Toggle => places::set_shut(profile, id, shows),
        GumpOp::Minimize | GumpOp::Maximize => {
            places::set_shut(profile, id, false);
            places::set_folded(profile, id, op == GumpOp::Minimize);
        }
    }
}

/// Opens or closes a panel that has only a switch.
pub fn switch_panel(op: GumpOp, open: bool, toggle: impl FnOnce()) {
    if wanted(op, open) != open {
        toggle();
    }
}

/// The tab of the deck that shows a window.
pub fn deck_tab(kind: GumpKind) -> Option<Tab> {
    Some(match kind {
        GumpKind::Skills => Tab::Skills,
        GumpKind::Party => Tab::Party,
        _ => return None,
    })
}

/// The view of the character tab that shows a window.
pub fn character_view(kind: GumpKind) -> Option<CharacterView> {
    Some(match kind {
        GumpKind::Paperdoll => CharacterView::Worn,
        GumpKind::Status => CharacterView::Status,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOURNAL_ID: &str = "journal";
    const RADAR_ID: &str = "radar";

    #[test]
    fn the_journal_and_the_radar_shut_show_fold_and_unfold() {
        let mut profile = Profile::default();
        shown_panel(GumpOp::Toggle, JOURNAL_ID, &mut profile);
        assert!(places::is_shut(&profile, JOURNAL_ID));
        shown_panel(GumpOp::Minimize, JOURNAL_ID, &mut profile);
        assert!(!places::is_shut(&profile, JOURNAL_ID));
        assert!(places::is_folded(&profile, JOURNAL_ID));
        shown_panel(GumpOp::Maximize, JOURNAL_ID, &mut profile);
        assert!(!places::is_folded(&profile, JOURNAL_ID));
        shown_panel(GumpOp::Close, RADAR_ID, &mut profile);
        assert!(places::is_shut(&profile, RADAR_ID));
        shown_panel(GumpOp::Open, RADAR_ID, &mut profile);
        assert!(!places::is_shut(&profile, RADAR_ID));
    }

    #[test]
    fn a_window_op_opens_or_closes_as_the_official_client_does() {
        assert!(wanted(GumpOp::Toggle, false));
        assert!(!wanted(GumpOp::Toggle, true));
        assert!(wanted(GumpOp::Maximize, false));
        assert!(!wanted(GumpOp::Minimize, true));
    }

    #[test]
    fn the_deck_shows_the_character_windows() {
        assert_eq!(deck_tab(GumpKind::Skills), Some(Tab::Skills));
        assert_eq!(deck_tab(GumpKind::Party), Some(Tab::Party));
        assert_eq!(deck_tab(GumpKind::WorldMap), None);
        assert_eq!(
            character_view(GumpKind::Paperdoll),
            Some(CharacterView::Worn)
        );
        assert_eq!(
            character_view(GumpKind::Status),
            Some(CharacterView::Status)
        );
        assert_eq!(character_view(GumpKind::Skills), None);
    }
}
