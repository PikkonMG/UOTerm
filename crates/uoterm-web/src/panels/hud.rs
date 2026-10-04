//! The panels that float on the map: what the agent does, the vitals and
//! the pack. Each stands at its own spot of the plan; the player moves and
//! locks it. What each row shows is `uoterm_view::ui::hud`.

use super::{Colored, FrameSpec, Framed, PANEL_ACTIVITY, PANEL_PACK, PANEL_VITALS};
use crate::WebView;
use serde::Serialize;
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Rgba, Vector};
use uoterm_view::ui::hud::{
    activity_details, activity_height, clock_words, goal_words, hits_look, pack_height, pack_lists,
    share, states, vitals_height, weight_color, weight_words, Bar, ACTIVITY_ID, BAR_COUNT, PACK_ID,
    PACK_WIDTH, SIDE_PANEL_WIDTH, VITALS_ID, WORDS_FIGHTS, WORDS_GOLD, WORDS_HITS, WORDS_MANA,
    WORDS_NO_JOB, WORDS_PACK, WORDS_STAMINA, WORDS_TIME, WORDS_WEIGHT,
};
use uoterm_view::ui::layout::{first_place, Spot};
use uoterm_view::ui::theme::{css_color, ALARM, MANA, NOTO_SELF, STAM, TEXT, TEXT_DIM, TEXT_FAINT};

/// The bars of the vitals as they move toward the vitals now.
#[derive(Default)]
pub(crate) struct HudBars {
    bars: [Bar; BAR_COUNT],
    /// The clock of the last move.
    last: Option<f64>,
}

/// What the agent does: its goal, then the details.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ActivityData {
    pub heading: String,
    /// The words of an agent with nothing to do.
    pub idle: Option<Colored>,
    pub details: Vec<Detail>,
}

/// A dim label and its value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Detail {
    pub label: &'static str,
    pub value: Colored,
}

/// The name, the states, the bars and the fight.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VitalsData {
    pub name: Colored,
    pub states: Vec<Colored>,
    pub bars: Vec<VitalBar>,
    pub fights: Option<Detail>,
}

/// One bar of the vitals.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct VitalBar {
    pub label: &'static str,
    pub numbers: Colored,
    /// The shares of the bar that are full, and that were lost a moment
    /// ago.
    pub fill: f32,
    pub ghost: f32,
    pub color: String,
    /// The hits bar is the large one.
    pub main: bool,
}

/// The pack: the gold, the weight, the clock and the lists.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PackData {
    pub heading: &'static str,
    pub gold: Colored,
    pub gold_words: &'static str,
    pub rows: Vec<Detail>,
}

fn colored(words: impl Into<String>, color: Rgba) -> Colored {
    Colored {
        words: words.into(),
        color: css_color(color),
    }
}

impl WebView {
    /// Moves the bars of the vitals toward the vitals of `frame` at `time`.
    /// True while a bar still moves.
    pub(crate) fn follow_vitals(&mut self, frame: &WatchFrame, time: f64) -> bool {
        let hud = &mut self.panels.hud;
        let dt = hud.last.map_or(0.0, |last| (time - last).max(0.0)) as f32;
        hud.last = Some(time);
        let goals = [
            share(frame.hits, frame.hits_max),
            share(frame.mana, frame.mana_max),
            share(frame.stam, frame.stam_max),
        ];
        let mut moving = false;
        for (bar, goal) in hud.bars.iter_mut().zip(goals) {
            moving |= bar.follow(goal, dt);
        }
        moving
    }

    pub(super) fn activity_spec(&self, frame: &WatchFrame) -> FrameSpec {
        let size = Vector::new(
            SIDE_PANEL_WIDTH,
            activity_height(activity_details(frame).len()),
        );
        FrameSpec::fixed(
            ACTIVITY_ID,
            "",
            first_place(self.panel_room(), Spot::Activity, size),
        )
    }

    pub(super) fn activity_data(&self, frame: &WatchFrame) -> Framed<ActivityData> {
        let details: Vec<Detail> = activity_details(frame)
            .into_iter()
            .map(|(label, value, color)| Detail {
                label,
                value: colored(value, color),
            })
            .collect();
        let body = ActivityData {
            heading: goal_words(frame),
            idle: details
                .is_empty()
                .then(|| colored(WORDS_NO_JOB, TEXT_FAINT)),
            details,
        };
        self.framed(PANEL_ACTIVITY, &self.activity_spec(frame), body)
    }

    pub(super) fn vitals_spec(&self, frame: &WatchFrame) -> FrameSpec {
        let fights = !frame.combatant.is_empty();
        let size = Vector::new(
            SIDE_PANEL_WIDTH,
            vitals_height(!states(frame).is_empty(), fights),
        );
        FrameSpec::fixed(
            VITALS_ID,
            "",
            first_place(self.panel_room(), Spot::Vitals, size),
        )
    }

    pub(super) fn vitals_data(&self, frame: &WatchFrame) -> Framed<VitalsData> {
        let (hits_fill, hits_numbers) = hits_look(frame);
        let bars = &self.panels.hud.bars;
        let bar = |at: usize, label, (now, max): (u16, u16), color: Rgba, numbers: Rgba| VitalBar {
            label,
            numbers: colored(format!("{now}/{max}"), numbers),
            fill: bars[at].fill,
            ghost: bars[at].ghost,
            color: css_color(color),
            main: at == 0,
        };
        let body = VitalsData {
            name: colored(frame.name.clone(), NOTO_SELF),
            states: states(frame)
                .into_iter()
                .map(|(words, color)| colored(words, color))
                .collect(),
            bars: vec![
                bar(
                    0,
                    WORDS_HITS,
                    (frame.hits, frame.hits_max),
                    hits_fill,
                    hits_numbers,
                ),
                bar(1, WORDS_MANA, (frame.mana, frame.mana_max), MANA, TEXT),
                bar(2, WORDS_STAMINA, (frame.stam, frame.stam_max), STAM, TEXT),
            ],
            fights: (!frame.combatant.is_empty()).then(|| Detail {
                label: WORDS_FIGHTS,
                value: colored(frame.combatant.clone(), ALARM),
            }),
        };
        self.framed(PANEL_VITALS, &self.vitals_spec(frame), body)
    }

    pub(super) fn pack_spec(&self, frame: &WatchFrame) -> FrameSpec {
        let size = Vector::new(PACK_WIDTH, pack_height(pack_lists(frame).len()));
        FrameSpec::fixed(
            PACK_ID,
            "",
            first_place(self.panel_room(), Spot::Pack, size),
        )
    }

    pub(super) fn pack_data(&self, frame: &WatchFrame) -> Framed<PackData> {
        let mut rows = vec![
            Detail {
                label: WORDS_WEIGHT,
                value: colored(weight_words(frame), weight_color(frame)),
            },
            Detail {
                label: WORDS_TIME,
                value: colored(clock_words(frame), TEXT_DIM),
            },
        ];
        rows.extend(pack_lists(frame).into_iter().map(|(label, list)| Detail {
            label,
            value: colored(list, TEXT),
        }));
        let body = PackData {
            heading: WORDS_PACK,
            gold: colored(frame.gold.to_string(), NOTO_SELF),
            gold_words: WORDS_GOLD,
            rows,
        };
        self.framed(PANEL_PACK, &self.pack_spec(frame), body)
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::settled;

    #[test]
    fn the_vitals_follow_the_frame_and_the_pack_tells_the_weight() {
        let mut view = settled();
        let frame = view.frame_ref().unwrap().clone();
        view.follow_vitals(&frame, 0.0);
        assert!(view.follow_vitals(&frame, 0.1), "the bars still fill");
        let data = view.panel_data(0.1);
        let vitals = data.vitals.unwrap().body;
        assert_eq!(vitals.name.words, "Mara");
        assert_eq!(vitals.bars[0].numbers.words, "50/50");
        assert!(vitals.bars[0].fill > 0.0 && vitals.bars[0].main);
        let pack = data.pack.unwrap().body;
        assert_eq!(pack.rows[0].label, "Weight");
        assert_eq!(data.activity.unwrap().body.heading, "Idle");
    }
}
