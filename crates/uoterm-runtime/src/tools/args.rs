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
        crate::characters::TOOL_CONNECT,
        &[
            arg("profile", Text, "saved login name in the profiles folder"),
            arg("account", Text, "account name; needs password_env"),
            arg("password_env", Text, "environment variable that holds the password"),
            arg("host", Text, "login server; default the config file"),
            arg("port", Integer, "login port; default the config file"),
            arg("shard", Text, "shard name of the list; default the first"),
            arg("era", Text, "t2a or modern; default the saved login or config"),
            arg("version", Text, "client version, such as 7.0.20.0"),
            arg("encryption", Text, "none or osi; default none"),
            arg("proxy", Text, "socks5:// or http:// proxy; default the config file"),
            arg("character", Text, "character to play; default the first"),
        ],
    ),
    (
        crate::characters::TOOL_DISCONNECT,
        &[needs("session_id", Text, "the session to end")],
    ),
    (
        crate::characters::TOOL_CHARACTERS,
        &[
            arg("profile", Text, "saved login name in the profiles folder"),
            arg("account", Text, "account name; needs password_env"),
            arg("password_env", Text, "environment variable that holds the password"),
            arg("host", Text, "login server; default the config file"),
            arg("port", Integer, "login port; default the config file"),
            arg("shard", Text, "shard name of the list; default the first"),
            arg("era", Text, "t2a or modern; default the saved login or config"),
            arg("version", Text, "client version, such as 7.0.20.0"),
            arg("encryption", Text, "none or osi; default none"),
            arg("proxy", Text, "socks5:// or http:// proxy; default the config file"),
        ],
    ),
    (
        crate::characters::TOOL_CHARACTER_CREATE,
        &[
            arg("profile", Text, "saved login name in the profiles folder"),
            arg("account", Text, "account name; needs password_env"),
            arg("password_env", Text, "environment variable that holds the password"),
            arg("host", Text, "login server; default the config file"),
            arg("port", Integer, "login port; default the config file"),
            arg("shard", Text, "shard name of the list; default the first"),
            arg("era", Text, "t2a or modern; default the saved login or config"),
            arg("version", Text, "client version, such as 7.0.20.0"),
            arg("encryption", Text, "none or osi; default none"),
            arg("proxy", Text, "socks5:// or http:// proxy; default the config file"),
            needs("name", Text, "the new character's name"),
            arg("female", Boolean, "default false"),
            arg("race", Text, "human, elf or gargoyle; default human"),
            arg("str", Integer, "strength, 10 to 60"),
            arg("dex", Integer, "dexterity, 10 to 60"),
            arg("int", Integer, "intelligence, 10 to 60"),
            arg("skills", Objects, "starting skills as objects with skill and value"),
            arg("skin_hue", Integer, "skin colour"),
            arg("hair", Integer, "hair graphic; 0 is none"),
            arg("hair_hue", Integer, "hair colour"),
            arg("beard", Integer, "beard graphic; 0 is none"),
            arg("beard_hue", Integer, "beard colour"),
            arg("shirt_hue", Integer, "shirt colour"),
            arg("pants_hue", Integer, "trousers colour"),
            arg("profession", Integer, "profession number; 0 uses the stats and skills given"),
            arg("start_city", Integer, "start town index of characters"),
        ],
    ),
    (
        crate::characters::TOOL_CHARACTER_DELETE,
        &[
            arg("profile", Text, "saved login name in the profiles folder"),
            arg("account", Text, "account name; needs password_env"),
            arg("password_env", Text, "environment variable that holds the password"),
            arg("host", Text, "login server; default the config file"),
            arg("port", Integer, "login port; default the config file"),
            arg("shard", Text, "shard name of the list; default the first"),
            arg("era", Text, "t2a or modern; default the saved login or config"),
            arg("version", Text, "client version, such as 7.0.20.0"),
            arg("encryption", Text, "none or osi; default none"),
            arg("proxy", Text, "socks5:// or http:// proxy; default the config file"),
            arg("name", Text, "the character to delete"),
            arg("slot", Integer, "the slot to delete, from 0"),
        ],
    ),
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
            arg("z_min", Integer, "lowest height kept"),
            arg("z_max", Integer, "highest height kept"),
        ],
    ),
    (
        TOOL_ROUTE,
        &[
            arg("x", Integer, "destination tile x; needs y"),
            arg("y", Integer, "destination tile y; needs x"),
            arg("z", Integer, "destination height; default the surface there"),
            arg("name", Text, "walk to the nearest landmark with this name"),
            arg("accuracy", Integer, "stop within this many tiles; default 0, most 18"),
            arg("avoid", Objects, "areas {x, y, radius} and creatures {serial, radius} to keep away from"),
            arg("open_doors", Boolean, "open doors on the way; default true"),
            arg("roads", Boolean, "prefer roads to grass and forest; default true"),
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
        &[
            needs("serial", Serial, "the mobile whose status is asked"),
            arg("close", Boolean, "tell the shard its status bar is shut; default false"),
        ],
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
            arg("z", Integer, "spot floor height; default the floor there"),
            arg("from", Serial, "mobile or item to look from; default the character"),
            arg("from_x", Integer, "spot to look from, x; needs from_y"),
            arg("from_y", Integer, "spot to look from, y; needs from_x"),
            arg("from_z", Integer, "floor height to look from; default the floor there"),
            arg("mode", Text, "runuo, modernuo, servuo, pol or sphere; default the sight_mode option"),
            arg("trace", Boolean, "give every point of the line; default false"),
        ],
    ),
    (
        TOOL_FIND_TILES,
        &[
            arg("group", Text, "water, trees, ore, forge, anvil, loom, oven or mill"),
            arg("graphics", Integers, "land ids or static graphics; any of them"),
            arg("flags", Texts, "tiledata flag names; every one must be set"),
            arg("name", Text, "a word of the tiledata name"),
            arg("layer", Text, "land, statics or both; default both"),
            arg("x", Integer, "area middle x; default the character"),
            arg("y", Integer, "area middle y; default the character"),
            arg("radius", Integer, "area radius in tiles; default 12, most 64"),
            arg("x1", Integer, "rectangle corner x; needs y1, x2, y2"),
            arg("y1", Integer, "rectangle corner y"),
            arg("x2", Integer, "rectangle far corner x"),
            arg("y2", Integer, "rectangle far corner y"),
            arg("z_min", Integer, "lowest height kept"),
            arg("z_max", Integer, "highest height kept"),
            arg("map", Integer, "map index; default the map underfoot"),
            arg("page", Integer, "page from 1; default 1"),
            arg("page_size", Integer, "rows a page; default 50, most 200"),
        ],
    ),
    (
        TOOL_MULTI_PARTS,
        &[
            arg("serial", Serial, "the house or boat"),
            arg("x", Integer, "tile x; needs y"),
            arg("y", Integer, "tile y; needs x"),
            arg("page", Integer, "page from 1; default 1"),
            arg("page_size", Integer, "rows a page; default 50, most 200"),
        ],
    ),
    (
        TOOL_FIND_ENTRANCES,
        &[
            arg("x", Integer, "scan middle x; default the character"),
            arg("y", Integer, "scan middle y; default the character"),
            arg("radius", Integer, "scan radius; default 32, most 64"),
            arg("map", Integer, "map index; default the map underfoot"),
            arg("page", Integer, "page from 1; default 1"),
            arg("page_size", Integer, "rows a page; default 50, most 200"),
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
            arg("z_min", Integer, "lowest height kept"),
            arg("z_max", Integer, "highest height kept"),
        ],
    ),
    (
        TOOL_FIND_LANDMARKS,
        &[
            arg("name", Text, "part of the place name"),
            arg("kind", Text, "marker word, such as bank or moongate"),
            arg("closest", Text, "a kind: give only the nearest one"),
            arg("map", Integer, "map index; default is the map underfoot"),
            arg("distance", TextOrInteger, "maximum distance in tiles; digit text also read"),
        ],
    ),
    (TOOL_LANDMARKS_INFO, &[]),
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
            needs("x", Integer, "tile x"),
            needs("y", Integer, "tile y"),
            arg("z", Integer, "floor height; default the character height"),
            arg("map", Integer, "map index; default the map underfoot"),
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
            arg("hue", TextOrInteger, "speech colour; default the speech_hue option"),
        ],
    ),
    (
        TOOL_WHISPER,
        &[
            needs("text", Text, "words to whisper; empty text is refused"),
            arg("hue", TextOrInteger, "speech colour; default the speech_hue option"),
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
            arg("x", Integer, "destination tile x; needs y"),
            arg("y", Integer, "destination tile y; needs x"),
            arg("z", Integer, "height hint; default the character height"),
            arg("name", Text, "walk to the nearest landmark with this name"),
            arg("run", Boolean, "true runs, false walks; default by stamina and danger"),
            arg("accuracy", Integer, "stop within this many tiles; default 0, most 18"),
            arg("avoid", Objects, "areas {x, y, radius} and creatures {serial, radius} to keep away from"),
            arg("open_doors", Boolean, "open doors on the way; default true"),
            arg("roads", Boolean, "prefer roads to grass and forest; default true"),
            arg("exact", Boolean, "fail instead of walking as near as a route goes"),
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
            arg("open_doors", Boolean, "open a closed door on the first tile ahead"),
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
    (
        TOOL_LOGOUT,
        &[
            arg("then_play", Text, "character of the account to log in as next, in this session"),
        ],
    ),
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
            arg("book", Serial, "spellbook to cast from; none lets the shard find one"),
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
            arg("ambient", Texts, "busy kinds to get too: sound, effect, animation, item_deleted, member_positions or all"),
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
            arg("trade", Serial, "the other player or a trade box; default the newest trade"),
        ],
    ),
    (
        TOOL_TRADE_CANCEL,
        &[
            arg("trade", Serial, "the other player or a trade box; default the newest trade"),
        ],
    ),
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
            arg("slot", Text, "slot name; default the script name, or text"),
            arg("for", Number, "stop after this many seconds"),
            arg("iterations", Integer, "stop after this many runs from the top"),
        ],
    ),
    (
        TOOL_STOP_SCRIPT,
        &[
            arg("slot", Text, "the slot to stop; none stops every script"),
        ],
    ),
    (
        TOOL_SCRIPT_STATUS,
        &[
            arg("slot", Text, "the slot to report; none reports them all"),
        ],
    ),
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
        TOOL_DYE,
        &[
            needs("hue", Integer, "the colour for the dye tub"),
        ],
    ),
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
            arg("action", Text, "move, remove, clear or edit; none adds a pin"),
            arg("pin", Integer, "pin place in the list from 0; move and remove need it"),
            arg("x", Integer, "pin x in map pixels; add and move need it"),
            arg("y", Integer, "pin y in map pixels; add and move need it"),
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
            needs("action", Text, "add, remove, stair, roof, remove_roof, floor, clear, revert, commit, exit, backup, restore, sync"),
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
            needs("action", Text, "open, join, create, say or leave"),
            arg("name", Text, "chat name for open; default the character name"),
            arg("channel", Text, "channel to join or create; join and create need it"),
            arg("password", Text, "channel password for join or create"),
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
            arg("trade", Serial, "the other player or a trade box; default the newest trade"),
        ],
    ),
    (
        TOOL_OPEN_SPELLBOOK,
        &[
            needs("kind", Text, "magery, necromancy, chivalry, bushido, ninjitsu, spellweaving or mysticism"),
        ],
    ),
    (
        TOOL_TIP,
        &[
            arg("next", Boolean, "the next tip; false for the one before; default true"),
        ],
    ),
    (
        TOOL_QUEST_ARROW,
        &[
            arg("right", Boolean, "click with the right button; default false"),
        ],
    ),
    (
        TOOL_BOAT_MOVE,
        &[
            needs("direction", Text, "n, ne, e, se, s, sw, w or nw"),
            arg("speed", Text, "stop, slow or fast; default fast"),
        ],
    ),
    (
        TOOL_TRACK_MEMBERS,
        &[
            needs("who", Text, "party or guild"),
        ],
    ),
    (
        TOOL_HOUSE_CONTENT,
        &[
            needs("show", Boolean, "show what stands inside public houses"),
        ],
    ),
    (
        TOOL_RACE_CHANGE,
        &[
            arg("cancel", Boolean, "say no to the race change; default false"),
            arg("skin_hue", Integer, "skin hue from skin_hues; default the first"),
            arg("hair", Integer, "hair graphic from hair_styles, 0 for none; default 0"),
            arg("hair_hue", Integer, "hair hue from hair_hues; default the first"),
            arg("beard", Integer, "beard graphic from beard_styles, 0 for none; default 0"),
            arg("beard_hue", Integer, "beard hue from hair_hues; default the first"),
        ],
    ),
    (
        TOOL_BOOK_READ,
        &[
            needs("page", Integer, "page number from 1"),
        ],
    ),
    (
        TOOL_VIRTUE,
        &[needs("name", Text, "humility, sacrifice, compassion, spirituality, valor, honor, justice or honesty")],
    ),
    (
        TOOL_VIRTUE_GUMP,
        &[arg("serial", Serial, "whose virtues; default the character")],
    ),
    (
        TOOL_SKILL_LOCK,
        &[
            needs("skill", TextOrInteger, "skill number or name"),
            needs("lock", Text, "up, down or locked"),
        ],
    ),
    (
        TOOL_STAT_LOCK,
        &[
            needs("stat", Text, "str, dex or int"),
            needs("lock", Text, "up, down or locked"),
        ],
    ),
    (
        TOOL_RENAME,
        &[
            needs("serial", Serial, "the pet"),
            needs("name", Text, "its new name"),
        ],
    ),
    (
        TOOL_SET_ABILITY,
        &[
            needs("ability", Text, "primary, secondary, stun or disarm"),
            arg("on", Boolean, "false clears it; default true"),
        ],
    ),
    (
        TOOL_EMOTE_ACTION,
        &[needs("action", Text, "body action, such as bow or salute")],
    ),
    (
        TOOL_FLY,
        &[arg("on", Boolean, "true takes off, false lands; default true")],
    ),
    (
        TOOL_MENU_BUTTON,
        &[needs("which", Text, "quests or guild")],
    ),
    (
        TOOL_TARGET_RESOURCE,
        &[
            needs("tool", Serial, "the harvest tool"),
            needs("resource", Text, "ore, sand, wood, graves or red mushrooms"),
        ],
    ),
    (
        TOOL_USE_TYPE,
        &[
            needs("graphic", Integer, "item graphic"),
            arg("hue", TextOrInteger, "item colour; default any"),
            arg("source", TextOrInteger, "backpack, ground, world or a container serial; default backpack"),
            arg("range", Integer, "ground range in tiles"),
        ],
    ),
    (
        TOOL_USE_ON,
        &[
            needs("item", Serial, "the item to use"),
            needs("target", Serial, "the mobile it is used on"),
        ],
    ),
    (
        TOOL_CATCH_BAG,
        &[
            arg("serial", Serial, "the container loot goes into"),
            arg("clear", Boolean, "true clears it"),
        ],
    ),
    (
        TOOL_MOUNT,
        &[arg("serial", Serial, "the mount; default the remount agent's or the nearest pet mount")],
    ),
    (TOOL_DISMOUNT, &[]),
    (
        TOOL_ATTACK_NEAREST,
        &[
            arg("notoriety", Text, "gray, criminal, enemy or murderer; default all four"),
            arg("species", Text, "species read from the body, such as orc"),
            arg("name", Text, "part of the name or title"),
            arg("distance", Integer, "most tiles away"),
        ],
    ),
    (
        TOOL_IGNORE_LIST,
        &[
            arg("list", Text, "gumps or journal; show needs none"),
            arg("action", Text, "add, remove, clear or show; default show"),
            arg("value", TextOrInteger, "a gump id, or words of a speaker or a line"),
        ],
    ),
    (
        TOOL_SKILL_GAINS,
        &[
            arg("skill", TextOrInteger, "only this skill, by number or name"),
            arg("clear", Boolean, "start the record again"),
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
    (TOOL_MULTI_PARTS, &["serial", "x"]),
    (TOOL_FIND_TILES, &["group", "graphics", "flags", "name"]),
    (TOOL_USE, &["serial", "who"]),
    (TOOL_EQUIP, &["serial", "who"]),
    (TOOL_OPEN_CONTAINER, &["serial", "who"]),
    (TOOL_RUN_SCRIPT, &["name", "text"]),
    (TOOL_JOB_START, &["x", "name"]),
    (TOOL_MOVE_TO, &["x", "name"]),
    (crate::characters::TOOL_CONNECT, &["profile", "account"]),
    (crate::characters::TOOL_CHARACTERS, &["profile", "account"]),
    (
        crate::characters::TOOL_CHARACTER_CREATE,
        &["profile", "account"],
    ),
    (crate::characters::TOOL_CHARACTER_DELETE, &["name", "slot"]),
    (TOOL_ROUTE, &["x", "name"]),
    (TOOL_BOOK_WRITE, &["page", "title", "author"]),
];
