//! The game window of the Classic style, as the reference client has it: the world is drawn inside a frame of gump art
//! that the player moves by its edge and resizes by the grip at its corner.
//! Gumps may sit outside it. The Video page keeps its place and its size,
//! can lock it, and can make it always fill the window.
//!
//! The place and the size are those of the world inside the frame, in
//! window points.

use super::canvas::{ButtonArt, Canvas};
use crate::window::settings::VideoOptions;
use eframe::egui::{Id, Pos2, Rect, Sense, Vec2};

/// The frame lies this far outside the world on each side.
pub const BORDER: f32 = 5.0;
/// The art of the frame is this thick.
const EDGE: i32 = 4;
const EDGE_ACROSS: u16 = 0x0A8C;
const EDGE_DOWN: u16 = 0x0A8D;
const GRIP: ButtonArt = ButtonArt::new(0x0837, 0x0838, 0x0838);
/// The world is never smaller than this.
pub(super) const LEAST_SIZE: Vec2 = Vec2::new(640.0, 480.0);
const HALF: f32 = 2.0;
const EDGE_KEY: &str = "classic-viewport-edge";

/// Where the world is drawn this frame: the whole window when the Video
/// page asks for a full-size game window, else the place and the size the
/// page keeps, held inside the window.
pub fn view_rect(video: &VideoOptions, screen: Rect) -> Rect {
    if video.game_window_full_size {
        return screen;
    }
    let most = (screen.size() - Vec2::splat(BORDER)).max(Vec2::ZERO);
    let size = Vec2::new(video.game_window_width, video.game_window_height)
        .max(LEAST_SIZE)
        .min(most);
    let corner = Pos2::new(video.game_window_x, video.game_window_y);
    Rect::from_min_size(held_in(corner, size, screen), size)
}

/// A corner that keeps the world inside the window, with its frame allowed
/// off the edge, as the classic game window stops when dropped.
fn held_in(corner: Pos2, size: Vec2, screen: Rect) -> Pos2 {
    let low = screen.min;
    let high = (screen.max - size).max(low);
    corner.clamp(low, high)
}

/// Draws the frame round the world, and moves or resizes the game window
/// when the player drags its edge or its grip, unless the Video page locks
/// it. True when the player let go of a drag, so the profile is kept.
pub fn frame(g: &mut Canvas<'_>, view: Rect, screen: Rect, video: &mut VideoOptions) -> bool {
    if video.game_window_full_size {
        return false;
    }
    let size = view.size() + Vec2::splat(BORDER * HALF);
    let (w, h) = (size.x as i32, size.y as i32);
    g.pic_tiled(0, 0, w, EDGE, EDGE_ACROSS, 0);
    g.pic_tiled(0, h - EDGE, w, EDGE, EDGE_ACROSS, 0);
    g.pic_tiled(0, 0, EDGE, h, EDGE_DOWN, 0);
    let down = g
        .gump_size(EDGE_DOWN)
        .map_or(0, |art| (art.x / HALF) as i32);
    g.pic_tiled(w - EDGE, down, EDGE, h - EDGE, EDGE_DOWN, 0);
    let grip = g.gump_size(GRIP.normal).unwrap_or(Vec2::ZERO);
    let grip_at = (w - (grip.x / HALF) as i32, h - (grip.y / HALF) as i32);
    if video.game_window_locked {
        g.pic(grip_at.0, grip_at.1, GRIP.normal, 0);
        return false;
    }
    let outer = view.expand(BORDER);
    let strips = [
        Rect::from_min_max(outer.min, Pos2::new(outer.max.x, view.min.y)),
        Rect::from_min_max(Pos2::new(outer.min.x, view.max.y), outer.max),
        Rect::from_min_max(outer.min, Pos2::new(view.min.x, outer.max.y)),
        Rect::from_min_max(Pos2::new(view.max.x, outer.min.y), outer.max),
    ];
    let mut stopped = false;
    let mut moved = Vec2::ZERO;
    for (at, strip) in strips.into_iter().enumerate() {
        let response = g
            .ui()
            .interact(strip, Id::new((EDGE_KEY, at)), Sense::drag());
        moved += response.drag_delta();
        stopped |= response.drag_stopped();
    }
    if moved != Vec2::ZERO {
        let corner = held_in(view.min + moved, view.size(), screen);
        video.game_window_x = corner.x;
        video.game_window_y = corner.y;
    }
    let grip_area = g.area(grip_at.0, grip_at.1, grip);
    let response = g
        .ui()
        .interact(grip_area, Id::new((EDGE_KEY, "grip")), Sense::drag());
    let art = if response.is_pointer_button_down_on() {
        GRIP.pressed
    } else {
        GRIP.normal
    };
    g.pic(grip_at.0, grip_at.1, art, 0);
    if response.dragged() {
        let most = (screen.max - view.min - Vec2::splat(BORDER)).max(LEAST_SIZE);
        let grown = (view.size() + response.drag_delta())
            .max(LEAST_SIZE)
            .min(most);
        video.game_window_width = grown.x;
        video.game_window_height = grown.y;
    }
    stopped || response.drag_stopped()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))
    }

    #[test]
    fn the_world_takes_the_kept_place_held_inside_the_window() {
        let video = VideoOptions {
            game_window_x: 20.0,
            game_window_y: 30.0,
            game_window_width: 700.0,
            game_window_height: 500.0,
            ..VideoOptions::default()
        };
        let view = view_rect(&video, screen());
        assert_eq!(view.min, Pos2::new(20.0, 30.0));
        assert_eq!(view.size(), Vec2::new(700.0, 500.0));
        let far = VideoOptions {
            game_window_x: 5000.0,
            ..video.clone()
        };
        assert_eq!(view_rect(&far, screen()).max.x, screen().max.x);
        let small = VideoOptions {
            game_window_width: 100.0,
            game_window_height: 100.0,
            ..video.clone()
        };
        assert_eq!(view_rect(&small, screen()).size(), LEAST_SIZE);
        let full = VideoOptions {
            game_window_full_size: true,
            ..video
        };
        assert_eq!(view_rect(&full, screen()), screen());
    }

    #[test]
    fn a_window_smaller_than_the_least_size_holds_the_world_inside() {
        let tiny = Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 400.0));
        let view = view_rect(&VideoOptions::default(), tiny);
        assert!(tiny.expand(BORDER).contains_rect(view));
    }
}
