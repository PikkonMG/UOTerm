//! The improved buff bar of the Modern style, as both windows show it:
//! where it first stands, the size of an icon, the words that stand for a
//! buff with no picture, and the color of the time a buff has left. Which
//! buffs show and in what order are `model::buffs`'.

use super::layout::{first_place, Spot};
use super::places::{with_title_room, TITLE_ROW};
use super::theme::{ALARM, PANEL_PAD, TEXT_DIM};
use crate::frame::WatchBuff;
use crate::geom::{Area, Rgba, Vector};
use crate::model::buffs;

pub const BUFFS_ID: &str = "modern:buffs";
pub const BUFF_ICON: f32 = 34.0;
pub const BUFF_GAP: f32 = 4.0;
/// The row of the time under an icon, when the Combat page shows it.
pub const BUFF_TIME_ROW: f32 = 14.0;
/// The words of a buff with no picture show this many letters.
const SHORT_NAME_CHARS: usize = 4;
pub const WORDS_BUFFS: &str = "Buffs";

/// Where the bar of `count` buffs first stands in `window`, with the time
/// under each icon when `show_time` is on.
pub fn buffs_first_place(window: Area, count: usize, show_time: bool) -> Area {
    let row = BUFF_ICON + if show_time { BUFF_TIME_ROW } else { 0.0 };
    let width = count as f32 * (BUFF_ICON + BUFF_GAP) - BUFF_GAP;
    let size =
        with_title_room(Vector::new(width, row + TITLE_ROW) + Vector::splat(PANEL_PAD * 2.0));
    first_place(window, Spot::MiddleTop(0), size)
}

/// The words that stand for a buff the client files have no picture of.
pub fn short_name(buff: &WatchBuff) -> String {
    buff.title.chars().take(SHORT_NAME_CHARS).collect()
}

/// The color of the time a buff has left: the alarm when it ends soon.
pub fn time_color(buff: &WatchBuff) -> Rgba {
    if buffs::is_ending(buff) {
        ALARM
    } else {
        TEXT_DIM
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;

    #[test]
    fn the_bar_grows_with_its_buffs_and_its_time_row() {
        let window = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(1600.0, 900.0));
        let few = buffs_first_place(window, 6, false);
        let more = buffs_first_place(window, 7, true);
        assert_eq!(more.width() - few.width(), BUFF_ICON + BUFF_GAP);
        assert_eq!(more.height() - few.height(), BUFF_TIME_ROW);
        let buff = WatchBuff {
            title: "Protection".into(),
            ..WatchBuff::default()
        };
        assert_eq!(short_name(&buff), "Prot");
    }
}
