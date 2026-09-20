//! A human takes the character from the agent, and gives it back.
//!
//! The watch window is the human's hand on the character. While the human
//! has control, the agent may look but may not act: each tool call of the
//! agent that does something is refused, with words that tell it to wait.
//! A human who goes away does not keep the character for ever. Control goes
//! back to the agent when the human has done nothing for a while.

use super::*;
use crate::tools::ARG_HUMAN;
use uoterm_world::{Event, EventKind};

/// The human did nothing for this long, so the agent has the character again.
pub(super) const HUMAN_IDLE: Duration = Duration::from_secs(90);
pub(super) const REFUSED: &str =
    "a human has control of the character; look only, and wait for the control_released event";
const REASON_GIVEN_BACK: &str = "the human gave the character back";
const REASON_IDLE: &str = "the human did nothing for a while";

#[derive(Default)]
pub(super) struct HumanControl {
    /// When the human last did something. None while the agent has control.
    last_act: Option<Instant>,
}

impl HumanControl {
    pub(super) fn active(&self) -> bool {
        self.last_act.is_some()
    }
}

fn by_human(call: &ToolCall) -> bool {
    call.args.get(ARG_HUMAN).and_then(Value::as_bool) == Some(true)
}

/// The refusal for a call the agent may not make now. None lets it through.
pub(super) fn gate(inner: &mut Inner, call: &ToolCall, now: Instant) -> Option<ToolResult> {
    if !inner.human.active() {
        return None;
    }
    if by_human(call) {
        inner.human.last_act = Some(now);
        return None;
    }
    (!crate::tools::is_read_only(&call.name)).then(|| ToolResult::err(REFUSED))
}

/// Stops what the agent set going, and keeps the agent out.
pub(super) fn take(inner: &mut Inner, now: Instant) -> ToolResult {
    if !inner.human.active() {
        let stop = ToolCall {
            name: TOOL_STOP.into(),
            args: json!({}),
        };
        handle_tool(inner, stop);
        // Each of these answers with an error when nothing runs. That is fine.
        scripting::stop_script(inner);
        agents::agent_stop(inner);
        let event = Event::new(EventKind::ControlTaken, None, REFUSED);
        inner.world.write().push_event(event);
    }
    inner.human.last_act = Some(now);
    inner.movement.steady_pace = true;
    ToolResult::ok(json!({ "human_control": true }))
}

pub(super) fn release(inner: &mut Inner) -> ToolResult {
    give_back(inner, REASON_GIVEN_BACK);
    ToolResult::ok(json!({ "human_control": false }))
}

fn give_back(inner: &mut Inner, reason: &str) {
    inner.movement.steady_pace = false;
    if inner.human.last_act.take().is_some() {
        let event = Event::new(EventKind::ControlReleased, None, reason);
        inner.world.write().push_event(event);
    }
}

/// Gives the character back when the human has gone away.
pub(super) fn pump(inner: &mut Inner, now: Instant) {
    let idle = inner
        .human
        .last_act
        .is_some_and(|last| now.duration_since(last) >= HUMAN_IDLE);
    if idle {
        give_back(inner, REASON_IDLE);
    }
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::armed_session;
    use super::*;

    const FOLLOWED: Serial = Serial(0x0000_0F01);

    fn call(name: &str, args: Value) -> ToolCall {
        ToolCall {
            name: name.into(),
            args,
        }
    }

    fn event_kinds(inner: &Inner) -> Vec<EventKind> {
        inner.world.read().events.iter().map(|e| e.kind).collect()
    }

    #[test]
    fn a_human_walks_at_the_exact_pace_and_the_agent_gets_his_own_pace_back() {
        let mut inner = armed_session();
        let walk = Duration::from_millis(crate::config::STEP_WALK_MS);
        assert!(take(&mut inner, Instant::now()).ok);
        for _ in 0..20 {
            assert_eq!(inner.movement.next_interval(false), walk);
        }
        assert!(release(&mut inner).ok);
        assert!(!inner.movement.steady_pace);
    }

    #[test]
    fn the_agent_may_look_but_not_act_while_a_human_has_control() {
        let mut inner = armed_session();
        inner.follow = Some(FOLLOWED);
        assert!(
            answer_agent(
                &mut inner,
                call(TOOL_TAKE_CONTROL, json!({ "human": true }))
            )
            .ok
        );
        assert!(inner.follow.is_none(), "what the agent set going stops");
        assert!(event_kinds(&inner).contains(&EventKind::ControlTaken));

        let refused = answer_agent(&mut inner, call(TOOL_SAY, json!({ "text": "hi" })));
        assert_eq!(refused.error.as_deref(), Some(REFUSED));
        let looked = answer_agent(&mut inner, call(TOOL_OBSERVE, json!({})));
        assert_eq!(looked.result["human_control"], true);
        let by_hand = answer_agent(
            &mut inner,
            call(TOOL_WAR_MODE, json!({ "on": true, "human": true })),
        );
        assert!(by_hand.ok);

        assert!(
            answer_agent(
                &mut inner,
                call(TOOL_RELEASE_CONTROL, json!({ "human": true }))
            )
            .ok
        );
        assert!(event_kinds(&inner).contains(&EventKind::ControlReleased));
        assert!(answer_agent(&mut inner, call(TOOL_WAR_MODE, json!({ "on": false }))).ok);
    }

    #[test]
    fn one_script_command_runs_for_a_human_and_not_for_an_agent() {
        let mut inner = armed_session();
        let line = json!({ "text": "sysmsg hello" });
        let refused = answer_agent(&mut inner, call(TOOL_COMMAND, line));
        assert!(!refused.ok, "an agent uses run_script");
        let by_hand = json!({ "text": "sysmsg hello", "human": true });
        let done = answer_agent(&mut inner, call(TOOL_COMMAND, by_hand));
        assert!(done.ok, "{:?}", done.error);
        assert_eq!(done.result["output"][0], "hello");
    }

    #[test]
    fn a_human_who_goes_away_gives_the_character_back() {
        let mut inner = armed_session();
        let start = Instant::now();
        take(&mut inner, start);
        pump(&mut inner, start + HUMAN_IDLE - Duration::from_secs(1));
        assert!(inner.human.active());
        pump(&mut inner, start + HUMAN_IDLE);
        assert!(!inner.human.active());
        assert!(event_kinds(&inner).contains(&EventKind::ControlReleased));
    }
}
