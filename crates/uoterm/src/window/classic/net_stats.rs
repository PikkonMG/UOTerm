//! The network statistics of the classic client: a dark see-through box with the round trip to the
//! shard, in a color that goes from green to red as it grows, and the bytes
//! that came and went in the last half second. A double click folds it to
//! the round trip alone. The words are `model::stats`', shared with the
//! Modern panel.

use super::canvas::Canvas;
use super::registry::{well_known, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::window::model::stats::{self, PingLevel};

pub const NET_STATS: GumpKind = GumpKind {
    id: well_known::NET_STATS,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(NetStats::default()),
};

const OPACITY: f32 = 0.7;
const BLACK_HUE: u16 = 0;
/// The words stand this far from each side of the box.
const PAD: i32 = 10;
const SIDES: i32 = 2;
const WORDS_FONT: u8 = 1;
const QUICK_HUE: u16 = 0x0044;
const FAIR_HUE: u16 = 0x0034;
const SLOW_HUE: u16 = 0x0031;
const LAGGING_HUE: u16 = 0x0020;

/// The hue of the words for a round trip: green, yellow, orange or red.
fn ping_hue(level: PingLevel) -> u16 {
    match level {
        PingLevel::Quick => QUICK_HUE,
        PingLevel::Fair => FAIR_HUE,
        PingLevel::Slow => SLOW_HUE,
        PingLevel::Lagging => LAGGING_HUE,
    }
}

#[derive(Default)]
pub struct NetStats {
    folded: bool,
}

impl GumpBody for NetStats {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if g.body_double_click() {
            self.folded = !self.folded;
        }
        let frame = cx.frame;
        let words = stats::net_words(frame, self.folded);
        let hue = ping_hue(stats::ping_level(stats::ping(frame)));
        let look = TextLook::unicode(WORDS_FONT, hue).bordered();
        let size = g.measure(&words, &look);
        let (width, height) = (size.x as i32 + PAD * SIDES, size.y as i32 + PAD * SIDES);
        g.shade(0, 0, width, height, BLACK_HUE, OPACITY);
        g.label(PAD, PAD, &words, &look);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_round_trip_goes_from_green_to_red() {
        assert_eq!(ping_hue(stats::ping_level(0)), QUICK_HUE);
        assert_eq!(ping_hue(stats::ping_level(150)), FAIR_HUE);
        assert_eq!(ping_hue(stats::ping_level(250)), SLOW_HUE);
        assert_eq!(ping_hue(stats::ping_level(300)), LAGGING_HUE);
    }
}
