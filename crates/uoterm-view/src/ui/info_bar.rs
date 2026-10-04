//! The info bar, the network statistics and the debug window of the
//! Modern style, as both windows show them: the items of the Info Bar page
//! in one row, each label in its hue and each value in the hue that tells
//! when it runs low, or with a colored bar under it; and the panels of
//! words of `model::stats`, which a double click shows in short or in
//! full. Where each first stands needs the width of its words, which each
//! window measures with its own font.

use super::layout::{first_place, Spot};
use super::lists::{info_highlight, ping_color};
use super::places::{with_title_room, TITLE_ROW};
use super::theme::{PANEL_PAD, TEXT};
use crate::frame::WatchFrame;
use crate::geom::{Area, Rgba, Vector};
use crate::model::info_bar::value_of;
use crate::model::stats::{self, DebugFacts};
use crate::settings::{InfoBarHighlight, Profile};

pub const INFO_BAR_ID: &str = "modern:info_bar";
pub const INFO_ITEM_GAP: f32 = 14.0;
pub const INFO_LABEL_GAP: f32 = 4.0;
pub const INFO_ROW: f32 = 20.0;
pub const INFO_BAR_HEIGHT: f32 = 3.0;
pub const WORDS_INFO: &str = "Info";

pub const WORDS_NET_STATS: &str = "Network";
pub const WORDS_DEBUG: &str = "Debug";
pub const HINT_MORE: &str = "Double-click: more or less.";
/// The statistics panels first stand at the top of the middle, one under
/// the other.
const NET_STATS_SPOT: Spot = Spot::MiddleTop(0);
const DEBUG_SPOT: Spot = Spot::MiddleTop(1);

/// One item of the info bar: its label in the hue of the page, and its
/// value in the hue that tells when it runs low.
#[derive(Clone, Debug, PartialEq)]
pub struct InfoPart {
    pub label: String,
    pub label_hue: u16,
    pub words: String,
    pub words_hue: u16,
    /// How full the colored bar under the value is, when the page asks for
    /// bars and the value has a most; its hue is `bar_hue`.
    pub fill: Option<f32>,
    pub bar_hue: u16,
}

/// The items of the info bar now.
pub fn info_parts(frame: &WatchFrame, profile: &Profile) -> Vec<InfoPart> {
    let options = &profile.info_bar;
    options
        .items
        .iter()
        .map(|item| {
            let value = value_of(item.data, frame, &profile.combat);
            InfoPart {
                label: item.label.clone(),
                label_hue: item.hue,
                words_hue: info_highlight(options.highlight, value.hue),
                fill: value
                    .fill
                    .filter(|_| options.highlight == InfoBarHighlight::ColoredBars),
                bar_hue: value.hue,
                words: value.words,
            }
        })
        .collect()
}

/// Where the info bar first stands in `window`. `width` gives the width of
/// words in the font of the bar.
pub fn info_first_place(window: Area, parts: &[InfoPart], width: impl Fn(&str) -> f32) -> Area {
    let row: f32 = parts
        .iter()
        .map(|part| width(&part.label) + INFO_LABEL_GAP + width(&part.words) + INFO_ITEM_GAP)
        .sum::<f32>()
        .max(INFO_ITEM_GAP);
    let size = with_title_room(
        Vector::new(row - INFO_ITEM_GAP, INFO_ROW + TITLE_ROW) + Vector::splat(PANEL_PAD * 2.0),
    );
    first_place(window, Spot::MiddleBottom(0), size)
}

/// Which statistics panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatsPanel {
    Network,
    Debug,
}

/// Whether each statistics panel shows its other words: the network
/// statistics start in full and fold, the debug window starts short.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatsLook {
    pub net_folded: bool,
    pub debug_full: bool,
}

impl StatsLook {
    /// A double click on a panel shows its other words.
    pub fn flip(&mut self, panel: StatsPanel) {
        match panel {
            StatsPanel::Network => self.net_folded = !self.net_folded,
            StatsPanel::Debug => self.debug_full = !self.debug_full,
        }
    }

    /// The words of a panel and their color. `facts` are those of the
    /// debug window.
    pub fn words(
        &self,
        panel: StatsPanel,
        frame: &WatchFrame,
        facts: &DebugFacts,
    ) -> (String, Rgba) {
        match panel {
            StatsPanel::Network => (
                stats::net_words(frame, self.net_folded),
                ping_color(stats::ping_level(stats::ping(frame))),
            ),
            StatsPanel::Debug => (stats::debug_words(frame, facts, self.debug_full), TEXT),
        }
    }
}

/// The title of a statistics panel.
pub fn stats_title(panel: StatsPanel) -> &'static str {
    match panel {
        StatsPanel::Network => WORDS_NET_STATS,
        StatsPanel::Debug => WORDS_DEBUG,
    }
}

/// Where a statistics panel first stands in `window`, by the size of its
/// words.
pub fn stats_first_place(window: Area, panel: StatsPanel, words: Vector) -> Area {
    let spot = match panel {
        StatsPanel::Network => NET_STATS_SPOT,
        StatsPanel::Debug => DEBUG_SPOT,
    };
    let size =
        with_title_room(words + Vector::new(0.0, TITLE_ROW) + Vector::splat(PANEL_PAD * 2.0));
    first_place(window, spot, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;
    use crate::settings::{InfoBarData, InfoBarItem};

    #[test]
    fn the_info_bar_is_as_wide_as_its_words_and_bars_come_with_the_option() {
        let mut profile = Profile::default();
        profile.info_bar.items = vec![InfoBarItem {
            label: "Hits".into(),
            hue: 0,
            data: InfoBarData::HitPoints,
        }];
        let frame = WatchFrame::default();
        profile.info_bar.highlight = InfoBarHighlight::TextColor;
        let parts = info_parts(&frame, &profile);
        assert!(parts[0].fill.is_none(), "text colors only");
        profile.info_bar.highlight = InfoBarHighlight::ColoredBars;
        let barred = info_parts(&frame, &profile);
        assert_eq!(barred[0].label, "Hits");
        const LETTER: f32 = 10.0;
        let window = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(4000.0, 900.0));
        let width = |words: &str| words.chars().count() as f32 * LETTER;
        let wide = InfoPart {
            label: "x".repeat(60),
            ..parts[0].clone()
        };
        let place = info_first_place(window, &[wide], width);
        assert!(place.width() > 600.0);
    }

    #[test]
    fn a_double_click_shows_the_other_words_of_a_panel() {
        let mut look = StatsLook::default();
        let frame = WatchFrame::default();
        let facts = DebugFacts {
            fps: 60.0,
            zoom: 1.0,
            selected: None,
        };
        let short = look.words(StatsPanel::Debug, &frame, &facts).0;
        look.flip(StatsPanel::Debug);
        let full = look.words(StatsPanel::Debug, &frame, &facts).0;
        assert!(full.lines().count() > short.lines().count());
        assert_eq!(stats_title(StatsPanel::Network), WORDS_NET_STATS);
    }
}
