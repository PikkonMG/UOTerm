//! The rules of the mouse pointer over the world: the arrow of the classic
//! client that points the way from the character to the mouse, the cross
//! while the shard waits for a target, the hue of the aura under the mouse,
//! and how far the thing under the mouse is. The window draws them.

use crate::frame::WatchFrame;
use crate::geom::Point;
use crate::steer::way_of;
use uoterm_nav::CursorShape;
use uoterm_protocol::types::{tile_distance, Direction};

const HUE_NEUTRAL: u16 = 0x03B2;
const HUE_HARMFUL: u16 = 0x0023;
const HUE_BENEFICIAL: u16 = 0x005A;
/// What the target does, as the shard's target cursor says.
const TARGET_HARMFUL: u8 = 1;
const TARGET_BENEFICIAL: u8 = 2;

/// The pointer for the mouse at `mouse` while the character stands at
/// `character` on the screen.
pub fn cursor_shape(target: bool, character: Point, mouse: Point) -> CursorShape {
    if target {
        return CursorShape::Target;
    }
    Direction::from_name(way_of(mouse - character)).map_or(CursorShape::Normal, CursorShape::Walk)
}

/// The hue of the aura under the mouse for what the target does.
pub fn aura_hue(target_flags: u8) -> u16 {
    match target_flags {
        TARGET_HARMFUL => HUE_HARMFUL,
        TARGET_BENEFICIAL => HUE_BENEFICIAL,
        _ => HUE_NEUTRAL,
    }
}

/// How far a mobile or an item in view is from the character, in tiles.
pub fn distance_to(frame: &WatchFrame, serial: u32) -> Option<u32> {
    let mobile = frame.mobiles.iter().find(|mobile| mobile.serial == serial);
    let place = mobile.map(|mobile| (mobile.x, mobile.y)).or_else(|| {
        frame
            .items
            .iter()
            .find(|item| item.serial == serial)
            .map(|item| (item.x, item.y))
    })?;
    Some(tile_distance(place, (frame.x, frame.y)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchItem, WatchMobile};
    use crate::geom::Vector;

    #[test]
    fn the_aura_takes_the_hue_of_what_the_target_does() {
        assert_eq!(aura_hue(0), HUE_NEUTRAL);
        assert_eq!(aura_hue(TARGET_HARMFUL), HUE_HARMFUL);
        assert_eq!(aura_hue(TARGET_BENEFICIAL), HUE_BENEFICIAL);
    }

    #[test]
    fn the_distance_is_to_a_mobile_or_an_item_in_view() {
        let frame = WatchFrame {
            x: 100,
            y: 100,
            mobiles: vec![WatchMobile {
                serial: 5,
                x: 103,
                y: 98,
                ..WatchMobile::default()
            }],
            items: vec![WatchItem {
                serial: 9,
                x: 100,
                y: 107,
                ..WatchItem::default()
            }],
            ..WatchFrame::default()
        };
        assert_eq!(distance_to(&frame, 5), Some(3));
        assert_eq!(distance_to(&frame, 9), Some(7));
        assert_eq!(distance_to(&frame, 1), None);
    }

    const CHARACTER: Point = Point::new(400.0, 300.0);

    #[test]
    fn the_arrow_points_from_the_character_to_the_mouse() {
        // The map is turned by an eighth: up the screen is north-west.
        let up = CHARACTER - Vector::new(0.0, 100.0);
        assert_eq!(
            cursor_shape(false, CHARACTER, up),
            CursorShape::Walk(Direction::Northwest)
        );
        let right = CHARACTER + Vector::new(100.0, 0.0);
        assert_eq!(
            cursor_shape(false, CHARACTER, right),
            CursorShape::Walk(Direction::Northeast)
        );
        let down_right = CHARACTER + Vector::new(100.0, 100.0);
        assert_eq!(
            cursor_shape(false, CHARACTER, down_right),
            CursorShape::Walk(Direction::East)
        );
    }

    #[test]
    fn a_target_request_shows_the_cross() {
        assert_eq!(
            cursor_shape(true, CHARACTER, CHARACTER),
            CursorShape::Target
        );
    }
}
