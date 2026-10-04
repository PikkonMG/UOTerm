//! What the web view asks the page to do, and the hand of the human on the
//! character: each act goes through the guard, then out as a call the page
//! makes on its live link. The page gives each answer back by its id.

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use uoterm_view::act::{
    split_answers, string_list, tip_lines, Act, Answer, Ask, Asker, PageAct, Report, Tip,
};
use uoterm_view::asks::{AskCall, AskRun, AskStep, CallResult};
use uoterm_view::frame::WatchFrame;
use uoterm_view::guard::{Guard, HandStep, KeptGrabBags, LocalAim, GRAB_BAGS_FILE};
use uoterm_view::model::reads::{ReadCache, ReadKey};
use uoterm_view::settings::CombatOptions;
use uoterm_view::tips::{TIP_RETRY_SECONDS, TIP_TRIES};
use uoterm_world::tool_names::TOOL_PROPERTIES;

/// The Jev route of an order in plain words.
pub const JEV_ORDER: &str = "order";
/// The Jev route that picks one of a list of names.
pub const JEV_PICK: &str = "pick";
/// The Jev route that turns a wish into script lines.
pub const JEV_LINES: &str = "lines";
/// The key of the place Jev picked in the answer of a pick.
const INDEX_KEY: &str = "index";
/// The key of the script lines in the answer of a wish.
const LINES_KEY: &str = "lines";
/// The script lines of a wish are one text, a line each.
const LINE_BREAK: &str = "\n";
/// The key of the error words in a failed answer of the server.
const ERROR_KEY: &str = "error";
/// The key of the act in the answer of an order.
const ACT_KEY: &str = "act";

/// One thing the page does for the view.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum OutCall {
    /// One act: its calls go on the live link as one `act` message. The
    /// page gives the answer back by `id`.
    Act { id: u64, act: PageAct },
    /// One call that reads: its answer comes back by `id`.
    Read { id: u64, tool: String, args: Value },
    /// A question for Jev: a post to `/v1/sessions/{session}/jev/{route}`.
    /// The answer comes back by `id`.
    Jev { id: u64, route: String, body: Value },
    /// Keep the profile where the page keeps it.
    SaveProfile { profile: Value },
    /// Keep a file of the config folder, by its name.
    SaveKept { name: String, data: Value },
    /// Take a picture of the window, and give where it went back.
    Screenshot,
    /// The field of the chat line takes the keys, or lets them go.
    ChatFocus { take: bool },
    /// Paste the clipboard into the chat line.
    ChatPaste,
    /// Give the player a text file to keep, as the journal saves one.
    Download { name: String, text: String },
    /// Keep the profile as the start of each new character.
    SaveDefaultProfile { profile: Value },
    /// The window mode of the profile asks for the full screen, or lets it
    /// go. The browser grants it only during a press of the player.
    Fullscreen { on: bool },
}

/// What a call the page makes for the view answers.
enum Asked {
    Act {
        words: String,
    },
    Read(ReadKey),
    Tip {
        serial: u32,
        tries: usize,
    },
    Order,
    /// One call of the ask of a panel.
    Ask {
        asker: Asker,
        run: AskRun,
        call: CallKind,
    },
}

/// The kind of call an ask made, which tells how to read its answer.
#[derive(Clone, Copy)]
enum CallKind {
    Read,
    Pick,
    Lines,
}

/// A call an ask makes, before it has its id.
enum Made {
    Read(&'static str, Value),
    Jev(&'static str, Value),
}

/// The words of a failed answer: the error words of the server, words
/// alone, or the answer as it came.
fn failure_words(answer: &Value) -> String {
    answer
        .get(ERROR_KEY)
        .and_then(Value::as_str)
        .or_else(|| answer.as_str())
        .map_or_else(|| answer.to_string(), str::to_string)
}

/// The sending end of the view: the guard, the calls for the page, and
/// the answers that came.
pub struct Hand {
    guard: Guard,
    out: Vec<OutCall>,
    asked: HashMap<u64, Asked>,
    next_id: u64,
    /// The picture of the `watch` tool the page shows, for an order.
    watch: Value,
    reads: ReadCache,
    /// The newest report, and when it came.
    report: Option<(Report, f64)>,
    /// Words of the client for the journal, until the controls take them.
    notes: Vec<String>,
    tips: Vec<Tip>,
    /// The answers to the asks of the panels, until their askers take them.
    answers: Vec<(Asker, Answer)>,
    /// The tooltips to ask again, when, and how many times they were
    /// asked.
    tip_retries: Vec<(f64, u32, usize)>,
    /// The clock of the last frame.
    time: f64,
}

impl Default for Hand {
    fn default() -> Self {
        Self {
            guard: Guard::new(KeptGrabBags::default()),
            out: Vec::new(),
            asked: HashMap::new(),
            next_id: 0,
            watch: Value::Null,
            reads: ReadCache::default(),
            report: None,
            notes: Vec::new(),
            tips: Vec::new(),
            answers: Vec::new(),
            tip_retries: Vec::new(),
            time: 0.0,
        }
    }
}

impl Hand {
    fn wait_for(&mut self, asked: Asked) -> u64 {
        self.next_id += 1;
        self.asked.insert(self.next_id, asked);
        self.next_id
    }

    /// The calls for the page since the last call.
    pub fn take_out(&mut self) -> Vec<OutCall> {
        std::mem::take(&mut self.out)
    }

    /// Asks the page for something that is not an act.
    pub fn push(&mut self, call: OutCall) {
        self.out.push(call);
    }

    /// Sends one act, after the guard checked it.
    pub fn act(&mut self, act: Act) {
        for step in self.guard.check(act).steps() {
            match step {
                HandStep::Send(act) => self.send(act),
                HandStep::Report(words) => self.report(words),
                HandStep::KeepGrabBags(grab_bags) => self.save_grab_bags(&grab_bags),
            }
        }
    }

    fn save_grab_bags(&mut self, grab_bags: &KeptGrabBags) {
        if let Ok(data) = serde_json::to_value(grab_bags) {
            self.out.push(OutCall::SaveKept {
                name: GRAB_BAGS_FILE.to_string(),
                data,
            });
        }
    }

    fn send(&mut self, act: Act) {
        if let Act::Order(words, _) = &act {
            let body = json!({ "words": words, "frame": self.watch });
            let id = self.wait_for(Asked::Order);
            self.out.push(OutCall::Jev {
                id,
                route: JEV_ORDER.to_string(),
                body,
            });
            return;
        }
        let act = act.for_page();
        let id = self.wait_for(Asked::Act {
            words: act.words.clone(),
        });
        self.out.push(OutCall::Act { id, act });
    }

    /// Gives the guard the newest picture, as the `watch` tool gave it,
    /// and the combat options.
    pub fn watch_over(&mut self, frame: &WatchFrame, combat: &CombatOptions) {
        self.guard.watch_over(frame, combat);
    }

    /// The picture of the `watch` tool the page shows now.
    pub fn set_watch(&mut self, watch: Value) {
        self.watch = watch;
    }

    pub fn aim(&mut self, aim: LocalAim) {
        self.guard.aim(aim);
    }

    pub fn aiming(&self) -> Option<LocalAim> {
        self.guard.aiming()
    }

    /// The thing a click took for `aim`, once.
    pub fn take_picked(&mut self, aim: LocalAim) -> Option<u32> {
        self.guard.take_picked(aim)
    }

    /// The bag grabbed items go into.
    pub fn grab_bag(&self) -> Option<u32> {
        self.guard.grab_bag()
    }

    pub fn cancel_aim(&mut self) {
        self.guard.cancel_aim();
    }

    /// The question that waits for the player, in words.
    pub fn question(&self) -> Option<&'static str> {
        self.guard.question()
    }

    /// Answers the question. Yes sends the act that waited.
    pub fn answer_question(&mut self, yes: bool) {
        if let Some(act) = self.guard.answer(yes) {
            self.send(act);
        }
    }

    /// Takes the grab bags the page read from the session.
    pub fn keep_grab_bags(&mut self, grab_bags: KeptGrabBags) {
        self.guard.keep_grab_bags(grab_bags);
    }

    /// Tells the player something the view found, as a report of an act.
    pub fn report(&mut self, words: &str) {
        self.report = Some((
            Report {
                text: words.to_string(),
                failed: false,
            },
            self.time,
        ));
    }

    /// Tells the player of something that failed, as a failed act.
    pub fn fail(&mut self, words: &str) {
        self.report = Some((
            Report {
                text: words.to_string(),
                failed: true,
            },
            self.time,
        ));
    }

    /// The clock of the last frame.
    pub fn time(&self) -> f64 {
        self.time
    }

    /// The newest report, and when it came.
    pub fn newest_report(&self) -> Option<&(Report, f64)> {
        self.report.as_ref()
    }

    /// Prints words of the client in the journal, as the classic client
    /// prints its own answers.
    pub fn note(&mut self, words: &str) {
        self.notes.push(words.to_string());
    }

    /// The words to print since the last frame.
    pub fn take_notes(&mut self) -> Vec<String> {
        std::mem::take(&mut self.notes)
    }

    /// Asks for something that comes back as an answer to `asker`: the
    /// calls of the ask go out one after the other.
    pub fn ask(&mut self, asker: Asker, ask: Ask) {
        let (run, call) = AskRun::start(ask);
        self.ask_call(asker, run, call);
    }

    fn ask_call(&mut self, asker: Asker, run: AskRun, call: AskCall) {
        let (kind, made) = match call {
            AskCall::Read { tool, args } => (CallKind::Read, Made::Read(tool, args)),
            AskCall::Pick {
                question,
                wish,
                names,
            } => (
                CallKind::Pick,
                Made::Jev(
                    JEV_PICK,
                    json!({ "question": question, "names": names, "wish": wish }),
                ),
            ),
            AskCall::Lines { wish } => (
                CallKind::Lines,
                Made::Jev(JEV_LINES, json!({ "wish": wish })),
            ),
        };
        let id = self.wait_for(Asked::Ask {
            asker,
            run,
            call: kind,
        });
        self.out.push(match made {
            Made::Read(tool, args) => OutCall::Read {
                id,
                tool: tool.to_string(),
                args,
            },
            Made::Jev(route, body) => OutCall::Jev {
                id,
                route: route.to_string(),
                body,
            },
        });
    }

    /// The answers that came to the asks of `asker`, in the order they
    /// came.
    pub fn new_answers(&mut self, asker: Asker) -> Vec<Answer> {
        let (own, others) = split_answers(std::mem::take(&mut self.answers), asker);
        self.answers = others;
        own
    }

    /// Asks for the tooltip of a thing. It reads, so it needs no control.
    pub fn want_tip(&mut self, serial: u32) {
        self.ask_tip(serial, 1);
    }

    fn ask_tip(&mut self, serial: u32, tries: usize) {
        let id = self.wait_for(Asked::Tip { serial, tries });
        self.out.push(OutCall::Read {
            id,
            tool: TOOL_PROPERTIES.to_string(),
            args: json!({ "serial": serial }),
        });
    }

    /// The tooltips that came since the last frame.
    pub fn take_tips(&mut self) -> Vec<Tip> {
        std::mem::take(&mut self.tips)
    }

    /// The reads of the panels.
    pub fn reads(&mut self) -> &mut ReadCache {
        &mut self.reads
    }

    /// Call this once in each frame, before the panels read: it asks again
    /// for the tooltips that came empty.
    pub fn begin(&mut self, time: f64) {
        self.time = time;
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.tip_retries)
            .into_iter()
            .partition(|(at, _, _)| *at <= time);
        self.tip_retries = later;
        for (_, serial, tries) in due {
            self.ask_tip(serial, tries + 1);
        }
    }

    /// Call this once in each frame, after the panels read: it asks for
    /// the reads they wanted that are due.
    pub fn ask_due(&mut self) {
        for key in self.reads.due(self.time) {
            let tool = key.tool.to_string();
            let args = key.arguments();
            let id = self.wait_for(Asked::Read(key));
            self.out.push(OutCall::Read { id, tool, args });
        }
    }

    /// The answer to call `id` came: `ok` with its result, or failed with
    /// the words of the result.
    pub fn answered(&mut self, id: u64, ok: bool, result: Value, time: f64) {
        let Some(asked) = self.asked.remove(&id) else {
            return;
        };
        match asked {
            Asked::Act { words } => {
                let report = if ok {
                    Report {
                        text: words,
                        failed: false,
                    }
                } else {
                    Report {
                        text: failure_words(&result),
                        failed: true,
                    }
                };
                if !report.text.is_empty() {
                    self.report = Some((report, time));
                }
            }
            Asked::Read(key) => {
                let answer = if ok {
                    Ok(result)
                } else {
                    Err(failure_words(&result))
                };
                self.reads.arrived(key, answer, time);
            }
            Asked::Tip { serial, tries } => {
                let lines = tip_lines(ok.then_some(&result));
                if lines.is_empty() && tries < TIP_TRIES {
                    self.tip_retries
                        .push((time + TIP_RETRY_SECONDS, serial, tries));
                } else {
                    self.tips.push(Tip { serial, lines });
                }
            }
            Asked::Ask {
                asker,
                mut run,
                call,
            } => {
                let words = || failure_words(&result);
                let result = match call {
                    CallKind::Read => {
                        CallResult::Read(if ok { Ok(result.clone()) } else { Err(words()) })
                    }
                    CallKind::Pick => CallResult::Picked(if ok {
                        Ok(result
                            .get(INDEX_KEY)
                            .and_then(Value::as_u64)
                            .and_then(|index| usize::try_from(index).ok()))
                    } else {
                        Err(words())
                    }),
                    CallKind::Lines => CallResult::Lines(if ok {
                        Ok(string_list(Some(&result), LINES_KEY).join(LINE_BREAK))
                    } else {
                        Err(words())
                    }),
                };
                match run.next(result) {
                    AskStep::Call(next) => self.ask_call(asker, run, next),
                    AskStep::Done(answer) => self.answers.push((asker, answer)),
                }
            }
            Asked::Order => {
                let act = ok
                    .then(|| result.get(ACT_KEY).cloned())
                    .flatten()
                    .and_then(|act| serde_json::from_value::<PageAct>(act).ok());
                match act {
                    Some(act) => {
                        let id = self.wait_for(Asked::Act {
                            words: act.words.clone(),
                        });
                        self.out.push(OutCall::Act { id, act });
                    }
                    None => {
                        self.report = Some((
                            Report {
                                text: failure_words(&result),
                                failed: true,
                            },
                            time,
                        ));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_view::act::ToolCallOut;
    use uoterm_world::tool_names::TOOL_FIND_LANDMARKS;

    const ITEM: u32 = 0x4000_0001;

    #[test]
    fn an_act_goes_out_with_its_calls_and_its_answer_reports() {
        let mut hand = Hand::default();
        hand.act(Act::Use(ITEM));
        let out = hand.take_out();
        let [OutCall::Act { id, act }] = out.as_slice() else {
            panic!("{out:?}");
        };
        assert_eq!(*act, Act::Use(ITEM).for_page());
        hand.answered(*id, true, Value::Null, 1.0);
        let (report, at) = hand.newest_report().unwrap();
        assert_eq!((report.text.as_str(), *at), ("Use.", 1.0));
        hand.answered(*id, false, json!({ "error": "late" }), 2.0);
        assert_eq!(hand.newest_report().unwrap().0.text, "Use.", "one answer");
    }

    #[test]
    fn an_act_goes_on_the_wire_as_the_page_reads_it() {
        let call = OutCall::Act {
            id: 3,
            act: PageAct {
                calls: vec![ToolCallOut {
                    tool: "use".into(),
                    args: json!({ "serial": 1 }),
                }],
                words: "Use.".into(),
            },
        };
        let wire = serde_json::to_value(&call).unwrap();
        assert_eq!(wire["kind"], "Act");
        assert_eq!(wire["id"], 3);
        assert_eq!(wire["act"]["calls"][0]["tool"], "use");
    }

    #[test]
    fn an_empty_tooltip_is_asked_again_a_few_times() {
        let mut hand = Hand::default();
        hand.want_tip(ITEM);
        for tries in 1..=TIP_TRIES {
            let out = hand.take_out();
            let [OutCall::Read { id, tool, .. }] = out.as_slice() else {
                panic!("try {tries}: {out:?}");
            };
            assert_eq!(tool, TOOL_PROPERTIES);
            let at = tries as f64;
            hand.answered(*id, true, json!({ "lines": [] }), at);
            hand.begin(at + TIP_RETRY_SECONDS);
        }
        assert!(hand.take_out().is_empty(), "no more tries");
        assert_eq!(
            hand.take_tips(),
            [Tip {
                serial: ITEM,
                lines: Vec::new()
            }]
        );
    }

    #[test]
    fn an_order_asks_jev_and_its_act_goes_out() {
        let mut hand = Hand::default();
        hand.set_watch(json!({ "x": 1 }));
        hand.act(Act::Order("attack the orc".into(), Box::default()));
        let out = hand.take_out();
        let [OutCall::Jev { id, route, body }] = out.as_slice() else {
            panic!("{out:?}");
        };
        assert_eq!(route, JEV_ORDER);
        assert_eq!(body["frame"]["x"], 1);
        let act = Act::Attack(ITEM).for_page();
        hand.answered(*id, true, json!({ "act": act }), 1.0);
        assert!(
            matches!(hand.take_out().as_slice(), [OutCall::Act { act: sent, .. }] if *sent == act)
        );
    }

    #[test]
    fn a_read_of_a_panel_goes_out_when_due_and_its_answer_is_kept() {
        let mut hand = Hand::default();
        let key = ReadKey::new(TOOL_PROPERTIES, &json!({ "serial": ITEM }));
        hand.begin(0.0);
        assert!(hand.reads().want(key.clone(), 1.0).is_none());
        hand.ask_due();
        let out = hand.take_out();
        let [OutCall::Read { id, args, .. }] = out.as_slice() else {
            panic!("{out:?}");
        };
        assert_eq!(args["serial"], ITEM);
        hand.answered(*id, true, json!({ "lines": ["a"] }), 0.5);
        assert_eq!(hand.reads().value(&key), Some(&json!({ "lines": ["a"] })));
    }

    #[test]
    fn a_wear_ask_goes_to_jev_and_its_answer_comes_back_to_its_asker() {
        let mut hand = Hand::default();
        hand.ask(
            Asker::Deck,
            Ask::WearItem {
                wish: "my sword".into(),
                options: vec!["sword".into(), "shield".into()],
            },
        );
        let out = hand.take_out();
        let [OutCall::Jev { id, route, body }] = out.as_slice() else {
            panic!("{out:?}");
        };
        assert_eq!(route, JEV_PICK);
        assert_eq!(body["question"], "wear");
        assert_eq!(body["names"], json!(["sword", "shield"]));
        assert_eq!(body["wish"], "my sword");
        hand.answered(*id, true, json!({ "index": 1 }), 1.0);
        assert!(hand.new_answers(Asker::Chat).is_empty(), "the deck asked");
        assert_eq!(hand.new_answers(Asker::Deck), vec![Answer::Picked(Ok(1))]);
        assert!(hand.new_answers(Asker::Deck).is_empty(), "taken once");
    }

    #[test]
    fn an_ask_of_two_calls_reads_then_asks_jev() {
        let mut hand = Hand::default();
        hand.ask(
            Asker::MapItem,
            Ask::PlaceOnMap {
                wish: "bank".into(),
                map: 0,
                from: (0, 0),
                to: (100, 100),
            },
        );
        let out = hand.take_out();
        let [OutCall::Read { id, tool, .. }] = out.as_slice() else {
            panic!("{out:?}");
        };
        assert_eq!(tool, TOOL_FIND_LANDMARKS);
        let landmarks = json!([{"name": "Bank", "map": 0, "location": {"x": 5, "y": 6}}]);
        hand.answered(*id, true, landmarks, 1.0);
        let out = hand.take_out();
        let [OutCall::Jev { id, route, .. }] = out.as_slice() else {
            panic!("{out:?}");
        };
        assert_eq!(route, JEV_PICK);
        hand.answered(*id, false, json!({ "error": "off" }), 2.0);
        assert_eq!(
            hand.new_answers(Asker::MapItem),
            vec![Answer::Place(Err("off".into()))]
        );
    }

    #[test]
    fn script_lines_come_from_jev_as_one_text() {
        let mut hand = Hand::default();
        hand.ask(Asker::Macros, Ask::LinesFor("heal me".into()));
        let out = hand.take_out();
        let [OutCall::Jev { id, route, body }] = out.as_slice() else {
            panic!("{out:?}");
        };
        assert_eq!(
            (route.as_str(), &body["wish"]),
            (JEV_LINES, &json!("heal me"))
        );
        hand.answered(*id, true, json!({ "lines": ["a", "b"] }), 1.0);
        assert_eq!(
            hand.new_answers(Asker::Macros),
            vec![Answer::Lines(Ok("a\nb".into()))]
        );
    }
}
