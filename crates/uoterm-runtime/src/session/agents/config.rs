//! The settings of every agent, as a user or an agent writes them. They are
//! kept in one TOML file per character.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uoterm_protocol::Serial;

/// How long an agent waits between two moves when its list names no time.
pub const DEFAULT_DELAY_MS: u64 = 600;
/// How far an agent reaches when its list names no range.
pub const DEFAULT_RANGE: u32 = 2;
/// The health share, in percent, under which the bandage agent heals.
pub const DEFAULT_HEAL_PCT: u8 = 80;

/// One kind of item an agent looks for. A field left out matches anything.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemRule {
    /// A label for people; it takes no part in the match.
    pub name: String,
    pub graphic: Option<u16>,
    pub color: Option<u16>,
    /// How many: the most to move, the level to restock to, or the most to
    /// sell or buy. None means all.
    pub amount: Option<u32>,
    /// A bag for this item only, in place of the agent's bag.
    pub bag: Option<Serial>,
    /// Properties the item must have, each within its range.
    pub properties: Vec<PropertyRule>,
    /// A rule switched off stays in the list and matches nothing.
    pub disabled: bool,
}

impl ItemRule {
    pub fn matches(&self, graphic: u16, color: u16) -> bool {
        !self.disabled
            && self.graphic.map_or(true, |g| g == graphic)
            && self.color.map_or(true, |c| c == color)
    }
}

/// A property an item must carry, such as "Faster Casting" from 1 to 2.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PropertyRule {
    pub name: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// The name of the list an agent uses when none is chosen.
pub const DEFAULT_LIST: &str = "default";

fn default_list() -> String {
    DEFAULT_LIST.into()
}

/// An agent's item lists by name, and the one in use.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemLists {
    pub active: String,
    pub lists: BTreeMap<String, Vec<ItemRule>>,
}

impl Default for ItemLists {
    fn default() -> Self {
        Self {
            active: default_list(),
            lists: BTreeMap::new(),
        }
    }
}

impl ItemLists {
    /// The rules of the list in use. None when it has none.
    pub fn rules(&self) -> &[ItemRule] {
        self.lists.get(&self.active).map_or(&[], Vec::as_slice)
    }
}

/// Looting corpses as they fall.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LootAgent {
    pub enabled: bool,
    pub delay_ms: u64,
    pub range: u32,
    /// Where loot goes. The backpack when none is set.
    pub bag: Option<Serial>,
    /// Loot with the corpse left shut: its contents must be known already.
    pub no_open_corpse: bool,
    pub while_hidden: bool,
    #[serde(flatten)]
    pub items: ItemLists,
}

impl Default for LootAgent {
    fn default() -> Self {
        Self {
            enabled: false,
            delay_ms: DEFAULT_DELAY_MS,
            range: DEFAULT_RANGE,
            bag: None,
            no_open_corpse: false,
            while_hidden: false,
            items: ItemLists::default(),
        }
    }
}

/// Picking up items from the ground.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScavengerAgent {
    pub enabled: bool,
    pub delay_ms: u64,
    pub range: u32,
    pub bag: Option<Serial>,
    pub while_hidden: bool,
    #[serde(flatten)]
    pub items: ItemLists,
}

impl Default for ScavengerAgent {
    fn default() -> Self {
        Self {
            enabled: false,
            delay_ms: DEFAULT_DELAY_MS,
            range: DEFAULT_RANGE,
            bag: None,
            while_hidden: false,
            items: ItemLists::default(),
        }
    }
}

/// A list that moves items from one bag to another: the organizer moves
/// the listed items, and restock tops the listed items up to their amount.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MoveList {
    /// The bag to take from. The bank box when none is set and the list
    /// restocks; the backpack for the organizer.
    pub source: Option<Serial>,
    /// The bag to put into. The backpack when none is set and the list
    /// restocks.
    pub destination: Option<Serial>,
    pub delay_ms: u64,
    pub items: Vec<ItemRule>,
}

impl Default for MoveList {
    fn default() -> Self {
        Self {
            source: None,
            destination: None,
            delay_ms: DEFAULT_DELAY_MS,
            items: Vec::new(),
        }
    }
}

/// One item of a dress list: what to wear on a layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DressItem {
    pub layer: u8,
    pub serial: Serial,
}

/// Worn items to put on or take off together.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DressList {
    pub delay_ms: u64,
    /// Where taken-off items go. The backpack when none is set.
    pub undress_bag: Option<Serial>,
    /// Take off what is already on a layer before putting on the list's item.
    pub replace_worn: bool,
    pub items: Vec<DressItem>,
}

impl Default for DressList {
    fn default() -> Self {
        Self {
            delay_ms: DEFAULT_DELAY_MS,
            undress_bag: None,
            replace_worn: true,
            items: Vec::new(),
        }
    }
}

/// Buying from a vendor as its buy list opens.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BuyAgent {
    pub enabled: bool,
    /// Buy only what the backpack is short of each item's amount.
    pub complete_amount: bool,
    #[serde(flatten)]
    pub items: ItemLists,
}

/// Selling to a vendor as its sell list opens.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SellAgent {
    pub enabled: bool,
    /// Sell only from this bag and the bags in it. The backpack when unset.
    pub bag: Option<Serial>,
    #[serde(flatten)]
    pub items: ItemLists,
}

/// Whom the bandage agent heals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealWhom {
    #[default]
    SelfOnly,
    /// The most hurt friend in range.
    Friend,
    /// The most hurt friend in range, or the character when none is hurt.
    FriendOrSelf,
    /// One mobile, by serial.
    Target(Serial),
}

/// Bandaging the character or friends when hurt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BandageAgent {
    pub enabled: bool,
    pub whom: HealWhom,
    pub hp_pct: u8,
    pub range: u32,
    /// A fixed time between bandages. When unset the time follows the
    /// character's dexterity, as the game's own bandage time does.
    pub delay_ms: Option<u64>,
    pub skip_poisoned: bool,
    pub skip_when_hidden: bool,
    /// A bandage of another graphic or colour, for a shard with its own.
    pub bandage_graphic: Option<u16>,
    pub bandage_color: Option<u16>,
}

impl Default for BandageAgent {
    fn default() -> Self {
        Self {
            enabled: false,
            whom: HealWhom::SelfOnly,
            hp_pct: DEFAULT_HEAL_PCT,
            range: DEFAULT_RANGE,
            delay_ms: None,
            skip_poisoned: false,
            skip_when_hidden: true,
            bandage_graphic: None,
            bandage_color: None,
        }
    }
}

/// The people the character counts as friends.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FriendsAgent {
    pub friends: Vec<Serial>,
    /// Party members count as friends.
    pub include_party: bool,
    /// Refuse to attack a friend.
    pub prevent_attack: bool,
    /// Join a party a friend asks the character into.
    pub accept_party: bool,
}

/// Mounting again after being knocked off.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RemountAgent {
    pub enabled: bool,
    /// The mount: a pet mobile or an ethereal item.
    pub mount: Option<Serial>,
    pub delay_ms: u64,
}

impl Default for RemountAgent {
    fn default() -> Self {
        Self {
            enabled: false,
            mount: None,
            delay_ms: DEFAULT_DELAY_MS,
        }
    }
}

/// Using a blade on bone piles or on fresh corpses.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BladeAgent {
    pub enabled: bool,
    pub blade: Option<Serial>,
}

/// Opening each new corpse in range.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenCorpsesAgent {
    pub enabled: bool,
    pub range: u32,
}

impl Default for OpenCorpsesAgent {
    fn default() -> Self {
        Self {
            enabled: false,
            range: DEFAULT_RANGE,
        }
    }
}

/// How a target filter picks one mobile from those that pass it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Selector {
    #[default]
    Nearest,
    Farthest,
    /// The one with the least health left.
    Weakest,
    Strongest,
    Random,
    /// The next one after the last pick, nearest first.
    Next,
    Previous,
}

/// A named way to pick a target: by notoriety, body, colour, range and flags.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TargetFilter {
    /// Notorieties by name: innocent, friend, gray, criminal, enemy,
    /// murderer, invulnerable. Empty means any.
    pub notorieties: Vec<String>,
    pub bodies: Vec<u16>,
    pub colors: Vec<u16>,
    /// Part of the name, in any case.
    pub name: Option<String>,
    pub range_min: Option<u32>,
    pub range_max: Option<u32>,
    /// Each set flag must match; an unset one does not matter.
    pub poisoned: Option<bool>,
    pub human: Option<bool>,
    pub ghost: Option<bool>,
    pub war: Option<bool>,
    pub friend: Option<bool>,
    pub paralyzed: Option<bool>,
    pub selector: Selector,
}

/// Every agent's settings.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentsConfig {
    pub autoloot: LootAgent,
    pub scavenger: ScavengerAgent,
    pub organizer: BTreeMap<String, MoveList>,
    pub restock: BTreeMap<String, MoveList>,
    pub dress: BTreeMap<String, DressList>,
    pub buy: BuyAgent,
    pub sell: SellAgent,
    pub bandage: BandageAgent,
    pub friends: FriendsAgent,
    pub remount: RemountAgent,
    pub bone_cutter: BladeAgent,
    pub carver: BladeAgent,
    pub open_corpses: OpenCorpsesAgent,
    pub targets: BTreeMap<String, TargetFilter>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rule_left_open_matches_anything_and_a_disabled_one_nothing() {
        const GOLD: u16 = 0x0EED;
        let any = ItemRule::default();
        assert!(any.matches(GOLD, 0));
        let gold = ItemRule {
            graphic: Some(GOLD),
            ..ItemRule::default()
        };
        assert!(gold.matches(GOLD, 5) && !gold.matches(0x0F0C, 0));
        let off = ItemRule {
            disabled: true,
            ..ItemRule::default()
        };
        assert!(!off.matches(GOLD, 0));
    }

    #[test]
    fn a_short_file_fills_in_the_rest() {
        let text = r#"
[autoloot]
enabled = true
lists.default = [{ name = "gold", graphic = 3821 }]

[organizer.reagents]
items = [{ graphic = 3962 }]
"#;
        let config: AgentsConfig = toml::from_str(text).expect("agents file");
        assert!(config.autoloot.enabled);
        assert_eq!(config.autoloot.delay_ms, DEFAULT_DELAY_MS);
        assert_eq!(config.autoloot.items.rules()[0].graphic, Some(3821));
        assert_eq!(config.organizer["reagents"].items.len(), 1);
        assert!(!config.bandage.enabled);
        let back = toml::to_string(&config).expect("written back");
        assert_eq!(
            toml::from_str::<AgentsConfig>(&back).expect("read again"),
            config
        );
    }
}
