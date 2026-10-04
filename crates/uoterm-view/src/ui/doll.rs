//! The paperdoll of the Modern style, as both windows show it: which doll
//! shows, where it first stands, the figure and the health of its mobile,
//! the worn items with what a click on one does, and its buttons.

use super::layout::{first_place, Spot};
use super::places::{FOOT_ROW, TITLE_ROW};
use super::theme::PANEL_PAD;
use crate::act::Act;
use crate::desk::Zone;
use crate::frame::{WatchEquip, WatchFrame, WatchLook};
use crate::geom::{Area, Vector};
use crate::model::dolls::{DollWatch, GUILD_COMMAND, QUESTS_COMMAND};
use crate::model::durability::is_worn_layer;

pub const DOLL_ID: &str = "modern:paperdoll";
const DOLL_PANEL_WIDTH: f32 = 420.0;
/// The room of the figure.
pub const DOLL_PICTURE: Vector = Vector::new(140.0, 220.0);
pub const DOLL_ROW: f32 = 30.0;
const DOLL_ROWS: usize = 10;
/// A mobile's hits are a share out of this.
const PERCENT: f32 = 100.0;

pub const WORDS_OUT_OF_SIGHT: &str = "Out of sight.";
pub const WORDS_WEARS_NOTHING: &str = "Wears nothing.";
const WORDS_STATUS: &str = "Status";
const WORDS_VIRTUES: &str = "Virtues";
const WORDS_QUESTS: &str = "Quests";
const WORDS_GUILD: &str = "Guild";
pub const WORDS_CLOSE: &str = "Close";
const HINT_WORN: &str = "Double-click: use.";
const HINT_WORN_LIFT: &str = "Double-click: use.  Drag: take it off.";

/// Where the paperdoll first stands in `window`.
pub fn doll_first_place(window: Area) -> Area {
    let size = Vector::new(
        DOLL_PANEL_WIDTH,
        PANEL_PAD * 2.0 + TITLE_ROW + DOLL_ROWS as f32 * DOLL_ROW + FOOT_ROW,
    );
    first_place(window, Spot::Middle(0), size)
}

/// The paperdoll that shows: whose it is, the words the shard put at its
/// top, and whether the shard lets the character dress it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShownDoll {
    pub serial: u32,
    pub text: String,
    pub can_lift: bool,
}

/// The paperdolls the shard opened, and the one that shows until the
/// player closes it.
#[derive(Default)]
pub struct DollPanel {
    dolls: DollWatch,
    pub doll: Option<ShownDoll>,
}

impl DollPanel {
    /// Shows the paperdoll the shard sent since the last frame. True when
    /// a new one came.
    pub fn follow(&mut self, frame: &WatchFrame) -> bool {
        let Some(fresh) = self.dolls.take(frame) else {
            return false;
        };
        self.doll = Some(ShownDoll {
            serial: fresh.serial,
            text: fresh.text.clone(),
            can_lift: fresh.can_lift,
        });
        true
    }

    pub fn close(&mut self) {
        self.doll = None;
    }
}

/// The looks of the mobile of a paperdoll: the character's, or another's
/// while he is in sight.
pub fn doll_look(frame: &WatchFrame, serial: u32) -> Option<&WatchLook> {
    if serial == frame.serial {
        Some(&frame.look)
    } else {
        frame
            .mobiles
            .iter()
            .find(|mobile| mobile.serial == serial)
            .map(|mobile| &mobile.look)
    }
}

/// The worn items a paperdoll lists: those on the layers of things worn.
pub fn doll_worn(look: &WatchLook) -> Vec<&WatchEquip> {
    look.equipment
        .iter()
        .filter(|item| is_worn_layer(item.layer))
        .collect()
}

/// The health of the mobile of a paperdoll, as a share: the character's
/// hits, or the share of hits the shard told of another.
pub fn doll_health(frame: &WatchFrame, serial: u32) -> Option<f32> {
    if serial == frame.serial {
        (frame.hits_max > 0).then(|| f32::from(frame.hits) / f32::from(frame.hits_max))
    } else {
        frame
            .mobiles
            .iter()
            .find(|mobile| mobile.serial == serial)
            .and_then(|mobile| mobile.hits_percent)
            .map(|percent| f32::from(percent) / PERCENT)
    }
}

/// Where an item dropped on a paperdoll the character dresses goes: on
/// him, or into the other mobile.
pub fn doll_zone(frame: &WatchFrame, serial: u32) -> Zone {
    if serial == frame.serial {
        Zone::Wear
    } else {
        Zone::Into(serial)
    }
}

/// What a click on a worn item does, as the tip says.
pub fn worn_footer(live: bool, dresses: bool) -> &'static str {
    match (live, dresses) {
        (false, _) => "",
        (true, true) => HINT_WORN_LIFT,
        (true, false) => HINT_WORN,
    }
}

/// The buttons of a paperdoll and their acts: the status of the mobile
/// and his virtues, and the quests and the guild of the character on his
/// own.
pub fn doll_buttons(serial: u32, own: bool) -> Vec<(&'static str, Act)> {
    let mut buttons = vec![
        (
            WORDS_STATUS,
            Act::MobileStatus {
                serial,
                close: false,
            },
        ),
        (WORDS_VIRTUES, Act::VirtueGump(serial)),
    ];
    if own {
        buttons.push((WORDS_QUESTS, Act::Command(QUESTS_COMMAND.into())));
        buttons.push((WORDS_GUILD, Act::Command(GUILD_COMMAND.into())));
    }
    buttons
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchMobile, WatchPaperdoll};

    const ME: u32 = 0x0000_0001;
    const STRANGER: u32 = 0x0000_0A11;

    #[test]
    fn a_paperdoll_the_shard_sends_shows_until_closed() {
        let mut panel = DollPanel::default();
        let mut frame = WatchFrame {
            paperdoll: Some(WatchPaperdoll {
                serial: STRANGER,
                text: "Someone the Brave".into(),
                seq: 1,
                can_lift: true,
            }),
            ..WatchFrame::default()
        };
        assert!(!panel.follow(&frame), "an old doll");
        frame.paperdoll.as_mut().unwrap().seq = 2;
        assert!(panel.follow(&frame));
        let doll = panel.doll.as_ref().unwrap();
        assert!(doll.can_lift && doll.serial == STRANGER);
        panel.close();
        assert!(!panel.follow(&frame) && panel.doll.is_none());
    }

    #[test]
    fn the_doll_of_the_character_has_his_buttons_and_dresses_him() {
        let frame = WatchFrame {
            serial: ME,
            hits: 25,
            hits_max: 50,
            mobiles: vec![WatchMobile {
                serial: STRANGER,
                hits_percent: Some(40),
                ..WatchMobile::default()
            }],
            ..WatchFrame::default()
        };
        assert_eq!(doll_buttons(ME, true).len(), 4);
        assert_eq!(doll_buttons(STRANGER, false).len(), 2);
        assert_eq!(doll_health(&frame, ME), Some(0.5));
        assert_eq!(doll_health(&frame, STRANGER), Some(0.4));
        assert_eq!(doll_zone(&frame, ME), Zone::Wear);
        assert_eq!(doll_zone(&frame, STRANGER), Zone::Into(STRANGER));
        assert!(doll_look(&frame, ME + 7).is_none());
        assert_eq!(worn_footer(true, true), HINT_WORN_LIFT);
    }
}
