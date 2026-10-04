//! The selected target of the official client: the select actions step
//! through the things in view, nearest first, and the actions on the
//! selected target use it.

use super::{SelectHow, SelectKind};
use crate::view::WatchFrame;
use uoterm_protocol::types::{NOTO_ATTACKABLE, NOTO_ENEMY, NOTO_INVULNERABLE};

/// A follower of these kinds is not picked: the official client leaves out
/// a pet that turned on the character and one the shard keeps safe.
const NOTO_NOT_FOLLOWING: [u8; 2] = [NOTO_INVULNERABLE, NOTO_ENEMY];

/// The things of a kind in view, nearest first, with their names.
fn candidates(frame: &WatchFrame, kind: SelectKind) -> Vec<(u32, String)> {
    let here = (frame.x, frame.y);
    let away = |x: u16, y: u16| here.0.abs_diff(x).max(here.1.abs_diff(y));
    let in_party = |serial: u32| frame.party_members.iter().any(|m| m.serial == serial);
    let mut found: Vec<(u16, u32, String)> = match kind {
        SelectKind::Object => frame
            .items
            .iter()
            .map(|item| (away(item.x, item.y), item.serial, item.name.clone()))
            .collect(),
        _ => frame
            .mobiles
            .iter()
            .filter(|mobile| mobile.serial != frame.serial)
            .filter(|mobile| match kind {
                SelectKind::Hostile => NOTO_ATTACKABLE.contains(&mobile.notoriety),
                SelectKind::Party => in_party(mobile.serial),
                SelectKind::Follower => {
                    mobile.follower && !NOTO_NOT_FOLLOWING.contains(&mobile.notoriety)
                }
                SelectKind::Mobile | SelectKind::Object => true,
            })
            .map(|mobile| (away(mobile.x, mobile.y), mobile.serial, mobile.name.clone()))
            .collect(),
    };
    found.sort_by_key(|(distance, serial, _)| (*distance, *serial));
    found
        .into_iter()
        .map(|(_, serial, name)| (serial, name))
        .collect()
}

/// The thing a select action picks, with its name. `current` is the
/// selected target before it.
pub fn select(
    frame: &WatchFrame,
    how: SelectHow,
    kind: SelectKind,
    current: Option<u32>,
) -> Option<(u32, String)> {
    let found = candidates(frame, kind);
    let count = found.len();
    let at = current.and_then(|serial| found.iter().position(|(s, _)| *s == serial));
    let pick = match (how, at) {
        (_, _) if count == 0 => return None,
        (SelectHow::Nearest, _) | (_, None) => 0,
        (SelectHow::Next, Some(at)) => (at + 1) % count,
        (SelectHow::Previous, Some(at)) => (at + count - 1) % count,
    };
    found.into_iter().nth(pick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchMobile, WatchPartyMember};
    use uoterm_protocol::types::{NOTO_INNOCENT, NOTO_MURDERER};

    fn mobile(serial: u32, x: u16, notoriety: u8) -> WatchMobile {
        WatchMobile {
            serial,
            x,
            y: 0,
            notoriety,
            name: format!("m{serial}"),
            ..WatchMobile::default()
        }
    }

    #[test]
    fn select_steps_through_the_kind_nearest_first_and_goes_round() {
        let frame = WatchFrame {
            serial: 1,
            mobiles: vec![
                mobile(1, 0, NOTO_INNOCENT),
                mobile(5, 4, NOTO_MURDERER),
                mobile(3, 2, NOTO_MURDERER),
                mobile(4, 1, NOTO_INNOCENT),
                WatchMobile {
                    follower: true,
                    ..mobile(6, 6, NOTO_INNOCENT)
                },
                WatchMobile {
                    follower: true,
                    ..mobile(7, 3, NOTO_INVULNERABLE)
                },
            ],
            party_members: vec![WatchPartyMember {
                serial: 4,
                ..WatchPartyMember::default()
            }],
            ..WatchFrame::default()
        };
        let hostile = |how, current| {
            select(&frame, how, SelectKind::Hostile, current).map(|(serial, _)| serial)
        };
        assert_eq!(hostile(SelectHow::Nearest, None), Some(3));
        assert_eq!(hostile(SelectHow::Next, Some(3)), Some(5));
        assert_eq!(hostile(SelectHow::Next, Some(5)), Some(3));
        assert_eq!(hostile(SelectHow::Previous, Some(3)), Some(5));
        assert_eq!(
            select(&frame, SelectHow::Next, SelectKind::Party, None),
            Some((4, "m4".into()))
        );
        assert_eq!(
            select(&frame, SelectHow::Next, SelectKind::Follower, None),
            Some((6, "m6".into()))
        );
        assert_eq!(
            select(&frame, SelectHow::Next, SelectKind::Object, None),
            None
        );
        let all: Vec<u32> = candidates(&frame, SelectKind::Mobile)
            .into_iter()
            .map(|(serial, _)| serial)
            .collect();
        assert_eq!(all, [4, 3, 7, 5, 6]);
    }
}
