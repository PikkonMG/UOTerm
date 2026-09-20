//! What a human at the watch window needs from the session, and an agent
//! does not: the whole screen in one picture, the lines of a context menu,
//! the goods of a shopkeeper, the words of a tooltip.
//!
//! An agent reads `observe`, which is cut short on purpose. A window draws
//! everything in view, so it reads `watch`.

use super::*;
use uoterm_protocol::{
    BulletinEvent, ContextMenuEntry, Inbound, MenuEntry, VendorBuyEntry, VendorSellEntry,
};

/// How many journal lines a window gets. It shows the last few and draws the
/// newest over the heads of the speakers.
const WATCH_JOURNAL_LINES: usize = 60;
const ARG_INDEX: &str = "index";
const ARG_ITEMS: &str = "items";
const ARG_GOLD: &str = "gold";
const ARG_PLATINUM: &str = "platinum";
const NEEDS_SERIAL: &str = "needs serial";
const NO_MENU_SHOWN: &str = "no context menu is shown for this object; ask for it first";
const NO_SHOP_OPEN: &str = "no shop list is open";
const NO_TRADE_OPEN: &str = "no trade is open";
const NO_MENU_OPEN: &str = "no menu is open";
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

#[derive(Default)]
pub(super) struct Play {
    /// The object whose context menu the human waits for.
    menu_asked: Option<Serial>,
    menu: Option<ShownMenu>,
    shop: Option<Shop>,
    old_menu: Option<OldMenu>,
    book: Option<Book>,
    board: Option<Board>,
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

/// `menu_pick`: answers the old-style menu. The entries count from one.
pub(super) fn menu_pick(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(menu) = inner.play.old_menu.take() else {
        return ToolResult::err(NO_MENU_OPEN);
    };
    let packet = match args.get(ARG_INDEX).and_then(Value::as_u64) {
        None => encode::menu_cancel(menu.serial, menu.menu_id),
        Some(index) => {
            let entry = usize::try_from(index)
                .ok()
                .and_then(|index| index.checked_sub(1))
                .and_then(|at| menu.entries.get(at));
            let Some(entry) = entry else {
                inner.play.old_menu = Some(menu);
                return ToolResult::err(NO_SUCH_ENTRY);
            };
            encode::menu_response(
                menu.serial,
                menu.menu_id,
                index as u16,
                entry.graphic,
                entry.hue,
            )
        }
    };
    inner.outbound.push_back(packet);
    ToolResult::action(TOOL_MENU_PICK)
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
        inner
            .outbound
            .push_back(encode::batch_query_properties(&[serial]));
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

/// The whole screen in one picture: `observe`, with each list at its full
/// length and the things only a screen draws.
pub(super) fn watch_value(inner: &Inner, size: u16) -> Value {
    let mut picture = observe_value(inner, size);
    let world = inner.world.read();
    let containers: Vec<Value> = world
        .containers
        .values()
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
    let skills: Vec<Value> = {
        let mut skills: Vec<_> = world.self_state.skills.iter().collect();
        skills.sort_by_key(|(id, _)| **id);
        skills
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
            .collect()
    };
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
    picture["skills"] = json!(skills);
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
    picture["weather"] = json!(world
        .weather
        .map(|(kind, count)| json!({ "kind": kind, "count": count })));
    picture["target_cursor"] = json!(world.pending_target);
    picture["context_menu"] = json!(inner
        .play
        .menu
        .as_ref()
        .map(|menu| json!({ "serial": menu.serial, "lines": menu.lines })));
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
    picture["menu"] = json!(inner.play.old_menu.as_ref().map(|menu| json!({
        "question": menu.question,
        "entries": menu.entries,
    })));
    picture["book"] = json!(inner.play.book.as_ref().map(|book| json!({
        "serial": book.serial,
        "title": book.title,
        "author": book.author,
        "page_count": book.page_count,
        "pages": book.pages.values().collect::<Vec<_>>(),
    })));
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
    picture["shop"] = json!(inner.play.shop.as_ref().map(|shop| json!({
        "vendor": shop.vendor,
        "vendor_name": world.name_of(shop.vendor),
        "buying": shop.buying,
        "goods": shop.goods,
    })));
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
            "running",
            "season",
            "light",
            "weather",
            "prompt",
            "target_cursor",
        ] {
            assert!(shown.get(key).is_some(), "watch has no {key}");
        }
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
    fn gold_for_a_trade_needs_an_open_trade() {
        let mut inner = test_session();
        assert!(!trade_gold(&mut inner, &json!({ "gold": 5 })).ok);
    }
}
