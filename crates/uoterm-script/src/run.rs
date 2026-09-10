//! Runs a script a few steps at a time.
//!
//! The session calls [`Script::tick`] on each of its ticks. A tick runs lines
//! until one of them acts in the game, waits, or the step cap is reached. So a
//! script never goes faster than one action a tick, never blocks the
//! session, and stops the moment it is told to: there is no thread to kill.

use std::time::{Duration, Instant};

use crate::program::{Arg, Call, Condition, ForSpec, Join, Op, Operand, Program, Test};
use crate::token::Compare;
use crate::vars::Vars;

/// The most steps one tick runs. A loop with no action and no wait in it
/// goes on at the next tick, so it cannot hold the session up.
pub const MAX_STEPS_PER_TICK: usize = 200;

/// What a command did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Done, nothing sent to the shard. The next line runs in the same tick.
    Done,
    /// Something went to the shard. The next line waits for the next tick.
    Acted,
    /// Not done yet: run this line again at the next tick. A command that
    /// waits keeps its own time limit; see [`Ctx::waited`].
    Wait,
    /// The line cannot be done. The script stops with this message.
    Fail(String),
}

/// A value a condition word gives.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
    Bool(bool),
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Self::Number(n) => *n != 0.0,
            Self::Text(t) => !t.is_empty(),
            Self::Bool(b) => *b,
        }
    }

    fn number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Bool(b) => Some(f64::from(u8::from(*b))),
            Self::Text(t) => crate::program::parse_number(t).map(|n| n as f64),
        }
    }

    fn text(&self) -> String {
        match self {
            Self::Number(n) => n.to_string(),
            Self::Text(t) => t.clone(),
            Self::Bool(b) => b.to_string(),
        }
    }

    /// Compares two values: as numbers when both read as numbers, otherwise
    /// as text, with case ignored.
    fn compare(&self, compare: Compare, other: &Value) -> bool {
        match (self.number(), other.number()) {
            (Some(a), Some(b)) => compare.holds(a, b),
            _ => {
                let (a, b) = (self.text().to_lowercase(), other.text().to_lowercase());
                compare.holds(a.as_str(), b.as_str())
            }
        }
    }
}

/// What a command sees while it runs.
pub struct Ctx<'a> {
    pub vars: &'a mut Vars,
    pub now: Instant,
    /// When the current line began, the first time it ran.
    line_started: Instant,
    /// Lines the script shows its user, such as a `sysmsg`.
    output: &'a mut Vec<String>,
    /// A condition word that acted in the game sets this, so the tick ends.
    acted: bool,
}

impl Ctx<'_> {
    /// How long the current line has been running.
    pub fn waited(&self) -> Duration {
        self.now.saturating_duration_since(self.line_started)
    }

    /// Shows a line to the script's user.
    pub fn say(&mut self, text: impl Into<String>) {
        self.output.push(text.into());
    }

    /// A condition word calls this when it sent something to the shard.
    pub fn mark_acted(&mut self) {
        self.acted = true;
    }
}

/// The game side of the interpreter.
pub trait Host {
    /// Runs one command line.
    fn command(&mut self, call: &Call, ctx: &mut Ctx) -> Step;
    /// Reads a condition word, such as `poisoned` or `hits`.
    fn value(&mut self, call: &Call, ctx: &mut Ctx) -> Result<Value, String>;
}

/// How a script stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Running,
    /// It reached its last line.
    Done,
    /// A `stop` line, or the user stopped it.
    Stopped,
    Failed {
        line: usize,
        message: String,
    },
}

/// One pass of a `for` loop.
#[derive(Clone, Debug)]
struct LoopState {
    /// The pass number or the list place.
    at: i64,
    /// The last pass number or list place.
    last: i64,
    /// The list a list loop walks.
    list: Option<String>,
}

/// A script and where it has got to.
pub struct Script {
    program: Program,
    pc: usize,
    loops: Vec<Option<LoopState>>,
    /// When the current line first ran; `None` before it runs.
    line_started: Option<Instant>,
    status: Status,
    output: Vec<String>,
}

const LIST_CURRENT: &str = "[]";

impl Script {
    pub fn new(program: Program) -> Self {
        let loops = vec![None; program.loops];
        Self {
            program,
            pc: 0,
            loops,
            line_started: None,
            status: Status::Running,
            output: Vec::new(),
        }
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    /// The line the script is on, counting from one, or `None` when it is
    /// between lines that are not commands.
    pub fn line(&self) -> Option<usize> {
        self.program.ops.get(self.pc).and_then(op_line)
    }

    /// Takes the lines the script showed since the last call.
    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }

    /// Stops the script where it is.
    pub fn stop(&mut self) {
        if self.status == Status::Running {
            self.status = Status::Stopped;
        }
    }

    /// Runs lines until one acts, one waits, the step cap is reached or the
    /// script ends.
    pub fn tick(&mut self, host: &mut dyn Host, vars: &mut Vars, now: Instant) {
        for _ in 0..MAX_STEPS_PER_TICK {
            if self.status != Status::Running {
                return;
            }
            let Some(op) = self.program.ops.get(self.pc).cloned() else {
                self.status = Status::Done;
                return;
            };
            let started = *self.line_started.get_or_insert(now);
            let mut ctx = Ctx {
                vars,
                now,
                line_started: started,
                output: &mut self.output,
                acted: false,
            };
            let flow = match op {
                Op::Command(call) => {
                    let call = expand_list_items(&call, &self.loops, ctx.vars);
                    match run_builtin(&call, &mut ctx)
                        .unwrap_or_else(|| host.command(&call, &mut ctx))
                    {
                        Step::Done => Flow::Next,
                        Step::Acted => Flow::NextTick,
                        Step::Wait => return,
                        Step::Fail(message) => Flow::Fail(call.line, message),
                    }
                }
                Op::If { cond, jump, line } => match eval(&cond, host, &mut ctx, &self.loops) {
                    Ok(true) => after(Flow::Next, ctx.acted),
                    Ok(false) => after(Flow::Goto(jump), ctx.acted),
                    Err(message) => Flow::Fail(line, message),
                },
                Op::While { cond, exit, line } => match eval(&cond, host, &mut ctx, &self.loops) {
                    Ok(true) => after(Flow::Next, ctx.acted),
                    Ok(false) => after(Flow::Goto(exit), ctx.acted),
                    Err(message) => Flow::Fail(line, message),
                },
                Op::Jump(target) => Flow::Goto(target),
                Op::For {
                    spec,
                    slot,
                    exit,
                    line,
                } => match enter_loop(&mut self.loops, &spec, slot, ctx.vars) {
                    Ok(true) => Flow::Next,
                    Ok(false) => Flow::Goto(exit),
                    Err(message) => Flow::Fail(line, message),
                },
                Op::ForNext { slot, start } => match self.loops[slot].as_mut() {
                    Some(state) => match state.at.checked_add(1) {
                        Some(at) => {
                            state.at = at;
                            Flow::Goto(start)
                        }
                        // No pass after the largest number: the next op is
                        // the loop's `ForDone`, which ends it.
                        None => Flow::Next,
                    },
                    None => Flow::Goto(start),
                },
                Op::ForDone { slot } => {
                    self.loops[slot] = None;
                    Flow::Next
                }
                Op::Stop => {
                    self.status = Status::Stopped;
                    return;
                }
                Op::Replay => {
                    self.loops.iter_mut().for_each(|l| *l = None);
                    Flow::Goto(0)
                }
            };
            self.line_started = None;
            match flow {
                Flow::Next => self.pc += 1,
                Flow::NextTick => {
                    self.pc += 1;
                    if self.pc >= self.program.ops.len() {
                        self.status = Status::Done;
                    }
                    return;
                }
                Flow::Goto(target) => self.pc = target,
                Flow::GotoNextTick(target) => {
                    self.pc = target;
                    return;
                }
                Flow::Fail(line, message) => {
                    self.status = Status::Failed { line, message };
                    return;
                }
            }
        }
    }
}

enum Flow {
    Next,
    NextTick,
    Goto(usize),
    GotoNextTick(usize),
    Fail(usize, String),
}

/// Starts loop `slot` if it is not running, and says whether the body runs
/// this pass.
fn enter_loop(
    loops: &mut [Option<LoopState>],
    spec: &ForSpec,
    slot: usize,
    vars: &Vars,
) -> Result<bool, String> {
    if loops[slot].is_none() {
        loops[slot] = Some(start_loop(spec, vars)?);
    }
    let state = loops[slot].as_ref().expect("the loop was just started");
    let runs = state.at <= state.last;
    if !runs {
        loops[slot] = None;
    }
    Ok(runs)
}

/// A condition that acted in the game ends the tick after it moves on.
fn after(flow: Flow, acted: bool) -> Flow {
    match (flow, acted) {
        (Flow::Next, true) => Flow::NextTick,
        (Flow::Goto(t), true) => Flow::GotoNextTick(t),
        (flow, _) => flow,
    }
}

fn op_line(op: &Op) -> Option<usize> {
    match op {
        Op::Command(call) => Some(call.line),
        Op::If { line, .. } | Op::While { line, .. } | Op::For { line, .. } => Some(*line),
        _ => None,
    }
}

fn arg_number(arg: &Arg, what: &str) -> Result<i64, String> {
    arg.number()
        .ok_or_else(|| format!("{what} must be a number, not '{}'", arg.text))
}

fn start_loop(spec: &ForSpec, vars: &Vars) -> Result<LoopState, String> {
    let list_last = |list: &Arg| -> Result<i64, String> {
        let items = vars
            .list(&list.text)
            .ok_or_else(|| format!("there is no list '{}'", list.text))?;
        Ok(items.len() as i64 - 1)
    };
    Ok(match spec {
        ForSpec::Count(count) => LoopState {
            at: 1,
            last: arg_number(count, "a loop count")?,
            list: None,
        },
        ForSpec::Range(start, end) => LoopState {
            at: arg_number(start, "a loop start")?,
            last: arg_number(end, "a loop end")?,
            list: None,
        },
        ForSpec::List { start, list } => LoopState {
            at: arg_number(start, "a loop start")?,
            last: list_last(list)?,
            list: Some(list.text.to_ascii_lowercase()),
        },
        ForSpec::ListRange { start, end, list } => LoopState {
            at: arg_number(start, "a loop start")?,
            last: arg_number(end, "a loop end")?.min(list_last(list)?),
            list: Some(list.text.to_ascii_lowercase()),
        },
    })
}

/// Puts list items in for `name[]` (the item a list loop is on) and
/// `name[3]` (the item at a place).
fn expand_list_items(call: &Call, loops: &[Option<LoopState>], vars: &Vars) -> Call {
    let mut call = call.clone();
    for arg in &mut call.args {
        if let Some(item) = list_item(&arg.text, loops, vars) {
            *arg = Arg::quoted(item);
        }
    }
    call
}

fn list_item(text: &str, loops: &[Option<LoopState>], vars: &Vars) -> Option<String> {
    let open = text.find('[')?;
    let inner = text[open..].strip_prefix('[')?.strip_suffix(']')?;
    let name = text[..open].to_ascii_lowercase();
    let items = vars.list(&name)?;
    let place = if format!("[{inner}]") == LIST_CURRENT {
        loops
            .iter()
            .rev()
            .flatten()
            .find(|l| l.list.as_deref() == Some(name.as_str()))?
            .at
    } else {
        inner.parse::<i64>().ok()?
    };
    usize::try_from(place)
        .ok()
        .and_then(|i| items.get(i))
        .cloned()
}

fn eval(
    cond: &Condition,
    host: &mut dyn Host,
    ctx: &mut Ctx,
    loops: &[Option<LoopState>],
) -> Result<bool, String> {
    let mut result = test(&cond.first, host, ctx, loops)?;
    for (join, next) in &cond.rest {
        result = match join {
            Join::And => result && test(next, host, ctx, loops)?,
            Join::Or => result || test(next, host, ctx, loops)?,
        };
    }
    Ok(result)
}

fn test(
    t: &Test,
    host: &mut dyn Host,
    ctx: &mut Ctx,
    loops: &[Option<LoopState>],
) -> Result<bool, String> {
    let left = read(&t.left, host, ctx, loops)?;
    let holds = match &t.compare {
        None => left.truthy(),
        Some((compare, operand)) => {
            let right = match operand {
                Operand::Value(arg) => match arg.number() {
                    Some(n) if !arg.quoted => Value::Number(n as f64),
                    _ => Value::Text(arg.text.clone()),
                },
                Operand::Call(call) => read(call, host, ctx, loops)?,
            };
            left.compare(*compare, &right)
        }
    };
    Ok(holds != t.not)
}

fn read(
    call: &Call,
    host: &mut dyn Host,
    ctx: &mut Ctx,
    loops: &[Option<LoopState>],
) -> Result<Value, String> {
    let call = expand_list_items(call, loops, ctx.vars);
    match builtin_value(&call, ctx) {
        Some(value) => value,
        None => host.value(&call, ctx),
    }
}

const PAUSE: &str = "pause";
const LIST_FRONT: &str = "front";
const LIST_BACK: &str = "back";

/// The commands the interpreter runs itself: waits, aliases, lists and
/// timers. `None` for every other command.
fn run_builtin(call: &Call, ctx: &mut Ctx) -> Option<Step> {
    let arg = |i: usize| call.args.get(i);
    let need = |i: usize, what: &str| -> Result<&Arg, Step> {
        arg(i).ok_or_else(|| Step::Fail(format!("{} needs {what}", call.name)))
    };
    let result = match call.name.as_str() {
        PAUSE => need(0, "a time in milliseconds").and_then(|ms| {
            let ms = ms
                .number()
                .ok_or_else(|| Step::Fail("pause needs a time in milliseconds".into()))?;
            Ok(if ctx.waited() >= Duration::from_millis(ms.max(0) as u64) {
                Step::Done
            } else {
                Step::Wait
            })
        }),
        // A setalias with no serial asks the player to pick one: the host's.
        "setalias" if call.args.len() >= 2 => {
            let name = need(0, "a name").ok()?;
            let serial = call.args[1]
                .number()
                .and_then(|n| u32::try_from(n).ok())
                .or_else(|| ctx.vars.alias(&call.args[1].text));
            match serial {
                Some(serial) => {
                    ctx.vars.set_alias(&name.text, serial);
                    Ok(Step::Done)
                }
                // Other aliases, such as `found`, are the host's to read.
                None => return None,
            }
        }
        "unsetalias" => need(0, "a name").map(|name| {
            ctx.vars.unset_alias(&name.text);
            Step::Done
        }),
        "createlist" => need(0, "a list name").map(|name| {
            ctx.vars.create_list(&name.text);
            Step::Done
        }),
        "clearlist" => need(0, "a list name").map(|name| {
            ctx.vars.list_mut(&name.text).clear();
            Step::Done
        }),
        "removelist" => need(0, "a list name").map(|name| {
            ctx.vars.remove_list(&name.text);
            Step::Done
        }),
        "pushlist" => need(0, "a list name").and_then(|name| {
            let value = list_value(need(1, "a value")?, ctx.vars);
            let front = arg(2).is_some_and(|a| a.is(LIST_FRONT));
            let list = ctx.vars.list_mut(&name.text);
            if call.force && list.iter().any(|v| v.eq_ignore_ascii_case(&value)) {
                return Ok(Step::Done);
            }
            if front {
                list.insert(0, value);
            } else {
                list.push(value);
            }
            Ok(Step::Done)
        }),
        "poplist" => need(0, "a list name").and_then(|name| {
            let which = need(1, "a value, 'front' or 'back'")?;
            let value = list_value(which, ctx.vars);
            let list = ctx.vars.list_mut(&name.text);
            if which.is(LIST_FRONT) {
                if !list.is_empty() {
                    list.remove(0);
                }
            } else if which.is(LIST_BACK) {
                list.pop();
            } else if call.force {
                list.retain(|v| !v.eq_ignore_ascii_case(&value));
            } else if let Some(i) = list.iter().position(|v| v.eq_ignore_ascii_case(&value)) {
                list.remove(i);
            }
            Ok(Step::Done)
        }),
        "createtimer" => need(0, "a timer name").map(|name| {
            ctx.vars.set_timer(&name.text, Duration::ZERO, ctx.now);
            Step::Done
        }),
        "settimer" => need(0, "a timer name").and_then(|name| {
            let ms = need(1, "a value in milliseconds")?
                .number()
                .ok_or_else(|| Step::Fail("settimer needs a value in milliseconds".into()))?;
            ctx.vars
                .set_timer(&name.text, Duration::from_millis(ms.max(0) as u64), ctx.now);
            Ok(Step::Done)
        }),
        "removetimer" => need(0, "a timer name").map(|name| {
            ctx.vars.remove_timer(&name.text);
            Step::Done
        }),
        _ => return None,
    };
    Some(result.unwrap_or_else(|fail| fail))
}

/// A value as a list keeps it. An alias goes in as the serial it holds now,
/// so a later change to the alias does not change the list. `pushlist`,
/// `poplist` and `inlist` all read it this way, so they agree.
fn list_value(arg: &Arg, vars: &Vars) -> String {
    match vars.alias(&arg.text) {
        Some(serial) if arg.number().is_none() => serial.to_string(),
        _ => arg.text.clone(),
    }
}

/// The condition words the interpreter reads itself. `None` for every other
/// word.
fn builtin_value(call: &Call, ctx: &Ctx) -> Option<Result<Value, String>> {
    let name = |i: usize| {
        call.args
            .get(i)
            .map(|a| a.text.clone())
            .ok_or_else(|| format!("{} needs a name", call.name))
    };
    let value = match call.name.as_str() {
        // A name the script did not set may be one the game knows, such as
        // 'bank' or 'lefthand': the host answers for those.
        "findalias" => match name(0) {
            Ok(n) if ctx.vars.alias(&n).is_none() => return None,
            other => other.map(|_| Value::Bool(true)),
        },
        "listexists" => name(0).map(|n| Value::Bool(ctx.vars.list(&n).is_some())),
        "list" => name(0).map(|n| Value::Number(ctx.vars.list(&n).map_or(0, Vec::len) as f64)),
        "inlist" => name(0).and_then(|n| {
            let value = call
                .args
                .get(1)
                .ok_or_else(|| "inlist needs a value".to_string())?;
            let value = list_value(value, ctx.vars);
            let found = ctx.vars.list(&n).is_some_and(|items| {
                items.iter().any(|v| {
                    if call.force {
                        *v == value
                    } else {
                        v.eq_ignore_ascii_case(&value)
                    }
                })
            });
            Ok(Value::Bool(found))
        }),
        "timer" => name(0).and_then(|n| {
            ctx.vars
                .timer(&n, ctx.now)
                .map(|d| Value::Number(d.as_millis() as f64))
                .ok_or_else(|| format!("there is no timer '{n}'"))
        }),
        "timerexists" => name(0).map(|n| Value::Bool(ctx.vars.timer(&n, ctx.now).is_some())),
        _ => return None,
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A game side that records each command and answers from tables.
    #[derive(Default)]
    struct Fake {
        ran: Vec<String>,
        steps: HashMap<&'static str, Step>,
        values: HashMap<&'static str, Value>,
    }

    impl Host for Fake {
        fn command(&mut self, call: &Call, _ctx: &mut Ctx) -> Step {
            let args: Vec<&str> = call.args.iter().map(|a| a.text.as_str()).collect();
            self.ran.push(
                format!("{} {}", call.name, args.join(" "))
                    .trim()
                    .to_string(),
            );
            self.steps
                .get(call.name.as_str())
                .cloned()
                .unwrap_or(Step::Done)
        }

        fn value(&mut self, call: &Call, _ctx: &mut Ctx) -> Result<Value, String> {
            self.values
                .get(call.name.as_str())
                .cloned()
                .ok_or_else(|| format!("unknown word {}", call.name))
        }
    }

    const TICK: Duration = Duration::from_millis(100);

    fn script(source: &str) -> Script {
        Script::new(Program::parse(source).expect("a script"))
    }

    /// Ticks until the script ends or the tick budget runs out.
    fn run(s: &mut Script, host: &mut Fake, vars: &mut Vars, ticks: usize) -> Instant {
        let mut now = Instant::now();
        for _ in 0..ticks {
            s.tick(host, vars, now);
            if *s.status() != Status::Running {
                break;
            }
            now += TICK;
        }
        now
    }

    #[test]
    fn one_action_a_tick() {
        let mut host = Fake::default();
        host.steps.insert("say", Step::Acted);
        let mut vars = Vars::default();
        let mut s = script("say 'one'\nsay 'two'");
        s.tick(&mut host, &mut vars, Instant::now());
        assert_eq!(host.ran, vec!["say one"]);
        s.tick(&mut host, &mut vars, Instant::now());
        assert_eq!(host.ran, vec!["say one", "say two"]);
        s.tick(&mut host, &mut vars, Instant::now());
        assert_eq!(*s.status(), Status::Done);
    }

    #[test]
    fn a_script_whose_last_line_acts_is_done_at_once() {
        let mut host = Fake::default();
        host.steps.insert("say", Step::Acted);
        let mut vars = Vars::default();
        let mut s = script("say 'bye'");
        s.tick(&mut host, &mut vars, Instant::now());
        assert_eq!(*s.status(), Status::Done);
    }

    #[test]
    fn a_pause_waits_its_time_then_goes_on() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        let mut s = script("pause 250\nsay 'after'");
        let start = Instant::now();
        s.tick(&mut host, &mut vars, start);
        s.tick(&mut host, &mut vars, start + TICK);
        assert!(host.ran.is_empty(), "still waiting");
        s.tick(&mut host, &mut vars, start + TICK * 3);
        assert_eq!(host.ran, vec!["say after"]);
    }

    #[test]
    fn a_waiting_command_runs_again_until_it_is_done() {
        let mut host = Fake::default();
        host.steps.insert("waitfortarget", Step::Wait);
        let mut vars = Vars::default();
        let mut s = script("waitfortarget 1000\ntarget 'self'");
        run(&mut s, &mut host, &mut vars, 3);
        assert_eq!(host.ran, vec!["waitfortarget 1000"; 3]);
        host.steps.insert("waitfortarget", Step::Done);
        run(&mut s, &mut host, &mut vars, 2);
        assert_eq!(host.ran.last().map(String::as_str), Some("target self"));
    }

    #[test]
    fn if_elseif_else_take_one_branch() {
        let mut host = Fake::default();
        host.values.insert("poisoned", Value::Bool(false));
        host.values.insert("hits", Value::Number(40.0));
        host.values.insert("maxhits", Value::Number(100.0));
        let mut vars = Vars::default();
        let mut s = script(
            "if poisoned\n  say 'cure'\nelseif hits < maxhits\n  say 'bandage'\nelse\n  say 'fine'\nendif",
        );
        run(&mut s, &mut host, &mut vars, 5);
        assert_eq!(host.ran, vec!["say bandage"]);
    }

    #[test]
    fn not_and_or_read_left_to_right() {
        let mut host = Fake::default();
        host.values.insert("dead", Value::Bool(false));
        host.values.insert("hidden", Value::Bool(true));
        let mut vars = Vars::default();
        let mut s = script(
            "if not dead and hidden\n  say 'yes'\nendif\nif dead or not hidden\n  say 'no'\nendif",
        );
        run(&mut s, &mut host, &mut vars, 5);
        assert_eq!(host.ran, vec!["say yes"]);
    }

    #[test]
    fn a_loop_with_no_action_gives_the_tick_back() {
        let mut host = Fake::default();
        host.values.insert("dead", Value::Bool(false));
        let mut vars = Vars::default();
        let mut s = script("while not dead\nendwhile");
        s.tick(&mut host, &mut vars, Instant::now());
        assert_eq!(*s.status(), Status::Running, "the loop goes on next tick");
    }

    #[test]
    fn a_count_loop_runs_its_body_that_many_times_and_again_later() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        let mut s = script("for 3\n  say 'hi'\nendfor\nfor 2\n  say 'yo'\nendfor");
        run(&mut s, &mut host, &mut vars, 3);
        assert_eq!(
            host.ran,
            vec!["say hi", "say hi", "say hi", "say yo", "say yo"]
        );
    }

    #[test]
    fn a_range_loop_counts_from_start_to_end() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        let mut s = script("for 1 to 3\n  say 'x'\nendfor");
        run(&mut s, &mut host, &mut vars, 3);
        assert_eq!(host.ran.len(), 3);
    }

    #[test]
    fn a_range_loop_that_ends_at_the_largest_number_stops() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        let mut s = script(&format!(
            "for {} to {}\n  say 'x'\nendfor",
            i64::MAX - 1,
            i64::MAX
        ));
        run(&mut s, &mut host, &mut vars, 3);
        assert_eq!(host.ran.len(), 2);
        assert_eq!(*s.status(), Status::Done);
    }

    #[test]
    fn a_list_loop_hands_each_item_to_the_body() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        let mut s = script(
            "createlist 'fruit'\npushlist 'fruit' 'apple'\npushlist 'fruit' 'pear'\nfor 0 to 'fruit'\n  say fruit[]\nendfor\nsay fruit[1]",
        );
        run(&mut s, &mut host, &mut vars, 3);
        assert_eq!(host.ran, vec!["say apple", "say pear", "say pear"]);
    }

    #[test]
    fn break_and_continue_steer_a_loop() {
        let mut host = Fake::default();
        host.values.insert("hidden", Value::Bool(true));
        let mut vars = Vars::default();
        let mut s = script("for 5\n  if hidden\n    continue\n  endif\n  say 'never'\nendfor\nwhile hidden\n  break\nendwhile\nsay 'out'");
        run(&mut s, &mut host, &mut vars, 3);
        assert_eq!(host.ran, vec!["say out"]);
    }

    #[test]
    fn aliases_lists_and_timers_are_kept_in_the_shared_set() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        let mut s = script(
            "setalias 'pet' 0x1234\ncreatelist 'l'\npushlist 'l' 'a'\npushlist! 'l' 'A'\npushlist 'l' 'b' 'front'\npoplist 'l' 'back'\ncreatetimer 't'",
        );
        run(&mut s, &mut host, &mut vars, 2);
        assert_eq!(vars.alias("PET"), Some(0x1234));
        assert_eq!(vars.list("l"), Some(&vec!["b".to_string()]));
        assert!(vars.timer("t", Instant::now()).is_some());
        let mut s = script("pushlist 'l' 'pet'\ncreatelist 'l'");
        run(&mut s, &mut host, &mut vars, 2);
        assert_eq!(
            vars.list("l").and_then(|l| l.last()).map(String::as_str),
            Some("4660"),
            "the alias went in as its serial, and createlist kept the list"
        );
    }

    #[test]
    fn inlist_and_poplist_find_an_alias_the_way_pushlist_put_it() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        vars.set_alias("found", 0x1234);
        let mut s = script(
            "pushlist 'seen' 'found'\nif inlist 'seen' 'found' and inlist! 'seen' 'found'\n  say 'in'\nendif\npoplist 'seen' 'found'",
        );
        run(&mut s, &mut host, &mut vars, 2);
        assert_eq!(host.ran, vec!["say in"]);
        assert_eq!(vars.list("seen"), Some(&Vec::new()));
    }

    #[test]
    fn built_in_words_read_aliases_lists_and_timers() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        vars.set_alias("pet", 1);
        vars.list_mut("l").push("x".into());
        let mut s = script("if findalias 'pet' and listexists 'l' and list 'l' == 1 and inlist 'l' 'X'\n  say 'ok'\nendif");
        run(&mut s, &mut host, &mut vars, 2);
        assert_eq!(host.ran, vec!["say ok"]);
    }

    #[test]
    fn a_failed_line_stops_the_script_and_names_the_line() {
        let mut host = Fake::default();
        host.steps
            .insert("cast", Step::Fail("no such spell".into()));
        let mut vars = Vars::default();
        let mut s = script("say 'a'\ncast 'Fly Me'\nsay 'b'");
        run(&mut s, &mut host, &mut vars, 3);
        assert_eq!(
            *s.status(),
            Status::Failed {
                line: 2,
                message: "no such spell".into()
            }
        );
        assert_eq!(host.ran, vec!["say a", "cast Fly Me"]);
    }

    #[test]
    fn an_unknown_word_fails_its_condition_line() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        let mut s = script("\nif flying\nendif");
        run(&mut s, &mut host, &mut vars, 2);
        assert!(matches!(s.status(), Status::Failed { line: 2, .. }));
    }

    #[test]
    fn stop_ends_the_script_and_replay_starts_it_again() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        let mut s = script("say 'a'\nstop\nsay 'b'");
        run(&mut s, &mut host, &mut vars, 3);
        assert_eq!(*s.status(), Status::Stopped);
        assert_eq!(host.ran, vec!["say a"]);

        let mut host = Fake::default();
        host.steps.insert("say", Step::Acted);
        let mut s = script("say 'again'\nreplay");
        run(&mut s, &mut host, &mut vars, 3);
        assert_eq!(host.ran, vec!["say again"; 3]);
    }

    #[test]
    fn a_stopped_script_runs_no_more_lines() {
        let mut host = Fake::default();
        let mut vars = Vars::default();
        let mut s = script("pause 10000\nsay 'late'");
        s.tick(&mut host, &mut vars, Instant::now());
        s.stop();
        run(&mut s, &mut host, &mut vars, 3);
        assert!(host.ran.is_empty());
        assert_eq!(*s.status(), Status::Stopped);
    }
}
