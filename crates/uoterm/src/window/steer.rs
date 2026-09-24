//! Walking by hand: the arrow keys (or W A S D when the General page says
//! so), walk keys of the Macros page, the left stick of a controller, and
//! the right mouse button held on the map. The keys name the way on the
//! screen, so "up" is up the window. While a way is held the window sends
//! a short walk again and again, and it stops the character when the way
//! is let go.
//!
//! As in the official client, a left click while the right button is held
//! keeps the character going toward the mouse after the right button is
//! let go, until the next right press, unless the Experimental page turns
//! that off.

use super::actions::Direction;
use super::control::{Act, Hand};
use super::keys::WalkKeys;
use eframe::egui::{self, Key, Pos2, Vec2};
use std::f32::consts::TAU;

/// How often a held way is sent again.
const SEND_EVERY: f64 = 0.25;
/// The mouse this far from the character makes him run.
const RUN_DISTANCE: f32 = 190.0;
/// A right press this near the character is a click, not a way.
const DEAD_ZONE: f32 = 24.0;
const WAYS: usize = 8;
/// The ways round the screen from the right, clockwise. The map is turned
/// by an eighth, so the right of the screen is north-east.
const SCREEN_WAYS: [&str; WAYS] = ["ne", "e", "se", "s", "sw", "w", "nw", "n"];

/// Everything that walks the character this frame, besides the mouse.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Movement {
    pub keys: WalkKeys,
    /// The walk keys of the Macros page that are held.
    pub held: Vec<Direction>,
    /// The way the left stick of a controller walks, and whether it runs.
    pub pad: Option<(&'static str, bool)>,
    /// Always run is on and allowed now.
    pub always_run: bool,
    /// A left click while the right button is held keeps him going.
    pub auto_move: bool,
    /// A closed door ahead opens as he walks, by the General page's auto
    /// open doors, while he lives.
    pub open_doors: bool,
}

#[derive(Default)]
pub struct Steer {
    /// The way that was sent last, and when.
    held: Option<(&'static str, bool, f64)>,
    /// The character goes on toward the mouse with no button held.
    going_on: bool,
}

/// The way on the map for a way on the screen.
pub fn way_of(on_screen: Vec2) -> &'static str {
    let turn = on_screen.y.atan2(on_screen.x).rem_euclid(TAU) / TAU;
    SCREEN_WAYS[(turn * WAYS as f32).round() as usize % WAYS]
}

/// The way the pressed keys name. Two opposite keys name none.
fn way_of_keys(up: bool, right: bool, down: bool, left: bool) -> Option<&'static str> {
    let on_screen = Vec2::new(
        f32::from(i8::from(right) - i8::from(left)),
        f32::from(i8::from(down) - i8::from(up)),
    );
    (on_screen != Vec2::ZERO).then(|| way_of(on_screen))
}

/// The way and the pace for the right mouse button held at `mouse`.
fn way_of_mouse(character: Pos2, mouse: Pos2) -> Option<(&'static str, bool)> {
    let pull = mouse - character;
    (pull.length() > DEAD_ZONE).then(|| (way_of(pull), pull.length() > RUN_DISTANCE))
}

/// The keys held for each way on the screen: up, right, down, left.
fn keys_down(input: &egui::InputState, keys: WalkKeys) -> [bool; 4] {
    let arrows = [
        Key::ArrowUp,
        Key::ArrowRight,
        Key::ArrowDown,
        Key::ArrowLeft,
    ];
    let letters = [Key::W, Key::D, Key::S, Key::A];
    let mut down = [false; 4];
    for (at, held) in down.iter_mut().enumerate() {
        *held = (keys.arrows && input.key_down(arrows[at]))
            || (keys.wasd && input.key_down(letters[at]));
    }
    down
}

impl Steer {
    /// True while the human steers with the mouse, so the map takes no
    /// right-click.
    pub fn run(
        &mut self,
        ui: &egui::Ui,
        character: Pos2,
        mouse_on_map: Option<Pos2>,
        hand: &Hand,
        time: f64,
        movement: &Movement,
    ) -> bool {
        let (keys, shift, right_down, right_pressed, left_pressed) = ui.input(|i| {
            let [up, right, down, left] = keys_down(i, movement.keys);
            (
                way_of_keys(up, right, down, left),
                i.modifiers.shift,
                i.pointer.secondary_down(),
                i.pointer.secondary_pressed(),
                i.pointer.primary_pressed(),
            )
        });
        if right_pressed {
            self.going_on = false;
        } else if right_down && left_pressed && movement.auto_move {
            self.going_on = true;
        }
        let by_mouse = mouse_on_map
            .filter(|_| right_down || self.going_on)
            .and_then(|mouse| way_of_mouse(character, mouse));
        let held = movement.held.last().map(|way| (way.way(), false));
        let wanted = keys
            .map(|way| (way, shift))
            .or(held)
            .or(movement.pad)
            .or(by_mouse)
            .map(|(way, run)| (way, run || movement.always_run));
        match (wanted, self.held) {
            (Some((way, run)), held) => {
                let due = held.is_none_or(|(held_way, held_run, at)| {
                    held_way != way || held_run != run || time - at >= SEND_EVERY
                });
                if due {
                    hand.act(Act::Step {
                        direction: way,
                        run,
                        open_doors: movement.open_doors,
                    });
                    self.held = Some((way, run, time));
                }
                ui.ctx().request_repaint();
            }
            (None, Some(_)) => {
                hand.act(Act::Stop);
                self.held = None;
            }
            (None, None) => {}
        }
        by_mouse.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keys_name_the_way_on_the_screen() {
        assert_eq!(way_of_keys(true, false, false, false), Some("nw"));
        assert_eq!(way_of_keys(false, true, false, false), Some("ne"));
        assert_eq!(way_of_keys(false, false, true, false), Some("se"));
        assert_eq!(way_of_keys(false, false, false, true), Some("sw"));
        assert_eq!(way_of_keys(true, true, false, false), Some("n"));
        assert_eq!(way_of_keys(false, true, true, false), Some("e"));
        assert_eq!(way_of_keys(true, false, true, false), None);
        assert_eq!(way_of_keys(false, false, false, false), None);
    }

    #[test]
    fn the_mouse_pulls_the_character_and_a_far_mouse_makes_him_run() {
        let character = Pos2::new(400.0, 300.0);
        assert_eq!(
            way_of_mouse(character, character + Vec2::new(5.0, 0.0)),
            None
        );
        assert_eq!(
            way_of_mouse(character, character + Vec2::new(0.0, -60.0)),
            Some(("nw", false))
        );
        assert_eq!(
            way_of_mouse(character, character + Vec2::new(RUN_DISTANCE + 10.0, 0.0)),
            Some(("ne", true))
        );
    }
}
