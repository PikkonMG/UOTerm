//! The frame rules of every Modern panel the player moves: the height of
//! its title, the least width that holds the title and the marks, where
//! the panel stands now, and the part that shows when it is folded. Where
//! a panel is kept in the profile is `model::places`'.

use super::theme::PANEL_PAD;
use crate::geom::{Area, Vector};
use crate::model::places;
use crate::settings::Profile;

pub const TITLE_ROW: f32 = 28.0;
/// A panel is at least this wide, so its title and marks fit.
pub const PANEL_MIN_WIDTH: f32 = 170.0;
/// What the title and the marks of a frame do, as their tips say.
pub const HINT_MOVE: &str = "Drag: move.  Double-click: put it back.";
pub const HINT_LOCK: &str = "Lock or free the panel.";
pub const HINT_CLOSE: &str = "Close.";
pub const HINT_SIZE: &str = "Drag: size.";
pub const HINT_FOLD: &str = "Fold to the title, or unfold.";
/// The wheel over the lines or the map of a panel turns it one notch for
/// this many points of scroll.
pub const PANEL_WHEEL_POINTS: f32 = 50.0;
/// A folded panel shows its title alone.
pub const FOLDED_HEIGHT: f32 = TITLE_ROW + PANEL_PAD * 2.0;

/// A size made wide enough for the title and the marks.
pub fn with_title_room(size: Vector) -> Vector {
    Vector::new(size.x.max(PANEL_MIN_WIDTH), size.y)
}

/// Where the panel `id` stands now: where the player left it, or else at
/// `default`, inside `window`. `min_size` is the least size of a panel
/// the player sizes, and None for a panel of a fixed size.
pub fn place(
    window: Area,
    id: &str,
    default: Area,
    min_size: Option<Vector>,
    profile: &Profile,
) -> Area {
    places::placed_rect(window, default, places::kept(profile, id), min_size)
}

/// The place a panel takes: its title alone when it is folded.
pub fn shown_rect(area: Area, folded: bool) -> Area {
    if folded {
        Area::from_min_size(area.min, Vector::new(area.width(), FOLDED_HEIGHT))
    } else {
        area
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;

    const ID: &str = "test";

    #[test]
    fn a_folded_panel_keeps_its_title() {
        let area = Area::from_min_size(Point::new(10.0, 20.0), Vector::new(300.0, 200.0));
        assert_eq!(shown_rect(area, false), area);
        let folded = shown_rect(area, true);
        assert_eq!((folded.min, folded.width()), (area.min, area.width()));
        assert_eq!(folded.height(), FOLDED_HEIGHT);
    }

    #[test]
    fn a_panel_stands_where_the_player_left_it_and_is_wide_enough_for_its_title() {
        let window = Area::from_min_size(Point::default(), Vector::new(1000.0, 800.0));
        let default = Area::from_min_size(Point::new(10.0, 20.0), Vector::new(300.0, 200.0));
        let mut profile = Profile::default();
        assert_eq!(place(window, ID, default, None, &profile), default);
        let moved = default.translate(Vector::new(100.0, 50.0));
        places::remember(&mut profile, ID, moved, false);
        assert_eq!(place(window, ID, default, None, &profile), moved);
        assert_eq!(
            with_title_room(Vector::new(10.0, 40.0)),
            Vector::new(PANEL_MIN_WIDTH, 40.0)
        );
    }
}
