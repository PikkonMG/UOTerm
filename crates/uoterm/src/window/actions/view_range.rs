//! The view range of the official client: mobiles and items farther than
//! this many tiles from the character are not drawn. It is the window's
//! own, as in the reference client; the shard still sends them.

use super::RangeChange;
use crate::view::WatchFrame;

pub const MIN_VIEW_RANGE: u8 = 5;
pub const MAX_VIEW_RANGE: u8 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewRange {
    tiles: u8,
}

impl Default for ViewRange {
    fn default() -> Self {
        Self {
            tiles: MAX_VIEW_RANGE,
        }
    }
}

/// The distance of the official client: the longer of the two sides.
fn tiles_between(from: (u16, u16), to: (u16, u16)) -> u16 {
    from.0.abs_diff(to.0).max(from.1.abs_diff(to.1))
}

impl ViewRange {
    /// Changes the range, inside the range the official client allows.
    /// Gives the new range.
    pub fn change(&mut self, change: RangeChange) -> u8 {
        let tiles = match change {
            RangeChange::Set(tiles) => tiles,
            RangeChange::Up => self.tiles.saturating_add(1),
            RangeChange::Down => self.tiles.saturating_sub(1),
            RangeChange::Max | RangeChange::Default => MAX_VIEW_RANGE,
            RangeChange::Min => MIN_VIEW_RANGE,
        };
        self.tiles = tiles.clamp(MIN_VIEW_RANGE, MAX_VIEW_RANGE);
        self.tiles
    }

    /// Takes the mobiles and items out of range from the picture.
    pub fn cull(self, frame: &mut WatchFrame) {
        if self.tiles >= MAX_VIEW_RANGE {
            return;
        }
        let here = (frame.x, frame.y);
        let near = |x: u16, y: u16| tiles_between(here, (x, y)) <= u16::from(self.tiles);
        frame.mobiles.retain(|mobile| near(mobile.x, mobile.y));
        frame.items.retain(|item| near(item.x, item.y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchItem, WatchMobile};

    #[test]
    fn the_range_stays_inside_the_limits_of_the_official_client() {
        let mut range = ViewRange::default();
        assert_eq!(range.change(RangeChange::Up), MAX_VIEW_RANGE);
        assert_eq!(range.change(RangeChange::Set(2)), MIN_VIEW_RANGE);
        assert_eq!(range.change(RangeChange::Up), MIN_VIEW_RANGE + 1);
        assert_eq!(range.change(RangeChange::Default), MAX_VIEW_RANGE);
        assert_eq!(range.change(RangeChange::Min), MIN_VIEW_RANGE);
    }

    #[test]
    fn things_out_of_range_are_not_drawn() {
        let mut frame = WatchFrame {
            x: 100,
            y: 100,
            mobiles: vec![
                WatchMobile {
                    serial: 1,
                    x: 104,
                    y: 99,
                    ..WatchMobile::default()
                },
                WatchMobile {
                    serial: 2,
                    x: 110,
                    y: 100,
                    ..WatchMobile::default()
                },
            ],
            items: vec![WatchItem {
                serial: 3,
                x: 100,
                y: 107,
                ..WatchItem::default()
            }],
            ..WatchFrame::default()
        };
        let mut range = ViewRange::default();
        range.change(RangeChange::Set(6));
        range.cull(&mut frame);
        assert_eq!(frame.mobiles.len(), 1);
        assert!(frame.items.is_empty());
    }
}
