//! The macro editor of the Macros page, without its drawing: each UI style
//! draws the list its own way and changes it through these calls.
//!
//! A macro has a name, a key chord or controller buttons, and its steps.
//! One chord runs one macro: a chord given to a second macro leaves the
//! first.

use super::{new_step, step_action, ActionId};
use crate::window::settings::{KeyBinding, KeyChord, PadChord};

const NEW_MACRO_NAME: &str = "New macro";
const UNKNOWN_ACTION: &str = "Unknown action";

/// What waits for the player to press it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capture {
    /// The key chord of the macro at this place.
    Chord(usize),
    /// The controller buttons of the macro at this place.
    Pad(usize),
}

/// Which way a step moves in its macro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Up,
    Down,
}

/// The state of the editor between frames.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MacroEditor {
    /// The macro whose steps show.
    pub open: Option<usize>,
    pub capture: Option<Capture>,
}

/// The name a new macro gets: the first free "New macro", "New macro 2"...
fn free_name(macros: &[KeyBinding]) -> String {
    let taken = |name: &str| macros.iter().any(|m| m.name == name);
    if !taken(NEW_MACRO_NAME) {
        return NEW_MACRO_NAME.to_string();
    }
    (2..)
        .map(|number| format!("{NEW_MACRO_NAME} {number}"))
        .find(|name| !taken(name))
        .expect("a free name")
}

/// A step in words: the action, and its argument when it has one.
pub fn step_words(action: &str, argument: &str) -> String {
    let label = ActionId::from_id(action).map_or(UNKNOWN_ACTION, |a| a.spec().label);
    if argument.trim().is_empty() {
        label.to_string()
    } else {
        format!("{label}: {argument}")
    }
}

impl MacroEditor {
    /// Adds an empty macro and opens it. Gives its place.
    pub fn add(&mut self, macros: &mut Vec<KeyBinding>) -> usize {
        macros.push(KeyBinding {
            name: free_name(macros),
            ..KeyBinding::default()
        });
        let at = macros.len() - 1;
        self.open = Some(at);
        at
    }

    pub fn remove(&mut self, macros: &mut Vec<KeyBinding>, at: usize) {
        if at >= macros.len() {
            return;
        }
        macros.remove(at);
        self.open = match self.open {
            Some(open) if open == at => None,
            Some(open) if open > at => Some(open - 1),
            other => other,
        };
        self.capture = None;
    }

    /// Waits for the next key or controller buttons for a macro.
    pub fn capture(&mut self, capture: Capture) {
        self.capture = Some(capture);
    }

    pub fn cancel_capture(&mut self) {
        self.capture = None;
    }

    /// The key chord the player pressed while the editor waited for one.
    pub fn take_chord(&mut self, macros: &mut [KeyBinding], chord: KeyChord) -> bool {
        let Some(Capture::Chord(at)) = self.capture.take() else {
            return false;
        };
        if at >= macros.len() {
            return false;
        }
        for other in macros.iter_mut() {
            if other.chord.as_ref() == Some(&chord) {
                other.chord = None;
            }
        }
        macros[at].chord = Some(chord);
        true
    }

    /// The controller buttons the player pressed while the editor waited.
    pub fn take_pad(&mut self, macros: &mut [KeyBinding], chord: PadChord) -> bool {
        let Some(Capture::Pad(at)) = self.capture.take() else {
            return false;
        };
        if at >= macros.len() {
            return false;
        }
        for other in macros.iter_mut() {
            if other.pad.as_ref() == Some(&chord) {
                other.pad = None;
            }
        }
        macros[at].pad = Some(chord);
        true
    }

    /// Adds a step of the action at the end of a macro.
    pub fn add_step(macros: &mut [KeyBinding], at: usize, action: ActionId) {
        if let Some(binding) = macros.get_mut(at) {
            binding.steps.push(new_step(action));
        }
    }

    pub fn remove_step(macros: &mut [KeyBinding], at: usize, step: usize) {
        if let Some(binding) = macros.get_mut(at).filter(|b| step < b.steps.len()) {
            binding.steps.remove(step);
        }
    }

    /// Moves a step one place. False when it is already at that end.
    pub fn move_step(macros: &mut [KeyBinding], at: usize, step: usize, way: Move) -> bool {
        let Some(binding) = macros.get_mut(at) else {
            return false;
        };
        let other = match way {
            Move::Up => step.checked_sub(1),
            Move::Down => Some(step + 1).filter(|next| *next < binding.steps.len()),
        };
        match other.filter(|_| step < binding.steps.len()) {
            Some(other) => {
                binding.steps.swap(step, other);
                true
            }
            None => false,
        }
    }

    /// Gives a step another action. Its argument starts again from the
    /// default of the new action, unless both take the same kind.
    pub fn set_action(macros: &mut [KeyBinding], at: usize, step: usize, action: ActionId) {
        let Some(step) = macros.get_mut(at).and_then(|b| b.steps.get_mut(step)) else {
            return;
        };
        let same_kind =
            step_action(step).is_some_and(|old| old.spec().argument == action.spec().argument);
        let argument = std::mem::take(&mut step.argument);
        *step = new_step(action);
        if same_kind {
            step.argument = argument;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::settings::MacroStep;

    fn chord(words: &str) -> KeyChord {
        words.parse().unwrap()
    }

    #[test]
    fn a_new_macro_gets_a_free_name_and_opens() {
        let mut macros = Vec::new();
        let mut editor = MacroEditor::default();
        assert_eq!(editor.add(&mut macros), 0);
        assert_eq!(editor.add(&mut macros), 1);
        assert_eq!(macros[1].name, "New macro 2");
        assert_eq!(editor.open, Some(1));
        editor.remove(&mut macros, 0);
        assert_eq!(editor.open, Some(0));
        editor.remove(&mut macros, 0);
        assert_eq!(editor.open, None);
        assert!(macros.is_empty());
    }

    #[test]
    fn a_captured_chord_leaves_the_macro_that_had_it() {
        let mut macros = Vec::new();
        let mut editor = MacroEditor::default();
        editor.add(&mut macros);
        editor.add(&mut macros);
        assert!(!editor.take_chord(&mut macros, chord("F1")));
        editor.capture(Capture::Chord(0));
        assert!(editor.take_chord(&mut macros, chord("F1")));
        editor.capture(Capture::Chord(1));
        assert!(editor.take_chord(&mut macros, chord("F1")));
        assert_eq!(macros[0].chord, None);
        assert_eq!(macros[1].chord, Some(chord("F1")));
        assert_eq!(editor.capture, None);
        editor.capture(Capture::Pad(0));
        assert!(editor.take_pad(&mut macros, "South".parse().unwrap()));
        assert_eq!(macros[0].pad, Some("South".parse().unwrap()));
    }

    #[test]
    fn steps_are_added_moved_changed_and_removed() {
        let mut macros = Vec::new();
        let mut editor = MacroEditor::default();
        editor.add(&mut macros);
        MacroEditor::add_step(&mut macros, 0, ActionId::CastSpell);
        MacroEditor::add_step(&mut macros, 0, ActionId::WaitForTarget);
        MacroEditor::add_step(&mut macros, 0, ActionId::TargetSelf);
        assert!(MacroEditor::move_step(&mut macros, 0, 2, Move::Up));
        assert_eq!(macros[0].steps[1].action, "target_self");
        assert!(!MacroEditor::move_step(&mut macros, 0, 0, Move::Up));
        assert!(!MacroEditor::move_step(&mut macros, 0, 2, Move::Down));
        macros[0].steps[0].argument = "Heal".into();
        MacroEditor::set_action(&mut macros, 0, 0, ActionId::Say);
        assert_eq!(macros[0].steps[0], MacroStep::new("say", ""));
        MacroEditor::remove_step(&mut macros, 0, 0);
        assert_eq!(macros[0].steps.len(), 2);
        assert_eq!(step_words("cast", "Heal"), "Cast spell: Heal");
        assert_eq!(step_words("bow", ""), "Bow");
    }
}
