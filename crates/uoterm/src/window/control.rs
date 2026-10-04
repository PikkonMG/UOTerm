//! The hand of the human on the character. The window turns a click or a
//! line of text into one act, and a worker thread makes the tool call, so
//! the window never waits for the session.
//!
//! The acts and their tool calls are `uoterm_view::act`, which the window
//! and the web client share. Each call says `human: true`. The session lets
//! those through while it refuses the agent.

use super::actions::guard::{load_grab_bags, save_grab_bags, Checked, Guard, NOTE_GRAB_BAG_SET};
use super::actions::LocalAim;
use super::link::Link;
use super::orders::{self, ORDER_OFF};
use super::settings::CombatOptions;
use crate::view::WatchFrame;
use eframe::egui;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;
use uoterm_runtime::tools::{
    TOOL_FIND_LANDMARKS, TOOL_HOTKEYS, TOOL_LIST_SCRIPTS, TOOL_PROPERTIES, TOOL_SCRIPT_READ,
    TOOL_SCRIPT_STATUS,
};

pub use uoterm_view::act::*;

const NOT_SURE: &str = "Jev is not sure which one you mean. Pick it from the list.";
const NO_PLACE_ON_MAP: &str = "The marker file names no place on this map.";
const NO_SUCH_PLACE: &str = "Jev is not sure which place you mean. Click the map instead.";

/// The answers that came, kept for their askers. A panel that takes its
/// answers leaves the answers of the other panels in the box.
struct AnswerBox {
    inbox: Receiver<(Asker, Answer)>,
    kept: RefCell<Vec<(Asker, Answer)>>,
}

impl AnswerBox {
    fn new(inbox: Receiver<(Asker, Answer)>) -> Self {
        Self {
            inbox,
            kept: RefCell::new(Vec::new()),
        }
    }

    /// The answers to the asks of `asker`, in the order they came.
    fn take(&self, asker: Asker) -> Vec<Answer> {
        let mut kept = self.kept.borrow_mut();
        kept.extend(self.inbox.try_iter());
        let (own, others) = split_answers(std::mem::take(&mut *kept), asker);
        *kept = others;
        own
    }
}

/// The sending end the window holds, and the reports that come back.
pub struct Hand {
    acts: Sender<Act>,
    reports: Receiver<Report>,
    wanted_tips: Sender<u32>,
    tips: Receiver<Tip>,
    asks: Sender<(Asker, Ask)>,
    answers: AnswerBox,
    pub orders_on: bool,
    /// Checks each act before it goes: the criminal question, and a click
    /// the window waits for.
    guard: RefCell<Guard>,
    /// Reports the window makes itself, with no call to the session.
    own_reports: RefCell<Vec<Report>>,
    /// Words of the client for the journal, as the classic client prints
    /// its own answers, until the keys take them.
    notes: RefCell<Vec<String>>,
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
            answers: AnswerBox::new(answers),
            orders_on,
            guard: RefCell::new(Guard::new(load_grab_bags())),
            own_reports: RefCell::new(Vec::new()),
            notes: RefCell::new(Vec::new()),
        }
    }

    /// Asks for the tooltip of a thing. It reads, so it needs no control and
    /// does not wait behind the acts.
    pub fn want_tip(&self, serial: u32) {
        let _ = self.wanted_tips.send(serial);
    }

    /// Asks for something that comes back as an answer to `asker`. It does
    /// not wait behind the acts.
    pub fn ask(&self, asker: Asker, ask: Ask) {
        let _ = self.asks.send((asker, ask));
    }

    /// The answers that came to the asks of `asker`.
    pub fn new_answers(&self, asker: Asker) -> Vec<Answer> {
        self.answers.take(asker)
    }

    pub fn new_tips(&self) -> Vec<Tip> {
        self.tips.try_iter().collect()
    }

    /// Sends one act, after the guard checked it.
    pub fn act(&self, act: Act) {
        let checked = self.guard.borrow_mut().check(act);
        match checked {
            Checked::Send(acts) => acts.into_iter().for_each(|act| self.send(act)),
            Checked::Asked => {}
            Checked::Aimed(words) => self.report(words),
            Checked::GrabBagSet(grab_bags) => {
                save_grab_bags(&grab_bags);
                self.report(NOTE_GRAB_BAG_SET);
            }
        }
    }

    fn send(&self, act: Act) {
        // The worker lives as long as the window, so a send cannot fail.
        let _ = self.acts.send(act);
    }

    /// Gives the guard the newest picture and the combat options.
    pub fn watch_over(&self, frame: &WatchFrame, combat: &CombatOptions) {
        self.guard.borrow_mut().watch_over(frame, combat);
    }

    /// The next click on a thing does this in place of its usual act.
    pub fn aim(&self, aim: LocalAim) {
        self.guard.borrow_mut().aim(aim);
    }

    pub fn aiming(&self) -> Option<LocalAim> {
        self.guard.borrow().aiming()
    }

    pub fn cancel_aim(&self) {
        self.guard.borrow_mut().cancel_aim();
    }

    /// The thing the last click took for this aim, once.
    pub fn take_picked(&self, aim: LocalAim) -> Option<u32> {
        self.guard.borrow_mut().take_picked(aim)
    }

    /// The bag grabbed items go into: the one the player set, or the
    /// backpack.
    pub fn grab_bag(&self) -> Option<u32> {
        self.guard.borrow().grab_bag()
    }

    /// The question that waits for the player, in words.
    pub fn question(&self) -> Option<&'static str> {
        self.guard.borrow().question()
    }

    /// Answers the question. Yes sends the act that waited.
    pub fn answer(&self, yes: bool) {
        let waited = self.guard.borrow_mut().answer(yes);
        if let Some(act) = waited {
            self.send(act);
        }
    }

    /// Leaves the world and closes the whole program.
    pub fn quit(&self, ctx: &egui::Context) {
        self.act(Act::Quit);
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Tells the player something the window found, as a report of an act.
    pub fn report(&self, words: &str) {
        self.own_reports.borrow_mut().push(Report {
            text: words.to_string(),
            failed: false,
        });
    }

    /// Prints words of the client in the journal, as the classic client
    /// prints its own answers, such as "You are not in a party."
    pub fn note(&self, words: &str) {
        self.notes.borrow_mut().push(words.to_string());
    }

    /// The words to print since the last frame.
    pub fn take_notes(&self) -> Vec<String> {
        std::mem::take(&mut *self.notes.borrow_mut())
    }

    /// The newest report with words, when one came since the last frame.
    pub fn newest_report(&self) -> Option<Report> {
        let own = self.own_reports.borrow_mut().pop();
        self.reports
            .try_iter()
            .filter(|report| !report.text.is_empty())
            .last()
            .or(own)
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
    inbox: &Receiver<(Asker, Ask)>,
    outbox: &Sender<(Asker, Answer)>,
    ctx: &egui::Context,
) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    for (asker, ask) in inbox {
        let answer = rt.block_on(answer_one(link, key, ask));
        if outbox.send((asker, answer)).is_err() {
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
        Ask::WearItem { wish, options } => {
            Answer::Picked(pick_one(key, orders::ASK_WEAR, &wish, &options).await)
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
    for (i, (tool, args)) in act.calls().into_iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(LIFT_TO_DROP).await;
        }
        if let Err(words) = link.call(tool, with_human(args)).await {
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
    fn each_asker_takes_only_its_own_answers() {
        let (outbox, inbox) = mpsc::channel();
        let answers = AnswerBox::new(inbox);
        outbox.send((Asker::Chat, Answer::Picked(Ok(2)))).unwrap();
        outbox.send((Asker::Deck, Answer::Picked(Ok(5)))).unwrap();
        outbox
            .send((Asker::Chat, Answer::Picked(Err("no".into()))))
            .unwrap();
        assert_eq!(
            answers.take(Asker::Chat),
            vec![Answer::Picked(Ok(2)), Answer::Picked(Err("no".into()))]
        );
        assert!(answers.take(Asker::Designer).is_empty());
        outbox.send((Asker::Deck, Answer::Picked(Ok(6)))).unwrap();
        assert_eq!(
            answers.take(Asker::Deck),
            vec![Answer::Picked(Ok(5)), Answer::Picked(Ok(6))]
        );
        assert!(answers.take(Asker::Deck).is_empty());
    }

    #[test]
    fn each_tool_name_is_a_tool_of_the_session() {
        let known = uoterm_runtime::tools::tool_names();
        let acts = [
            Act::Take,
            Act::GiveBack,
            Act::WalkTo { x: 0, y: 0 },
            Act::RunTo { x: 0, y: 0 },
            Act::Use(ITEM),
            Act::Look(ITEM),
            Act::Attack(ITEM),
            Act::Follow(ITEM),
            Act::Loot(ITEM),
            Act::Target(ITEM),
            Act::War(true),
            Act::Say {
                text: "hail".into(),
                hue: 0,
            },
            Act::Speak {
                channel: Channel::Yell,
                text: "guards".into(),
                hue: 0,
            },
            Act::Speak {
                channel: Channel::Whisper,
                text: "psst".into(),
                hue: 0,
            },
            Act::Speak {
                channel: Channel::Emote,
                text: "smiles".into(),
                hue: 0,
            },
            Act::Speak {
                channel: Channel::PartyMember(ITEM),
                text: "heal me".into(),
                hue: 0,
            },
            Act::Hotkey("Resync".into()),
            Act::Stop,
            Act::Deposit,
            Act::HouseContent(true),
            Act::GameView {
                width: 800,
                height: 600,
            },
            Act::Step {
                direction: "n",
                run: false,
                open_doors: false,
            },
            Act::Move {
                item: ITEM,
                amount: 1,
                to: DropTo::Into(BAG),
            },
            Act::Wear(ITEM),
            Act::WearLastWeapon,
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
            Act::BookName {
                title: "Tales".into(),
                author: "Ann".into(),
            },
            Act::BookPage {
                page: 1,
                text: "Once".into(),
            },
            Act::BookRead(1),
            Act::BoardRead(ITEM),
            Act::BoardPost {
                subject: "Hi".into(),
                text: "one".into(),
                reply_to: None,
            },
            Act::BoardRemove(ITEM),
            Act::BoardClose,
            Act::MapPin { x: 1, y: 2 },
            Act::MapPinMove { pin: 0, x: 1, y: 2 },
            Act::MapPinRemove(0),
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
            Act::QuestArrow { right: true },
            Act::RaceChange(None),
            Act::RaceChange(Some(uoterm_protocol::NewLooks::default())),
            Act::ChatOpen("Mara".into()),
            Act::ChatJoin("General".into()),
            Act::ChatJoinWithPassword {
                channel: "Guild".into(),
                password: "pw".into(),
            },
            Act::ChatCreate("Trade".into()),
            Act::ChatSay("hail".into()),
            Act::ChatLeave,
            Act::Tip { next: true },
            Act::Quit,
            Act::Checkout(vec![(ITEM, 1)]),
            Act::ShopClose,
            Act::TradeWith(ITEM),
            Act::TradeAccept {
                trade: ITEM,
                accept: true,
            },
            Act::TradeCancel(ITEM),
            Act::TradeGold {
                trade: ITEM,
                gold: 1,
                platinum: 0,
            },
            Act::UseSkill(1),
            Act::Cast(1),
            Act::CastFrom {
                spell: 1,
                book: ITEM,
            },
            Act::Dye(2),
            Act::OpenSpellbook("magery"),
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
            Act::AgentOn {
                agent: "bandage".into(),
                on: true,
            },
            Act::AgentSet {
                agent: "bandage".into(),
                list: None,
                settings: json!({}),
            },
            Act::AgentRun {
                agent: "organizer".into(),
                list: Some("reagents".into()),
            },
            Act::AgentStop,
            Act::DamageMeter("start"),
            Act::PartyInvite(ITEM),
            Act::PartyLeave,
            Act::PartyKick(ITEM),
            Act::PartyLoot(true),
            Act::MobileStatus {
                serial: ITEM,
                close: true,
            },
            Act::VirtueGump(ITEM),
        ];
        for act in acts {
            for (tool, _) in act.calls() {
                assert!(known.contains(&tool), "{tool} is not a session tool");
            }
        }
    }
}
