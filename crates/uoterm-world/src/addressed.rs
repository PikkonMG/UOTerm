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

/// Where a line was said. An answer goes back the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    Say,
    Whisper,
    Yell,
    Party,
    /// A party line sent to this character only.
    PartyPrivate,
    Guild,
    Alliance,
}

/// Who hears a line in a channel: the square, the party, the guild or the
/// alliance. A line said in one group answers the lines of that group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelGroup {
    Nearby,
    Party,
    Guild,
    Alliance,
}

impl Channel {
    pub fn group(self) -> ChannelGroup {
        match self {
            Self::Say | Self::Whisper | Self::Yell => ChannelGroup::Nearby,
            Self::Party | Self::PartyPrivate => ChannelGroup::Party,
            Self::Guild => ChannelGroup::Guild,
            Self::Alliance => ChannelGroup::Alliance,
        }
    }

    /// True for the channels that reach the character from anywhere, so
    /// the speaker need not be in sight.
    pub fn reaches_far(self) -> bool {
        matches!(
            self,
            Self::Party | Self::PartyPrivate | Self::Guild | Self::Alliance
        )
    }
}

/// A line another character said to this one by name.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpokenTo {
    pub serial: Serial,
    pub name: String,
    pub text: String,
    pub channel: Channel,
    /// The line asks whether the character is a bot or a macro.
    pub asks_if_bot: bool,
    pub unix_ms: u64,
}

/// The lines spoken to the character, newest last.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SpokenToLog {
    /// Each line, and whether the character has answered it.
    lines: VecDeque<(SpokenTo, bool)>,
}

impl SpokenToLog {
    pub fn push(&mut self, line: SpokenTo) {
        self.lines.push_back((line, false));
        while self.lines.len() > SPOKEN_TO_KEEP {
            self.lines.pop_front();
        }
    }

    /// The lines said in the last [`SPOKEN_TO_FRESH_MS`] before `now_ms`.
    pub fn fresh(&self, now_ms: u64) -> Vec<SpokenTo> {
        self.fresh_where(now_ms, |_| true)
    }

    /// The fresh lines the character has not answered yet.
    pub fn unanswered(&self, now_ms: u64) -> Vec<SpokenTo> {
        self.fresh_where(now_ms, |answered| !answered)
    }

    fn fresh_where(&self, now_ms: u64, keep: impl Fn(bool) -> bool) -> Vec<SpokenTo> {
        self.lines
            .iter()
            .filter(|(l, answered)| {
                keep(*answered) && now_ms.saturating_sub(l.unix_ms) < SPOKEN_TO_FRESH_MS
            })
            .map(|(l, _)| l.clone())
            .collect()
    }

    /// True when this mobile said the character's name in the last
    /// [`SPOKEN_TO_FRESH_MS`] before `now_ms`.
    pub fn asked_by(&self, serial: Serial, now_ms: u64) -> bool {
        self.fresh(now_ms).iter().any(|l| l.serial == serial)
    }

    /// Marks the lines an answer covers: those of one speaker for a reply,
    /// those of one channel group for a line said to everyone there.
    pub fn mark_answered(&mut self, covers: impl Fn(&SpokenTo) -> bool) {
        for (line, answered) in &mut self.lines {
            if covers(line) {
                *answered = true;
            }
        }
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
                channel: Channel::Say,
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

    /// Answering one person, or one channel, leaves the others waiting.
    #[test]
    fn an_answer_clears_only_the_lines_it_answers() {
        const NOW: u64 = 10;
        let line = |serial: u32, channel: Channel| SpokenTo {
            serial: Serial(serial),
            name: String::new(),
            text: "mara?".into(),
            channel,
            asks_if_bot: false,
            unix_ms: 1,
        };
        let mut log = SpokenToLog::default();
        log.push(line(1, Channel::Party));
        log.push(line(2, Channel::Say));
        log.push(line(3, Channel::Whisper));
        log.mark_answered(|l| l.serial == Serial(2));
        let left: Vec<u32> = log.unanswered(NOW).iter().map(|l| l.serial.0).collect();
        assert_eq!(left, vec![1, 3]);
        log.mark_answered(|l| l.channel.group() == Channel::Say.group());
        let left: Vec<u32> = log.unanswered(NOW).iter().map(|l| l.serial.0).collect();
        assert_eq!(left, vec![1], "a line in the square answers the square");
    }

    #[test]
    fn a_reply_answers_every_line_so_far() {
        const NOW: u64 = 10;
        let line = |text: &str, unix_ms: u64| SpokenTo {
            serial: Serial(1),
            name: "Ann".into(),
            text: text.into(),
            channel: Channel::Say,
            asks_if_bot: false,
            unix_ms,
        };
        let mut log = SpokenToLog::default();
        log.push(line("mara?", 1));
        log.push(line("mara, hello", 2));
        assert_eq!(log.unanswered(NOW).len(), 2);
        log.mark_answered(|_| true);
        assert!(log.unanswered(NOW).is_empty());
        assert_eq!(log.fresh(NOW).len(), 2, "observe still shows them");
        log.push(line("mara, again", 2));
        assert_eq!(log.unanswered(NOW).len(), 1);
    }
}
