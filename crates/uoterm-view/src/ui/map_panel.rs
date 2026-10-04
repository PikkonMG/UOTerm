//! The world map panel of the Modern style, apart from how it draws: its
//! place and size, its words, how its field lays the land near the
//! character or the whole facet, the wheel, the go-to box, the buttons of
//! the markers, and what a click on the field does. The land and the marks
//! are `map_lay`'s. The Rust window draws the panel with egui and the
//! browser with its page.

use crate::act::Act;
use crate::frame::WatchFrame;
use crate::geom::{Area, Vector};
use crate::map_lay::{whole_tile, zoomed, Lay, SPAN};
use crate::model::world_map::{self, Marker, NEW_MARKER_COLOR};
use crate::settings::WorldMapOptions;
use crate::ui::layout::{first_place, Spot};
use crate::ui::places::TITLE_ROW;
use crate::ui::theme::PANEL_PAD;

pub const MAP_ID: &str = "modern:map";
/// The smallest the map field is drawn, whatever the size of the window.
const PANEL_SIDE: f32 = 470.0;
/// On a large screen the map grows to this share of the shorter side of the
/// window, so a person who plays at a high resolution can still read it.
const PANEL_SHARE: f32 = 0.7;
/// The smallest the human can make the map field.
const FIELD_SIDE_SMALLEST: f32 = 200.0;
/// The two rows of tools under the title: the coordinates with the go-to
/// box, and the marker buttons.
pub const TOOL_ROW: f32 = 26.0;
const TOOL_ROWS: f32 = 2.0;
pub const TOOL_GAP: f32 = 6.0;
/// Above the least zoom the land near the character comes closer and
/// shows fewer tiles.
pub const ZOOM_MAX: f32 = 8.0;
const HALF: f32 = 2.0;
/// The words under the field show this long, in seconds.
pub const NOTE_SECONDS: f64 = 5.0;

pub const WORDS_TITLE: &str = "Map";
pub const WORDS_NO_FILES: &str = "The map needs the client files.";
pub const WORDS_GO: &str = "Look";
pub const WORDS_WALK: &str = "Walk";
pub const WORDS_WORLD: &str = "World";
pub const WORDS_NEAR: &str = "Near";
pub const WORDS_BUILDING: &str = "Drawing the world...";
pub const WORDS_NO_PLACE: &str = "Give x y, or a sextant place.";
pub const WORDS_RELOADED: &str = "The marker and zone files were read again.";
pub const HINT_GOTO: &str = "x y or 12o 34'N, 56o 7'E";
const HINT_WALK: &str = "Click: walk there. Ctrl+click: mark it. Wheel: zoom.";
const HINT_TARGET: &str = "Click: target the ground there.";
const HINT_ZOOM: &str = "Wheel: zoom.";
const HINT_PAN: &str = "Drag: move the view.";

/// A button of the row of the markers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapButton {
    /// Draw the land of the map again.
    Redraw,
    /// Read the marker and zone files again.
    Reload,
    /// Mark the place where the character stands.
    MarkMe,
    /// Open or close the markers manager.
    Markers,
}

impl MapButton {
    pub fn words(self) -> &'static str {
        match self {
            Self::Redraw => "Redraw",
            Self::Reload => "Reload",
            Self::MarkMe => "Mark me",
            Self::Markers => "Markers",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::Redraw => "Draw the land of the map again.",
            Self::Reload => "Read the marker and zone files again.",
            Self::MarkMe => "Mark the place where the character stands.",
            Self::Markers => "List, find, change and go to the markers.",
        }
    }
}

/// The buttons of the row of the markers, from the right.
pub const MAP_BUTTONS: [MapButton; 4] = [
    MapButton::Redraw,
    MapButton::Reload,
    MapButton::MarkMe,
    MapButton::Markers,
];

/// How wide one side of the map field is in a window of this size.
pub fn field_side(window: Area) -> f32 {
    let room = window.height().min(window.width()) * PANEL_SHARE - TITLE_ROW - PANEL_PAD * 2.0;
    room.max(PANEL_SIDE)
}

/// The size of the panel round a field of this side.
pub fn panel_size(side: f32) -> Vector {
    Vector::new(
        side + PANEL_PAD * 2.0,
        side + TITLE_ROW + (TOOL_ROW + TOOL_GAP) * TOOL_ROWS + PANEL_PAD * 2.0,
    )
}

/// Where the map first stands in a window, and the least size the human
/// sizes it to.
pub fn map_first_place(window: Area) -> (Area, Vector) {
    let size = panel_size(field_side(window));
    (
        first_place(window, Spot::Middle(0), size),
        panel_size(FIELD_SIDE_SMALLEST),
    )
}

/// The field of the map in the body of its panel, under the two tool rows.
pub fn field_of(body: Area) -> Area {
    let top = body.min.y + (TOOL_ROW + TOOL_GAP) * TOOL_ROWS;
    Area::from_two_points(crate::geom::Point::new(body.min.x, top), body.max)
}

/// The whole turns of the wheel in `wheel`, taken out of it.
pub fn whole_turns(wheel: &mut f32) -> i32 {
    let turns = wheel.trunc();
    *wheel -= turns;
    turns as i32
}

/// The words of the button that turns between the near view and the
/// whole world.
pub fn view_words(whole_world: bool) -> &'static str {
    if whole_world {
        WORDS_NEAR
    } else {
        WORDS_WORLD
    }
}

/// The view near the character, turned as the play field: the turned
/// picture is a diamond as wide as the field, and the zoom spreads it
/// wider than that.
pub fn near_lay(field: Area, frame: &WatchFrame, zoom: f32) -> Lay {
    Lay::Turned {
        center: field.center(),
        from: Vector::new(f32::from(frame.x), f32::from(frame.y)),
        unit: field.width().min(field.height()) / (SPAN as f32 * HALF) * zoom,
    }
}

/// The zoom of the near view after `notches` of the wheel.
pub fn near_zoom(zoom: f32, notches: f32) -> f32 {
    zoomed(zoom, notches, ZOOM_MAX)
}

/// The whole facet, north up, at the zoom step of the World Map page, and
/// never smaller than the field. The whole facet fits at the least zoom;
/// closer, the view follows the character, or stays where the player moved
/// it.
pub fn world_lay(
    field: Area,
    frame: &WatchFrame,
    options: &WorldMapOptions,
    looking_at: Option<(u16, u16)>,
) -> Lay {
    let (width, height) = world_map::facet_size(frame.map);
    let fit = (field.width() / f32::from(width)).min(field.height() / f32::from(height));
    let scale = world_map::zoom_points(options.zoom_step).max(fit);
    let middle = match looking_at {
        Some((x, y)) => Vector::new(f32::from(x), f32::from(y)),
        None if scale <= fit => Vector::new(f32::from(width), f32::from(height)) / HALF,
        None => Vector::new(f32::from(frame.x), f32::from(frame.y)),
    };
    Lay::NorthUp {
        center: field.center(),
        middle,
        scale,
    }
}

/// Where the whole-world view looks after the player dragged it by
/// `dragged` points, when the World Map page lets the view go free.
pub fn dragged_look(lay: Lay, dragged: Vector) -> Option<(u16, u16)> {
    match lay {
        Lay::NorthUp { middle, scale, .. } => Some(whole_tile(middle - dragged / scale)),
        Lay::Turned { .. } => None,
    }
}

/// The zoom step of the whole world after whole turns of the wheel.
pub fn world_zoom(options: &mut WorldMapOptions, turns: i32) -> bool {
    if turns == 0 {
        return false;
    }
    options.zoom_step = world_map::zoom_step(options.zoom_step, turns);
    true
}

/// The tip of the field and the words of a drag, by what a click does.
pub fn field_hints(frame: &WatchFrame, options: &WorldMapOptions) -> (&'static str, &'static str) {
    let pan = if options.whole_world && options.free_view {
        HINT_PAN
    } else {
        ""
    };
    if !frame.human_control {
        return (HINT_ZOOM, pan);
    }
    if world_map::targets_ground(frame, options) {
        (HINT_TARGET, pan)
    } else {
        (HINT_WALK, pan)
    }
}

/// What a click on the field does.
#[derive(Clone, Debug, PartialEq)]
pub enum FieldClick {
    /// Ctrl+click opens the box that marks the tile.
    Mark(Marker),
    Act(Act),
}

/// A click on tile `x`, `y` of the field, with Ctrl down or not: it walks
/// the character there, or targets the ground when the World Map page
/// lets it, at the height of the land `land_z` gives. Nothing without
/// control.
pub fn field_click(
    frame: &WatchFrame,
    options: &WorldMapOptions,
    (x, y): (u16, u16),
    ctrl: bool,
    land_z: impl FnOnce() -> Option<i8>,
) -> Option<FieldClick> {
    if !frame.human_control {
        return None;
    }
    if ctrl {
        return Some(FieldClick::Mark(Marker {
            name: String::new(),
            map: frame.map,
            x,
            y,
            icon: String::new(),
            color: NEW_MARKER_COLOR.to_string(),
        }));
    }
    let act = if world_map::targets_ground(frame, options) {
        let z = land_z().unwrap_or(frame.z);
        Act::TargetGround { x, y, z }
    } else {
        Act::WalkTo { x, y }
    };
    Some(FieldClick::Act(act))
}

/// What the map shows: open or not, and the place the whole-world view
/// looks at, when the player moved it or looked for a place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MapLook {
    pub open: bool,
    pub looking_at: Option<(u16, u16)>,
}

impl MapLook {
    /// Opens the whole-world view on a place. True when the World Map
    /// page changed and the profile is to be kept.
    pub fn look_at(&mut self, place: (u16, u16), options: &mut WorldMapOptions) -> bool {
        self.open = true;
        self.looking_at = Some(place);
        !std::mem::replace(&mut options.whole_world, true)
    }
}

/// The place the go-to box names, or the words that say it named none.
pub fn goto_place(words: &str) -> Result<(u16, u16), &'static str> {
    world_map::parse_goto(words).ok_or(WORDS_NO_PLACE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;

    #[test]
    fn the_wheel_steps_the_world_zoom_by_whole_turns() {
        let mut wheel = 0.6;
        assert_eq!(whole_turns(&mut wheel), 0, "not yet a whole turn");
        wheel += 0.6;
        assert_eq!(whole_turns(&mut wheel), 1);
        assert!((wheel - 0.2).abs() < 0.001, "the rest waits for the next");
        wheel = -2.5;
        assert_eq!(whole_turns(&mut wheel), -2);
    }

    #[test]
    fn looking_at_a_place_opens_the_whole_world_once() {
        let mut look = MapLook::default();
        let mut options = WorldMapOptions {
            whole_world: false,
            ..WorldMapOptions::default()
        };
        assert!(look.look_at((5, 6), &mut options));
        assert!(look.open && options.whole_world);
        assert_eq!(look.looking_at, Some((5, 6)));
        assert!(!look.look_at((7, 8), &mut options), "already whole");
    }

    #[test]
    fn a_large_window_gets_a_large_map() {
        let area = |w, h| Area::from_min_size(Point::new(0.0, 0.0), Vector::new(w, h));
        let small = area(900.0, 600.0);
        assert_eq!(field_side(small), PANEL_SIDE);
        let large = area(2560.0, 1440.0);
        assert!(field_side(large) > PANEL_SIDE, "a big screen shows more");
        assert!(field_side(large) < large.height(), "it stays in the window");
    }

    #[test]
    fn a_click_walks_targets_or_marks_and_needs_control() {
        let mut frame = WatchFrame {
            human_control: true,
            map: 1,
            ..WatchFrame::default()
        };
        let options = WorldMapOptions::default();
        assert_eq!(
            field_click(&frame, &options, (5, 6), false, || Some(3)),
            Some(FieldClick::Act(Act::WalkTo { x: 5, y: 6 }))
        );
        let Some(FieldClick::Mark(marker)) = field_click(&frame, &options, (5, 6), true, || None)
        else {
            panic!("Ctrl marks");
        };
        assert_eq!((marker.map, marker.x, marker.y), (1, 5, 6));
        frame.human_control = false;
        assert_eq!(field_click(&frame, &options, (5, 6), false, || None), None);
        assert_eq!(field_hints(&frame, &options).0, HINT_ZOOM);
    }

    #[test]
    fn the_whole_world_fits_at_the_least_zoom_and_a_drag_moves_it() {
        const HUGE_FIELD: f32 = 1.0e6;
        let field = Area::from_min_size(Point::new(0.0, 0.0), Vector::splat(HUGE_FIELD));
        let frame = WatchFrame::default();
        let options = WorldMapOptions::default();
        let lay = world_lay(field, &frame, &options, None);
        let (width, height) = world_map::facet_size(0);
        let Lay::NorthUp { middle, scale, .. } = lay else {
            panic!("north up");
        };
        assert_eq!(
            middle,
            Vector::new(f32::from(width), f32::from(height)) / HALF
        );
        let moved = dragged_look(lay, Vector::new(scale * 10.0, 0.0)).unwrap();
        assert_eq!(
            moved,
            whole_tile(Vector::new(
                f32::from(width) / HALF - 10.0,
                f32::from(height) / HALF
            ))
        );
        assert_eq!(goto_place("nowhere"), Err(WORDS_NO_PLACE));
    }
}
