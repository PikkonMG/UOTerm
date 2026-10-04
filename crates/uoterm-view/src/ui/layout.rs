//! The first place of every Modern panel: one plan for the whole window.
//!
//! The control bar has the middle of the top. Under it, the left column
//! holds what the agent does at its top, the near list, and the vitals at
//! its foot; the right column holds the radar at its top and the journal
//! at its foot; the middle holds the pack at its foot with the hotbar on
//! it. So the panels that show at the first start never lie on each other
//! or on the bar, and the middle of the world, where the character stands,
//! stays clear. A panel that opens later stands in the middle of the room
//! under the bar, a step from the one before it, at the top of a column,
//! or over or under the middle. The containers open under the bar between
//! the panels of the left column and the character, and in a window too
//! small for that, in the room of the near list, over it. No first place
//! is ever under the bar.

use super::bars::NEAR_WIDTH;
use super::control_bar::{BAR_MOST_HEIGHT, BAR_WIDTH};
use super::deck::hotbar_size;
use super::hud::{
    ACTIVITY_MOST_HEIGHT, PACK_MOST_HEIGHT, PACK_WIDTH, SIDE_PANEL_WIDTH, VITALS_MOST_HEIGHT,
};
use super::lists::{radar_panel_size, JOURNAL_LEAST};
use super::theme::SCREEN_MARGIN;
use crate::geom::{Area, Point, Vector};
use crate::model::places::held_inside;

/// The room between two panels of the plan.
pub const GAP: f32 = 12.0;
/// A side column is this wide in the smallest window, and at most this
/// wide in a large one.
const SIDE_LEAST: f32 = 300.0;
const SIDE_MOST: f32 = 400.0;
/// The middle keeps at least this width, for the character and the pack.
const MIDDLE_LEAST: f32 = 424.0;
/// How far each panel of a kind stands from the one before it.
pub const CASCADE_STEP: f32 = 28.0;
/// How far each strip over or under the middle stands from the one before
/// it: the height of the tallest strip and a gap.
pub const STRIP_STEP: f32 = 96.0;
/// The room a container keeps clear left of the middle of the window,
/// where the character stands: half the width of a figure, and a gap.
pub const FIGURE_HALF_WIDTH: f32 = 30.0;
const HALF: f32 = 0.5;

/// Where a panel first stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    /// The control bar, at the middle of the top.
    ControlBar,
    /// The panels that show at the first start: each has a room of its
    /// own.
    Activity,
    Near,
    Vitals,
    Radar,
    Journal,
    Hotbar,
    Pack,
    /// The panel launcher, under the left end of the control bar.
    Launcher,
    /// The middle of the room under the bar, a step down and right for
    /// each panel of a kind before it.
    Middle(usize),
    /// The top of the left column, a step down and right for each one
    /// before it.
    LeftColumn(usize),
    /// The room of the near list, a step down and right for each one
    /// before it.
    NearRoom(usize),
    /// The containers, a step down and right for each one before it: under
    /// the bar, between the panels of the left column and the character,
    /// so they hide neither. In a window with no such room for the first
    /// one, the room of the near list.
    Container(usize),
    /// The top of the right column, a step down and left for each one
    /// before it.
    RightColumn(usize),
    /// The top of the middle, a strip lower for each one before it.
    MiddleTop(usize),
    /// Over the hotbar, a strip higher for each one before it.
    MiddleBottom(usize),
    /// The left column, over the vitals.
    OverVitals,
}

/// Where a panel lies against a room or a point, across and down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Start,
    Middle,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Align(Edge, Edge);

const LEFT_TOP: Align = Align(Edge::Start, Edge::Start);
const LEFT_BOTTOM: Align = Align(Edge::Start, Edge::End);
const CENTER_TOP: Align = Align(Edge::Middle, Edge::Start);
const CENTER_CENTER: Align = Align(Edge::Middle, Edge::Middle);
const CENTER_BOTTOM: Align = Align(Edge::Middle, Edge::End);
const RIGHT_TOP: Align = Align(Edge::End, Edge::Start);
const RIGHT_BOTTOM: Align = Align(Edge::End, Edge::End);

impl Edge {
    /// The start of a length `size` at this edge of the span `min..max`.
    fn within(self, size: f32, min: f32, max: f32) -> f32 {
        match self {
            Edge::Start => min,
            Edge::Middle => (min + max) * HALF - size * HALF,
            Edge::End => max - size,
        }
    }

    /// The start of a length `size` that hangs from `at` by this edge.
    fn from(self, size: f32, at: f32) -> f32 {
        match self {
            Edge::Start => at,
            Edge::Middle => at - size * HALF,
            Edge::End => at - size,
        }
    }
}

impl Align {
    /// An area of `size` at this side of `room`.
    fn within(self, size: Vector, room: Area) -> Area {
        let left = self.0.within(size.x, room.min.x, room.max.x);
        let top = self.1.within(size.y, room.min.y, room.max.y);
        Area::from_min_size(Point::new(left, top), size)
    }

    /// An area of `size` that hangs from `point` by this side.
    fn hung_from(self, point: Point, size: Vector) -> Area {
        let left = self.0.from(size.x, point.x);
        let top = self.1.from(size.y, point.y);
        Area::from_min_size(Point::new(left, top), size)
    }
}

/// Where a spot is: a room the panel fills up to its own size, or a point
/// it hangs from.
enum Hold {
    Room(Area, Align),
    At(Point, Align),
}

/// The parts of the window the plan lays the panels in.
pub struct Plan {
    /// The room of the control bar, as wide as the window.
    bar: Area,
    /// The window under the bar, less its margin.
    below_bar: Area,
    left: Area,
    right: Area,
    middle: Area,
    /// Where the character stands across the window: its middle.
    character_x: f32,
}

impl Plan {
    pub fn of(window: Area) -> Self {
        let area = window.expand(-SCREEN_MARGIN);
        let bar = Area::from_min_size(area.min, Vector::new(area.width(), BAR_MOST_HEIGHT));
        let below_bar = Area {
            min: Point::new(area.min.x, bar.max.y + GAP),
            max: area.max,
        };
        let side = ((below_bar.width() - MIDDLE_LEAST) / 2.0 - GAP).clamp(SIDE_LEAST, SIDE_MOST);
        let left = Area::from_min_size(below_bar.min, Vector::new(side, below_bar.height()));
        let right = Area {
            min: Point::new(below_bar.max.x - side, below_bar.min.y),
            max: below_bar.max,
        };
        let middle = Area {
            min: Point::new(left.max.x + GAP, below_bar.min.y),
            max: Point::new(right.min.x - GAP, below_bar.max.y),
        };
        Self {
            bar,
            below_bar,
            left,
            right,
            middle,
            character_x: window.center().x,
        }
    }

    /// The window under the bar, less its margin.
    pub fn below_bar(&self) -> Area {
        self.below_bar
    }

    /// The room right of the panels of the left column, left of the
    /// character, under the bar and over the hotbar.
    fn beside_character(&self) -> Area {
        let panels_right = self.left.min.x + SIDE_PANEL_WIDTH.max(NEAR_WIDTH);
        Area {
            min: Point::new(panels_right + GAP, self.below_bar.min.y),
            max: Point::new(
                self.character_x - FIGURE_HALF_WIDTH,
                self.hotbar().min.y - GAP,
            ),
        }
    }

    /// Where the containers open: beside the character when the first one
    /// fits there whole, and else in the room of the near list.
    fn container(&self, n: usize, size: Vector) -> Spot {
        let room = self.beside_character().size();
        if room.x >= size.x && room.y >= size.y {
            Spot::Container(n)
        } else {
            Spot::NearRoom(n)
        }
    }

    fn activity(&self) -> Area {
        top_of(self.left, ACTIVITY_MOST_HEIGHT)
    }

    fn vitals(&self) -> Area {
        foot_of(self.left, VITALS_MOST_HEIGHT)
    }

    fn near(&self) -> Area {
        Area {
            min: Point::new(self.left.min.x, self.activity().max.y + GAP),
            max: Point::new(self.left.max.x, self.vitals().min.y - GAP),
        }
    }

    fn pack(&self) -> Area {
        foot_of(self.middle, PACK_MOST_HEIGHT)
    }

    fn hotbar(&self) -> Area {
        let above_pack = Area {
            min: self.middle.min,
            max: Point::new(self.middle.max.x, self.pack().min.y - GAP),
        };
        foot_of(above_pack, hotbar_size(PACK_WIDTH).y)
    }

    fn hold(&self, spot: Spot) -> Hold {
        let step = |n: usize, x: f32| Vector::new(x, 1.0) * (CASCADE_STEP * n as f32);
        let strip = |n: usize| STRIP_STEP * n as f32;
        match spot {
            Spot::ControlBar => Hold::Room(self.bar, CENTER_TOP),
            Spot::Activity => Hold::Room(self.activity(), LEFT_TOP),
            Spot::Near => Hold::Room(self.near(), LEFT_TOP),
            Spot::Vitals => Hold::Room(self.vitals(), LEFT_BOTTOM),
            // The radar may grow over the journal, as far as the journal
            // keeps its least height.
            Spot::Radar => Hold::Room(
                top_of(self.right, self.right.height() - JOURNAL_LEAST.y - GAP),
                RIGHT_TOP,
            ),
            Spot::Journal => Hold::Room(
                Area {
                    min: Point::new(
                        self.right.min.x,
                        self.right.min.y + radar_panel_size(false).y + GAP,
                    ),
                    max: self.right.max,
                },
                RIGHT_BOTTOM,
            ),
            Spot::Hotbar => Hold::Room(self.hotbar(), CENTER_BOTTOM),
            Spot::Pack => Hold::Room(self.pack(), CENTER_BOTTOM),
            Spot::Launcher => Hold::At(
                Point::new(
                    self.below_bar.center().x - BAR_WIDTH / 2.0,
                    self.below_bar.min.y,
                ),
                LEFT_TOP,
            ),
            Spot::Middle(n) => Hold::At(self.below_bar.center() + step(n, 1.0), CENTER_CENTER),
            Spot::LeftColumn(n) => Hold::At(self.left.min + step(n, 1.0), LEFT_TOP),
            Spot::NearRoom(n) => Hold::Room(self.near().translate(step(n, 1.0)), LEFT_TOP),
            Spot::Container(n) => Hold::At(self.beside_character().min + step(n, 1.0), LEFT_TOP),
            Spot::RightColumn(n) => Hold::At(
                Point::new(self.right.max.x, self.right.min.y) + step(n, -1.0),
                RIGHT_TOP,
            ),
            Spot::MiddleTop(n) => Hold::At(
                Point::new(self.middle.center().x, self.middle.min.y + strip(n)),
                CENTER_TOP,
            ),
            Spot::MiddleBottom(n) => Hold::At(
                Point::new(self.middle.center().x, self.hotbar().min.y - GAP - strip(n)),
                CENTER_BOTTOM,
            ),
            Spot::OverVitals => Hold::At(
                Point::new(self.left.min.x, self.vitals().min.y - GAP),
                LEFT_BOTTOM,
            ),
        }
    }
}

/// The top of `column`, `height` high.
fn top_of(column: Area, height: f32) -> Area {
    Area::from_min_size(column.min, Vector::new(column.width(), height))
}

/// The foot of `column`, `height` high.
fn foot_of(column: Area, height: f32) -> Area {
    Area {
        min: Point::new(column.min.x, column.max.y - height),
        max: column.max,
    }
}

/// The first place of a panel of `size` at `spot` in `window`. A panel in
/// a room of its own is made smaller when the room is; every other panel
/// is held under the control bar.
pub fn first_place(window: Area, spot: Spot, size: Vector) -> Area {
    let plan = Plan::of(window);
    let spot = match spot {
        Spot::Container(n) => plan.container(n, size),
        other => other,
    };
    let area = match plan.hold(spot) {
        Hold::Room(room, align) => {
            let room_size = room.size();
            let fitted = Vector::new(size.x.min(room_size.x), size.y.min(room_size.y));
            align.within(fitted, room)
        }
        Hold::At(point, align) => align.hung_from(point, size),
    };
    let room = if spot == Spot::ControlBar {
        plan.bar
    } else {
        plan.below_bar
    };
    held_inside(area, room)
}

#[cfg(test)]
impl Spot {
    /// One spot of each kind; the cascading kinds at their first place and
    /// at their fourth.
    fn every_kind() -> Vec<Spot> {
        const FOURTH: usize = 3;
        let mut spots = vec![
            Spot::ControlBar,
            Spot::Activity,
            Spot::Near,
            Spot::Vitals,
            Spot::Radar,
            Spot::Journal,
            Spot::Hotbar,
            Spot::Pack,
            Spot::Launcher,
            Spot::OverVitals,
        ];
        for n in [0, FOURTH] {
            spots.extend([
                Spot::Middle(n),
                Spot::LeftColumn(n),
                Spot::NearRoom(n),
                Spot::Container(n),
                Spot::RightColumn(n),
                Spot::MiddleTop(n),
                Spot::MiddleBottom(n),
            ]);
        }
        spots
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{MOBILE_LINES, WINDOW_HEIGHT, WINDOW_WIDTH};
    use crate::settings::{Profile, WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};
    use crate::ui::bars::{near_size, NEAR_LEAST};
    use crate::ui::grid_clicks::first_size;
    use crate::ui::lists::{JOURNAL_HEIGHT, JOURNAL_WIDTH};

    /// A panel far larger than any window.
    const HUGE: Vector = Vector::new(4000.0, 4000.0);
    const SMALL: Vector = Vector::new(200.0, 120.0);
    const MOST_OF_A_KIND: usize = 6;
    /// Two sizes this near are the same size, for the rounding of floats.
    const SAME_SIZE: f32 = 0.01;
    /// The room the first panels keep clear around the character, who
    /// stands in the middle of the window. The figure rises above the tile
    /// it stands on, so the room stands a little higher.
    const CHARACTER_ROOM: Vector = Vector::new(320.0, 180.0);
    const CHARACTER_RISE: f32 = 20.0;

    fn character_room(window: Area) -> Area {
        let center = window.center();
        Area::from_center_size(
            Point::new(center.x, center.y - CHARACTER_RISE),
            CHARACTER_ROOM,
        )
    }

    fn windows() -> [Area; 2] {
        [
            Area::from_min_size(Point::default(), Vector::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
            Area::from_min_size(
                Point::default(),
                Vector::new(WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT),
            ),
        ]
    }

    fn holds(room: Area, area: Area) -> bool {
        room.contains(area.min) && room.contains(area.max)
    }

    /// True when the two areas share more than an edge.
    fn overlap(a: Area, b: Area) -> bool {
        a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
    }

    fn differs_by(a: Vector, b: Vector) -> f32 {
        Vector::new(a.x - b.x, a.y - b.y).length()
    }

    /// Every panel that shows at the first start, at its largest, with the
    /// control bar at its tallest.
    fn first_panels(window: Area) -> Vec<(&'static str, Area)> {
        let at = |spot, size| first_place(window, spot, size);
        vec![
            (
                "control bar",
                at(Spot::ControlBar, Vector::new(BAR_WIDTH, BAR_MOST_HEIGHT)),
            ),
            (
                "activity",
                at(
                    Spot::Activity,
                    Vector::new(SIDE_PANEL_WIDTH, ACTIVITY_MOST_HEIGHT),
                ),
            ),
            ("near", at(Spot::Near, near_size(MOBILE_LINES))),
            (
                "vitals",
                at(
                    Spot::Vitals,
                    Vector::new(SIDE_PANEL_WIDTH, VITALS_MOST_HEIGHT),
                ),
            ),
            ("radar", at(Spot::Radar, radar_panel_size(false))),
            (
                "journal",
                at(Spot::Journal, Vector::new(JOURNAL_WIDTH, JOURNAL_HEIGHT)),
            ),
            ("hotbar", at(Spot::Hotbar, hotbar_size(PACK_WIDTH))),
            (
                "pack",
                at(Spot::Pack, Vector::new(PACK_WIDTH, PACK_MOST_HEIGHT)),
            ),
        ]
    }

    #[test]
    fn the_first_panels_never_lie_on_each_other_on_the_bar_or_on_the_character() {
        for window in windows() {
            let panels = first_panels(window);
            let character = character_room(window);
            for (at, (name, panel)) in panels.iter().enumerate() {
                assert!(holds(window, *panel), "{name} leaves the window {window:?}");
                assert!(
                    !overlap(*panel, character),
                    "{name} lies on the character in {window:?}"
                );
                for (other, area) in &panels[at + 1..] {
                    assert!(
                        !overlap(*panel, *area),
                        "{name} lies on {other} in {window:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_first_panels_keep_their_whole_size_or_their_least_one() {
        for window in windows() {
            let panels = first_panels(window);
            let size = |wanted: &str| {
                panels
                    .iter()
                    .find(|(name, _)| *name == wanted)
                    .map(|(_, area)| area.size())
                    .unwrap_or_default()
            };
            let whole = [
                (
                    "activity",
                    Vector::new(SIDE_PANEL_WIDTH, ACTIVITY_MOST_HEIGHT),
                ),
                ("vitals", Vector::new(SIDE_PANEL_WIDTH, VITALS_MOST_HEIGHT)),
                ("pack", Vector::new(PACK_WIDTH, PACK_MOST_HEIGHT)),
                ("radar", radar_panel_size(false)),
                ("hotbar", hotbar_size(PACK_WIDTH)),
            ];
            for (name, wanted) in whole {
                assert!(
                    differs_by(size(name), wanted) < SAME_SIZE,
                    "{name} is cut in {window:?}"
                );
            }
            assert!(size("near").y >= NEAR_LEAST.y, "{window:?}");
            let journal = size("journal");
            assert!(
                journal.x >= JOURNAL_LEAST.x && journal.y >= JOURNAL_LEAST.y,
                "{window:?}"
            );
        }
    }

    #[test]
    fn a_panel_that_opens_later_is_never_under_the_control_bar() {
        let later = |n| {
            [
                Spot::Launcher,
                Spot::Middle(n),
                Spot::LeftColumn(n),
                Spot::NearRoom(n),
                Spot::Container(n),
                Spot::RightColumn(n),
                Spot::MiddleTop(n),
                Spot::MiddleBottom(n),
                Spot::OverVitals,
            ]
        };
        for window in windows() {
            let bar = first_place(
                window,
                Spot::ControlBar,
                Vector::new(BAR_WIDTH, BAR_MOST_HEIGHT),
            );
            for n in 0..MOST_OF_A_KIND {
                for spot in later(n) {
                    for size in [HUGE, SMALL] {
                        let panel = first_place(window, spot, size);
                        assert!(holds(window, panel), "{spot:?} leaves {window:?}");
                        assert!(
                            panel.min.y > bar.max.y,
                            "{spot:?} of {size:?} is under the bar in {window:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_first_container_hides_no_first_panel_nor_the_character() {
        let window = windows()[0];
        let size = first_size(&Profile::default());
        let container = first_place(window, Spot::Container(0), size);
        assert!(
            differs_by(container.size(), size) < SAME_SIZE,
            "it keeps its size"
        );
        assert!(holds(window, container));
        for (name, panel) in first_panels(window) {
            assert!(!overlap(container, panel), "the container lies on {name}");
        }
        assert!(
            container.max.x < window.center().x - FIGURE_HALF_WIDTH,
            "the container lies on the character"
        );
        let second = first_place(window, Spot::Container(1), size);
        assert_eq!(
            second.min - container.min,
            Vector::new(CASCADE_STEP, CASCADE_STEP)
        );
    }

    #[test]
    fn a_container_opens_over_the_near_list_in_a_window_too_small_for_it() {
        let window = windows()[1];
        let size = first_size(&Profile::default());
        for n in 0..MOST_OF_A_KIND {
            assert_eq!(
                first_place(window, Spot::Container(n), size),
                first_place(window, Spot::NearRoom(n), size)
            );
        }
    }

    #[test]
    fn panels_of_a_kind_step_apart_from_the_middle_and_the_column() {
        let window = windows()[0];
        let first = first_place(window, Spot::Middle(0), SMALL);
        let second = first_place(window, Spot::Middle(1), SMALL);
        assert_eq!(
            second.min - first.min,
            Vector::new(CASCADE_STEP, CASCADE_STEP)
        );
        assert_eq!(first.center(), Plan::of(window).below_bar().center());
        let right = first_place(window, Spot::RightColumn(1), SMALL);
        let right_first = first_place(window, Spot::RightColumn(0), SMALL);
        assert_eq!(
            right.min - right_first.min,
            Vector::new(-CASCADE_STEP, CASCADE_STEP)
        );
    }
}

#[cfg(test)]
mod place_tests {
    use super::*;
    use crate::geom::{Area, Point, Vector};

    const WINDOW: Vector = Vector {
        x: 1280.0,
        y: 800.0,
    };

    #[test]
    fn every_spot_lands_inside_the_window() {
        let window = Area::from_min_size(Point::default(), WINDOW);
        for spot in Spot::every_kind() {
            let placed = first_place(window, spot, Vector::new(300.0, 200.0));
            assert!(
                window.contains(placed.min) && window.contains(placed.max),
                "{spot:?}"
            );
        }
    }

    #[test]
    fn a_small_window_still_holds_the_journal() {
        let window = Area::from_min_size(Point::default(), Vector::new(800.0, 600.0));
        let journal = first_place(window, Spot::Journal, Vector::new(360.0, 240.0));
        assert!(window.contains(journal.max));
    }
}
