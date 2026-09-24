//! The arguments each tool reads, so a client that lists the tools knows
//! what to send to each one and what each one cannot do without.

use super::*;

/// The kind of JSON value an argument is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArgKind {
    Text,
    Integer,
    Number,
    Boolean,
    /// An object's serial: a number, or `0x` text.
    Serial,
    /// A number, or the same number as decimal or `0x` text.
    TextOrInteger,
    Integers,
    Texts,
    Objects,
    Object,
}

/// One argument a tool reads.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ToolArg {
    pub key: &'static str,
    pub kind: ArgKind,
    /// The call fails without it.
    pub required: bool,
    pub about: &'static str,
}

const fn arg(key: &'static str, kind: ArgKind, about: &'static str) -> ToolArg {
    ToolArg {
        key,
        kind,
        required: false,
        about,
    }
}

const fn needs(key: &'static str, kind: ArgKind, about: &'static str) -> ToolArg {
    ToolArg {
        key,
        kind,
        required: true,
        about,
    }
}

use ArgKind::*;

/// The arguments of every tool an agent may call, in the order of [`TOOLS`].
pub(crate) const TOOL_ARGS: &[(&str, &[ToolArg])] = &[
    (
        TOOL_OBSERVE,
        &[
            arg("size", Integer, "radar width in tiles; default 21"),
        ],
    ),
    (
        TOOL_LOOK_AROUND,
        &[
            arg("radius", Integer, "scene radius in tiles"),
        ],
    ),
    (
        TOOL_FIND_MOBILES,
        &[
            arg("name", Text, "part of the mobile name or title"),
            arg("graphic", Integer, "body graphic to match"),
            arg("distance", Integer, "maximum distance in tiles"),
            arg("notoriety", Text, "innocent, friend, gray, criminal, enemy, murderer, invulnerable or any"),
            arg("species", Text, "species read from the body, such as orc"),
            arg("in_sight", Boolean, "only mobiles a shot or a spell can reach"),
        ],
    ),
    (
        TOOL_ROUTE,
        &[
            needs("x", Integer, "destination tile x"),
            needs("y", Integer, "destination tile y"),
            arg("z", Integer, "destination height; default the surface there"),
        ],
    ),
    (
        TOOL_PARTY,
        &[
            needs("action", Text, "invite, accept, decline, leave, kick or loot"),
            arg("serial", Serial, "the mobile to invite or kick"),
            arg("on", Boolean, "for loot: may the party loot; default true"),
        ],
    ),
    (
        TOOL_MOBILE_STATUS,
        &[needs("serial", Serial, "the mobile whose status is asked")],
    ),
    (
        TOOL_PROMPT_ANSWER,
        &[needs("text", Text, "the answer; a prompt takes at most 128 characters")],
    ),
    (TOOL_PROMPT_CANCEL, &[]),
    (
        TOOL_LINE_OF_SIGHT,
        &[
            arg("serial", Serial, "mobile or item to look at"),
            arg("x", Integer, "spot x; needs y"),
            arg("y", Integer, "spot y; needs x"),
            arg("z", Integer, "spot height; default the character height"),
        ],
    ),
    (
        TOOL_FIND_ITEMS,
        &[
            arg("graphic", Integer, "item graphic to match"),
            arg("container", Serial, "search only inside this container"),
            arg("name", Text, "part of the item display name"),
            arg("distance", TextOrInteger, "maximum distance in tiles; digit text also read"),
            arg("graphics", Integers, "any of these item graphics"),
            arg("hue", TextOrInteger, "only items of this color"),
            arg("x", Integer, "only items on this tile; needs y"),
            arg("y", Integer, "only items on this tile; needs x"),
        ],
    ),
    (
        TOOL_FIND_LANDMARKS,
        &[
            arg("name", Text, "part of the place name"),
            arg("map", Integer, "map index; default is the map underfoot"),
            arg("distance", TextOrInteger, "maximum distance in tiles; digit text also read"),
        ],
    ),
    (
        TOOL_JOURNAL_SEARCH,
        &[
            arg("q", Text, "words to find; empty matches all lines"),
            arg("query", Text, "alias of q, read when q is missing"),
            arg("since", Integer, "only lines after this journal number"),
        ],
    ),
    (
        TOOL_MAP_TILE,
        &[
            arg("x", Integer, "tile x; default 0"),
            arg("y", Integer, "tile y; default 0"),
            arg("z", Integer, "floor height; default the character height"),
        ],
    ),
    (
        TOOL_CAN_WALK,
        &[
            arg("x", Integer, "tile x; default 0"),
            arg("y", Integer, "tile y; default 0"),
            arg("z", Integer, "height to test; none tests any surface"),
        ],
    ),
    (
        TOOL_SAY,
        &[
            needs("text", Text, "words to say; empty text is refused"),
            arg("channel", Text, "say, yell, party, guild or alliance; default say"),
        ],
    ),
    (
        TOOL_WHISPER,
        &[
            needs("text", Text, "words to whisper; empty text is refused"),
        ],
    ),
    (
        TOOL_REPLY,
        &[
            needs("text", Text, "answer words; empty text is refused"),
            arg("to", TextOrInteger, "speaker name or serial; default newest line"),
        ],
    ),
    (
        TOOL_EMOTE,
        &[
            needs("text", Text, "emote words; empty text is refused"),
        ],
    ),
    (
        TOOL_MOVE_TO,
        &[
            needs("x", Integer, "destination tile x"),
            needs("y", Integer, "destination tile y"),
            arg("z", Integer, "height hint; default the character height"),
        ],
    ),
    (
        TOOL_WALK,
        &[
            needs("direction", Text, "n, ne, e, se, s, sw, w or nw"),
            arg("dir", Text, "alias of direction"),
            arg("running", Boolean, "run instead of walk; default false"),
            arg("run", Boolean, "alias of running"),
            arg("hold_ms", Integer, "how long the walk is held"),
            arg("force", Boolean, "send one step even when the map blocks it"),
            arg("slide", Boolean, "a held walk slides along a wall"),
        ],
    ),
    (TOOL_OPEN_DOOR, &[]),
    (
        TOOL_FOLLOW,
        &[
            needs("serial", Serial, "mobile to follow"),
        ],
    ),
    (TOOL_STOP, &[]),
    (TOOL_LOGOUT, &[]),
    (
        TOOL_USE,
        &[
            arg("serial", Serial, "object to double-click"),
            arg("who", Text, "last uses the last object"),
        ],
    ),
    (
        TOOL_SINGLE_CLICK,
        &[
            needs("serial", Serial, "object to click"),
        ],
    ),
    (
        TOOL_ATTACK,
        &[
            needs("serial", Serial, "mobile to attack"),
        ],
    ),
    (
        TOOL_WAR_MODE,
        &[
            arg("on", Boolean, "true for war, false for peace; default true"),
        ],
    ),
    (
        TOOL_LIFT,
        &[
            needs("serial", Serial, "item to lift"),
            arg("amount", Integer, "stack amount; default 1"),
        ],
    ),
    (
        TOOL_DROP,
        &[
            needs("serial", Serial, "the lifted item"),
            arg("dest", Serial, "container or mobile; none drops on the ground"),
            arg("x", Integer, "exact x in container or on ground"),
            arg("y", Integer, "exact y in container or on ground"),
            arg("z", Integer, "ground height; default the character height"),
        ],
    ),
    (
        TOOL_EQUIP,
        &[
            arg("serial", Serial, "item to wear"),
            arg("who", Text, "last wears the last weapon put away"),
            arg("layer", Integer, "wear layer; default one-handed"),
        ],
    ),
    (
        TOOL_UNEQUIP,
        &[
            needs("layer", Integer, "worn layer to move into the backpack"),
        ],
    ),
    (
        TOOL_CAST,
        &[
            needs("spell", TextOrInteger, "spell number or name, such as greater heal"),
            arg("target", Serial, "serial or self; answers the spell's cursor"),
        ],
    ),
    (
        TOOL_USE_SKILL,
        &[
            needs("skill", TextOrInteger, "skill number or name, such as hiding"),
        ],
    ),
    (
        TOOL_NEXT_EVENT,
        &[
            arg("timeout_ms", Integer, "wait time; default 5000, max 7000"),
        ],
    ),
    (
        TOOL_WAIT_JOURNAL,
        &[
            needs("q", Text, "words the new journal line must hold"),
            arg("timeout_ms", Integer, "wait time; default 5000, max 7000"),
        ],
    ),
    (
        TOOL_WAIT_TARGET,
        &[
            arg("timeout_ms", Integer, "wait time; default 5000, max 7000"),
        ],
    ),
    (
        TOOL_TARGET,
        &[
            arg("serial", Serial, "object to target"),
            arg("who", Text, "self or last"),
            arg("x", Integer, "ground x; needs y"),
            arg("y", Integer, "ground y; needs x"),
            arg("z", Integer, "ground height; default the character height"),
            arg("graphic", Integer, "ground tile graphic; default bare land"),
        ],
    ),
    (
        TOOL_OPEN_CONTAINER,
        &[
            arg("serial", Serial, "container to open"),
            arg("who", Text, "last opens the last object"),
        ],
    ),
    (
        TOOL_LOOT,
        &[
            needs("serial", Serial, "corpse to loot"),
        ],
    ),
    (
        TOOL_DEPOSIT,
        &[
            arg("graphic", Integer, "bank only items of this graphic"),
        ],
    ),
    (
        TOOL_TRADE_OFFER,
        &[
            needs("serial", Serial, "mobile to trade with"),
        ],
    ),
    (
        TOOL_TRADE_ACCEPT,
        &[
            arg("accept", Boolean, "true ticks, false unticks; default true"),
        ],
    ),
    (TOOL_TRADE_CANCEL, &[]),
    (
        TOOL_VENDOR_SELL,
        &[
            needs("vendor_name", Text, "name of the nearby vendor"),
            needs("graphic", Integer, "graphic of the backpack items to sell"),
        ],
    ),
    (
        TOOL_VENDOR_BUY,
        &[
            needs("vendor", Serial, "vendor mobile serial"),
            needs("item", Serial, "shop item serial"),
            arg("amount", Integer, "amount to buy; default 1, 0 is refused"),
        ],
    ),
    (
        TOOL_CONTEXT_MENU,
        &[
            needs("serial", Serial, "object whose context menu is asked"),
            arg("cliloc", TextOrInteger, "entry cliloc to pick at once"),
            arg("index", Integer, "without cliloc: pick this shown entry"),
        ],
    ),
    (
        TOOL_GUMP_RESPOND,
        &[
            arg("gump", TextOrInteger, "gump id; default the first open gump"),
            arg("button", Integer, "button id; default 0 closes"),
            arg("switches", Integers, "switch ids to tick"),
            arg("texts", Objects, "text fields as objects with id and text"),
        ],
    ),
    (
        TOOL_GUMP_CLOSE,
        &[
            arg("gump", TextOrInteger, "gump id; default the first open gump"),
        ],
    ),
    (
        TOOL_SET_GOAL,
        &[
            arg("goal", Text, "idle, travel, hunt, gather, chop, mine, bank, shop, social, flee or ress"),
            arg("x", Integer, "travel spot x; needs y"),
            arg("y", Integer, "travel spot y; needs x"),
            arg("z", Integer, "travel height hint"),
        ],
    ),
    (TOOL_CANCEL_GOAL, &[]),
    (
        TOOL_RUN_SCRIPT,
        &[
            arg("name", Text, "script file name in the scripts folder"),
            arg("text", Text, "script source text; wins over name"),
            arg("loop", Boolean, "run again each time it ends"),
        ],
    ),
    (TOOL_STOP_SCRIPT, &[]),
    (TOOL_SCRIPT_STATUS, &[]),
    (TOOL_LIST_SCRIPTS, &[]),
    (
        TOOL_SCRIPT_READ,
        &[
            needs("name", Text, "script name; letters, digits, space, - and _"),
        ],
    ),
    (
        TOOL_SCRIPT_SAVE,
        &[
            needs("name", Text, "script name; letters, digits, space, - and _"),
            arg("text", Text, "script text; must parse; default empty"),
        ],
    ),
    (TOOL_AGENTS, &[]),
    (
        TOOL_AGENT_SET,
        &[
            needs("agent", Text, "agent name, such as autoloot or bandage"),
            needs("settings", Object, "new settings; an array for an item list"),
            arg("list", Text, "named list; organizer, restock, dress, targets need it"),
        ],
    ),
    (
        TOOL_AGENT_ON,
        &[
            needs("agent", Text, "agent to switch"),
            arg("on", Boolean, "true on, false off; default true"),
            arg("list", Text, "item list the agent uses"),
        ],
    ),
    (
        TOOL_AGENT_RUN,
        &[
            needs("agent", Text, "organizer, restock, dress, undress or autoloot"),
            arg("list", Text, "list name; organizer and restock need it"),
        ],
    ),
    (TOOL_AGENT_STOP, &[]),
    (
        TOOL_DAMAGE_METER,
        &[
            arg("action", Text, "start, pause, resume, stop or report; default report"),
        ],
    ),
    (
        TOOL_TARGET_FILTER,
        &[
            needs("name", Text, "name of the target filter"),
        ],
    ),
    (
        TOOL_HOTKEYS,
        &[
            arg("group", Text, "show only this hotkey group"),
            arg("name", Text, "show one hotkey with its script lines"),
        ],
    ),
    (
        TOOL_HOTKEY,
        &[
            needs("name", Text, "hotkey name to press"),
        ],
    ),
    (
        TOOL_RECORD_MACRO,
        &[
            needs("action", Text, "start, stop or cancel"),
            arg("name", Text, "macro name; start needs it"),
        ],
    ),
    (TOOL_JOBS, &[]),
    (
        TOOL_JOB_START,
        &[
            needs("job", Text, "hunt or walk"),
            arg("replace", Boolean, "take over a running job; default false"),
            arg("include", Texts, "hunt terms to attack"),
            arg("avoid", Texts, "hunt terms to leave alone"),
            arg("x", Integer, "walk destination x; needs y"),
            arg("y", Integer, "walk destination y; needs x"),
            arg("z", Integer, "walk destination height; default the character height"),
            arg("name", Text, "walk to the nearest landmark with this name"),
            arg("watch", Boolean, "stand guard after the walk arrives"),
        ],
    ),
    (TOOL_JOB_STOP, &[]),
    (
        TOOL_WATCH,
        &[
            arg("size", Integer, "radar width in tiles; default 21"),
        ],
    ),
    (
        TOOL_PROPERTIES,
        &[
            needs("serial", Serial, "object whose tooltip words are asked"),
        ],
    ),
    (TOOL_CLOSE_MENU, &[]),
    (
        TOOL_SHOP_CHECKOUT,
        &[
            needs("items", Objects, "cart rows as objects with serial and amount"),
        ],
    ),
    (TOOL_SHOP_CLOSE, &[]),
    (
        TOOL_MENU_PICK,
        &[
            arg("index", Integer, "entry from 1; none cancels the menu"),
        ],
    ),
    (TOOL_BOOK_CLOSE, &[]),
    (
        TOOL_BOARD_READ,
        &[
            needs("message", Serial, "message serial on the open board"),
        ],
    ),
    (
        TOOL_BOARD_POST,
        &[
            needs("subject", Text, "message subject"),
            arg("text", Text, "message body; lines split by line breaks"),
            arg("reply_to", Serial, "message this post answers"),
        ],
    ),
    (
        TOOL_BOARD_REMOVE,
        &[
            needs("message", Serial, "own message serial to remove"),
        ],
    ),
    (TOOL_BOARD_CLOSE, &[]),
    (
        TOOL_MAP_PIN,
        &[
            arg("serial", Serial, "open map item; default the last opened map"),
            arg("action", Text, "clear or edit; none adds a pin"),
            arg("x", Integer, "pin x in map pixels; needed to add a pin"),
            arg("y", Integer, "pin y in map pixels; needed to add a pin"),
        ],
    ),
    (
        TOOL_MAP_CLOSE,
        &[
            arg("serial", Serial, "map item to forget; default the last opened"),
        ],
    ),
    (
        TOOL_PROFILE,
        &[
            needs("serial", Serial, "character whose profile is asked or written"),
            arg("text", Text, "new profile words for your own character"),
        ],
    ),
    (
        TOOL_HOUSE_EDIT,
        &[
            needs("action", Text, "add, remove, stair, roof, remove_roof, floor, clear, revert, commit, exit, backup, restore"),
            arg("graphic", Integer, "part graphic; add, remove, stair, roof, remove_roof need it"),
            arg("x", Integer, "tile x for a part; default 0"),
            arg("y", Integer, "tile y for a part; default 0"),
            arg("z", Integer, "height for remove, roof, remove_roof; default 0"),
            arg("level", Integer, "floor level from 1; default 1"),
        ],
    ),
    (TOOL_HELP, &[]),
    (
        TOOL_CHAT,
        &[
            needs("action", Text, "open, join, say or leave"),
            arg("name", Text, "chat name for open; default the character name"),
            arg("channel", Text, "channel to join; join needs it"),
            arg("password", Text, "channel password for join"),
            arg("text", Text, "words to say; say needs it"),
        ],
    ),
    (
        TOOL_BOOK_WRITE,
        &[
            arg("title", Text, "new book title"),
            arg("author", Text, "new book author"),
            arg("page", Integer, "page number from 1 to write"),
            arg("text", Text, "page text; lines split by line breaks"),
        ],
    ),
    (
        TOOL_TRADE_GOLD,
        &[
            arg("gold", TextOrInteger, "gold offered; default 0"),
            arg("platinum", TextOrInteger, "platinum offered; default 0"),
        ],
    ),
    (
        TOOL_SET_PERSONA,
        &[
            needs("name", Text, "persona name"),
            needs("class", Text, "persona class, such as miner or warrior"),
            needs("tier", Text, "persona tier"),
            arg("active_hours", Texts, "hours the persona plays"),
            arg("risk_tolerance", Number, "fraction from 0 to 1; default from the class"),
            arg("chat_rate_per_hour", Integer, "speech lines per hour; default from the class"),
            arg("typo_rate", Number, "fraction from 0 to 1; default from the class"),
            arg("allow_emote", Boolean, "allow asterisk emotes; default false"),
            arg("play_along", Object, "plans, stay_minutes, risk_tolerance, reply_style"),
        ],
    ),
];

/// The tools that need one of several arguments, and which.
pub(crate) const ONE_OF: &[(&str, &[&str])] = &[
    (TOOL_LINE_OF_SIGHT, &["serial", "x"]),
    (TOOL_USE, &["serial", "who"]),
    (TOOL_EQUIP, &["serial", "who"]),
    (TOOL_OPEN_CONTAINER, &["serial", "who"]),
    (TOOL_RUN_SCRIPT, &["name", "text"]),
    (TOOL_JOB_START, &["x", "name"]),
    (TOOL_BOOK_WRITE, &["page", "title", "author"]),
];
