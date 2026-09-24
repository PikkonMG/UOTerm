//! The party of the character, apart from how a window draws it: its ten
//! places, who leads it, what the leave button says, the invite the shard
//! sent, and the script lines that add a member and answer an invite. The
//! Classic party gumps and the Modern party tab both read it.

use crate::view::WatchFrame;

/// A party holds this many members.
pub const PARTY_PLACES: usize = 10;
/// The script command that gives the target cursor of a party invite.
pub const INVITE_COMMAND: &str = "partyinvite";
pub const ACCEPT_COMMAND: &str = "partyaccept";
pub const DECLINE_COMMAND: &str = "partydecline";
pub const LEAVE_WORDS: &str = "Leave the party";
pub const DISBAND_WORDS: &str = "Disband the party";
const NO_NAME: &str = "No Name";

/// The leader of the party: the first member the shard lists.
pub fn leader(frame: &WatchFrame) -> Option<u32> {
    frame.party_members.first().map(|member| member.serial)
}

/// The character may kick and add: he leads the party, or has none.
pub fn leads(frame: &WatchFrame) -> bool {
    leader(frame).is_none_or(|leader| leader == frame.serial)
}

/// The words of the leave button: a member leaves, the leader disbands.
pub fn leave_words(frame: &WatchFrame) -> &'static str {
    if !frame.party_members.is_empty() && !leads(frame) {
        LEAVE_WORDS
    } else {
        DISBAND_WORDS
    }
}

/// The name of the one who invites the character, when he is in sight.
pub fn inviter_name(frame: &WatchFrame, leader: u32) -> String {
    frame
        .mobiles
        .iter()
        .find(|mobile| mobile.serial == leader)
        .map(|mobile| mobile.name.clone())
        .unwrap_or_default()
}

/// The words of an invite: who invites the character.
pub fn invite_words(name: &str) -> String {
    let name = if name.is_empty() { NO_NAME } else { name };
    format!("{name} has invited you to join a party.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchMobile, WatchPartyMember};

    const ME: u32 = 1;
    const BOB: u32 = 2;

    fn member(serial: u32) -> WatchPartyMember {
        WatchPartyMember {
            serial,
            name: format!("member {serial}"),
            hits_percent: None,
            ..WatchPartyMember::default()
        }
    }

    #[test]
    fn the_first_member_leads_and_no_party_lets_the_character_add() {
        let mut frame = WatchFrame {
            serial: ME,
            ..WatchFrame::default()
        };
        assert!(leads(&frame));
        assert_eq!(leave_words(&frame), DISBAND_WORDS);
        frame.party_members = vec![member(BOB), member(ME)];
        assert!(!leads(&frame));
        assert_eq!(leave_words(&frame), LEAVE_WORDS);
        frame.party_members.reverse();
        assert!(leads(&frame));
        assert_eq!(leave_words(&frame), DISBAND_WORDS);
    }

    #[test]
    fn the_invite_names_the_leader_when_he_is_in_sight() {
        let mut frame = WatchFrame::default();
        assert_eq!(inviter_name(&frame, BOB), "");
        frame.mobiles.push(WatchMobile {
            serial: BOB,
            name: "Bob".into(),
            ..WatchMobile::default()
        });
        assert_eq!(inviter_name(&frame, BOB), "Bob");
        assert_eq!(invite_words("Bob"), "Bob has invited you to join a party.");
        assert_eq!(invite_words(""), "No Name has invited you to join a party.");
    }
}
