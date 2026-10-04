//! The sizes of the control bar at the middle of the top, which the plan
//! of the panels keeps room for.

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
