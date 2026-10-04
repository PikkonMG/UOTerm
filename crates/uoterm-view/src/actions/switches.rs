//! The options of the profile an action switches on and off, and the
//! words the journal shows for each.

use super::Switch;
use crate::settings::{AuraRule, Profile};

/// The words of an option in the journal.
pub fn switch_words(switch: Switch) -> &'static str {
    match switch {
        Switch::AlwaysRun => "Always run",
        Switch::ClickToRun => "Click to run",
        Switch::CircleOfTransparency => "Circle of transparency",
        Switch::HideRoofs => "Hiding roofs",
        Switch::TreesToStumps => "Trees to stumps",
        Switch::HideVegetation => "Hiding vegetation",
        Switch::CaveTiles => "Cave tile marks",
        Switch::Names => "Names over heads",
        Switch::Aura => "Auras",
        Switch::OutOfRangeColor => "Out of range color",
        Switch::NewTargetSystem => "Target system",
    }
}

/// The journal line after an option was switched.
pub fn switched_words(switch: Switch, on: bool) -> String {
    let state = if on { "on" } else { "off" };
    format!("{} is now {state}.", switch_words(switch))
}

/// Whether an option is on.
pub fn switch_on(profile: &Profile, switch: Switch) -> bool {
    let general = &profile.general;
    match switch {
        Switch::AlwaysRun => general.always_run,
        Switch::ClickToRun => general.click_to_run,
        Switch::CircleOfTransparency => general.circle_of_transparency,
        Switch::HideRoofs => general.hide_roofs,
        Switch::TreesToStumps => general.trees_to_stumps,
        Switch::HideVegetation => general.hide_vegetation,
        Switch::CaveTiles => general.mark_cave_tiles,
        Switch::Names => profile.nameplates.enabled,
        Switch::Aura => general.aura_under_feet != AuraRule::Never,
        Switch::OutOfRangeColor => general.out_of_range_no_color,
        Switch::NewTargetSystem => profile.combat.new_target_system,
    }
}

/// Turns an option on or off. The aura key keeps the rule it turned off
/// in `aura_before`, and brings it back when it turns auras on.
pub fn set_switch(
    profile: &mut Profile,
    switch: Switch,
    on: bool,
    aura_before: &mut Option<AuraRule>,
) {
    let general = &mut profile.general;
    let field = match switch {
        Switch::AlwaysRun => &mut general.always_run,
        Switch::ClickToRun => &mut general.click_to_run,
        Switch::CircleOfTransparency => &mut general.circle_of_transparency,
        Switch::HideRoofs => &mut general.hide_roofs,
        Switch::TreesToStumps => &mut general.trees_to_stumps,
        Switch::HideVegetation => &mut general.hide_vegetation,
        Switch::CaveTiles => &mut general.mark_cave_tiles,
        Switch::Names => &mut profile.nameplates.enabled,
        Switch::OutOfRangeColor => &mut general.out_of_range_no_color,
        Switch::NewTargetSystem => &mut profile.combat.new_target_system,
        Switch::Aura => {
            let rule = &mut general.aura_under_feet;
            if on && *rule == AuraRule::Never {
                *rule = aura_before.take().unwrap_or(AuraRule::Always);
            } else if !on && *rule != AuraRule::Never {
                *aura_before = Some(*rule);
                *rule = AuraRule::Never;
            }
            return;
        }
    };
    *field = on;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_switch_turns_its_own_option_on_and_off() {
        let switches = [
            Switch::AlwaysRun,
            Switch::ClickToRun,
            Switch::CircleOfTransparency,
            Switch::HideRoofs,
            Switch::TreesToStumps,
            Switch::HideVegetation,
            Switch::CaveTiles,
            Switch::Names,
            Switch::Aura,
            Switch::OutOfRangeColor,
            Switch::NewTargetSystem,
        ];
        let mut aura_before = None;
        for switch in switches {
            let mut profile = Profile::default();
            let before = switch_on(&profile, switch);
            set_switch(&mut profile, switch, !before, &mut aura_before);
            assert_eq!(switch_on(&profile, switch), !before, "{switch:?}");
            set_switch(&mut profile, switch, before, &mut aura_before);
            assert_eq!(profile, Profile::default(), "{switch:?}");
        }
    }

    #[test]
    fn the_journal_tells_the_new_state() {
        assert_eq!(
            switched_words(Switch::AlwaysRun, true),
            "Always run is now on."
        );
    }

    #[test]
    fn the_aura_key_brings_back_the_rule_it_turned_off() {
        let mut aura_before = None;
        let mut profile = Profile::default();
        profile.general.aura_under_feet = AuraRule::WarMode;
        set_switch(&mut profile, Switch::Aura, false, &mut aura_before);
        assert_eq!(profile.general.aura_under_feet, AuraRule::Never);
        set_switch(&mut profile, Switch::Aura, true, &mut aura_before);
        assert_eq!(profile.general.aura_under_feet, AuraRule::WarMode);
    }
}
