//! The markers manager of the Modern world map, apart from how it draws:
//! its place, its words, the words of one marker row, and the buttons of a
//! row, which only the player's own file may change. The files are
//! `model::world_map`'s.

use crate::geom::{Area, Vector};
use crate::model::world_map::{Marker, USER_MARKERS};
use crate::ui::layout::{first_place, Spot};

pub const MANAGER_ID: &str = "modern:markers";
pub const BOX_ID: &str = "modern:marker_box";
const MANAGER_SIZE: Vector = Vector::new(480.0, 420.0);
const MANAGER_LEAST: Vector = Vector::new(320.0, 240.0);
pub const BOX_SIZE: Vector = Vector::new(320.0, 270.0);
pub const WORDS_MANAGER: &str = "Markers";
pub const WORDS_ADD: &str = "Add marker";
pub const WORDS_EDIT_MARKER: &str = "Edit marker";
pub const WORDS_CREATE: &str = "Create";
pub const WORDS_CANCEL: &str = "Cancel";
pub const WORDS_NO_FILES: &str = "No marker files. Ctrl+click the map to add a marker.";
pub const WORDS_NONE_FOUND: &str = "No marker holds these words.";
pub const WORDS_BAD_FIELDS: &str = "Give a name, and x and y inside the facet.";
pub const WORDS_X: &str = "X";
pub const WORDS_Y: &str = "Y";
pub const WORDS_NAME: &str = "Name";
pub const WORDS_ICON: &str = "Icon";
pub const WORDS_COLOR: &str = "Color";
pub const HINT_SEARCH: &str = "search the markers";
pub const HINT_ICON: &str = "no icon";

/// A button of a marker row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerButton {
    Edit,
    Remove,
    /// Show the place on the map.
    Go,
}

impl MarkerButton {
    pub fn words(self) -> &'static str {
        match self {
            Self::Edit => "Edit",
            Self::Remove => "Remove",
            Self::Go => "Go",
        }
    }
}

/// The buttons of the rows of a file: the player's own file may be changed
/// when `may_change`; every marker may be gone to.
pub fn row_buttons(file: &str, may_change: bool) -> &'static [MarkerButton] {
    if may_change && file == USER_MARKERS {
        &[MarkerButton::Edit, MarkerButton::Remove, MarkerButton::Go]
    } else {
        &[MarkerButton::Go]
    }
}

/// The words of one marker row: its name, its place and its color.
pub fn row_words(marker: &Marker) -> String {
    format!(
        "{}  {}, {}  {}",
        marker.name, marker.x, marker.y, marker.color
    )
}

/// Where the manager first stands in a window, and the least size the
/// human sizes it to.
pub fn manager_first_place(window: Area) -> (Area, Vector) {
    (
        first_place(window, Spot::Middle(0), MANAGER_SIZE),
        MANAGER_LEAST,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_own_file_changes_and_only_where_files_are_written() {
        assert_eq!(row_buttons(USER_MARKERS, true).len(), 3);
        assert_eq!(row_buttons(USER_MARKERS, false), &[MarkerButton::Go]);
        assert_eq!(row_buttons("towns", true), &[MarkerButton::Go]);
        let marker = Marker {
            name: "Camp".into(),
            map: 0,
            x: 1,
            y: 2,
            icon: String::new(),
            color: "red".into(),
        };
        assert_eq!(row_words(&marker), "Camp  1, 2  red");
    }
}
