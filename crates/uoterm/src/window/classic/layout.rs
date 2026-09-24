//! The arithmetic of classic gumps, with no drawing: where the nine parts of
//! a frame go, how a picture is laid side by side, where a dragged gump may
//! rest, where a slider knob sits, and how wide a bar is for a share. The
//! rules follow the reference client.

use eframe::egui::{Pos2, Rect, Vec2};

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
const PERCENT_FULL: i32 = 100;

/// One part of a frame: the gump id to draw and the area it fills, laid
/// side by side when `tiled`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FramePart {
    pub gump: u16,
    pub area: Rect,
    pub tiled: bool,
}

/// The gump id of each frame part, in the order `frame_parts` wants sizes.
pub fn frame_part_ids(first: u16) -> [u16; FRAME_PARTS] {
    FRAME_PART_OFFSETS.map(|offset| first.wrapping_add(offset))
}

/// Where the nine parts of a frame go in `area`, as the classic client puts
/// them. `sizes` are the sizes of the parts in the order of
/// [`frame_part_ids`]; the frame ends at the first part the files lack.
pub fn frame_parts(first: u16, sizes: &[Option<Vec2>; FRAME_PARTS], area: Rect) -> Vec<FramePart> {
    let present = sizes.iter().take_while(|size| size.is_some()).count();
    // The size of each part; a part past the end of the frame takes none.
    let s = |part: usize| sizes[part].filter(|_| part < present).unwrap_or(Vec2::ZERO);
    let (x, y, w, h) = (area.left(), area.top(), area.width(), area.height());
    let offset_top = s(TOP_LEFT).y.max(s(TOP_RIGHT).y) - s(TOP).y;
    let offset_bottom = s(BOTTOM_LEFT).y.max(s(BOTTOM_RIGHT).y) - s(BOTTOM).y;
    let offset_left = (s(TOP_LEFT).x.max(s(BOTTOM_LEFT).x) - s(TOP_RIGHT).x).abs();
    let offset_right = s(TOP_RIGHT).x.max(s(BOTTOM_RIGHT).x) - s(RIGHT).x;
    let rect = |left: f32, top: f32, width: f32, height: f32| {
        Rect::from_min_size(Pos2::new(left, top), Vec2::new(width, height))
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

/// One copy of a picture laid over an area: where it goes, and how much of
/// the picture it shows across and down, from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tile {
    pub area: Rect,
    pub shown: Vec2,
}

/// Lays a picture of `size` side by side over `area`, cut at the far edges.
pub fn tiles(area: Rect, size: Vec2) -> Vec<Tile> {
    let mut out = Vec::new();
    if size.x < 1.0 || size.y < 1.0 {
        return out;
    }
    let mut y = area.top();
    while y < area.bottom() {
        let height = size.y.min(area.bottom() - y);
        let mut x = area.left();
        while x < area.right() {
            let width = size.x.min(area.right() - x);
            out.push(Tile {
                area: Rect::from_min_size(Pos2::new(x, y), Vec2::new(width, height)),
                shown: Vec2::new(width / size.x, height / size.y),
            });
            x += size.x;
        }
        y += size.y;
    }
    out
}

/// Where a gump rests after a drag: at least a quarter of it stays on the
/// screen on each side.
pub fn rest_place(place: Pos2, size: Vec2, screen: Rect) -> Pos2 {
    let kept = size / KEEP_SHARE_DIVISOR;
    let hidden = size - kept;
    Pos2::new(
        place
            .x
            .max(screen.left() - hidden.x)
            .min(screen.right() - kept.x),
        place
            .y
            .max(screen.top() - hidden.y)
            .min(screen.bottom() - kept.y),
    )
}

/// A gump wholly off the screen goes back to its corner, as the classic
/// client does when the window shrinks.
pub fn in_screen(place: Pos2, size: Vec2, screen: Rect) -> Pos2 {
    if Rect::from_min_size(place, size).intersects(screen) {
        place
    } else {
        screen.min
    }
}

/// How wide a bar is for `current` of `max`, in a full width of `width`, as
/// the classic health bars count it: whole percents, never past full.
pub fn bar_width(max: i32, current: i32, width: i32) -> i32 {
    if max <= 0 {
        return max;
    }
    let percent = (current * PERCENT_FULL / max).min(PERCENT_FULL);
    if percent > 1 {
        width * percent / PERCENT_FULL
    } else {
        percent
    }
}

/// Where the knob of a slider sits, from its left end, for a value.
pub fn knob_offset(value: i32, min: i32, max: i32, travel: i32) -> i32 {
    let span = max - min;
    if span <= 0 {
        return 0;
    }
    let share = (value.clamp(min, max) - min) as f32 / span as f32;
    ((travel as f32 * share) as i32).max(0)
}

/// The value a slider takes when the pointer is `along` pixels from its
/// left end.
pub fn knob_value(along: i32, min: i32, max: i32, travel: i32) -> i32 {
    if travel <= 0 {
        return min;
    }
    let share = along as f32 / travel as f32;
    ((max - min) as f32 * share) as i32 + min
}

/// The value of a scroll bar whose knob is `along` pixels down the room it
/// slides in, rounded to the nearest.
pub fn scroll_value(along: i32, min: i32, max: i32, room: i32) -> i32 {
    if room <= 0 {
        return min;
    }
    let along = along.clamp(0, room);
    (along as f32 / room as f32 * (max - min) as f32 + min as f32).round() as i32
}

/// How far down its room the knob of a scroll bar sits for a value.
pub fn scroll_knob(value: i32, min: i32, max: i32, room: i32) -> i32 {
    if max <= min {
        return 0;
    }
    (room as f32 * (value.clamp(min, max) - min) as f32 / (max - min) as f32).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(w: f32, h: f32) -> Option<Vec2> {
        Some(Vec2::new(w, h))
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
        let area = Rect::from_min_size(Pos2::new(100.0, 50.0), Vec2::new(60.0, 40.0));
        let parts = frame_parts(3000, &sizes, area);
        assert_eq!(parts.len(), FRAME_PARTS);
        let part = |gump: u16| *parts.iter().find(|p| p.gump == gump).unwrap();
        assert_eq!(
            part(3000).area,
            Rect::from_min_size(area.min, Vec2::new(10.0, 10.0))
        );
        assert!(!part(3000).tiled);
        assert_eq!(
            part(3001).area,
            Rect::from_min_size(Pos2::new(110.0, 50.0), Vec2::new(40.0, 10.0))
        );
        assert_eq!(
            part(3008).area,
            Rect::from_min_size(Pos2::new(150.0, 80.0), Vec2::new(10.0, 10.0))
        );
        // The middle is the part numbered four, drawn last, inside the frame.
        assert_eq!(parts.last().unwrap().gump, 3004);
        assert_eq!(
            part(3004).area,
            Rect::from_min_size(Pos2::new(110.0, 60.0), Vec2::new(40.0, 20.0))
        );
    }

    #[test]
    fn a_frame_ends_at_its_first_missing_part() {
        let mut sizes = [size(4.0, 4.0); FRAME_PARTS];
        sizes[3] = None;
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(20.0, 20.0));
        let gumps: Vec<u16> = frame_parts(10, &sizes, area)
            .iter()
            .map(|p| p.gump)
            .collect();
        assert_eq!(gumps, vec![10, 11, 12]);
    }

    #[test]
    fn tiles_cover_the_area_and_cut_the_last_ones() {
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::new(25.0, 10.0));
        let laid = tiles(area, Vec2::new(10.0, 10.0));
        assert_eq!(laid.len(), 3);
        assert_eq!(laid[2].area.width(), 5.0);
        assert_eq!(laid[2].shown, Vec2::new(0.5, 1.0));
        assert!(tiles(area, Vec2::ZERO).is_empty());
    }

    #[test]
    fn a_dragged_gump_keeps_a_quarter_on_the_screen() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let size = Vec2::new(200.0, 100.0);
        assert_eq!(
            rest_place(Pos2::new(-500.0, -500.0), size, screen),
            Pos2::new(-150.0, -75.0)
        );
        assert_eq!(
            rest_place(Pos2::new(900.0, 900.0), size, screen),
            Pos2::new(750.0, 575.0)
        );
        assert_eq!(
            rest_place(Pos2::new(10.0, 20.0), size, screen),
            Pos2::new(10.0, 20.0)
        );
    }

    #[test]
    fn a_gump_off_the_screen_goes_to_the_corner() {
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let size = Vec2::new(50.0, 50.0);
        assert_eq!(in_screen(Pos2::new(900.0, 10.0), size, screen), Pos2::ZERO);
        assert_eq!(
            in_screen(Pos2::new(780.0, 10.0), size, screen),
            Pos2::new(780.0, 10.0)
        );
    }

    #[test]
    fn bars_and_sliders_follow_the_classic_rounding() {
        assert_eq!(bar_width(100, 50, 109), 54);
        assert_eq!(bar_width(100, 150, 109), 109);
        assert_eq!(bar_width(100, 1, 109), 1);
        assert_eq!(bar_width(0, 0, 109), 0);
        assert_eq!(knob_offset(50, 0, 100, 200), 100);
        assert_eq!(knob_offset(-5, 0, 100, 200), 0);
        assert_eq!(knob_value(100, 0, 100, 200), 50);
        assert_eq!(knob_value(0, 12, 250, 0), 12);
        assert_eq!(scroll_value(50, 0, 300, 100), 150);
        assert_eq!(scroll_value(500, 0, 300, 100), 300);
        assert_eq!(scroll_knob(150, 0, 300, 100), 50);
        assert_eq!(scroll_knob(10, 0, 0, 100), 0);
    }
}
