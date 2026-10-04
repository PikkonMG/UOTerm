//! The list of the party tab: the ten places of the party, then the people
//! near the character that the leader may invite.

use crate::frame::WatchFrame;
use crate::model::party::{leads, PARTY_PLACES};
use uoterm_assist::mobiles::is_humanoid;

/// People this near can be invited from the party tab.
pub const INVITE_TILES: u16 = 12;

/// One row of the list of the party tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyEntry {
    Place(usize),
    NearTitle,
    Near(u32),
}

/// The people near the character a player may invite: humans in sight who
/// are not in the party.
pub fn party_near(frame: &WatchFrame) -> Vec<u32> {
    frame
        .mobiles
        .iter()
        .filter(|mobile| {
            mobile.serial != frame.serial
                && mobile.dist <= INVITE_TILES
                && is_humanoid(mobile.look.body)
                && !frame
                    .party_members
                    .iter()
                    .any(|member| member.serial == mobile.serial)
        })
        .map(|mobile| mobile.serial)
        .collect()
}

/// The rows of the list: the ten places, then the people near to invite
/// when the character may add.
pub fn party_entries(frame: &WatchFrame) -> Vec<PartyEntry> {
    let mut rows: Vec<PartyEntry> = (0..PARTY_PLACES).map(PartyEntry::Place).collect();
    let near = party_near(frame);
    if leads(frame) && !near.is_empty() {
        rows.push(PartyEntry::NearTitle);
        rows.extend(near.into_iter().map(PartyEntry::Near));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchLook, WatchMobile, WatchPartyMember};

    const ME: u32 = 1;
    const BOB: u32 = 2;
    const HUMAN_BODY: u16 = 0x0190;

    #[test]
    fn the_list_holds_ten_places_then_the_people_near_for_the_leader() {
        let mut frame = WatchFrame {
            serial: ME,
            mobiles: vec![WatchMobile {
                serial: BOB,
                name: "Bob".into(),
                dist: 3,
                look: WatchLook {
                    body: HUMAN_BODY,
                    ..WatchLook::default()
                },
                ..WatchMobile::default()
            }],
            ..WatchFrame::default()
        };
        let rows = party_entries(&frame);
        assert_eq!(rows.len(), PARTY_PLACES + 2);
        assert_eq!(rows[PARTY_PLACES], PartyEntry::NearTitle);
        assert_eq!(rows[PARTY_PLACES + 1], PartyEntry::Near(BOB));
        frame.party_members = vec![
            WatchPartyMember {
                serial: BOB,
                ..WatchPartyMember::default()
            },
            WatchPartyMember {
                serial: ME,
                ..WatchPartyMember::default()
            },
        ];
        assert_eq!(
            party_entries(&frame).len(),
            PARTY_PLACES,
            "a member does not add"
        );
    }
}
