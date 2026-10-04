//! The damage and durability windows of the Modern style, as both windows
//! show them: where each first stands, how often it reads the session,
//! its words, and the buttons of the damage meter with what each sends.
//! The reports are `model::dps`' and `model::durability`'.

use super::layout::{first_place, Spot};
use super::places::TITLE_ROW;
use super::theme::{GOAL, PANEL_PAD, TEXT_DIM};
use crate::act::Act;
use crate::geom::{Area, Rgba, Vector};
use crate::model::dps::{DamageReport, Dealt};
use crate::model::durability::Wear;
use crate::model::reads::ReadKey;
use serde_json::json;
use uoterm_world::tool_names::TOOL_DAMAGE_METER;

pub const METER_WIDTH: f32 = 300.0;
pub const DPS_ROW: f32 = 22.0;
pub const DPS_BUTTON_ROW: f32 = 28.0;
/// The damage window lists this many mobiles at most.
pub const DPS_MAX_ROWS: usize = 8;
/// The meter is read this often while the window shows, in seconds.
pub const DPS_MAX_AGE: f64 = 1.0;
const METER_START: &str = "start";
const METER_PAUSE: &str = "pause";
const METER_RESUME: &str = "resume";
const METER_STOP: &str = "stop";
pub const WORDS_DAMAGE: &str = "Damage";
const WORDS_START: &str = "Start";
const WORDS_PAUSE: &str = "Pause";
const WORDS_RESUME: &str = "Resume";
const WORDS_STOP: &str = "Stop";
const WORDS_PER_SECOND: &str = "per second";
pub const WORDS_NO_DAMAGE: &str = "No damage counted. Press Start.";

pub const DURABILITY_ROW: f32 = 34.0;
pub const DURABILITY_ART: f32 = 30.0;
/// The durability window lists this many items at most.
pub const DURABILITY_MAX_ROWS: usize = 12;
pub const WORDS_DURABILITY: &str = "Durability";
pub const WORDS_NO_WEAR: &str = "Nothing worn has a durability, or the shard has not said yet.";

/// The read of the damage meter.
pub fn meter_key() -> ReadKey {
    ReadKey::new(TOOL_DAMAGE_METER, &json!({}))
}

/// Where the damage window first stands in `window`, with the rows of
/// `report`.
pub fn dps_first_place(window: Area, report: &DamageReport) -> Area {
    let rows = report.mobiles.len().clamp(1, DPS_MAX_ROWS) + 1;
    let height = TITLE_ROW + DPS_BUTTON_ROW + rows as f32 * DPS_ROW + PANEL_PAD * 2.0;
    first_place(window, Spot::Middle(1), Vector::new(METER_WIDTH, height))
}

/// The buttons of the meter and their acts: Start, Pause or Resume, Stop.
pub fn dps_buttons(report: &DamageReport) -> [(&'static str, Act); 3] {
    let pause = if report.running {
        (WORDS_PAUSE, METER_PAUSE)
    } else {
        (WORDS_RESUME, METER_RESUME)
    };
    [(WORDS_START, METER_START), pause, (WORDS_STOP, METER_STOP)]
        .map(|(words, action)| (words, Act::DamageMeter(action)))
}

/// The damage each second, in words.
pub fn per_second_words(report: &DamageReport) -> String {
    format!("{:.1} {WORDS_PER_SECOND}", report.per_second())
}

/// The color of the damage each second: the goal while the meter runs.
pub fn per_second_color(report: &DamageReport) -> Rgba {
    if report.running {
        GOAL
    } else {
        TEXT_DIM
    }
}

/// The whole damage and the time the meter ran, in words.
pub fn total_words(report: &DamageReport) -> String {
    format!("{}  {:.0}s", report.total(), report.seconds)
}

/// The damage of one mobile and its share each second, in words.
pub fn dealt_words(dealt: &Dealt) -> String {
    format!("{}  {:.1}/s", dealt.damage, dealt.per_second)
}

/// Where the durability window first stands in `window`, with `rows`
/// worn items.
pub fn durability_first_place(window: Area, rows: usize) -> Area {
    let rows = rows.clamp(1, DURABILITY_MAX_ROWS);
    let height = TITLE_ROW + rows as f32 * DURABILITY_ROW + PANEL_PAD * 2.0;
    first_place(window, Spot::Middle(0), Vector::new(METER_WIDTH, height))
}

/// The durability of a worn item now and at most, in words.
pub fn wear_words(wear: &Wear) -> String {
    format!("{} / {}", wear.now, wear.max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_meter_pauses_while_it_runs_and_resumes_while_it_does_not() {
        let mut report = DamageReport {
            running: true,
            seconds: 4.0,
            mobiles: vec![Dealt {
                name: "a rat".into(),
                damage: 10,
                per_second: 2.5,
            }],
        };
        assert_eq!(dps_buttons(&report)[1].1, Act::DamageMeter(METER_PAUSE));
        assert_eq!(per_second_words(&report), "2.5 per second");
        assert_eq!(total_words(&report), "10  4s");
        assert_eq!(dealt_words(&report.mobiles[0]), "10  2.5/s");
        report.running = false;
        assert_eq!(
            dps_buttons(&report)[1],
            (WORDS_RESUME, Act::DamageMeter(METER_RESUME))
        );
    }
}
