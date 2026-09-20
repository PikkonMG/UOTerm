//! The hand of the human on the character. The window turns a click or a
//! line of text into one act, and a worker thread makes the tool call, so
//! the window never waits for the session.
//!
//! Each call says `human: true`. The session lets those through while it
//! refuses the agent.

use super::link::Link;
use super::orders;
use crate::view::WatchFrame;
use eframe::egui;
use serde_json::{json, Value};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use uoterm_runtime::tools::{
    ARG_HUMAN, TOOL_ATTACK, TOOL_BOARD_CLOSE, TOOL_BOARD_POST, TOOL_BOARD_READ, TOOL_BOARD_REMOVE,
    TOOL_BOOK_CLOSE, TOOL_CAST, TOOL_CHAT, TOOL_CLOSE_MENU, TOOL_COMMAND, TOOL_CONTEXT_MENU,
    TOOL_DEPOSIT, TOOL_DROP, TOOL_EQUIP, TOOL_FIND_LANDMARKS, TOOL_FOLLOW, TOOL_GUMP_CLOSE,
    TOOL_GUMP_RESPOND, TOOL_HELP, TOOL_HOTKEYS, TOOL_HOUSE_EDIT, TOOL_LIFT, TOOL_LIST_SCRIPTS,
    TOOL_LOOT, TOOL_MAP_CLOSE, TOOL_MAP_PIN, TOOL_MENU_PICK, TOOL_MOVE_TO, TOOL_PROFILE,
    TOOL_PROPERTIES, TOOL_RECORD_MACRO, TOOL_RELEASE_CONTROL, TOOL_RUN_SCRIPT, TOOL_SAY,
    TOOL_SCRIPT_READ, TOOL_SCRIPT_SAVE, TOOL_SCRIPT_STATUS, TOOL_SHOP_CHECKOUT, TOOL_SHOP_CLOSE,
    TOOL_SINGLE_CLICK, TOOL_STOP, TOOL_STOP_SCRIPT, TOOL_TAKE_CONTROL, TOOL_TARGET,
    TOOL_TRADE_ACCEPT, TOOL_TRADE_CANCEL, TOOL_TRADE_GOLD, TOOL_TRADE_OFFER, TOOL_UNEQUIP,
    TOOL_USE, TOOL_USE_SKILL, TOOL_WALK, TOOL_WAR_MODE,
};

/// The shard refuses a drop that comes too soon after the lift.
const LIFT_TO_DROP: Duration = Duration::from_millis(650);
const NOT_SURE: &str = "Jev is not sure which one you mean. Pick it from the list.";
const NO_PLACE_ON_MAP: &str = "The marker file names no place on this map.";
const NO_SUCH_PLACE: &str = "Jev is not sure which place you mean. Click the map instead.";
const ORDER_OFF: &str = "Orders need a TypeSafe key. Put TYPESAFE_API_KEY in the environment.";

/// How long one sent step keeps the character on his way. The window sends
/// the next one before this ends, so a held key is one smooth walk.
const STEP_HOLD_MS: u64 = 600;

/// A lift of this many takes the whole pile: the shard cuts it to the pile.
pub const WHOLE_PILE: u16 = u16::MAX;

/// One thing the human tells the character to do.
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Take,
    GiveBack,
    WalkTo {
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
    Say(String),
    Stop,
    Deposit,
    /// One step while a key or the right mouse button is down.
    Step {
        direction: &'static str,
        run: bool,
    },
    /// Lift an item, or a part of a pile, and drop it at a place.
    Move {
        item: u32,
        amount: u16,
        to: DropTo,
    },
    Wear(u32),
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
    ChatOpen(String),
    ChatJoin(String),
    ChatSay(String),
    ChatLeave,
    /// Buy or sell the rows of the cart: the item and how many.
    Checkout(Vec<(u32, u16)>),
    ShopClose,
    TradeWith(u32),
    TradeAccept,
    TradeCancel,
    TradeGold {
        gold: u32,
        platinum: u32,
    },
    UseSkill(u16),
    Cast(u16),
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
    /// Words for Jev to turn into one of the acts above.
    Order(String, Box<WatchFrame>),
}

/// Where a moved item lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropTo {
    /// Into a container or onto a mobile. The shard picks the spot.
    Into(u32),
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

/// A thing the macro editor asks the session, or Jev, for.
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
    fn calls(&self) -> Vec<(&'static str, Value)> {
        let serial = |serial: &u32| json!({ "serial": serial });
        match self {
            Self::Take => vec![(TOOL_TAKE_CONTROL, json!({}))],
            Self::GiveBack => vec![(TOOL_RELEASE_CONTROL, json!({}))],
            Self::WalkTo { x, y } => vec![(TOOL_MOVE_TO, json!({ "x": x, "y": y }))],
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
            Self::Say(text) => vec![(TOOL_SAY, json!({ "text": text }))],
            Self::Stop => vec![(TOOL_STOP, json!({}))],
            Self::Deposit => vec![(TOOL_DEPOSIT, json!({}))],
            Self::Step { direction, run } => {
                vec![(
                    TOOL_WALK,
                    json!({ "direction": direction, "running": run, "hold_ms": STEP_HOLD_MS }),
                )]
            }
            Self::Move { item, amount, to } => {
                let mut drop = json!({ "serial": item });
                match *to {
                    DropTo::Into(dest) => drop["dest"] = json!(dest),
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
            Self::ChatOpen(name) => {
                vec![(TOOL_CHAT, json!({ "action": "open", "name": name }))]
            }
            Self::ChatJoin(channel) => {
                vec![(TOOL_CHAT, json!({ "action": "join", "channel": channel }))]
            }
            Self::ChatSay(text) => vec![(TOOL_CHAT, json!({ "action": "say", "text": text }))],
            Self::ChatLeave => vec![(TOOL_CHAT, json!({ "action": "leave" }))],
            Self::Checkout(rows) => {
                let items: Vec<Value> = rows
                    .iter()
                    .map(|(serial, amount)| json!({ "serial": serial, "amount": amount }))
                    .collect();
                vec![(TOOL_SHOP_CHECKOUT, json!({ "items": items }))]
            }
            Self::ShopClose => vec![(TOOL_SHOP_CLOSE, json!({}))],
            Self::TradeWith(s) => vec![(TOOL_TRADE_OFFER, serial(s))],
            Self::TradeAccept => vec![(TOOL_TRADE_ACCEPT, json!({}))],
            Self::TradeCancel => vec![(TOOL_TRADE_CANCEL, json!({}))],
            Self::TradeGold { gold, platinum } => vec![(
                TOOL_TRADE_GOLD,
                json!({ "gold": gold, "platinum": platinum }),
            )],
            Self::UseSkill(skill) => vec![(TOOL_USE_SKILL, json!({ "skill": skill }))],
            Self::Cast(spell) => vec![(TOOL_CAST, json!({ "spell": spell }))],
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
            Self::Order(..) => Vec::new(),
        }
    }

    /// The act in words, for the line that tells the human what was done.
    fn words(&self) -> String {
        match self {
            Self::Take => "You have the character.".into(),
            Self::GiveBack => "The agent has the character again.".into(),
            Self::WalkTo { x, y } => format!("Walk to {x}, {y}."),
            Self::Use(_) => "Use.".into(),
            Self::Look(_) => "Look.".into(),
            Self::Attack(_) => "Attack.".into(),
            Self::Follow(_) => "Follow.".into(),
            Self::Loot(_) => "Loot.".into(),
            Self::Target(_) | Self::TargetGround { .. } => "Target.".into(),
            Self::CancelTarget => "Target canceled.".into(),
            Self::War(true) => "War mode.".into(),
            Self::War(false) => "Peace mode.".into(),
            Self::Say(text) => format!("Said: {text}"),
            Self::Stop => "Stop.".into(),
            Self::Deposit => "Put the pack in the bank.".into(),
            // A step comes many times each second, so it says nothing.
            Self::Step { .. } => String::new(),
            Self::Move { .. } => "Item moved.".into(),
            Self::Wear(_) => "Put on.".into(),
            Self::TakeOff(_) => "Taken off.".into(),
            Self::Menu(_) | Self::MenuClose | Self::BookClose => String::new(),
            Self::BoardRead(_) | Self::BoardClose => String::new(),
            Self::BoardPost { .. } => "Message posted.".into(),
            Self::BoardRemove(_) => "Message removed.".into(),
            Self::MapPin { .. } => "Pin put on the map.".into(),
            Self::MapClear => "Pins cleared.".into(),
            Self::MapEdit | Self::MapClose(_) | Self::ProfileRead(_) => String::new(),
            Self::ProfileWrite { .. } => "Profile written.".into(),
            // A designer step comes with each click, so it says nothing.
            Self::HouseEdit { .. } => String::new(),
            Self::HouseFloor(level) => format!("Floor {level}."),
            Self::HouseCommand(action) => format!("House: {action}."),
            Self::Help => "Help asked for.".into(),
            Self::ChatOpen(_) => "Chat opened.".into(),
            Self::ChatJoin(channel) => format!("Joined {channel}."),
            Self::ChatSay(_) => String::new(),
            Self::ChatLeave => "Left the channel.".into(),
            Self::OldMenuPick(Some(_)) => "Menu answered.".into(),
            Self::OldMenuPick(None) => "Menu closed.".into(),
            Self::MenuPick { .. } => "Menu line picked.".into(),
            Self::Checkout(_) => "Deal made.".into(),
            Self::ShopClose => "Shop closed.".into(),
            Self::TradeWith(_) => "Trade offered.".into(),
            Self::TradeAccept => "Trade accepted.".into(),
            Self::TradeCancel => "Trade canceled.".into(),
            Self::TradeGold { .. } => "Gold offered.".into(),
            Self::UseSkill(_) => "Skill used.".into(),
            Self::Cast(_) => "Spell cast.".into(),
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
            Self::Order(order, _) => format!("Order: {order}"),
        }
    }
}

/// The sending end the window holds, and the reports that come back.
pub struct Hand {
    acts: Sender<Act>,
    reports: Receiver<Report>,
    wanted_tips: Sender<u32>,
    tips: Receiver<Tip>,
    asks: Sender<Ask>,
    answers: Receiver<Answer>,
    pub orders_on: bool,
}

impl Hand {
    pub fn start(link: Link, ctx: egui::Context) -> Self {
        let (acts, inbox) = mpsc::channel();
        let (outbox, reports) = mpsc::channel();
        let key = orders::api_key();
        let orders_on = key.is_some();
        let (wanted_tips, tip_inbox) = mpsc::channel();
        let (tip_outbox, tips) = mpsc::channel();
        let (tip_link, tip_ctx) = (link.clone(), ctx.clone());
        let (tip_link_for_asks, ctx_for_asks, key_for_asks) =
            (link.clone(), ctx.clone(), key.clone());
        thread::spawn(move || work(&link, key.as_deref(), &inbox, &outbox, &ctx));
        thread::spawn(move || read_tips(&tip_link, &tip_inbox, &tip_outbox, &tip_ctx));
        let (asks, ask_inbox) = mpsc::channel();
        let (answer_outbox, answers) = mpsc::channel();
        let (ask_link, ask_ctx, ask_key) = (tip_link_for_asks, ctx_for_asks, key_for_asks);
        thread::spawn(move || {
            answer_asks(
                &ask_link,
                ask_key.as_deref(),
                &ask_inbox,
                &answer_outbox,
                &ask_ctx,
            );
        });
        Self {
            acts,
            reports,
            wanted_tips,
            tips,
            asks,
            answers,
            orders_on,
        }
    }

    /// Asks for the tooltip of a thing. It reads, so it needs no control and
    /// does not wait behind the acts.
    pub fn want_tip(&self, serial: u32) {
        let _ = self.wanted_tips.send(serial);
    }

    /// Asks for something that comes back as an answer. It does not wait
    /// behind the acts.
    pub fn ask(&self, ask: Ask) {
        let _ = self.asks.send(ask);
    }

    pub fn new_answers(&self) -> Vec<Answer> {
        self.answers.try_iter().collect()
    }

    pub fn new_tips(&self) -> Vec<Tip> {
        self.tips.try_iter().collect()
    }

    pub fn act(&self, act: Act) {
        // The worker lives as long as the window, so a send cannot fail.
        let _ = self.acts.send(act);
    }

    /// The newest report with words, when one came since the last frame.
    pub fn newest_report(&self) -> Option<Report> {
        self.reports
            .try_iter()
            .filter(|report| !report.text.is_empty())
            .last()
    }
}

fn work(
    link: &Link,
    key: Option<&str>,
    inbox: &Receiver<Act>,
    outbox: &Sender<Report>,
    ctx: &egui::Context,
) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    for act in inbox {
        let report = rt.block_on(perform(link, key, act));
        if outbox.send(report).is_err() {
            return;
        }
        ctx.request_repaint();
    }
}

/// How many times the worker asks for one tooltip. The first answer of the
/// session is empty when it must ask the shard.
const TIP_TRIES: usize = 4;
const TIP_RETRY: Duration = Duration::from_millis(250);

fn read_tips(link: &Link, inbox: &Receiver<u32>, outbox: &Sender<Tip>, ctx: &egui::Context) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    for serial in inbox {
        let lines = rt.block_on(async {
            for _ in 0..TIP_TRIES {
                let answer = link
                    .call(TOOL_PROPERTIES, json!({ "serial": serial }))
                    .await;
                let lines = tip_lines(answer.as_ref().ok());
                if !lines.is_empty() {
                    return lines;
                }
                tokio::time::sleep(TIP_RETRY).await;
            }
            Vec::new()
        });
        if outbox.send(Tip { serial, lines }).is_err() {
            return;
        }
        ctx.request_repaint();
    }
}

fn answer_asks(
    link: &Link,
    key: Option<&str>,
    inbox: &Receiver<Ask>,
    outbox: &Sender<Answer>,
    ctx: &egui::Context,
) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    for ask in inbox {
        let answer = rt.block_on(answer_one(link, key, ask));
        if outbox.send(answer).is_err() {
            return;
        }
        ctx.request_repaint();
    }
}

async fn answer_one(link: &Link, key: Option<&str>, ask: Ask) -> Answer {
    match ask {
        Ask::Scripts => {
            let listed = link.call(TOOL_LIST_SCRIPTS, json!({})).await.ok();
            Answer::Scripts(string_list(listed.as_ref(), "scripts"))
        }
        Ask::ScriptText(name) => {
            let read = link.call(TOOL_SCRIPT_READ, json!({ "name": name })).await;
            let text = read
                .ok()
                .and_then(|v| v.get("text").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_default();
            Answer::ScriptText { name, text }
        }
        Ask::ScriptStatus => {
            let status = link.call(TOOL_SCRIPT_STATUS, json!({})).await.ok();
            Answer::ScriptStatus(status_words(status.as_ref()))
        }
        Ask::HousePart { wish, options } => {
            Answer::Picked(pick_one(key, orders::ASK_HOUSE_PART, &wish, &options).await)
        }
        Ask::Channel { wish, options } => {
            Answer::Picked(pick_one(key, orders::ASK_CHANNEL, &wish, &options).await)
        }
        Ask::PlaceOnMap {
            wish,
            map,
            from,
            to,
        } => Answer::Place(match key {
            None => Err(ORDER_OFF.into()),
            Some(key) => place_on_map(link, key, &wish, map, from, to).await,
        }),
        Ask::LinesFor(wish) => Answer::Lines(match key {
            None => Err(ORDER_OFF.into()),
            Some(key) => {
                let hotkeys = |name: Option<String>| async move {
                    let args = name.map_or_else(|| json!({}), |name| json!({ "name": name }));
                    link.call(TOOL_HOTKEYS, args).await
                };
                orders::lines_for(key, &wish, hotkeys).await
            }
        }),
    }
}

/// Asks Jev which of `options` the wish names, and gives its place.
async fn pick_one(
    key: Option<&str>,
    ask: &str,
    wish: &str,
    options: &[String],
) -> Result<usize, String> {
    let key = key.ok_or(ORDER_OFF)?;
    let names: Vec<&str> = options.iter().map(String::as_str).collect();
    orders::pick(key, ask, wish, &names)
        .await?
        .ok_or_else(|| NOT_SURE.to_string())
}

/// The named places that lie between `from` and `to` of a map, with their
/// tiles. The marker file of the operator names them.
fn places_in(
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

/// The tile of the place the words name. Jev picks it from the places that
/// lie on the map.
async fn place_on_map(
    link: &Link,
    key: &str,
    wish: &str,
    map: u8,
    from: (u16, u16),
    to: (u16, u16),
) -> Result<(u16, u16), String> {
    let landmarks = link
        .call(TOOL_FIND_LANDMARKS, json!({ "map": map }))
        .await?;
    let places = places_in(&landmarks, map, from, to);
    if places.is_empty() {
        return Err(NO_PLACE_ON_MAP.into());
    }
    let names: Vec<&str> = places.iter().map(|(name, ..)| name.as_str()).collect();
    let place = orders::pick(key, orders::ASK_LANDMARK, wish, &names)
        .await?
        .ok_or(NO_SUCH_PLACE)?;
    let (_, x, y) = &places[place];
    Ok((*x, *y))
}

fn string_list(answer: Option<&Value>, key: &str) -> Vec<String> {
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
fn status_words(status: Option<&Value>) -> String {
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

fn tip_lines(answer: Option<&Value>) -> Vec<String> {
    string_list(answer, "lines")
}

async fn perform(link: &Link, key: Option<&str>, act: Act) -> Report {
    let failed = |text: String| Report { text, failed: true };
    let act = match act {
        Act::Order(order, frame) => {
            let Some(key) = key else {
                return failed(ORDER_OFF.into());
            };
            match orders::ask(key, &order, &frame).await {
                Ok(understood) => understood,
                Err(words) => return failed(words),
            }
        }
        plain => plain,
    };
    for (i, (tool, mut args)) in act.calls().into_iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(LIFT_TO_DROP).await;
        }
        args[ARG_HUMAN] = json!(true);
        if let Err(words) = link.call(tool, args).await {
            return failed(words);
        }
    }
    Report {
        text: act.words(),
        failed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM: u32 = 0x4000_0001;
    const BAG: u32 = 0x4000_0002;

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
    fn an_act_is_the_tool_calls_the_session_knows() {
        assert_eq!(
            Act::WalkTo { x: 10, y: 20 }.calls(),
            vec![(TOOL_MOVE_TO, json!({ "x": 10, "y": 20 }))]
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
    }

    #[test]
    fn each_tool_name_is_a_tool_of_the_session() {
        let listed = uoterm_runtime::tools::mcp_tool_list();
        let known: Vec<&str> = listed["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        let acts = [
            Act::Take,
            Act::GiveBack,
            Act::WalkTo { x: 0, y: 0 },
            Act::Use(ITEM),
            Act::Look(ITEM),
            Act::Attack(ITEM),
            Act::Follow(ITEM),
            Act::Loot(ITEM),
            Act::Target(ITEM),
            Act::War(true),
            Act::Say("hail".into()),
            Act::Stop,
            Act::Deposit,
            Act::Step {
                direction: "n",
                run: false,
            },
            Act::Move {
                item: ITEM,
                amount: 1,
                to: DropTo::Into(BAG),
            },
            Act::Wear(ITEM),
            Act::TakeOff(1),
            Act::Menu(ITEM),
            Act::MenuPick {
                serial: ITEM,
                index: 0,
            },
            Act::MenuClose,
            Act::OldMenuPick(Some(1)),
            Act::OldMenuPick(None),
            Act::BookClose,
            Act::BoardRead(ITEM),
            Act::BoardPost {
                subject: "Hi".into(),
                text: "one".into(),
                reply_to: None,
            },
            Act::BoardRemove(ITEM),
            Act::BoardClose,
            Act::MapPin { x: 1, y: 2 },
            Act::MapClear,
            Act::MapEdit,
            Act::MapClose(ITEM),
            Act::ProfileRead(ITEM),
            Act::ProfileWrite {
                serial: ITEM,
                text: "hi".into(),
            },
            Act::HouseEdit {
                action: "add",
                graphic: 10,
                x: 1,
                y: 2,
                z: 0,
            },
            Act::HouseFloor(2),
            Act::HouseCommand("commit"),
            Act::Help,
            Act::ChatOpen("Mara".into()),
            Act::ChatJoin("General".into()),
            Act::ChatSay("hail".into()),
            Act::ChatLeave,
            Act::Checkout(vec![(ITEM, 1)]),
            Act::ShopClose,
            Act::TradeWith(ITEM),
            Act::TradeAccept,
            Act::TradeCancel,
            Act::TradeGold {
                gold: 1,
                platinum: 0,
            },
            Act::UseSkill(1),
            Act::Cast(1),
            Act::Command("promptmsg 'hi'".into()),
            Act::ScriptRun {
                text: "msg 'hi'".into(),
                looping: false,
            },
            Act::ScriptStop,
            Act::ScriptSave {
                name: "hi".into(),
                text: "msg 'hi'".into(),
            },
            Act::RecordStart("hi".into()),
            Act::RecordStop,
            Act::GumpButton {
                gump: 1,
                button: 1,
                switches: Vec::new(),
                texts: Vec::new(),
            },
            Act::GumpClose(1),
        ];
        for act in acts {
            for (tool, _) in act.calls() {
                assert!(known.contains(&tool), "{tool} is not a session tool");
            }
        }
    }
}
