//! The arithmetic of classic gumps that both windows share, with no
//! drawing: where the nine parts of a frame go, and where a dragged gump
//! rests. The rules follow the reference client.

use crate::geom::{Area, Point, Vector};

/// A frame is nine pictures with ids in a row.
pub const FRAME_PARTS: usize = 9;
/// The parts in the order the classic client names them: the top row, the
/// left and the right sides, the bottom row, then the middle. Each is the
/// offset from the first id of the frame.
const FRAME_PART_OFFSETS: [u16; FRAME_PARTS] = [0, 1, 2, 3, 5, 6, 7, 8, 4];
const TOP_LEFT: usize = 0;
const TOP: usize = 1;
const TOP_RIGHT: usize = 2;
const LEFT: usize = 3;
const RIGHT: usize = 4;
const BOTTOM_LEFT: usize = 5;
const BOTTOM: usize = 6;
const BOTTOM_RIGHT: usize = 7;
const MIDDLE: usize = 8;
/// A dragged gump keeps at least a quarter of its size on the screen.
const KEEP_SHARE_DIVISOR: f32 = 4.0;

/// One part of a frame: the gump id to draw and the area it fills, laid
/// side by side when `tiled`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FramePart {
    pub gump: u16,
    pub area: Area,
    pub tiled: bool,
}

/// The gump id of each frame part, in the order `frame_parts` wants sizes.
pub fn frame_part_ids(first: u16) -> [u16; FRAME_PARTS] {
    FRAME_PART_OFFSETS.map(|offset| first.wrapping_add(offset))
}

/// Where the nine parts of a frame go in `area`, as the classic client puts
/// them. `sizes` are the sizes of the parts in the order of
/// [`frame_part_ids`]; the frame ends at the first part the files lack.
pub fn frame_parts(
    first: u16,
    sizes: &[Option<Vector>; FRAME_PARTS],
    area: Area,
) -> Vec<FramePart> {
    let present = sizes.iter().take_while(|size| size.is_some()).count();
    // The size of each part; a part past the end of the frame takes none.
    let s = |part: usize| {
        sizes[part]
            .filter(|_| part < present)
            .unwrap_or(Vector::ZERO)
    };
    let (x, y, w, h) = (area.min.x, area.min.y, area.width(), area.height());
    let offset_top = s(TOP_LEFT).y.max(s(TOP_RIGHT).y) - s(TOP).y;
    let offset_bottom = s(BOTTOM_LEFT).y.max(s(BOTTOM_RIGHT).y) - s(BOTTOM).y;
    let offset_left = (s(TOP_LEFT).x.max(s(BOTTOM_LEFT).x) - s(TOP_RIGHT).x).abs();
    let offset_right = s(TOP_RIGHT).x.max(s(BOTTOM_RIGHT).x) - s(RIGHT).x;
    let rect = |left: f32, top: f32, width: f32, height: f32| {
        Area::from_min_size(Point::new(left, top), Vector::new(width, height))
    };
    let areas = [
        (TOP_LEFT, rect(x, y, s(TOP_LEFT).x, s(TOP_LEFT).y), false),
        (
            TOP,
            rect(
                x + s(TOP_LEFT).x,
                y,
                w - s(TOP_LEFT).x - s(TOP_RIGHT).x,
                s(TOP).y,
            ),
            true,
        ),
        (
            TOP_RIGHT,
            rect(
                x + w - s(TOP_RIGHT).x,
                y + offset_top,
                s(TOP_RIGHT).x,
                s(TOP_RIGHT).y,
            ),
            false,
        ),
        (
            LEFT,
            rect(
                x,
                y + s(TOP_LEFT).y,
                s(LEFT).x,
                h - s(TOP_LEFT).y - s(BOTTOM_LEFT).y,
            ),
            true,
        ),
        (
            RIGHT,
            rect(
                x + w - s(RIGHT).x,
                y + s(TOP_RIGHT).y,
                s(RIGHT).x,
                h - s(TOP_RIGHT).y - s(BOTTOM_RIGHT).y,
            ),
            true,
        ),
        (
            BOTTOM_LEFT,
            rect(
                x,
                y + h - s(BOTTOM_LEFT).y,
                s(BOTTOM_LEFT).x,
                s(BOTTOM_LEFT).y,
            ),
            false,
        ),
        (
            BOTTOM,
            rect(
                x + s(BOTTOM_LEFT).x,
                y + h - s(BOTTOM).y - offset_bottom,
                w - s(BOTTOM_LEFT).x - s(BOTTOM_RIGHT).x,
                s(BOTTOM).y,
            ),
            true,
        ),
        (
            BOTTOM_RIGHT,
            rect(
                x + w - s(BOTTOM_RIGHT).x,
                y + h - s(BOTTOM_RIGHT).y,
                s(BOTTOM_RIGHT).x,
                s(BOTTOM_RIGHT).y,
            ),
            false,
        ),
        (
            MIDDLE,
            rect(
                x + s(TOP_LEFT).x,
                y + s(TOP_LEFT).y,
                w - s(TOP_LEFT).x - s(TOP_RIGHT).x + offset_left + offset_right,
                h - s(TOP_RIGHT).y - s(BOTTOM_RIGHT).y,
            ),
            true,
        ),
    ];
    let ids = frame_part_ids(first);
    areas
        .into_iter()
        .filter(|(part, area, _)| *part < present && area.width() > 0.0 && area.height() > 0.0)
        .map(|(part, area, tiled)| FramePart {
            gump: ids[part],
            area,
            tiled,
        })
        .collect()
}

/// Where a gump rests after a drag: at least a quarter of it stays on the
/// screen on each side.
pub fn rest_place(place: Point, size: Vector, screen: Area) -> Point {
    let kept = size / KEEP_SHARE_DIVISOR;
    let hidden = size - kept;
    Point::new(
        place
            .x
            .max(screen.min.x - hidden.x)
            .min(screen.max.x - kept.x),
        place
            .y
            .max(screen.min.y - hidden.y)
            .min(screen.max.y - kept.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(w: f32, h: f32) -> Option<Vector> {
        Some(Vector::new(w, h))
    }

    fn area(x: f32, y: f32, w: f32, h: f32) -> Area {
        Area::from_min_size(Point::new(x, y), Vector::new(w, h))
    }

    #[test]
    fn a_frame_puts_corners_at_their_size_and_tiles_the_rest() {
        let sizes = [
            size(10.0, 10.0),
            size(5.0, 10.0),
            size(10.0, 10.0),
            size(10.0, 5.0),
            size(10.0, 5.0),
            size(10.0, 10.0),
            size(5.0, 10.0),
            size(10.0, 10.0),
            size(8.0, 8.0),
        ];
        let whole = area(100.0, 50.0, 60.0, 40.0);
        let parts = frame_parts(3000, &sizes, whole);
        assert_eq!(parts.len(), FRAME_PARTS);
        let part = |gump: u16| *parts.iter().find(|p| p.gump == gump).unwrap();
        assert_eq!(part(3000).area, area(100.0, 50.0, 10.0, 10.0));
        assert!(!part(3000).tiled);
        assert_eq!(part(3001).area, area(110.0, 50.0, 40.0, 10.0));
        assert_eq!(part(3008).area, area(150.0, 80.0, 10.0, 10.0));
        // The middle is the part numbered four, drawn last, inside the frame.
        assert_eq!(parts.last().unwrap().gump, 3004);
        assert_eq!(part(3004).area, area(110.0, 60.0, 40.0, 20.0));
    }

    #[test]
    fn a_frame_ends_at_its_first_missing_part() {
        let mut sizes = [size(4.0, 4.0); FRAME_PARTS];
        sizes[3] = None;
        let gumps: Vec<u16> = frame_parts(10, &sizes, area(0.0, 0.0, 20.0, 20.0))
            .iter()
            .map(|p| p.gump)
            .collect();
        assert_eq!(gumps, vec![10, 11, 12]);
    }

    #[test]
    fn a_dragged_gump_keeps_a_quarter_on_the_screen() {
        let screen = area(0.0, 0.0, 800.0, 600.0);
        let size = Vector::new(200.0, 100.0);
        assert_eq!(
            rest_place(Point::new(-500.0, -500.0), size, screen),
            Point::new(-150.0, -75.0)
        );
        assert_eq!(
            rest_place(Point::new(900.0, 900.0), size, screen),
            Point::new(750.0, 575.0)
        );
        assert_eq!(
            rest_place(Point::new(10.0, 20.0), size, screen),
            Point::new(10.0, 20.0)
        );
    }
}
