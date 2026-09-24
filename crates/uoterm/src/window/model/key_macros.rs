//! The macros of the profile by their names, as a spellbook makes one with
//! "Fast spell assign" and a hotbar slot runs one, in both styles.

use crate::window::settings::{KeyBinding, MacroStep};

/// Adds a macro of this name with `steps`, when the profile has none by
/// the name. True when it was added.
pub fn ensure(macros: &mut Vec<KeyBinding>, name: &str, steps: Vec<MacroStep>) -> bool {
    if macros.iter().any(|binding| binding.name == name) {
        return false;
    }
    macros.push(KeyBinding {
        name: name.to_string(),
        steps,
        ..KeyBinding::default()
    });
    true
}

/// The steps of the macro of this name.
pub fn steps_of(macros: &[KeyBinding], name: &str) -> Option<Vec<MacroStep>> {
    macros
        .iter()
        .find(|binding| binding.name == name)
        .map(|binding| binding.steps.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_macro_is_made_once_by_its_name_and_found_by_it() {
        let mut macros = Vec::new();
        let steps = vec![MacroStep::new("cast", "Heal")];
        assert!(ensure(&mut macros, "Heal", steps.clone()));
        assert!(!ensure(&mut macros, "Heal", Vec::new()));
        assert_eq!(macros.len(), 1);
        assert_eq!(steps_of(&macros, "Heal"), Some(steps));
        assert_eq!(steps_of(&macros, "Cure"), None);
    }
}
