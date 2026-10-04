//! The game controller of the window, read with gilrs. The rules are
//! `uoterm_view::pad`; here gilrs gives the sticks and the buttons down.

pub use uoterm_view::pad::*;

use super::settings::{PadChord, Profile};
use eframe::egui::{self, Id};
use gilrs::{Axis, Button, EventType, Gilrs};

/// Where the window keeps the buttons pressed this frame, for the key
/// editor of the Options screen.
const PRESSED_ID: &str = "pad-pressed";
const NOTE_NO_CONTROLLER: &str = "No game controller: the system gave no controller input.";

#[derive(Default)]
pub struct Pad {
    /// None until the controller is turned on, and when the system has no
    /// controller input.
    gilrs: Option<Gilrs>,
    tried: bool,
    note: String,
    /// The buttons down now, in the order they went down.
    down: Vec<PadButton>,
    state: PadState,
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

/// The button of the shared rules for a gilrs button.
fn pad_button(button: Button) -> Option<PadButton> {
    Some(match button {
        Button::South => PadButton::South,
        Button::East => PadButton::East,
        Button::North => PadButton::North,
        Button::West => PadButton::West,
        Button::C => PadButton::C,
        Button::Z => PadButton::Z,
        Button::LeftTrigger => PadButton::LeftTrigger,
        Button::LeftTrigger2 => PadButton::LeftTrigger2,
        Button::RightTrigger => PadButton::RightTrigger,
        Button::RightTrigger2 => PadButton::RightTrigger2,
        Button::Select => PadButton::Select,
        Button::Start => PadButton::Start,
        Button::Mode => PadButton::Mode,
        Button::LeftThumb => PadButton::LeftThumb,
        Button::RightThumb => PadButton::RightThumb,
        Button::DPadUp => PadButton::DPadUp,
        Button::DPadDown => PadButton::DPadDown,
        Button::DPadLeft => PadButton::DPadLeft,
        Button::DPadRight => PadButton::DPadRight,
        Button::Unknown => return None,
    })
}

impl Pad {
    /// Why the controller does nothing, or nothing.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Reads the controller. `seconds` is the time since the last read.
    pub fn poll(&mut self, profile: &Profile, seconds: f32) -> PadFrame {
        let (sticks, buttons) = if profile.macros.controller_enabled {
            self.read_gilrs()
        } else {
            self.down.clear();
            ([0.0; 4], Vec::new())
        };
        self.state.read(sticks, &buttons, profile, seconds)
    }

    /// The sticks and the buttons down in this read. A button pressed and
    /// let go between two reads counts as down in this one, so a quick tap
    /// still runs its macro.
    fn read_gilrs(&mut self) -> ([f32; 4], Vec<PadButton>) {
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
            return ([0.0; 4], Vec::new());
        };
        let mut tapped = Vec::new();
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::ButtonPressed(button, _) => {
                    if let Some(button) = pad_button(button).filter(|b| !self.down.contains(b)) {
                        self.down.push(button);
                    }
                }
                EventType::ButtonReleased(button, _) => {
                    if let Some(button) = pad_button(button) {
                        if self.down.contains(&button) && !self.state.is_down(button) {
                            tapped.push(button);
                        }
                        self.down.retain(|down| *down != button);
                    }
                }
                EventType::Disconnected => self.down.clear(),
                _ => {}
            }
        }
        let sticks = gilrs.gamepads().next().map_or([0.0; 4], |(_, pad)| {
            [
                pad.value(Axis::LeftStickX),
                pad.value(Axis::LeftStickY),
                pad.value(Axis::RightStickX),
                pad.value(Axis::RightStickY),
            ]
        });
        let buttons = self.down.iter().chain(&tapped).copied().collect();
        (sticks, buttons)
    }
}
