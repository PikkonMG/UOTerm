//! The sizes of the control bar at the middle of the top, which the plan
//! of the panels keeps room for, and its words.

use crate::frame::WatchFrame;
use crate::geom::Vector;

/// The words between the numbers of the place row.
pub const WORDS_MAP_LABEL: &str = "map";
pub const WORDS_FACES: &str = "faces";
/// The question of the guard stands in the middle, this large.
pub const GUARD_QUESTION_SIZE: Vector = Vector::new(300.0, 110.0);
/// The words of the last report stand this far under the bar.
pub const REPORT_GAP: f32 = 8.0;

/// Where the character is, in numbers.
pub fn place_numbers(frame: &WatchFrame) -> String {
    format!("{}, {}, {}", frame.x, frame.y, frame.z)
}

/// The words of the chat line while the agent has the character.
pub const WORDS_TAKE_TO_TALK: &str = "Take control to talk.";
/// The button that puts the words of the chat line on the hotbar.
pub const WORDS_PIN: &str = "Pin";

/// The top panel has one width in each state, so nothing in it moves when
/// the buttons change.
pub const BAR_WIDTH: f32 = 880.0;
pub const STRIP_PAD: f32 = 10.0;
pub const STRIP_ROW: f32 = 24.0;
pub const RULE_GAP: f32 = 8.0;
pub const SEGMENT_HEIGHT: f32 = 30.0;
/// The bar at its tallest: the place row, the row of what the human does,
/// and the buttons.
pub const BAR_MOST_HEIGHT: f32 =
    STRIP_PAD * 2.0 + STRIP_ROW * 2.0 + RULE_GAP * 2.0 + SEGMENT_HEIGHT;

/// The size of the bar: folded to its place row, or with the row of what
/// the human does when there are words for it.
pub fn bar_size(folded: bool, status: bool) -> Vector {
    let menu = if folded {
        0.0
    } else {
        RULE_GAP * 2.0 + if status { STRIP_ROW } else { 0.0 } + SEGMENT_HEIGHT
    };
    Vector::new(BAR_WIDTH, STRIP_PAD * 2.0 + STRIP_ROW + menu)
}
