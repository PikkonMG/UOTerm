//! The first place of every Modern panel, for egui. The plan of the whole
//! window is `uoterm_view::ui::layout`, which the browser shares.

pub use uoterm_view::ui::layout::Spot;

use crate::window::bridge;
use eframe::egui::{Rect, Vec2};
use uoterm_view::ui::layout;

/// The first place of a panel of `size` at `spot` in `window`.
pub fn first_place(window: Rect, spot: Spot, size: Vec2) -> Rect {
    bridge::rect(layout::first_place(
        bridge::area(window),
        spot,
        bridge::vector(size),
    ))
}
