//! Key bindings: a key with Ctrl, Alt and Shift, or buttons of a game
//! controller, and the macro it runs. A chord is kept as words, such as
//! `Ctrl+Shift+F1` or `LeftTrigger+South`.

use crate::input::{KeyName, Mods};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

const JOIN: char = '+';
const WORD_CTRL: &str = "Ctrl";
const WORD_ALT: &str = "Alt";
const WORD_SHIFT: &str = "Shift";
const OTHER_WORD_CTRL: &str = "Control";

/// A key and the modifier keys held with it. `key` is the name egui gives
/// the key (`KeyName`), such as `F1`, `A` or `Tab`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct KeyChord {
    pub key: String,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl KeyChord {
    /// The chord of a key the player pressed. Ctrl is the command key on
    /// a Mac.
    pub fn from_press(key: &KeyName, mods: Mods) -> Self {
        Self {
            key: key.0.clone(),
            ctrl: mods.command,
            alt: mods.alt,
            shift: mods.shift,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyChordError {
    NoKey,
    NotAModifier(String),
}

impl fmt::Display for KeyChordError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoKey => f.write_str("the key chord has no key"),
            Self::NotAModifier(word) => write!(f, "\"{word}\" is not Ctrl, Alt or Shift"),
        }
    }
}

impl std::error::Error for KeyChordError {}

impl FromStr for KeyChord {
    type Err = KeyChordError;

    fn from_str(words: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<&str> = words.split(JOIN).map(str::trim).collect();
        let key = parts
            .pop()
            .filter(|key| !key.is_empty())
            .ok_or(KeyChordError::NoKey)?;
        let mut chord = KeyChord {
            key: key.to_string(),
            ctrl: false,
            alt: false,
            shift: false,
        };
        for modifier in parts {
            let held = if modifier.eq_ignore_ascii_case(WORD_CTRL)
                || modifier.eq_ignore_ascii_case(OTHER_WORD_CTRL)
            {
                &mut chord.ctrl
            } else if modifier.eq_ignore_ascii_case(WORD_ALT) {
                &mut chord.alt
            } else if modifier.eq_ignore_ascii_case(WORD_SHIFT) {
                &mut chord.shift
            } else {
                return Err(KeyChordError::NotAModifier(modifier.to_string()));
            };
            *held = true;
        }
        Ok(chord)
    }
}

impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let held = [
            (self.ctrl, WORD_CTRL),
            (self.alt, WORD_ALT),
            (self.shift, WORD_SHIFT),
        ];
        for (_, word) in held.into_iter().filter(|(down, _)| *down) {
            write!(f, "{word}{JOIN}")?;
        }
        f.write_str(&self.key)
    }
}

impl TryFrom<String> for KeyChord {
    type Error = KeyChordError;

    fn try_from(words: String) -> Result<Self, Self::Error> {
        words.parse()
    }
}

impl From<KeyChord> for String {
    fn from(chord: KeyChord) -> Self {
        chord.to_string()
    }
}

/// Buttons of a game controller pressed together: the last one is the
/// button that fires, the others are held with it. A button is named as
/// the controller library names it, such as `South` or `LeftTrigger`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct PadChord {
    pub held: Vec<String>,
    pub button: String,
}

impl FromStr for PadChord {
    type Err = KeyChordError;

    fn from_str(words: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<String> = words
            .split(JOIN)
            .map(str::trim)
            .map(str::to_string)
            .collect();
        let button = parts
            .pop()
            .filter(|button| !button.is_empty())
            .ok_or(KeyChordError::NoKey)?;
        if parts.iter().any(String::is_empty) {
            return Err(KeyChordError::NoKey);
        }
        Ok(Self {
            held: parts,
            button,
        })
    }
}

impl fmt::Display for PadChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for held in &self.held {
            write!(f, "{held}{JOIN}")?;
        }
        f.write_str(&self.button)
    }
}

impl TryFrom<String> for PadChord {
    type Error = KeyChordError;

    fn try_from(words: String) -> Result<Self, Self::Error> {
        words.parse()
    }
}

impl From<PadChord> for String {
    fn from(chord: PadChord) -> Self {
        chord.to_string()
    }
}

/// One step of a macro: the id of an action and the words it takes (a
/// spell name, a skill name, a line to say, a delay in milliseconds).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroStep {
    pub action: String,
    #[serde(default)]
    pub argument: String,
}

impl MacroStep {
    pub fn new(action: &str, argument: &str) -> Self {
        Self {
            action: action.to_string(),
            argument: argument.to_string(),
        }
    }
}

/// One macro the player made: its name, the key or the controller
/// buttons that run it, and its steps in order. A macro with no key runs
/// only from another macro or a button.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyBinding {
    pub name: String,
    pub chord: Option<KeyChord>,
    pub pad: Option<PadChord>,
    pub steps: Vec<MacroStep>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chord_reads_its_modifiers_in_any_case_and_writes_them_in_order() {
        let chord: KeyChord = "shift+control+F1".parse().unwrap();
        assert!(chord.ctrl && chord.shift && !chord.alt);
        assert_eq!(chord.key, "F1");
        assert_eq!(chord.to_string(), "Ctrl+Shift+F1");
        let plain: KeyChord = "Tab".parse().unwrap();
        assert_eq!(plain.to_string(), "Tab");
    }

    #[test]
    fn a_pressed_key_becomes_the_chord_that_reads_back_the_same() {
        let mods = Mods {
            command: true,
            alt: true,
            ..Mods::default()
        };
        let chord = KeyChord::from_press(&KeyName("F5".into()), mods);
        assert_eq!(chord.to_string(), "Ctrl+Alt+F5");
        assert_eq!(chord.to_string().parse::<KeyChord>().unwrap(), chord);
        assert_eq!(chord.key, "F5");
    }

    #[test]
    fn a_chord_with_no_key_or_a_strange_modifier_is_an_error() {
        assert_eq!("Ctrl+".parse::<KeyChord>(), Err(KeyChordError::NoKey));
        assert_eq!("".parse::<KeyChord>(), Err(KeyChordError::NoKey));
        assert_eq!(
            "Super+A".parse::<KeyChord>(),
            Err(KeyChordError::NotAModifier("Super".into()))
        );
    }

    #[test]
    fn a_binding_keeps_its_chord_and_its_steps_as_words() {
        let binding = KeyBinding {
            name: "heal".into(),
            chord: Some("Alt+Q".parse().unwrap()),
            pad: Some("LeftTrigger+South".parse().unwrap()),
            steps: vec![
                MacroStep::new("cast", "Greater Heal"),
                MacroStep::new("delay", "500"),
            ],
        };
        let text = toml::to_string(&binding).unwrap();
        assert!(text.contains("chord = \"Alt+Q\""));
        assert!(text.contains("pad = \"LeftTrigger+South\""));
        assert_eq!(toml::from_str::<KeyBinding>(&text).unwrap(), binding);
        let bare = KeyBinding {
            chord: Some("F1".parse().unwrap()),
            ..KeyBinding::default()
        };
        let text = toml::to_string(&bare).unwrap();
        assert_eq!(toml::from_str::<KeyBinding>(&text).unwrap(), bare);
    }

    #[test]
    fn a_pad_chord_reads_its_held_buttons_and_its_button() {
        let chord: PadChord = "LeftTrigger + South".parse().unwrap();
        assert_eq!(chord.held, vec!["LeftTrigger".to_string()]);
        assert_eq!(chord.button, "South");
        assert_eq!(chord.to_string(), "LeftTrigger+South");
        assert_eq!("".parse::<PadChord>(), Err(KeyChordError::NoKey));
        assert_eq!("+South".parse::<PadChord>(), Err(KeyChordError::NoKey));
    }
}

#[cfg(test)]
mod press_tests {
    use super::*;
    use crate::settings::Profile;

    #[test]
    fn a_press_becomes_the_saved_chord_words() {
        // On Linux and Windows, Ctrl sets both `ctrl` and `command`.
        let mods = Mods {
            ctrl: true,
            command: true,
            shift: true,
            ..Mods::default()
        };
        let chord = KeyChord::from_press(&KeyName("F1".into()), mods);
        assert_eq!(chord.to_string(), "Ctrl+Shift+F1");
    }

    #[test]
    fn a_saved_profile_reads_the_same_after_the_move() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/profile_before_move.toml"
        ))
        .unwrap();
        let profile: Profile = toml::from_str(&text).unwrap();
        assert_eq!(toml::to_string(&profile).unwrap(), text);
    }
}
