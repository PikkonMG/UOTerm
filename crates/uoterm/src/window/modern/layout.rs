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

use super::super::control_ui::{BAR_MOST_HEIGHT, BAR_WIDTH};
use super::super::deck_ui::hotbar_size;
use super::super::hud::{
    ACTIVITY_MOST_HEIGHT, PACK_MOST_HEIGHT, PACK_WIDTH, SIDE_PANEL_WIDTH, VITALS_MOST_HEIGHT,
};
use super::super::model::places::held_inside;
use super::super::theme;
use super::bars_ui::NEAR_WIDTH;
use super::journal_ui::JOURNAL_LEAST;
use super::radar_ui::panel_size as radar_size;
use eframe::egui::{Align2, Pos2, Rect, Vec2};

/// The room between two panels of the plan.
const GAP: f32 = 12.0;
/// A side column is this wide in the smallest window, and at most this
/// wide in a large one.
const SIDE_LEAST: f32 = 300.0;
const SIDE_MOST: f32 = 400.0;
/// The middle keeps at least this width, for the character and the pack.
const MIDDLE_LEAST: f32 = 424.0;
/// How far each panel of a kind stands from the one before it.
const CASCADE_STEP: f32 = 28.0;
/// How far each strip over or under the middle stands from the one before
/// it: the height of the tallest strip and a gap.
const STRIP_STEP: f32 = 96.0;
/// The room a container keeps clear left of the middle of the window,
/// where the character stands: half the width of a figure, and a gap.
const FIGURE_HALF_WIDTH: f32 = 30.0;

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

/// Where a spot is: a room the panel fills up to its own size, or a point
/// it hangs from.
enum Hold {
    Room(Rect, Align2),
    At(Pos2, Align2),
}

/// The parts of the window the plan lays the panels in.
struct Plan {
    /// The room of the control bar, as wide as the window.
    bar: Rect,
    /// The window under the bar, less its margin.
    below_bar: Rect,
    left: Rect,
    right: Rect,
    middle: Rect,
    /// Where the character stands across the window: its middle.
    character_x: f32,
}

impl Plan {
    fn of(window: Rect) -> Self {
        let area = window.shrink(theme::SCREEN_MARGIN);
        let bar = Rect::from_min_size(area.min, Vec2::new(area.width(), BAR_MOST_HEIGHT));
        let below_bar = Rect::from_min_max(Pos2::new(area.left(), bar.bottom() + GAP), area.max);
        let side = ((below_bar.width() - MIDDLE_LEAST) / 2.0 - GAP).clamp(SIDE_LEAST, SIDE_MOST);
        let left = Rect::from_min_size(below_bar.min, Vec2::new(side, below_bar.height()));
        let right = Rect::from_min_max(
            Pos2::new(below_bar.right() - side, below_bar.top()),
            below_bar.max,
        );
        let middle = Rect::from_min_max(
            Pos2::new(left.right() + GAP, below_bar.top()),
            Pos2::new(right.left() - GAP, below_bar.bottom()),
        );
        Self {
            bar,
            below_bar,
            left,
            right,
            middle,
            character_x: window.center().x,
        }
    }

    /// The room right of the panels of the left column, left of the
    /// character, under the bar and over the hotbar.
    fn beside_character(&self) -> Rect {
        let panels_right = self.left.left() + SIDE_PANEL_WIDTH.max(NEAR_WIDTH);
        Rect::from_min_max(
            Pos2::new(panels_right + GAP, self.below_bar.top()),
            Pos2::new(
                self.character_x - FIGURE_HALF_WIDTH,
                self.hotbar().top() - GAP,
            ),
        )
    }

    /// Where the containers open: beside the character when the first one
    /// fits there whole, and else in the room of the near list.
    fn container(&self, n: usize, size: Vec2) -> Spot {
        let room = self.beside_character().size();
        if room.x >= size.x && room.y >= size.y {
            Spot::Container(n)
        } else {
            Spot::NearRoom(n)
        }
    }

    fn activity(&self) -> Rect {
        top_of(self.left, ACTIVITY_MOST_HEIGHT)
    }

    fn vitals(&self) -> Rect {
        foot_of(self.left, VITALS_MOST_HEIGHT)
    }

    fn near(&self) -> Rect {
        Rect::from_min_max(
            Pos2::new(self.left.left(), self.activity().bottom() + GAP),
            Pos2::new(self.left.right(), self.vitals().top() - GAP),
        )
    }

    fn pack(&self) -> Rect {
        foot_of(self.middle, PACK_MOST_HEIGHT)
    }

    fn hotbar(&self) -> Rect {
        let above_pack = Rect::from_min_max(
            self.middle.min,
            Pos2::new(self.middle.right(), self.pack().top() - GAP),
        );
        foot_of(above_pack, hotbar_size(PACK_WIDTH).y)
    }

    fn hold(&self, spot: Spot) -> Hold {
        let step = |n: usize, x: f32| Vec2::new(x, 1.0) * CASCADE_STEP * n as f32;
        let strip = |n: usize| STRIP_STEP * n as f32;
        match spot {
            Spot::ControlBar => Hold::Room(self.bar, Align2::CENTER_TOP),
            Spot::Activity => Hold::Room(self.activity(), Align2::LEFT_TOP),
            Spot::Near => Hold::Room(self.near(), Align2::LEFT_TOP),
            Spot::Vitals => Hold::Room(self.vitals(), Align2::LEFT_BOTTOM),
            // The radar may grow over the journal, as far as the journal
            // keeps its least height.
            Spot::Radar => Hold::Room(
                top_of(self.right, self.right.height() - JOURNAL_LEAST.y - GAP),
                Align2::RIGHT_TOP,
            ),
            Spot::Journal => Hold::Room(
                Rect::from_min_max(
                    Pos2::new(
                        self.right.left(),
                        self.right.top() + radar_size(false).y + GAP,
                    ),
                    self.right.max,
                ),
                Align2::RIGHT_BOTTOM,
            ),
            Spot::Hotbar => Hold::Room(self.hotbar(), Align2::CENTER_BOTTOM),
            Spot::Pack => Hold::Room(self.pack(), Align2::CENTER_BOTTOM),
            Spot::Launcher => Hold::At(
                Pos2::new(
                    self.below_bar.center().x - BAR_WIDTH / 2.0,
                    self.below_bar.top(),
                ),
                Align2::LEFT_TOP,
            ),
            Spot::Middle(n) => Hold::At(
                self.below_bar.center() + step(n, 1.0),
                Align2::CENTER_CENTER,
            ),
            Spot::LeftColumn(n) => Hold::At(self.left.left_top() + step(n, 1.0), Align2::LEFT_TOP),
            Spot::NearRoom(n) => Hold::Room(self.near().translate(step(n, 1.0)), Align2::LEFT_TOP),
            Spot::Container(n) => Hold::At(
                self.beside_character().left_top() + step(n, 1.0),
                Align2::LEFT_TOP,
            ),
            Spot::RightColumn(n) => {
                Hold::At(self.right.right_top() + step(n, -1.0), Align2::RIGHT_TOP)
            }
            Spot::MiddleTop(n) => Hold::At(
                self.middle.center_top() + Vec2::new(0.0, strip(n)),
                Align2::CENTER_TOP,
            ),
            Spot::MiddleBottom(n) => Hold::At(
                Pos2::new(self.middle.center().x, self.hotbar().top() - GAP - strip(n)),
                Align2::CENTER_BOTTOM,
            ),
            Spot::OverVitals => Hold::At(
                Pos2::new(self.left.left(), self.vitals().top() - GAP),
                Align2::LEFT_BOTTOM,
            ),
        }
    }
}

/// The top of `column`, `height` high.
fn top_of(column: Rect, height: f32) -> Rect {
    Rect::from_min_size(column.min, Vec2::new(column.width(), height))
}

/// The foot of `column`, `height` high.
fn foot_of(column: Rect, height: f32) -> Rect {
    Rect::from_min_max(
        Pos2::new(column.left(), column.bottom() - height),
        column.max,
    )
}

/// The first place of a panel of `size` at `spot` in `window`. A panel in
/// a room of its own is made smaller when the room is; every other panel
/// is held under the control bar.
pub fn first_place(window: Rect, spot: Spot, size: Vec2) -> Rect {
    let plan = Plan::of(window);
    let spot = match spot {
        Spot::Container(n) => plan.container(n, size),
        other => other,
    };
    let rect = match plan.hold(spot) {
        Hold::Room(room, align) => align.align_size_within_rect(size.min(room.size()), room),
        Hold::At(point, align) => align.anchor_size(point, size),
    };
    let room = if spot == Spot::ControlBar {
        plan.bar
    } else {
        plan.below_bar
    };
    held_inside(rect, room)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{MOBILE_LINES, WINDOW_HEIGHT, WINDOW_WIDTH};
    use crate::window::hud::SIDE_PANEL_WIDTH;
    use crate::window::modern::bars_ui::{near_size, NEAR_LEAST};
    use crate::window::modern::grid_ui::first_size;
    use crate::window::modern::journal_ui::{JOURNAL_HEIGHT, JOURNAL_WIDTH};
    use crate::window::settings::Profile;
    use crate::window::{WINDOW_MIN_HEIGHT, WINDOW_MIN_WIDTH};

    /// A panel far larger than any window.
    const HUGE: Vec2 = Vec2::new(4000.0, 4000.0);
    const SMALL: Vec2 = Vec2::new(200.0, 120.0);
    const MOST_OF_A_KIND: usize = 6;
    /// Two sizes this near are the same size, for the rounding of floats.
    const SAME_SIZE: f32 = 0.01;
    /// The room the first panels keep clear around the character, who
    /// stands in the middle of the window. The figure rises above the tile
    /// it stands on, so the room stands a little higher.
    const CHARACTER_ROOM: Vec2 = Vec2::new(320.0, 180.0);
    const CHARACTER_RISE: f32 = 20.0;

    fn character_room(window: Rect) -> Rect {
        Rect::from_center_size(
            window.center() - Vec2::new(0.0, CHARACTER_RISE),
            CHARACTER_ROOM,
        )
    }

    fn windows() -> [Rect; 2] {
        [
            Rect::from_min_size(Pos2::ZERO, Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT)),
        ]
    }

    /// Every panel that shows at the first start, at its largest, with the
    /// control bar at its tallest.
    fn first_panels(window: Rect) -> Vec<(&'static str, Rect)> {
        let at = |spot, size| first_place(window, spot, size);
        vec![
            (
                "control bar",
                at(Spot::ControlBar, Vec2::new(BAR_WIDTH, BAR_MOST_HEIGHT)),
            ),
            (
                "activity",
                at(
                    Spot::Activity,
                    Vec2::new(SIDE_PANEL_WIDTH, ACTIVITY_MOST_HEIGHT),
                ),
            ),
            ("near", at(Spot::Near, near_size(MOBILE_LINES))),
            (
                "vitals",
                at(
                    Spot::Vitals,
                    Vec2::new(SIDE_PANEL_WIDTH, VITALS_MOST_HEIGHT),
                ),
            ),
            ("radar", at(Spot::Radar, radar_size(false))),
            (
                "journal",
                at(Spot::Journal, Vec2::new(JOURNAL_WIDTH, JOURNAL_HEIGHT)),
            ),
            ("hotbar", at(Spot::Hotbar, hotbar_size(PACK_WIDTH))),
            (
                "pack",
                at(Spot::Pack, Vec2::new(PACK_WIDTH, PACK_MOST_HEIGHT)),
            ),
        ]
    }

    #[test]
    fn the_first_panels_never_lie_on_each_other_on_the_bar_or_on_the_character() {
        for window in windows() {
            let panels = first_panels(window);
            let character = character_room(window);
            for (at, (name, panel)) in panels.iter().enumerate() {
                assert!(
                    window.contains_rect(*panel),
                    "{name} leaves the window {window:?}"
                );
                assert!(
                    !panel.intersects(character),
                    "{name} lies on the character in {window:?}"
                );
                for (other, rect) in &panels[at + 1..] {
                    assert!(
                        !panel.intersects(*rect),
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
                    .map(|(_, rect)| rect.size())
                    .unwrap_or_default()
            };
            let whole = [
                (
                    "activity",
                    Vec2::new(SIDE_PANEL_WIDTH, ACTIVITY_MOST_HEIGHT),
                ),
                ("vitals", Vec2::new(SIDE_PANEL_WIDTH, VITALS_MOST_HEIGHT)),
                ("pack", Vec2::new(PACK_WIDTH, PACK_MOST_HEIGHT)),
                ("radar", radar_size(false)),
                ("hotbar", hotbar_size(PACK_WIDTH)),
            ];
            for (name, wanted) in whole {
                assert!(
                    (size(name) - wanted).length() < SAME_SIZE,
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
                Vec2::new(BAR_WIDTH, BAR_MOST_HEIGHT),
            );
            for n in 0..MOST_OF_A_KIND {
                for spot in later(n) {
                    for size in [HUGE, SMALL] {
                        let panel = first_place(window, spot, size);
                        assert!(window.contains_rect(panel), "{spot:?} leaves {window:?}");
                        assert!(
                            panel.top() > bar.bottom(),
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
            (container.size() - size).length() < SAME_SIZE,
            "it keeps its size"
        );
        assert!(window.contains_rect(container));
        for (name, panel) in first_panels(window) {
            assert!(!container.intersects(panel), "the container lies on {name}");
        }
        assert!(
            container.right() < window.center().x - FIGURE_HALF_WIDTH,
            "the container lies on the character"
        );
        let second = first_place(window, Spot::Container(1), size);
        assert_eq!(second.min - container.min, Vec2::splat(CASCADE_STEP));
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
        assert_eq!(second.min - first.min, Vec2::splat(CASCADE_STEP));
        assert_eq!(first.center(), Plan::of(window).below_bar.center());
        let right = first_place(window, Spot::RightColumn(1), SMALL);
        let right_first = first_place(window, Spot::RightColumn(0), SMALL);
        assert_eq!(
            right.min - right_first.min,
            Vec2::new(-CASCADE_STEP, CASCADE_STEP)
        );
    }
}
