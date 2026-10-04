//! The names of the hotkeys every character has, whatever he knows. A window
//! offers them for a key; the session builds the hotkeys that carry them.
//! Each name is written once, here: the list and the session tables use
//! the same constants.

/// Makes a constant for each name and the list of them all from one
/// table, so a name cannot be in the list and not have a constant.
macro_rules! fixed_hotkeys {
    ($($constant:ident => $name:literal,)+) => {
        $(pub const $constant: &str = $name;)+

        /// The names of the hotkeys every character has, in the order the
        /// session lists them.
        pub const FIXED_HOTKEY_NAMES: &[&str] = &[$($constant),+];
    };
}

fixed_hotkeys! {
    RESYNC => "Resync",
    PING_SERVER => "Ping Server",
    ACCEPT_PARTY => "Accept Party",
    DECLINE_PARTY => "Decline Party",
    WHERE_AM_I => "Where Am I",
    FLY_ON_OFF => "Fly On/Off",
    USE_LAST_ITEM => "Use Last Item",
    USE_LEFT_HAND => "Use Left Hand",
    USE_RIGHT_HAND => "Use Right Hand",
    SHOW_NAMES_MOBILES => "Show Names Mobiles",
    SHOW_NAMES_CORPSES => "Show Names Corpses",
    MOUNT_DISMOUNT => "Mount / Dismount",
    ALL_COME => "All Come",
    ALL_FOLLOW_ME => "All Follow Me",
    ALL_FOLLOW => "All Follow",
    ALL_GUARD_ME => "All Guard Me",
    ALL_GUARD => "All Guard",
    ALL_KILL => "All Kill",
    ALL_STAY => "All Stay",
    ALL_STOP => "All Stop",
    AUTOLOOT_ONCE => "Autoloot Once",
    DRESS => "Dress",
    UNDRESS => "Undress",
    SAVE_DRESS => "Save Dress",
    BUY_ON => "Buy On",
    BUY_OFF => "Buy Off",
    SELL_ON => "Sell On",
    SELL_OFF => "Sell Off",
    PRIMARY_ABILITY => "Primary Ability",
    SECONDARY_ABILITY => "Secondary Ability",
    STUN => "Stun",
    DISARM => "Disarm",
    CANCEL_ABILITY => "Cancel Ability",
    ATTACK_LAST_TARGET => "Attack Last Target",
    ATTACK_NEAREST_ENEMY => "Attack Nearest Enemy",
    WAR_MODE_ON_OFF => "War Mode On/Off",
    BANDAGE_SELF => "Bandage Self",
    BANDAGE_LAST => "Bandage Last",
    USE_BANDAGE => "Use Bandage",
    CLEAR_LEFT_HAND => "Clear Left Hand",
    CLEAR_RIGHT_HAND => "Clear Right Hand",
    TOGGLE_LEFT_HAND => "Toggle Left Hand",
    TOGGLE_RIGHT_HAND => "Toggle Right Hand",
    ENCHANTED_APPLE => "Enchanted Apple",
    ORANGE_PETALS => "Orange Petals",
    WRATH_GRAPES => "Wrath Grapes",
    ROSE_OF_TRINSIC => "Rose Of Trinsic",
    SMOKE_BOMB => "Smoke Bomb",
    SPELL_STONE => "Spell Stone",
    HEALING_STONE => "Healing Stone",
    MINI_HEAL => "Mini Heal",
    BIG_HEAL => "Big Heal",
    CHIVALRY_HEAL => "Chivalry Heal",
    INTERRUPT => "Interrupt",
    LAST_SPELL => "Last Spell",
    LAST_SPELL_ON_LAST_TARGET => "Last Spell On Last Target",
    LAST_SKILL => "Last Skill",
    HONOR => "Honor",
    SACRIFICE => "Sacrifice",
    VALOR => "Valor",
    TARGET_SELF => "Target Self",
    TARGET_LAST => "Target Last",
    TARGET_SELF_QUEUED => "Target Self Queued",
    TARGET_LAST_QUEUED => "Target Last Queued",
    CANCEL_TARGET => "Cancel Target",
    CLEAR_TARGET_QUEUE => "Clear Target Queue",
    CLEAR_LAST_TARGET => "Clear Last Target",
    CLEAR_LAST_AND_QUEUE => "Clear Last And Queue",
    AUTOLOOT_ON_OFF => "Autoloot On/Off",
    SCAVENGER_ON_OFF => "Scavenger On/Off",
    BANDAGE_HEAL_ON_OFF => "Bandage Heal On/Off",
    AUTO_REMOUNT_ON_OFF => "Auto Remount On/Off",
    BONE_CUTTER_ON_OFF => "Bone Cutter On/Off",
    AUTO_CARVER_ON_OFF => "Auto Carver On/Off",
    OPEN_CORPSES_ON_OFF => "Open Corpses On/Off",
    DAMAGE_METER_START => "Damage Meter Start",
    DAMAGE_METER_PAUSE => "Damage Meter Pause",
    DAMAGE_METER_RESUME => "Damage Meter Resume",
    DAMAGE_METER_STOP => "Damage Meter Stop",
    STOP_ALL_SCRIPTS => "Stop All Scripts",
}

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
