//! The arithmetic of classic gumps, with no drawing: how a picture is laid
//! side by side, where a gump off the screen goes, where a slider knob
//! sits, and how wide a bar is for a share. The rules follow the reference
//! client. Where the nine parts of a frame go and where a dragged gump
//! rests are `uoterm_view::ui::gump_frame`, which the browser shares.

use eframe::egui::{Pos2, Rect, Vec2};

const PERCENT_FULL: i32 = 100;

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
