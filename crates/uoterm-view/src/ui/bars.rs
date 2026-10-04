//! The rules of the health bars of the Modern style: the ids a bar keeps
//! its place and its being open under, the sizes of the near list and of
//! a bar, the party buttons, the color of the hits, and which bars come
//! back with the game and which a close shuts. What each bar shows is
//! `model::health_bars`'.

use super::places::TITLE_ROW;
use super::theme::{BAR_HEIGHT, HITS, HITS_POISONED, PANEL_PAD, STAM};
use crate::frame::{WatchFrame, MOBILE_LINES};
use crate::geom::{Rgba, Vector};
use crate::model::health_bars::{BarFacts, Subject};
use crate::model::places;
use crate::settings::Profile;

pub const NEAR_ID: &str = "modern:near";
const BAR_ID_PREFIX: &str = "modern:bar:";
/// The target bar keeps its place under its own id; it opens again with
/// the next target, not with the next game.
pub const TARGET_BAR_ID: &str = "modern:bar:target";
pub const NEAR_WIDTH: f32 = 300.0;
pub const NEAR_LEAST: Vector = Vector::new(220.0, 110.0);
/// The height of a row of the near list.
pub const NEAR_ROW: f32 = 20.0;
/// The width of a bar of its own.
pub const BAR_WIDTH: f32 = 210.0;
pub const LINE_GAP: f32 = 4.0;
pub const BUTTON_ROW: f32 = 24.0;

/// The id a bar of a mobile keeps its place and its being open under.
pub fn bar_id(serial: u32) -> String {
    format!("{BAR_ID_PREFIX}{serial:08X}")
}

/// The mobile of a kept bar id. None for other ids and the target bar.
pub fn bar_serial(id: &str) -> Option<u32> {
    u32::from_str_radix(id.strip_prefix(BAR_ID_PREFIX)?, 16).ok()
}

/// The id of the bar of a subject: a mobile's own, or the target bar's.
pub fn subject_bar_id(subject: Subject) -> String {
    match subject {
        Subject::Mobile(serial) => bar_id(serial),
        _ => TARGET_BAR_ID.to_string(),
    }
}

/// How many lines of hits, mana and stamina a bar shows.
pub fn line_count(facts: &BarFacts) -> usize {
    [facts.hits, facts.mana, facts.stam]
        .iter()
        .filter(|value| value.is_some())
        .count()
        .max(1)
}

/// The size of the near list with room for `rows` rows.
pub fn near_size(rows: usize) -> Vector {
    Vector::new(
        NEAR_WIDTH,
        TITLE_ROW + NEAR_ROW * rows as f32 + PANEL_PAD * 2.0,
    )
}

/// The size of the near list for the people near now.
pub fn near_list_size(frame: &WatchFrame) -> Vector {
    near_size(frame.mobiles.len().clamp(1, MOBILE_LINES))
}

/// The size of a bar for its facts.
pub fn bar_size(facts: &BarFacts) -> Vector {
    let lines = line_count(facts) as f32;
    let buttons = if party_buttons(facts) {
        BUTTON_ROW + LINE_GAP
    } else {
        0.0
    };
    Vector::new(
        BAR_WIDTH,
        TITLE_ROW + lines * (BAR_HEIGHT + LINE_GAP) + buttons + PANEL_PAD * 2.0,
    )
}

/// The party buttons show on the bar of another member in range.
pub fn party_buttons(facts: &BarFacts) -> bool {
    facts.party && !facts.own && facts.in_range
}

/// The color of the hits of a bar.
pub fn hits_color(facts: &BarFacts) -> Rgba {
    if facts.poisoned {
        HITS_POISONED
    } else if facts.yellow_hits {
        STAM
    } else {
        HITS
    }
}

/// The mobiles whose bars the profile kept open, for the bars to come
/// back with the game.
pub fn restore_bars(profile: &Profile) -> Vec<u32> {
    profile
        .interface
        .open_panels
        .iter()
        .filter_map(|id| bar_serial(id))
        .collect()
}

/// Closes the bars of their own among `open`: every one, or only those
/// whose mobile is out of view. Gives the ids of the bars it closed, which
/// the profile no longer keeps open.
pub fn close_bars(
    open: impl IntoIterator<Item = Subject>,
    frame: &WatchFrame,
    inactive_only: bool,
    profile: &mut Profile,
) -> Vec<String> {
    let in_view = |serial: u32| frame.mobiles.iter().any(|mobile| mobile.serial == serial);
    let closing: Vec<String> = open
        .into_iter()
        .filter(|subject| match subject {
            Subject::Mobile(serial) => !inactive_only || !in_view(*serial),
            _ => false,
        })
        .map(subject_bar_id)
        .collect();
    for id in &closing {
        places::set_open(profile, id, false);
    }
    closing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchMobile, WatchPartyMember};
    use crate::model::health_bars;

    const ORC: u32 = 0x0000_0B01;
    const FRIEND: u32 = 0x0000_0B02;

    #[test]
    fn a_bar_id_names_its_mobile_and_nothing_else_does() {
        assert_eq!(bar_serial(&bar_id(ORC)), Some(ORC));
        assert_eq!(bar_serial(TARGET_BAR_ID), None);
        assert_eq!(bar_serial(NEAR_ID), None);
        assert_eq!(subject_bar_id(Subject::LastTarget), TARGET_BAR_ID);
    }

    #[test]
    fn a_party_bar_is_taller_than_the_bar_of_another() {
        let frame = WatchFrame {
            party_members: vec![WatchPartyMember {
                serial: FRIEND,
                name: "Bob".into(),
                hits_percent: Some(80),
                ..WatchPartyMember::default()
            }],
            ..WatchFrame::default()
        };
        let other = health_bars::facts(&frame, ORC);
        let mut member = health_bars::facts(&frame, FRIEND);
        member.in_range = true;
        member.mana = Some((1, 2));
        assert!(!party_buttons(&other) && party_buttons(&member));
        assert!(bar_size(&member).y > bar_size(&other).y);
        assert_eq!(
            line_count(&other),
            1,
            "an unknown mobile still has its line"
        );
        member.poisoned = true;
        assert_eq!(hits_color(&member), HITS_POISONED);
    }

    #[test]
    fn closing_bars_keeps_those_in_view_when_asked() {
        let frame = WatchFrame {
            mobiles: vec![WatchMobile {
                serial: ORC,
                ..WatchMobile::default()
            }],
            ..WatchFrame::default()
        };
        let mut profile = Profile::default();
        for serial in [ORC, FRIEND] {
            places::set_open(&mut profile, &bar_id(serial), true);
        }
        assert_eq!(restore_bars(&profile), vec![ORC, FRIEND]);
        let open = [
            Subject::Mobile(ORC),
            Subject::Mobile(FRIEND),
            Subject::LastTarget,
        ];
        assert_eq!(
            close_bars(open, &frame, true, &mut profile),
            vec![bar_id(FRIEND)]
        );
        assert_eq!(restore_bars(&profile), vec![ORC]);
        assert_eq!(
            close_bars(open, &frame, false, &mut profile),
            vec![bar_id(ORC), bar_id(FRIEND)]
        );
        assert!(profile.interface.open_panels.is_empty());
    }
}
