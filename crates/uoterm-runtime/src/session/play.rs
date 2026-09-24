//! What a human at the watch window needs from the session, and an agent
//! does not: the whole screen in one picture, the lines of a context menu,
//! the goods of a shopkeeper, the words of a tooltip.
//!
//! An agent reads `observe`, which is cut short on purpose. A window draws
//! everything in view, so it reads `watch`.

use super::*;
use uoterm_protocol::{
    BulletinEvent, ChatEvent, ContextMenuEntry, DisplayMap, HouseEdit, Inbound, MapChange,
    MenuEntry, VendorBuyEntry, VendorSellEntry,
};

/// How many journal lines a window gets. It shows the last few and draws the
/// newest over the heads of the speakers.
const WATCH_JOURNAL_LINES: usize = 60;
const ARG_INDEX: &str = "index";
const ARG_ITEMS: &str = "items";
const ARG_GOLD: &str = "gold";
const ARG_PLATINUM: &str = "platinum";
const NO_MENU_SHOWN: &str = "no context menu is shown for this object; ask for it first";
const NO_CONTEXT_MENUS: &str = "this shard has no context menus; say the words instead";
const NO_SHOP_OPEN: &str = "no shop list is open";
const NO_TRADE_OPEN: &str = "no trade is open";
const NO_MENU_OPEN: &str = "no menu is open";
const NO_DESIGNER: &str = "the house designer is not open; use the house sign and start it";
const NEEDS_GRAPHIC: &str = "needs graphic: one of the parts in watch house_parts";
const NEEDS_ACTION: &str = "needs action";
const BAD_ACTION: &str = "action must be add, remove, stair, roof, remove_roof, floor, clear, revert, commit, exit, backup or restore";
const BAD_CHAT_ACTION: &str = "action must be open, join, say or leave";
const NEEDS_CHANNEL: &str = "needs channel";
const NEEDS_WORDS: &str = "needs text";
const NO_BOOK_OPEN: &str = "no book is open; use a book first";
const NEEDS_PAGE: &str = "needs page, from 1";
const ARG_PAGE: &str = "page";
const ARG_TITLE: &str = "title";
const ARG_AUTHOR: &str = "author";
const ARG_GRAPHIC: &str = "graphic";
const ARG_Z: &str = "z";
const ARG_LEVEL: &str = "level";
const ARG_NAME: &str = "name";
const ARG_CHANNEL: &str = "channel";
const ARG_PASSWORD: &str = "password";
/// How many chat lines the session keeps for a window.
const CHAT_LINES_KEPT: usize = 80;
const NO_MAP_OPEN: &str = "no map item is open; use a map first";
const NEEDS_PLACE: &str = "needs x and y, in pixels of the map picture";
const ARG_ACTION: &str = "action";
const ARG_X: &str = "x";
const ARG_Y: &str = "y";
const ACTION_CLEAR: &str = "clear";
const ACTION_EDIT: &str = "edit";
/// How many profiles the session keeps. A window shows one at a time.
const PROFILES_KEPT: usize = 8;
const NO_BOARD_OPEN: &str = "no bulletin board is open; use one first";
const NEEDS_MESSAGE: &str = "needs message: the serial of a message of the board";
const NEEDS_SUBJECT: &str = "needs subject";
const ARG_MESSAGE: &str = "message";
const ARG_SUBJECT: &str = "subject";
const ARG_TEXT: &str = "text";
const ARG_REPLY_TO: &str = "reply_to";
const NO_SUCH_ENTRY: &str = "the menu has no entry with that index";
const CART_IS_EMPTY: &str = "items is empty: give [{serial, amount}]";

/// A context menu the human asked to see, with its lines in words.
#[derive(Clone, Debug)]
struct ShownMenu {
    serial: Serial,
    lines: Vec<Value>,
}

/// The goods of a shopkeeper: what he sells, or what he buys.
#[derive(Clone, Debug)]
struct Shop {
    vendor: Serial,
    /// True when the character buys from him.
    buying: bool,
    goods: Vec<Value>,
}

/// An old-style menu the shard opened: a question with a list of answers.
#[derive(Clone, Debug)]
struct OldMenu {
    serial: Serial,
    menu_id: u16,
    question: String,
    entries: Vec<MenuEntry>,
}

/// A book the character opened. The pages come after the cover.
#[derive(Clone, Debug)]
struct Book {
    serial: Serial,
    title: String,
    author: String,
    page_count: u16,
    /// The lines of each page that came, by the number of the page.
    pages: std::collections::BTreeMap<u16, Vec<String>>,
}

/// One message of a bulletin board. The lines come when it is read.
#[derive(Clone, Debug, Default)]
struct Post {
    /// The message this one answers.
    parent: Option<Serial>,
    poster: String,
    subject: String,
    time: String,
    lines: Option<Vec<String>>,
}

/// A bulletin board the character opened.
#[derive(Clone, Debug)]
struct Board {
    serial: Serial,
    name: String,
    /// The messages under their serial numbers, so the oldest is first.
    posts: std::collections::BTreeMap<u32, Post>,
    /// The message that was read last.
    reading: Option<Serial>,
}

/// A map item the character opened: a treasure map or a city map.
#[derive(Clone, Debug)]
struct OpenMap {
    what: DisplayMap,
    /// The pins, in pixels of the picture.
    pins: Vec<(u16, u16)>,
    /// The shard lets the player draw on this map.
    may_plot: bool,
}

/// The chat of the shard: its channels, the one the character is in, and
/// the lines that were said.
#[derive(Clone, Debug, Default)]
struct Chat {
    /// The chat is on. Before that the shard may ask for a name.
    open: bool,
    asks_for_name: bool,
    name: String,
    channels: Vec<(String, bool)>,
    in_channel: String,
    lines: Vec<(String, String)>,
}

/// The profile a player wrote about a character.
#[derive(Clone, Debug)]
struct CharacterProfile {
    serial: Serial,
    title: String,
    shard_words: String,
    own_words: String,
}

#[derive(Default)]
pub(super) struct Play {
    /// The object whose context menu the human waits for.
    menu_asked: Option<Serial>,
    menu: Option<ShownMenu>,
    shop: Option<Shop>,
    old_menu: Option<OldMenu>,
    book: Option<Book>,
    board: Option<Board>,
    maps: Vec<OpenMap>,
    profiles: Vec<CharacterProfile>,
    /// The houses players designed, by the item their foundation is.
    houses: std::collections::HashMap<Serial, uoterm_world::DesignedHouse>,
    /// The revision each house design was asked for at, until it comes.
    design_asked: std::collections::HashMap<Serial, u32>,
    /// The building the shard waits for a place for.
    placing: Option<Value>,
    /// The house the designer works on, and the level it works on.
    designing: Option<(Serial, u8)>,
    chat: Chat,
}

impl Play {
    /// The design a player built for a house, once the shard has sent it.
    pub(super) fn designed_house(&self, serial: Serial) -> Option<&uoterm_world::DesignedHouse> {
        self.houses.get(&serial)
    }
}

/// The shard sent a context menu. It is kept when the human asked for it.
pub(super) fn on_context_menu(inner: &mut Inner, serial: Serial, entries: &[ContextMenuEntry]) {
    if inner.play.menu_asked != Some(serial) {
        return;
    }
    inner.play.menu_asked = None;
    let lines = entries
        .iter()
        .map(|entry| {
            let words = inner
                .cliloc
                .as_deref()
                .and_then(|db| db.text(entry.cliloc))
                .map_or_else(|| format!("#{}", entry.cliloc), str::to_string);
            json!({
                "index": entry.index,
                "cliloc": entry.cliloc,
                "words": words,
                "enabled": entry.enabled(),
            })
        })
        .collect();
    inner.play.menu = Some(ShownMenu { serial, lines });
}

/// The shard sent what a shopkeeper sells. The prices come in the order of
/// the goods in his container, and each of those has its place as its `x`.
pub(super) fn on_buy_list(inner: &mut Inner, container: Serial, entries: &[VendorBuyEntry]) {
    let world = inner.world.read();
    let mut stock = world.items_inside(container, false);
    stock.sort_by_key(|item| item.location.x);
    let goods = stock
        .iter()
        .zip(entries)
        .map(|(item, entry)| {
            json!({
                "serial": item.serial,
                "graphic": item.graphic,
                "hue": item.hue,
                "amount": item.amount,
                "price": entry.price,
                "name": entry.description,
            })
        })
        .collect();
    // The container hangs on the shopkeeper.
    let vendor = world
        .items
        .get(&container)
        .and_then(|item| item.parent)
        .unwrap_or(container);
    drop(world);
    inner.play.shop = Some(Shop {
        vendor,
        buying: true,
        goods,
    });
}

pub(super) fn on_sell_list(inner: &mut Inner, vendor: Serial, entries: &[VendorSellEntry]) {
    let goods = entries
        .iter()
        .map(|entry| {
            json!({
                "serial": entry.serial,
                "graphic": entry.graphic,
                "hue": entry.hue,
                "amount": entry.amount,
                "price": entry.price,
                "name": entry.name,
            })
        })
        .collect();
    inner.play.shop = Some(Shop {
        vendor,
        buying: false,
        goods,
    });
}

/// Keeps the book and the old-style menu the shard opened, for `watch`.
pub(super) fn on_book_or_menu(inner: &mut Inner, msg: &Inbound) {
    match msg {
        Inbound::OpenMenu {
            serial,
            menu_id,
            question,
            entries,
        } => {
            inner.play.old_menu = Some(OldMenu {
                serial: *serial,
                menu_id: *menu_id,
                question: question.clone(),
                entries: entries.clone(),
            });
        }
        Inbound::BookHeader {
            serial,
            page_count,
            title,
            author,
            ..
        } => {
            inner.play.book = Some(Book {
                serial: *serial,
                title: title.clone(),
                author: author.clone(),
                page_count: *page_count,
                pages: std::collections::BTreeMap::new(),
            });
        }
        Inbound::Bulletin(BulletinEvent::Opened { board, name }) => {
            inner.play.board = Some(Board {
                serial: *board,
                name: name.clone(),
                posts: std::collections::BTreeMap::new(),
                reading: None,
            });
            // The messages that came before the board opened.
            let known: Vec<Serial> = inner
                .world
                .read()
                .items_inside(*board, false)
                .iter()
                .map(|item| item.serial)
                .collect();
            for message in known {
                ask_for_summary(inner, message);
            }
        }
        Inbound::AddItem(item)
            if inner.play.board.as_ref().map(|b| b.serial) == Some(item.container) =>
        {
            ask_for_summary(inner, item.serial);
        }
        Inbound::ContainerContents { items } => {
            let board = inner.play.board.as_ref().map(|b| b.serial);
            let messages: Vec<Serial> = items
                .iter()
                .filter(|item| Some(item.container) == board)
                .map(|item| item.serial)
                .collect();
            for message in messages {
                ask_for_summary(inner, message);
            }
        }
        Inbound::Bulletin(BulletinEvent::Summary {
            board,
            message,
            parent,
            poster,
            subject,
            time,
        }) => {
            if let Some(open) = inner.play.board.as_mut().filter(|b| b.serial == *board) {
                let post = open.posts.entry(message.0).or_default();
                post.parent = parent.is_valid().then_some(*parent);
                post.poster = poster.clone();
                post.subject = subject.clone();
                post.time = time.clone();
            }
        }
        Inbound::Bulletin(BulletinEvent::Message {
            board,
            message,
            poster,
            subject,
            time,
            lines,
        }) => {
            if let Some(open) = inner.play.board.as_mut().filter(|b| b.serial == *board) {
                let post = open.posts.entry(message.0).or_default();
                post.poster = poster.clone();
                post.subject = subject.clone();
                post.time = time.clone();
                post.lines = Some(lines.clone());
                open.reading = Some(*message);
            }
        }
        Inbound::BookContent { serial, pages } => {
            if let Some(book) = inner.play.book.as_mut().filter(|b| b.serial == *serial) {
                for page in pages {
                    book.pages.insert(page.number, page.lines.clone());
                }
            }
        }
        _ => {}
    }
}

/// Keeps whether the house designer is open.
pub(super) fn on_designer(inner: &mut Inner, msg: &Inbound) {
    if let Inbound::HouseDesigner { serial, designing } = msg {
        inner.play.designing = designing.then_some((*serial, DESIGNER_FIRST_FLOOR));
    }
}

/// The level the designer starts on.
const DESIGNER_FIRST_FLOOR: u8 = 1;

/// Keeps what the chat of the shard says.
pub(super) fn on_chat(inner: &mut Inner, msg: &Inbound) {
    let Inbound::Chat(event) = msg else {
        return;
    };
    let chat = &mut inner.play.chat;
    match event {
        ChatEvent::ChannelAdded { name, has_password } => {
            chat.channels.retain(|(kept, _)| kept != name);
            chat.channels.push((name.clone(), *has_password));
        }
        ChatEvent::ChannelRemoved { name } => chat.channels.retain(|(kept, _)| kept != name),
        ChatEvent::AsksForName => chat.asks_for_name = true,
        ChatEvent::Opened { name } => {
            chat.open = true;
            chat.asks_for_name = false;
            chat.name = name.clone();
        }
        ChatEvent::Closed => *chat = Chat::default(),
        ChatEvent::Joined { name } => chat.in_channel = name.clone(),
        ChatEvent::Left { name } if chat.in_channel == *name => chat.in_channel.clear(),
        ChatEvent::Left { .. } => {}
        ChatEvent::Said { who, words } => {
            chat.lines.push((who.clone(), words.clone()));
            while chat.lines.len() > CHAT_LINES_KEPT {
                chat.lines.remove(0);
            }
        }
    }
}

/// `chat`: turns the chat on, joins a channel, says words, or leaves.
pub(super) fn chat(inner: &mut Inner, args: &Value) -> ToolResult {
    let action = args.get(ARG_ACTION).and_then(Value::as_str).unwrap_or("");
    let words = |key: &str| args.get(key).and_then(Value::as_str).map(str::trim);
    let packet = match action {
        "open" => {
            let name = words(ARG_NAME).unwrap_or_default();
            let name = if name.is_empty() {
                inner.world.read().self_state.name.clone()
            } else {
                name.to_string()
            };
            encode::chat_open(&name)
        }
        "join" => {
            let Some(channel) = words(ARG_CHANNEL).filter(|channel| !channel.is_empty()) else {
                return ToolResult::err(NEEDS_CHANNEL);
            };
            encode::chat_join(channel, words(ARG_PASSWORD).filter(|p| !p.is_empty()))
        }
        "say" => {
            let Some(text) = words(ARG_TEXT).filter(|text| !text.is_empty()) else {
                return ToolResult::err(NEEDS_WORDS);
            };
            encode::chat_say(text)
        }
        "leave" => encode::chat_leave(),
        "" => return ToolResult::err(NEEDS_ACTION),
        _ => return ToolResult::err(BAD_CHAT_ACTION),
    };
    inner.outbound.push_back(packet);
    ToolResult::action(TOOL_CHAT)
}

/// `help`: asks the shard for its help menu.
pub(super) fn help(inner: &mut Inner) -> ToolResult {
    inner.outbound.push_back(encode::help_request());
    ToolResult::action(TOOL_HELP)
}

/// One step of the house designer, from the words of the call.
fn house_step(args: &Value) -> std::result::Result<HouseEdit, &'static str> {
    let number = |key: &str| args.get(key).and_then(Value::as_i64).map(|n| n as i32);
    let graphic = || {
        args.get(ARG_GRAPHIC)
            .and_then(Value::as_u64)
            .map(|graphic| graphic as u16)
            .ok_or(NEEDS_GRAPHIC)
    };
    let place = || (number(ARG_X).unwrap_or(0), number(ARG_Y).unwrap_or(0));
    match args.get(ARG_ACTION).and_then(Value::as_str).unwrap_or("") {
        "add" => {
            let (x, y) = place();
            Ok(HouseEdit::Add {
                graphic: graphic()?,
                x,
                y,
            })
        }
        "stair" => {
            let (x, y) = place();
            Ok(HouseEdit::AddStair {
                graphic: graphic()?,
                x,
                y,
            })
        }
        "remove" => {
            let (x, y) = place();
            Ok(HouseEdit::Remove {
                graphic: graphic()?,
                x,
                y,
                z: number(ARG_Z).unwrap_or(0),
            })
        }
        "roof" => {
            let (x, y) = place();
            Ok(HouseEdit::AddRoof {
                graphic: graphic()?,
                x,
                y,
                z: number(ARG_Z).unwrap_or(0),
            })
        }
        "remove_roof" => {
            let (x, y) = place();
            Ok(HouseEdit::RemoveRoof {
                graphic: graphic()?,
                x,
                y,
                z: number(ARG_Z).unwrap_or(0),
            })
        }
        "floor" => Ok(HouseEdit::GoToFloor(
            number(ARG_LEVEL).unwrap_or(1).clamp(1, i32::from(u8::MAX)) as u8,
        )),
        "clear" => Ok(HouseEdit::Clear),
        "revert" => Ok(HouseEdit::Revert),
        "commit" => Ok(HouseEdit::Commit),
        "exit" => Ok(HouseEdit::Exit),
        "backup" => Ok(HouseEdit::Backup),
        "restore" => Ok(HouseEdit::Restore),
        "" => Err(NEEDS_ACTION),
        _ => Err(BAD_ACTION),
    }
}

/// `house_edit`: one step of the house designer.
pub(super) fn house_edit(inner: &mut Inner, args: &Value) -> ToolResult {
    if inner.play.designing.is_none() {
        return ToolResult::err(NO_DESIGNER);
    }
    let step = match house_step(args) {
        Ok(step) => step,
        Err(words) => return ToolResult::err(words),
    };
    let me = inner.world.read().self_state.serial;
    inner.outbound.push_back(encode::house_edit(me, step));
    match step {
        HouseEdit::GoToFloor(level) => {
            if let Some((_, floor)) = inner.play.designing.as_mut() {
                *floor = level;
            }
        }
        HouseEdit::Commit | HouseEdit::Exit => inner.play.designing = None,
        _ => {}
    }
    ToolResult::action(TOOL_HOUSE_EDIT)
}

/// Keeps the map items and the profiles the shard sends.
pub(super) fn on_map_or_profile(inner: &mut Inner, msg: &Inbound) {
    match msg {
        Inbound::MapOpened(what) => {
            inner.play.maps.retain(|map| map.what.serial != what.serial);
            inner.play.maps.push(OpenMap {
                what: *what,
                pins: Vec::new(),
                may_plot: false,
            });
        }
        Inbound::MapChanged { serial, change } => {
            let Some(map) = inner
                .play
                .maps
                .iter_mut()
                .find(|map| map.what.serial == *serial)
            else {
                return;
            };
            match change {
                MapChange::Pin { x, y } => map.pins.push((*x, *y)),
                MapChange::Clear => map.pins.clear(),
                MapChange::MayPlot(may) => map.may_plot = *may,
            }
        }
        Inbound::MultiPlacement {
            multi_id,
            x_offset,
            y_offset,
            z_offset,
            hue,
            ..
        } => {
            inner.play.placing = Some(json!({
                "multi_id": multi_id,
                "x_offset": x_offset,
                "y_offset": y_offset,
                "z_offset": z_offset,
                "hue": hue,
            }));
        }
        Inbound::Profile {
            serial,
            title,
            own_words,
            shard_words,
        } => {
            inner.play.profiles.retain(|kept| kept.serial != *serial);
            inner.play.profiles.push(CharacterProfile {
                serial: *serial,
                title: title.clone(),
                shard_words: shard_words.clone(),
                own_words: own_words.clone(),
            });
            while inner.play.profiles.len() > PROFILES_KEPT {
                inner.play.profiles.remove(0);
            }
        }
        _ => {}
    }
}

/// Keeps the design of a house. Without the client files the house has no
/// bounds, so its tiles cannot be placed and it is left out.
pub(super) fn on_custom_house(
    inner: &mut Inner,
    house: &uoterm_protocol::CustomHouse,
    bounds: Option<uoterm_world::HouseBounds>,
) {
    let Some(bounds) = bounds else {
        return;
    };
    inner.play.design_asked.remove(&house.serial);
    inner.play.houses.insert(
        house.serial,
        uoterm_world::DesignedHouse {
            serial: house.serial,
            revision: house.revision,
            tiles: uoterm_world::house_tiles(house, bounds),
        },
    );
}

/// The shard said which design revision a house is at. The design is asked
/// for once when the one held is older or missing, as the reference client
/// asks, so the house shows and blocks with the walls its owner built.
pub(super) fn on_house_revision(inner: &mut Inner, serial: Serial, revision: u32) {
    let held = inner.play.houses.get(&serial).map(|house| house.revision);
    if held == Some(revision) || inner.play.design_asked.get(&serial) == Some(&revision) {
        return;
    }
    inner.play.design_asked.insert(serial, revision);
    inner
        .outbound
        .push_back(uoterm_protocol::encode::house_design_request(serial));
}

/// The map the call names, or the one that opened last.
fn open_map(inner: &Inner, args: &Value) -> std::result::Result<Serial, &'static str> {
    let named = arg_serial_opt(args, ARG_SERIAL).filter(|serial| serial.is_valid());
    match named {
        Some(serial) => inner
            .play
            .maps
            .iter()
            .any(|map| map.what.serial == serial)
            .then_some(serial)
            .ok_or(NO_MAP_OPEN),
        None => inner
            .play
            .maps
            .last()
            .map(|map| map.what.serial)
            .ok_or(NO_MAP_OPEN),
    }
}

/// `map_pin`: puts a pin on the open map, clears its pins, or asks the
/// shard to let it be drawn on.
pub(super) fn map_pin(inner: &mut Inner, args: &Value) -> ToolResult {
    let serial = match open_map(inner, args) {
        Ok(serial) => serial,
        Err(words) => return ToolResult::err(words),
    };
    let packet = match args.get(ARG_ACTION).and_then(Value::as_str) {
        Some(ACTION_CLEAR) => encode::map_clear_pins(serial),
        Some(ACTION_EDIT) => encode::map_toggle_edit(serial),
        _ => {
            let place = args
                .get(ARG_X)
                .and_then(Value::as_u64)
                .zip(args.get(ARG_Y).and_then(Value::as_u64));
            let Some((x, y)) = place else {
                return ToolResult::err(NEEDS_PLACE);
            };
            encode::map_add_pin(serial, x as u16, y as u16)
        }
    };
    inner.outbound.push_back(packet);
    ToolResult::action(TOOL_MAP_PIN)
}

pub(super) fn map_close(inner: &mut Inner, args: &Value) -> ToolResult {
    match arg_serial_opt(args, ARG_SERIAL).filter(|serial| serial.is_valid()) {
        Some(serial) => inner.play.maps.retain(|map| map.what.serial != serial),
        None => {
            inner.play.maps.pop();
        }
    }
    ToolResult::ok(json!({ "maps": inner.play.maps.len() }))
}

/// `profile`: asks for the profile of a character, or writes your own.
pub(super) fn profile(inner: &mut Inner, args: &Value) -> ToolResult {
    let serial = arg_serial(args, ARG_SERIAL);
    if serial == Serial(0) {
        return ToolResult::err(format!("{TOOL_PROFILE} {NEEDS_SERIAL}"));
    }
    let words = args.get(ARG_TEXT).and_then(Value::as_str);
    let packet = match words {
        Some(words) => encode::profile_write(serial, words),
        None => encode::profile_request(serial),
    };
    inner.outbound.push_back(packet);
    ToolResult::action(TOOL_PROFILE)
}

/// Asks for the list line of a message, once.
fn ask_for_summary(inner: &mut Inner, message: Serial) {
    let Some(board) = inner.play.board.as_mut() else {
        return;
    };
    if board.posts.contains_key(&message.0) {
        return;
    }
    board.posts.insert(message.0, Post::default());
    let packet = encode::bulletin_ask(board.serial, message, false);
    inner.outbound.push_back(packet);
}

/// The open board, or the words that refuse the call.
fn open_board(inner: &Inner) -> std::result::Result<Serial, &'static str> {
    inner
        .play
        .board
        .as_ref()
        .map(|board| board.serial)
        .ok_or(NO_BOARD_OPEN)
}

fn message_arg(inner: &Inner, args: &Value) -> std::result::Result<(Serial, Serial), &'static str> {
    let board = open_board(inner)?;
    let message = arg_serial(args, ARG_MESSAGE);
    let known = inner
        .play
        .board
        .as_ref()
        .is_some_and(|b| b.posts.contains_key(&message.0));
    if known {
        Ok((board, message))
    } else {
        Err(NEEDS_MESSAGE)
    }
}

pub(super) fn board_read(inner: &mut Inner, args: &Value) -> ToolResult {
    let (board, message) = match message_arg(inner, args) {
        Ok(found) => found,
        Err(refusal) => return ToolResult::err(refusal),
    };
    inner
        .outbound
        .push_back(encode::bulletin_ask(board, message, true));
    ToolResult::action(TOOL_BOARD_READ)
}

pub(super) fn board_post(inner: &mut Inner, args: &Value) -> ToolResult {
    let board = match open_board(inner) {
        Ok(board) => board,
        Err(refusal) => return ToolResult::err(refusal),
    };
    let subject = args
        .get(ARG_SUBJECT)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if subject.is_empty() {
        return ToolResult::err(NEEDS_SUBJECT);
    }
    let text = args.get(ARG_TEXT).and_then(Value::as_str).unwrap_or("");
    let lines: Vec<&str> = text.lines().collect();
    let reply_to = arg_serial(args, ARG_REPLY_TO);
    inner
        .outbound
        .push_back(encode::bulletin_post(board, reply_to, subject, &lines));
    ToolResult::action(TOOL_BOARD_POST)
}

pub(super) fn board_remove(inner: &mut Inner, args: &Value) -> ToolResult {
    let (board, message) = match message_arg(inner, args) {
        Ok(found) => found,
        Err(refusal) => return ToolResult::err(refusal),
    };
    inner
        .outbound
        .push_back(encode::bulletin_remove(board, message));
    if let Some(open) = inner.play.board.as_mut() {
        open.posts.remove(&message.0);
        open.reading = open.reading.filter(|reading| *reading != message);
    }
    ToolResult::action(TOOL_BOARD_REMOVE)
}

pub(super) fn board_close(inner: &mut Inner) -> ToolResult {
    inner.play.board = None;
    ToolResult::ok(json!({ "board": Value::Null }))
}

/// Which answer an old-style menu gets.
pub(super) enum MenuPick {
    /// The entry at this place, counted from one.
    Place(usize),
    /// The first entry whose words hold these, in any case.
    Words(String),
    /// No entry: the menu is closed.
    Cancel,
}

/// True while the shard waits on an old-style menu.
pub(super) fn old_menu_open(inner: &Inner) -> bool {
    inner.play.old_menu.is_some()
}

/// Answers the old-style menu, and closes it. The menu stays open when the
/// pick names no entry of it.
pub(super) fn pick_old_menu(
    inner: &mut Inner,
    pick: MenuPick,
) -> std::result::Result<(), &'static str> {
    let Some(menu) = inner.play.old_menu.take() else {
        return Err(NO_MENU_OPEN);
    };
    let place = match &pick {
        MenuPick::Cancel => None,
        MenuPick::Place(place) => Some(*place),
        MenuPick::Words(words) => {
            let words = words.to_lowercase();
            menu.entries
                .iter()
                .position(|entry| entry.name.to_lowercase().contains(&words))
                .map(|at| at + 1)
        }
    };
    let packet = match (pick, place) {
        (MenuPick::Cancel, _) => encode::menu_cancel(menu.serial, menu.menu_id),
        (_, Some(place)) => match place.checked_sub(1).and_then(|at| menu.entries.get(at)) {
            Some(entry) => encode::menu_response(
                menu.serial,
                menu.menu_id,
                place as u16,
                entry.graphic,
                entry.hue,
            ),
            None => {
                inner.play.old_menu = Some(menu);
                return Err(NO_SUCH_ENTRY);
            }
        },
        (_, None) => {
            inner.play.old_menu = Some(menu);
            return Err(NO_SUCH_ENTRY);
        }
    };
    inner.outbound.push_back(packet);
    Ok(())
}

/// `menu_pick`: answers the old-style menu. The entries count from one.
pub(super) fn menu_pick(inner: &mut Inner, args: &Value) -> ToolResult {
    let pick = match args.get(ARG_INDEX).and_then(Value::as_u64) {
        None => MenuPick::Cancel,
        Some(index) => MenuPick::Place(usize::try_from(index).unwrap_or(usize::MAX)),
    };
    match pick_old_menu(inner, pick) {
        Ok(()) => ToolResult::action(TOOL_MENU_PICK),
        Err(why) => ToolResult::err(why),
    }
}

/// `book_write`: names the open book, or writes one of its pages.
pub(super) fn book_write(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(book) = inner.play.book.as_ref().map(|book| book.serial) else {
        return ToolResult::err(NO_BOOK_OPEN);
    };
    let words = |key: &str| args.get(key).and_then(Value::as_str).map(str::trim);
    let title = words(ARG_TITLE);
    let author = words(ARG_AUTHOR);
    if title.is_some() || author.is_some() {
        let (kept_title, kept_author) = inner
            .play
            .book
            .as_ref()
            .map(|open| (open.title.clone(), open.author.clone()))
            .unwrap_or_default();
        let title = title.unwrap_or(&kept_title).to_string();
        let author = author.unwrap_or(&kept_author).to_string();
        inner
            .outbound
            .push_back(encode::book_header(book, &title, &author));
        if let Some(open) = inner.play.book.as_mut() {
            open.title = title;
            open.author = author;
        }
        return ToolResult::action(TOOL_BOOK_WRITE);
    }
    let Some(page) = args
        .get(ARG_PAGE)
        .and_then(Value::as_u64)
        .and_then(|page| u16::try_from(page).ok())
        .filter(|page| *page > 0)
    else {
        return ToolResult::err(NEEDS_PAGE);
    };
    let text = args.get(ARG_TEXT).and_then(Value::as_str).unwrap_or("");
    let lines: Vec<&str> = text.lines().collect();
    inner
        .outbound
        .push_back(encode::book_page(book, page, &lines));
    if let Some(open) = inner.play.book.as_mut() {
        open.pages
            .insert(page, lines.iter().map(|line| (*line).to_string()).collect());
    }
    ToolResult::action(TOOL_BOOK_WRITE)
}

pub(super) fn book_close(inner: &mut Inner) -> ToolResult {
    inner.play.book = None;
    ToolResult::ok(json!({ "book": Value::Null }))
}

pub(super) fn on_shop_closed(inner: &mut Inner) {
    inner.play.shop = None;
}

/// `context_menu` with a serial alone: ask for the menu and show its lines
/// in `watch`. With `index`: pick that line of the menu that is shown.
pub(super) fn context_menu(inner: &mut Inner, args: &Value) -> ToolResult {
    let serial = arg_serial(args, "serial");
    if serial == Serial(0) {
        return ToolResult::err(format!("{TOOL_CONTEXT_MENU} {NEEDS_SERIAL}"));
    }
    if !inner.world.read().has_context_menus() {
        return ToolResult::err(NO_CONTEXT_MENUS);
    }
    let Some(index) = args.get(ARG_INDEX).and_then(Value::as_u64) else {
        inner.play.menu = None;
        inner.play.menu_asked = Some(serial);
        inner
            .outbound
            .push_back(encode::context_menu_request(serial));
        return ToolResult::action(TOOL_CONTEXT_MENU);
    };
    if inner.play.menu.as_ref().map(|menu| menu.serial) != Some(serial) {
        return ToolResult::err(NO_MENU_SHOWN);
    }
    inner.play.menu = None;
    inner
        .outbound
        .push_back(encode::context_menu_response(serial, index as u16));
    ToolResult::action(TOOL_CONTEXT_MENU)
}

/// Closes the context menu the human looked at and picked nothing from.
pub(super) fn close_menu(inner: &mut Inner) -> ToolResult {
    inner.play.menu = None;
    inner.play.menu_asked = None;
    ToolResult::ok(json!({ "context_menu": Value::Null }))
}

/// The `[{serial, amount}]` rows the human ticked in a shop.
fn cart(args: &Value) -> Vec<(Serial, u16)> {
    args.get(ARG_ITEMS)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|row| {
                    let amount = row.get("amount").and_then(Value::as_u64)?;
                    let amount = u16::try_from(amount).ok().filter(|n| *n > 0)?;
                    Some((arg_serial(row, "serial"), amount))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Buys or sells the rows of the cart, by the kind of list that is open.
pub(super) fn shop_checkout(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(shop) = inner.play.shop.take() else {
        return ToolResult::err(NO_SHOP_OPEN);
    };
    let items = cart(args);
    if items.is_empty() {
        inner.play.shop = Some(shop);
        return ToolResult::err(CART_IS_EMPTY);
    }
    let packet = if shop.buying {
        encode::vendor_buy(shop.vendor, &items)
    } else {
        encode::vendor_sell(shop.vendor, &items)
    };
    inner.outbound.push_back(packet);
    ToolResult::action(TOOL_SHOP_CHECKOUT)
}

pub(super) fn shop_close(inner: &mut Inner) -> ToolResult {
    on_shop_closed(inner);
    ToolResult::ok(json!({ "shop": Value::Null }))
}

/// Sets the gold and platinum the character offers in the open trade.
pub(super) fn trade_gold(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(mine) = inner.world.read().trade.as_ref().map(|trade| trade.mine) else {
        return ToolResult::err(NO_TRADE_OPEN);
    };
    let gold = arg_u32(args, ARG_GOLD, 0);
    let platinum = arg_u32(args, ARG_PLATINUM, 0);
    inner
        .outbound
        .push_back(encode::trade_gold(mine, gold, platinum));
    ToolResult::action(TOOL_TRADE_GOLD)
}

/// The words of the tooltip of one object. The shard is asked for them when
/// the session has none yet, so the next call has them.
pub(super) fn properties(inner: &mut Inner, args: &Value) -> ToolResult {
    let serial = arg_serial(args, "serial");
    if serial == Serial(0) {
        return ToolResult::err(format!("{TOOL_PROPERTIES} {NEEDS_SERIAL}"));
    }
    let lines = property_lines(inner, serial);
    if lines.is_empty() {
        ask_what_it_is(inner, serial);
    }
    ToolResult::ok(json!({ "serial": serial, "lines": lines }))
}

fn contained(item: &uoterm_world::Item) -> Value {
    json!({
        "serial": item.serial,
        "graphic": item.graphic,
        "amount": item.amount,
        "hue": item.hue,
        "name": item.name,
        "x": item.location.x,
        "y": item.location.y,
        "grid": item.grid,
    })
}

/// The window's sheet lists every skill, trained or not.
const EVERY_SKILL: bool = true;

/// What the shard has open for the character to answer or read, and what he
/// knows: the goods of a shop, the lines of a context menu and of an old-style
/// menu, his skills and the spells in his books. `observe` carries them, so an
/// agent sees what a player sees on his screen.
///
/// With `every_skill` the skill list holds every skill, as a character sheet
/// shows them; without it, only those he has trained or locked.
pub(super) fn open_panels(
    inner: &Inner,
    into: &mut serde_json::Map<String, Value>,
    every_skill: bool,
) {
    let world = inner.world.read();
    into.insert(
        "shop".into(),
        json!(inner.play.shop.as_ref().map(|shop| json!({
            "vendor": shop.vendor,
            "vendor_name": world.name_of(shop.vendor),
            "buying": shop.buying,
            "goods": shop.goods,
        }))),
    );
    into.insert(
        "context_menu".into(),
        json!(inner
            .play
            .menu
            .as_ref()
            .map(|menu| json!({ "serial": menu.serial, "lines": menu.lines }))),
    );
    into.insert(
        "book".into(),
        json!(inner.play.book.as_ref().map(|book| json!({
            "serial": book.serial,
            "title": book.title,
            "author": book.author,
            "page_count": book.page_count,
            "pages": book.pages.values().collect::<Vec<_>>(),
        }))),
    );
    into.insert("paperdoll".into(), json!(world.paperdoll));
    into.insert(
        "menu".into(),
        json!(inner.play.old_menu.as_ref().map(|menu| json!({
            "question": menu.question,
            "entries": menu.entries,
        }))),
    );
    // The skills he has trained or locked; the rest are nought and say
    // nothing.
    let mut skills: Vec<_> = world
        .self_state
        .skills
        .iter()
        .filter(|(_, value)| {
            every_skill || value.base > 0 || value.value > 0 || value.lock != SKILL_LOCK_UP
        })
        .collect();
    skills.sort_by_key(|(id, _)| **id);
    let skills: Vec<Value> = skills
        .into_iter()
        .map(|(id, value)| {
            let known = inner.scripting.skills.by_id(*id);
            json!({
                "id": id,
                "name": known.map_or_else(|| format!("skill {id}"), |s| s.name.clone()),
                "usable": known.is_some_and(|s| s.usable),
                "value": value.value,
                "base": value.base,
                "cap": value.cap,
                "lock": value.lock,
            })
        })
        .collect();
    into.insert("skills".into(), json!(skills));
    let spells: Vec<Value> = world
        .spellbooks
        .iter()
        .map(|(book, content)| {
            let spells: Vec<Value> = content
                .spell_numbers()
                .into_iter()
                .map(|number| {
                    let name = inner.scripting.spells.by_id(number).map(|s| s.name.clone());
                    json!({ "number": number, "name": name })
                })
                .collect();
            json!({ "book": book, "spells": spells })
        })
        .collect();
    into.insert("spellbooks".into(), json!(spells));
}

/// The whole screen in one picture: `observe`, with each list at its full
/// length and the things only a screen draws.
pub(super) fn watch_value(inner: &Inner, size: u16) -> Value {
    let mut picture = observe_value(inner, size);
    if let Some(sheet) = picture.as_object_mut() {
        open_panels(inner, sheet, EVERY_SKILL);
    }
    let world = inner.world.read();
    // The newest first: the container the player just opened is the one
    // he wants to see, and a window shows only the first few.
    let mut open: Vec<&uoterm_world::Container> = world.containers.values().collect();
    open.sort_by_key(|container| std::cmp::Reverse(container.opened));
    let containers: Vec<Value> = open
        .into_iter()
        .map(|container| {
            let record = world.items.get(&container.serial);
            let contents: Vec<Value> = container
                .items
                .iter()
                .filter_map(|serial| world.items.get(serial))
                .map(contained)
                .collect();
            json!({
                "serial": container.serial,
                "name": record.map(|item| item.name.clone()).unwrap_or_default(),
                "graphic": record.map(|item| item.graphic),
                "gump": container.gump,
                "total": contents.len(),
                "contents": contents,
            })
        })
        .collect();
    let journal: Vec<Value> = world
        .journal
        .last_lines(WATCH_JOURNAL_LINES)
        .into_iter()
        .map(|line| {
            json!({
                "seq": line.seq,
                "serial": line.serial,
                "name": line.name,
                "hue": line.hue,
                "kind": line.kind,
                "text": line.text,
            })
        })
        .collect();
    let party: Vec<Value> = world
        .party
        .iter()
        .map(|member| {
            let seen = world.mobiles.get(member);
            json!({
                "serial": member,
                "name": world.name_of(*member),
                "hits": seen.and_then(|m| m.hits),
                "hits_max": seen.and_then(|m| m.hits_max),
            })
        })
        .collect();
    picture["mobiles"] = json!(world.mobiles.values().collect::<Vec<_>>());
    // A building is drawn from its pieces, not as one item.
    picture["items"] = json!(world
        .items
        .values()
        .filter(|item| item.parent.is_none() && !world.multis.contains_key(&item.serial))
        .collect::<Vec<_>>());
    picture["multis"] = json!(world.multis.values().collect::<Vec<_>>());
    picture["containers"] = json!(containers);
    picture["journal_lines"] = json!(journal);
    picture["party_members"] = json!(party);
    picture["cues"] = json!(world.cues.all());
    if let Some(trade) = &world.trade {
        let offered = |side: Serial| -> Vec<Value> {
            world
                .items_inside(side, false)
                .into_iter()
                .map(contained)
                .collect()
        };
        picture["trade"]["mine_items"] = json!(offered(trade.mine));
        picture["trade"]["their_items"] = json!(offered(trade.theirs));
    }
    picture["prompt"] = json!(world.prompt.is_some());
    picture["running"] = json!(inner.movement.last_step_ran);
    picture["season"] = json!(world.season);
    picture["light"] = json!(world.light);
    picture["personal_light"] = json!(world.personal_light);
    picture["time"] = json!({
        "hour": world.time.0,
        "minute": world.time.1,
        "second": world.time.2,
    });
    picture["quest_arrow"] = json!(world.quest_arrow.map(|(x, y)| json!({ "x": x, "y": y })));
    picture["waypoints"] = json!(world.waypoints.values().collect::<Vec<_>>());
    picture["shard_url"] = json!(world.shard_url);
    picture["shard_notice"] = json!(world.shard_notice);
    picture["weather"] = json!(world
        .weather
        .map(|(kind, count)| json!({ "kind": kind, "count": count })));
    picture["target_cursor"] = json!(world.pending_target);
    let words = |number: u32, arguments: &str| {
        inner
            .cliloc
            .as_ref()
            .and_then(|table| table.render(number, arguments))
    };
    picture["gump_layouts"] = json!(world
        .gumps
        .iter()
        .map(|gump| uoterm_world::gump_layout(gump, &words))
        .collect::<Vec<_>>());
    picture["maps"] = json!(inner
        .play
        .maps
        .iter()
        .map(|map| json!({
            "serial": map.what.serial,
            "gump": map.what.gump_id,
            "facet": map.what.facet,
            "start_x": map.what.start_x,
            "start_y": map.what.start_y,
            "end_x": map.what.end_x,
            "end_y": map.what.end_y,
            "width": map.what.width,
            "height": map.what.height,
            "may_plot": map.may_plot,
            "pins": map.pins.iter().map(|(x, y)| json!({ "x": x, "y": y })).collect::<Vec<_>>(),
        }))
        .collect::<Vec<_>>());
    picture["profiles"] = json!(inner
        .play
        .profiles
        .iter()
        .map(|profile| json!({
            "serial": profile.serial,
            "name": world.name_of(profile.serial),
            "title": profile.title,
            "shard_words": profile.shard_words,
            "own_words": profile.own_words,
        }))
        .collect::<Vec<_>>());
    // A building shows only while the cursor that places it is up.
    picture["chat"] = json!(inner.play.chat.open.then(|| json!({
        "name": inner.play.chat.name,
        "channels": inner
            .play
            .chat
            .channels
            .iter()
            .map(|(name, locked)| json!({ "name": name, "has_password": locked }))
            .collect::<Vec<_>>(),
        "in_channel": inner.play.chat.in_channel,
        "lines": inner
            .play
            .chat
            .lines
            .iter()
            .map(|(who, words)| json!({ "who": who, "words": words }))
            .collect::<Vec<_>>(),
    })));
    picture["chat_asks_for_name"] = json!(inner.play.chat.asks_for_name);
    picture["house_parts"] = json!(inner
        .house_parts
        .as_ref()
        .filter(|_| inner.play.designing.is_some())
        .map(|catalog| catalog.parts()));
    picture["designing"] = json!(inner
        .play
        .designing
        .map(|(serial, floor)| json!({ "serial": serial, "floor": floor })));
    picture["placing"] = json!(inner
        .play
        .placing
        .as_ref()
        .filter(|_| world.pending_target.is_some()));
    picture["designed_houses"] = json!(inner.play.houses.values().collect::<Vec<_>>());
    picture["board"] = json!(inner.play.board.as_ref().map(|board| json!({
        "serial": board.serial,
        "name": board.name,
        "reading": board.reading,
        "posts": board
            .posts
            .iter()
            .map(|(serial, post)| json!({
                "serial": serial,
                "parent": post.parent,
                "poster": post.poster,
                "subject": post.subject,
                "time": post.time,
                "lines": post.lines,
            }))
            .collect::<Vec<_>>(),
    })));
    picture["text_entry"] = json!(world
        .text_entry
        .as_ref()
        .map(|dialog| json!({ "title": dialog.text, "description": dialog.description })));
    picture
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::test_session;
    use super::*;

    const GUARD: Serial = Serial(0x0000_0A01);
    const SHOPKEEPER: Serial = Serial(0x0000_0A02);
    const LOG: Serial = Serial(0x4000_0B01);
    const LINE_INDEX: u16 = 3;
    const LINE_CLILOC: u32 = 3_006_123;
    const LOG_GRAPHIC: u16 = 0x1BDD;
    const LOG_PRICE: u16 = 2;
    const LOGS_HELD: u16 = 40;
    const LOGS_SOLD: u16 = 25;

    fn menu_line() -> ContextMenuEntry {
        ContextMenuEntry {
            index: LINE_INDEX,
            cliloc: LINE_CLILOC,
            flags: 0,
            colour: None,
        }
    }

    #[test]
    fn a_menu_the_human_asked_for_shows_in_watch_and_a_pick_closes_it() {
        let mut inner = test_session();
        assert!(context_menu(&mut inner, &json!({ "serial": GUARD })).ok);
        assert!(inner
            .outbound
            .contains(&encode::context_menu_request(GUARD)));
        on_context_menu(&mut inner, GUARD, &[menu_line()]);
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["context_menu"]["lines"][0]["index"], LINE_INDEX);
        assert_eq!(shown["context_menu"]["lines"][0]["enabled"], true);
        let pick = json!({ "serial": GUARD, "index": LINE_INDEX });
        assert!(context_menu(&mut inner, &pick).ok);
        assert!(inner
            .outbound
            .contains(&encode::context_menu_response(GUARD, LINE_INDEX)));
        assert!(watch_value(&inner, RADAR_DEFAULT)["context_menu"].is_null());
    }

    #[test]
    fn a_menu_nobody_asked_for_is_not_shown() {
        let mut inner = test_session();
        on_context_menu(&mut inner, GUARD, &[menu_line()]);
        assert!(watch_value(&inner, RADAR_DEFAULT)["context_menu"].is_null());
        assert!(!context_menu(&mut inner, &json!({ "serial": GUARD, "index": LINE_INDEX })).ok);
    }

    #[test]
    fn the_cart_of_a_sell_list_goes_to_the_shopkeeper() {
        let mut inner = test_session();
        assert!(!shop_checkout(&mut inner, &json!({})).ok);
        let entry = VendorSellEntry {
            serial: LOG,
            graphic: LOG_GRAPHIC,
            hue: 0,
            amount: LOGS_HELD,
            price: LOG_PRICE,
            name: "log".into(),
        };
        on_sell_list(&mut inner, SHOPKEEPER, &[entry]);
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["shop"]["buying"], false);
        assert_eq!(shown["shop"]["goods"][0]["price"], LOG_PRICE);
        assert!(!shop_checkout(&mut inner, &json!({ "items": [] })).ok);
        let cart = json!({ "items": [{ "serial": LOG, "amount": LOGS_SOLD }] });
        assert!(shop_checkout(&mut inner, &cart).ok);
        assert!(inner
            .outbound
            .contains(&encode::vendor_sell(SHOPKEEPER, &[(LOG, LOGS_SOLD)])));
        assert!(watch_value(&inner, RADAR_DEFAULT)["shop"].is_null());
    }

    #[test]
    fn watch_holds_what_only_a_screen_draws() {
        let inner = test_session();
        let shown = watch_value(&inner, RADAR_DEFAULT);
        for key in [
            "containers",
            "journal_lines",
            "skills",
            "party_members",
            "cues",
            "multis",
            "gump_layouts",
            "maps",
            "profiles",
            "designed_houses",
            "placing",
            "chat",
            "designing",
            "house_parts",
            "running",
            "season",
            "light",
            "time",
            "waypoints",
            "weather",
            "prompt",
            "target_cursor",
        ] {
            assert!(shown.get(key).is_some(), "watch has no {key}");
        }
    }

    /// A script picks an old-style menu entry by its words as well as its
    /// place, and words no entry holds leave the menu open.
    #[test]
    fn an_old_menu_is_picked_by_its_words() {
        const ANVIL: Serial = Serial(0x4000_0C02);
        const MENU_ID: u16 = 8;
        const DAGGER: u16 = 0x0F52;
        const KRYSS: u16 = 0x1401;
        let mut inner = test_session();
        let entry = |graphic, name: &str| MenuEntry {
            graphic,
            hue: 0,
            name: name.into(),
        };
        on_book_or_menu(
            &mut inner,
            &Inbound::OpenMenu {
                serial: ANVIL,
                menu_id: MENU_ID,
                question: "What do you make?".into(),
                entries: vec![entry(DAGGER, "dagger"), entry(KRYSS, "kryss")],
            },
        );
        assert_eq!(
            pick_old_menu(&mut inner, MenuPick::Words("halberd".into())),
            Err(NO_SUCH_ENTRY)
        );
        assert!(old_menu_open(&inner), "still open");
        assert_eq!(
            pick_old_menu(&mut inner, MenuPick::Words("KRYSS".into())),
            Ok(())
        );
        assert!(inner
            .outbound
            .contains(&encode::menu_response(ANVIL, MENU_ID, 2, KRYSS, 0)));
        assert!(!old_menu_open(&inner));
    }

    #[test]
    fn an_old_menu_shows_in_watch_and_a_pick_answers_it() {
        const FORGE: Serial = Serial(0x4000_0C01);
        const MENU_ID: u16 = 7;
        const DAGGER_GRAPHIC: u16 = 0x0F52;
        let mut inner = test_session();
        assert!(!menu_pick(&mut inner, &json!({ "index": 1 })).ok);
        let open = Inbound::OpenMenu {
            serial: FORGE,
            menu_id: MENU_ID,
            question: "What do you make?".into(),
            entries: vec![MenuEntry {
                graphic: DAGGER_GRAPHIC,
                hue: 0,
                name: "dagger".into(),
            }],
        };
        on_book_or_menu(&mut inner, &open);
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["menu"]["entries"][0]["name"], "dagger");
        assert!(!menu_pick(&mut inner, &json!({ "index": 2 })).ok);
        assert!(menu_pick(&mut inner, &json!({ "index": 1 })).ok);
        assert!(inner.outbound.contains(&encode::menu_response(
            FORGE,
            MENU_ID,
            1,
            DAGGER_GRAPHIC,
            0
        )));
        assert!(watch_value(&inner, RADAR_DEFAULT)["menu"].is_null());
        on_book_or_menu(&mut inner, &open);
        assert!(menu_pick(&mut inner, &json!({})).ok);
        assert!(inner
            .outbound
            .contains(&encode::menu_cancel(FORGE, MENU_ID)));
    }

    #[test]
    fn a_book_shows_its_cover_and_the_pages_that_came() {
        const BOOK: Serial = Serial(0x4000_0D01);
        let mut inner = test_session();
        on_book_or_menu(
            &mut inner,
            &Inbound::BookHeader {
                serial: BOOK,
                writable: false,
                page_count: 2,
                title: "Tales".into(),
                author: "Ann".into(),
            },
        );
        on_book_or_menu(
            &mut inner,
            &Inbound::BookContent {
                serial: BOOK,
                pages: vec![uoterm_protocol::BookPage {
                    number: 1,
                    lines: vec!["Once".into()],
                }],
            },
        );
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["book"]["title"], "Tales");
        assert_eq!(shown["book"]["pages"][0][0], "Once");
        assert!(book_close(&mut inner).ok);
        assert!(watch_value(&inner, RADAR_DEFAULT)["book"].is_null());
    }

    #[test]
    fn a_board_lists_its_messages_and_one_can_be_read_posted_and_removed() {
        const BOARD: Serial = Serial(0x4000_0E01);
        const NOTE: Serial = Serial(0x4000_0E02);
        let mut inner = test_session();
        assert!(!board_read(&mut inner, &json!({ "message": NOTE })).ok);
        on_book_or_menu(
            &mut inner,
            &Inbound::Bulletin(BulletinEvent::Opened {
                board: BOARD,
                name: "town board".into(),
            }),
        );
        let arrives = Inbound::AddItem(uoterm_protocol::ContainerItem {
            serial: NOTE,
            graphic: 0x0EB0,
            amount: 1,
            x: 0,
            y: 0,
            grid: 0,
            container: BOARD,
            hue: 0,
        });
        on_book_or_menu(&mut inner, &arrives);
        on_book_or_menu(&mut inner, &arrives);
        let asked = encode::bulletin_ask(BOARD, NOTE, false);
        assert_eq!(
            inner.outbound.iter().filter(|p| **p == asked).count(),
            1,
            "the list line is asked for once"
        );
        on_book_or_menu(
            &mut inner,
            &Inbound::Bulletin(BulletinEvent::Summary {
                board: BOARD,
                message: NOTE,
                parent: Serial(0),
                poster: "Ann".into(),
                subject: "Selling ore".into(),
                time: "Day 1".into(),
            }),
        );
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["board"]["posts"][0]["subject"], "Selling ore");
        assert!(shown["board"]["posts"][0]["lines"].is_null());
        assert!(board_read(&mut inner, &json!({ "message": NOTE })).ok);
        assert!(inner
            .outbound
            .contains(&encode::bulletin_ask(BOARD, NOTE, true)));
        assert!(!board_post(&mut inner, &json!({ "text": "no subject" })).ok);
        let post = json!({ "subject": "Re: ore", "text": "I buy.\nTonight.", "reply_to": NOTE });
        assert!(board_post(&mut inner, &post).ok);
        assert!(inner.outbound.contains(&encode::bulletin_post(
            BOARD,
            NOTE,
            "Re: ore",
            &["I buy.", "Tonight."]
        )));
        assert!(board_remove(&mut inner, &json!({ "message": NOTE })).ok);
        assert!(watch_value(&inner, RADAR_DEFAULT)["board"]["posts"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(board_close(&mut inner).ok);
        assert!(watch_value(&inner, RADAR_DEFAULT)["board"].is_null());
    }

    #[test]
    fn the_container_that_opened_last_comes_first() {
        const PACK: Serial = Serial(0x4000_1001);
        const BANK: Serial = Serial(0x4000_1002);
        let inner = test_session();
        let open = |serial: Serial| Inbound::OpenContainer { serial, gump: 0x3C };
        inner.world.write().apply(&open(PACK));
        inner.world.write().apply(&open(BANK));
        let shown = watch_value(&inner, RADAR_DEFAULT);
        let order: Vec<u64> = shown["containers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|container| container["serial"].as_u64().unwrap())
            .collect();
        assert_eq!(order, vec![u64::from(BANK.0), u64::from(PACK.0)]);
        // Opening the pack again brings it to the front.
        inner.world.write().apply(&open(PACK));
        let again = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(again["containers"][0]["serial"], u64::from(PACK.0));
    }

    #[test]
    fn a_map_item_keeps_its_pins_and_takes_a_new_one() {
        const MAP: Serial = Serial(0x4000_0F01);
        let mut inner = test_session();
        assert!(!map_pin(&mut inner, &json!({ "x": 5, "y": 6 })).ok);
        let opened = Inbound::MapOpened(DisplayMap {
            serial: MAP,
            gump_id: 0x139D,
            start_x: 1000,
            start_y: 1200,
            end_x: 1400,
            end_y: 1600,
            width: 200,
            height: 200,
            facet: 0,
        });
        on_map_or_profile(&mut inner, &opened);
        on_map_or_profile(
            &mut inner,
            &Inbound::MapChanged {
                serial: MAP,
                change: MapChange::Pin { x: 40, y: 90 },
            },
        );
        on_map_or_profile(
            &mut inner,
            &Inbound::MapChanged {
                serial: MAP,
                change: MapChange::MayPlot(true),
            },
        );
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["maps"][0]["pins"][0]["x"], 40);
        assert_eq!(shown["maps"][0]["may_plot"], true);
        assert!(!map_pin(&mut inner, &json!({})).ok, "a pin needs a place");
        assert!(map_pin(&mut inner, &json!({ "x": 5, "y": 6 })).ok);
        assert!(inner.outbound.contains(&encode::map_add_pin(MAP, 5, 6)));
        assert!(map_pin(&mut inner, &json!({ "action": "clear" })).ok);
        assert!(inner.outbound.contains(&encode::map_clear_pins(MAP)));
        assert!(map_close(&mut inner, &json!({})).ok);
        assert!(watch_value(&inner, RADAR_DEFAULT)["maps"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_profile_is_asked_for_and_written_and_shows_in_watch() {
        const ANN: Serial = Serial(0x0000_0A05);
        let mut inner = test_session();
        assert!(!profile(&mut inner, &json!({})).ok);
        assert!(profile(&mut inner, &json!({ "serial": ANN })).ok);
        assert!(inner.outbound.contains(&encode::profile_request(ANN)));
        assert!(profile(&mut inner, &json!({ "serial": ANN, "text": "I dig ore." })).ok);
        assert!(inner
            .outbound
            .contains(&encode::profile_write(ANN, "I dig ore.")));
        on_map_or_profile(
            &mut inner,
            &Inbound::Profile {
                serial: ANN,
                title: "Ann the miner".into(),
                own_words: "I dig ore.".into(),
                shard_words: "Guild of Miners".into(),
            },
        );
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["profiles"][0]["title"], "Ann the miner");
        assert_eq!(shown["profiles"][0]["own_words"], "I dig ore.");
    }

    #[test]
    fn a_house_is_kept_only_when_its_bounds_are_known() {
        const FOUNDATION: Serial = Serial(0x4000_0F02);
        let mut inner = test_session();
        let house = uoterm_protocol::CustomHouse {
            serial: FOUNDATION,
            revision: 3,
            planes: vec![uoterm_protocol::HousePlane {
                z_index: 0,
                mode: 0,
                data: vec![0x00, 0x64, 1, 2, 0],
            }],
        };
        on_custom_house(&mut inner, &house, None);
        assert!(watch_value(&inner, RADAR_DEFAULT)["designed_houses"]
            .as_array()
            .unwrap()
            .is_empty());
        let bounds = uoterm_world::HouseBounds {
            min_x: -3,
            min_y: -3,
            max_y: 3,
        };
        on_custom_house(&mut inner, &house, Some(bounds));
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["designed_houses"][0]["revision"], 3);
        assert_eq!(shown["designed_houses"][0]["tiles"][0]["graphic"], 0x64);
    }

    #[test]
    fn the_designer_takes_steps_only_while_it_is_open() {
        const HOUSE: Serial = Serial(0x4000_0F03);
        const WALL: u16 = 10;
        let mut inner = test_session();
        let add = json!({ "action": "add", "graphic": WALL, "x": -3, "y": 4 });
        assert!(!house_edit(&mut inner, &add).ok, "the designer is shut");
        on_designer(
            &mut inner,
            &Inbound::HouseDesigner {
                serial: HOUSE,
                designing: true,
            },
        );
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["designing"]["floor"], DESIGNER_FIRST_FLOOR);
        assert!(
            !house_edit(&mut inner, &json!({ "action": "add" })).ok,
            "no part"
        );
        assert!(!house_edit(&mut inner, &json!({ "action": "dance" })).ok);
        assert!(house_edit(&mut inner, &add).ok);
        let me = inner.world.read().self_state.serial;
        assert!(inner.outbound.contains(&encode::house_edit(
            me,
            HouseEdit::Add {
                graphic: WALL,
                x: -3,
                y: 4
            }
        )));
        assert!(house_edit(&mut inner, &json!({ "action": "floor", "level": 3 })).ok);
        assert_eq!(watch_value(&inner, RADAR_DEFAULT)["designing"]["floor"], 3);
        assert!(house_edit(&mut inner, &json!({ "action": "commit" })).ok);
        assert!(watch_value(&inner, RADAR_DEFAULT)["designing"].is_null());
    }

    #[test]
    fn the_chat_keeps_its_channels_and_its_lines() {
        let mut inner = test_session();
        assert!(!chat(&mut inner, &json!({})).ok);
        assert!(!chat(&mut inner, &json!({ "action": "sing" })).ok);
        assert!(!chat(&mut inner, &json!({ "action": "join" })).ok);
        assert!(watch_value(&inner, RADAR_DEFAULT)["chat"].is_null());
        let event = |event: ChatEvent| Inbound::Chat(event);
        on_chat(&mut inner, &event(ChatEvent::AsksForName));
        assert_eq!(
            watch_value(&inner, RADAR_DEFAULT)["chat_asks_for_name"],
            true
        );
        on_chat(
            &mut inner,
            &event(ChatEvent::Opened {
                name: "Mara".into(),
            }),
        );
        on_chat(
            &mut inner,
            &event(ChatEvent::ChannelAdded {
                name: "General".into(),
                has_password: false,
            }),
        );
        on_chat(
            &mut inner,
            &event(ChatEvent::Joined {
                name: "General".into(),
            }),
        );
        on_chat(
            &mut inner,
            &event(ChatEvent::Said {
                who: "Ann".into(),
                words: "Anyone selling ore?".into(),
            }),
        );
        let shown = watch_value(&inner, RADAR_DEFAULT);
        assert_eq!(shown["chat"]["name"], "Mara");
        assert_eq!(shown["chat"]["channels"][0]["name"], "General");
        assert_eq!(shown["chat"]["in_channel"], "General");
        assert_eq!(shown["chat"]["lines"][0]["who"], "Ann");
        assert_eq!(shown["chat_asks_for_name"], false);
        assert!(chat(&mut inner, &json!({ "action": "say", "text": "hail" })).ok);
        assert!(inner.outbound.contains(&encode::chat_say("hail")));
        assert!(chat(&mut inner, &json!({ "action": "join", "channel": "Trade" })).ok);
        assert!(inner.outbound.contains(&encode::chat_join("Trade", None)));
        on_chat(&mut inner, &event(ChatEvent::Closed));
        assert!(watch_value(&inner, RADAR_DEFAULT)["chat"].is_null());
    }

    #[test]
    fn help_asks_the_shard_for_its_menu() {
        let mut inner = test_session();
        assert!(help(&mut inner).ok);
        assert!(inner.outbound.contains(&encode::help_request()));
    }

    #[test]
    fn gold_for_a_trade_needs_an_open_trade() {
        let mut inner = test_session();
        assert!(!trade_gold(&mut inner, &json!({ "gold": 5 })).ok);
    }
}
