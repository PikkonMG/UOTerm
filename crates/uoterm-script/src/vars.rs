//! What scripts keep between lines and between scripts: aliases, lists and
//! timers. One set serves every script of a character, the way players
//! expect an alias set in one script to work in the next.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Aliases, lists and timers, by lower-case name.
#[derive(Clone, Debug, Default)]
pub struct Vars {
    aliases: HashMap<String, u32>,
    lists: HashMap<String, Vec<String>>,
    /// When each timer was set, and the value it was set to.
    timers: HashMap<String, (Instant, Duration)>,
}

fn key(name: &str) -> String {
    name.to_ascii_lowercase()
}

impl Vars {
    pub fn alias(&self, name: &str) -> Option<u32> {
        self.aliases.get(&key(name)).copied()
    }

    pub fn set_alias(&mut self, name: &str, serial: u32) {
        self.aliases.insert(key(name), serial);
    }

    pub fn unset_alias(&mut self, name: &str) {
        self.aliases.remove(&key(name));
    }

    pub fn list(&self, name: &str) -> Option<&Vec<String>> {
        self.lists.get(&key(name))
    }

    /// Makes an empty list. A list that exists already is kept as it is.
    pub fn create_list(&mut self, name: &str) {
        self.lists.entry(key(name)).or_default();
    }

    pub fn remove_list(&mut self, name: &str) {
        self.lists.remove(&key(name));
    }

    /// The list with that name, made if it does not exist yet.
    pub fn list_mut(&mut self, name: &str) -> &mut Vec<String> {
        self.lists.entry(key(name)).or_default()
    }

    /// Sets a timer to `value`; it counts up from there.
    pub fn set_timer(&mut self, name: &str, value: Duration, now: Instant) {
        self.timers.insert(key(name), (now, value));
    }

    pub fn remove_timer(&mut self, name: &str) {
        self.timers.remove(&key(name));
    }

    /// How far a timer has counted, or `None` when there is no such timer.
    pub fn timer(&self, name: &str, now: Instant) -> Option<Duration> {
        self.timers
            .get(&key(name))
            .map(|&(set_at, value)| value + now.saturating_duration_since(set_at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_the_same_in_any_case() {
        let mut vars = Vars::default();
        vars.set_alias("Weapon", 0x4000_0001);
        assert_eq!(vars.alias("weapon"), Some(0x4000_0001));
        vars.list_mut("Fruit").push("apple".into());
        assert_eq!(vars.list("FRUIT").map(Vec::len), Some(1));
    }

    #[test]
    fn a_timer_counts_up_from_the_value_it_was_set_to() {
        const STARTED_AT: Duration = Duration::from_millis(500);
        const LATER: Duration = Duration::from_millis(1500);
        let now = Instant::now();
        let mut vars = Vars::default();
        vars.set_timer("band", STARTED_AT, now);
        assert_eq!(vars.timer("band", now), Some(STARTED_AT));
        assert_eq!(vars.timer("band", now + LATER), Some(STARTED_AT + LATER));
        vars.remove_timer("band");
        assert_eq!(vars.timer("band", now), None);
    }
}
