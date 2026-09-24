//! The grid of hues a color picker offers, apart from how it draws, as
//! the reference client lays it: rows of cells whose hues step by
//! five from a graduation the player shifts, the cell that shows a hue, and
//! the hue of a thing the eyedropper picks. The classic color picker and
//! the Modern dye panel both pick from it.

use crate::view::{WatchFrame, WatchLook};

pub const GRADUATION_MIN: i32 = 0;
pub const GRADUATION_MAX: i32 = 4;
/// The graduation a picker starts at when its hue is not in the grid.
pub const GRADUATION_START: i32 = 1;
pub const GRID_ROWS: usize = 10;
pub const GRID_COLUMNS: usize = 20;
/// The hues of the grid go up by this much from one cell to the next.
const HUE_STEP: i32 = 5;
/// The first hue of the grid is the graduation plus this.
const HUE_START: i32 = 2;
/// The words of the client for a hue the picker cannot show.
pub const BAD_HUE_CLILOC: u32 = 1_042_295;
pub const BAD_HUE_WORDS: &str = "That hue is not valid.";

/// The hue in one cell of the grid for a graduation.
pub fn grid_hue(graduation: i32, index: usize) -> u16 {
    (graduation + HUE_START + index as i32 * HUE_STEP) as u16
}

/// The graduation and the cell that show a hue, when the grid can.
pub fn find_hue(hue: u16) -> Option<(i32, usize)> {
    (GRADUATION_MIN..=GRADUATION_MAX).find_map(|graduation| {
        let from_start = i32::from(hue) - graduation - HUE_START;
        let index = from_start / HUE_STEP;
        let fits = from_start >= 0
            && from_start % HUE_STEP == 0
            && (index as usize) < GRID_ROWS * GRID_COLUMNS;
        fits.then_some((graduation, index as usize))
    })
}

/// The hue of a thing the window knows: on the ground, a mobile, worn, in
/// a container or in a trade.
pub fn hue_of(frame: &WatchFrame, serial: u32) -> Option<u16> {
    let worn = |look: &WatchLook| {
        look.equipment
            .iter()
            .find(|item| item.serial == serial)
            .map(|item| item.hue)
    };
    let packed = frame
        .containers
        .iter()
        .flat_map(|container| &container.items)
        .chain(
            frame
                .trades
                .iter()
                .flat_map(|trade| trade.mine_items.iter().chain(&trade.their_items)),
        )
        .find(|item| item.serial == serial)
        .map(|item| item.hue);
    frame
        .items
        .iter()
        .find(|item| item.serial == serial)
        .map(|item| item.hue)
        .or_else(|| {
            frame
                .mobiles
                .iter()
                .find(|mobile| mobile.serial == serial)
                .map(|mobile| mobile.look.hue)
        })
        .or_else(|| (serial == frame.serial).then_some(frame.look.hue))
        .or_else(|| worn(&frame.look))
        .or_else(|| frame.mobiles.iter().find_map(|mobile| worn(&mobile.look)))
        .or(packed)
}

/// The place in the grid a picker shows: the graduation and the cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HuePick {
    pub graduation: i32,
    pub index: usize,
}

impl HuePick {
    /// The pick that shows `hue`, or the first cell when the grid cannot.
    pub fn of(hue: u16) -> Self {
        let (graduation, index) = find_hue(hue).unwrap_or((GRADUATION_START, 0));
        Self { graduation, index }
    }

    /// The hue picked.
    pub fn hue(self) -> u16 {
        grid_hue(self.graduation, self.index)
    }

    /// Takes a hue the eyedropper found. False when the grid cannot show it.
    pub fn take(&mut self, hue: Option<u16>) -> bool {
        match hue.and_then(find_hue) {
            Some((graduation, index)) => {
                *self = Self { graduation, index };
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchContainer, WatchItem, WatchPackItem};

    #[test]
    fn the_grid_steps_by_five_from_the_graduation() {
        assert_eq!(grid_hue(1, 0), 3);
        assert_eq!(grid_hue(1, 2), 13);
        assert_eq!(grid_hue(4, 199), 1001);
    }

    #[test]
    fn a_hue_is_found_in_the_grid_or_not() {
        assert_eq!(find_hue(13), Some((1, 2)));
        assert_eq!(find_hue(2), Some((0, 0)));
        assert_eq!(find_hue(1), None);
        assert_eq!(find_hue(5000), None);
        assert_eq!(HuePick::of(1001).hue(), 1001);
        assert_eq!(
            HuePick::of(1),
            HuePick {
                graduation: GRADUATION_START,
                index: 0
            }
        );
    }

    #[test]
    fn the_eyedropper_finds_the_hue_of_a_thing() {
        const TUB: u32 = 0x4000_0100;
        const SHIRT: u32 = 0x4000_0200;
        let frame = WatchFrame {
            items: vec![WatchItem {
                serial: TUB,
                hue: 13,
                ..WatchItem::default()
            }],
            containers: vec![WatchContainer {
                items: vec![WatchPackItem {
                    serial: SHIRT,
                    hue: 23,
                    ..WatchPackItem::default()
                }],
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        };
        assert_eq!(hue_of(&frame, TUB), Some(13));
        assert_eq!(hue_of(&frame, SHIRT), Some(23));
        assert_eq!(hue_of(&frame, 7), None);
        let mut pick = HuePick::of(0);
        assert!(pick.take(hue_of(&frame, SHIRT)));
        assert_eq!(pick.hue(), 23);
        assert!(!pick.take(hue_of(&frame, 7)));
        assert_eq!(pick.hue(), 23, "a hue the grid lacks changes nothing");
    }
}
