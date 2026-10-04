//! The size of the radar panel, small or large.

use super::super::places::TITLE_ROW;
use super::super::theme::PANEL_PAD;
use crate::geom::Vector;

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
