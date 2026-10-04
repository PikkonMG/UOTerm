//! The cooldown bars and the spell cast indicator of the Modern style, as
//! both windows show them: the journal lines and the cursor they follow,
//! where each first stands, and their words. The bars run by the rules of
//! the Combat & Spells page (`model::cooldowns`); the indicator shows the
//! spell being cast, how far its cast is, and how many stand in the range
//! of the range circle (`model::casting`).

use super::layout::{first_place, Spot};
use super::places::TITLE_ROW;
use super::theme::PANEL_PAD;
use crate::frame::WatchFrame;
use crate::geom::{Area, Vector};
use crate::model::casting::{self, Cast, CastWatch};
use crate::model::cooldowns::{Cooldowns, Running};
use crate::settings::Profile;

pub const COOLDOWNS_ID: &str = "modern:cooldowns";
pub const CAST_ID: &str = "modern:cast";
pub const COMBAT_WIDTH: f32 = 260.0;
pub const COOLDOWN_ROW: f32 = 26.0;
/// The height of the bar of a cooldown and of the cast.
pub const COMBAT_BAR_HEIGHT: f32 = 8.0;
const CAST_HEIGHT: f32 = 64.0;
pub const WORDS_COOLDOWNS: &str = "Cooldowns";
pub const WORDS_CAST: &str = "Casting";
const WORDS_AIM: &str = "choose a target";
const WORDS_IN_RANGE: &str = "in range";

/// The bars that run and the cast that runs, from the journal and the
/// cursor of each frame.
#[derive(Default)]
pub struct CombatWatch {
    cooldowns: Cooldowns,
    casts: CastWatch,
}

impl CombatWatch {
    /// Reads the journal and the cursor. Call it once in each frame.
    pub fn observe(&mut self, frame: &WatchFrame, profile: &Profile, time: f64) {
        self.cooldowns
            .observe(&profile.combat.cooldowns, &frame.speech, frame.serial, time);
        self.casts.observe(frame, time);
    }

    /// The cooldown bars that run at `time`.
    pub fn bars(&self, time: f64) -> Vec<&Running> {
        self.cooldowns.bars(time)
    }

    /// The cast that runs, when one does.
    pub fn cast(&self) -> Option<&Cast> {
        self.casts.cast()
    }
}

/// Where the panel of `bars` cooldown bars first stands in `window`.
pub fn cooldowns_first_place(window: Area, bars: usize) -> Area {
    let height = TITLE_ROW + bars as f32 * COOLDOWN_ROW + PANEL_PAD * 2.0;
    first_place(
        window,
        Spot::MiddleTop(1),
        Vector::new(COMBAT_WIDTH, height),
    )
}

/// Where the cast indicator first stands in `window`.
pub fn cast_first_place(window: Area) -> Area {
    first_place(
        window,
        Spot::MiddleBottom(1),
        Vector::new(COMBAT_WIDTH, CAST_HEIGHT + TITLE_ROW),
    )
}

/// The seconds a bar has left, in words.
pub fn seconds_words(bar: &Running, time: f64) -> String {
    format!("{:.1}s", bar.seconds_left(time))
}

/// The words right of the spell: that it waits for a target, or how many
/// stand in the range circle of the Combat page.
pub fn cast_aside_words(cast: &Cast, frame: &WatchFrame, profile: &Profile) -> String {
    if cast.aiming {
        return WORDS_AIM.to_string();
    }
    let range = profile.combat.range_circle_tiles;
    let reach = casting::in_range(frame, range).len();
    format!("{reach} {WORDS_IN_RANGE} ({range})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Point;
    use uoterm_assist::spells::SpellFlag;

    #[test]
    fn the_cast_says_its_aim_or_who_stands_in_range() {
        let mut cast = Cast {
            spell: 1,
            name: "Clumsy".into(),
            flag: SpellFlag::Harmful,
            started: 0.0,
            seconds: Some(1.0),
            aiming: true,
        };
        let frame = WatchFrame::default();
        let profile = Profile::default();
        assert_eq!(cast_aside_words(&cast, &frame, &profile), WORDS_AIM);
        cast.aiming = false;
        let range = profile.combat.range_circle_tiles;
        assert_eq!(
            cast_aside_words(&cast, &frame, &profile),
            format!("0 {WORDS_IN_RANGE} ({range})")
        );
        let window = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(1600.0, 900.0));
        let two = cooldowns_first_place(window, 2);
        assert_eq!(
            two.height(),
            TITLE_ROW + 2.0 * COOLDOWN_ROW + PANEL_PAD * 2.0
        );
    }
}
