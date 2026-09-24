//! The steps of one macro as the classic gumps edit them, over the macro
//! editor of the actions (`actions::editor`): each step with its action, its
//! argument and the buttons that move and remove it, then Add step. The
//! Macros page of the Options gump and the macro gump draw them both, and
//! both take the key or the controller buttons of a macro the same way.

use super::canvas::Canvas;
use super::options::{
    input, text, BUTTON_HEIGHT, COMBO_HEIGHT, FIELD_HEIGHT, LABEL_GAP, ROW_GAP, SMALL_BUTTON,
};
use super::text_field::TextField;
use crate::window::actions::editor::{Capture, MacroEditor, Move};
use crate::window::actions::{ActionId, ACTIONS};
use crate::window::options_ui::{
    take_pressed, Pressed, WORDS_ADD_STEP, WORDS_DOWN, WORDS_REMOVE, WORDS_UP,
};
use crate::window::pad::pressed_this_frame;
use crate::window::settings::KeyBinding;
use std::collections::HashMap;

/// The width of the number, the action and each half of the argument of a
/// step.
pub const STEP_NUMBER_WIDTH: i32 = 18;
pub const ACTION_WIDTH: i32 = 170;
pub const ARGUMENT_WIDTH: i32 = 110;
/// The width of one row of steps, from its number to its last button.
pub const STEPS_WIDTH: i32 =
    STEP_NUMBER_WIDTH + ACTION_WIDTH + LABEL_GAP * 2 + ARGUMENT_WIDTH * 2 + SMALL_BUTTON * 3;

/// Takes the key or the controller buttons the editor waits for. Escape
/// cancels. True when a macro took them.
pub fn take_capture(editor: &mut MacroEditor, g: &Canvas<'_>, keys: &mut [KeyBinding]) -> bool {
    match editor.capture {
        Some(Capture::Chord(_)) => match take_pressed(g.ui()) {
            Some(Pressed::Cancel) => editor.cancel_capture(),
            Some(Pressed::Chord(chord)) => return editor.take_chord(keys, chord),
            None => {}
        },
        Some(Capture::Pad(_)) => {
            if matches!(take_pressed(g.ui()), Some(Pressed::Cancel)) {
                editor.cancel_capture();
            } else if let Some(chord) = pressed_this_frame(g.ctx()) {
                return editor.take_pad(keys, chord);
            }
        }
        None => {}
    }
    false
}

/// The words typed in the arguments of the steps, by macro and step.
#[derive(Default)]
pub struct StepList {
    arguments: HashMap<(usize, usize), TextField>,
}

impl StepList {
    /// Forgets every typed argument, as when the list of macros changes.
    pub fn forget_all(&mut self) {
        self.arguments.clear();
    }

    /// Forgets the typed arguments of a macro whose steps moved.
    fn forget(&mut self, at: usize) {
        self.arguments.retain(|(of, _), _| *of != at);
    }

    /// The steps of the macro at `at`, from `left` and `top`, and Add step
    /// under them. Gives the height they take and whether they changed.
    pub fn draw(
        &mut self,
        g: &mut Canvas<'_>,
        keys: &mut [KeyBinding],
        at: usize,
        (left, top): (i32, i32),
    ) -> (i32, bool) {
        let look = text().bordered();
        let labels: Vec<&str> = ACTIONS.iter().map(|spec| spec.label).collect();
        let mut changed = false;
        let mut moved = None;
        let mut removed = None;
        let mut y = top;
        for step in 0..keys[at].steps.len() {
            let mut x = left;
            g.label(x, y, &(step + 1).to_string(), &text());
            x += STEP_NUMBER_WIDTH;
            let action = ActionId::from_id(&keys[at].steps[step].action);
            let mut index = ACTIONS
                .iter()
                .position(|spec| Some(spec.action) == action)
                .unwrap_or(0);
            if g.combobox(
                ("action", at, step),
                x,
                y,
                ACTION_WIDTH,
                &labels,
                &mut index,
            ) {
                MacroEditor::set_action(keys, at, step, ACTIONS[index].action);
                self.arguments.remove(&(at, step));
                changed = true;
            }
            x += ACTION_WIDTH + LABEL_GAP;
            let action = keys[at].steps[step].action.clone();
            changed |= self.argument(
                g,
                (at, step),
                &mut keys[at].steps[step].argument,
                &action,
                (x, y),
            );
            x += ARGUMENT_WIDTH * 2 + LABEL_GAP;
            for (key, words, press) in [
                ("up", WORDS_UP, Some(Move::Up)),
                ("down", WORDS_DOWN, Some(Move::Down)),
                ("remove-step", WORDS_REMOVE, None),
            ] {
                if g.nice_button(
                    (key, at, step),
                    x,
                    y,
                    SMALL_BUTTON,
                    BUTTON_HEIGHT,
                    words,
                    &look,
                    false,
                ) {
                    match press {
                        Some(way) => moved = Some((step, way)),
                        None => removed = Some(step),
                    }
                }
                x += SMALL_BUTTON;
            }
            y += COMBO_HEIGHT + ROW_GAP;
        }
        if let Some((step, way)) = moved {
            changed |= MacroEditor::move_step(keys, at, step, way);
            self.forget(at);
        }
        if let Some(step) = removed {
            MacroEditor::remove_step(keys, at, step);
            self.forget(at);
            changed = true;
        }
        let mut adding: Vec<&str> = vec![WORDS_ADD_STEP];
        adding.extend(labels.iter());
        let mut picked = 0;
        if g.combobox(
            ("add-step", at),
            left + STEP_NUMBER_WIDTH,
            y,
            ACTION_WIDTH,
            &adding,
            &mut picked,
        ) && picked > 0
        {
            MacroEditor::add_step(keys, at, ACTIONS[picked - 1].action);
            changed = true;
        }
        y += COMBO_HEIGHT + ROW_GAP;
        (y - top, changed)
    }

    /// The argument of a step, as its kind needs: a field to type in, a
    /// list to pick from, or both. True when the player changed it.
    fn argument(
        &mut self,
        g: &mut Canvas<'_>,
        (at, step): (usize, usize),
        argument: &mut String,
        action: &str,
        (x, y): (i32, i32),
    ) -> bool {
        let Some(kind) = ActionId::from_id(action).map(|a| a.spec().argument) else {
            return false;
        };
        let mut changed = false;
        let mut x = x;
        if kind.is_typed() {
            let field = self
                .arguments
                .entry((at, step))
                .or_insert_with(|| TextField::new(argument));
            if input(
                g,
                ("argument", at, step),
                x,
                y,
                ARGUMENT_WIDTH,
                FIELD_HEIGHT,
                field,
            ) {
                *argument = field.text().to_string();
                changed = true;
            }
            x += ARGUMENT_WIDTH + LABEL_GAP;
        }
        let choices = kind.choices();
        if !choices.is_empty() {
            let words: Vec<&str> = choices.iter().map(String::as_str).collect();
            let mut index = choices
                .iter()
                .position(|choice| choice.eq_ignore_ascii_case(argument))
                .unwrap_or(0);
            if g.combobox(
                ("choice", at, step),
                x,
                y,
                ARGUMENT_WIDTH,
                &words,
                &mut index,
            ) {
                *argument = choices[index].clone();
                self.arguments.remove(&(at, step));
                changed = true;
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_macro_forgets_only_its_own_typed_arguments() {
        let mut list = StepList::default();
        list.arguments.insert((0, 0), TextField::new("Heal"));
        list.arguments.insert((1, 0), TextField::new("Cure"));
        list.forget(0);
        assert!(list.arguments.contains_key(&(1, 0)));
        assert!(!list.arguments.contains_key(&(0, 0)));
        list.forget_all();
        assert!(list.arguments.is_empty());
    }
}
