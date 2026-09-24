//! The keys of the play window: the macros the player bound, the default
//! keys of the official client, Tab for war mode, Ctrl+Q and Ctrl+W for the
//! chat history, and which keys walk.
//!
//! While the player types in a field, a key fires a macro only when it
//! cannot type: a chord with Ctrl or Alt, or a function key. The switches
//! of the Experimental page turn the default keys, the arrows, Tab and
//! Ctrl+Q / Ctrl+W off.

pub mod chat;

use super::actions::{chosen, step_action, ActionId, Direction, Look};
use super::settings::{KeyChord, MacroStep, Profile};
use eframe::egui::{self, Event, Id, Key, Modifiers};

/// The default keys of the official client, as the reference client makes
/// them for a new profile: a chord, an action and its argument.
const DEFAULT_KEYS: [(&str, &str, &str); 7] = [
    ("Alt+P", "open", "Paperdoll"),
    ("Alt+O", "open", "Options"),
    ("Alt+J", "open", "Journal"),
    ("Alt+I", "open", "Backpack"),
    ("Alt+R", "open", "Minimap"),
    ("Ctrl+B", "bow", ""),
    ("Ctrl+S", "salute", ""),
];
/// The chat history keys.
const HISTORY_OLDER: Key = Key::Q;
const HISTORY_NEWER: Key = Key::W;
const WAR_KEY: Key = Key::Tab;

/// Where the keys go now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    /// No field has the keys.
    Free,
    /// The chat line has the keys and holds no words.
    ChatEmpty,
    /// The chat line has the keys and holds words.
    ChatTyping,
    /// Another text field has the keys.
    OtherField,
}

/// The mark of a text field that is not an egui text edit, such as a text
/// box of a classic gump, so the keys know it takes words.
#[derive(Clone, Copy, Debug, Default)]
struct WordField;

/// Marks the widget `id` as a field that takes words. A field drawn by hand
/// calls it each frame it draws.
pub fn mark_word_field(ctx: &egui::Context, id: Id) {
    ctx.data_mut(|data| data.insert_temp(id, WordField));
}

fn is_word_field(ctx: &egui::Context, id: Id) -> bool {
    egui::text_edit::TextEditState::load(ctx, id).is_some()
        || ctx.data(|data| data.get_temp::<WordField>(id)).is_some()
}

impl Focus {
    /// The focus of the window: the chat line is the field with `chat_id`.
    pub fn of(ctx: &egui::Context, chat_id: Id, chat_empty: bool) -> Self {
        let focused = ctx.memory(|memory| memory.focused());
        match focused {
            None => Focus::Free,
            Some(id) if id == chat_id && chat_empty => Focus::ChatEmpty,
            Some(id) if id == chat_id => Focus::ChatTyping,
            Some(id) if is_word_field(ctx, id) => Focus::OtherField,
            // A button that was clicked keeps the focus, but it takes no
            // words.
            Some(_) => Focus::Free,
        }
    }

    fn typing(self) -> bool {
        self != Focus::Free
    }

    fn in_chat(self) -> bool {
        matches!(self, Focus::ChatEmpty | Focus::ChatTyping)
    }
}

/// One key the player pressed or let go this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyPress {
    pub key: Key,
    pub modifiers: Modifiers,
    pub pressed: bool,
    pub repeat: bool,
}

/// A chord that cannot type a letter: it has Ctrl or Alt, or its key is a
/// function key.
fn cannot_type(chord: &KeyChord) -> bool {
    chord.ctrl || chord.alt || is_function_key(&chord.key)
}

fn is_function_key(name: &str) -> bool {
    name.strip_prefix('F')
        .is_some_and(|number| !number.is_empty() && number.chars().all(|c| c.is_ascii_digit()))
}

/// What Tab asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarKey {
    /// War mode on or off.
    Set(bool),
    Toggle,
}

/// What the keys of one frame ask for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dispatched {
    /// The macros to start, in order: the steps of each.
    pub macros: Vec<Vec<MacroStep>>,
    pub war: Option<WarKey>,
    /// Ctrl+Q (true) or Ctrl+W (false) in the chat line.
    pub history_older: Option<bool>,
    /// The keys that fired something. Nothing else may act on them.
    pub used: Vec<(Modifiers, Key)>,
}

/// Which keys walk now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkKeys {
    pub arrows: bool,
    pub wasd: bool,
}

/// A held macro of one step: a walk or a look while its key is down.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Held {
    key: Key,
    step: MacroStep,
}

#[derive(Default)]
pub struct KeyDispatch {
    held: Vec<Held>,
    /// Tab put the character in war mode and waits for its release.
    tab_war: bool,
}

/// The macro a chord runs: the player's own first, then a default key.
fn bound_steps(chord: &KeyChord, profile: &Profile) -> Option<Vec<MacroStep>> {
    let own = profile
        .macros
        .key_bindings
        .iter()
        .find(|binding| binding.chord.as_ref() == Some(chord))
        .map(|binding| binding.steps.clone());
    own.or_else(|| {
        if profile.experimental.disable_default_hotkeys {
            return None;
        }
        DEFAULT_KEYS
            .iter()
            .find(|(words, _, _)| words.parse::<KeyChord>().ok().as_ref() == Some(chord))
            .map(|&(_, action, argument)| vec![MacroStep::new(action, argument)])
    })
}

/// The default keys, as words for the Options screen.
pub fn default_keys() -> impl Iterator<Item = (&'static str, &'static str, &'static str)> {
    DEFAULT_KEYS.into_iter()
}

/// A macro of one held step: walk or look.
fn held_step(steps: &[MacroStep]) -> Option<&MacroStep> {
    match steps {
        [step] if step_action(step).is_some_and(ActionId::is_held) => Some(step),
        _ => None,
    }
}

impl KeyDispatch {
    /// The key presses of this frame.
    pub fn presses(ctx: &egui::Context) -> Vec<KeyPress> {
        ctx.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    Event::Key {
                        key,
                        pressed,
                        repeat,
                        modifiers,
                        ..
                    } => Some(KeyPress {
                        key: *key,
                        modifiers: *modifiers,
                        pressed: *pressed,
                        repeat: *repeat,
                    }),
                    _ => None,
                })
                .collect()
        })
    }

    /// Takes the used keys from the input, so no field and no other part
    /// of the window acts on them, and drops the letters they would type.
    pub fn take_used(ctx: &egui::Context, dispatched: &Dispatched) {
        if dispatched.used.is_empty() {
            return;
        }
        ctx.input_mut(|input| {
            for (modifiers, key) in &dispatched.used {
                input.consume_key(*modifiers, *key);
            }
            input
                .events
                .retain(|event| !matches!(event, Event::Text(_)));
        });
    }

    /// Reads the keys of one frame.
    pub fn dispatch(
        &mut self,
        presses: &[KeyPress],
        focus: Focus,
        profile: &Profile,
        war: bool,
    ) -> Dispatched {
        let mut out = Dispatched::default();
        let experimental = &profile.experimental;
        for press in presses {
            let chord = KeyChord::from_egui(press.key, press.modifiers);
            let steps = bound_steps(&chord, profile);
            if !press.pressed {
                self.held.retain(|held| held.key != press.key);
                if press.key == WAR_KEY && steps.is_none() && !experimental.disable_tab_war_mode {
                    if profile.combat.hold_tab_for_combat {
                        if std::mem::take(&mut self.tab_war) {
                            out.war = Some(WarKey::Set(false));
                        }
                    } else if focus != Focus::OtherField {
                        out.war = Some(WarKey::Toggle);
                    }
                }
                continue;
            }
            if press.repeat {
                continue;
            }
            if let Some(steps) = steps.filter(|_| !focus.typing() || cannot_type(&chord)) {
                match held_step(&steps) {
                    Some(step) => self.held.push(Held {
                        key: press.key,
                        step: step.clone(),
                    }),
                    None => out.macros.push(steps),
                }
                out.used.push((press.modifiers, press.key));
                continue;
            }
            let plain_ctrl =
                press.modifiers.command && !press.modifiers.alt && !press.modifiers.shift;
            let history_key = press.key == HISTORY_OLDER || press.key == HISTORY_NEWER;
            if plain_ctrl && history_key && focus.in_chat() && !experimental.disable_message_history
            {
                out.history_older = Some(press.key == HISTORY_OLDER);
                out.used.push((press.modifiers, press.key));
                continue;
            }
            let plain = press.modifiers.is_none();
            let tab = plain && press.key == WAR_KEY && !experimental.disable_tab_war_mode;
            if tab && focus != Focus::OtherField {
                out.used.push((press.modifiers, press.key));
                if profile.combat.hold_tab_for_combat && !war {
                    self.tab_war = true;
                    out.war = Some(WarKey::Set(true));
                }
            }
        }
        out
    }

    /// The way the held walk keys name, from their directions.
    pub fn held_walk(&self) -> Vec<Direction> {
        self.held
            .iter()
            .filter(|held| step_action(&held.step) == Some(ActionId::Walk))
            .filter_map(|held| chosen::<Direction>(&held.step.argument))
            .collect()
    }

    /// The way the camera looks while a look key is held.
    pub fn held_look(&self) -> Option<Look> {
        self.held
            .iter()
            .filter(|held| step_action(&held.step) == Some(ActionId::LookAtMouse))
            .find_map(|held| chosen::<Look>(&held.step.argument))
    }

    /// Which keys walk: the arrows while no words are typed, W A S D only
    /// while no field takes the keys.
    pub fn walk_keys(focus: Focus, profile: &Profile) -> WalkKeys {
        WalkKeys {
            arrows: !profile.experimental.disable_arrow_keys
                && matches!(focus, Focus::Free | Focus::ChatEmpty),
            wasd: profile.general.wasd_movement && focus == Focus::Free,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::settings::KeyBinding;

    fn press(key: Key, modifiers: Modifiers) -> KeyPress {
        KeyPress {
            key,
            modifiers,
            pressed: true,
            repeat: false,
        }
    }

    fn release(key: Key) -> KeyPress {
        KeyPress {
            pressed: false,
            ..press(key, Modifiers::NONE)
        }
    }

    fn bound(profile: &mut Profile, chord: &str, action: &str, argument: &str) {
        profile.macros.key_bindings.push(KeyBinding {
            chord: Some(chord.parse().unwrap()),
            steps: vec![MacroStep::new(action, argument)],
            ..KeyBinding::default()
        });
    }

    #[test]
    fn a_focused_hand_drawn_field_takes_the_keys_and_a_button_does_not() {
        let ctx = egui::Context::default();
        let chat = Id::new("chat");
        let field = Id::new("field");
        let button = Id::new("button");
        let focus_on = |id| {
            ctx.memory_mut(|memory| memory.request_focus(id));
            Focus::of(&ctx, chat, true)
        };
        assert_eq!(focus_on(field), Focus::Free, "not marked yet");
        mark_word_field(&ctx, field);
        assert_eq!(focus_on(field), Focus::OtherField);
        assert_eq!(focus_on(button), Focus::Free);
        assert_eq!(focus_on(chat), Focus::ChatEmpty);
    }

    #[test]
    fn a_chord_matches_only_with_its_own_modifiers() {
        let mut profile = Profile::default();
        bound(&mut profile, "Ctrl+H", "say", "heal");
        let mut keys = KeyDispatch::default();
        let ran = |keys: &mut KeyDispatch, modifiers| {
            keys.dispatch(&[press(Key::H, modifiers)], Focus::Free, &profile, false)
                .macros
        };
        assert_eq!(ran(&mut keys, Modifiers::COMMAND).len(), 1);
        assert!(ran(&mut keys, Modifiers::NONE).is_empty());
        assert!(ran(&mut keys, Modifiers::COMMAND | Modifiers::SHIFT).is_empty());
    }

    #[test]
    fn typing_fires_only_chords_that_cannot_type() {
        let mut profile = Profile::default();
        bound(&mut profile, "A", "say", "a");
        bound(&mut profile, "F5", "say", "five");
        bound(&mut profile, "Alt+A", "say", "alt a");
        let mut keys = KeyDispatch::default();
        let fired = |keys: &mut KeyDispatch, key, modifiers, focus| {
            keys.dispatch(&[press(key, modifiers)], focus, &profile, false)
                .macros
                .len()
        };
        assert_eq!(fired(&mut keys, Key::A, Modifiers::NONE, Focus::Free), 1);
        assert_eq!(
            fired(&mut keys, Key::A, Modifiers::NONE, Focus::ChatEmpty),
            0
        );
        assert_eq!(
            fired(&mut keys, Key::A, Modifiers::NONE, Focus::OtherField),
            0
        );
        assert_eq!(
            fired(&mut keys, Key::F5, Modifiers::NONE, Focus::ChatTyping),
            1
        );
        assert_eq!(
            fired(&mut keys, Key::A, Modifiers::ALT, Focus::ChatTyping),
            1
        );
    }

    #[test]
    fn a_default_key_runs_unless_the_player_bound_the_chord_or_turned_them_off() {
        let mut profile = Profile::default();
        let mut keys = KeyDispatch::default();
        let alt_p = [press(Key::P, Modifiers::ALT)];
        let first = |out: Dispatched| out.macros.into_iter().next().unwrap();
        assert_eq!(
            first(keys.dispatch(&alt_p, Focus::ChatEmpty, &profile, false)),
            vec![MacroStep::new("open", "Paperdoll")]
        );
        bound(&mut profile, "Alt+P", "say", "mine");
        assert_eq!(
            first(keys.dispatch(&alt_p, Focus::Free, &profile, false)),
            vec![MacroStep::new("say", "mine")]
        );
        profile.macros.key_bindings.clear();
        profile.experimental.disable_default_hotkeys = true;
        assert!(keys
            .dispatch(&alt_p, Focus::Free, &profile, false)
            .macros
            .is_empty());
    }

    #[test]
    fn tab_holds_war_mode_or_toggles_it() {
        let mut profile = Profile::default();
        let mut keys = KeyDispatch::default();
        let tab = [press(Key::Tab, Modifiers::NONE)];
        let down = keys.dispatch(&tab, Focus::ChatEmpty, &profile, false);
        assert_eq!(down.war, Some(WarKey::Set(true)));
        assert_eq!(down.used.len(), 1);
        let up = keys.dispatch(&[release(Key::Tab)], Focus::ChatEmpty, &profile, true);
        assert_eq!(up.war, Some(WarKey::Set(false)));
        profile.combat.hold_tab_for_combat = false;
        assert_eq!(keys.dispatch(&tab, Focus::Free, &profile, false).war, None);
        assert_eq!(
            keys.dispatch(&[release(Key::Tab)], Focus::Free, &profile, false)
                .war,
            Some(WarKey::Toggle)
        );
        profile.experimental.disable_tab_war_mode = true;
        assert_eq!(
            keys.dispatch(&[release(Key::Tab)], Focus::Free, &profile, false)
                .war,
            None
        );
    }

    #[test]
    fn ctrl_q_and_ctrl_w_walk_the_history_only_in_the_chat_line() {
        let mut profile = Profile::default();
        let mut keys = KeyDispatch::default();
        let ctrl_q = [press(Key::Q, Modifiers::COMMAND)];
        assert_eq!(
            keys.dispatch(&ctrl_q, Focus::ChatTyping, &profile, false)
                .history_older,
            Some(true)
        );
        assert_eq!(
            keys.dispatch(&ctrl_q, Focus::Free, &profile, false)
                .history_older,
            None
        );
        profile.experimental.disable_message_history = true;
        assert_eq!(
            keys.dispatch(&ctrl_q, Focus::ChatTyping, &profile, false)
                .history_older,
            None
        );
    }

    #[test]
    fn a_walk_key_walks_while_it_is_held() {
        let mut profile = Profile::default();
        bound(&mut profile, "F2", "walk", "North");
        let mut keys = KeyDispatch::default();
        let out = keys.dispatch(
            &[press(Key::F2, Modifiers::NONE)],
            Focus::Free,
            &profile,
            false,
        );
        assert!(out.macros.is_empty());
        assert_eq!(keys.held_walk(), vec![Direction::North]);
        keys.dispatch(&[release(Key::F2)], Focus::Free, &profile, false);
        assert!(keys.held_walk().is_empty());
    }

    #[test]
    fn the_arrows_walk_unless_words_are_typed_and_wasd_only_when_free() {
        let mut profile = Profile::default();
        let walk = |focus, profile: &Profile| KeyDispatch::walk_keys(focus, profile);
        assert!(walk(Focus::ChatEmpty, &profile).arrows);
        assert!(!walk(Focus::ChatTyping, &profile).arrows);
        assert!(!walk(Focus::Free, &profile).wasd);
        profile.general.wasd_movement = true;
        assert!(walk(Focus::Free, &profile).wasd);
        assert!(!walk(Focus::ChatEmpty, &profile).wasd);
        profile.experimental.disable_arrow_keys = true;
        assert!(!walk(Focus::Free, &profile).arrows);
    }

    #[test]
    fn only_function_keys_and_modifier_chords_cannot_type() {
        let chord = |words: &str| words.parse::<KeyChord>().unwrap();
        assert!(cannot_type(&chord("F12")));
        assert!(cannot_type(&chord("Ctrl+A")));
        assert!(!cannot_type(&chord("Shift+A")));
        assert!(!cannot_type(&chord("F")));
    }
}
