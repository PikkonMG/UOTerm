//! The acts of the human on the character: what a click or a line of text
//! asks for, the tool calls that make it, and the words that tell the
//! human what was done. The play window and the web client both send them;
//! each caller makes the calls its own way.
//!
//! Each call says `human: true` (`with_human`). The session lets those
//! through while it refuses the agent.

use crate::frame::WatchFrame;
use serde_json::{json, Value};
use std::time::Duration;
use uoterm_world::tool_names::{
    ARG_HUMAN, TOOL_AGENT_ON, TOOL_AGENT_RUN, TOOL_AGENT_SET, TOOL_AGENT_STOP, TOOL_ATTACK,
    TOOL_BOARD_CLOSE, TOOL_BOARD_POST, TOOL_BOARD_READ, TOOL_BOARD_REMOVE, TOOL_BOOK_CLOSE,
    TOOL_BOOK_READ, TOOL_BOOK_WRITE, TOOL_CAST, TOOL_CHAT, TOOL_CLOSE_MENU, TOOL_COMMAND,
    TOOL_CONTEXT_MENU, TOOL_DAMAGE_METER, TOOL_DEPOSIT, TOOL_DROP, TOOL_DYE, TOOL_EQUIP,
    TOOL_FOLLOW, TOOL_GAME_VIEW, TOOL_GUMP_CLOSE, TOOL_GUMP_RESPOND, TOOL_HELP, TOOL_HOTKEY,
    TOOL_HOUSE_CONTENT, TOOL_HOUSE_EDIT, TOOL_LIFT, TOOL_LOGOUT, TOOL_LOOT, TOOL_MAP_CLOSE,
    TOOL_MAP_PIN, TOOL_MENU_PICK, TOOL_MOBILE_STATUS, TOOL_MOVE_TO, TOOL_OPEN_SPELLBOOK,
    TOOL_PARTY, TOOL_PROFILE, TOOL_QUEST_ARROW, TOOL_RACE_CHANGE, TOOL_RECORD_MACRO,
    TOOL_RELEASE_CONTROL, TOOL_RUN_SCRIPT, TOOL_SAY, TOOL_SCRIPT_SAVE, TOOL_SHOP_CHECKOUT,
    TOOL_SHOP_CLOSE, TOOL_SINGLE_CLICK, TOOL_STOP, TOOL_STOP_SCRIPT, TOOL_TAKE_CONTROL,
    TOOL_TARGET, TOOL_TIP, TOOL_TRADE_ACCEPT, TOOL_TRADE_CANCEL, TOOL_TRADE_GOLD, TOOL_TRADE_OFFER,
    TOOL_UNEQUIP, TOOL_USE, TOOL_USE_SKILL, TOOL_VIRTUE_GUMP, TOOL_WALK, TOOL_WAR_MODE,
    TOOL_WHISPER,
};

/// The shard refuses a drop that comes too soon after the lift.
pub const LIFT_TO_DROP: Duration = Duration::from_millis(650);

/// How long one sent step keeps the character on his way. The window sends
/// the next one before this ends, so a held key is one smooth walk.
pub const STEP_HOLD_MS: u64 = 600;

/// A lift of this many takes the whole pile: the shard cuts it to the pile.
pub const WHOLE_PILE: u16 = u16::MAX;

/// The equip tool's word for the weapon the character held last.
const WHO_LAST_WEAPON: &str = "last";
/// The mark round the words of an emote, as the official client sends it.
const EMOTE_MARK: char = '*';
const QUOTE_SINGLE: char = '\'';
const QUOTE_DOUBLE: char = '"';

/// The way words are spoken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Yell,
    Whisper,
    Emote,
    /// To the whole party.
    Party,
    /// To one member of the party, by serial.
    PartyMember(u32),
    Guild,
    Alliance,
}

impl Channel {
    /// The channel as the say tool names it. None for the ones it does not
    /// speak on.
    pub fn say_channel(self) -> Option<&'static str> {
        Some(match self {
            Channel::Yell => "yell",
            Channel::Party => "party",
            Channel::Guild => "guild",
            Channel::Alliance => "alliance",
            Channel::Whisper | Channel::Emote | Channel::PartyMember(_) => return None,
        })
    }
}

/// Words in the quotes of a script line: single quotes, or double quotes
/// for words with an apostrophe. Words with both lose the double quotes,
/// which a line cannot hold.
pub fn quoted(words: &str) -> String {
    if !words.contains(QUOTE_SINGLE) {
        format!("{QUOTE_SINGLE}{words}{QUOTE_SINGLE}")
    } else {
        let kept: String = words.chars().filter(|c| *c != QUOTE_DOUBLE).collect();
        format!("{QUOTE_DOUBLE}{kept}{QUOTE_DOUBLE}")
    }
}

/// One thing the human tells the character to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Take,
    GiveBack,
    WalkTo {
        x: u16,
        y: u16,
    },
    /// Pathfind to a tile at a run, whatever the stamina.
    RunTo {
        x: u16,
        y: u16,
    },
    Use(u32),
    Look(u32),
    Attack(u32),
    Follow(u32),
    Loot(u32),
    Target(u32),
    TargetGround {
        x: u16,
        y: u16,
        z: i8,
    },
    CancelTarget,
    War(bool),
    /// Plain speech, in a hue.
    Say {
        text: String,
        hue: u16,
    },
    /// Words on another channel than plain speech, in a hue. A party line
    /// carries none.
    Speak {
        channel: Channel,
        text: String,
        hue: u16,
    },
    /// A hotkey of the session, by name.
    Hotkey(String),
    Stop,
    Deposit,
    /// Ask the shard to show what stands inside public houses, or not.
    HouseContent(bool),
    /// Tell the shard the size of the game view the window draws.
    GameView {
        width: u32,
        height: u32,
    },
    /// One step while a key or the right mouse button is down.
    Step {
        direction: &'static str,
        run: bool,
        /// A closed door ahead opens, by the General page's auto open doors.
        open_doors: bool,
    },
    /// Lift an item, or a part of a pile, and drop it at a place.
    Move {
        item: u32,
        amount: u16,
        to: DropTo,
    },
    Wear(u32),
    /// Wear the weapon the character held last.
    WearLastWeapon,
    /// Take off what the character wears on this layer.
    TakeOff(u8),
    /// Ask the shard for the context menu of a thing.
    Menu(u32),
    MenuPick {
        serial: u32,
        index: u16,
    },
    MenuClose,
    /// Answer the old-style menu: an entry from one, or none to walk away.
    OldMenuPick(Option<u16>),
    BookClose,
    /// Name the open book, or write one of its pages.
    BookName {
        title: String,
        author: String,
    },
    BookPage {
        page: u16,
        text: String,
    },
    /// Ask the shard for a page of the open book, from one, that it has
    /// not sent yet.
    BookRead(u16),
    /// Ask for the lines of a message of the open bulletin board.
    BoardRead(u32),
    BoardPost {
        subject: String,
        text: String,
        reply_to: Option<u32>,
    },
    BoardRemove(u32),
    BoardClose,
    /// Put a pin on the open map, in pixels of its picture.
    MapPin {
        x: u16,
        y: u16,
    },
    /// Move one pin of the open map, by its place in the list from 0.
    MapPinMove {
        pin: u8,
        x: u16,
        y: u16,
    },
    /// Take one pin off the open map, by its place in the list from 0.
    MapPinRemove(u8),
    MapClear,
    /// Ask the shard to let the open map be drawn on.
    MapEdit,
    MapClose(u32),
    /// Ask for the profile a player wrote about a character.
    ProfileRead(u32),
    ProfileWrite {
        serial: u32,
        text: String,
    },
    /// One step of the house designer.
    HouseEdit {
        action: &'static str,
        graphic: u16,
        x: i32,
        y: i32,
        z: i32,
    },
    /// The level the designer works on, from 1.
    HouseFloor(u8),
    /// A step of the designer that names no part: clear, revert, commit,
    /// exit, backup, restore.
    HouseCommand(&'static str),
    /// Ask the shard for its help menu.
    Help,
    /// Click the arrow the shard points at a place, with the right button
    /// or the left one.
    QuestArrow {
        right: bool,
    },
    /// Answer the race change of the shard with new looks, or say no.
    RaceChange(Option<uoterm_protocol::NewLooks>),
    ChatOpen(String),
    ChatJoin(String),
    /// Join a chat channel that has a password.
    ChatJoinWithPassword {
        channel: String,
        password: String,
    },
    /// Make a chat channel and join it.
    ChatCreate(String),
    ChatSay(String),
    ChatLeave,
    /// Ask for the tip of the day after the one shown, or before it.
    Tip {
        next: bool,
    },
    /// Leave the world. The window closes and the program ends.
    Quit,
    /// Buy or sell the rows of the cart: the item and how many.
    Checkout(Vec<(u32, u16)>),
    ShopClose,
    TradeWith(u32),
    /// Tick or untick the accept box of a trade, named by the character's
    /// own box of it.
    TradeAccept {
        trade: u32,
        accept: bool,
    },
    TradeCancel(u32),
    TradeGold {
        trade: u32,
        gold: u32,
        platinum: u32,
    },
    UseSkill(u16),
    Cast(u16),
    /// Cast a spell from one spellbook, as a click in the book does.
    CastFrom {
        spell: u16,
        book: u32,
    },
    /// Answer the dye tub that asks for a colour.
    Dye(u16),
    /// Ask the shard to open the character's spellbook of a school.
    OpenSpellbook(&'static str),
    /// One line of the script language: a prompt answer, a skill lock.
    Command(String),
    ScriptRun {
        text: String,
        looping: bool,
    },
    ScriptStop,
    ScriptSave {
        name: String,
        text: String,
    },
    /// Start to record what the human does as a macro with this name.
    RecordStart(String),
    RecordStop,
    GumpButton {
        gump: u32,
        button: u32,
        switches: Vec<u32>,
        /// The words the human typed, by the id of the field.
        texts: Vec<(u16, String)>,
    },
    GumpClose(u32),
    /// Switch an agent of the session on or off.
    AgentOn {
        agent: String,
        on: bool,
    },
    /// Replace the settings of an agent, or one named list of it.
    AgentSet {
        agent: String,
        list: Option<String>,
        settings: Value,
    },
    /// Run an agent job once, with its list when it needs one.
    AgentRun {
        agent: String,
        list: Option<String>,
    },
    AgentStop,
    /// Start, pause, resume or stop the damage meter.
    DamageMeter(&'static str),
    PartyInvite(u32),
    PartyLeave,
    PartyKick(u32),
    /// Let the party loot what the character kills, or not.
    PartyLoot(bool),
    /// Ask the shard for the status of a mobile, whose health bar opens,
    /// or tell it the bar closed.
    MobileStatus {
        serial: u32,
        close: bool,
    },
    /// Ask the shard for the virtue gump of a mobile.
    VirtueGump(u32),
    /// Words for Jev to turn into one of the acts above.
    Order(String, Box<WatchFrame>),
}

/// Where a moved item lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropTo {
    /// Into a container or onto a mobile. The shard picks the spot.
    Into(u32),
    /// Into a container at a place in its gump, in its pixels.
    IntoAt {
        container: u32,
        x: u16,
        y: u16,
    },
    Ground {
        x: u16,
        y: u16,
        z: i8,
    },
}

/// The tooltip of one thing, as the shard wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tip {
    pub serial: u32,
    pub lines: Vec<String>,
}

/// The panel that asked. Each panel takes only the answers to its own asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asker {
    Macros,
    Designer,
    Chat,
    Deck,
    MapItem,
}

/// A thing a panel asks the session, or Jev, for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    Scripts,
    ScriptText(String),
    ScriptStatus,
    /// Plain words that Jev turns into the script lines of one hotkey.
    LinesFor(String),
    /// Which part of the house catalog the words mean. The answer is its
    /// place in the list the window holds.
    HousePart {
        wish: String,
        options: Vec<String>,
    },
    /// Which thing of the bag or of the body the words mean.
    WearItem {
        wish: String,
        options: Vec<String>,
    },
    /// Which chat channel the words mean.
    Channel {
        wish: String,
        options: Vec<String>,
    },
    /// The named places that lie on a map, and which one the words mean.
    /// The answer is the tile of the place Jev picked.
    PlaceOnMap {
        wish: String,
        map: u8,
        from: (u16, u16),
        to: (u16, u16),
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Scripts(Vec<String>),
    ScriptText {
        name: String,
        text: String,
    },
    /// The state of the running or last script, in words.
    ScriptStatus(String),
    Lines(Result<String, String>),
    /// The tile of the place that was asked for, or words for the human.
    Place(Result<(u16, u16), String>),
    /// The place in the list that Jev picked, or words for the human.
    Picked(Result<usize, String>),
}

/// What came of an act, in words for the human.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub text: String,
    pub failed: bool,
}

impl Act {
    /// The tool calls that make the act, in order.
    pub fn calls(&self) -> Vec<(&'static str, Value)> {
        let serial = |serial: &u32| json!({ "serial": serial });
        match self {
            Self::Take => vec![(TOOL_TAKE_CONTROL, json!({}))],
            Self::GiveBack => vec![(TOOL_RELEASE_CONTROL, json!({}))],
            Self::WalkTo { x, y } => vec![(TOOL_MOVE_TO, json!({ "x": x, "y": y }))],
            Self::RunTo { x, y } => {
                vec![(TOOL_MOVE_TO, json!({ "x": x, "y": y, "run": true }))]
            }
            Self::Use(s) => vec![(TOOL_USE, serial(s))],
            Self::Look(s) => vec![(TOOL_SINGLE_CLICK, serial(s))],
            Self::Attack(s) => vec![(TOOL_ATTACK, serial(s))],
            Self::Follow(s) => vec![(TOOL_FOLLOW, serial(s))],
            Self::Loot(s) => vec![(TOOL_LOOT, serial(s))],
            Self::Target(s) => vec![(TOOL_TARGET, serial(s))],
            Self::TargetGround { x, y, z } => {
                vec![(TOOL_TARGET, json!({ "x": x, "y": y, "z": z }))]
            }
            Self::CancelTarget => vec![(TOOL_TARGET, json!({}))],
            Self::War(on) => vec![(TOOL_WAR_MODE, json!({ "on": on }))],
            Self::Say { text, hue } => vec![(TOOL_SAY, json!({ "text": text, "hue": hue }))],
            Self::Speak { channel, text, hue } => vec![match (channel, channel.say_channel()) {
                (_, Some(on)) => (TOOL_SAY, json!({ "text": text, "channel": on, "hue": hue })),
                (Channel::Whisper, _) => (TOOL_WHISPER, json!({ "text": text, "hue": hue })),
                (Channel::PartyMember(member), _) => (
                    TOOL_COMMAND,
                    json!({ "text": format!("partymsg {} 0 {member:#010X}", quoted(text)) }),
                ),
                (_, None) => (
                    TOOL_COMMAND,
                    json!({ "text": format!("emotemsg {} {hue:#06X}", quoted(&format!("{EMOTE_MARK}{text}{EMOTE_MARK}"))) }),
                ),
            }],
            Self::Hotkey(name) => vec![(TOOL_HOTKEY, json!({ "name": name }))],
            Self::Stop => vec![(TOOL_STOP, json!({}))],
            Self::Deposit => vec![(TOOL_DEPOSIT, json!({}))],
            Self::HouseContent(show) => vec![(TOOL_HOUSE_CONTENT, json!({ "show": show }))],
            Self::GameView { width, height } => {
                vec![(TOOL_GAME_VIEW, json!({ "width": width, "height": height }))]
            }
            Self::Step {
                direction,
                run,
                open_doors,
            } => {
                vec![(
                    TOOL_WALK,
                    json!({
                        "direction": direction,
                        "running": run,
                        "hold_ms": STEP_HOLD_MS,
                        // A held walk slides along a wall, so a doorway
                        // taken a little off the line does not stop him.
                        "slide": true,
                        "open_doors": open_doors,
                    }),
                )]
            }
            Self::Move { item, amount, to } => {
                let mut drop = json!({ "serial": item });
                match *to {
                    DropTo::Into(dest) => drop["dest"] = json!(dest),
                    DropTo::IntoAt { container, x, y } => {
                        drop["dest"] = json!(container);
                        drop["x"] = json!(x);
                        drop["y"] = json!(y);
                    }
                    DropTo::Ground { x, y, z } => {
                        drop["x"] = json!(x);
                        drop["y"] = json!(y);
                        drop["z"] = json!(z);
                    }
                }
                vec![
                    (TOOL_LIFT, json!({ "serial": item, "amount": amount })),
                    (TOOL_DROP, drop),
                ]
            }
            Self::Wear(s) => vec![(TOOL_EQUIP, serial(s))],
            Self::WearLastWeapon => vec![(TOOL_EQUIP, json!({ "who": WHO_LAST_WEAPON }))],
            Self::TakeOff(layer) => vec![(TOOL_UNEQUIP, json!({ "layer": layer }))],
            Self::Menu(s) => vec![(TOOL_CONTEXT_MENU, serial(s))],
            Self::MenuPick { serial, index } => vec![(
                TOOL_CONTEXT_MENU,
                json!({ "serial": serial, "index": index }),
            )],
            Self::MenuClose => vec![(TOOL_CLOSE_MENU, json!({}))],
            Self::OldMenuPick(Some(index)) => vec![(TOOL_MENU_PICK, json!({ "index": index }))],
            Self::OldMenuPick(None) => vec![(TOOL_MENU_PICK, json!({}))],
            Self::BookClose => vec![(TOOL_BOOK_CLOSE, json!({}))],
            Self::BookName { title, author } => {
                vec![(TOOL_BOOK_WRITE, json!({ "title": title, "author": author }))]
            }
            Self::BookPage { page, text } => {
                vec![(TOOL_BOOK_WRITE, json!({ "page": page, "text": text }))]
            }
            Self::BookRead(page) => vec![(TOOL_BOOK_READ, json!({ "page": page }))],
            Self::BoardRead(message) => vec![(TOOL_BOARD_READ, json!({ "message": message }))],
            Self::BoardPost {
                subject,
                text,
                reply_to,
            } => vec![(
                TOOL_BOARD_POST,
                json!({ "subject": subject, "text": text, "reply_to": reply_to }),
            )],
            Self::BoardRemove(message) => {
                vec![(TOOL_BOARD_REMOVE, json!({ "message": message }))]
            }
            Self::BoardClose => vec![(TOOL_BOARD_CLOSE, json!({}))],
            Self::MapPin { x, y } => vec![(TOOL_MAP_PIN, json!({ "x": x, "y": y }))],
            Self::MapPinMove { pin, x, y } => vec![(
                TOOL_MAP_PIN,
                json!({ "action": "move", "pin": pin, "x": x, "y": y }),
            )],
            Self::MapPinRemove(pin) => {
                vec![(TOOL_MAP_PIN, json!({ "action": "remove", "pin": pin }))]
            }
            Self::MapClear => vec![(TOOL_MAP_PIN, json!({ "action": "clear" }))],
            Self::MapEdit => vec![(TOOL_MAP_PIN, json!({ "action": "edit" }))],
            Self::MapClose(map) => vec![(TOOL_MAP_CLOSE, json!({ "serial": map }))],
            Self::ProfileRead(s) => vec![(TOOL_PROFILE, serial(s))],
            Self::ProfileWrite { serial, text } => {
                vec![(TOOL_PROFILE, json!({ "serial": serial, "text": text }))]
            }
            Self::HouseEdit {
                action,
                graphic,
                x,
                y,
                z,
            } => vec![(
                TOOL_HOUSE_EDIT,
                json!({ "action": action, "graphic": graphic, "x": x, "y": y, "z": z }),
            )],
            Self::HouseFloor(level) => vec![(
                TOOL_HOUSE_EDIT,
                json!({ "action": "floor", "level": level }),
            )],
            Self::HouseCommand(action) => {
                vec![(TOOL_HOUSE_EDIT, json!({ "action": action }))]
            }
            Self::Help => vec![(TOOL_HELP, json!({}))],
            Self::QuestArrow { right } => vec![(TOOL_QUEST_ARROW, json!({ "right": right }))],
            Self::RaceChange(None) => vec![(TOOL_RACE_CHANGE, json!({ "cancel": true }))],
            Self::RaceChange(Some(looks)) => vec![(
                TOOL_RACE_CHANGE,
                json!({
                    "skin_hue": looks.skin_hue,
                    "hair": looks.hair,
                    "hair_hue": looks.hair_hue,
                    "beard": looks.beard,
                    "beard_hue": looks.beard_hue,
                }),
            )],
            Self::ChatOpen(name) => {
                vec![(TOOL_CHAT, json!({ "action": "open", "name": name }))]
            }
            Self::ChatJoin(channel) => {
                vec![(TOOL_CHAT, json!({ "action": "join", "channel": channel }))]
            }
            Self::ChatJoinWithPassword { channel, password } => vec![(
                TOOL_CHAT,
                json!({ "action": "join", "channel": channel, "password": password }),
            )],
            Self::ChatCreate(channel) => {
                vec![(TOOL_CHAT, json!({ "action": "create", "channel": channel }))]
            }
            Self::ChatSay(text) => vec![(TOOL_CHAT, json!({ "action": "say", "text": text }))],
            Self::ChatLeave => vec![(TOOL_CHAT, json!({ "action": "leave" }))],
            Self::Tip { next } => vec![(TOOL_TIP, json!({ "next": next }))],
            Self::Quit => vec![(TOOL_LOGOUT, json!({}))],
            Self::Checkout(rows) => {
                let items: Vec<Value> = rows
                    .iter()
                    .map(|(serial, amount)| json!({ "serial": serial, "amount": amount }))
                    .collect();
                vec![(TOOL_SHOP_CHECKOUT, json!({ "items": items }))]
            }
            Self::ShopClose => vec![(TOOL_SHOP_CLOSE, json!({}))],
            Self::TradeWith(s) => vec![(TOOL_TRADE_OFFER, serial(s))],
            Self::TradeAccept { trade, accept } => vec![(
                TOOL_TRADE_ACCEPT,
                json!({ "trade": trade, "accept": accept }),
            )],
            Self::TradeCancel(trade) => vec![(TOOL_TRADE_CANCEL, json!({ "trade": trade }))],
            Self::TradeGold {
                trade,
                gold,
                platinum,
            } => vec![(
                TOOL_TRADE_GOLD,
                json!({ "trade": trade, "gold": gold, "platinum": platinum }),
            )],
            Self::UseSkill(skill) => vec![(TOOL_USE_SKILL, json!({ "skill": skill }))],
            Self::Cast(spell) => vec![(TOOL_CAST, json!({ "spell": spell }))],
            Self::CastFrom { spell, book } => {
                vec![(TOOL_CAST, json!({ "spell": spell, "book": book }))]
            }
            Self::Dye(hue) => vec![(TOOL_DYE, json!({ "hue": hue }))],
            Self::OpenSpellbook(kind) => vec![(TOOL_OPEN_SPELLBOOK, json!({ "kind": kind }))],
            Self::Command(text) => vec![(TOOL_COMMAND, json!({ "text": text }))],
            Self::ScriptRun { text, looping } => {
                vec![(TOOL_RUN_SCRIPT, json!({ "text": text, "loop": looping }))]
            }
            Self::ScriptStop => vec![(TOOL_STOP_SCRIPT, json!({}))],
            Self::ScriptSave { name, text } => {
                vec![(TOOL_SCRIPT_SAVE, json!({ "name": name, "text": text }))]
            }
            Self::RecordStart(name) => vec![(
                TOOL_RECORD_MACRO,
                json!({ "action": "start", "name": name }),
            )],
            Self::RecordStop => vec![(TOOL_RECORD_MACRO, json!({ "action": "stop" }))],
            Self::GumpButton {
                gump,
                button,
                switches,
                texts,
            } => {
                let texts: Vec<Value> = texts
                    .iter()
                    .map(|(id, text)| json!({ "id": id, "text": text }))
                    .collect();
                vec![(
                    TOOL_GUMP_RESPOND,
                    json!({ "gump": gump, "button": button, "switches": switches, "texts": texts }),
                )]
            }
            Self::GumpClose(gump) => vec![(TOOL_GUMP_CLOSE, json!({ "gump": gump }))],
            Self::AgentOn { agent, on } => {
                vec![(TOOL_AGENT_ON, json!({ "agent": agent, "on": on }))]
            }
            Self::AgentSet {
                agent,
                list,
                settings,
            } => vec![(
                TOOL_AGENT_SET,
                json!({ "agent": agent, "list": list, "settings": settings }),
            )],
            Self::AgentRun { agent, list } => {
                vec![(TOOL_AGENT_RUN, json!({ "agent": agent, "list": list }))]
            }
            Self::AgentStop => vec![(TOOL_AGENT_STOP, json!({}))],
            Self::DamageMeter(action) => {
                vec![(TOOL_DAMAGE_METER, json!({ "action": action }))]
            }
            Self::PartyInvite(s) => {
                vec![(TOOL_PARTY, json!({ "action": "invite", "serial": s }))]
            }
            Self::PartyLeave => vec![(TOOL_PARTY, json!({ "action": "leave" }))],
            Self::PartyKick(s) => vec![(TOOL_PARTY, json!({ "action": "kick", "serial": s }))],
            Self::PartyLoot(on) => vec![(TOOL_PARTY, json!({ "action": "loot", "on": on }))],
            Self::MobileStatus { serial, close } => vec![(
                TOOL_MOBILE_STATUS,
                json!({ "serial": serial, "close": close }),
            )],
            Self::VirtueGump(s) => vec![(TOOL_VIRTUE_GUMP, serial(s))],
            Self::Order(..) => Vec::new(),
        }
    }

    /// The act in words, for the line that tells the human what was done.
    pub fn words(&self) -> String {
        match self {
            Self::Take => "You have the character.".into(),
            Self::GiveBack => "The agent has the character again.".into(),
            Self::WalkTo { x, y } => format!("Walk to {x}, {y}."),
            Self::RunTo { x, y } => format!("Run to {x}, {y}."),
            Self::Use(_) => "Use.".into(),
            Self::Look(_) => "Look.".into(),
            Self::Attack(_) => "Attack.".into(),
            Self::Follow(_) => "Follow.".into(),
            Self::Loot(_) => "Loot.".into(),
            Self::Target(_) | Self::TargetGround { .. } => "Target.".into(),
            Self::CancelTarget => "Target canceled.".into(),
            Self::War(true) => "War mode.".into(),
            Self::War(false) => "Peace mode.".into(),
            Self::Say { text, .. } | Self::Speak { text, .. } => format!("Said: {text}"),
            Self::Hotkey(name) => format!("Hotkey: {name}"),
            Self::Stop => "Stop.".into(),
            Self::Deposit => "Put the pack in the bank.".into(),
            // The window asks these by itself, so they say nothing.
            Self::HouseContent(_) | Self::GameView { .. } => String::new(),
            // A step comes many times each second, so it says nothing.
            Self::Step { .. } => String::new(),
            Self::Move { .. } => "Item moved.".into(),
            Self::Wear(_) | Self::WearLastWeapon => "Put on.".into(),
            Self::TakeOff(_) => "Taken off.".into(),
            Self::Menu(_) | Self::MenuClose | Self::BookClose => String::new(),
            Self::BoardRead(_) | Self::BoardClose | Self::BookRead(_) => String::new(),
            Self::BookName { .. } => "Book named.".into(),
            Self::BookPage { page, .. } => format!("Page {page} written."),
            Self::BoardPost { .. } => "Message posted.".into(),
            Self::BoardRemove(_) => "Message removed.".into(),
            Self::MapPin { .. } => "Pin put on the map.".into(),
            Self::MapPinMove { .. } => "Pin moved.".into(),
            Self::MapPinRemove(_) => "Pin taken off.".into(),
            Self::MapClear => "Pins cleared.".into(),
            Self::MapEdit | Self::MapClose(_) | Self::ProfileRead(_) => String::new(),
            Self::ProfileWrite { .. } => "Profile written.".into(),
            // A designer step comes with each click, so it says nothing.
            Self::HouseEdit { .. } => String::new(),
            Self::HouseFloor(level) => format!("Floor {level}."),
            Self::HouseCommand(action) => format!("House: {action}."),
            Self::Help => "Help asked for.".into(),
            Self::QuestArrow { .. } => "Quest arrow clicked.".into(),
            Self::RaceChange(Some(_)) => "Race changed.".into(),
            Self::RaceChange(None) => "Race change refused.".into(),
            Self::ChatOpen(_) => "Chat opened.".into(),
            Self::ChatJoin(channel) | Self::ChatJoinWithPassword { channel, .. } => {
                format!("Joined {channel}.")
            }
            Self::ChatCreate(channel) => format!("Made {channel}."),
            Self::ChatSay(_) | Self::Tip { .. } => String::new(),
            Self::ChatLeave => "Left the channel.".into(),
            Self::Quit => "Leaving the world.".into(),
            Self::OldMenuPick(Some(_)) => "Menu answered.".into(),
            Self::OldMenuPick(None) => "Menu closed.".into(),
            Self::MenuPick { .. } => "Menu line picked.".into(),
            Self::Checkout(_) => "Deal made.".into(),
            Self::ShopClose => "Shop closed.".into(),
            Self::TradeWith(_) => "Trade offered.".into(),
            Self::TradeAccept { accept: true, .. } => "Trade accepted.".into(),
            Self::TradeAccept { accept: false, .. } => "Trade not accepted.".into(),
            Self::TradeCancel(_) => "Trade canceled.".into(),
            Self::TradeGold { .. } => "Gold offered.".into(),
            Self::UseSkill(_) => "Skill used.".into(),
            Self::Cast(_) | Self::CastFrom { .. } => "Spell cast.".into(),
            Self::Dye(_) => "Color picked.".into(),
            Self::OpenSpellbook(_) => "Spellbook opened.".into(),
            Self::Command(text) => format!("Command: {text}"),
            Self::ScriptRun { .. } => "Macro started.".into(),
            Self::ScriptStop => "Macro stopped.".into(),
            Self::ScriptSave { name, .. } => format!("Macro saved: {name}"),
            Self::RecordStart(name) => {
                format!("Recording: {name}. Play, then press Stop recording.")
            }
            Self::RecordStop => "Recording saved.".into(),
            Self::GumpButton { .. } => "Gump answered.".into(),
            Self::GumpClose(_) => "Gump closed.".into(),
            Self::AgentOn { agent, on: true } => format!("{agent} is on."),
            Self::AgentOn { agent, on: false } => format!("{agent} is off."),
            Self::AgentSet { agent, .. } => format!("{agent} settings kept."),
            Self::AgentRun { agent, .. } => format!("{agent} runs."),
            Self::AgentStop => "Agent job stopped.".into(),
            Self::DamageMeter(action) => format!("Damage meter: {action}."),
            Self::PartyInvite(_) => "Party invite sent.".into(),
            Self::PartyLeave => "Left the party.".into(),
            Self::PartyKick(_) => "Removed from the party.".into(),
            Self::PartyLoot(true) => "The party may loot your kills.".into(),
            Self::PartyLoot(false) => "The party may not loot your kills.".into(),
            Self::MobileStatus { close: false, .. } => "Status asked for.".into(),
            Self::MobileStatus { close: true, .. } => "Status bar closed.".into(),
            Self::VirtueGump(_) => "Virtue gump asked for.".into(),
            Self::Order(order, _) => format!("Order: {order}"),
        }
    }
    /// True when the act is a lift then a drop. The caller waits
    /// `LIFT_TO_DROP` between the two calls.
    pub fn is_two_step(&self) -> bool {
        matches!(self.calls().as_slice(), [(TOOL_LIFT, _), (TOOL_DROP, _)])
    }
}

/// The arguments of a call, marked as the human's.
pub fn with_human(mut args: Value) -> Value {
    args[ARG_HUMAN] = json!(true);
    args
}

/// The answers to the asks of `wanted`, in the order they came, and the
/// answers that stay kept for the other askers.
pub fn split_answers(
    items: Vec<(Asker, Answer)>,
    wanted: Asker,
) -> (Vec<Answer>, Vec<(Asker, Answer)>) {
    let (own, others): (Vec<_>, Vec<_>) = items.into_iter().partition(|(by, _)| *by == wanted);
    (own.into_iter().map(|(_, answer)| answer).collect(), others)
}

/// The named places that lie between `from` and `to` of a map, with their
/// tiles. The marker file of the operator names them.
pub fn places_in(
    landmarks: &Value,
    map: u8,
    from: (u16, u16),
    to: (u16, u16),
) -> Vec<(String, u16, u16)> {
    let inside = |x: u16, y: u16| x >= from.0 && x <= to.0 && y >= from.1 && y <= to.1;
    landmarks
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter(|place| place.get("map").and_then(Value::as_u64) == Some(u64::from(map)))
        .filter_map(|place| {
            let at = place.get("location")?;
            let number = |key: &str| u16::try_from(at.get(key)?.as_u64()?).ok();
            let (x, y) = (number("x")?, number("y")?);
            let name = place.get("name")?.as_str()?.to_string();
            inside(x, y).then_some((name, x, y))
        })
        .collect()
}

/// The strings of the array `key` of an answer.
pub fn string_list(answer: Option<&Value>, key: &str) -> Vec<String> {
    answer
        .and_then(|value| value.get(key))
        .and_then(Value::as_array)
        .map(|lines| {
            lines
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// The state of a script in words: its status, and the fault with its line.
pub fn status_words(status: Option<&Value>) -> String {
    let Some(status) = status else {
        return String::new();
    };
    let word = status.get("status").and_then(Value::as_str).unwrap_or("");
    match (
        status.get("line").and_then(Value::as_u64),
        status.get("error").and_then(Value::as_str),
    ) {
        (Some(line), Some(error)) => format!("{word}: line {line}: {error}"),
        _ => word.to_string(),
    }
}

/// The lines of the tooltip in the answer of the properties tool.
pub fn tip_lines(answer: Option<&Value>) -> Vec<String> {
    string_list(answer, "lines")
}

#[cfg(test)]
mod moved_tests {
    use super::*;
    use uoterm_world::tool_names::{ARG_HUMAN, TOOL_DROP, TOOL_LIFT, TOOL_USE};

    const ITEM: u32 = 0x4000_0001;
    const BAG: u32 = 0x4000_0002;

    #[test]
    fn a_move_lifts_then_drops() {
        let act = Act::Move {
            item: ITEM,
            amount: 5,
            to: DropTo::Ground { x: 10, y: 20, z: 0 },
        };
        let names: Vec<&str> = act.calls().iter().map(|(name, _)| *name).collect();
        assert_eq!(names, vec![TOOL_LIFT, TOOL_DROP]);
        assert!(act.is_two_step());
    }

    #[test]
    fn a_use_is_one_call() {
        let act = Act::Use(BAG);
        assert_eq!(act.calls().len(), 1);
        assert_eq!(act.calls()[0].0, TOOL_USE);
        assert!(!act.is_two_step());
    }

    #[test]
    fn a_human_call_is_marked() {
        assert_eq!(
            with_human(json!({"serial": 1})),
            json!({"serial": 1, ARG_HUMAN: true})
        );
    }

    #[test]
    fn each_asker_takes_only_its_own_answers() {
        let came = vec![
            (Asker::Chat, Answer::Picked(Ok(2))),
            (Asker::Deck, Answer::Picked(Ok(5))),
            (Asker::Chat, Answer::Picked(Err("no".into()))),
        ];
        let (own, kept) = split_answers(came, Asker::Chat);
        assert_eq!(
            own,
            vec![Answer::Picked(Ok(2)), Answer::Picked(Err("no".into()))]
        );
        assert_eq!(kept, vec![(Asker::Deck, Answer::Picked(Ok(5)))]);
        let (own, kept) = split_answers(kept, Asker::Designer);
        assert!(own.is_empty());
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn only_the_places_that_lie_on_the_map_are_asked_about() {
        let landmarks = json!([
            { "name": "Britain bank", "map": 0, "location": { "x": 1400, "y": 1500 } },
            { "name": "Yew moongate", "map": 0, "location": { "x": 9000, "y": 9000 } },
            { "name": "Luna bank", "map": 1, "location": { "x": 1400, "y": 1500 } },
            { "name": "no place", "map": 0 },
        ]);
        let places = places_in(&landmarks, 0, (1000, 1200), (1600, 1600));
        assert_eq!(places, vec![("Britain bank".to_string(), 1400, 1500)]);
        assert!(places_in(&landmarks, 4, (0, 0), (u16::MAX, u16::MAX)).is_empty());
    }

    #[test]
    fn a_failed_script_tells_its_line_and_its_fault() {
        let failed = json!({ "status": "failed", "line": 3, "error": "no such command" });
        assert_eq!(
            status_words(Some(&failed)),
            "failed: line 3: no such command"
        );
        assert_eq!(
            status_words(Some(&json!({ "status": "running" }))),
            "running"
        );
        assert_eq!(status_words(None), "");
    }

    #[test]
    fn a_tooltip_is_the_lines_of_the_answer() {
        let answer = json!({ "serial": ITEM, "lines": ["a katana", "Durability 40 / 40"] });
        assert_eq!(tip_lines(Some(&answer)).len(), 2);
        assert!(tip_lines(None).is_empty());
    }

    #[test]
    fn words_keep_their_apostrophes_in_a_script_line() {
        assert_eq!(quoted("hail"), "'hail'");
        assert_eq!(quoted("Bob's shop"), "\"Bob's shop\"");
        assert_eq!(quoted("say \"hi\" Bob's"), "\"say hi Bob's\"");
    }

    #[test]
    fn each_channel_speaks_with_the_tool_that_knows_it() {
        const HUE: u16 = 0x0022;
        let speak = |channel| {
            Act::Speak {
                channel,
                text: "hi".into(),
                hue: HUE,
            }
            .calls()
        };
        assert_eq!(
            speak(Channel::Guild),
            vec![(
                TOOL_SAY,
                json!({ "text": "hi", "channel": "guild", "hue": HUE })
            )]
        );
        assert_eq!(
            speak(Channel::Whisper),
            vec![(TOOL_WHISPER, json!({ "text": "hi", "hue": HUE }))]
        );
        assert_eq!(
            speak(Channel::Emote),
            vec![(TOOL_COMMAND, json!({ "text": "emotemsg '*hi*' 0x0022" }))]
        );
        assert_eq!(
            Act::Say {
                text: "hail".into(),
                hue: HUE
            }
            .calls(),
            vec![(TOOL_SAY, json!({ "text": "hail", "hue": HUE }))]
        );
        assert_eq!(
            speak(Channel::PartyMember(0x1234)),
            vec![(
                TOOL_COMMAND,
                json!({ "text": "partymsg 'hi' 0 0x00001234" })
            )]
        );
    }

    #[test]
    fn an_act_is_the_tool_calls_the_session_knows() {
        assert_eq!(
            Act::WalkTo { x: 10, y: 20 }.calls(),
            vec![(TOOL_MOVE_TO, json!({ "x": 10, "y": 20 }))]
        );
        assert_eq!(
            Act::RunTo { x: 10, y: 20 }.calls(),
            vec![(TOOL_MOVE_TO, json!({ "x": 10, "y": 20, "run": true }))]
        );
        assert_eq!(Act::CancelTarget.calls(), vec![(TOOL_TARGET, json!({}))]);
        let to_ground = Act::Move {
            item: ITEM,
            amount: 5,
            to: DropTo::Ground { x: 44, y: 65, z: 7 },
        };
        assert_eq!(
            to_ground.calls(),
            vec![
                (TOOL_LIFT, json!({ "serial": ITEM, "amount": 5 })),
                (
                    TOOL_DROP,
                    json!({ "serial": ITEM, "x": 44, "y": 65, "z": 7 })
                ),
            ]
        );
        let into_spot = Act::Move {
            item: ITEM,
            amount: 1,
            to: DropTo::IntoAt {
                container: BAG,
                x: 60,
                y: 70,
            },
        };
        assert_eq!(
            into_spot.calls()[1],
            (
                TOOL_DROP,
                json!({ "serial": ITEM, "dest": BAG, "x": 60, "y": 70 })
            )
        );
        assert_eq!(
            Act::CastFrom {
                spell: 5,
                book: BAG
            }
            .calls(),
            vec![(TOOL_CAST, json!({ "spell": 5, "book": BAG }))]
        );
    }
}
