//! A game controller. The left stick walks the character in eight ways
//! and runs him when it is pushed far; the right stick moves the mouse;
//! each button, or buttons held together, runs a macro of the Macros page
//! as a key does. The Macros page turns the controller on.
//!
//! The caller reads the controller: the sticks and the buttons down in
//! each frame. The window reads it with gilrs, the browser with the
//! Gamepad API.

use crate::geom::{Area, Point, Rgba, Vector};
use crate::scene::Overlay;
use crate::settings::{KeyBinding, MacroStep, PadChord, Profile};
use crate::steer::way_of;

/// A stick nearer the middle than this does nothing.
pub const STICK_DEAD_ZONE: f32 = 0.25;
/// The left stick pushed this far runs.
pub const STICK_RUN: f32 = 0.85;
/// The mouse moves this many points each second for each step of the
/// sensitivity, with the right stick pushed all the way.
const POINTS_PER_SENSITIVITY: f32 = 60.0;
/// The buttons of a new profile, as words of the Macros page: a button and
/// the action it runs. The Experimental page's "turn off the default keys"
/// turns them off too.
pub const DEFAULT_BUTTONS: [(&str, &str, &str); 5] = [
    ("South", "left_click", ""),
    ("East", "right_click", ""),
    ("West", "double_click", ""),
    ("North", "war_peace", ""),
    ("Start", "toggle", "Options"),
];

/// Makes the buttons of a controller and their names from one list, so a
/// button cannot have no name. The names are the ones gilrs gives, which
/// the profiles keep.
macro_rules! pad_buttons {
    ($($button:ident),+ $(,)?) => {
        /// A button of a controller.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ::serde::Deserialize)]
        pub enum PadButton {
            $($button),+
        }

        impl PadButton {
            /// The name the Macros page keeps.
            pub fn name(self) -> &'static str {
                match self {
                    $(PadButton::$button => stringify!($button)),+
                }
            }
        }
    };
}

pad_buttons! {
    South,
    East,
    North,
    West,
    C,
    Z,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

/// What the controller asks for in one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PadFrame {
    /// The way the left stick walks, and whether it runs.
    pub walk: Option<(&'static str, bool)>,
    /// How far the right stick moves the mouse, in points.
    pub pointer: Vector,
    /// The macros to start, in order.
    pub macros: Vec<Vec<MacroStep>>,
    /// The buttons pressed this frame, for the key editor.
    pub pressed: Option<PadChord>,
}

/// The soft pointer the right stick moves, where the client cannot move
/// the mouse: an arrow this long down its left edge, with its tip at the
/// pointer, white with a black edge.
const SOFT_POINTER_LENGTH: f32 = 18.0;
/// The arrow leans this far right of its left edge, at its end.
const SOFT_POINTER_LEAN: f32 = 12.0;
const SOFT_POINTER_EDGE_WIDTH: f32 = 1.5;
const SOFT_POINTER_EDGE: Rgba = Rgba::from_rgb(0, 0, 0);

/// Where the pointer stands after the right stick moved it `by`: from `at`,
/// or from the middle of `room` when it is not over the window, and never
/// out of `room`. None when the stick does not move it.
pub fn moved_pointer(at: Option<Point>, by: Vector, room: Area) -> Option<Point> {
    if by == Vector::ZERO {
        return None;
    }
    let moved = at.unwrap_or_else(|| room.center()) + by;
    Some(Point::new(
        moved.x.clamp(room.min.x, room.max.x),
        moved.y.clamp(room.min.y, room.max.y),
    ))
}

/// The soft pointer at `at`, for a client that cannot move the mouse.
pub fn soft_pointer(at: Point) -> Overlay {
    Overlay::Polygon {
        points: vec![
            at,
            at + Vector::new(0.0, SOFT_POINTER_LENGTH),
            at + Vector::new(SOFT_POINTER_LEAN, SOFT_POINTER_LEAN),
        ],
        fill: Rgba::WHITE,
        width: SOFT_POINTER_EDGE_WIDTH,
        edge: SOFT_POINTER_EDGE,
    }
}

/// The buttons that were down in the last frame, in the order they went
/// down.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PadState {
    down: Vec<PadButton>,
}

/// A stick position with the dead zone taken out, in screen terms: y down.
pub fn stick(x: f32, y: f32) -> Option<Vector> {
    let push = Vector::new(x, -y);
    (push.length() > STICK_DEAD_ZONE).then_some(push)
}

/// The walk the left stick asks for.
pub fn stick_walk(x: f32, y: f32) -> Option<(&'static str, bool)> {
    stick(x, y).map(|push| (way_of(push), push.length() >= STICK_RUN))
}

/// The macro the buttons run: the player's own first, then a default.
pub fn bound_steps(
    chord: &PadChord,
    bindings: &[KeyBinding],
    defaults_on: bool,
) -> Option<Vec<MacroStep>> {
    let same = |other: &PadChord| {
        other.button == chord.button
            && other.held.len() == chord.held.len()
            && other.held.iter().all(|held| chord.held.contains(held))
    };
    let own = bindings
        .iter()
        .find(|binding| binding.pad.as_ref().is_some_and(same))
        .map(|binding| binding.steps.clone());
    own.or_else(|| {
        defaults_on
            .then(|| {
                DEFAULT_BUTTONS
                    .iter()
                    .find(|(button, _, _)| chord.held.is_empty() && *button == chord.button)
                    .map(|&(_, action, argument)| vec![MacroStep::new(action, argument)])
            })
            .flatten()
    })
}

/// The default buttons, as words for the Options screen.
pub fn default_buttons() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    DEFAULT_BUTTONS.into_iter()
}

impl PadState {
    /// True when the button was down in the last read.
    pub fn is_down(&self, button: PadButton) -> bool {
        self.down.contains(&button)
    }

    /// Reads the controller of one frame: the sticks as left x, left y,
    /// right x, right y (y up), and the buttons down now in the order they
    /// went down. `seconds` is the time since the last read. A button that
    /// was not down in the last frame is pressed, with the ones down before
    /// it held.
    pub fn read(
        &mut self,
        sticks: [f32; 4],
        buttons_down: &[PadButton],
        profile: &Profile,
        seconds: f32,
    ) -> PadFrame {
        let mut frame = PadFrame::default();
        let options = &profile.macros;
        if !options.controller_enabled {
            self.down.clear();
            return frame;
        }
        let defaults_on = !profile.experimental.disable_default_hotkeys;
        self.down.retain(|button| buttons_down.contains(button));
        for &button in buttons_down {
            if self.down.contains(&button) {
                continue;
            }
            let chord = PadChord {
                held: self
                    .down
                    .iter()
                    .map(|held| held.name().to_string())
                    .collect(),
                button: button.name().to_string(),
            };
            if let Some(steps) = bound_steps(&chord, &options.key_bindings, defaults_on) {
                frame.macros.push(steps);
            }
            frame.pressed = Some(chord);
            self.down.push(button);
        }
        let [left_x, left_y, right_x, right_y] = sticks;
        frame.walk = stick_walk(left_x, left_y);
        frame.pointer = stick(right_x, right_y).map_or(Vector::ZERO, |push| {
            push * f32::from(options.controller_mouse_sensitivity)
                * POINTS_PER_SENSITIVITY
                * seconds
        });
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_left_stick_walks_eight_ways_and_runs_when_pushed_far() {
        assert_eq!(stick_walk(0.1, 0.1), None);
        assert_eq!(stick_walk(0.0, 0.5), Some(("nw", false)));
        assert_eq!(stick_walk(1.0, 0.0), Some(("ne", true)));
        assert_eq!(stick_walk(0.7, -0.7), Some(("e", true)));
    }

    #[test]
    fn buttons_run_the_macro_bound_to_them_or_a_default() {
        let chord = |words: &str| words.parse::<PadChord>().unwrap();
        let bindings = vec![KeyBinding {
            pad: Some(chord("LeftTrigger+South")),
            steps: vec![MacroStep::new("bow", "")],
            ..KeyBinding::default()
        }];
        assert_eq!(
            bound_steps(&chord("LeftTrigger+South"), &bindings, true),
            Some(vec![MacroStep::new("bow", "")])
        );
        assert_eq!(
            bound_steps(&chord("South"), &bindings, true),
            Some(vec![MacroStep::new("left_click", "")])
        );
        assert_eq!(bound_steps(&chord("South"), &bindings, false), None);
        assert_eq!(
            bound_steps(&chord("RightTrigger+South"), &bindings, true),
            None
        );
    }

    #[test]
    fn a_button_pressed_while_another_is_held_makes_a_chord_once() {
        const SECONDS: f32 = 1.0 / 60.0;
        let mut profile = Profile::default();
        profile.macros.controller_enabled = true;
        profile.macros.key_bindings.push(KeyBinding {
            pad: Some("LeftTrigger+South".parse().unwrap()),
            steps: vec![MacroStep::new("bow", "")],
            ..KeyBinding::default()
        });
        let mut pad = PadState::default();
        let sticks = [0.0; 4];
        let first = pad.read(sticks, &[PadButton::LeftTrigger], &profile, SECONDS);
        assert_eq!(
            first.pressed.map(|chord| chord.button),
            Some("LeftTrigger".into())
        );
        let both = [PadButton::LeftTrigger, PadButton::South];
        let chord = pad.read(sticks, &both, &profile, SECONDS);
        assert_eq!(chord.macros, vec![vec![MacroStep::new("bow", "")]]);
        assert!(pad.read(sticks, &both, &profile, SECONDS).pressed.is_none());
        profile.macros.controller_enabled = false;
        assert_eq!(
            pad.read(sticks, &both, &profile, SECONDS),
            PadFrame::default()
        );
    }
}

#[cfg(test)]
mod read_tests {
    use super::*;
    use crate::settings::Profile;

    const SECONDS: f32 = 1.0 / 60.0;

    const ROOM: Area = Area {
        min: Point { x: 0.0, y: 0.0 },
        max: Point { x: 100.0, y: 50.0 },
    };

    #[test]
    fn the_right_stick_moves_the_pointer_inside_the_window() {
        let by = Vector::new(10.0, 5.0);
        let at = Some(Point::new(20.0, 20.0));
        assert_eq!(moved_pointer(at, by, ROOM), Some(Point::new(30.0, 25.0)));
        let far = Vector::new(500.0, -500.0);
        assert_eq!(moved_pointer(at, far, ROOM), Some(Point::new(100.0, 0.0)));
        assert_eq!(moved_pointer(at, Vector::ZERO, ROOM), None);
    }

    #[test]
    fn a_pointer_not_over_the_window_starts_from_its_middle() {
        let by = Vector::new(1.0, 1.0);
        assert_eq!(moved_pointer(None, by, ROOM), Some(Point::new(51.0, 26.0)));
    }

    #[test]
    fn the_soft_pointer_is_drawn_with_its_tip_at_the_pointer() {
        let at = Point::new(30.0, 40.0);
        let Overlay::Polygon { points, .. } = soft_pointer(at) else {
            panic!("an arrow");
        };
        assert_eq!(points.first(), Some(&at));
        assert!(points
            .iter()
            .all(|point| point.x >= at.x && point.y >= at.y));
    }

    #[test]
    fn a_stick_pushed_far_runs() {
        let mut pad = PadState::default();
        let mut profile = Profile::default();
        profile.macros.controller_enabled = true;
        let frame = pad.read([0.0, -1.0, 0.0, 0.0], &[], &profile, SECONDS);
        assert!(frame.walk.map(|(_, run)| run).unwrap_or(false));
    }
}
