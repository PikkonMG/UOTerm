use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uoterm_assist::harvest::Harvest;
use uoterm_protocol::types::Point3;

mod args;
use args::{ArgKind, ToolArg, ONE_OF, TOOL_ARGS};

pub const TOOL_OBSERVE: &str = "observe";
pub const TOOL_LOOK_AROUND: &str = "look_around";
pub const TOOL_FIND_MOBILES: &str = "find_mobiles";
pub const TOOL_FIND_ITEMS: &str = "find_items";
pub const TOOL_FIND_LANDMARKS: &str = "find_landmarks";
pub const TOOL_JOURNAL_SEARCH: &str = "journal_search";
pub const TOOL_MAP_TILE: &str = "map_tile";
pub const TOOL_CAN_WALK: &str = "can_walk";
pub const TOOL_LINE_OF_SIGHT: &str = "line_of_sight";
pub const TOOL_PROMPT_ANSWER: &str = "prompt_answer";
pub const TOOL_PARTY: &str = "party";
pub const TOOL_ROUTE: &str = "route";
pub const TOOL_MOBILE_STATUS: &str = "mobile_status";
pub const TOOL_PROMPT_CANCEL: &str = "prompt_cancel";
pub const TOOL_SAY: &str = "say";
pub const TOOL_WHISPER: &str = "whisper";
pub const TOOL_REPLY: &str = "reply";
pub const TOOL_EMOTE: &str = "emote";
pub const TOOL_MOVE_TO: &str = "move_to";
pub const TOOL_WALK: &str = "walk";
pub const TOOL_OPEN_DOOR: &str = "open_door";
pub const TOOL_FOLLOW: &str = "follow";
pub const TOOL_STOP: &str = "stop";
pub const TOOL_LOGOUT: &str = "logout";
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
pub const TOOL_WAIT_JOURNAL: &str = "wait_journal";
pub const TOOL_NEXT_EVENT: &str = "next_event";
pub const TOOL_TARGET: &str = "target";
pub const TOOL_OPEN_CONTAINER: &str = "open_container";
pub const TOOL_LOOT: &str = "loot";
pub const TOOL_DEPOSIT: &str = "deposit";
pub const TOOL_TRADE_OFFER: &str = "trade_offer";
pub const TOOL_TRADE_ACCEPT: &str = "trade_accept";
pub const TOOL_TRADE_CANCEL: &str = "trade_cancel";
pub const TOOL_VENDOR_SELL: &str = "vendor_sell";
pub const TOOL_VENDOR_BUY: &str = "vendor_buy";
pub const TOOL_CONTEXT_MENU: &str = "context_menu";
pub const TOOL_GUMP_RESPOND: &str = "gump_respond";
pub const TOOL_GUMP_CLOSE: &str = "gump_close";
pub const TOOL_SET_GOAL: &str = "set_goal";
pub const TOOL_CANCEL_GOAL: &str = "cancel_goal";
pub const TOOL_SET_PERSONA: &str = "set_persona";
pub const TOOL_RUN_SCRIPT: &str = "run_script";
pub const TOOL_STOP_SCRIPT: &str = "stop_script";
pub const TOOL_SCRIPT_STATUS: &str = "script_status";
pub const TOOL_LIST_SCRIPTS: &str = "list_scripts";
pub const TOOL_SCRIPT_READ: &str = "script_read";
pub const TOOL_SCRIPT_SAVE: &str = "script_save";
pub const TOOL_MAP_PIN: &str = "map_pin";
pub const TOOL_MAP_CLOSE: &str = "map_close";
pub const TOOL_PROFILE: &str = "profile";
pub const TOOL_HOUSE_EDIT: &str = "house_edit";
pub const TOOL_HELP: &str = "help";
pub const TOOL_CHAT: &str = "chat";
pub const TOOL_BOOK_WRITE: &str = "book_write";
pub const TOOL_AGENTS: &str = "agents";
pub const TOOL_AGENT_SET: &str = "agent_set";
pub const TOOL_AGENT_ON: &str = "agent_on";
pub const TOOL_AGENT_RUN: &str = "agent_run";
pub const TOOL_AGENT_STOP: &str = "agent_stop";
pub const TOOL_DAMAGE_METER: &str = "damage_meter";
pub const TOOL_TARGET_FILTER: &str = "target_filter";
pub const TOOL_HOTKEYS: &str = "hotkeys";
pub const TOOL_HOTKEY: &str = "hotkey";
pub const TOOL_RECORD_MACRO: &str = "record_macro";
pub const TOOL_JOBS: &str = "jobs";
pub const TOOL_JOB_START: &str = "job_start";
pub const TOOL_JOB_STOP: &str = "job_stop";
/// The watch window puts this in the arguments of each call a human makes.
pub const ARG_HUMAN: &str = "human";
pub const TOOL_WATCH: &str = "watch";
pub const TOOL_PROPERTIES: &str = "properties";
pub const TOOL_CLOSE_MENU: &str = "close_menu";
pub const TOOL_SHOP_CHECKOUT: &str = "shop_checkout";
pub const TOOL_SHOP_CLOSE: &str = "shop_close";
pub const TOOL_DYE: &str = "dye";
pub const TOOL_MENU_PICK: &str = "menu_pick";
pub const TOOL_BOOK_CLOSE: &str = "book_close";
pub const TOOL_BOARD_READ: &str = "board_read";
pub const TOOL_BOARD_POST: &str = "board_post";
pub const TOOL_BOARD_REMOVE: &str = "board_remove";
pub const TOOL_BOARD_CLOSE: &str = "board_close";
pub const TOOL_TRADE_GOLD: &str = "trade_gold";
pub const TOOL_COMMAND: &str = "command";
pub const TOOL_TAKE_CONTROL: &str = "take_control";
pub const TOOL_RELEASE_CONTROL: &str = "release_control";
pub const TOOL_GAME_VIEW: &str = "game_view";
pub const TOOL_OPEN_SPELLBOOK: &str = "open_spellbook";
pub const TOOL_TIP: &str = "tip";
pub const TOOL_QUEST_ARROW: &str = "quest_arrow";
pub const TOOL_BOAT_MOVE: &str = "boat_move";
pub const TOOL_TRACK_MEMBERS: &str = "track_members";
pub const TOOL_HOUSE_CONTENT: &str = "house_content";
pub const TOOL_RACE_CHANGE: &str = "race_change";
pub const TOOL_BOOK_READ: &str = "book_read";
pub const TOOL_FIND_TILES: &str = "find_tiles";
pub const TOOL_MULTI_PARTS: &str = "multi_parts";
pub const TOOL_FIND_ENTRANCES: &str = "find_entrances";
pub const TOOL_VIRTUE: &str = "virtue";
pub const TOOL_VIRTUE_GUMP: &str = "virtue_gump";
pub const TOOL_SKILL_LOCK: &str = "skill_lock";
pub const TOOL_STAT_LOCK: &str = "stat_lock";
pub const TOOL_RENAME: &str = "rename";
pub const TOOL_SET_ABILITY: &str = "set_ability";
pub const TOOL_EMOTE_ACTION: &str = "emote_action";
pub const TOOL_FLY: &str = "fly";
pub const TOOL_MENU_BUTTON: &str = "menu_button";
pub const TOOL_TARGET_RESOURCE: &str = "target_resource";
pub const TOOL_USE_TYPE: &str = "use_type";
pub const TOOL_USE_ON: &str = "use_on";
pub const TOOL_CATCH_BAG: &str = "catch_bag";
pub const TOOL_MOUNT: &str = "mount";
pub const TOOL_DISMOUNT: &str = "dismount";
pub const TOOL_ATTACK_NEAREST: &str = "attack_nearest";
pub const TOOL_IGNORE_LIST: &str = "ignore_list";
pub const TOOL_SKILL_GAINS: &str = "skill_gains";
pub const TOOL_LANDMARKS_INFO: &str = "landmarks_info";

pub const NO_BANK_KNOWN: &str =
    "no bank is known near here; find a banker in observe and walk to it";

pub const NO_MARKERS_LOADED: &str =
    "no marker file is loaded; set 'markers' in the config to a UO Auto Map .map or Ultima Mapper Waypoints.lua file";

/// A travel goal before its destination is set: `set_goal` fills it with
/// the spot asked, or the nearest bank.
const DEST_NOT_SET: Point3 = Point3 { x: 0, y: 0, z: 0 };

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
    /// Lines other characters said to this one by name, not answered yet.
    /// Every tool result carries them, so a busy agent still sees them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unanswered: Vec<uoterm_world::SpokenTo>,
    /// With unanswered lines: `basic` or `play_along`, what the agent may
    /// agree to when it answers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_mode: Option<String>,
    /// With unanswered lines: how the persona talks, when it says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_style: Option<String>,
}

impl ToolResult {
    pub fn ok(result: Value) -> Self {
        Self {
            ok: true,
            result,
            error: None,
            action_id: Some(new_action_id()),
            unanswered: Vec::new(),
            chat_mode: None,
            reply_style: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            result: Value::Null,
            error: Some(msg.into()),
            action_id: None,
            unanswered: Vec::new(),
            chat_mode: None,
            reply_style: None,
        }
    }

    pub fn action(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            ok: true,
            result: json!({ "action_id": id }),
            error: None,
            action_id: Some(id),
            unanswered: Vec::new(),
            chat_mode: None,
            reply_style: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Goal {
    Idle,
    Travel { dest: Point3 },
    Hunt,
    Gather { resource: Harvest },
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
            name if Harvest::from_goal(name).is_some() => Self::Gather {
                resource: Harvest::from_goal(name).unwrap_or(Harvest::Lumber),
            },
            "bank" => Self::Bank,
            "shop" => Self::Shop,
            "social" => Self::Social,
            "flee" => Self::Flee,
            "ress" | "res" => Self::Ress,
            "travel" => Self::Travel { dest: DEST_NOT_SET },
            _ => Self::Idle,
        }
    }

    pub fn for_class(class: &str) -> Self {
        match class {
            "lumberjack" | "gatherer" => Self::Gather {
                resource: Harvest::Lumber,
            },
            "miner" => Self::Gather {
                resource: Harvest::Ore,
            },
            "warrior" | "pk" => Self::Hunt,
            "traveler" => Self::Travel { dest: DEST_NOT_SET },
            "sitter" | "banker" | "banker_idle" => Self::Social,
            _ => Self::Idle,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Travel { .. } => "travel",
            Self::Hunt => "hunt",
            Self::Gather { resource } => resource.goal_name(),
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
        "Compact world snapshot, ASCII radar, recent journal, and the assistant features the shard forbids. It also holds the spells of each spellbook, the armed weapon move and the stances on (abilities), party and guild members out of sight (tracked_members), the character's own pets and summons by serial (followers), the round trip to the shard (latency_ms) and the bytes from it and to it in the last half second (traffic). Optional size (odd tiles, 5-41, default 21) sets the radar width.",
        "session exists",
    ),
    (
        TOOL_LOOK_AROUND,
        "Describe the surroundings in words: the surface underfoot, named furniture and walls from the map files, loose items, and people. Takes an optional radius in tiles.",
        "in world",
    ),
    (TOOL_FIND_MOBILES, "Filter nearby mobiles. Args: name (part of the name or of the title, so \"banker\" finds a banker), graphic, distance, notoriety (innocent, friend, gray, criminal, enemy, murderer, invulnerable or any), species (read from the body, so a named orc is an orc), in_sight (only those a shot or a spell can reach), z_min and z_max. Each mobile has its location, dist, title, species, notoriety, hits_percent when known, war, hidden, poisoned, follower (one of yours), dead, in_sight, npc (a guess: no human body, or nobody can harm it), multi (the house or boat it stands on) and what it wears.", "in world"),
    (
        TOOL_ROUTE,
        "Plans the walk move_to would take, with the same choices (x, y or name, accuracy, avoid, open_doors, roads), and says whether the character can get there, where the route ends, how many steps it takes, the tiles on the way, and the search (nodes opened, ms, flat when the heights had to be ignored), without taking a step.",
        "in world",
    ),
    (
        TOOL_PARTY,
        "Party acts: action invite (serial), accept or decline the invite that waits, leave, kick (serial), or loot (on: whether the party may loot what the character kills).",
        "in world",
    ),
    (
        TOOL_MOBILE_STATUS,
        "Asks the shard for a mobile's status, as a click on its health bar does: its hits come back into find_mobiles and observe, and every stat for the character or a pet he owns. close=true instead tells the shard the status bar of that mobile is shut.",
        "in world",
    ),
    (
        TOOL_PROMPT_ANSWER,
        "Answers the words the shard waits for, after a prompt_opened event: its prompt (such as the name for a rune), or else its one-field dialog. text is the answer.",
        "a prompt or text dialog is open",
    ),
    (
        TOOL_PROMPT_CANCEL,
        "Cancels the prompt or the one-field dialog the shard waits on.",
        "a prompt or text dialog is open",
    ),
    (
        TOOL_LINE_OF_SIGHT,
        "Says whether one point sees another, as the shard judges a shot or a spell: to serial, or x, y and z; from the character's eyes, or from (a serial) or from_x, from_y and from_z. mode picks the rules (runuo, modernuo, servuo, pol or sphere; default the sight_mode option). trace=true gives every point of the line with what stops it; without it, blocked_at names the first stop.",
        "in world",
    ),
    (
        TOOL_FIND_TILES,
        "Finds map tiles in an area of any map, nearest first, a page at a time: group (water, trees, ore, forge, anvil, loom, oven, mill), graphics (land ids or static graphics), flags (tiledata flag names, all needed: wet, impassable, surface, wall, door and the rest), name (a word of the tiledata name). The area is x, y and radius (default the character, 12, most 64) or the rectangle x1, y1, x2, y2 (at most 129 on a side). layer land, statics or both; z_min and z_max; map; page and page_size (default 50, most 200). Each tile has x, y, z, layer, graphic, name, flags and dist.",
        "map files loaded, or the mock grid for the map underfoot",
    ),
    (
        TOOL_MULTI_PARTS,
        "The parts of the houses and boats in view: serial for every part of one, or x and y for the parts of any on that tile. A house a player designed gives the parts the shard sent. Each part has multi, graphic, name, x, y, z, height, flags and designed; a page at a time (page, page_size).",
        "a house or boat in view",
    ),
    (
        TOOL_FIND_ENTRANCES,
        "Scans the map round a spot for ways into a dungeon: runs of stair and ladder statics (each run once, with its tile count), the teleporter pads the character learned, and dungeon or cave landmarks. x, y, radius (default the character and 32), map. Nearest first, a page at a time.",
        "map files loaded",
    ),
    (
        TOOL_FIND_ITEMS,
        "Filter items on the ground and inside containers. Args: graphic, graphics (a list; any of them), hue, name (part of the display name), container (a container serial, to search only that one), distance (tiles), x and y (only the items on that tile), z_min and z_max. Each item has its map location, dist, hue, movable (a player may lift it), is_container, and multi (the house or boat it is, or stands on).",
        "in world",
    ),
    (
        TOOL_FIND_LANDMARKS,
        "Named places from the marker file: gates, banks, towns. Args: name (part of the place name, e.g. \"new haven moongate\"), kind (the marker word, such as bank or moongate; for a file with no words, part of the name), closest (a kind: only the nearest one), map (a map index; the one underfoot by default), distance (tiles). Each place has its map, location, dist (only on the current map) and kind. When the map underfoot has none and no map was named, the answer is {places, note, elsewhere} with the matches on other maps. Walk to the place, then find_items to lock the live thing that stands there.",
        "in world",
    ),
    (
        TOOL_LANDMARKS_INFO,
        "What marker data the session read: loaded, count, and how many places on each map and of each kind.",
        "session exists",
    ),
    (
        TOOL_JOURNAL_SEARCH,
        "Search journal text (q). Give since=N for only lines after number N, with numbers.",
        "session exists",
    ),
    (
        TOOL_MAP_TILE,
        "Everything on the tile x, y of any map (map): the land with its id, name, z and flags, each static with graphic, name, z, height, hue and flags, and on the map underfoot the items and the house or boat parts on it; with walkable, door and the standing z (from z, default the character's height).",
        "map loaded or mock grid",
    ),
    (
        TOOL_CAN_WALK,
        "True if the tile has a walkable surface. With z, only at that height; with none, at any height (ground, floor, porch, roof).",
        "map loaded or mock grid",
    ),
    (
        TOOL_SAY,
        "Speak. channel: say (default), yell, party, guild or alliance; hue (a colour; default the speech_hue option). Persona blocks *emotes* and rate-limits.",
        "in world",
    ),
    (TOOL_WHISPER, "Whisper.", "in world"),
    (
        TOOL_REPLY,
        "Answer a line said to the character by name, in the channel it came in: say, whisper, party, private party, guild or alliance. `to` (a name or serial) picks the speaker; without it, the newest line.",
        "a line waits in unanswered",
    ),
    (
        TOOL_EMOTE,
        "Emote only if persona.allow_emote is true.",
        "persona allows emotes",
    ),
    (
        TOOL_MOVE_TO,
        "Walk to the tile x, y, or to the nearest landmark called name: plans a route and steps it at a person's pace. run (true runs, false walks; default by stamina and danger), accuracy (stop within that many tiles, up to 18; the goal may be a tree or an anvil), open_doors (default true; the shard's rules still apply), avoid (a list of {x, y, radius} areas and {serial, radius} creatures to keep away from), roads (default true: grass and forest cost a little more), exact (fail instead of walking as near as a route goes). A goal no route reaches is walked toward as far as it goes, and the answer says partial. The answer names heading_to and the route search (nodes, ms, steps, flat).",
        "in world",
    ),
    (
        TOOL_WALK,
        "Send 0x02 steps in a direction (Classic Client hold-right-click analog). Args: direction, running, hold_ms, slide (a held walk that meets a wall goes on beside it).",
        "in world",
    ),
    (
        TOOL_OPEN_DOOR,
        "Open the door you face (packet 0x12/0x58). Stand next to the door first.",
        "in world, adjacent to a door",
    ),
    (TOOL_FOLLOW, "Follow a mobile serial.", "mobile in range"),
    (TOOL_STOP, "Clear path and set idle.", "session exists"),
    (TOOL_LOGOUT, "Ask the shard to log the character out. A shard may hold the request until she is somewhere it allows it, such as an inn or a house. then_play (a character name of the account) logs in as that character in the same session once the shard lets this one go; without it the session ends, and characters and connect go on from the character list.", "in world"),
    (TOOL_USE, "Double-click serial.", "serial exists"),
    (TOOL_SINGLE_CLICK, "Single-click for name.", "serial exists"),
    (TOOL_ATTACK, "Attack mobile.", "mobile serial"),
    (TOOL_WAR_MODE, "Set war/peace.", "in world"),
    (TOOL_LIFT, "Pick up item.", "item serial"),
    (TOOL_DROP, "Drop the lifted item: dest for a container or a mobile, none for the ground. x, y (and z on the ground) give an exact place.", "serial"),
    (TOOL_EQUIP, "Lift and wear an item: serial, or who=last for the last weapon put away. With no weapon put away here, who=last asks the shard to wear the last weapon it remembers.", "item serial"),
    (TOOL_UNEQUIP, "Lift a worn item into the backpack; a weapon is remembered.", "layer occupied"),
    (
        TOOL_CAST,
        "Cast a spell, named by its number or its name (\"greater heal\"). target (a serial, or self) answers the spell's cursor as it comes. book (a spellbook serial) casts from that book, as a click in an open book does.",
        "enough mana",
    ),
    (
        TOOL_USE_SKILL,
        "Use a skill from its button, named by its number or its name (\"hiding\").",
        "in world",
    ),
    (
        TOOL_NEXT_EVENT,
        "Wait for the next important event: named in chat, hurt or low health, an enemy near, a target cursor, gump, prompt or trade, an item in the pack, a party invite, death, a skill or stat change, a quest arrow, a map opened, a weapon move or stance on or off. Returns the events and the state (health, enemies near, unanswered lines, pack). Call it in a loop; events wait for you. timeout_ms (default 5000, max 7000). ambient lists the busy kinds to get too: sound, effect, animation, item_deleted, member_positions, or all.",
        "session exists",
    ),
    (
        TOOL_WAIT_JOURNAL,
        "Wait for a new journal line holding q; timeout_ms (default 5000, max 7000).",
        "session exists",
    ),
    (
        TOOL_WAIT_TARGET,
        "Wait for a target cursor; timeout_ms (default 5000, max 7000).",
        "none",
    ),
    (
        TOOL_TARGET,
        "Answer the target cursor: serial, who=self|last, or ground x,y,z,graphic. No args cancels.",
        "must have a target cursor",
    ),
    (TOOL_OPEN_CONTAINER, "Open container serial.", "item serial"),
    (
        TOOL_LOOT,
        "Walk to a corpse, open it, lift each stack and drop it in the pack.",
        "corpse serial",
    ),
    (
        TOOL_DEPOSIT,
        "Open the bank box and move the pack into it. Give a graphic to bank only that kind.",
        "at a banker",
    ),
    (TOOL_TRADE_OFFER, "Secure trade offer.", "mobile serial"),
    (
        TOOL_TRADE_ACCEPT,
        "Tick the accept box of an open trade (accept: true, the default), or untick it (accept: false). Several trades may be open at once: trade (the other player, or a box of the trade) picks one; the newest by default. Read observe.trades first: what they offer, and who has accepted.",
        "a trade window open",
    ),
    (TOOL_TRADE_CANCEL, "Close an open trade: trade (the other player, or a box of the trade); the newest by default.", "a trade window open"),
    (
        TOOL_VENDOR_SELL,
        "Ask a named nearby NPC vendor for its sell list and sell every listed backpack item matching the required graphic.",
        "vendor name and item graphic",
    ),
    (
        TOOL_VENDOR_BUY,
        "Buy an amount of one item from an open NPC vendor basket.",
        "vendor serial, shop item serial, and amount",
    ),
    (
        TOOL_CONTEXT_MENU,
        "Ask for an object's context menu. With cliloc, the enabled entry of that text number is picked as the menu comes; with none, the entries come back to pick by index.",
        "in world",
    ),
    (
        TOOL_GUMP_RESPOND,
        "Click a gump button. Args: button (a button id), switches (the choices to tick), texts ([{id, text}] for the text fields). button 0 closes. observe gumps shows each button id and choice switch with its words.",
        "open gump",
    ),
    (TOOL_GUMP_CLOSE, "Close gump.", "open gump"),
    (
        TOOL_SET_GOAL,
        "High-level goal: idle, travel, hunt, gather, bank, shop, social, flee, ress. hunt starts the hunt job.",
        "in world",
    ),
    (
        TOOL_CANCEL_GOAL,
        "Set goal idle and stop movement.",
        "session exists",
    ),
    (
        TOOL_RUN_SCRIPT,
        "Run a script: name (a file in the scripts folder) or text (the script itself), in a slot of its own (slot; default the script name, or text). Up to 8 run side by side, a healer beside a task, and share the character's pace fairly. loop true runs it again each time it ends; for (seconds) and iterations (runs from the top; more than 1 loops) end it by themselves.",
        "no script in that slot",
    ),
    (TOOL_STOP_SCRIPT, "Stop the script of one slot (slot), or every script.", "a script is running"),
    (
        TOOL_SCRIPT_STATUS,
        "A script by its slot (slot): status (running, suspended, done, stopped, failed), line, iterations, error, ended_by (time up, iterations done) and its output lines. With no slot: the first running or last ended, with slots (every running one), ended (the last 16 that ended) and shown (lines of hotkeys and commands).",
        "session exists",
    ),
    (TOOL_LIST_SCRIPTS, "The scripts in the scripts folder.", "session exists"),
    (
        TOOL_SCRIPT_READ,
        "The text of one script of the scripts folder: name.",
        "the script exists",
    ),
    (
        TOOL_SCRIPT_SAVE,
        "Saves a script in the scripts folder: name (letters, digits, space, - and _) and text. A script that does not parse is refused with the line of the fault. A script with that name is replaced.",
        "session exists",
    ),
    (
        TOOL_AGENTS,
        "Every agent's settings, which agents are on, and the job running.",
        "session exists",
    ),
    (
        TOOL_AGENT_SET,
        "Replace an agent's settings (agent, settings), or one named list (agent, list, settings). Agents: autoloot, scavenger, organizer, restock, dress, buy, sell, bandage, self_heal, friends, remount, bone_cutter, carver, open_corpses, targets. Saved per character.",
        "session exists",
    ),
    (
        TOOL_AGENT_ON,
        "Switch an agent on or off (agent, on). A list name picks the list autoloot, scavenger, buy or sell uses.",
        "session exists",
    ),
    (
        TOOL_AGENT_RUN,
        "Run a job once: organizer or restock (with list), dress or undress (list optional), or autoloot on the corpses in range.",
        "in world",
    ),
    (TOOL_AGENT_STOP, "Stop the running agent job.", "session exists"),
    (
        TOOL_DAMAGE_METER,
        "Damage dealt per mobile: action start, pause, resume, stop, or report.",
        "session exists",
    ),
    (
        TOOL_TARGET_FILTER,
        "Pick a mobile with a named target filter and make it the last target.",
        "in world",
    ),
    (
        TOOL_HOTKEYS,
        "The hotkeys by group: general, actions, pets, agents, combat, potions, items, wands, skills, spells, virtues, targets, scripts. Give group for one group. Give name for one hotkey with the script lines it runs.",
        "session exists",
    ),
    (
        TOOL_HOTKEY,
        "Press a hotkey by name, such as 'Bandage Self', 'Cast Greater Heal', 'Potion Cure' or 'Attack Nearest Enemy'.",
        "in world",
    ),
    (
        TOOL_RECORD_MACRO,
        "Record a macro: action start (with name), stop (saves it as a script), or cancel. While it records, tool calls and hotkeys become script lines, with waits for cursors, gumps and prompts.",
        "session exists",
    ),
    (
        TOOL_JOBS,
        "The session job running now, if any: name, phase, include and avoid lists.",
        "session exists",
    ),
    (
        TOOL_JOB_START,
        "Start a session job. Hunt: job=hunt, optional include and avoid term lists (species:, name:, graphic:, any:). Walk: job=walk, x and y or name of a landmark, watch=true to stand guard after arrival. replace=true takes over a job already running. Hands back with job_ended.",
        "in world",
    ),
    (
        TOOL_JOB_STOP,
        "Stop the running session job. Sends job_ended with reason stopped.",
        "a job is running",
    ),
    (
        TOOL_WATCH,
        "For the watch window. The whole screen in one picture: observe with each list at its full length, and the things only a screen draws: item places in containers, journal lines with speaker and hue, skills with names, party vitals, hit and animation cues, the target cursor, the shown context menu, the open shop list. An agent reads observe, which is short on purpose.",
        "session exists",
    ),
    (
        TOOL_PROPERTIES,
        "The words of the tooltip of one object: serial. lines are the words; entries are the same lines as the shard sent them, each a cliloc text number with its arguments (on shards with property lists). When the session has none yet, it asks the shard, and the next call has them.",
        "in world",
    ),
    (
        TOOL_CLOSE_MENU,
        "Closes the context menu that context_menu showed, with no pick.",
        "session exists",
    ),
    (
        TOOL_SHOP_CHECKOUT,
        "Buys or sells a cart from the shop list that is open: items is [{serial, amount}]. watch shows the list under shop.",
        "a shop list is open",
    ),
    (
        TOOL_SHOP_CLOSE,
        "Forgets the open shop list with no trade.",
        "session exists",
    ),
    (
        TOOL_DYE,
        "Answers the dye tub that asks for a colour (watch shows it under dye) with a hue.",
        "a dye tub asks for a colour",
    ),
    (
        TOOL_MENU_PICK,
        "Answers the old-style menu that watch shows under menu: index picks that entry, from 1; no index walks away with no pick.",
        "a menu is open",
    ),
    (
        TOOL_BOOK_CLOSE,
        "Forgets the open book that watch shows under book.",
        "session exists",
    ),
    (
        TOOL_BOARD_READ,
        "Asks for the lines of one message of the open bulletin board: message is its serial. Use a bulletin board to open it; watch then shows it under board, with the list of messages and the message that was read.",
        "a bulletin board is open",
    ),
    (
        TOOL_BOARD_POST,
        "Posts a message on the open bulletin board: subject, text (lines parted by a line break), and reply_to for an answer to a message.",
        "a bulletin board is open",
    ),
    (
        TOOL_BOARD_REMOVE,
        "Removes a message the character posted from the open bulletin board: message.",
        "a bulletin board is open",
    ),
    (
        TOOL_BOARD_CLOSE,
        "Forgets the open bulletin board.",
        "session exists",
    ),
    (
        TOOL_MAP_PIN,
        "Works on the map item that is open (watch shows it under maps): x and y in pixels of its picture put a pin there; action move with pin (its place in the list, from 0) and x, y moves one pin; action remove with pin takes one pin off; action clear takes every pin off; action edit asks the shard to let the map be drawn on.",
        "a map is open",
    ),
    (
        TOOL_MAP_CLOSE,
        "Forgets the open map item: serial, or none for the one that opened last.",
        "session exists",
    ),
    (
        TOOL_PROFILE,
        "The profile a player wrote about his character: serial. With text, writes the profile of your own character. The words come back in watch under profiles.",
        "in world",
    ),
    (
        TOOL_HOUSE_EDIT,
        "One step of the house designer, while it is open (watch shows it under designing): action add, remove, stair, roof, remove_roof with graphic, x and y (and z to remove); floor with level from 1; and clear, revert, commit, exit, backup, restore, sync (the shard sends the design again). The parts to build with are in watch under house_parts.",
        "the house designer is open",
    ),
    (
        TOOL_HELP,
        "Asks the shard for its help menu. It answers with a gump.",
        "in world",
    ),
    (
        TOOL_CHAT,
        "The chat of the shard: action open (with name) turns it on, join (with channel and password), create (a new channel, with channel and password), say (with text), leave. watch shows the channels and the lines under chat.",
        "in world",
    ),
    (
        TOOL_BOOK_WRITE,
        "Writes in the open book (watch shows it under book): title and author name it, page and text write one page, its lines parted by a line break. A book the shard sealed cannot be written in.",
        "a book is open",
    ),
    (
        TOOL_TRADE_GOLD,
        "Sets the gold and platinum offered in an open trade: gold, platinum, and trade (the other player, or a box of the trade; the newest by default).",
        "a trade is open",
    ),
    (
        TOOL_OPEN_SPELLBOOK,
        "Opens the character's spellbook of one school: kind magery, necromancy, chivalry, bushido, ninjitsu, spellweaving or mysticism. Its spells come back in observe spellbooks.",
        "in world, the book in the pack",
    ),
    (
        TOOL_TIP,
        "Asks the shard for the next tip of the day, or the one before (next=false). The words come back in watch shard_notice.",
        "a tip was shown",
    ),
    (
        TOOL_QUEST_ARROW,
        "Clicks the quest arrow the shard shows (observe quest_arrow), with the left button, or the right one with right=true.",
        "a quest arrow is shown",
    ),
    (
        TOOL_BOAT_MOVE,
        "Steers the boat the character pilots: direction n, ne, e, se, s, sw, w or nw, and speed stop, slow or fast (default fast).",
        "piloting a boat",
    ),
    (
        TOOL_TRACK_MEMBERS,
        "Asks the shard where the party or the guild members out of sight stand: who party or guild. The places come back in observe tracked_members and a member_positions event.",
        "in a party or a guild",
    ),
    (
        TOOL_HOUSE_CONTENT,
        "Tells the shard whether to show what stands inside public houses: show true or false.",
        "in world",
    ),
    (
        TOOL_RACE_CHANGE,
        "Answers the race change the shard asks for (a race_change_opened event; observe race_change lists the looks): hair and beard are item graphics from hair_styles and beard_styles (0 for none), skin_hue comes from skin_hues, hair_hue and beard_hue from hair_hues. Each one left out takes the first choice. cancel=true says no.",
        "a race change is open",
    ),
    (
        TOOL_BOOK_READ,
        "One page of the open book (watch shows it under book): its lines when the shard sent them, or else the shard is asked for that page and the next call has it.",
        "a book is open",
    ),
    (
        TOOL_VIRTUE,
        "Invokes a virtue: name humility, sacrifice, compassion, spirituality, valor, honor, justice or honesty. Honor, sacrifice and valor go as the virtue macro; the rest as a press in the virtue gump. The shard says when one is not active.",
        "in world",
    ),
    (
        TOOL_VIRTUE_GUMP,
        "Asks the shard for the virtue gump of a mobile: serial, or the character.",
        "in world",
    ),
    (
        TOOL_SKILL_LOCK,
        "Sets the lock of a skill: skill (number or name), lock up, down or locked.",
        "in world",
    ),
    (
        TOOL_STAT_LOCK,
        "Sets the lock of a stat: stat str, dex or int, lock up, down or locked.",
        "in world",
    ),
    (TOOL_RENAME, "Gives a pet of the character a new name: serial and name.", "a pet in view"),
    (
        TOOL_SET_ABILITY,
        "Arms a weapon move: ability primary, secondary, stun or disarm; on false clears it. observe abilities shows what is armed.",
        "in world, a weapon for primary and secondary",
    ),
    (
        TOOL_EMOTE_ACTION,
        "Plays a body action, as the emote gestures do: action such as bow or salute.",
        "in world",
    ),
    (TOOL_FLY, "A gargoyle takes off; on false lands.", "a gargoyle character"),
    (
        TOOL_MENU_BUTTON,
        "Presses a button of the paperdoll: which quests or guild. The shard answers with a gump.",
        "in world",
    ),
    (
        TOOL_TARGET_RESOURCE,
        "Aims a harvest tool at a resource with no cursor: tool (serial), resource ore, sand, wood, graves or red mushrooms.",
        "a harvest tool in the pack",
    ),
    (
        TOOL_USE_TYPE,
        "Double-clicks the first item of a graphic: graphic, hue (default any), source backpack, ground, world or a container serial (default backpack), range for the ground.",
        "in world",
    ),
    (
        TOOL_USE_ON,
        "Uses an item on a mobile with no cursor, as a bandage is: item and target (serials). Takes an action's time.",
        "in world",
    ),
    (
        TOOL_CATCH_BAG,
        "The container loot goes into in place of the backpack: serial sets it, clear=true clears it; with neither it says which. Saved per character.",
        "in world",
    ),
    (
        TOOL_MOUNT,
        "Rides a mount: serial, or the remount agent's mount, or the nearest pet of the character's that can be ridden. War mode goes off first, so the double-click is no attack.",
        "not riding",
    ),
    (TOOL_DISMOUNT, "Gets off the mount, with war mode off first.", "riding"),
    (
        TOOL_ATTACK_NEAREST,
        "Attacks the nearest mobile in sight that may be harmed without a crime: never an innocent, a friend, a pet or a party member, or one nobody can harm. notoriety (gray, criminal, enemy, murderer), species, name and distance narrow it. Obeys the shard's rule on closest targets.",
        "in world",
    ),
    (
        TOOL_IGNORE_LIST,
        "The gumps and the journal lines an agent does not hear of: list gumps or journal, action add, remove, clear or show (default), value a gump id or words (a speaker or words of a line). Saved per character.",
        "session exists",
    ),
    (
        TOOL_SKILL_GAINS,
        "The skills gained this session: for each, the points gained, the changes and the rate an hour, and the newest changes. skill narrows it to one; clear=true starts the record again.",
        "session exists",
    ),
    (
        TOOL_SET_PERSONA,
        "Sets who the character is: name, class and tier, the hours he plays, how often and how he talks, and how he plays along with players. See docs/PERSONAS.md.",
        "session exists",
    ),
    (
        TOOL_COMMAND,
        "For the watch window, not for an agent. Runs one script command at once as one act of a human: text is the command line, as in docs/SCRIPTS.md. Needs human=true. A command that waits is not waited for.",
        "session exists",
    ),
    (
        TOOL_TAKE_CONTROL,
        "For the watch window, not for an agent. A human takes the character: the goal, the job, the script and the macro agent stop, and each acting call without human=true is refused until release_control or 90 s with no human act. Sends control_taken.",
        "session exists",
    ),
    (
        TOOL_RELEASE_CONTROL,
        "For the watch window, not for an agent. Gives the character back to the agent. Sends control_released.",
        "session exists",
    ),
    (
        TOOL_GAME_VIEW,
        "For the watch window, not for an agent. The size of the game view the window draws: width and height in pixels. The shard hears it at each login, and at once when it changes in the world.",
        "session exists",
    ),
];

/// The tools that only look. An agent may call them while a human has the
/// character.
const READ_ONLY_TOOLS: [&str; 28] = [
    TOOL_OBSERVE,
    TOOL_LANDMARKS_INFO,
    TOOL_SKILL_GAINS,
    TOOL_FIND_TILES,
    TOOL_MULTI_PARTS,
    TOOL_FIND_ENTRANCES,
    TOOL_WATCH,
    TOOL_PROPERTIES,
    TOOL_LOOK_AROUND,
    TOOL_FIND_MOBILES,
    TOOL_FIND_ITEMS,
    TOOL_FIND_LANDMARKS,
    TOOL_JOURNAL_SEARCH,
    TOOL_MAP_TILE,
    TOOL_CAN_WALK,
    TOOL_LINE_OF_SIGHT,
    TOOL_ROUTE,
    TOOL_WAIT_JOURNAL,
    TOOL_NEXT_EVENT,
    TOOL_SCRIPT_STATUS,
    TOOL_LIST_SCRIPTS,
    TOOL_SCRIPT_READ,
    TOOL_MAP_CLOSE,
    TOOL_AGENTS,
    TOOL_DAMAGE_METER,
    TOOL_HOTKEYS,
    TOOL_JOBS,
    TOOL_SINGLE_CLICK,
];

pub fn is_read_only(tool: &str) -> bool {
    READ_ONLY_TOOLS.contains(&tool)
}

/// The tools only a human's window calls. An agent is refused each one, so
/// the list an agent reads does not offer them.
const HUMAN_ONLY_TOOLS: [&str; 4] = [
    TOOL_COMMAND,
    TOOL_TAKE_CONTROL,
    TOOL_RELEASE_CONTROL,
    TOOL_GAME_VIEW,
];

/// The argument that names the session a call is for. The MCP server reads
/// it; with none, it takes a session of its own choosing.
const ARG_SESSION_ID: &str = "session_id";
const ABOUT_SESSION_ID: &str = "session to act in; default the first session";

/// The name of every tool of a session, the window's own tools among them.
pub fn tool_names() -> Vec<&'static str> {
    TOOLS.iter().map(|(name, _, _)| *name).collect()
}

/// The tools an agent may call, each with the arguments it reads, which of
/// them it cannot do without, and when it needs one of several.
pub fn mcp_tool_list() -> Value {
    let session_tools = TOOLS
        .iter()
        .filter(|(name, _, _)| !HUMAN_ONLY_TOOLS.contains(name))
        .map(|tool| tool_entry(tool, true));
    // The runtime's own tools come before any session, so they name none.
    let runtime_tools = crate::characters::RUNTIME_TOOLS
        .iter()
        .map(|tool| tool_entry(tool, false));
    let tools: Vec<Value> = runtime_tools.chain(session_tools).collect();
    json!({ "tools": tools })
}

/// One tool as the list gives it: its words, the arguments it reads, which
/// it cannot do without, and, for a tool of a session, the session to act
/// in.
fn tool_entry((name, desc, pre): &(&str, &str, &str), in_a_session: bool) -> Value {
    let reads = tool_args(name);
    let mut properties = serde_json::Map::new();
    if in_a_session {
        properties.insert(
            ARG_SESSION_ID.into(),
            json!({"type": "string", "description": ABOUT_SESSION_ID}),
        );
    }
    for a in reads {
        properties.insert(a.key.into(), arg_schema(a));
    }
    let required: Vec<&str> = reads.iter().filter(|a| a.required).map(|a| a.key).collect();
    let one_of = ONE_OF
        .iter()
        .find(|(tool, _)| tool == name)
        .map(|(_, keys)| format!(" Needs one of: {}.", keys.join(", ")))
        .unwrap_or_default();
    json!({
        "name": name,
        "description": format!("{desc}{one_of} Precondition: {pre}"),
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}

/// The arguments one tool reads.
fn tool_args(name: &str) -> &'static [ToolArg] {
    TOOL_ARGS
        .iter()
        .find(|(tool, _)| *tool == name)
        .map_or(&[], |(_, reads)| reads)
}

fn arg_schema(a: &ToolArg) -> Value {
    let about = a.about;
    match a.kind {
        ArgKind::Text => json!({"type": "string", "description": about}),
        ArgKind::Integer => json!({"type": "integer", "description": about}),
        ArgKind::Number => json!({"type": "number", "description": about}),
        ArgKind::Boolean => json!({"type": "boolean", "description": about}),
        ArgKind::Serial | ArgKind::TextOrInteger => {
            json!({"type": ["integer", "string"], "description": about})
        }
        ArgKind::Integers => {
            json!({"type": "array", "items": {"type": "integer"}, "description": about})
        }
        ArgKind::Texts => {
            json!({"type": "array", "items": {"type": "string"}, "description": about})
        }
        ArgKind::Objects => {
            json!({"type": "array", "items": {"type": "object"}, "description": about})
        }
        ArgKind::Object => json!({"type": "object", "description": about}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every tool an agent may call says what it reads, and only tools that
    /// exist are described. A tool left out of the table would reach a client
    /// with no arguments at all, and one it could not call would be offered.
    #[test]
    fn each_agent_tool_lists_its_own_arguments() {
        let runtime_tools: Vec<&str> = crate::characters::RUNTIME_TOOLS
            .iter()
            .map(|(name, _, _)| *name)
            .collect();
        let names: Vec<&str> = tool_names()
            .into_iter()
            .chain(runtime_tools.clone())
            .collect();
        for name in names.iter().filter(|name| !HUMAN_ONLY_TOOLS.contains(name)) {
            assert!(
                TOOL_ARGS.iter().any(|(tool, _)| tool == name),
                "{name} has no argument list"
            );
        }
        let described = TOOL_ARGS.iter().map(|(tool, _)| tool);
        let one_of = ONE_OF.iter().map(|(tool, _)| tool);
        for tool in described.chain(one_of) {
            assert!(names.contains(tool), "{tool} is described but is no tool");
        }
        let listed = mcp_tool_list();
        let tools = listed["tools"].as_array().unwrap();
        for human in HUMAN_ONLY_TOOLS {
            assert!(
                !tools.iter().any(|t| t["name"] == human),
                "{human} is offered to agents"
            );
        }
        let move_to = tools.iter().find(|t| t["name"] == TOOL_MOVE_TO).unwrap();
        assert_eq!(move_to["inputSchema"]["required"], json!([]));
        assert!(move_to["inputSchema"]["properties"]["avoid"].is_object());
        assert!(move_to["description"]
            .as_str()
            .unwrap()
            .contains("Needs one of: x, name."));
        let persona = tools.iter().find(|t| t["name"] == TOOL_SET_PERSONA);
        assert!(persona.is_some(), "set_persona is offered");
        let connect = tools
            .iter()
            .find(|t| t["name"] == crate::characters::TOOL_CONNECT)
            .unwrap();
        let reads = &connect["inputSchema"]["properties"];
        assert!(reads["password_env"].is_object());
        assert!(
            reads["password"].is_null(),
            "a password is never an argument"
        );
        assert!(
            reads["session_id"].is_null(),
            "connect comes before a session"
        );
        let use_tool = tools.iter().find(|t| t["name"] == TOOL_USE).unwrap();
        assert!(use_tool["description"]
            .as_str()
            .unwrap()
            .contains("Needs one of: serial, who."));
    }

    #[test]
    fn observe_describes_radar_size() {
        let desc = TOOLS
            .iter()
            .find(|(name, _, _)| *name == TOOL_OBSERVE)
            .map(|(_, desc, _)| *desc)
            .unwrap();
        assert!(desc.contains("size"));
        let listed = mcp_tool_list();
        let observe = listed["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["name"] == TOOL_OBSERVE)
            .unwrap();
        assert!(observe["inputSchema"]["properties"]["size"].is_object());
    }

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
