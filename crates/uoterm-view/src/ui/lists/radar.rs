//! The size of the radar panel, small or large.

use super::super::places::TITLE_ROW;
use super::super::theme::PANEL_PAD;
use crate::geom::Vector;

/// The radar starts close, and the wheel takes it closer or back to the
/// whole picture round the character.
pub const RADAR_FIRST_ZOOM: f32 = 4.0;
pub const RADAR_ZOOM_MAX: f32 = 16.0;
pub const WORDS_RADAR: &str = "Radar";
pub const WORDS_RADAR_NO_FILES: &str = "The radar needs the client files.";
pub const HINT_RADAR: &str = "Wheel: zoom. Double-click: larger or smaller.";
const SMALL_SIDE: f32 = 190.0;
const LARGE_SIDE: f32 = 320.0;

/// The side of the field for the size the World Map page keeps.
pub fn radar_field_side(large: bool) -> f32 {
    if large {
        LARGE_SIDE
    } else {
        SMALL_SIDE
    }
}

/// The size of the radar panel, large or small.
pub fn radar_panel_size(large: bool) -> Vector {
    let side = radar_field_side(large);
    Vector::new(side + PANEL_PAD * 2.0, side + TITLE_ROW + PANEL_PAD * 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_large_radar_is_larger() {
        assert!(radar_field_side(true) > radar_field_side(false));
        assert!(radar_panel_size(true).x > radar_panel_size(false).x);
    }
}
