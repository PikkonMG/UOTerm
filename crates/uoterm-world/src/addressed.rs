//! Lines where another character speaks to this one by name, so the agent
//! can answer like a person would.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use uoterm_protocol::Serial;

/// How many lines spoken to the character the world keeps.
pub const SPOKEN_TO_KEEP: usize = 5;
/// How long a line spoken to the character stays in the observation.
pub const SPOKEN_TO_FRESH_MS: u64 = 60_000;
/// The agent answers in a few friendly words and says no to every plan.
pub const CHAT_MODE_BASIC: &str = "basic";
/// The agent may also party up with, follow and fight beside a player who
/// spoke to the character.
pub const CHAT_MODE_PLAY_ALONG: &str = "play_along";
/// A first name shorter than this is too common a word to count alone.
const FIRST_NAME_MIN: usize = 3;
/// Words that ask whether the character is played by a program.
const BOT_WORDS: [&str; 8] = [
    "bot",
    "bots",
    "botting",
    "macro",
    "macroing",
    "afk",
    "script",
    "scripting",
];
/// Word pairs that ask the same.
const BOT_PHRASES: [&str; 3] = ["you real", "are you human", "a real person"];

/// A line another character said to this one by name.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpokenTo {
    pub serial: Serial,
    pub name: String,
    pub text: String,
    /// The line asks whether the character is a bot or a macro.
    pub asks_if_bot: bool,
    pub unix_ms: u64,
}

/// The lines spoken to the character, newest last.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpokenToLog {
    lines: VecDeque<SpokenTo>,
    /// How many lines were ever pushed, and how many of those the
    /// character has answered. A count, not a time: two lines can share a
    /// millisecond.
    pushed: u64,
    answered: u64,
}

impl SpokenToLog {
    pub fn push(&mut self, line: SpokenTo) {
        self.lines.push_back(line);
        self.pushed += 1;
        while self.lines.len() > SPOKEN_TO_KEEP {
            self.lines.pop_front();
        }
    }

    /// The lines said in the last [`SPOKEN_TO_FRESH_MS`] before `now_ms`.
    pub fn fresh(&self, now_ms: u64) -> Vec<SpokenTo> {
        self.lines
            .iter()
            .filter(|l| now_ms.saturating_sub(l.unix_ms) < SPOKEN_TO_FRESH_MS)
            .cloned()
            .collect()
    }

    /// The fresh lines the character has not answered yet.
    pub fn unanswered(&self, now_ms: u64) -> Vec<SpokenTo> {
        let first = self.pushed - self.lines.len() as u64;
        let skip = self.answered.saturating_sub(first) as usize;
        self.lines
            .iter()
            .skip(skip)
            .filter(|l| now_ms.saturating_sub(l.unix_ms) < SPOKEN_TO_FRESH_MS)
            .cloned()
            .collect()
    }

    /// True when this mobile said the character's name in the last
    /// [`SPOKEN_TO_FRESH_MS`] before `now_ms`.
    pub fn asked_by(&self, serial: Serial, now_ms: u64) -> bool {
        self.fresh(now_ms).iter().any(|l| l.serial == serial)
    }

    /// The character spoke: every line so far counts as answered.
    pub fn mark_answered(&mut self) {
        self.answered = self.pushed;
    }
}

/// The words of a line, lower case, split at anything that is not a letter,
/// digit or apostrophe.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '\''))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// True when `text` holds the character's name as whole words: the full
/// name, or its first word when that is long enough. "Tamara" does not
/// name "Mara".
pub fn names_character(text: &str, name: &str) -> bool {
    let said = words(text);
    let name = words(name);
    let Some(first) = name.first() else {
        return false;
    };
    let full = !said.is_empty() && said.windows(name.len()).any(|w| w == name.as_slice());
    full || (first.chars().count() >= FIRST_NAME_MIN && said.contains(first))
}

/// True when the line asks whether the character is a bot or a macro.
pub fn asks_if_bot(text: &str) -> bool {
    let said = words(text);
    // Spaces at both ends make each phrase match whole words only.
    let padded = format!(" {} ", said.join(" "));
    said.iter().any(|w| BOT_WORDS.contains(&w.as_str()))
        || BOT_PHRASES
            .iter()
            .any(|p| padded.contains(&format!(" {p} ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_counts_only_as_a_whole_word() {
        assert!(names_character("hey Mara, what are you doing?", "Mara"));
        assert!(names_character("MARA!", "Mara"));
        assert!(!names_character("Tamara is here", "Mara"));
        assert!(!names_character("hello there", "Mara"));
        assert!(!names_character("hi", ""));
    }

    #[test]
    fn a_long_name_counts_whole_or_by_its_first_word() {
        assert!(names_character("hail Mara of Yew", "Mara of Yew"));
        assert!(names_character("mara, come here", "Mara of Yew"));
        assert!(
            !names_character("of course", "Of Yew"),
            "a short first word is too common"
        );
    }

    #[test]
    fn bot_questions_are_seen() {
        assert!(asks_if_bot("mara are you a bot?"));
        assert!(asks_if_bot("u macroing?"));
        assert!(asks_if_bot("are you real"));
        assert!(asks_if_bot("mara are you human"));
        assert!(!asks_if_bot("mara nice robot costume"));
        assert!(!asks_if_bot("want to hunt?"));
    }

    #[test]
    fn the_log_keeps_the_newest_lines_while_they_are_fresh() {
        let mut log = SpokenToLog::default();
        for i in 0..(SPOKEN_TO_KEEP as u64 + 2) {
            log.push(SpokenTo {
                serial: Serial(1),
                name: "Ann".into(),
                text: format!("mara {i}"),
                asks_if_bot: false,
                unix_ms: i,
            });
        }
        let fresh = log.fresh(SPOKEN_TO_KEEP as u64);
        assert_eq!(fresh.len(), SPOKEN_TO_KEEP);
        assert_eq!(fresh.last().map(|l| l.text.as_str()), Some("mara 6"));
        assert!(log
            .fresh(SPOKEN_TO_FRESH_MS + SPOKEN_TO_KEEP as u64 + 2)
            .is_empty());
    }

    #[test]
    fn a_reply_answers_every_line_so_far() {
        const NOW: u64 = 10;
        let line = |text: &str, unix_ms: u64| SpokenTo {
            serial: Serial(1),
            name: "Ann".into(),
            text: text.into(),
            asks_if_bot: false,
            unix_ms,
        };
        let mut log = SpokenToLog::default();
        log.push(line("mara?", 1));
        log.push(line("mara, hello", 2));
        assert_eq!(log.unanswered(NOW).len(), 2);
        log.mark_answered();
        assert!(log.unanswered(NOW).is_empty());
        assert_eq!(log.fresh(NOW).len(), 2, "observe still shows them");
        log.push(line("mara, again", 2));
        assert_eq!(log.unanswered(NOW).len(), 1);
    }
}
