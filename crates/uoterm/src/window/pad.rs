//! A game controller. The left stick walks the character in eight ways
//! and runs him when it is pushed far; the right stick moves the mouse;
//! each button, or buttons held together, runs a macro of the Macros page
//! as a key does. The Macros page turns the controller on.

use super::settings::{KeyBinding, MacroStep, PadChord};
use super::steer::way_of;
use eframe::egui::{self, Id, Vec2};
use gilrs::{Axis, Button, EventType, Gilrs};

/// A stick nearer the middle than this does nothing.
const STICK_DEAD_ZONE: f32 = 0.25;
/// The left stick pushed this far runs.
const STICK_RUN: f32 = 0.85;
/// The mouse moves this many points each second for each step of the
/// sensitivity, with the right stick pushed all the way.
const POINTS_PER_SENSITIVITY: f32 = 60.0;
/// The buttons of a new profile, as words of the Macros page: a button and
/// the action it runs. The Experimental page's "turn off the default keys"
/// turns them off too.
const DEFAULT_BUTTONS: [(&str, &str, &str); 5] = [
    ("South", "left_click", ""),
    ("East", "right_click", ""),
    ("West", "double_click", ""),
    ("North", "war_peace", ""),
    ("Start", "toggle", "Options"),
];
/// Where the window keeps the buttons pressed this frame, for the key
/// editor of the Options screen.
const PRESSED_ID: &str = "pad-pressed";
const NOTE_NO_CONTROLLER: &str = "No game controller: the system gave no controller input.";

/// What the controller asks for in one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PadFrame {
    /// The way the left stick walks, and whether it runs.
    pub walk: Option<(&'static str, bool)>,
    /// How far the right stick moves the mouse, in points.
    pub pointer: Vec2,
    /// The macros to start, in order.
    pub macros: Vec<Vec<MacroStep>>,
    /// The buttons pressed this frame, for the key editor.
    pub pressed: Option<PadChord>,
}

#[derive(Default)]
pub struct Pad {
    /// None until the controller is turned on, and when the system has no
    /// controller input.
    gilrs: Option<Gilrs>,
    tried: bool,
    note: String,
    /// The buttons down now, by name.
    down: Vec<String>,
}

/// A stick position with the dead zone taken out, in screen terms: y down.
fn stick(x: f32, y: f32) -> Option<Vec2> {
    let push = Vec2::new(x, -y);
    (push.length() > STICK_DEAD_ZONE).then_some(push)
}

/// The walk the left stick asks for.
fn stick_walk(x: f32, y: f32) -> Option<(&'static str, bool)> {
    stick(x, y).map(|push| (way_of(push), push.length() >= STICK_RUN))
}

/// The macro the buttons run: the player's own first, then a default.
fn bound_steps(
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

/// The buttons pressed this frame, as the pad left them for the editor.
pub fn pressed_this_frame(ctx: &egui::Context) -> Option<PadChord> {
    ctx.data(|data| data.get_temp::<PadChord>(Id::new(PRESSED_ID)))
}

/// Leaves the buttons pressed this frame for the editor, or clears them.
pub fn leave_pressed(ctx: &egui::Context, pressed: Option<PadChord>) {
    ctx.data_mut(|data| match pressed {
        Some(chord) => data.insert_temp(Id::new(PRESSED_ID), chord),
        None => data.remove::<PadChord>(Id::new(PRESSED_ID)),
    });
}

impl Pad {
    /// Why the controller does nothing, or nothing.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Reads the controller. `seconds` is the time since the last read.
    pub fn poll(
        &mut self,
        enabled: bool,
        sensitivity: u8,
        bindings: &[KeyBinding],
        defaults_on: bool,
        seconds: f32,
    ) -> PadFrame {
        let mut frame = PadFrame::default();
        if !enabled {
            self.down.clear();
            return frame;
        }
        if !self.tried {
            self.tried = true;
            match Gilrs::new() {
                Ok(gilrs) => self.gilrs = Some(gilrs),
                Err(e) => {
                    tracing::warn!(error = %e, "game controller");
                    self.note = NOTE_NO_CONTROLLER.to_string();
                }
            }
        }
        let Some(gilrs) = self.gilrs.as_mut() else {
            return frame;
        };
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::ButtonPressed(button, _) if button != Button::Unknown => {
                    let name = format!("{button:?}");
                    let chord = PadChord {
                        held: self.down.clone(),
                        button: name.clone(),
                    };
                    if let Some(steps) = bound_steps(&chord, bindings, defaults_on) {
                        frame.macros.push(steps);
                    }
                    frame.pressed = Some(chord);
                    self.down.push(name);
                }
                EventType::ButtonReleased(button, _) => {
                    let name = format!("{button:?}");
                    self.down.retain(|down| *down != name);
                }
                EventType::Disconnected => self.down.clear(),
                _ => {}
            }
        }
        if let Some((_, pad)) = gilrs.gamepads().next() {
            frame.walk = stick_walk(pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
            frame.pointer = stick(pad.value(Axis::RightStickX), pad.value(Axis::RightStickY))
                .map_or(Vec2::ZERO, |push| {
                    push * f32::from(sensitivity) * POINTS_PER_SENSITIVITY * seconds
                });
        }
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
}
