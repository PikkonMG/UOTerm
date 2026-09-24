//! The cooldown bars: a journal line with the trigger words of a rule
//! starts its bar, and the bar runs down for the time of the rule.

use super::journal::NewLines;
use crate::view::WatchSpeech;
use crate::window::settings::{CooldownRule, CooldownSource};
use uoterm_protocol::types::SPEECH_SYSTEM;

/// A bar that runs now.
#[derive(Clone, Debug, PartialEq)]
pub struct Running {
    pub label: String,
    pub hue: u16,
    pub started: f64,
    pub seconds: f32,
}

impl Running {
    pub fn seconds_left(&self, time: f64) -> f32 {
        (f64::from(self.seconds) - (time - self.started)).max(0.0) as f32
    }

    /// The share of the time left, from 1 at the start to 0 at the end.
    pub fn share_left(&self, time: f64) -> f32 {
        if self.seconds <= 0.0 {
            0.0
        } else {
            self.seconds_left(time) / self.seconds
        }
    }
}

/// True when the line comes from the source a rule asks for.
fn from_source(source: CooldownSource, line: &WatchSpeech, me: u32) -> bool {
    let from_shard = line.serial == 0 || line.kind == SPEECH_SYSTEM;
    match source {
        CooldownSource::Anyone => true,
        CooldownSource::Myself => line.serial == me && !from_shard,
        CooldownSource::Others => line.serial != me && !from_shard,
        CooldownSource::System => from_shard,
    }
}

fn triggers(rule: &CooldownRule, line: &WatchSpeech, me: u32) -> bool {
    let trigger = rule.trigger.trim().to_lowercase();
    !trigger.is_empty()
        && from_source(rule.source, line, me)
        && line.text.to_lowercase().contains(&trigger)
}

#[derive(Default)]
pub struct Cooldowns {
    running: Vec<Running>,
    new_lines: NewLines,
}

impl Cooldowns {
    /// Reads the journal lines that are new since the last call, and starts
    /// the bars they trigger. The first call only marks where the journal is,
    /// so old lines start nothing.
    pub fn observe(&mut self, rules: &[CooldownRule], speech: &[WatchSpeech], me: u32, time: f64) {
        for line in self.new_lines.take(speech) {
            for rule in rules.iter().filter(|rule| triggers(rule, line, me)) {
                let running = self.running.iter_mut().find(|bar| bar.label == rule.label);
                match running {
                    Some(bar) if rule.restart => bar.started = time,
                    Some(_) => {}
                    None => self.running.push(Running {
                        label: rule.label.clone(),
                        hue: rule.hue,
                        started: time,
                        seconds: rule.seconds,
                    }),
                }
            }
        }
        self.running.retain(|bar| bar.seconds_left(time) > 0.0);
    }

    /// The bars that run now, the one that ends first on top.
    pub fn bars(&self, time: f64) -> Vec<&Running> {
        let mut bars: Vec<&Running> = self.running.iter().collect();
        bars.sort_by(|a, b| a.seconds_left(time).total_cmp(&b.seconds_left(time)));
        bars
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ME: u32 = 0x55;

    fn rule(trigger: &str, source: CooldownSource, restart: bool) -> CooldownRule {
        CooldownRule {
            label: trigger.to_string(),
            hue: 0x35,
            trigger: trigger.to_string(),
            seconds: 10.0,
            source,
            restart,
        }
    }

    fn line(seq: u64, serial: u32, text: &str) -> WatchSpeech {
        WatchSpeech {
            seq,
            serial,
            text: text.to_string(),
            ..WatchSpeech::default()
        }
    }

    #[test]
    fn a_new_line_starts_its_bar_and_the_bar_runs_down() {
        let rules = [rule("hidden yourself", CooldownSource::System, false)];
        let mut cooldowns = Cooldowns::default();
        cooldowns.observe(
            &rules,
            &[line(1, 0, "You have hidden yourself well.")],
            ME,
            0.0,
        );
        assert!(cooldowns.bars(0.0).is_empty(), "an old line starts nothing");
        let lines = [
            line(1, 0, "x"),
            line(2, 0, "You have HIDDEN yourself well."),
        ];
        cooldowns.observe(&rules, &lines, ME, 1.0);
        let bars = cooldowns.bars(6.0);
        assert_eq!(bars.len(), 1);
        assert!((bars[0].share_left(6.0) - 0.5).abs() < 0.001);
        cooldowns.observe(&rules, &lines, ME, 12.0);
        assert!(cooldowns.bars(12.0).is_empty(), "a bar that ran down goes");
    }

    #[test]
    fn a_rule_listens_to_its_source_and_restarts_only_when_asked() {
        let mut cooldowns = Cooldowns::default();
        cooldowns.observe(&[], &[], ME, 0.0);
        let rules = [
            rule("kal ort por", CooldownSource::Myself, false),
            rule("in mani", CooldownSource::Others, true),
        ];
        let lines = [line(1, 9, "Kal Ort Por"), line(2, ME, "In Mani")];
        cooldowns.observe(&rules, &lines, ME, 1.0);
        assert!(
            cooldowns.bars(1.0).is_empty(),
            "each came from the other side"
        );
        cooldowns.observe(&rules, &[line(3, ME, "Kal Ort Por")], ME, 2.0);
        cooldowns.observe(&rules, &[line(4, ME, "Kal Ort Por")], ME, 5.0);
        assert_eq!(cooldowns.bars(5.0)[0].started, 2.0, "it runs on");
        let restart = [rule("kal ort por", CooldownSource::Anyone, true)];
        cooldowns.observe(&restart, &[line(5, ME, "Kal Ort Por")], ME, 6.0);
        assert_eq!(cooldowns.bars(6.0)[0].started, 6.0, "it starts again");
    }
}
