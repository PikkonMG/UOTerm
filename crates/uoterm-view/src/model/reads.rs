//! The session reads of the panels: the agents and their settings, the
//! damage meter, the properties of items, the named places. A panel wants a
//! read and shows its newest answer; the host makes the calls the cache
//! names as due and gives back each answer, so a panel never waits. A read
//! is asked again when its answer is old.

use serde_json::Value;
use std::collections::{HashMap, HashSet};

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

    /// The arguments as JSON again, for the call. The text is JSON that
    /// `new` wrote, so it always reads.
    pub fn arguments(&self) -> Value {
        serde_json::from_str(&self.args).unwrap_or_default()
    }
}

/// The newest answer of one read, and when it came, in seconds of the
/// caller's clock.
struct Kept {
    answer: Result<Value, String>,
    at: f64,
}

/// The answers of the reads, and which reads are on their way.
#[derive(Default)]
pub struct ReadCache {
    kept: HashMap<ReadKey, Kept>,
    in_flight: HashSet<ReadKey>,
    /// Reads to make again at the next want, whatever their age.
    stale: HashSet<ReadKey>,
    /// The reads wanted since the last `due`, each with the oldest answer
    /// it takes, in seconds.
    wanted: HashMap<ReadKey, f64>,
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

impl ReadCache {
    /// The newest good answer of a read. The read is due again when its
    /// answer is older than `max_age` seconds.
    pub fn want(&mut self, key: ReadKey, max_age: f64) -> Option<&Value> {
        let oldest = self.wanted.entry(key.clone()).or_insert(max_age);
        *oldest = oldest.min(max_age);
        self.value(&key)
    }

    /// The newest good answer of a read, with no want.
    pub fn value(&self, key: &ReadKey) -> Option<&Value> {
        self.kept.get(key)?.answer.as_ref().ok()
    }

    /// The words of the newest failed answer of a read, if it failed.
    pub fn failure(&self, key: &ReadKey) -> Option<&str> {
        self.kept
            .get(key)?
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

    /// The reads to make now: each read wanted since the last call that is
    /// not on its way and has no fresh answer. They count as on their way
    /// from now.
    pub fn due(&mut self, now: f64) -> Vec<ReadKey> {
        let mut due = Vec::new();
        for (key, max_age) in std::mem::take(&mut self.wanted) {
            let kept_at = self.kept.get(&key).map(|kept| kept.at);
            if needs_asking(
                kept_at,
                self.in_flight.contains(&key),
                self.stale.contains(&key),
                now,
                max_age,
            ) {
                self.stale.remove(&key);
                self.in_flight.insert(key.clone());
                due.push(key);
            }
        }
        due
    }

    /// Keeps the answer of a read made.
    pub fn arrived(&mut self, key: ReadKey, answer: Result<Value, String>, now: f64) {
        self.in_flight.remove(&key);
        self.kept.insert(key, Kept { answer, at: now });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const TOOL: &str = "agents";
    const MAX_AGE: f64 = 2.0;

    fn key() -> ReadKey {
        ReadKey::new(TOOL, &json!({}))
    }

    #[test]
    fn a_read_is_asked_once_and_again_when_its_answer_is_old() {
        let mut cache = ReadCache::default();
        assert!(cache.want(key(), MAX_AGE).is_none());
        assert!(cache.want(key(), MAX_AGE).is_none());
        assert_eq!(cache.due(0.0), vec![key()], "one ask for two wants");
        cache.want(key(), MAX_AGE);
        assert!(cache.due(0.5).is_empty(), "one ask while one is on its way");
        cache.arrived(key(), Ok(json!({ "on": ["bandage"] })), 1.0);
        assert_eq!(
            cache.want(key(), MAX_AGE),
            Some(&json!({ "on": ["bandage"] }))
        );
        assert!(
            cache.due(1.0).is_empty(),
            "a fresh answer is not asked again"
        );
        assert!(cache.want(key(), MAX_AGE).is_some(), "the old answer shows");
        assert_eq!(
            cache.due(1.0 + MAX_AGE),
            vec![key()],
            "and it is asked again"
        );
    }

    #[test]
    fn a_refreshed_read_is_asked_again_at_once_and_a_failure_keeps_its_words() {
        let mut cache = ReadCache::default();
        cache.want(key(), MAX_AGE);
        cache.due(0.0);
        cache.arrived(key(), Err("no session".into()), 0.5);
        assert!(cache.want(key(), MAX_AGE).is_none());
        assert_eq!(cache.failure(&key()), Some("no session"));
        assert!(cache.due(0.5).is_empty());
        cache.refresh(TOOL);
        cache.want(key(), MAX_AGE);
        assert_eq!(cache.due(0.5), vec![key()]);
    }

    #[test]
    fn a_read_not_wanted_is_not_due() {
        let mut cache = ReadCache::default();
        cache.want(key(), MAX_AGE);
        cache.due(0.0);
        cache.arrived(key(), Ok(json!({})), 0.0);
        assert!(cache.due(10.0).is_empty(), "no panel wanted it again");
    }

    #[test]
    fn the_arguments_come_back_as_json() {
        let args = json!({ "serial": 5 });
        assert_eq!(ReadKey::new(TOOL, &args).arguments(), args);
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

#[cfg(test)]
mod cache_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_wanted_read_is_due_once_until_it_arrives() {
        let mut cache = ReadCache::default();
        let key = ReadKey::new("damage_meter", &json!({}));
        cache.want(key.clone(), 0.0);
        assert_eq!(cache.due(0.0), vec![key.clone()]);
        assert!(cache.due(0.1).is_empty());
        cache.arrived(key.clone(), Ok(json!({"total": 3})), 0.2);
        assert_eq!(cache.value(&key), Some(&json!({"total": 3})));
    }
}
