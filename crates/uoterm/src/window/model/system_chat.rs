//! The lines the reference client's chat line shows over itself at the foot of the
//! game window, apart from how they draw: the shard's own words, the party,
//! guild and alliance lines, and the words the client prints, each for ten
//! seconds, at most thirty at a time.

use super::journal::NewLines;
use crate::view::{WatchFrame, WatchSpeech};
use crate::window::settings::SpeechOptions;
use std::collections::VecDeque;
use uoterm_protocol::types::{
    SPEECH_ALLIANCE, SPEECH_GUILD, SPEECH_LABEL, SPEECH_REGULAR, SPEECH_SYSTEM,
};
use uoterm_world::{SPEECH_KIND_PARTY, SPEECH_KIND_PARTY_PRIVATE};

/// A line shows this long, as the reference client's system message time.
const SHOW_SECONDS: f64 = 10.0;
/// At most this many lines show; a new one pushes the oldest out.
const MOST_LINES: usize = 30;
/// The name the shard gives its own words.
const SYSTEM_NAME: &str = "system";

/// One line over the chat line.
#[derive(Clone, Debug, PartialEq)]
pub struct ShownLine {
    pub text: String,
    pub hue: u16,
    /// When it came, in seconds of the window's clock.
    since: f64,
}

/// The lines over the chat line.
#[derive(Default)]
pub struct SystemChat {
    new_lines: NewLines,
    lines: VecDeque<ShownLine>,
}

impl SystemChat {
    /// Takes the new lines of a frame, and lets the old ones go.
    pub fn take(&mut self, frame: &WatchFrame, speech: &SpeechOptions, time: f64) {
        for line in self.new_lines.take(&frame.speech) {
            if let Some((text, hue)) = shown_words(line, speech) {
                self.lines.push_back(ShownLine {
                    text,
                    hue,
                    since: time,
                });
                if self.lines.len() > MOST_LINES {
                    self.lines.pop_front();
                }
            }
        }
        self.lines.retain(|line| time - line.since < SHOW_SECONDS);
    }

    /// The lines that show, the oldest first.
    pub fn lines(&self) -> impl DoubleEndedIterator<Item = &ShownLine> {
        self.lines.iter()
    }

    /// True while a line shows, so the window keeps drawing to let it go.
    pub fn showing(&self) -> bool {
        !self.lines.is_empty()
    }
}

/// The words and the hue a journal line shows with over the chat line, as
/// the reference client's system chat takes a message. None for a line said by a
/// mobile or an item in the world, which floats over it instead.
fn shown_words(line: &WatchSpeech, speech: &SpeechOptions) -> Option<(String, u16)> {
    let from_shard = line.serial == 0;
    let named = !line.name.is_empty() && !line.name.eq_ignore_ascii_case(SYSTEM_NAME);
    let tagged = |tag: &str, hue: u16| (format!("[{tag}][{}]: {}", line.name, line.text), hue);
    match line.kind {
        SPEECH_SYSTEM | SPEECH_REGULAR if from_shard || line.kind == SPEECH_SYSTEM => {
            let text = if named {
                format!("{}: {}", line.name, line.text)
            } else {
                line.text.clone()
            };
            Some((text, line.hue))
        }
        SPEECH_LABEL if from_shard => Some((line.text.clone(), line.hue)),
        SPEECH_KIND_PARTY | SPEECH_KIND_PARTY_PRIVATE => Some(tagged("Party", speech.party_hue)),
        SPEECH_GUILD => Some(tagged("Guild", speech.guild_hue)),
        SPEECH_ALLIANCE => Some(tagged("Alliance", speech.alliance_hue)),
        _ if from_shard && !named => Some((line.text.clone(), line.hue)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_protocol::types::SPEECH_YELL;

    fn line(seq: u64, serial: u32, name: &str, kind: u8, text: &str) -> WatchSpeech {
        WatchSpeech {
            seq,
            serial,
            name: name.into(),
            kind,
            hue: 0x0035,
            text: text.into(),
        }
    }

    #[test]
    fn the_shard_and_the_channels_show_and_the_world_does_not() {
        let speech = SpeechOptions::default();
        let words = |line: &WatchSpeech| shown_words(line, &speech).map(|(text, _)| text);
        let system = line(1, 0, "System", SPEECH_SYSTEM, "The world will save.");
        assert_eq!(words(&system).as_deref(), Some("The world will save."));
        let named = line(1, 0, "Lord British", SPEECH_REGULAR, "Welcome");
        assert_eq!(words(&named).as_deref(), Some("Lord British: Welcome"));
        let party = line(1, 5, "Ann", SPEECH_KIND_PARTY, "heal me");
        assert_eq!(
            shown_words(&party, &speech),
            Some(("[Party][Ann]: heal me".into(), speech.party_hue))
        );
        let guild = line(1, 5, "Ann", SPEECH_GUILD, "meet");
        assert_eq!(words(&guild).as_deref(), Some("[Guild][Ann]: meet"));
        assert_eq!(words(&line(1, 5, "Ann", SPEECH_REGULAR, "hail")), None);
        assert_eq!(words(&line(1, 5, "Ann", SPEECH_YELL, "guards")), None);
        assert_eq!(
            words(&line(1, 0x4000_0001, "", SPEECH_LABEL, "a chest")),
            None
        );
    }

    #[test]
    fn a_line_shows_ten_seconds_and_thirty_at_most() {
        let speech = SpeechOptions::default();
        let mut chat = SystemChat::default();
        chat.take(&WatchFrame::default(), &speech, 0.0);
        let frame = WatchFrame {
            speech: (1..=40)
                .map(|seq| line(seq, 0, "", SPEECH_SYSTEM, &seq.to_string()))
                .collect(),
            ..WatchFrame::default()
        };
        chat.take(&frame, &speech, 1.0);
        assert_eq!(chat.lines().count(), MOST_LINES);
        assert_eq!(chat.lines().next().map(|l| l.text.as_str()), Some("11"));
        assert!(chat.showing());
        chat.take(&frame, &speech, 1.0 + SHOW_SECONDS);
        assert!(!chat.showing(), "gone after ten seconds");
    }
}
