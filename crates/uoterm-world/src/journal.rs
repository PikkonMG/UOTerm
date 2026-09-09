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
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Journal {
    entries: Vec<JournalEntry>,
}

impl Journal {
    pub fn push(&mut self, entry: JournalEntry) {
        self.entries.push(entry);
        if self.entries.len() > JOURNAL_CAP {
            let extra = self.entries.len() - JOURNAL_CAP;
            self.entries.drain(..extra);
        }
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
            .filter(|e| {
                e.text.to_ascii_lowercase().contains(&n) || e.name.to_ascii_lowercase().contains(&n)
            })
            .cloned()
            .collect()
    }
}
