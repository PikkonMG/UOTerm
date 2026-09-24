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
        "Compact world snapshot, ASCII radar, recent journal, and the assistant features the shard forbids. Optional size (odd tiles, 5-41, default 21) sets the radar width.",
        "session exists",
    ),
    (
        TOOL_LOOK_AROUND,
        "Describe the surroundings in words: the surface underfoot, named furniture and walls from the map files, loose items, and people. Takes an optional radius in tiles.",
        "in world",
    ),
    (TOOL_FIND_MOBILES, "Filter nearby mobiles. Args: name (part of the name or of the title, so \"banker\" finds a banker), graphic, distance, notoriety (innocent, friend, gray, criminal, enemy, murderer, invulnerable or any), species (read from the body, so a named orc is an orc), in_sight (only those a shot or a spell can reach). Each mobile has its location, dist, title, species, notoriety, hits_percent when known, war, hidden, poisoned, dead, in_sight and what it wears.", "in world"),
    (
        TOOL_ROUTE,
        "Plans a walk to the tile x, y and says whether the character can get there, how many steps it takes, and the tiles on the way, without taking a step.",
        "in world",
    ),
    (
        TOOL_PARTY,
        "Party acts: action invite (serial), accept or decline the invite that waits, leave, kick (serial), or loot (on: whether the party may loot what the character kills).",
        "in world",
    ),
    (
        TOOL_MOBILE_STATUS,
        "Asks the shard for a mobile's status, as a click on its health bar does: its hits come back into find_mobiles and observe, and every stat for the character or a pet he owns.",
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
        "Says whether the character sees an object or a spot, as the shard judges a shot or a spell: serial, or x, y and z.",
        "in world",
    ),
    (
        TOOL_FIND_ITEMS,
        "Filter items on the ground and inside containers. Args: graphic, graphics (a list; any of them), hue, name (part of the display name), container (a container serial, to search only that one), distance (tiles), x and y (only the items on that tile). Each item has its map location, dist and hue.",
        "in world",
    ),
    (
        TOOL_FIND_LANDMARKS,
        "Named places from the marker file: gates, banks, towns. Args: name (part of the place name, e.g. \"new haven moongate\"), map (a map index; the one underfoot by default), distance (tiles). Each place has its map, location, dist (only on the current map) and kind. Walk to the place, then find_items to lock the live thing that stands there.",
        "in world",
    ),
    (
        TOOL_JOURNAL_SEARCH,
        "Search journal text (q). Give since=N for only lines after number N, with numbers.",
        "session exists",
    ),
    (
        TOOL_MAP_TILE,
        "Tile kind and Z at x,y.",
        "map loaded or mock grid",
    ),
    (
        TOOL_CAN_WALK,
        "True if the tile has a walkable surface. With z, only at that height; with none, at any height (ground, floor, porch, roof).",
        "map loaded or mock grid",
    ),
    (
        TOOL_SAY,
        "Speak. channel: say (default), party, guild or alliance. Persona blocks *emotes* and rate-limits.",
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
        "Walk to the tile x, y: plans a route and steps it at a person's pace. A goal no route reaches is walked toward as far as it goes, and the answer says partial.",
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
    (TOOL_LOGOUT, "Ask the shard to log the character out. A shard may hold the request until she is somewhere it allows it, such as an inn or a house.", "in world"),
    (TOOL_USE, "Double-click serial.", "serial exists"),
    (TOOL_SINGLE_CLICK, "Single-click for name.", "serial exists"),
    (TOOL_ATTACK, "Attack mobile.", "mobile serial"),
    (TOOL_WAR_MODE, "Set war/peace.", "in world"),
    (TOOL_LIFT, "Pick up item.", "item serial"),
    (TOOL_DROP, "Drop the lifted item: dest for a container or a mobile, none for the ground. x, y (and z on the ground) give an exact place.", "serial"),
    (TOOL_EQUIP, "Lift and wear an item: serial, or who=last for the last weapon put away.", "item serial"),
    (TOOL_UNEQUIP, "Lift a worn item into the backpack; a weapon is remembered.", "layer occupied"),
    (
        TOOL_CAST,
        "Cast a spell, named by its number or its name (\"greater heal\"). target (a serial, or self) answers the spell's cursor as it comes.",
        "enough mana",
    ),
    (
        TOOL_USE_SKILL,
        "Use a skill from its button, named by its number or its name (\"hiding\").",
        "in world",
    ),
    (
        TOOL_NEXT_EVENT,
        "Wait for the next important event: named in chat, hurt or low health, an enemy near, a target cursor, gump, prompt or trade, an item in the pack, a party invite, death. Returns the events and the state (health, enemies near, unanswered lines, pack). Call it in a loop; events wait for you. timeout_ms (default 5000, max 7000).",
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
        "Tick the accept box of the open trade (accept: true, the default), or untick it (accept: false). Read observe.trade first: what they offer, and who has accepted.",
        "a trade window open",
    ),
    (TOOL_TRADE_CANCEL, "Close the open trade.", "a trade window open"),
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
        "Run a script: name (a file in the scripts folder) or text (the script itself); loop true runs it again each time it ends. One script runs at a time.",
        "no script running",
    ),
    (TOOL_STOP_SCRIPT, "Stop the running script.", "a script is running"),
    (
        TOOL_SCRIPT_STATUS,
        "The running or last script: status (running, done, stopped, failed), line, error, and its output lines.",
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
        "Replace an agent's settings (agent, settings), or one named list (agent, list, settings). Agents: autoloot, scavenger, organizer, restock, dress, buy, sell, bandage, friends, remount, bone_cutter, carver, open_corpses, targets. Saved per character.",
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
        "The words of the tooltip of one object: serial. When the session has none yet, it asks the shard, and the next call has them.",
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
        "Works on the map item that is open (watch shows it under maps): x and y in pixels of its picture put a pin there; action clear takes every pin off; action edit asks the shard to let the map be drawn on.",
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
        "One step of the house designer, while it is open (watch shows it under designing): action add, remove, stair, roof, remove_roof with graphic, x and y (and z to remove); floor with level from 1; and clear, revert, commit, exit, backup, restore. The parts to build with are in watch under house_parts.",
        "the house designer is open",
    ),
    (
        TOOL_HELP,
        "Asks the shard for its help menu. It answers with a gump.",
        "in world",
    ),
    (
        TOOL_CHAT,
        "The chat of the shard: action open (with name) turns it on, join (with channel and password), say (with text), leave. watch shows the channels and the lines under chat.",
        "in world",
    ),
    (
        TOOL_BOOK_WRITE,
        "Writes in the open book (watch shows it under book): title and author name it, page and text write one page, its lines parted by a line break. A book the shard sealed cannot be written in.",
        "a book is open",
    ),
    (
        TOOL_TRADE_GOLD,
        "Sets the gold and platinum offered in the open trade: gold, platinum.",
        "a trade is open",
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
];

/// The tools that only look. An agent may call them while a human has the
/// character.
const READ_ONLY_TOOLS: [&str; 23] = [
    TOOL_OBSERVE,
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
const HUMAN_ONLY_TOOLS: [&str; 3] = [TOOL_COMMAND, TOOL_TAKE_CONTROL, TOOL_RELEASE_CONTROL];

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
    let tools: Vec<Value> = TOOLS
        .iter()
        .filter(|(name, _, _)| !HUMAN_ONLY_TOOLS.contains(name))
        .map(|(name, desc, pre)| {
            let reads = tool_args(name);
            let mut properties = serde_json::Map::new();
            properties.insert(
                ARG_SESSION_ID.into(),
                json!({"type": "string", "description": ABOUT_SESSION_ID}),
            );
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
        })
        .collect();
    json!({ "tools": tools })
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
        let names = tool_names();
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
        assert_eq!(move_to["inputSchema"]["required"], json!(["x", "y"]));
        let persona = tools.iter().find(|t| t["name"] == TOOL_SET_PERSONA);
        assert!(persona.is_some(), "set_persona is offered");
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
