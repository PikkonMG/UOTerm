//! The house designer panel of the Modern style, apart from how it draws:
//! its place, its words, the rows of its buttons, the storeys a plot has,
//! the counts of the design in words, and the part Jev picks from the
//! catalog. The design itself is `model::house_design`'s, which the clicks
//! on the map and the drawing of the house read.

use crate::act::{Answer, Ask};
use crate::frame::{WatchDesigning, WatchFrame};
use crate::geom::{Area, Vector};
use crate::model::house_design::{
    design_counts, part_words, plot_limits, HouseDesign, PlotLimits, ACTION_EXIT, DESIGNER_FLOORS,
    DESIGN_COMMANDS,
};
use crate::ui::layout::{first_place, Spot};
use crate::ui::places::TITLE_ROW;
use crate::ui::theme::PANEL_PAD;

pub const BUILD_ID: &str = "modern:build";
const PANEL_WIDTH: f32 = 430.0;
/// The designer's foot has five rows: the changes, the backups and the
/// eyedropper, the levels, how each storey shows, and the counts.
pub const BUTTON_ROWS: usize = 5;
/// The first steps of `DESIGN_COMMANDS` keep the design and bring it back;
/// the rest change it.
const KEEPING_COMMANDS: usize = 3;
pub const KIND_ROW: f32 = 32.0;
pub const FIELD_ROW: f32 = 30.0;
pub const FOOT_ROW: f32 = 40.0;
pub const GAP: f32 = 8.0;
pub const LIST_ROWS: usize = 8;
pub const LIST_ROW: f32 = 30.0;
pub const PIECE_SIDE: f32 = 40.0;
/// The words under the panel show this long, in seconds.
pub const NOTE_SECONDS: f64 = 6.0;
/// The gaps between the rows of the panel.
const ROW_GAPS: f32 = 4.0;

pub const WORDS_TITLE: &str = "Build";
pub const WORDS_REMOVE: &str = "Remove";
pub const WORDS_PICK: &str = "Pick";
const WORDS_STOREY: &str = "Storey";
const WORDS_COMPONENTS: &str = "Components";
const WORDS_FIXTURES: &str = "Fixtures";
const WORDS_COST: &str = "Cost";
pub const HINT_PICK: &str = "Click a part of the house to build with it.";
pub const HINT_STOREY: &str = "Click: how this storey shows while you design.";
pub const WORDS_FLOOR: &str = "Floor";
const WORDS_EXIT: &str = "Leave";
pub const WORDS_FIND: &str = "Find";
pub const WORDS_ASKING: &str = "Jev looks at the catalog...";
pub const WORDS_NO_PARTS: &str = "The catalog needs the client files.";
pub const HINT_WISH: &str = "Say the part in plain words, for example: a stone wall";

/// Where the designer first stands in a window.
pub fn build_first_place(window: Area) -> Area {
    let height = TITLE_ROW
        + KIND_ROW
        + LIST_ROWS as f32 * LIST_ROW
        + PIECE_SIDE
        + FIELD_ROW
        + FOOT_ROW * BUTTON_ROWS as f32
        + GAP * ROW_GAPS
        + PANEL_PAD * 2.0;
    first_place(
        window,
        Spot::RightColumn(0),
        Vector::new(PANEL_WIDTH, height),
    )
}

/// The two rows of the steps of the designer that name no part: the steps
/// that change the design with the one that leaves it, and the steps that
/// keep the design and bring it back. Each is its words and its action.
pub fn command_rows() -> [Vec<(&'static str, &'static str)>; 2] {
    let (kept, changes) = DESIGN_COMMANDS.split_at(KEEPING_COMMANDS);
    let mut changing = changes.to_vec();
    changing.push((WORDS_EXIT, ACTION_EXIT));
    [changing, kept.to_vec()]
}

/// The limits of the plot of the house, when the client files hold its
/// foundation.
pub fn limits_of(designing: &WatchDesigning) -> Option<PlotLimits> {
    designing
        .plot
        .map(|(width, depth)| plot_limits(width, depth))
}

/// How many storeys the levels and the storey buttons count.
pub fn storeys_of(limits: Option<PlotLimits>) -> u8 {
    limits.map_or(DESIGNER_FLOORS, |limits| limits.storeys)
}

/// The words of the button of a storey, from 0.
pub fn storey_words(storey: usize) -> String {
    format!("{WORDS_STOREY} {}", storey + 1)
}

/// The components and fixtures of the design against the most the plot
/// takes, and what it costs, in words; true for a count at its most.
pub fn counts_words(frame: &WatchFrame, limits: PlotLimits) -> [(String, bool); 3] {
    let counts = design_counts(frame);
    [
        (
            format!(
                "{WORDS_COMPONENTS} {}/{}",
                counts.components, limits.components
            ),
            counts.components >= limits.components,
        ),
        (
            format!("{WORDS_FIXTURES} {}/{}", counts.fixtures, limits.fixtures),
            counts.fixtures >= limits.fixtures,
        ),
        (format!("{WORDS_COST} {}", counts.cost()), false),
    ]
}

/// The question to Jev for the part of the catalog plain words mean. None
/// for no words, or a catalog with no parts.
pub fn part_ask(frame: &WatchFrame, wish: &str) -> Option<Ask> {
    let wish = wish.trim();
    if wish.is_empty() || frame.house_parts.is_empty() {
        return None;
    }
    Some(Ask::HousePart {
        wish: wish.to_string(),
        options: frame.house_parts.iter().map(part_words).collect(),
    })
}

/// Takes the answer of Jev about a part: the part he picked becomes the
/// part of the design. Gives true when it picked one; the words of a
/// failure as an error.
pub fn take_part_answer(
    design: &mut HouseDesign,
    frame: &WatchFrame,
    answer: Answer,
) -> Result<bool, String> {
    match answer {
        Answer::Picked(Ok(place)) if place < frame.house_parts.len() => {
            design.pick_part(&frame.house_parts, place);
            Ok(true)
        }
        Answer::Picked(Err(words)) => Err(words),
        // The designer asks only for a pick, of a part the catalog has.
        _ => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_nav::{HousePart, HousePartKind};

    fn frame() -> WatchFrame {
        WatchFrame {
            house_parts: vec![
                HousePart {
                    kind: HousePartKind::Wall,
                    name: "Stone".into(),
                    pieces: vec![20, 22],
                },
                HousePart {
                    kind: HousePartKind::Roof,
                    name: "Tile Roof".into(),
                    pieces: vec![11314],
                },
            ],
            ..WatchFrame::default()
        }
    }

    #[test]
    fn the_rows_hold_every_step_and_leave_ends_the_changes() {
        let [changes, kept] = command_rows();
        assert_eq!(changes.len() + kept.len(), DESIGN_COMMANDS.len() + 1);
        assert_eq!(changes.last(), Some(&(WORDS_EXIT, ACTION_EXIT)));
        assert_eq!(kept[0], DESIGN_COMMANDS[0]);
        assert_eq!(storey_words(0), "Storey 1");
        assert_eq!(storeys_of(None), DESIGNER_FLOORS);
    }

    #[test]
    fn jev_picks_a_part_of_the_catalog_and_a_failure_says_why() {
        let frame = frame();
        assert!(part_ask(&frame, "  ").is_none());
        assert!(matches!(
            part_ask(&frame, "a roof"),
            Some(Ask::HousePart { options, .. }) if options.len() == 2
        ));
        let mut design = HouseDesign::default();
        assert_eq!(
            take_part_answer(&mut design, &frame, Answer::Picked(Ok(1))),
            Ok(true)
        );
        assert_eq!(design.kind, HousePartKind::Roof);
        assert_eq!(
            take_part_answer(&mut design, &frame, Answer::Picked(Ok(9))),
            Ok(false)
        );
        assert_eq!(
            take_part_answer(&mut design, &frame, Answer::Picked(Err("no".into()))),
            Err("no".into())
        );
    }
}
