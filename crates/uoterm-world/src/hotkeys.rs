//! The names of the hotkeys every character has, whatever he knows. A window
//! offers them for a key; the session builds the hotkeys that carry them.

/// The names of the hotkeys every character has, in the order the session
/// lists them.
pub const FIXED_HOTKEY_NAMES: &[&str] = &[
    "Resync",
    "Ping Server",
    "Accept Party",
    "Decline Party",
    "Where Am I",
    "Fly On/Off",
    "Use Last Item",
    "Use Left Hand",
    "Use Right Hand",
    "Show Names Mobiles",
    "Show Names Corpses",
    "Mount / Dismount",
    "All Come",
    "All Follow Me",
    "All Follow",
    "All Guard Me",
    "All Guard",
    "All Kill",
    "All Stay",
    "All Stop",
    "Autoloot Once",
    "Dress",
    "Undress",
    "Save Dress",
    "Buy On",
    "Buy Off",
    "Sell On",
    "Sell Off",
    "Primary Ability",
    "Secondary Ability",
    "Stun",
    "Disarm",
    "Cancel Ability",
    "Attack Last Target",
    "Attack Nearest Enemy",
    "War Mode On/Off",
    "Bandage Self",
    "Bandage Last",
    "Use Bandage",
    "Clear Left Hand",
    "Clear Right Hand",
    "Toggle Left Hand",
    "Toggle Right Hand",
    "Enchanted Apple",
    "Orange Petals",
    "Wrath Grapes",
    "Rose Of Trinsic",
    "Smoke Bomb",
    "Spell Stone",
    "Healing Stone",
    "Mini Heal",
    "Big Heal",
    "Chivalry Heal",
    "Interrupt",
    "Last Spell",
    "Last Spell On Last Target",
    "Last Skill",
    "Honor",
    "Sacrifice",
    "Valor",
    "Target Self",
    "Target Last",
    "Target Self Queued",
    "Target Last Queued",
    "Cancel Target",
    "Clear Target Queue",
    "Clear Last Target",
    "Clear Last And Queue",
    "Autoloot On/Off",
    "Scavenger On/Off",
    "Bandage Heal On/Off",
    "Auto Remount On/Off",
    "Bone Cutter On/Off",
    "Auto Carver On/Off",
    "Open Corpses On/Off",
    "Damage Meter Start",
    "Damage Meter Pause",
    "Damage Meter Resume",
    "Damage Meter Stop",
    "Stop All Scripts",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixed_names_hold_each_name_once() {
        let mut names = FIXED_HOTKEY_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), FIXED_HOTKEY_NAMES.len());
    }
}
