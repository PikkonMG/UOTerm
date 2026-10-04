//! Input as the shared rules see it: keys by their egui names (the names
//! saved profiles use), the modifier keys, and the pointer buttons.

use serde::{Deserialize, Serialize};

/// The modifier keys held during an input event. A key left out of the
/// words of a client is not held.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    /// Ctrl on Windows and Linux, the Command key on a Mac.
    pub command: bool,
}

impl Mods {
    /// True when no modifier key is held.
    pub fn is_none(self) -> bool {
        self == Self::default()
    }
}

/// A key by the egui name that saved profiles use, such as `F1`, `A` or
/// `Up`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct KeyName(pub String);

/// One key event.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct KeyPress {
    pub key: KeyName,
    pub mods: Mods,
    /// True when the key goes down, false when it comes up.
    pub pressed: bool,
    /// True when the system repeats a held key.
    pub repeat: bool,
}

/// A pointer button.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PointerButton {
    #[default]
    Primary,
    Secondary,
    Middle,
}
