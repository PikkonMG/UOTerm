//! The keys of the play window: the macros the player bound, the default
//! keys of the official client, Tab for war mode, Ctrl+Q and Ctrl+W for the
//! chat history, and which keys walk.
//!
//! While the player types in a field, a key fires a macro only when it
//! cannot type: a chord with Ctrl or Alt, or a function key. The switches
//! of the Experimental page turn the default keys, the arrows, Tab and
//! Ctrl+Q / Ctrl+W off.

pub mod chat;

use crate::actions::{chosen, step_action, ActionId, Direction, Look};
use crate::input::{KeyName, KeyPress, Mods};
use crate::settings::{KeyChord, MacroStep, Profile};

/// The default keys of the official client, as the reference client makes
/// them for a new profile: a chord, an action and its argument.
pub const DEFAULT_KEYS: [(&str, &str, &str); 7] = [
    ("Alt+P", "open", "Paperdoll"),
    ("Alt+O", "open", "Options"),
    ("Alt+J", "open", "Journal"),
    ("Alt+I", "open", "Backpack"),
    ("Alt+R", "open", "Minimap"),
    ("Ctrl+B", "bow", ""),
    ("Ctrl+S", "salute", ""),
];
/// The chat history keys.
pub const HISTORY_OLDER: &str = "Q";
pub const HISTORY_NEWER: &str = "W";
pub const WAR_KEY: &str = "Tab";

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

impl Focus {
    /// The focus from what has the keys: the chat line (which holds no
    /// words when `chat_empty`), or another field that takes words. A
    /// button that has the keys takes no words, so it is neither.
    pub fn of(chat_focused: bool, chat_empty: bool, other_field_focused: bool) -> Self {
        match (chat_focused, other_field_focused) {
            (true, _) if chat_empty => Focus::ChatEmpty,
            (true, _) => Focus::ChatTyping,
            (false, true) => Focus::OtherField,
            (false, false) => Focus::Free,
        }
    }

    fn typing(self) -> bool {
        self != Focus::Free
    }

    fn in_chat(self) -> bool {
        matches!(self, Focus::ChatEmpty | Focus::ChatTyping)
    }
}

/// A chord that cannot type a letter: it has Ctrl or Alt, or its key is a
/// function key.
pub fn cannot_type(chord: &KeyChord) -> bool {
    chord.ctrl || chord.alt || is_function_key(&chord.key)
}

pub fn is_function_key(name: &str) -> bool {
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
    pub used: Vec<(Mods, KeyName)>,
}

/// Which keys walk now.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WalkKeys {
    pub arrows: bool,
    pub wasd: bool,
}

/// A held macro of one step: a walk or a look while its key is down.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Held {
    pub key: KeyName,
    pub step: MacroStep,
}

#[derive(Default)]
pub struct KeyDispatch {
    held: Vec<Held>,
    /// Tab put the character in war mode and waits for its release.
    tab_war: bool,
}

/// The macro a chord runs: the player's own first, then a default key.
pub fn bound_steps(chord: &KeyChord, profile: &Profile) -> Option<Vec<MacroStep>> {
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
pub fn held_step(steps: &[MacroStep]) -> Option<&MacroStep> {
    match steps {
        [step] if step_action(step).is_some_and(ActionId::is_held) => Some(step),
        _ => None,
    }
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

impl KeyDispatch {
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
            let chord = KeyChord::from_press(&press.key, press.mods);
            let steps = bound_steps(&chord, profile);
            let is_war_key = press.key.0 == WAR_KEY;
            if !press.pressed {
                self.held.retain(|held| held.key != press.key);
                if is_war_key && steps.is_none() && !experimental.disable_tab_war_mode {
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
                        key: press.key.clone(),
                        step: step.clone(),
                    }),
                    None => out.macros.push(steps),
                }
                out.used.push((press.mods, press.key.clone()));
                continue;
            }
            let plain_ctrl = press.mods.command && !press.mods.alt && !press.mods.shift;
            let history_key = press.key.0 == HISTORY_OLDER || press.key.0 == HISTORY_NEWER;
            if plain_ctrl && history_key && focus.in_chat() && !experimental.disable_message_history
            {
                out.history_older = Some(press.key.0 == HISTORY_OLDER);
                out.used.push((press.mods, press.key.clone()));
                continue;
            }
            let tab = press.mods.is_none() && is_war_key && !experimental.disable_tab_war_mode;
            if tab && focus != Focus::OtherField {
                out.used.push((press.mods, press.key.clone()));
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::KeyBinding;

    const NONE: Mods = Mods {
        ctrl: false,
        alt: false,
        shift: false,
        command: false,
    };
    const COMMAND: Mods = Mods {
        command: true,
        ..NONE
    };
    const ALT: Mods = Mods { alt: true, ..NONE };
    const COMMAND_SHIFT: Mods = Mods {
        shift: true,
        ..COMMAND
    };

    fn key(name: &str) -> KeyName {
        KeyName(name.into())
    }

    fn press(name: &str, mods: Mods) -> KeyPress {
        KeyPress {
            key: key(name),
            mods,
            pressed: true,
            repeat: false,
        }
    }

    fn release(name: &str) -> KeyPress {
        KeyPress {
            pressed: false,
            ..press(name, NONE)
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
    fn the_focus_follows_the_field_that_has_the_keys() {
        assert_eq!(Focus::of(false, true, false), Focus::Free);
        assert_eq!(Focus::of(false, true, true), Focus::OtherField);
        assert_eq!(Focus::of(true, true, false), Focus::ChatEmpty);
        assert_eq!(Focus::of(true, false, false), Focus::ChatTyping);
    }

    #[test]
    fn a_chord_matches_only_with_its_own_modifiers() {
        let mut profile = Profile::default();
        bound(&mut profile, "Ctrl+H", "say", "heal");
        let mut keys = KeyDispatch::default();
        let ran = |keys: &mut KeyDispatch, mods| {
            keys.dispatch(&[press("H", mods)], Focus::Free, &profile, false)
                .macros
        };
        assert_eq!(ran(&mut keys, COMMAND).len(), 1);
        assert!(ran(&mut keys, NONE).is_empty());
        assert!(ran(&mut keys, COMMAND_SHIFT).is_empty());
    }

    #[test]
    fn typing_fires_only_chords_that_cannot_type() {
        let mut profile = Profile::default();
        bound(&mut profile, "A", "say", "a");
        bound(&mut profile, "F5", "say", "five");
        bound(&mut profile, "Alt+A", "say", "alt a");
        let mut keys = KeyDispatch::default();
        let fired = |keys: &mut KeyDispatch, name, mods, focus| {
            keys.dispatch(&[press(name, mods)], focus, &profile, false)
                .macros
                .len()
        };
        assert_eq!(fired(&mut keys, "A", NONE, Focus::Free), 1);
        assert_eq!(fired(&mut keys, "A", NONE, Focus::ChatEmpty), 0);
        assert_eq!(fired(&mut keys, "A", NONE, Focus::OtherField), 0);
        assert_eq!(fired(&mut keys, "F5", NONE, Focus::ChatTyping), 1);
        assert_eq!(fired(&mut keys, "A", ALT, Focus::ChatTyping), 1);
    }

    #[test]
    fn a_default_key_runs_unless_the_player_bound_the_chord_or_turned_them_off() {
        let mut profile = Profile::default();
        let mut keys = KeyDispatch::default();
        let alt_p = [press("P", ALT)];
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
        let tab = [press(WAR_KEY, NONE)];
        let down = keys.dispatch(&tab, Focus::ChatEmpty, &profile, false);
        assert_eq!(down.war, Some(WarKey::Set(true)));
        assert_eq!(down.used, vec![(NONE, key(WAR_KEY))]);
        let up = keys.dispatch(&[release(WAR_KEY)], Focus::ChatEmpty, &profile, true);
        assert_eq!(up.war, Some(WarKey::Set(false)));
        profile.combat.hold_tab_for_combat = false;
        assert_eq!(keys.dispatch(&tab, Focus::Free, &profile, false).war, None);
        assert_eq!(
            keys.dispatch(&[release(WAR_KEY)], Focus::Free, &profile, false)
                .war,
            Some(WarKey::Toggle)
        );
        profile.experimental.disable_tab_war_mode = true;
        assert_eq!(
            keys.dispatch(&[release(WAR_KEY)], Focus::Free, &profile, false)
                .war,
            None
        );
    }

    #[test]
    fn ctrl_q_and_ctrl_w_walk_the_history_only_in_the_chat_line() {
        let mut profile = Profile::default();
        let mut keys = KeyDispatch::default();
        let ctrl_q = [press(HISTORY_OLDER, COMMAND)];
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
        let out = keys.dispatch(&[press("F2", NONE)], Focus::Free, &profile, false);
        assert!(out.macros.is_empty());
        assert_eq!(keys.held_walk(), vec![Direction::North]);
        keys.dispatch(&[release("F2")], Focus::Free, &profile, false);
        assert!(keys.held_walk().is_empty());
    }

    #[test]
    fn the_arrows_walk_unless_words_are_typed_and_wasd_only_when_free() {
        let mut profile = Profile::default();
        assert!(walk_keys(Focus::ChatEmpty, &profile).arrows);
        assert!(!walk_keys(Focus::ChatTyping, &profile).arrows);
        assert!(!walk_keys(Focus::Free, &profile).wasd);
        profile.general.wasd_movement = true;
        assert!(walk_keys(Focus::Free, &profile).wasd);
        assert!(!walk_keys(Focus::ChatEmpty, &profile).wasd);
        profile.experimental.disable_arrow_keys = true;
        assert!(!walk_keys(Focus::Free, &profile).arrows);
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
