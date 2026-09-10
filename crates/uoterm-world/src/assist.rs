//! The assistant features a shard forbids.
//!
//! A shard can send a list of the features an assistant beside the client
//! must not use, one bit for each feature. The bit numbers are the same on
//! every shard that sends the list. The character obeys the list: a feature
//! the shard forbids is a feature the character does not use by itself.

use serde::{Deserialize, Serialize};

/// One feature the shard's list can forbid. The value is its bit number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssistFeature {
    FilterWeather = 0,
    FilterLight = 1,
    SmartTarget = 2,
    RangedTarget = 3,
    AutoOpenDoors = 4,
    UnequipOnCast = 5,
    AutoPotionEquip = 6,
    PoisonedChecks = 7,
    LoopedMacros = 8,
    UseOnceAgent = 9,
    RestockAgent = 10,
    SellAgent = 11,
    BuyAgent = 12,
    PotionHotkeys = 13,
    RandomTargets = 14,
    ClosestTargets = 15,
    OverheadHealth = 16,
    AutolootAgent = 17,
    BoneCutterAgent = 18,
    ScriptMacros = 19,
    AutoRemount = 20,
    AutoBandage = 21,
    EnemyTargetShare = 22,
    FilterSeason = 23,
    SpellTargetShare = 24,
}

impl AssistFeature {
    /// Every feature, in bit order.
    pub const ALL: [AssistFeature; 25] = [
        Self::FilterWeather,
        Self::FilterLight,
        Self::SmartTarget,
        Self::RangedTarget,
        Self::AutoOpenDoors,
        Self::UnequipOnCast,
        Self::AutoPotionEquip,
        Self::PoisonedChecks,
        Self::LoopedMacros,
        Self::UseOnceAgent,
        Self::RestockAgent,
        Self::SellAgent,
        Self::BuyAgent,
        Self::PotionHotkeys,
        Self::RandomTargets,
        Self::ClosestTargets,
        Self::OverheadHealth,
        Self::AutolootAgent,
        Self::BoneCutterAgent,
        Self::ScriptMacros,
        Self::AutoRemount,
        Self::AutoBandage,
        Self::EnemyTargetShare,
        Self::FilterSeason,
        Self::SpellTargetShare,
    ];

    /// The name an agent reads in `observe`.
    pub fn name(self) -> &'static str {
        match self {
            Self::FilterWeather => "filter_weather",
            Self::FilterLight => "filter_light",
            Self::SmartTarget => "smart_target",
            Self::RangedTarget => "ranged_target",
            Self::AutoOpenDoors => "auto_open_doors",
            Self::UnequipOnCast => "unequip_on_cast",
            Self::AutoPotionEquip => "auto_potion_equip",
            Self::PoisonedChecks => "poisoned_checks",
            Self::LoopedMacros => "looped_macros",
            Self::UseOnceAgent => "use_once_agent",
            Self::RestockAgent => "restock_agent",
            Self::SellAgent => "sell_agent",
            Self::BuyAgent => "buy_agent",
            Self::PotionHotkeys => "potion_hotkeys",
            Self::RandomTargets => "random_targets",
            Self::ClosestTargets => "closest_targets",
            Self::OverheadHealth => "overhead_health",
            Self::AutolootAgent => "autoloot_agent",
            Self::BoneCutterAgent => "bone_cutter_agent",
            Self::ScriptMacros => "script_macros",
            Self::AutoRemount => "auto_remount",
            Self::AutoBandage => "auto_bandage",
            Self::EnemyTargetShare => "enemy_target_share",
            Self::FilterSeason => "filter_season",
            Self::SpellTargetShare => "spell_target_share",
        }
    }

    fn bit(self) -> u64 {
        1 << self as u32
    }
}

/// The shard's list of forbidden assistant features. A shard that sends no
/// list forbids nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssistRules {
    disallowed: u64,
}

impl AssistRules {
    pub fn from_bits(disallowed: u64) -> Self {
        Self { disallowed }
    }

    /// True unless the shard forbids the feature.
    pub fn allows(self, feature: AssistFeature) -> bool {
        self.disallowed & feature.bit() == 0
    }

    /// The names of the features the shard forbids, in bit order.
    pub fn forbidden(self) -> Vec<&'static str> {
        AssistFeature::ALL
            .into_iter()
            .filter(|feature| !self.allows(*feature))
            .map(AssistFeature::name)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_list_forbids_nothing() {
        let rules = AssistRules::default();
        assert!(AssistFeature::ALL.into_iter().all(|f| rules.allows(f)));
        assert!(rules.forbidden().is_empty());
    }

    #[test]
    fn each_feature_is_its_own_bit() {
        const AUTO_OPEN_DOORS_BIT: u64 = 1 << 4;
        const AUTO_BANDAGE_BIT: u64 = 1 << 21;
        let rules = AssistRules::from_bits(AUTO_OPEN_DOORS_BIT | AUTO_BANDAGE_BIT);
        assert!(!rules.allows(AssistFeature::AutoOpenDoors));
        assert!(!rules.allows(AssistFeature::AutoBandage));
        assert!(rules.allows(AssistFeature::PotionHotkeys));
        assert_eq!(rules.forbidden(), vec!["auto_open_doors", "auto_bandage"]);
    }

    #[test]
    fn the_list_of_features_is_in_bit_order() {
        for (bit, feature) in AssistFeature::ALL.into_iter().enumerate() {
            assert_eq!(feature as usize, bit);
        }
    }

    #[test]
    fn a_bit_no_feature_names_is_not_listed() {
        const AN_UNNAMED_BIT: u64 = 1 << 40;
        assert!(AssistRules::from_bits(AN_UNNAMED_BIT)
            .forbidden()
            .is_empty());
    }
}
