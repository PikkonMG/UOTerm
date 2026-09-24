//! The agent windows: which runtime agents each window drives, and their
//! settings as the `agents` tool gives them. The session owns the agents
//! and keeps their settings; a window only reads them and asks for a
//! change with the agent tools.

use crate::window::control::Act;
use crate::window::settings::AgentPanelOptions;
use serde_json::{json, Map, Value};

const KEY_ON: &str = "on";
const KEY_JOB: &str = "job";
const KEY_SETTINGS: &str = "settings";
pub const KEY_ACTIVE: &str = "active";
const KEY_LISTS: &str = "lists";
pub const KEY_ITEMS: &str = "items";
const KEY_FRIENDS: &str = "friends";
pub const KEY_NAME: &str = "name";
pub const KEY_GRAPHIC: &str = "graphic";
pub const KEY_COLOR: &str = "color";
pub const KEY_AMOUNT: &str = "amount";
pub const KEY_LAYER: &str = "layer";
pub const KEY_SERIAL: &str = "serial";
/// The list an agent with item lists uses when none is chosen.
pub const DEFAULT_LIST: &str = "default";

/// The runtime names of the agents.
pub const AUTOLOOT: &str = "autoloot";
pub const SCAVENGER: &str = "scavenger";
pub const BANDAGE: &str = "bandage";
pub const SELF_HEAL: &str = "self_heal";
pub const BUY: &str = "buy";
pub const SELL: &str = "sell";
pub const DRESS: &str = "dress";
pub const UNDRESS: &str = "undress";
pub const ORGANIZER: &str = "organizer";
pub const CARVER: &str = "carver";
pub const BONE_CUTTER: &str = "bone_cutter";
pub const REMOUNT: &str = "remount";
pub const FRIENDS: &str = "friends";

/// The heal spells the self-heal window offers, by number: Heal and
/// Greater Heal.
pub const HEAL_SPELLS: [(u16, &str); 2] = [(4, "Heal"), (29, "Greater Heal")];
pub const KEY_HEAL_SPELL: &str = "heal_spell";
/// Whom the bandage agent heals, by its runtime word, and the words for it.
pub const BANDAGE_WHOM: [(&str, &str); 3] = [
    ("self_only", "Me"),
    ("friend", "Friends"),
    ("friend_or_self", "Friends, then me"),
];
pub const KEY_WHOM: &str = "whom";
pub const KEY_MOUNT: &str = "mount";
pub const KEY_BLADE: &str = "blade";
pub const KEY_SOURCE: &str = "source";
pub const KEY_DESTINATION: &str = "destination";

const DELAY_NUMBER: (&str, &str, u64) = ("delay_ms", "Delay (ms)", 100);
const RANGE_NUMBER: (&str, &str, u64) = ("range", "Range (tiles)", 1);
const HEAL_PERCENT_NUMBER: (&str, &str, u64) = ("hp_pct", "Heal under (%)", 5);

/// One agent window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AgentPanel {
    Loot,
    Scavenger,
    Bandage,
    SelfHeal,
    Buy,
    Sell,
    Dress,
    Organizer,
    Skinning,
    Remount,
    Friends,
}

/// How a window keeps the items its agent looks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lists {
    None,
    /// Named item lists, one of them in use: loot, scavenge, buy, sell.
    Items(&'static str),
    /// Named lists run as a job: the organizer, dress.
    Jobs(&'static str),
}

impl AgentPanel {
    pub const ALL: [Self; 11] = [
        Self::Loot,
        Self::Scavenger,
        Self::Bandage,
        Self::SelfHeal,
        Self::Buy,
        Self::Sell,
        Self::Dress,
        Self::Organizer,
        Self::Skinning,
        Self::Remount,
        Self::Friends,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Loot => "Loot",
            Self::Scavenger => "Scavenger",
            Self::Bandage => "Bandage",
            Self::SelfHeal => "Self heal",
            Self::Buy => "Buy",
            Self::Sell => "Sell",
            Self::Dress => "Dress",
            Self::Organizer => "Organizer",
            Self::Skinning => "Skinning",
            Self::Remount => "Remount",
            Self::Friends => "Friends",
        }
    }

    /// The id of the window's place and of its open mark.
    pub fn place_id(self) -> String {
        format!("agent:{}", self.title().to_lowercase().replace(' ', "_"))
    }

    /// True when the Agents page offers this window.
    pub fn offered(self, options: &AgentPanelOptions) -> bool {
        match self {
            Self::Loot => options.loot,
            Self::Scavenger => options.scavenger,
            Self::Bandage => options.bandage,
            Self::SelfHeal => options.self_heal,
            Self::Buy => options.buy,
            Self::Sell => options.sell,
            Self::Dress => options.dress,
            Self::Organizer => options.organizer,
            Self::Skinning => options.skinning,
            Self::Remount => options.remount,
            Self::Friends => options.friends,
        }
    }

    /// The runtime agents the window switches on and off.
    pub fn switches(self) -> &'static [&'static str] {
        match self {
            Self::Loot => &[AUTOLOOT],
            Self::Scavenger => &[SCAVENGER],
            Self::Bandage => &[BANDAGE],
            Self::SelfHeal => &[SELF_HEAL],
            Self::Buy => &[BUY],
            Self::Sell => &[SELL],
            Self::Skinning => &[CARVER, BONE_CUTTER],
            Self::Remount => &[REMOUNT],
            Self::Dress | Self::Organizer | Self::Friends => &[],
        }
    }

    pub fn lists(self) -> Lists {
        match self {
            Self::Loot => Lists::Items(AUTOLOOT),
            Self::Scavenger => Lists::Items(SCAVENGER),
            Self::Buy => Lists::Items(BUY),
            Self::Sell => Lists::Items(SELL),
            Self::Dress => Lists::Jobs(DRESS),
            Self::Organizer => Lists::Jobs(ORGANIZER),
            _ => Lists::None,
        }
    }

    /// The whole-number settings the window shows: their runtime name, the
    /// words for them, and the step of one press.
    pub fn numbers(self) -> &'static [(&'static str, &'static str, u64)] {
        match self {
            Self::Loot | Self::Scavenger => &[DELAY_NUMBER, RANGE_NUMBER],
            Self::Bandage => &[HEAL_PERCENT_NUMBER, RANGE_NUMBER],
            Self::SelfHeal => &[HEAL_PERCENT_NUMBER],
            Self::Remount => &[DELAY_NUMBER],
            _ => &[],
        }
    }

    /// The switches of the settings the window shows, by runtime name.
    pub fn flags(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Loot => &[
                ("while_hidden", "Loot while hidden"),
                ("no_open_corpse", "Leave corpses shut"),
            ],
            Self::Scavenger => &[("while_hidden", "Pick up while hidden")],
            Self::Bandage => &[
                ("skip_poisoned", "Skip the poisoned"),
                ("skip_when_hidden", "Not while hidden"),
            ],
            Self::SelfHeal => &[
                ("cure_poison", "Cure poison"),
                ("skip_when_hidden", "Not while hidden"),
            ],
            Self::Buy => &[("complete_amount", "Buy up to the amount")],
            Self::Friends => &[
                ("include_party", "The party are friends"),
                ("prevent_attack", "Never attack a friend"),
                ("accept_party", "Join a friend's party"),
            ],
            _ => &[],
        }
    }

    /// The runtime agent whose settings the numbers and the flags are.
    pub fn settings_agent(self) -> Option<&'static str> {
        match self {
            Self::Loot => Some(AUTOLOOT),
            Self::Scavenger => Some(SCAVENGER),
            Self::Bandage => Some(BANDAGE),
            Self::SelfHeal => Some(SELF_HEAL),
            Self::Buy => Some(BUY),
            Self::Sell => Some(SELL),
            Self::Remount => Some(REMOUNT),
            Self::Friends => Some(FRIENDS),
            Self::Dress | Self::Organizer | Self::Skinning => None,
        }
    }
}

/// One item rule of a list, as a window shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuleRow {
    pub name: String,
    pub graphic: Option<u16>,
    pub color: Option<u16>,
    pub amount: Option<u32>,
}

impl RuleRow {
    pub fn to_value(&self) -> Value {
        json!({
            KEY_NAME: self.name,
            KEY_GRAPHIC: self.graphic,
            KEY_COLOR: self.color,
            KEY_AMOUNT: self.amount,
        })
    }

    fn from_value(value: &Value) -> Self {
        let number = |key: &str| value.get(key).and_then(Value::as_u64);
        Self {
            name: value
                .get(KEY_NAME)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            graphic: number(KEY_GRAPHIC).and_then(|n| u16::try_from(n).ok()),
            color: number(KEY_COLOR).and_then(|n| u16::try_from(n).ok()),
            amount: number(KEY_AMOUNT).and_then(|n| u32::try_from(n).ok()),
        }
    }
}

/// The answer of the `agents` tool.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AgentsView {
    status: Value,
}

impl AgentsView {
    pub fn new(status: &Value) -> Self {
        Self {
            status: status.clone(),
        }
    }

    pub fn is_on(&self, agent: &str) -> bool {
        self.status
            .get(KEY_ON)
            .and_then(Value::as_array)
            .is_some_and(|on| on.iter().any(|name| name.as_str() == Some(agent)))
    }

    /// The job that runs now, by its agent's name.
    pub fn job(&self) -> Option<&str> {
        self.status.get(KEY_JOB).and_then(Value::as_str)
    }

    pub fn settings(&self, agent: &str) -> Option<&Value> {
        self.status.get(KEY_SETTINGS)?.get(agent)
    }

    pub fn number(&self, agent: &str, key: &str) -> Option<u64> {
        self.settings(agent)?.get(key)?.as_u64()
    }

    pub fn flag(&self, agent: &str, key: &str) -> bool {
        self.settings(agent)
            .and_then(|settings| settings.get(key))
            .and_then(Value::as_bool)
            .unwrap_or_default()
    }

    /// The agent's settings with one field changed, for `agent_set`.
    pub fn with_field(&self, agent: &str, key: &str, value: Value) -> Value {
        let mut settings = self
            .settings(agent)
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        if let Some(fields) = settings.as_object_mut() {
            fields.insert(key.to_string(), value);
        }
        settings
    }

    /// The names of the lists of an agent that keeps item lists, or of one
    /// whose lists are jobs.
    pub fn list_names(&self, lists: Lists) -> Vec<String> {
        let names = |value: Option<&Value>| -> Vec<String> {
            value
                .and_then(Value::as_object)
                .map(|lists| lists.keys().cloned().collect())
                .unwrap_or_default()
        };
        match lists {
            Lists::None => Vec::new(),
            Lists::Items(agent) => {
                let mut found = names(self.settings(agent).and_then(|s| s.get(KEY_LISTS)));
                if !found.iter().any(|name| name == DEFAULT_LIST) {
                    found.insert(0, DEFAULT_LIST.to_string());
                }
                found
            }
            Lists::Jobs(agent) => names(self.settings(agent)),
        }
    }

    /// The list in use of an agent that keeps item lists.
    pub fn active_list(&self, agent: &str) -> String {
        self.settings(agent)
            .and_then(|settings| settings.get(KEY_ACTIVE))
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_LIST)
            .to_string()
    }

    /// The items of one list, as JSON values.
    fn list_values(&self, lists: Lists, list: &str) -> Vec<Value> {
        let array = match lists {
            Lists::None => None,
            Lists::Items(agent) => self
                .settings(agent)
                .and_then(|s| s.get(KEY_LISTS)?.get(list)),
            Lists::Jobs(agent) => self
                .settings(agent)
                .and_then(|s| s.get(list)?.get(KEY_ITEMS)),
        };
        array.and_then(Value::as_array).cloned().unwrap_or_default()
    }

    pub fn rules(&self, lists: Lists, list: &str) -> Vec<RuleRow> {
        self.list_values(lists, list)
            .iter()
            .map(RuleRow::from_value)
            .collect()
    }

    /// The settings `agent_set` takes for a list with one more item.
    pub fn list_with(&self, lists: Lists, list: &str, item: Value) -> Value {
        let mut items = self.list_values(lists, list);
        items.push(item);
        self.list_settings(lists, list, items)
    }

    /// The settings `agent_set` takes for a list with one item less.
    pub fn list_without(&self, lists: Lists, list: &str, at: usize) -> Value {
        let mut items = self.list_values(lists, list);
        if at < items.len() {
            items.remove(at);
        }
        self.list_settings(lists, list, items)
    }

    fn list_settings(&self, lists: Lists, list: &str, items: Vec<Value>) -> Value {
        match lists {
            Lists::Jobs(agent) => self.job_list_with(agent, list, KEY_ITEMS, Value::Array(items)),
            Lists::Items(_) | Lists::None => Value::Array(items),
        }
    }

    /// The settings of one job list with one field changed, such as the
    /// bag the organizer moves into.
    pub fn job_list_with(&self, agent: &str, list: &str, key: &str, value: Value) -> Value {
        let mut settings = self
            .settings(agent)
            .and_then(|s| s.get(list))
            .cloned()
            .unwrap_or_else(|| json!({}));
        if let Some(fields) = settings.as_object_mut() {
            fields.insert(key.to_string(), value);
        }
        settings
    }

    /// A number field of one job list, such as the bag it moves into.
    pub fn job_list_number(&self, agent: &str, list: &str, key: &str) -> Option<u64> {
        self.settings(agent)?.get(list)?.get(key)?.as_u64()
    }

    /// The serials of the friends list.
    pub fn friends(&self) -> Vec<u32> {
        self.settings(FRIENDS)
            .and_then(|friends| friends.get(KEY_FRIENDS))
            .and_then(Value::as_array)
            .map(|serials| {
                serials
                    .iter()
                    .filter_map(Value::as_u64)
                    .filter_map(|serial| u32::try_from(serial).ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The friends settings with a serial added, or taken away when it is
    /// in the list.
    pub fn friends_toggled(&self, serial: u32) -> Value {
        let mut friends = self.friends();
        match friends.iter().position(|friend| *friend == serial) {
            Some(at) => {
                friends.remove(at);
            }
            None => friends.push(serial),
        }
        self.with_field(FRIENDS, KEY_FRIENDS, json!(friends))
    }
}

/// The act that sets one field of an agent's settings.
pub fn set_field(view: &AgentsView, agent: &'static str, key: &str, value: Value) -> Act {
    Act::AgentSet {
        agent: agent.to_string(),
        list: None,
        settings: view.with_field(agent, key, value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status() -> Value {
        json!({
            "on": ["autoloot", "bandage"],
            "job": "organizer",
            "settings": {
                "autoloot": {
                    "enabled": true, "delay_ms": 600, "range": 2, "while_hidden": false,
                    "active": "gold",
                    "lists": { "gold": [{ "name": "coins", "graphic": 3821, "color": null, "amount": null }] }
                },
                "organizer": { "reagents": { "source": null, "destination": 5, "delay_ms": 600, "items": [] } },
                "friends": { "friends": [7, 9], "include_party": true }
            }
        })
    }

    #[test]
    fn the_status_says_what_is_on_and_what_runs() {
        let view = AgentsView::new(&status());
        assert!(view.is_on(AUTOLOOT) && !view.is_on(SCAVENGER));
        assert_eq!(view.job(), Some(ORGANIZER));
        assert_eq!(view.number(AUTOLOOT, "delay_ms"), Some(600));
        assert!(view.flag(FRIENDS, "include_party") && !view.flag(AUTOLOOT, "while_hidden"));
        assert_eq!(view.active_list(AUTOLOOT), "gold");
        assert_eq!(view.active_list(SCAVENGER), DEFAULT_LIST);
    }

    #[test]
    fn item_lists_and_job_lists_grow_and_shrink_for_agent_set() {
        let view = AgentsView::new(&status());
        let loot = AgentPanel::Loot.lists();
        assert_eq!(view.list_names(loot), vec!["default", "gold"]);
        assert_eq!(view.rules(loot, "gold")[0].graphic, Some(3821));
        let bone = RuleRow {
            name: "bones".into(),
            graphic: Some(0x0F7E),
            ..RuleRow::default()
        };
        let grown = view.list_with(loot, "gold", bone.to_value());
        assert_eq!(grown.as_array().unwrap().len(), 2);
        assert_eq!(view.list_without(loot, "gold", 0), json!([]));
        let organizer = AgentPanel::Organizer.lists();
        assert_eq!(view.list_names(organizer), vec!["reagents"]);
        let with_item = view.list_with(organizer, "reagents", bone.to_value());
        assert_eq!(with_item["destination"], 5, "the rest of the list stays");
        assert_eq!(
            view.job_list_number(ORGANIZER, "reagents", KEY_DESTINATION),
            Some(5)
        );
        let moved = view.job_list_with(ORGANIZER, "reagents", KEY_DESTINATION, json!(9));
        assert_eq!(
            (moved["destination"].clone(), moved["delay_ms"].clone()),
            (json!(9), json!(600))
        );
        assert_eq!(with_item["items"][0]["graphic"], 0x0F7E);
    }

    #[test]
    fn a_friend_is_added_or_taken_away_and_one_field_changes() {
        let view = AgentsView::new(&status());
        assert_eq!(view.friends(), vec![7, 9]);
        assert_eq!(view.friends_toggled(7)["friends"], json!([9]));
        assert_eq!(view.friends_toggled(3)["friends"], json!([7, 9, 3]));
        assert_eq!(view.friends_toggled(3)["include_party"], true);
        let act = set_field(&view, AUTOLOOT, "range", json!(4));
        let Act::AgentSet { settings, .. } = act else {
            panic!("an agent set");
        };
        assert_eq!(
            (settings["range"].clone(), settings["delay_ms"].clone()),
            (json!(4), json!(600))
        );
        assert_eq!(
            view.with_field(SCAVENGER, "range", json!(1)),
            json!({ "range": 1 })
        );
    }

    #[test]
    fn each_window_is_offered_by_its_option_and_has_a_place() {
        let mut options = AgentPanelOptions::default();
        assert!(AgentPanel::ALL.iter().all(|panel| panel.offered(&options)));
        options.skinning = false;
        assert!(!AgentPanel::Skinning.offered(&options));
        assert_eq!(AgentPanel::SelfHeal.place_id(), "agent:self_heal");
        assert_eq!(AgentPanel::Skinning.switches(), &[CARVER, BONE_CUTTER]);
    }
}
