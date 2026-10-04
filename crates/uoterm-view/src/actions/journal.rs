//! The window's own lines in the journal, as the official client prints
//! them: where a screenshot went, that always run is on. The session never
//! hears of them, so the window lays them into each picture it gets, after
//! the line that was newest when they were written.

use crate::view::{WatchFrame, WatchSpeech};

/// The message type of a system line.
const KIND_SYSTEM: u8 = 1;
/// The window keeps at most this many of its own lines.
const MOST_LINES: usize = 50;

#[derive(Clone, Debug, PartialEq, Eq)]
struct OwnLine {
    /// The newest line of the session when this one was written.
    after_seq: u64,
    text: String,
}

#[derive(Default)]
pub struct ClientJournal {
    lines: Vec<OwnLine>,
}

fn newest_seq(frame: &WatchFrame) -> u64 {
    frame.speech.iter().map(|line| line.seq).max().unwrap_or(0)
}

impl ClientJournal {
    /// Writes a line after the newest line of the picture.
    pub fn print(&mut self, frame: &WatchFrame, text: impl Into<String>) {
        self.lines.push(OwnLine {
            after_seq: newest_seq(frame),
            text: text.into(),
        });
        if self.lines.len() > MOST_LINES {
            self.lines.remove(0);
        }
    }

    /// Lays the kept lines into a new picture. A line older than the
    /// oldest line of the picture has scrolled away and is forgotten.
    pub fn lay_into(&mut self, frame: &mut WatchFrame) {
        if frame.speech.is_empty() {
            frame
                .journal
                .extend(self.lines.iter().map(|line| line.text.clone()));
            return;
        }
        let oldest = frame.speech.iter().map(|line| line.seq).min().unwrap_or(0);
        self.lines.retain(|line| line.after_seq >= oldest);
        for line in &self.lines {
            let at = frame
                .speech
                .iter()
                .rposition(|said| said.seq <= line.after_seq)
                .map_or(0, |place| place + 1);
            frame.speech.insert(
                at,
                WatchSpeech {
                    seq: line.after_seq,
                    kind: KIND_SYSTEM,
                    text: line.text.clone(),
                    ..WatchSpeech::default()
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn said(seq: u64, text: &str) -> WatchSpeech {
        WatchSpeech {
            seq,
            serial: 7,
            text: text.into(),
            ..WatchSpeech::default()
        }
    }

    #[test]
    fn an_own_line_stays_after_the_line_that_was_newest() {
        let mut journal = ClientJournal::default();
        let first = WatchFrame {
            speech: vec![said(1, "hail"), said(2, "well met")],
            ..WatchFrame::default()
        };
        journal.print(&first, "Always run is now on.");
        let mut next = WatchFrame {
            speech: vec![said(2, "well met"), said(3, "bye")],
            ..WatchFrame::default()
        };
        journal.lay_into(&mut next);
        let texts: Vec<&str> = next.speech.iter().map(|l| l.text.as_str()).collect();
        assert_eq!(texts, ["well met", "Always run is now on.", "bye"]);
        assert_eq!(next.speech[1].serial, 0);
        let mut later = WatchFrame {
            speech: vec![said(9, "later")],
            ..WatchFrame::default()
        };
        journal.lay_into(&mut later);
        assert_eq!(later.speech.len(), 1);
    }

    #[test]
    fn with_no_speech_lines_the_plain_journal_gets_the_line() {
        let mut journal = ClientJournal::default();
        let mut frame = WatchFrame {
            journal: vec!["old".into()],
            ..WatchFrame::default()
        };
        journal.print(&frame, "Screenshot saved.");
        journal.lay_into(&mut frame);
        assert_eq!(frame.journal, ["old", "Screenshot saved."]);
    }
}
