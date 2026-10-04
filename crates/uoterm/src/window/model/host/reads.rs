//! The worker that makes the session reads of the panels. The panels want
//! reads from the cache (`uoterm_view::model::reads`); once in each frame
//! the window hands the due reads to a worker thread, which makes each
//! call, so a panel never waits.

use crate::window::link::Link;
use serde_json::Value;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use uoterm_view::model::reads::{ReadCache, ReadKey};

/// A read made, with its answer.
type Answered = (ReadKey, Result<Value, String>);

pub struct Readings {
    cache: ReadCache,
    asks: Sender<ReadKey>,
    answers: Receiver<Answered>,
    time: f64,
}

impl Readings {
    /// Starts the worker that makes the calls over `link`.
    pub fn start(link: Link) -> Self {
        let (asks, inbox) = mpsc::channel::<ReadKey>();
        let (outbox, answers) = mpsc::channel::<Answered>();
        thread::spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            for key in inbox {
                let answer = rt.block_on(link.call(key.tool, key.arguments()));
                if outbox.send((key, answer)).is_err() {
                    return;
                }
            }
        });
        Self::with_channels(asks, answers)
    }

    fn with_channels(asks: Sender<ReadKey>, answers: Receiver<Answered>) -> Self {
        Self {
            cache: ReadCache::default(),
            asks,
            answers,
            time: 0.0,
        }
    }

    /// Call this once in each frame, before the panels read. It takes the
    /// answers that came.
    pub fn begin(&mut self, time: f64) {
        self.time = time;
        for (key, answer) in self.answers.try_iter() {
            self.cache.arrived(key, answer, time);
        }
    }

    /// The cache the panels want their reads from.
    pub fn cache(&mut self) -> &mut ReadCache {
        &mut self.cache
    }

    /// Call this once in each frame, after the panels read. It asks for
    /// the reads they wanted that are due.
    pub fn ask_due(&mut self) {
        for key in self.cache.due(self.time) {
            if self.asks.send(key).is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TOOL: &str = "agents";
    const MAX_AGE: f64 = 2.0;

    #[test]
    fn the_due_reads_go_to_the_worker_and_its_answers_come_back() {
        let (asks, inbox) = mpsc::channel();
        let (outbox, answers) = mpsc::channel();
        let mut readings = Readings::with_channels(asks, answers);
        let key = ReadKey::new(TOOL, &json!({ "serial": 5 }));
        readings.begin(0.0);
        assert!(readings.cache().want(key.clone(), MAX_AGE).is_none());
        assert!(
            inbox.try_recv().is_err(),
            "nothing goes before the frame ends"
        );
        readings.ask_due();
        let asked = inbox.try_recv().unwrap();
        assert_eq!(asked.arguments(), json!({ "serial": 5 }));
        outbox.send((asked, Ok(json!({ "on": true })))).unwrap();
        readings.begin(1.0);
        assert_eq!(
            readings.cache().want(key, MAX_AGE),
            Some(&json!({ "on": true }))
        );
        readings.ask_due();
        assert!(
            inbox.try_recv().is_err(),
            "a fresh answer is not asked again"
        );
    }
}
