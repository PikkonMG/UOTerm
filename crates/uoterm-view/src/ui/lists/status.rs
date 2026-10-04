//! The lines of the status view of the sheet, and how many stand in each
//! of its two columns.

use crate::frame::WatchFrame;
use crate::model::status::{sections, STAT_NAMES};

/// The height of one line of the status view.
pub const STATUS_LINE: f32 = 20.0;
/// The status view stands in this many columns.
pub const STATUS_COLUMNS: usize = 2;
const WORDS_STATS: &str = "Stats";

/// One line of the status view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatusLine {
    Title(&'static str),
    Stat(usize),
    Fact(&'static str, String),
}

/// Every line of the status view, in order.
pub fn status_lines(frame: &WatchFrame) -> Vec<StatusLine> {
    let mut lines = vec![StatusLine::Title(WORDS_STATS)];
    lines.extend((0..STAT_NAMES.len()).map(StatusLine::Stat));
    for section in sections(frame) {
        lines.push(StatusLine::Title(section.title));
        lines.extend(
            section
                .facts
                .into_iter()
                .map(|(words, value)| StatusLine::Fact(words, value)),
        );
    }
    lines
}

/// How many lines each column holds in `height`, and the last first line
/// the wheel may scroll to, for `count` lines.
pub fn column_room(height: f32, count: usize) -> (usize, usize) {
    let per_column = ((height / STATUS_LINE).floor() as usize).max(1);
    (
        per_column,
        count.saturating_sub(per_column * STATUS_COLUMNS),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_view_holds_the_stats_then_each_section_and_scrolls_for_the_rest() {
        let all = status_lines(&WatchFrame::default());
        assert_eq!(all[0], StatusLine::Title(WORDS_STATS));
        assert_eq!(all[1], StatusLine::Stat(0));
        let titles = all
            .iter()
            .filter(|line| matches!(line, StatusLine::Title(_)))
            .count();
        assert_eq!(titles, 6, "the stats and the five sections");
        const TEN_LINES: f32 = STATUS_LINE * 10.5;
        assert_eq!(column_room(TEN_LINES, 25), (10, 5));
        assert_eq!(column_room(TEN_LINES, 12), (10, 0));
        assert_eq!(column_room(0.0, 3), (1, 1), "one line shows at the least");
    }
}
