//! The rules of the panels that float on the map: what the agent does,
//! the vitals and the pack. Their sizes, which the plan keeps room for,
//! the bars that follow the vitals, the chips of the states, and the rows
//! each panel shows.

use super::theme::{
    ALARM, BAR_HEIGHT, BAR_HEIGHT_MAIN, GOAL, HITS, HITS_POISONED, PANEL_PAD, ROW_GAP, TEXT,
    TEXT_DIM, WAITING,
};
use crate::frame::{Danger, WatchFrame};
use crate::geom::Rgba;

pub const SIDE_PANEL_WIDTH: f32 = 300.0;
pub const PACK_WIDTH: f32 = 340.0;
pub const ROW_HEIGHT: f32 = 20.0;
pub const TITLE_HEIGHT: f32 = 28.0;
pub const BAR_ROW_GAP: f32 = 10.0;
/// The bars of the vitals: hits, mana and stamina.
pub const BAR_COUNT: usize = 3;
/// The most detail rows of the activity panel: the job, the walk goal, the
/// one followed and the script.
const ACTIVITY_MOST_DETAILS: usize = 4;
/// The lists the pack panel shows when they hold anything.
const PACK_LISTS: [&str; 2] = ["Buffs", "Party"];
/// The rows the pack panel always has: the weight with the gold, and the
/// clock.
const PACK_FIXED_ROWS: usize = 2;
/// The tallest each panel grows, for the plan to keep room for it.
pub const ACTIVITY_MOST_HEIGHT: f32 = activity_height(ACTIVITY_MOST_DETAILS);
pub const VITALS_MOST_HEIGHT: f32 = vitals_height(true, true);
pub const PACK_MOST_HEIGHT: f32 = pack_height(PACK_LISTS.len());

/// The share of the way to its goal that a bar covers each second.
const BAR_RATE: f32 = 10.0;
/// The lost part of a bar stays for a moment, then follows more slowly.
const GHOST_RATE: f32 = 1.6;
const BAR_AT_REST: f32 = 0.002;

/// The pack turns to the waiting color at this share of the most weight.
pub const WEIGHT_WARN_SHARE: f32 = 0.9;

const WORDS_IDLE: &str = "Idle";
/// The words of the agent that has no goal.
const NO_GOAL: &str = "-";
const NO_JOB: &str = "-";

/// What one bar shows now, as shares of its full length.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bar {
    pub fill: f32,
    pub ghost: f32,
}

impl Bar {
    /// Moves the bar `dt` seconds toward `goal`. True while it still moves.
    pub fn follow(&mut self, goal: f32, dt: f32) -> bool {
        let step = |rate: f32| 1.0 - (-rate * dt).exp();
        self.fill += (goal - self.fill) * step(BAR_RATE);
        self.ghost = if self.ghost < self.fill {
            self.fill
        } else {
            self.ghost + (self.fill - self.ghost) * step(GHOST_RATE)
        };
        (goal - self.fill).abs() > BAR_AT_REST || (self.ghost - self.fill).abs() > BAR_AT_REST
    }
}

pub fn share(now: u16, max: u16) -> f32 {
    if max == 0 {
        0.0
    } else {
        (f32::from(now) / f32::from(max)).clamp(0.0, 1.0)
    }
}

/// The word with a capital first letter.
pub fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

const fn panel_height(content: f32) -> f32 {
    content + PANEL_PAD * 2.0
}

/// The height of a panel of a title and `lines` rows, such as the message
/// in the middle of the window.
pub const fn message_height(lines: usize) -> f32 {
    panel_height(TITLE_HEIGHT + ROW_HEIGHT * lines as f32)
}

/// The height of the activity panel with `details` rows under its title.
/// With none it tells that the agent is idle.
pub const fn activity_height(details: usize) -> f32 {
    let rows = if details == 0 { 1 } else { details };
    message_height(rows)
}

/// The height of the vitals panel, with the row of states and the row of
/// the fight when they show.
pub const fn vitals_height(states: bool, fights: bool) -> f32 {
    let bars = BAR_COUNT as f32;
    let states_row = if states { ROW_HEIGHT + ROW_GAP } else { 0.0 };
    let fight_row = if fights { ROW_HEIGHT } else { 0.0 };
    panel_height(
        TITLE_HEIGHT
            + states_row
            + BAR_HEIGHT_MAIN
            + BAR_HEIGHT * (bars - 1.0)
            + BAR_ROW_GAP * bars
            + fight_row,
    )
}

/// The height of the pack panel with `lists` lists under its fixed rows.
pub const fn pack_height(lists: usize) -> f32 {
    panel_height(TITLE_HEIGHT + ROW_HEIGHT * (PACK_FIXED_ROWS + lists) as f32 - ROW_GAP)
}

/// The states of the character the vitals show as chips, each in its
/// color.
pub fn states(frame: &WatchFrame) -> Vec<(&'static str, Rgba)> {
    [
        (frame.dead, "Dead", ALARM),
        (frame.war, "War mode", ALARM),
        (frame.poisoned, "Poisoned", HITS_POISONED),
        (frame.paralyzed, "Paralyzed", WAITING),
        (frame.hidden, "Hidden", TEXT_DIM),
    ]
    .into_iter()
    .filter(|(on, _, _)| *on)
    .map(|(_, word, color)| (word, color))
    .collect()
}

/// The fill of the hits bar, and the color of its numbers.
pub fn hits_look(frame: &WatchFrame) -> (Rgba, Rgba) {
    let fill = if frame.poisoned { HITS_POISONED } else { HITS };
    let numbers = if frame.danger() >= Danger::Critical {
        ALARM
    } else {
        TEXT
    };
    (fill, numbers)
}

/// The heading of the activity panel: the goal of the agent, or idle.
pub fn goal_words(frame: &WatchFrame) -> String {
    if frame.goal.is_empty() || frame.goal == NO_GOAL {
        WORDS_IDLE.to_string()
    } else {
        capitalized(&frame.goal)
    }
}

/// What the agent does now, under the heading: its job, its walk goal, the
/// one it follows and its script, each a label, words and their color.
pub fn activity_details(frame: &WatchFrame) -> Vec<(&'static str, String, Rgba)> {
    let mut detail = Vec::new();
    if frame.job != NO_JOB && !frame.job.is_empty() {
        let job = if frame.phase.is_empty() {
            frame.job.clone()
        } else {
            format!("{}, {}", frame.job, frame.phase)
        };
        detail.push(("Job", job, TEXT));
    }
    if let (Some(x), Some(y)) = (frame.dest_x, frame.dest_y) {
        let tiles = uoterm_protocol::types::tile_distance((x, y), (frame.x, frame.y));
        detail.push(("Walks to", format!("{x}, {y}  ({tiles} tiles)"), GOAL));
    }
    if !frame.following.is_empty() {
        detail.push(("Follows", frame.following.clone(), TEXT));
    }
    if !frame.script.is_empty() {
        detail.push(("Script", frame.script.clone(), TEXT));
    }
    detail
}

/// The lists of the pack panel that hold anything, each its label and its
/// words.
pub fn pack_lists(frame: &WatchFrame) -> Vec<(&'static str, String)> {
    PACK_LISTS
        .into_iter()
        .zip([&frame.buffs, &frame.party])
        .filter(|(_, list)| !list.is_empty())
        .map(|(label, list)| (label, list.join(", ")))
        .collect()
}

/// The color of the weight: the waiting color when the pack is nearly
/// full.
pub fn weight_color(frame: &WatchFrame) -> Rgba {
    let heavy = f32::from(frame.weight) >= f32::from(frame.weight_max) * WEIGHT_WARN_SHARE
        && frame.weight_max > 0;
    if heavy {
        WAITING
    } else {
        TEXT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_FRAME: f32 = 1.0 / 60.0;
    const HALF: f32 = 0.5;

    #[test]
    fn a_lost_part_of_a_bar_stays_behind_the_fill() {
        let mut bar = Bar {
            fill: 1.0,
            ghost: 1.0,
        };
        assert!(bar.follow(HALF, ONE_FRAME));
        assert!(bar.fill < 1.0);
        assert!(bar.ghost > bar.fill);
    }

    #[test]
    fn a_bar_that_grows_has_no_ghost() {
        let mut bar = Bar::default();
        bar.follow(HALF, ONE_FRAME);
        assert_eq!(bar.ghost, bar.fill);
    }

    #[test]
    fn a_bar_with_no_maximum_is_empty() {
        assert_eq!(share(10, 0), 0.0);
        assert_eq!(share(30, 20), 1.0);
    }

    #[test]
    fn an_idle_agent_has_no_details_and_a_heavy_pack_warns() {
        let mut frame = WatchFrame::default();
        assert_eq!(goal_words(&frame), WORDS_IDLE);
        assert!(activity_details(&frame).is_empty());
        frame.goal = "loot".into();
        frame.job = "skin".into();
        frame.phase = "walk".into();
        assert_eq!(goal_words(&frame), "Loot");
        assert_eq!(activity_details(&frame)[0].1, "skin, walk");
        frame.weight_max = 100;
        frame.weight = 95;
        assert_eq!(weight_color(&frame), WAITING);
        frame.weight = 50;
        assert_eq!(weight_color(&frame), TEXT);
        frame.poisoned = true;
        assert_eq!(states(&frame), vec![("Poisoned", HITS_POISONED)]);
        assert_eq!(hits_look(&frame).0, HITS_POISONED);
        assert!(pack_lists(&frame).is_empty());
    }

    #[test]
    fn a_panel_is_tallest_with_all_its_rows() {
        assert!(activity_height(0) < ACTIVITY_MOST_HEIGHT);
        assert!(vitals_height(false, false) < VITALS_MOST_HEIGHT);
        assert!(pack_height(0) < PACK_MOST_HEIGHT);
    }
}
