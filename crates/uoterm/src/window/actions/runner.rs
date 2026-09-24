//! Runs macros as the official client does: the steps go in order, each at
//! once, until a delay or a wait for a target cursor holds the rest. A new
//! macro takes the place of the one that runs.

use super::resolve::{resolve, Context, Effect, Wait};
use super::step_action;
use crate::window::settings::MacroStep;
use std::collections::VecDeque;

struct Running {
    steps: Vec<MacroStep>,
    next: usize,
    /// The effects of the step that runs, which a wait held.
    pending: VecDeque<Effect>,
    /// What holds the macro, and the time it began.
    waiting: Option<(Wait, f64)>,
}

#[derive(Default)]
pub struct MacroRunner {
    running: Option<Running>,
}

/// True when a wait that began at `since` is over at `time`.
fn wait_over(wait: Wait, since: f64, time: f64, target_cursor: bool) -> bool {
    let over = |most: std::time::Duration| time - since >= most.as_secs_f64();
    match wait {
        Wait::For(pause) => over(pause),
        Wait::Target(most) => target_cursor || over(most),
    }
}

impl MacroRunner {
    /// Starts a macro. The one that ran stops.
    pub fn start(&mut self, steps: Vec<MacroStep>) {
        self.running = Some(Running {
            steps,
            next: 0,
            pending: VecDeque::new(),
            waiting: None,
        });
    }

    /// The effects that are due at `time`. Call it every frame.
    pub fn tick(&mut self, time: f64, context: &Context<'_>) -> Vec<Effect> {
        let mut due = Vec::new();
        while let Some(run) = self.running.as_mut() {
            if let Some((wait, since)) = run.waiting {
                if !wait_over(wait, since, time, context.frame.target_cursor) {
                    break;
                }
                run.waiting = None;
            }
            if let Some(effect) = run.pending.pop_front() {
                match effect {
                    Effect::Wait(wait) => run.waiting = Some((wait, time)),
                    other => due.push(other),
                }
                continue;
            }
            let Some(step) = run.steps.get(run.next) else {
                self.running = None;
                break;
            };
            run.next += 1;
            match step_action(step) {
                Some(action) => run.pending.extend(resolve(action, &step.argument, context)),
                None => due.push(Effect::Note(format!(
                    "\"{}\" is not an action this window knows.",
                    step.action
                ))),
            }
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchFrame;
    use crate::window::control::Act;
    use crate::window::settings::Profile;

    fn said(words: &str) -> Effect {
        Effect::Act(Act::Say {
            text: words.into(),
            hue: Profile::default().speech.speech_hue,
        })
    }

    #[test]
    fn a_delay_holds_the_next_step_until_its_time() {
        let frame = WatchFrame::default();
        let profile = Profile::default();
        let context = Context {
            frame: &frame,
            profile: &profile,
            selected: None,
        };
        let mut runner = MacroRunner::default();
        runner.start(vec![
            MacroStep::new("say", "one"),
            MacroStep::new("delay", "500"),
            MacroStep::new("say", "two"),
            MacroStep::new("say", "three"),
        ]);
        assert_eq!(runner.tick(10.0, &context), vec![said("one")]);
        assert!(runner.tick(10.4, &context).is_empty());
        assert_eq!(
            runner.tick(10.5, &context),
            vec![said("two"), said("three")]
        );
        assert!(runner.tick(11.0, &context).is_empty());
    }

    #[test]
    fn a_wait_for_target_ends_when_the_cursor_opens_or_the_time_runs_out() {
        let mut frame = WatchFrame::default();
        let profile = Profile::default();
        let mut runner = MacroRunner::default();
        let steps = vec![
            MacroStep::new("cast", "Heal"),
            MacroStep::new("wait_for_target", "2000"),
            MacroStep::new("target_self", ""),
        ];
        runner.start(steps.clone());
        let context = Context {
            frame: &frame,
            profile: &profile,
            selected: None,
        };
        assert_eq!(runner.tick(0.0, &context).len(), 1);
        assert!(runner.tick(1.0, &context).is_empty());
        frame.target_cursor = true;
        let context = Context {
            frame: &frame,
            profile: &profile,
            selected: None,
        };
        assert_eq!(runner.tick(1.1, &context).len(), 1);
        frame.target_cursor = false;
        runner.start(steps);
        let context = Context {
            frame: &frame,
            profile: &profile,
            selected: None,
        };
        assert_eq!(runner.tick(5.0, &context).len(), 1);
        assert_eq!(runner.tick(7.0, &context).len(), 1);
    }

    #[test]
    fn a_new_macro_takes_the_place_of_the_one_that_waits() {
        let frame = WatchFrame::default();
        let profile = Profile::default();
        let context = Context {
            frame: &frame,
            profile: &profile,
            selected: None,
        };
        let mut runner = MacroRunner::default();
        runner.start(vec![
            MacroStep::new("delay", "1000"),
            MacroStep::new("say", "late"),
        ]);
        assert!(runner.tick(0.0, &context).is_empty());
        runner.start(vec![MacroStep::new("say", "now")]);
        assert_eq!(runner.tick(0.1, &context), vec![said("now")]);
        assert!(runner.tick(2.0, &context).is_empty());
    }

    #[test]
    fn a_step_of_an_unknown_action_is_a_note() {
        let frame = WatchFrame::default();
        let profile = Profile::default();
        let context = Context {
            frame: &frame,
            profile: &profile,
            selected: None,
        };
        let mut runner = MacroRunner::default();
        runner.start(vec![MacroStep::new("fly_to_the_moon", "")]);
        assert!(matches!(
            runner.tick(0.0, &context).as_slice(),
            [Effect::Note(_)]
        ));
    }
}
