use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use uoterm_protocol::{Serial, SpeechLine};

pub const JOURNAL_CAP: usize = 400;
pub const JOURNAL_DEFAULT_WINDOW: Duration = Duration::from_secs(90);
pub const JOURNAL_RECENT_LINES: usize = 12;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    pub serial: Serial,
    pub name: String,
    pub hue: u16,
    pub kind: u8,
    pub text: String,
    #[serde(skip, default = "Instant::now")]
    pub at: Instant,
    /// The line's place in the order the journal heard it, counting up from
    /// one. A caller that remembers the last number it saw can ask for only
    /// the lines heard since, even after old lines are dropped off the end.
    #[serde(default)]
    pub seq: u64,
}

impl JournalEntry {
    /// True when the line's words or its speaker's name hold `needle`, which
    /// must already be lower case.
    pub fn mentions(&self, needle: &str) -> bool {
        self.text.to_ascii_lowercase().contains(needle)
            || self.name.to_ascii_lowercase().contains(needle)
    }
}

impl From<&SpeechLine> for JournalEntry {
    fn from(line: &SpeechLine) -> Self {
        Self {
            serial: line.serial,
            name: line.name.clone(),
            hue: line.hue,
            kind: line.kind,
            text: line.text.clone(),
            at: Instant::now(),
            seq: 0,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Journal {
    entries: Vec<JournalEntry>,
    /// The number the newest line was given, or zero before any line.
    #[serde(default)]
    last_seq: u64,
}

impl Journal {
    pub fn push(&mut self, mut entry: JournalEntry) {
        self.last_seq += 1;
        entry.seq = self.last_seq;
        self.entries.push(entry);
        if self.entries.len() > JOURNAL_CAP {
            let extra = self.entries.len() - JOURNAL_CAP;
            self.entries.drain(..extra);
        }
    }

    /// The number the newest line was given, or zero before any line.
    pub fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// The lines heard after the one numbered `seq`, oldest first.
    pub fn after(&self, seq: u64) -> impl Iterator<Item = &JournalEntry> {
        self.entries.iter().filter(move |e| e.seq > seq)
    }

    pub fn since(&self, window: Duration) -> impl Iterator<Item = &JournalEntry> {
        let now = Instant::now();
        self.entries
            .iter()
            .filter(move |e| now.saturating_duration_since(e.at) <= window)
    }

    pub fn last_lines(&self, n: usize) -> Vec<&JournalEntry> {
        let start = self.entries.len().saturating_sub(n);
        self.entries[start..].iter().collect()
    }

    pub fn recent_text(&self) -> Vec<String> {
        let lines: Vec<&JournalEntry> = self.since(JOURNAL_DEFAULT_WINDOW).collect();
        let start = lines.len().saturating_sub(JOURNAL_RECENT_LINES);
        lines[start..]
            .iter()
            .map(|e| {
                if e.name.is_empty() {
                    e.text.clone()
                } else {
                    format!("{}: {}", e.name, e.text)
                }
            })
            .collect()
    }

    pub fn search(&self, needle: &str) -> Vec<JournalEntry> {
        let n = needle.to_ascii_lowercase();
        self.entries
            .iter()
            .filter(|e| e.mentions(&n))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heard(text: &str) -> JournalEntry {
        JournalEntry {
            serial: Serial(0),
            name: String::new(),
            hue: 0,
            kind: 0,
            text: text.into(),
            at: Instant::now(),
            seq: 0,
        }
    }

    #[test]
    fn every_line_is_numbered_in_the_order_it_was_heard() {
        let mut j = Journal::default();
        assert_eq!(j.last_seq(), 0);
        j.push(heard("one"));
        j.push(heard("two"));
        assert_eq!(j.last_seq(), 2);
        let after_one: Vec<_> = j.after(1).map(|e| e.text.as_str()).collect();
        assert_eq!(after_one, ["two"]);
    }

    /// Numbers keep counting when old lines fall off the end, so a caller
    /// holding a number never gets old lines back as new.
    #[test]
    fn numbers_keep_counting_past_the_cap() {
        let mut j = Journal::default();
        for n in 0..JOURNAL_CAP + 5 {
            j.push(heard(&format!("line {n}")));
        }
        let newest = j.last_seq();
        assert_eq!(newest, (JOURNAL_CAP + 5) as u64);
        assert_eq!(j.after(newest - 1).count(), 1);
    }
}
