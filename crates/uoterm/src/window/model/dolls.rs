//! Which paperdoll the shard just opened: each one it sends after the first
//! picture opens a paperdoll window, in the Modern and the Classic style.
//! What the buttons of a paperdoll send, and who may dress a paperdoll.

use crate::view::{WatchFrame, WatchPaperdoll};

/// The script commands of the Quests and Guild buttons of the character's
/// own paperdoll.
pub const QUESTS_COMMAND: &str = "questsbutton";
pub const GUILD_COMMAND: &str = "guildbutton";

/// True when the character may take items off a paperdoll and put them
/// on: his own, or one the shard lets him dress, while he lives.
pub fn dresses(frame: &WatchFrame, serial: u32, can_lift: bool) -> bool {
    (serial == frame.serial || can_lift) && !frame.dead
}

/// The count of the last paperdoll seen. None before the first picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DollWatch {
    seen: Option<u64>,
}

impl DollWatch {
    /// The paperdoll the shard sent since the last frame. One that came
    /// before the window opened is old. The count starts again when the
    /// session logs in again, so any change is a new paperdoll.
    pub fn take<'a>(&mut self, frame: &'a WatchFrame) -> Option<&'a WatchPaperdoll> {
        let newest = frame.paperdoll.as_ref();
        let fresh = match (self.seen, newest) {
            (Some(seen), Some(doll)) if doll.seq != seen => Some(doll),
            _ => None,
        };
        self.seen = newest.map(|doll| doll.seq).or(self.seen).or(Some(0));
        fresh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRANGER: u32 = 0x0000_0A11;

    fn doll(seq: u64) -> WatchPaperdoll {
        WatchPaperdoll {
            serial: STRANGER,
            text: "Someone the Brave".into(),
            seq,
            ..WatchPaperdoll::default()
        }
    }

    #[test]
    fn the_character_dresses_his_own_doll_and_one_the_shard_lets_him() {
        const ME: u32 = 0x0000_0001;
        let mut frame = WatchFrame {
            serial: ME,
            ..WatchFrame::default()
        };
        assert!(dresses(&frame, ME, false));
        assert!(!dresses(&frame, STRANGER, false));
        assert!(dresses(&frame, STRANGER, true));
        frame.dead = true;
        assert!(!dresses(&frame, ME, false), "a ghost dresses nobody");
    }

    #[test]
    fn a_paperdoll_opens_when_the_shard_sends_one_after_the_first_picture() {
        let mut watch = DollWatch::default();
        let mut frame = WatchFrame {
            paperdoll: Some(doll(3)),
            ..WatchFrame::default()
        };
        assert_eq!(
            watch.take(&frame),
            None,
            "one from before the window is old"
        );
        frame.paperdoll = Some(doll(4));
        assert_eq!(watch.take(&frame).map(|d| d.serial), Some(STRANGER));
        assert_eq!(watch.take(&frame), None, "a doll opens once");
        frame.paperdoll = Some(doll(1));
        assert_eq!(
            watch.take(&frame).map(|d| d.serial),
            Some(STRANGER),
            "after a new login the count starts again"
        );
    }
}
