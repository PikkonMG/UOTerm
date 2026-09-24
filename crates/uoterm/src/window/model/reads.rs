//! The session read for the panels: the agents and their settings, the
//! damage meter, the properties of items, the named places. A worker thread
//! makes each call, so a panel never waits; it shows the newest answer and
//! asks again when that answer is old.

use crate::window::link::Link;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

/// One read: a tool and its arguments.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReadKey {
    pub tool: &'static str,
    /// The arguments as JSON text, so equal arguments are one key.
    pub args: String,
}

impl ReadKey {
    pub fn new(tool: &'static str, args: &Value) -> Self {
        Self {
            tool,
            args: args.to_string(),
        }
    }
}

/// The newest answer of one read, and when it came, in seconds of the
/// window's clock.
struct Kept {
    answer: Result<Value, String>,
    at: f64,
}

/// A read to make, with its arguments.
type Wanted = (ReadKey, Value);
/// A read made, with its answer.
type Answered = (ReadKey, Result<Value, String>);

pub struct Readings {
    asks: Sender<Wanted>,
    answers: Receiver<Answered>,
    kept: HashMap<ReadKey, Kept>,
    in_flight: HashSet<ReadKey>,
    /// Reads to make again at the next want, whatever their age.
    stale: HashSet<ReadKey>,
    time: f64,
}

/// True when a read must be asked for: it is not on its way, and it has no
/// answer yet, or an old one, or one marked stale.
fn needs_asking(
    kept_at: Option<f64>,
    in_flight: bool,
    stale: bool,
    time: f64,
    max_age: f64,
) -> bool {
    !in_flight && (stale || kept_at.is_none_or(|at| time - at >= max_age))
}

impl Readings {
    /// Starts the worker that makes the calls over `link`.
    pub fn start(link: Link) -> Self {
        let (asks, inbox) = mpsc::channel::<Wanted>();
        let (outbox, answers) = mpsc::channel::<Answered>();
        thread::spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            for (key, args) in inbox {
                let answer = rt.block_on(link.call(key.tool, args));
                if outbox.send((key, answer)).is_err() {
                    return;
                }
            }
        });
        Self::with_channels(asks, answers)
    }

    fn with_channels(asks: Sender<Wanted>, answers: Receiver<Answered>) -> Self {
        Self {
            asks,
            answers,
            kept: HashMap::new(),
            in_flight: HashSet::new(),
            stale: HashSet::new(),
            time: 0.0,
        }
    }

    /// Call this once in each frame, before the panels read. It takes the
    /// answers that came.
    pub fn begin(&mut self, time: f64) {
        self.time = time;
        for (key, answer) in self.answers.try_iter() {
            self.in_flight.remove(&key);
            self.kept.insert(key, Kept { answer, at: time });
        }
    }

    /// The newest good answer of a read. The read is asked for again when
    /// its answer is older than `max_age` seconds.
    pub fn want(&mut self, tool: &'static str, args: Value, max_age: f64) -> Option<&Value> {
        let key = ReadKey::new(tool, &args);
        let kept_at = self.kept.get(&key).map(|kept| kept.at);
        let stale = self.stale.contains(&key);
        if needs_asking(
            kept_at,
            self.in_flight.contains(&key),
            stale,
            self.time,
            max_age,
        ) && self.asks.send((key.clone(), args)).is_ok()
        {
            self.stale.remove(&key);
            self.in_flight.insert(key.clone());
        }
        self.kept.get(&key)?.answer.as_ref().ok()
    }

    /// The words of the newest failed answer of a read, if it failed.
    pub fn failure(&self, tool: &'static str, args: &Value) -> Option<&str> {
        self.kept
            .get(&ReadKey::new(tool, args))?
            .answer
            .as_ref()
            .err()
            .map(String::as_str)
    }

    /// Marks every read of a tool to be made again at its next want: an
    /// act just changed what it reads.
    pub fn refresh(&mut self, tool: &'static str) {
        let keys = self.kept.keys().filter(|key| key.tool == tool).cloned();
        self.stale.extend(keys.collect::<Vec<_>>());
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;

    const TOOL: &str = "agents";
    const MAX_AGE: f64 = 2.0;

    /// Readings whose calls a test sees and answers by hand.
    pub(crate) fn by_hand() -> (Readings, Receiver<Wanted>, Sender<Answered>) {
        let (asks, inbox) = mpsc::channel();
        let (outbox, answers) = mpsc::channel();
        (Readings::with_channels(asks, answers), inbox, outbox)
    }

    #[test]
    fn a_read_is_asked_once_and_again_when_its_answer_is_old() {
        let (mut readings, inbox, outbox) = by_hand();
        readings.begin(0.0);
        assert!(readings.want(TOOL, json!({}), MAX_AGE).is_none());
        assert!(readings.want(TOOL, json!({}), MAX_AGE).is_none());
        let (key, _) = inbox.try_recv().unwrap();
        assert!(inbox.try_recv().is_err(), "one ask while one is on its way");
        outbox
            .send((key, Ok(json!({ "on": ["bandage"] }))))
            .unwrap();
        readings.begin(1.0);
        assert_eq!(
            readings.want(TOOL, json!({}), MAX_AGE),
            Some(&json!({ "on": ["bandage"] }))
        );
        assert!(
            inbox.try_recv().is_err(),
            "a fresh answer is not asked again"
        );
        readings.begin(1.0 + MAX_AGE);
        assert!(
            readings.want(TOOL, json!({}), MAX_AGE).is_some(),
            "the old answer shows"
        );
        assert!(inbox.try_recv().is_ok(), "and it is asked again");
    }

    #[test]
    fn a_refreshed_read_is_asked_again_at_once_and_a_failure_keeps_its_words() {
        let (mut readings, inbox, outbox) = by_hand();
        readings.begin(0.0);
        readings.want(TOOL, json!({}), MAX_AGE);
        let (key, _) = inbox.try_recv().unwrap();
        outbox.send((key, Err("no session".into()))).unwrap();
        readings.begin(0.5);
        assert!(readings.want(TOOL, json!({}), MAX_AGE).is_none());
        assert_eq!(readings.failure(TOOL, &json!({})), Some("no session"));
        readings.refresh(TOOL);
        readings.want(TOOL, json!({}), MAX_AGE);
        assert!(inbox.try_recv().is_ok());
    }

    #[test]
    fn only_a_read_with_no_fresh_answer_on_its_way_is_asked() {
        assert!(needs_asking(None, false, false, 0.0, MAX_AGE));
        assert!(!needs_asking(None, true, false, 0.0, MAX_AGE));
        assert!(!needs_asking(Some(1.0), false, false, 2.0, MAX_AGE));
        assert!(needs_asking(Some(1.0), false, true, 2.0, MAX_AGE));
        assert!(needs_asking(Some(1.0), false, false, 3.0, MAX_AGE));
    }
}
