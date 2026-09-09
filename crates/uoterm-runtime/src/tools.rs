use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uoterm_protocol::types::Point3;

pub const TOOL_OBSERVE: &str = "observe";
pub const TOOL_LOOK_AROUND: &str = "look_around";
pub const TOOL_FIND_MOBILES: &str = "find_mobiles";
pub const TOOL_FIND_ITEMS: &str = "find_items";
pub const TOOL_JOURNAL_SEARCH: &str = "journal_search";
pub const TOOL_MAP_TILE: &str = "map_tile";
pub const TOOL_CAN_WALK: &str = "can_walk";
pub const TOOL_SAY: &str = "say";
pub const TOOL_WHISPER: &str = "whisper";
pub const TOOL_EMOTE: &str = "emote";
pub const TOOL_MOVE_TO: &str = "move_to";
pub const TOOL_WALK: &str = "walk";
pub const TOOL_OPEN_DOOR: &str = "open_door";
pub const TOOL_FOLLOW: &str = "follow";
pub const TOOL_STOP: &str = "stop";
pub const TOOL_USE: &str = "use";
pub const TOOL_SINGLE_CLICK: &str = "single_click";
pub const TOOL_ATTACK: &str = "attack";
pub const TOOL_WAR_MODE: &str = "war_mode";
pub const TOOL_LIFT: &str = "lift";
pub const TOOL_DROP: &str = "drop";
pub const TOOL_EQUIP: &str = "equip";
pub const TOOL_UNEQUIP: &str = "unequip";
pub const TOOL_CAST: &str = "cast";
pub const TOOL_USE_SKILL: &str = "use_skill";
pub const TOOL_WAIT_TARGET: &str = "wait_target";
pub const TOOL_TARGET: &str = "target";
pub const TOOL_OPEN_CONTAINER: &str = "open_container";
pub const TOOL_LOOT: &str = "loot";
pub const TOOL_TRADE_OFFER: &str = "trade_offer";
pub const TOOL_GUMP_RESPOND: &str = "gump_respond";
pub const TOOL_GUMP_CLOSE: &str = "gump_close";
pub const TOOL_SET_GOAL: &str = "set_goal";
pub const TOOL_CANCEL_GOAL: &str = "cancel_goal";
pub const TOOL_SET_PERSONA: &str = "set_persona";

pub const BANK_X: u16 = 1425;
pub const BANK_Y: u16 = 1695;
pub const BANK_Z: i8 = 0;

pub fn new_action_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub args: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub ok: bool,
    pub result: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
}

impl ToolResult {
    pub fn ok(result: Value) -> Self {
        Self {
            ok: true,
            result,
            error: None,
            action_id: Some(new_action_id()),
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            result: Value::Null,
            error: Some(msg.into()),
            action_id: None,
        }
    }

    pub fn action(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            ok: true,
            result: json!({ "action_id": id }),
            error: None,
            action_id: Some(id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Goal {
    Idle,
    Travel { dest: Point3 },
    Hunt,
    Gather,
    Bank,
    Shop,
    Social,
    Flee,
    Ress,
}

impl Goal {
    pub fn parse(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "hunt" => Self::Hunt,
            "gather" | "chop" | "mine" => Self::Gather,
            "bank" => Self::Bank,
            "shop" => Self::Shop,
            "social" => Self::Social,
            "flee" => Self::Flee,
            "ress" | "res" => Self::Ress,
            "travel" => Self::Travel {
                dest: Point3::new(BANK_X, BANK_Y, BANK_Z),
            },
            _ => Self::Idle,
        }
    }

    pub fn for_class(class: &str) -> Self {
        match class {
            "lumberjack" | "miner" | "gatherer" => Self::Gather,
            "warrior" | "pk" => Self::Hunt,
            "traveler" => Self::Travel {
                dest: Point3::new(BANK_X, BANK_Y, BANK_Z),
            },
            "sitter" | "banker" | "banker_idle" => Self::Social,
            _ => Self::Idle,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Travel { .. } => "travel",
            Self::Hunt => "hunt",
            Self::Gather => "gather",
            Self::Bank => "bank",
            Self::Shop => "shop",
            Self::Social => "social",
            Self::Flee => "flee",
            Self::Ress => "ress",
        }
    }
}

const TOOLS: &[(&str, &str, &str)] = &[
    (
        TOOL_OBSERVE,
        "Compact world snapshot, ASCII radar, recent journal.",
        "session exists",
    ),
    (
        TOOL_LOOK_AROUND,
        "Describe the surroundings in words: the surface underfoot, named furniture and walls from the map files, loose items, and people. Takes an optional radius in tiles.",
        "in world",
    ),
    (TOOL_FIND_MOBILES, "Filter nearby mobiles.", "in world"),
    (
        TOOL_FIND_ITEMS,
        "Filter items on the ground and inside containers. Args: graphic, name (part of the display name), container (a container serial, to search only that one).",
        "in world",
    ),
    (
        TOOL_JOURNAL_SEARCH,
        "Search journal text.",
        "session exists",
    ),
    (
        TOOL_MAP_TILE,
        "Tile kind and Z at x,y.",
        "map loaded or mock grid",
    ),
    (
        TOOL_CAN_WALK,
        "True if the tile is walkable.",
        "map loaded or mock grid",
    ),
    (
        TOOL_SAY,
        "Speak. Persona blocks *emotes* and rate-limits.",
        "in world",
    ),
    (TOOL_WHISPER, "Whisper.", "in world"),
    (
        TOOL_EMOTE,
        "Emote only if persona.allow_emote is true.",
        "persona allows emotes",
    ),
    (
        TOOL_MOVE_TO,
        "Pathfind and send 0x02 walk/run packets toward dest.",
        "in world, walkable dest",
    ),
    (
        TOOL_WALK,
        "Send 0x02 steps in a direction (Classic Client hold-right-click analog). Args: direction, running, hold_ms.",
        "in world",
    ),
    (
        TOOL_OPEN_DOOR,
        "Open the door you face (packet 0x12/0x58). Stand next to the door first.",
        "in world, adjacent to a door",
    ),
    (TOOL_FOLLOW, "Follow a mobile serial.", "mobile in range"),
    (TOOL_STOP, "Clear path and set idle.", "session exists"),
    (TOOL_USE, "Double-click serial.", "serial exists"),
    (TOOL_SINGLE_CLICK, "Single-click for name.", "serial exists"),
    (TOOL_ATTACK, "Attack mobile.", "mobile serial"),
    (TOOL_WAR_MODE, "Set war/peace.", "in world"),
    (TOOL_LIFT, "Pick up item.", "item serial"),
    (TOOL_DROP, "Drop item.", "serial"),
    (TOOL_EQUIP, "Equip item on self.", "item serial"),
    (TOOL_UNEQUIP, "Unequip into backpack.", "layer occupied"),
    (TOOL_CAST, "Cast spell id.", "enough mana"),
    (TOOL_USE_SKILL, "Use skill id.", "in world"),
    (
        TOOL_WAIT_TARGET,
        "True if a target cursor is pending.",
        "none",
    ),
    (
        TOOL_TARGET,
        "Answer the pending target cursor.",
        "must have a target cursor",
    ),
    (TOOL_OPEN_CONTAINER, "Open container serial.", "item serial"),
    (
        TOOL_LOOT,
        "Walk to a corpse, open it, lift each stack and drop it in the pack.",
        "corpse serial",
    ),
    (TOOL_TRADE_OFFER, "Secure trade offer.", "mobile serial"),
    (
        TOOL_GUMP_RESPOND,
        "Click a gump button. button 0 closes.",
        "open gump",
    ),
    (TOOL_GUMP_CLOSE, "Close gump.", "open gump"),
    (
        TOOL_SET_GOAL,
        "High-level goal: idle, travel, hunt, gather, bank, shop, social, flee, ress.",
        "in world",
    ),
    (
        TOOL_CANCEL_GOAL,
        "Set goal idle and stop movement.",
        "session exists",
    ),
];

pub fn mcp_tool_list() -> Value {
    let tools: Vec<Value> = TOOLS
        .iter()
        .map(|(name, desc, pre)| {
            json!({
                "name": name,
                "description": format!("{desc} Precondition: {pre}"),
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "session_id": {"type": "string"},
                        "text": {"type": "string"},
                        "serial": {"type": "string"},
                        "x": {"type": "integer"},
                        "y": {"type": "integer"},
                        "z": {"type": "integer"},
                        "goal": {"type": "string"},
                        "direction": {"type": "string"},
                        "running": {"type": "boolean"},
                        "hold_ms": {"type": "integer"},
                        "force": {"type": "boolean"},
                        "radius": {"type": "integer"},
                        "name": {"type": "string"},
                        "graphic": {"type": "integer"},
                        "container": {"type": "string"},
                        "distance": {"type": "integer"}
                    }
                }
            })
        })
        .collect();
    json!({ "tools": tools })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn craft_is_not_a_goal() {
        assert_eq!(Goal::parse("craft"), Goal::Idle);
        assert_eq!(Goal::parse("craft").name(), Goal::Idle.name());
        let desc = TOOLS
            .iter()
            .find(|(name, _, _)| *name == TOOL_SET_GOAL)
            .map(|(_, d, _)| *d)
            .unwrap();
        assert!(!desc.contains("craft"));
    }
}
