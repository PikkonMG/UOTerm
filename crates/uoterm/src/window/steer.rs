//! Walking by hand in the window. The rules are `uoterm_view::steer`;
//! here the window reads the keys, the mouse and Shift from egui and sends
//! the acts the rules decide.

pub use uoterm_view::steer::*;

use super::bridge;
use super::control::Hand;
use eframe::egui::{self, Pos2};
use uoterm_view::input::KeyName;

/// Steers the character for one frame. True while the human steers with
/// the mouse, so the map takes no right-click.
pub fn run(
    steer: &mut Steer,
    ui: &egui::Ui,
    character: Pos2,
    mouse_on_map: Option<Pos2>,
    hand: &Hand,
    time: f64,
    movement: &Movement,
) -> bool {
    let (keys_down, input) = ui.input(|i| {
        let keys_down: Vec<KeyName> = i
            .keys_down
            .iter()
            .map(|key| bridge::key_name(*key))
            .collect();
        let input = SteerInput {
            shift: i.modifiers.shift,
            right_down: i.pointer.secondary_down(),
            right_pressed: i.pointer.secondary_pressed(),
            left_pressed: i.pointer.primary_pressed(),
            mouse_way: mouse_on_map.map(|mouse| (bridge::point(character), bridge::point(mouse))),
        };
        (keys_down, input)
    });
    for act in steer.decide(&keys_down, input, time, movement) {
        hand.act(act);
    }
    if steer.walks() {
        ui.ctx().request_repaint();
    }
    steer.by_mouse()
}
