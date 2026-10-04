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

use crate::act::Act;
use crate::actions::Direction;
use crate::geom::{Point, Vector};
use crate::input::KeyName;
use crate::keys::WalkKeys;
use std::f32::consts::TAU;

/// How often a held way is sent again.
pub const SEND_EVERY: f64 = 0.25;
/// The mouse this far from the character makes him run.
pub const RUN_DISTANCE: f32 = 190.0;
/// A right press this near the character is a click, not a way.
pub const DEAD_ZONE: f32 = 24.0;
const WAYS: usize = 8;
/// The ways round the screen from the right, clockwise. The map is turned
/// by an eighth, so the right of the screen is north-east.
const SCREEN_WAYS: [&str; WAYS] = ["ne", "e", "se", "s", "sw", "w", "nw", "n"];
/// The keys of each way on the screen: up, right, down, left, by the
/// names egui gives them.
pub const ARROW_KEYS: [&str; 4] = ["Up", "Right", "Down", "Left"];
pub const LETTER_KEYS: [&str; 4] = ["W", "D", "S", "A"];

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

/// The mouse and Shift as the steering reads them in one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SteerInput {
    /// Shift is held: the keys run.
    pub shift: bool,
    pub right_down: bool,
    pub right_pressed: bool,
    pub left_pressed: bool,
    /// The character and the mouse on the screen, while the mouse is on
    /// the map.
    pub mouse_way: Option<(Point, Point)>,
}

#[derive(Default)]
pub struct Steer {
    /// The way that was sent last, and when.
    held: Option<(&'static str, bool, f64)>,
    /// The character goes on toward the mouse with no button held.
    going_on: bool,
    /// The mouse steered in the last decision.
    by_mouse: bool,
}

/// The way on the map for a way on the screen.
pub fn way_of(on_screen: Vector) -> &'static str {
    let turn = on_screen.y.atan2(on_screen.x).rem_euclid(TAU) / TAU;
    SCREEN_WAYS[(turn * WAYS as f32).round() as usize % WAYS]
}

/// The way the pressed keys name. Two opposite keys name none.
pub fn way_of_keys(up: bool, right: bool, down: bool, left: bool) -> Option<&'static str> {
    let on_screen = Vector::new(
        f32::from(i8::from(right) - i8::from(left)),
        f32::from(i8::from(down) - i8::from(up)),
    );
    (on_screen != Vector::ZERO).then(|| way_of(on_screen))
}

/// The way and the pace for the right mouse button held at `mouse`.
pub fn way_of_mouse(character: Point, mouse: Point) -> Option<(&'static str, bool)> {
    let pull = mouse - character;
    (pull.length() > DEAD_ZONE).then(|| (way_of(pull), pull.length() > RUN_DISTANCE))
}

/// The keys held for each way on the screen: up, right, down, left.
fn ways_held(keys_down: &[KeyName], keys: WalkKeys) -> [bool; 4] {
    let is_down = |name: &str| keys_down.iter().any(|key| key.0 == name);
    let mut down = [false; 4];
    for (at, held) in down.iter_mut().enumerate() {
        *held = (keys.arrows && is_down(ARROW_KEYS[at])) || (keys.wasd && is_down(LETTER_KEYS[at]));
    }
    down
}

impl Steer {
    /// The acts of one frame: a step when a way is held and one is due, a
    /// stop when the way is let go. `keys_down` are the keys held now.
    pub fn decide(
        &mut self,
        keys_down: &[KeyName],
        input: SteerInput,
        time: f64,
        movement: &Movement,
    ) -> Vec<Act> {
        let [up, right, down, left] = ways_held(keys_down, movement.keys);
        let keys = way_of_keys(up, right, down, left);
        if input.right_pressed {
            self.going_on = false;
        } else if input.right_down && input.left_pressed && movement.auto_move {
            self.going_on = true;
        }
        let by_mouse = input
            .mouse_way
            .filter(|_| input.right_down || self.going_on)
            .and_then(|(character, mouse)| way_of_mouse(character, mouse));
        self.by_mouse = by_mouse.is_some();
        let held = movement.held.last().map(|way| (way.way(), false));
        let wanted = keys
            .map(|way| (way, input.shift))
            .or(held)
            .or(movement.pad)
            .or(by_mouse)
            .map(|(way, run)| (way, run || movement.always_run));
        match (wanted, self.held) {
            (Some((way, run)), held) => {
                let due = held.is_none_or(|(held_way, held_run, at)| {
                    held_way != way || held_run != run || time - at >= SEND_EVERY
                });
                if !due {
                    return Vec::new();
                }
                self.held = Some((way, run, time));
                vec![Act::Step {
                    direction: way,
                    run,
                    open_doors: movement.open_doors,
                }]
            }
            (None, Some(_)) => {
                self.held = None;
                vec![Act::Stop]
            }
            (None, None) => Vec::new(),
        }
    }

    /// True while the human steers with the mouse, so the map takes no
    /// right-click.
    pub fn by_mouse(&self) -> bool {
        self.by_mouse
    }

    /// True while a way is held, so the window draws again soon.
    pub fn walks(&self) -> bool {
        self.held.is_some()
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
        let character = Point::new(400.0, 300.0);
        assert_eq!(
            way_of_mouse(character, character + Vector::new(5.0, 0.0)),
            None
        );
        assert_eq!(
            way_of_mouse(character, character + Vector::new(0.0, -60.0)),
            Some(("nw", false))
        );
        assert_eq!(
            way_of_mouse(character, character + Vector::new(RUN_DISTANCE + 10.0, 0.0)),
            Some(("ne", true))
        );
    }

    #[test]
    fn a_left_click_with_the_right_button_held_keeps_him_going() {
        let mut steer = Steer::default();
        let movement = Movement {
            auto_move: true,
            ..Movement::default()
        };
        let character = Point::new(400.0, 300.0);
        let far = character + Vector::new(0.0, -60.0);
        let both = SteerInput {
            right_down: true,
            left_pressed: true,
            mouse_way: Some((character, far)),
            ..SteerInput::default()
        };
        assert_eq!(steer.decide(&[], both, 0.0, &movement).len(), 1);
        assert!(steer.by_mouse());
        let let_go = SteerInput {
            mouse_way: Some((character, far)),
            ..SteerInput::default()
        };
        assert!(steer.decide(&[], let_go, SEND_EVERY, &movement).len() == 1);
        assert!(steer.by_mouse() && steer.walks(), "he goes on");
        let right = SteerInput {
            right_pressed: true,
            ..let_go
        };
        steer.decide(&[], right, SEND_EVERY * 2.0, &movement);
        assert!(!steer.by_mouse(), "a right press ends it");
    }
}

#[cfg(test)]
mod decide_tests {
    use super::*;
    use crate::input::KeyName;
    use crate::keys::WalkKeys;

    #[test]
    fn a_held_arrow_steps_then_waits_for_the_send_gap() {
        let mut steer = Steer::default();
        let movement = Movement {
            keys: WalkKeys {
                arrows: true,
                wasd: false,
            },
            ..Movement::default()
        };
        let up = [KeyName(ARROW_KEYS[0].into())];
        let still = SteerInput::default();
        let first = steer.decide(&up, still, 0.0, &movement);
        assert!(matches!(first.as_slice(), [Act::Step { .. }]));
        assert!(steer
            .decide(&up, still, SEND_EVERY / 2.0, &movement)
            .is_empty());
        assert!(matches!(
            steer.decide(&[], still, SEND_EVERY, &movement).as_slice(),
            [Act::Stop]
        ));
    }
}
