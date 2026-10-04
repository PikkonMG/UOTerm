//! The colors that tell of a state: the wear of an item, the round trip
//! to the shard, and the hue a value of the info bar takes.

use super::super::theme::{ALARM, GOAL, HITS_POISONED, STAM, WAITING};
use crate::geom::Rgba;
use crate::model::durability::Wear;
use crate::model::info_bar::HUE_FINE;
use crate::model::stats::PingLevel;
use crate::settings::InfoBarHighlight;

/// The color of a durability bar: the alarm under the warning.
pub fn wear_color(wear: &Wear, warning: u8) -> Rgba {
    if wear.warns(warning) {
        ALARM
    } else {
        GOAL
    }
}

/// The color of the round trip: green, yellow, orange or red.
pub fn ping_color(level: PingLevel) -> Rgba {
    match level {
        PingLevel::Quick => HITS_POISONED,
        PingLevel::Fair => STAM,
        PingLevel::Slow => WAITING,
        PingLevel::Lagging => ALARM,
    }
}

/// The hue of the words of a value of the info bar: the hue that tells
/// when it runs low, or the fine hue when a colored bar under it tells.
pub fn info_highlight(highlight: InfoBarHighlight, value_hue: u16) -> u16 {
    match highlight {
        InfoBarHighlight::TextColor => value_hue,
        InfoBarHighlight::ColoredBars => HUE_FINE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_round_trip_goes_from_green_to_red() {
        assert_eq!(ping_color(PingLevel::Quick), HITS_POISONED);
        assert_eq!(ping_color(PingLevel::Lagging), ALARM);
        assert_ne!(ping_color(PingLevel::Fair), ping_color(PingLevel::Slow));
    }

    #[test]
    fn a_colored_bar_leaves_the_words_fine() {
        const LOW: u16 = 0x0021;
        assert_eq!(info_highlight(InfoBarHighlight::TextColor, LOW), LOW);
        assert_eq!(info_highlight(InfoBarHighlight::ColoredBars, LOW), HUE_FINE);
    }
}
