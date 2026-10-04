//! The markers manager of the Modern world map, apart from how it draws:
//! its place, its words, the words of one marker row, the buttons of a
//! row, which only the player's own file may change, and the box that
//! adds or changes one marker. The files are `model::world_map`'s.

use crate::geom::{Area, Vector};
pub use crate::model::world_map::WORDS_INVALID_MARKER as WORDS_BAD_FIELDS;

use crate::model::world_map::{Marker, MarkerChange, MarkerFields, USER_MARKERS};
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

/// The box that adds a marker, or changes one of the player's own file,
/// with the words that tell why its fields do not make a marker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkerBox {
    /// The place in the own file of the marker it changes, and that marker
    /// as it was.
    pub editing: Option<(usize, Marker)>,
    pub fields: MarkerFields,
    pub error: Option<String>,
}

impl MarkerBox {
    /// A box that adds a marker, filled from `marker`.
    pub fn adding(marker: &Marker) -> Self {
        Self {
            editing: None,
            fields: MarkerFields::of(marker),
            error: None,
        }
    }

    /// A box that changes the marker at `at` of the own file.
    pub fn editing(at: usize, marker: &Marker) -> Self {
        Self {
            editing: Some((at, marker.clone())),
            fields: MarkerFields::of(marker),
            error: None,
        }
    }

    pub fn title(&self) -> &'static str {
        if self.editing.is_some() {
            WORDS_EDIT_MARKER
        } else {
            WORDS_ADD
        }
    }

    /// The words of the button that keeps the marker.
    pub fn submit_words(&self) -> &'static str {
        if self.editing.is_some() {
            MarkerButton::Edit.words()
        } else {
            WORDS_CREATE
        }
    }

    /// The change the fields make, when they make a marker; else the box
    /// says why not.
    pub fn submit(&mut self) -> Option<MarkerChange> {
        let Some(marker) = self.fields.marker() else {
            self.error = Some(WORDS_BAD_FIELDS.to_string());
            return None;
        };
        Some(match &self.editing {
            Some((at, expected)) => MarkerChange::Keep {
                at: *at,
                marker,
                expected: expected.clone(),
            },
            None => MarkerChange::Add(marker),
        })
    }
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
    use crate::model::world_map::NEW_MARKER_COLOR;

    #[test]
    fn the_box_adds_or_keeps_a_marker_and_says_why_bad_fields_do_not() {
        let place = Marker {
            name: "Camp".into(),
            map: 0,
            x: 10,
            y: 20,
            icon: String::new(),
            color: NEW_MARKER_COLOR.into(),
        };
        let mut adding = MarkerBox::adding(&place);
        assert_eq!(adding.title(), WORDS_ADD);
        assert_eq!(adding.submit(), Some(MarkerChange::Add(place.clone())));
        let mut editing = MarkerBox::editing(2, &place);
        editing.fields.name = "Mine".into();
        let Some(MarkerChange::Keep {
            at,
            marker,
            expected,
        }) = editing.submit()
        else {
            panic!("a keep");
        };
        assert_eq!((at, marker.name.as_str(), expected), (2, "Mine", place));
        editing.fields.x = "70000".into();
        assert_eq!(editing.submit(), None);
        assert_eq!(editing.error.as_deref(), Some(WORDS_BAD_FIELDS));
    }

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
