//! The hue grid of the Modern style for every panel that picks a hue, and
//! the dye panel for a dye tub that asks for a colour when the client
//! files have no gump art: the sizes of the grid and its shade slider, the
//! eyedropper that takes the hue of a thing the player clicks, and the tub
//! the panel picks for. The hues are `model::hue_grid`'s.

use super::layout::{first_place, Spot};
use super::places::{FOOT_ROW, TITLE_ROW};
use super::theme::{PANEL_PAD, ROW_GAP};
use crate::act::Act;
use crate::frame::{WatchDye, WatchFrame};
use crate::geom::{Area, Vector};
use crate::guard::LocalAim;
use crate::model::hue_grid::{hue_of, HuePick, BAD_HUE_WORDS, GRID_COLUMNS, GRID_ROWS};

/// The side of a cell of the hue grid.
pub const HUE_CELL: f32 = 14.0;
/// The shade slider under the grid.
pub const SLIDER_ROW: f32 = 28.0;
pub const WORDS_SHADE: &str = "Shade";
pub const WORDS_EYEDROPPER: &str = "Eyedropper";

pub const DYE_ID: &str = "modern:dye";
/// The side of the tub beside the grid.
pub const TUB_SIDE: f32 = 64.0;
pub const WORDS_DYE: &str = "Dye";
pub const WORDS_OKAY: &str = "Okay";
/// A new tub starts at no hue.
const NO_HUE: u16 = 0;

/// The size of the grid of hues.
pub fn grid_size() -> Vector {
    Vector::new(GRID_COLUMNS as f32 * HUE_CELL, GRID_ROWS as f32 * HUE_CELL)
}

/// The size of the grid with the slider under it.
pub fn picker_size() -> Vector {
    grid_size() + Vector::new(0.0, SLIDER_ROW)
}

/// The size of the dye panel: the grid with its slider, and the tub
/// beside it.
pub fn dye_size() -> Vector {
    let picker = picker_size();
    Vector::new(
        picker.x + ROW_GAP * 2.0 + TUB_SIDE + PANEL_PAD * 2.0,
        TITLE_ROW + picker.y + FOOT_ROW + PANEL_PAD * 2.0,
    )
}

/// Where the dye panel first stands in `window`.
pub fn dye_first_place(window: Area) -> Area {
    first_place(window, Spot::Middle(0), dye_size())
}

/// Whether the eyedropper waits for a click on a thing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Eyedropper {
    pub picking: bool,
}

impl Eyedropper {
    /// A press of the eyedropper: the next click on a thing gives its hue.
    /// Gives the act that drops a target cursor of the shard first. The
    /// caller aims at [`LocalAim::PickThing`].
    pub fn press(&mut self, frame: &WatchFrame) -> Option<Act> {
        self.picking = true;
        frame.target_cursor.then_some(Act::CancelTarget)
    }

    /// Takes the hue of the thing the eyedropper clicked into the pick.
    /// `aiming` is the aim of the hand now and `picked` takes the thing a
    /// click took for it. Gives the words to tell when the grid cannot show
    /// the hue.
    pub fn follow(
        &mut self,
        aiming: Option<LocalAim>,
        picked: impl FnOnce() -> Option<u32>,
        pick: &mut HuePick,
        frame: &WatchFrame,
    ) -> Option<&'static str> {
        if !self.picking {
            return None;
        }
        if aiming != Some(LocalAim::PickThing) {
            self.picking = false;
        }
        let serial = picked()?;
        self.picking = false;
        (!pick.take(hue_of(frame, serial))).then_some(BAD_HUE_WORDS)
    }

    /// Stops waiting for a click, as a new pick starts.
    pub fn stop(&mut self) {
        self.picking = false;
    }
}

/// The tub whose colour the dye panel picks, and the pick.
#[derive(Default)]
pub struct DyePanel {
    pub tub: Option<(u32, HuePick)>,
}

impl DyePanel {
    /// Follows the tub that waits: a new tub starts at no hue, and its
    /// eyedropper waits no more. Gives the pick of the tub that waits.
    pub fn follow(
        &mut self,
        dye: Option<&WatchDye>,
        eyedropper: &mut Eyedropper,
    ) -> Option<&mut HuePick> {
        let Some(dye) = dye else {
            self.tub = None;
            return None;
        };
        if self.tub.is_none_or(|(tub, _)| tub != dye.serial) {
            self.tub = Some((dye.serial, HuePick::of(NO_HUE)));
            eyedropper.stop();
        }
        self.tub.as_mut().map(|(_, pick)| pick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::WatchItem;

    const TUB: u32 = 0x4000_0100;
    const CLOTH: u32 = 0x4000_0101;

    #[test]
    fn a_new_tub_starts_over_and_the_same_tub_keeps_its_pick() {
        let dye = |serial| WatchDye {
            serial,
            graphic: 0x0FAB,
        };
        let mut panel = DyePanel::default();
        let mut eyedropper = Eyedropper::default();
        *panel.follow(Some(&dye(TUB)), &mut eyedropper).unwrap() = HuePick::of(1001);
        eyedropper.picking = true;
        assert_eq!(
            panel
                .follow(Some(&dye(TUB)), &mut eyedropper)
                .unwrap()
                .hue(),
            1001
        );
        assert!(eyedropper.picking, "the same tub");
        let other = *panel.follow(Some(&dye(TUB + 1)), &mut eyedropper).unwrap();
        assert_eq!((other, eyedropper.picking), (HuePick::of(NO_HUE), false));
        assert!(panel.follow(None, &mut eyedropper).is_none() && panel.tub.is_none());
        assert!(dye_size().x > picker_size().x);
        assert_eq!(picker_size().y, grid_size().y + SLIDER_ROW);
    }

    #[test]
    fn the_eyedropper_takes_the_hue_of_a_clicked_thing() {
        let mut frame = WatchFrame {
            target_cursor: true,
            items: vec![WatchItem {
                serial: CLOTH,
                hue: 1001,
                ..WatchItem::default()
            }],
            ..WatchFrame::default()
        };
        let mut eyedropper = Eyedropper::default();
        assert_eq!(eyedropper.press(&frame), Some(Act::CancelTarget));
        frame.target_cursor = false;
        let mut pick = HuePick::of(NO_HUE);
        let aim = Some(LocalAim::PickThing);
        assert_eq!(eyedropper.follow(aim, || None, &mut pick, &frame), None);
        assert!(eyedropper.picking, "it waits");
        assert_eq!(
            eyedropper.follow(None, || Some(CLOTH), &mut pick, &frame),
            None
        );
        assert_eq!((pick.hue(), eyedropper.picking), (1001, false));
        eyedropper.press(&frame);
        assert_eq!(
            eyedropper.follow(None, || Some(CLOTH + 9), &mut pick, &frame),
            Some(BAD_HUE_WORDS)
        );
    }
}
