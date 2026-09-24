//! The game side of scripts: the commands and condition words a script uses,
//! the script tools, and the tick that runs the script.
//!
//! A script runs on the session tick, after the reflexes. Each command that
//! acts goes through the same pacing as a tool, so a script is never faster
//! than a player, and self-care always gets the tick first.

mod commands;
mod values;

pub(super) use values::buff_names;

use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use uoterm_assist::skills::SkillBook;
use uoterm_assist::spells::SpellBook;
use uoterm_script::{
    Arg, Call, Ctx, Host, Program, Script, Status, Step, Value as ScriptValue, Vars,
};

use super::*;

/// The folder scripts are read from, below the working directory and below
/// the user's config directory.
pub(super) const SCRIPTS_DIR: &str = "scripts";
/// The extension of a script file, in any case.
pub(super) const SCRIPT_EXT: &str = "txt";
/// How many lines of script output the status keeps.
const OUTPUT_KEPT: usize = 50;
/// The shortest gap after a tick in which the script sent packets. Speech,
/// clicks and requests do not use the action budget, and a loop of them
/// must not go faster than a person types or clicks.
pub(super) const SCRIPT_SEND_GAP: Duration = Duration::from_millis(SCRIPT_SEND_GAP_MS);
const SCRIPT_SEND_GAP_MS: u64 = 250;

/// A script's argument names for the script tools.
const ARG_NAME: &str = "name";
const ARG_TEXT: &str = "text";
const ARG_LOOP: &str = "loop";
const ARG_SLOT: &str = "slot";
const ARG_FOR: &str = "for";
const ARG_ITERATIONS: &str = "iterations";
/// The slot a script from text runs in when the call names none.
const TEXT_SLOT: &str = "text";
/// How many scripts may run at once, each in a slot of its own.
const SLOTS_MAX: usize = 8;
/// How many ended scripts the status remembers, the newest last.
const REPORTS_KEPT: usize = 16;
/// Why a bounded script ended.
const ENDED_TIME_UP: &str = "time up";
const ENDED_ITERATIONS: &str = "iterations done";

const A_SCRIPT_RUNS: &str = "a script runs in that slot; stop it first";
const TOO_MANY_SCRIPTS: &str = "8 scripts run already; stop one first";
const NO_SLOT_NAMED: &str = "no script runs in that slot";
const NO_SCRIPT_RUNS: &str = "no script is running";
const NO_SCRIPT_NAMED: &str = "no script by that name in the scripts folder";
const RUN_NEEDS_SCRIPT: &str = "run_script needs name or text";
const COMMAND_NEEDS_TEXT: &str = "command needs text: one script command";
const COMMAND_IS_FOR_A_HUMAN: &str =
    "command is for a human at the watch window; an agent uses run_script";

/// What the session keeps for scripts.
pub(super) struct Scripting {
    /// Aliases, lists and timers every script of the character shares.
    vars: Vars,
    /// The scripts running, each in its named slot, in the order they
    /// started.
    slots: Vec<Running>,
    /// The slot that goes first on the next tick, so no script keeps the
    /// shared action budget from the others.
    turn: usize,
    /// How the scripts that ended ended, the newest last, for
    /// `script_status` after they are gone.
    reports: VecDeque<Report>,
    /// Lines shown outside any slot: hotkeys, commands and timed lines,
    /// newest last.
    output: VecDeque<String>,
    /// The script sends nothing before this.
    pub(super) resume_at: Instant,
    pub(super) spells: SpellBook,
    pub(super) skills: SkillBook,
    /// Items `useonce` has used, so each is used one time.
    used_once: HashSet<Serial>,
    /// Objects the find commands pass over.
    ignored: HashSet<Serial>,
    /// The item each hand held before `clearhands` or `togglehands` put it
    /// away: left then right.
    hands: [Option<Serial>; 2],
    /// Journal lines at or below this number are behind a `clearjournal`.
    journal_floor: u64,
    /// The last spell and skill used, for `cast 'last'` and `useskill 'last'`.
    last_spell: Option<u16>,
    last_skill: Option<u16>,
    /// The last tile a script targeted, for `targettile 'last'`.
    last_ground: Option<Point3>,
    /// The colour the next dye tub request is answered with.
    pub(super) dye_hue: Option<u16>,
    /// The words a script typed for text fields of the next gump it answers,
    /// by field.
    pub(super) gump_texts: Vec<(u16, String)>,
    /// Lines to show later, each with the time it is due.
    later: Vec<(Instant, String)>,
    /// What the script ticking now asked for after its tick: another script
    /// in its place, or to stop.
    next: Option<Next>,
    /// The slot of the script ticking now.
    current: Option<String>,
}

/// What a script asks the session to do once its tick is over.
enum Next {
    Run(String),
    Stop,
}

struct Running {
    /// The slot it runs in, which names it to the tools.
    slot: String,
    name: String,
    script: Script,
    /// The script as read, to start it again when it loops.
    program: Program,
    /// Start again from the top each time it reaches its end.
    looping: bool,
    /// The lines this script showed its user, newest last.
    output: VecDeque<String>,
    /// It ends when this time comes.
    until: Option<Instant>,
    /// It ends after this many runs from the top.
    iterations_max: Option<u32>,
    /// The runs from the top it finished.
    iterations: u32,
    /// Held by another script: it keeps its place and does not tick.
    suspended: bool,
}

impl Running {
    fn keep_output(&mut self, lines: Vec<String>) {
        keep_lines(&mut self.output, &self.slot, lines);
    }

    /// Why a bounded script is over, when it is.
    fn bound_reached(&self, now: Instant) -> Option<&'static str> {
        if self.until.is_some_and(|until| now >= until) {
            return Some(ENDED_TIME_UP);
        }
        if self
            .iterations_max
            .is_some_and(|most| self.iterations >= most)
        {
            return Some(ENDED_ITERATIONS);
        }
        None
    }

    fn status_json(&self) -> Value {
        json!({
            "slot": self.slot,
            "script": self.name,
            "status": if self.suspended { "suspended" } else { "running" },
            "loop": self.looping,
            "line": self.script.line(),
            "iterations": self.iterations,
            "output": self.output,
        })
    }
}

#[derive(Clone, Debug)]
struct Report {
    slot: String,
    name: String,
    status: Status,
    /// Why a bounded script stopped, when a bound stopped it.
    bound: Option<&'static str>,
    output: VecDeque<String>,
}

impl Report {
    fn json(&self) -> Value {
        let mut body = status_json(&self.status);
        body["slot"] = json!(self.slot);
        body["script"] = json!(self.name);
        body["output"] = json!(self.output);
        if let Some(bound) = self.bound {
            body["ended_by"] = json!(bound);
        }
        body
    }
}

/// Keeps the newest [`OUTPUT_KEPT`] lines of a script's user.
fn keep_lines(kept: &mut VecDeque<String>, from: &str, lines: Vec<String>) {
    for line in lines {
        tracing::info!(from = %from, line = %line, "script says");
        kept.push_back(line);
    }
    while kept.len() > OUTPUT_KEPT {
        kept.pop_front();
    }
}

impl Scripting {
    /// Script state for a character. The skill names come from the client
    /// files; without them skills are used by number only.
    pub(super) fn new(uopath: Option<&Path>) -> Self {
        let skills = uopath
            .map(|path| match uoterm_nav::read_skills(path) {
                Ok(entries) => {
                    tracing::info!(skills = entries.len(), "skill names ready");
                    SkillBook::new(entries.into_iter().map(|s| (s.name, s.usable)))
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to read the client skill names");
                    SkillBook::default()
                }
            })
            .unwrap_or_default();
        Self {
            vars: Vars::default(),
            slots: Vec::new(),
            turn: 0,
            reports: VecDeque::new(),
            output: VecDeque::new(),
            resume_at: Instant::now(),
            spells: SpellBook::standard(),
            skills,
            used_once: HashSet::new(),
            ignored: HashSet::new(),
            hands: [None, None],
            journal_floor: 0,
            last_spell: None,
            last_skill: None,
            last_ground: None,
            dye_hue: None,
            gump_texts: Vec::new(),
            later: Vec::new(),
            next: None,
            current: None,
        }
    }

    fn keep_output(&mut self, lines: Vec<String>) {
        keep_lines(&mut self.output, TOOL_COMMAND, lines);
    }

    /// The place of the script in a slot, in any case.
    fn slot_at(&self, slot: &str) -> Option<usize> {
        self.slots
            .iter()
            .position(|running| running.slot.eq_ignore_ascii_case(slot))
    }

    fn remember(&mut self, report: Report) {
        self.reports.push_back(report);
        while self.reports.len() > REPORTS_KEPT {
            self.reports.pop_front();
        }
    }
}

/// The folders searched for a script by name, in order.
fn script_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from(SCRIPTS_DIR),
        crate::config::config_dir().join(SCRIPTS_DIR),
    ]
}

/// The script files in these folders, in folder order, each with its name:
/// the file name without its extension.
fn script_files(dirs: &[PathBuf]) -> Vec<(String, PathBuf)> {
    dirs.iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flat_map(|entries| entries.flatten())
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case(SCRIPT_EXT))
        })
        .filter_map(|path| {
            let name = path.file_stem()?.to_str()?.to_string();
            Some((name, path))
        })
        .collect()
}

/// The text of the script with this name, in any case.
fn find_script(name: &str) -> Option<String> {
    find_script_in(&script_dirs(), name)
}

fn find_script_in(dirs: &[PathBuf], name: &str) -> Option<String> {
    script_files(dirs)
        .into_iter()
        .filter(|(stem, _)| stem.eq_ignore_ascii_case(name))
        .find_map(|(_, path)| std::fs::read_to_string(path).ok())
}

/// The names of the scripts in the scripts folders.
pub(super) fn script_names() -> Vec<String> {
    let mut names: Vec<String> = script_files(&script_dirs())
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    names.sort_unstable_by_key(|n| n.to_ascii_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    names
}

/// Starts a script by name or from text in a slot of its own: the `slot`
/// named, or the script's name. Several run at once, each in its slot, and
/// share the character's pace. `for` (seconds) and `iterations` bound a run.
pub(super) fn run_script(inner: &mut Inner, args: &Value) -> ToolResult {
    if !shard_allows(inner, AssistFeature::ScriptMacros) {
        return ToolResult::err(forbidden(AssistFeature::ScriptMacros));
    }
    let name = args.get(ARG_NAME).and_then(|v| v.as_str()).map(str::trim);
    let text = args.get(ARG_TEXT).and_then(|v| v.as_str());
    let (name, source) = match (name, text) {
        (_, Some(text)) => (name.unwrap_or(TEXT_SLOT).to_string(), text.to_string()),
        (Some(name), None) if !name.is_empty() => match find_script(name) {
            Some(source) => (name.to_string(), source),
            None => return ToolResult::err(NO_SCRIPT_NAMED),
        },
        _ => return ToolResult::err(RUN_NEEDS_SCRIPT),
    };
    let slot = args
        .get(ARG_SLOT)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|slot| !slot.is_empty())
        .unwrap_or(&name)
        .to_string();
    if inner.scripting.slot_at(&slot).is_some() {
        return ToolResult::err(A_SCRIPT_RUNS);
    }
    if inner.scripting.slots.len() >= SLOTS_MAX {
        return ToolResult::err(TOO_MANY_SCRIPTS);
    }
    let iterations_max = arg_number(args, ARG_ITERATIONS).filter(|n| *n > 0);
    // More than one run from the top is a loop.
    let looping = args
        .get(ARG_LOOP)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || iterations_max.is_some_and(|n| n > 1);
    let bounds = Bounds {
        until: args
            .get(ARG_FOR)
            .and_then(Value::as_f64)
            .filter(|seconds| *seconds > 0.0)
            .map(|seconds| Instant::now() + Duration::from_secs_f64(seconds)),
        iterations_max,
    };
    start_script(inner, slot, name, &source, looping, bounds)
}

/// How long a script may run: until a time, and for a number of runs from
/// the top. Either may be left open.
#[derive(Clone, Copy, Default)]
struct Bounds {
    until: Option<Instant>,
    iterations_max: Option<u32>,
}

/// Runs one script command at once, for a human at the watch window: a
/// skill lock, an ability, a party invite, an answer to a prompt. One act of
/// a human is not a macro, so the shard's macro switch does not apply.
/// A command that must wait for the game is not waited for.
pub(super) fn command(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(text) = args.get(ARG_TEXT).and_then(|v| v.as_str()) else {
        return ToolResult::err(COMMAND_NEEDS_TEXT);
    };
    if args.get(crate::tools::ARG_HUMAN).and_then(Value::as_bool) != Some(true) {
        return ToolResult::err(COMMAND_IS_FOR_A_HUMAN);
    }
    run_now(inner, TOOL_COMMAND, text)
}

fn start_script(
    inner: &mut Inner,
    slot: String,
    name: String,
    source: &str,
    looping: bool,
    bounds: Bounds,
) -> ToolResult {
    let program = match Program::parse(source) {
        Ok(program) => program,
        Err(e) => return ToolResult::err(e.to_string()),
    };
    // A script that starts itself again is a looping macro.
    let loops = looping || program.ops.contains(&uoterm_script::Op::Replay);
    if loops && !shard_allows(inner, AssistFeature::LoopedMacros) {
        return ToolResult::err(forbidden(AssistFeature::LoopedMacros));
    }
    tracing::info!(script = %name, slot = %slot, looping, "script starts");
    inner.scripting.slots.push(Running {
        script: Script::new(program.clone()),
        program,
        slot: slot.clone(),
        name: name.clone(),
        looping,
        output: VecDeque::new(),
        until: bounds.until,
        iterations_max: bounds.iterations_max,
        iterations: 0,
        suspended: false,
    });
    ToolResult::ok(json!({
        "script": name,
        "slot": slot,
        "loop": looping,
        "iterations": bounds.iterations_max,
    }))
}

/// The slots of the scripts running now.
pub(super) fn running_slots(inner: &Inner) -> Vec<&str> {
    inner
        .scripting
        .slots
        .iter()
        .map(|r| r.slot.as_str())
        .collect()
}

/// Runs script lines at once, outside the one running script: a hotkey.
/// A line that must wait for the character is not waited for; the caller
/// hears so and may press again.
pub(super) fn run_now(inner: &mut Inner, name: &str, text: &str) -> ToolResult {
    let program = match Program::parse(text) {
        Ok(program) => program,
        Err(e) => return ToolResult::err(e.to_string()),
    };
    let mut script = Script::new(program);
    let mut vars = std::mem::take(&mut inner.scripting.vars);
    script.tick(&mut Game { inner }, &mut vars, Instant::now());
    inner.scripting.vars = vars;
    let output = script.take_output();
    inner.scripting.keep_output(output.clone());
    match script.status() {
        Status::Done | Status::Stopped => {
            ToolResult::ok(json!({ "hotkey": name, "output": output }))
        }
        Status::Running => ToolResult::err(MUST_WAIT),
        Status::Failed { message, .. } => ToolResult::err(message.clone()),
    }
}

/// Stops the script of one slot, or every script when the call names no
/// slot.
pub(super) fn stop_script(inner: &mut Inner, args: &Value) -> ToolResult {
    let slot = args.get(ARG_SLOT).and_then(Value::as_str).map(str::trim);
    let stopped: Vec<Running> = match slot {
        Some(slot) => match inner.scripting.slot_at(slot) {
            Some(at) => vec![inner.scripting.slots.remove(at)],
            None => return ToolResult::err(NO_SLOT_NAMED),
        },
        None if inner.scripting.slots.is_empty() => return ToolResult::err(NO_SCRIPT_RUNS),
        None => std::mem::take(&mut inner.scripting.slots),
    };
    let slots: Vec<String> = stopped.iter().map(|r| r.slot.clone()).collect();
    for mut running in stopped {
        running.script.stop();
        finish(inner, running, None);
    }
    ToolResult::ok(json!({ "stopped": true, "slots": slots }))
}

/// The script of one slot, running or ended; with no slot, the first one
/// running or else the last that ended, and every slot besides.
pub(super) fn script_status(inner: &Inner, args: &Value) -> ToolResult {
    let s = &inner.scripting;
    let ended = |slot: Option<&str>| {
        s.reports
            .iter()
            .rev()
            .find(|r| slot.is_none_or(|slot| r.slot.eq_ignore_ascii_case(slot)))
    };
    if let Some(slot) = args.get(ARG_SLOT).and_then(Value::as_str).map(str::trim) {
        return match (s.slot_at(slot), ended(Some(slot))) {
            (Some(at), _) => ToolResult::ok(s.slots[at].status_json()),
            (None, Some(report)) => ToolResult::ok(report.json()),
            (None, None) => ToolResult::err(NO_SLOT_NAMED),
        };
    }
    let mut body = match (s.slots.first(), ended(None)) {
        (Some(running), _) => running.status_json(),
        (None, Some(report)) => report.json(),
        (None, None) => json!({ "status": "none", "output": s.output }),
    };
    body["slots"] = json!(s.slots.iter().map(Running::status_json).collect::<Vec<_>>());
    body["ended"] = json!(s.reports.iter().map(Report::json).collect::<Vec<_>>());
    body["shown"] = json!(s.output);
    ToolResult::ok(body)
}

const NEEDS_NAME: &str = "needs name";
const BAD_NAME: &str = "a script name has letters, digits, spaces, - and _ only";
/// No script name is longer than this.
const NAME_MAX_CHARS: usize = 48;

/// A name that is safe as a file name: it cannot leave the scripts folder.
fn script_name(args: &Value) -> std::result::Result<&str, &'static str> {
    let name = args
        .get(ARG_NAME)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or(NEEDS_NAME)?;
    let safe = name.chars().count() <= NAME_MAX_CHARS
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'));
    safe.then_some(name).ok_or(BAD_NAME)
}

pub(super) fn script_read(args: &Value) -> ToolResult {
    let name = match script_name(args) {
        Ok(name) => name,
        Err(words) => return ToolResult::err(words),
    };
    match find_script(name) {
        Some(text) => ToolResult::ok(json!({ "name": name, "text": text })),
        None => ToolResult::err(NO_SCRIPT_NAMED),
    }
}

/// Saves a script that parses. A fault is refused with its line, so the
/// folder holds no script that cannot start.
pub(super) fn script_save(args: &Value) -> ToolResult {
    let name = match script_name(args) {
        Ok(name) => name,
        Err(words) => return ToolResult::err(words),
    };
    let text = args.get(ARG_TEXT).and_then(Value::as_str).unwrap_or("");
    if let Err(fault) = uoterm_script::Program::parse(text) {
        return ToolResult::err(fault.to_string());
    }
    match super::recorder::save(name, text) {
        Ok(path) => ToolResult::ok(json!({ "name": name, "path": path })),
        Err(words) => ToolResult::err(words),
    }
}

pub(super) fn list_scripts() -> ToolResult {
    ToolResult::ok(json!({ "scripts": script_names() }))
}

fn status_json(status: &Status) -> Value {
    match status {
        Status::Running => json!({ "status": "running" }),
        Status::Done => json!({ "status": "done" }),
        Status::Stopped => json!({ "status": "stopped" }),
        Status::Failed { line, message } => {
            json!({ "status": "failed", "line": line, "error": message })
        }
    }
}

fn finish(inner: &mut Inner, mut running: Running, bound: Option<&'static str>) {
    let lines = running.script.take_output();
    running.keep_output(lines);
    let status = running.script.status().clone();
    tracing::info!(script = %running.name, slot = %running.slot, status = ?status, ?bound, "script ends");
    let how = match (&status, bound) {
        (_, Some(bound)) => format!("{} {bound}", crate::jobs::REASON_DONE),
        (Status::Done, None) => crate::jobs::REASON_DONE.to_string(),
        (Status::Stopped, None) => crate::jobs::REASON_STOPPED.to_string(),
        (Status::Running, None) => String::new(),
        (Status::Failed { message, .. }, None) => format!("failed: {message}"),
    };
    job_ended(
        inner,
        crate::jobs::JOB_SCRIPT,
        &format!("{} {how}", running.slot),
    );
    inner.scripting.remember(Report {
        slot: running.slot,
        name: running.name,
        status,
        bound,
        output: running.output,
    });
}

/// Runs the scripts a tick's worth, each in turn. They share one pace: once
/// one has sent something, the rest wait for the gap after it, and the next
/// tick starts with the slot after it. Death and a lost login end them all.
pub(super) fn pump_script(inner: &mut Inner, now: Instant) {
    show_due_lines(inner, now);
    if inner.scripting.slots.is_empty() {
        return;
    }
    let dead = {
        let world = inner.world.read();
        !world.logged_in || world.self_state.dead
    };
    if dead {
        for mut running in std::mem::take(&mut inner.scripting.slots) {
            running.script.stop();
            finish(inner, running, None);
        }
        return;
    }
    let count = inner.scripting.slots.len();
    let first = inner.scripting.turn % count;
    let order: Vec<String> = (0..count)
        .map(|k| inner.scripting.slots[(first + k) % count].slot.clone())
        .collect();
    for slot in order {
        if now < inner.scripting.resume_at {
            break;
        }
        let Some(at) = inner.scripting.slot_at(&slot) else {
            continue;
        };
        if tick_slot(inner, at, now) {
            // It spent the pace: the next tick starts with the one after.
            inner.scripting.turn = inner.scripting.slot_at(&slot).map_or(at, |still| still + 1);
        }
    }
}

/// Runs one slot's script a tick's worth. True when it sent something.
fn tick_slot(inner: &mut Inner, at: usize, now: Instant) -> bool {
    let mut running = inner.scripting.slots.remove(at);
    if let Some(bound) = running.bound_reached(now) {
        running.script.stop();
        finish(inner, running, Some(bound));
        return false;
    }
    if running.suspended {
        inner.scripting.slots.insert(at, running);
        return false;
    }
    let mut vars = std::mem::take(&mut inner.scripting.vars);
    inner.scripting.current = Some(running.slot.clone());
    let sent_before = inner.outbound.len();
    running.script.tick(&mut Game { inner }, &mut vars, now);
    inner.scripting.vars = vars;
    inner.scripting.current = None;
    let sent = inner.outbound.len() > sent_before;
    if sent {
        inner.scripting.resume_at = now + SCRIPT_SEND_GAP;
    }
    let lines = running.script.take_output();
    running.keep_output(lines);
    match inner.scripting.next.take() {
        // Starting itself again is a looping macro.
        Some(Next::Run(name))
            if name.eq_ignore_ascii_case(&running.name)
                && !shard_allows(inner, AssistFeature::LoopedMacros) =>
        {
            running.keep_output(vec![forbidden(AssistFeature::LoopedMacros)]);
            running.script.stop();
            finish(inner, running, None);
        }
        Some(Next::Run(name)) => {
            let slot = running.slot.clone();
            running.script.stop();
            finish(inner, running, None);
            let result = match find_script(&name) {
                Some(source) => start_script(inner, slot, name, &source, false, Bounds::default()),
                None => ToolResult::err(format!("{name}: {NO_SCRIPT_NAMED}")),
            };
            if let Some(error) = result.error {
                inner.scripting.keep_output(vec![error]);
            }
        }
        Some(Next::Stop) => {
            running.script.stop();
            finish(inner, running, None);
        }
        None if *running.script.status() == Status::Running => {
            inner.scripting.slots.insert(at, running);
        }
        // A looping script starts again from the top at the next tick.
        None if running.looping && *running.script.status() == Status::Done => {
            running.iterations += 1;
            match running.bound_reached(now) {
                Some(bound) => finish(inner, running, Some(bound)),
                None => {
                    running.script = Script::new(running.program.clone());
                    inner.scripting.slots.insert(at, running);
                }
            }
        }
        None => {
            if *running.script.status() == Status::Done {
                running.iterations += 1;
            }
            finish(inner, running, None);
        }
    }
    sent
}

/// Shows the `timermsg` lines whose time has come.
fn show_due_lines(inner: &mut Inner, now: Instant) {
    let (due, waiting): (Vec<_>, Vec<_>) = std::mem::take(&mut inner.scripting.later)
        .into_iter()
        .partition(|(at, _)| *at <= now);
    inner.scripting.later = waiting;
    inner
        .scripting
        .keep_output(due.into_iter().map(|(_, line)| line).collect());
}

/// The session as a script sees it.
struct Game<'a> {
    inner: &'a mut Inner,
}

impl Host for Game<'_> {
    fn command(&mut self, call: &Call, ctx: &mut Ctx) -> Step {
        commands::run(self, call, ctx)
    }

    fn value(&mut self, call: &Call, ctx: &mut Ctx) -> std::result::Result<ScriptValue, String> {
        values::read(self, call, ctx)
    }
}

/// Where a find command looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Source {
    /// The backpack and the bags in it.
    Backpack,
    /// Items on the ground.
    Ground,
    /// The ground and every container the character can see.
    World,
    /// One container and the bags in it.
    Container(Serial),
}

const SOURCE_BACKPACK: &str = "backpack";
const SOURCE_GROUND: &str = "ground";
const SOURCE_WORLD: &str = "world";
const ANY: &str = "any";
/// A colour written as this number means any colour.
const ANY_COLOR: i64 = -1;
/// How far the ground is searched when a script names no range: the
/// distance the client shows objects at.
const DEFAULT_RANGE: u32 = 18;

impl Game<'_> {
    fn world(&self) -> parking_lot::RwLockReadGuard<'_, World> {
        self.inner.world.read()
    }

    fn me(&self) -> Serial {
        self.world().self_state.serial
    }

    /// The serial an argument names: a number, a system alias such as
    /// `self` or `backpack`, or an alias a script set.
    fn serial(&self, arg: &Arg, ctx: &Ctx) -> std::result::Result<Serial, String> {
        if let Some(n) = arg.number() {
            return u32::try_from(n)
                .map(Serial)
                .map_err(|_| format!("'{}' is not a serial", arg.text));
        }
        self.system_alias(&arg.text)
            .or_else(|| ctx.vars.alias(&arg.text).map(Serial))
            .ok_or_else(|| format!("no alias '{}'", arg.text))
    }

    fn system_alias(&self, name: &str) -> Option<Serial> {
        let key = name.to_ascii_lowercase();
        // Read with no guard held: it takes the lock itself.
        if key == "last" || key == "lasttarget" {
            return last_target_for_cursor(self.inner);
        }
        let world = self.world();
        match key.as_str() {
            "self" => Some(world.self_state.serial),
            "backpack" => backpack_serial(&world),
            "bank" => world.bank_box(),
            "lastobject" => self.inner.last_object,
            "lefthand" => world.worn(LAYER_TWO_HANDED).map(|e| e.serial),
            "righthand" => world.worn(LAYER_ONE_HANDED).map(|e| e.serial),
            _ => None,
        }
    }

    /// A graphic argument, as a number.
    fn graphic(arg: &Arg) -> std::result::Result<u16, String> {
        arg.number()
            .and_then(|n| u16::try_from(n).ok())
            .ok_or_else(|| format!("'{}' is not an item graphic", arg.text))
    }

    /// A colour argument: `None` means any colour.
    fn color(arg: Option<&Arg>) -> std::result::Result<Option<u16>, String> {
        match arg {
            None => Ok(None),
            Some(a) if a.is(ANY) => Ok(None),
            Some(a) => match a.number() {
                Some(ANY_COLOR) => Ok(None),
                Some(n) => u16::try_from(n)
                    .map(Some)
                    .map_err(|_| format!("'{}' is not a colour", a.text)),
                None => Err(format!("'{}' is not a colour", a.text)),
            },
        }
    }

    fn source(&self, arg: Option<&Arg>, ctx: &Ctx) -> std::result::Result<Source, String> {
        let Some(arg) = arg else {
            return Ok(Source::Backpack);
        };
        if arg.is(SOURCE_BACKPACK) {
            return Ok(Source::Backpack);
        }
        if arg.is(SOURCE_GROUND) {
            return Ok(Source::Ground);
        }
        if arg.is(SOURCE_WORLD) {
            return Ok(Source::World);
        }
        self.serial(arg, ctx).map(Source::Container)
    }

    fn range(arg: Option<&Arg>) -> std::result::Result<u32, String> {
        match arg {
            None => Ok(DEFAULT_RANGE),
            Some(a) => a
                .number()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| format!("'{}' is not a range", a.text)),
        }
    }

    /// The items of a kind in a place, nearest first on the ground and in
    /// serial order in a container. Ignored objects are passed over.
    fn find_items(
        &self,
        graphic: u16,
        color: Option<u16>,
        source: Source,
        range: u32,
    ) -> Vec<Serial> {
        let world = self.world();
        let here = world.self_state.location;
        let ignored = &self.inner.scripting.ignored;
        let pack = backpack_serial(&world);
        let mut found: Vec<&uoterm_world::Item> = world
            .items
            .values()
            .filter(|i| i.graphic == graphic && color.is_none_or(|c| i.hue == c))
            .filter(|i| !ignored.contains(&i.serial))
            .filter(|i| match source {
                Source::Backpack => pack.is_some_and(|p| world.is_inside(i.serial, p)),
                Source::Container(c) => world.is_inside(i.serial, c),
                Source::Ground => i.parent.is_none() && here.chebyshev(i.location) <= range,
                Source::World => world
                    .map_location(i.serial)
                    .is_some_and(|at| here.chebyshev(at) <= range),
            })
            .collect();
        found.sort_by_key(|i| {
            let at = world.map_location(i.serial).unwrap_or(i.location);
            (here.chebyshev(at), i.serial.0)
        });
        found.into_iter().map(|i| i.serial).collect()
    }

    /// The mobiles of a body within a range, nearest first. Ignored ones are
    /// passed over.
    fn find_mobiles(&self, body: u16, color: Option<u16>, range: u32) -> Vec<Serial> {
        let world = self.world();
        let here = world.self_state.location;
        let ignored = &self.inner.scripting.ignored;
        let mut found: Vec<&uoterm_world::Mobile> = world
            .mobiles
            .values()
            .filter(|m| m.body == body && color.is_none_or(|c| m.hue == c))
            .filter(|m| !ignored.contains(&m.serial))
            .filter(|m| here.chebyshev(m.location) <= range)
            .collect();
        found.sort_by_key(|m| (here.chebyshev(m.location), m.serial.0));
        found.into_iter().map(|m| m.serial).collect()
    }

    /// Says a line to the script's user unless the command was marked quiet.
    fn note(call: &Call, ctx: &mut Ctx, text: impl Into<String>) {
        if !call.quiet {
            ctx.say(format!("{}: {}", call.name, text.into()));
        }
    }
}

/// The argument at `i`, or a failure naming what is missing.
fn need<'c>(call: &'c Call, i: usize, what: &str) -> std::result::Result<&'c Arg, String> {
    call.args
        .get(i)
        .ok_or_else(|| format!("{} needs {what}", call.name))
}

/// A number argument at `i`.
fn need_number(call: &Call, i: usize, what: &str) -> std::result::Result<i64, String> {
    let arg = need(call, i, what)?;
    arg.number()
        .ok_or_else(|| format!("{}: '{}' is not {what}", call.name, arg.text))
}

/// Turns a result of a command helper into a script step.
fn step(result: std::result::Result<Step, String>) -> Step {
    result.unwrap_or_else(Step::Fail)
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::{armed_session, ready_to_act, target_cursor, A_CURSOR, PACK};
    use super::*;
    use uoterm_assist::items::PLAIN_HUE;
    use uoterm_protocol::{MobileView, NOTO_GREY, NOTO_INNOCENT};

    const ME: Serial = Serial(0x0000_0001);
    const POTION: Serial = Serial(0x4000_0C01);
    const BANDAGES: Serial = Serial(0x4000_0C02);
    const GREATER_HEAL: u16 = 29;
    const CURE: u16 = 11;

    /// A character in the world with a pack, ready to act.
    fn player() -> Inner {
        let inner = armed_session();
        {
            let mut w = inner.world.write();
            w.logged_in = true;
            w.self_state.serial = ME;
            w.self_state.hits = 50;
            w.self_state.hits_max = 100;
        }
        inner
    }

    fn put_in_pack(inner: &mut Inner, serial: Serial, graphic: u16) {
        inner.world.write().items.insert(
            serial,
            uoterm_world::Item {
                serial,
                graphic,
                amount: 5,
                hue: PLAIN_HUE,
                location: Point3::new(0, 0, 0),
                parent: Some(PACK),
                layer: None,
                grid: 0,
                name: String::new(),
                flags: 0,
            },
        );
    }

    fn start(inner: &mut Inner, text: &str) {
        let started = run_script(inner, &json!({ "text": text }));
        assert!(started.ok, "{:?}", started.error);
    }

    /// Ticks the script, letting the character act each tick.
    fn tick(inner: &mut Inner, ticks: usize) {
        for _ in 0..ticks {
            ready_to_act(inner);
            pump_script(inner, Instant::now());
        }
    }

    fn sent(inner: &Inner, packet: &[u8]) -> bool {
        inner.outbound.iter().any(|p| p.as_slice() == packet)
    }

    fn status(inner: &Inner) -> Value {
        script_status(inner, &json!({})).result
    }

    #[test]
    fn a_script_name_cannot_leave_the_scripts_folder() {
        assert_eq!(
            script_name(&json!({ "name": " heal self " })),
            Ok("heal self")
        );
        assert_eq!(
            script_name(&json!({ "name": "../etc/passwd" })),
            Err(BAD_NAME)
        );
        assert_eq!(script_name(&json!({ "name": "a/b" })), Err(BAD_NAME));
        assert_eq!(script_name(&json!({ "name": "" })), Err(NEEDS_NAME));
        assert_eq!(script_name(&json!({})), Err(NEEDS_NAME));
        let long = "a".repeat(NAME_MAX_CHARS + 1);
        assert_eq!(script_name(&json!({ "name": long })), Err(BAD_NAME));
    }

    #[test]
    fn a_script_that_does_not_parse_is_not_saved() {
        let refused = script_save(&json!({ "name": "broken", "text": "if\n" }));
        assert!(!refused.ok);
        assert!(!script_read(&json!({ "name": "no such script here" })).ok);
    }

    #[test]
    fn a_spell_is_cast_by_name_on_a_target() {
        let mut inner = player();
        start(&mut inner, "cast 'Greater Heal' 'self'");
        tick(&mut inner, 1);
        assert!(sent(&inner, &encode::cast(GREATER_HEAL, inner.version)));
        inner.outbound.clear();
        ingest(&mut inner, &target_cursor(A_CURSOR));
        assert!(
            sent(&inner, &encode::target_object(A_CURSOR, ME, 0, 0, 0, 0)),
            "the spell's cursor is answered with the character"
        );
    }

    #[test]
    fn miniheal_cures_first_when_the_target_is_poisoned() {
        let mut inner = player();
        inner.world.write().self_state.poisoned = true;
        start(&mut inner, "miniheal");
        tick(&mut inner, 1);
        assert!(sent(&inner, &encode::cast(CURE, inner.version)));
    }

    #[test]
    fn a_potion_is_drunk_by_its_graphic() {
        let mut inner = player();
        put_in_pack(&mut inner, POTION, GRAPHIC_POTION_HEAL);
        start(&mut inner, "usetype 0x0f0c");
        tick(&mut inner, 1);
        assert!(sent(&inner, &encode::double_click(POTION)));
    }

    #[test]
    fn a_hurt_character_bandages_herself() {
        let mut inner = player();
        put_in_pack(&mut inner, BANDAGES, GRAPHIC_BANDAGE);
        start(&mut inner, "if hits < maxhits\n  bandageself\nendif");
        tick(&mut inner, 2);
        assert!(sent(&inner, &encode::bandage_target(BANDAGES, ME)));
    }

    #[test]
    fn the_closest_grey_is_the_enemy_and_is_attacked() {
        const NEAR_GREY: Serial = Serial(0x0000_0100);
        const FAR_GREY: Serial = Serial(0x0000_0101);
        const BLUE: Serial = Serial(0x0000_0102);
        let mut inner = player();
        for (serial, x, noto) in [
            (FAR_GREY, 8, NOTO_GREY),
            (NEAR_GREY, 3, NOTO_GREY),
            (BLUE, 1, NOTO_INNOCENT),
        ] {
            inner
                .world
                .write()
                .apply(&Inbound::MobileIncoming(MobileView {
                    serial,
                    body: 0x0190,
                    x,
                    y: 0,
                    z: 0,
                    direction: 0,
                    hue: 0,
                    flags: 0,
                    notoriety: noto,
                    equipment: Vec::new(),
                    hits: None,
                    hits_max: None,
                }));
        }
        start(&mut inner, "getenemy 'gray' 'closest'\nattack 'enemy'");
        tick(&mut inner, 2);
        assert!(sent(&inner, &encode::attack(NEAR_GREY)));
    }

    #[test]
    fn a_script_waits_for_the_cursor_then_targets() {
        let mut inner = player();
        start(&mut inner, "waitfortarget 5000\ntarget 'self'");
        tick(&mut inner, 3);
        assert_eq!(status(&inner)["status"], "running", "still waiting");
        ingest(&mut inner, &target_cursor(A_CURSOR));
        tick(&mut inner, 2);
        assert!(sent(
            &inner,
            &encode::target_object(A_CURSOR, ME, 0, 0, 0, 0)
        ));
        assert_eq!(status(&inner)["status"], "done");
    }

    /// Pet and town commands are said again and again. The agent's small-talk
    /// rule against repeats must not stop a script's commands.
    #[test]
    fn a_script_can_say_the_same_command_twice() {
        let mut inner = player();
        start(&mut inner, "msg 'all kill'\nmsg 'all kill'");
        tick(&mut inner, 3);
        let said = inner
            .outbound
            .iter()
            .filter(|p| p.first() == Some(&PKT_UNICODE_SPEECH))
            .count();
        assert_eq!(said, 2);
    }

    #[test]
    fn a_colour_after_the_words_speaks_in_it() {
        const HUE: u16 = 0x0035;
        let mut inner = player();
        start(&mut inner, "yellmsg 'guards' 0x0035");
        tick(&mut inner, 1);
        let hued = encode::keyword_speech(SPEECH_YELL, HUE, &[], "guards");
        assert!(inner.outbound.contains(&hued));
    }

    #[test]
    fn a_party_remove_with_no_one_named_asks_for_a_target() {
        let mut inner = player();
        start(&mut inner, "partyremove");
        tick(&mut inner, 1);
        assert!(inner.outbound.contains(&encode::party_remove(Serial(0))));
    }

    #[test]
    fn a_number_off_the_map_stops_the_script() {
        for line in [
            "pathfindto -1 100",
            "targettile 100 100 300",
            "targettileoffset 70000 0 0",
            "replygump 'any' -1",
        ] {
            let mut inner = player();
            start(&mut inner, line);
            tick(&mut inner, 1);
            assert_eq!(status(&inner)["status"], "failed", "{line}");
        }
    }

    /// A script's gump answer is checked as the tool's is: a button the
    /// gump does not have stops the script and sends nothing. Words typed with
    /// `gumptext` go in their field, and `closegump 'gump'` answers with the
    /// button that closes it.
    #[test]
    fn a_script_answers_a_gump_only_with_what_it_has() {
        const NAME_GUMP: u32 = 0x0000_4242;
        const OKAY: u32 = 1;
        const NAME_FIELD: u16 = 3;
        const LAYOUT: &str = "{ page 0 }{ button 10 210 4005 4007 1 0 1 }\
            { textentry 20 40 200 20 0 3 0 }";
        let open = |inner: &mut Inner| {
            inner
                .world
                .write()
                .apply(&uoterm_protocol::Inbound::Gump(uoterm_protocol::OpenGump {
                    serial: ME,
                    gump_id: NAME_GUMP,
                    x: 0,
                    y: 0,
                    layout: LAYOUT.into(),
                    text: vec!["nobody".into()],
                }));
        };
        let mut inner = player();
        open(&mut inner);
        start(&mut inner, "replygump 'any' 9");
        tick(&mut inner, 1);
        assert_eq!(status(&inner)["status"], "failed", "no button 9");
        assert!(!inner
            .outbound
            .iter()
            .any(|p| p.first() == Some(&PKT_GUMP_RESPONSE)));

        let mut inner = player();
        open(&mut inner);
        start(&mut inner, "gumptext 3 'Mara'\nreplygump 'any' 1");
        tick(&mut inner, 2);
        assert!(sent(
            &inner,
            &encode::gump_response(ME, NAME_GUMP, OKAY, &[], &[(NAME_FIELD, "Mara".into())])
        ));

        let mut inner = player();
        open(&mut inner);
        start(&mut inner, "closegump 'gump' 'any'");
        tick(&mut inner, 1);
        assert!(sent(
            &inner,
            &encode::gump_response(ME, NAME_GUMP, 0, &[], &[(NAME_FIELD, "nobody".into())])
        ));
    }

    #[test]
    fn a_party_line_goes_to_the_member_named_after_its_colour() {
        const MEMBER: Serial = Serial(0x0000_0042);
        let mut inner = player();
        start(
            &mut inner,
            "partymsg 'heal me' 0 0x42\npartymsg 'all of you' 33",
        );
        tick(&mut inner, 1);
        assert!(sent(
            &inner,
            &encode::party_message(Some(MEMBER), "heal me")
        ));
        ready_to_act(&mut inner);
        tick(&mut inner, 1);
        assert!(sent(&inner, &encode::party_message(None, "all of you")));
    }

    #[test]
    fn a_sending_loop_goes_no_faster_than_a_person() {
        let mut inner = player();
        start(&mut inner, "while true\n  msg 'hi'\nendwhile");
        let now = Instant::now();
        pump_script(&mut inner, now);
        pump_script(&mut inner, now + SCRIPT_SEND_GAP / 2);
        let said = |inner: &Inner| {
            inner
                .outbound
                .iter()
                .filter(|p| p.first() == Some(&PKT_UNICODE_SPEECH))
                .count()
        };
        assert_eq!(said(&inner), 1, "the second tick is too soon");
        pump_script(&mut inner, now + SCRIPT_SEND_GAP);
        assert_eq!(said(&inner), 2);
    }

    #[test]
    fn findalias_knows_the_game_aliases() {
        let mut inner = player();
        start(
            &mut inner,
            "if findalias 'backpack'\n  msg 'pack'\nendif\nif findalias 'bank'\n  msg 'bank'\nendif",
        );
        tick(&mut inner, 4);
        let said = inner
            .outbound
            .iter()
            .filter(|p| p.first() == Some(&PKT_UNICODE_SPEECH))
            .count();
        assert_eq!(said, 1, "a pack is worn, no bank box is open");
    }

    /// `while movetype` keeps moving while items are left, even when the
    /// character must wait a moment before each move. A check that read the
    /// wait as false ended the loop before the first move.
    #[test]
    fn a_movetype_loop_waits_for_pacing_and_moves_every_item() {
        const BAG: Serial = Serial(0x4000_0C09);
        const FIRST: Serial = Serial(0x4000_0C0A);
        const SECOND: Serial = Serial(0x4000_0C0B);
        const GRAPHIC_HIDES: u16 = 0x1079;
        let mut inner = player();
        put_in_pack(&mut inner, FIRST, GRAPHIC_HIDES);
        put_in_pack(&mut inner, SECOND, GRAPHIC_HIDES);
        start(
            &mut inner,
            "while movetype 0x1079 'backpack' 0x40000C09\n  sysmsg 'moved'\nendwhile",
        );
        let lifted = |inner: &Inner, item: Serial| {
            inner
                .outbound
                .iter()
                .any(|p| p.first() == Some(&PKT_LIFT) && p[1..5] == item.0.to_be_bytes())
        };
        // The first tick is too soon to act: the check must wait, not end.
        inner.next_action_at = Instant::now() + Duration::from_secs(1);
        pump_script(&mut inner, Instant::now());
        assert_eq!(status(&inner)["status"], "running", "the loop waits");
        for item in [FIRST, SECOND] {
            ready_to_act(&mut inner);
            pump_script(&mut inner, Instant::now());
            assert!(lifted(&inner, item), "moved {item}");
            inner
                .world
                .write()
                .items
                .get_mut(&item)
                .expect("an item")
                .parent = Some(BAG);
        }
        tick(&mut inner, 3);
        assert_eq!(status(&inner)["status"], "done");
    }

    /// The shard says nothing when a lift is out of reach, so the script
    /// must: a move from a corpse eight tiles away stops with the distance.
    #[test]
    fn a_move_out_of_reach_stops_the_script_and_says_why() {
        const CORPSE: Serial = Serial(0x4000_0C0C);
        const HIDES: Serial = Serial(0x4000_0C0D);
        const FAR_TILES: u16 = 8;
        let mut inner = player();
        let here = inner.world.read().self_state.location;
        {
            let mut w = inner.world.write();
            for (serial, parent, at) in [
                (
                    CORPSE,
                    None,
                    Point3::new(here.x + FAR_TILES, here.y, here.z),
                ),
                (HIDES, Some(CORPSE), Point3::new(0, 0, 0)),
            ] {
                w.items.insert(
                    serial,
                    uoterm_world::Item {
                        serial,
                        graphic: 0x1079,
                        amount: 1,
                        hue: PLAIN_HUE,
                        location: at,
                        parent,
                        layer: None,
                        grid: 0,
                        name: String::new(),
                        flags: 0,
                    },
                );
            }
        }
        start(&mut inner, "moveitem 0x40000C0D 'backpack'");
        tick(&mut inner, 1);
        let s = status(&inner);
        assert_eq!(s["status"], "failed");
        assert!(
            s["error"]
                .as_str()
                .unwrap_or_default()
                .contains("8 tiles away"),
            "{s}"
        );
    }

    #[test]
    fn findtype_sets_found_for_the_next_line() {
        let mut inner = player();
        put_in_pack(&mut inner, POTION, GRAPHIC_POTION_HEAL);
        start(&mut inner, "if findtype 0xf0c\n  useobject 'found'\nendif");
        tick(&mut inner, 2);
        assert!(sent(&inner, &encode::double_click(POTION)));
    }

    #[test]
    fn an_unknown_command_stops_the_script_on_its_line() {
        let mut inner = player();
        start(&mut inner, "pause 0\nflapwings");
        tick(&mut inner, 2);
        let s = status(&inner);
        assert_eq!(s["status"], "failed");
        assert_eq!(s["line"], 2);
    }

    #[test]
    fn one_script_runs_at_a_time_and_stop_ends_it() {
        let mut inner = player();
        start(&mut inner, "pause 60000");
        assert!(!run_script(&mut inner, &json!({ "text": "pause 1" })).ok);
        assert!(stop_script(&mut inner, &json!({})).ok);
        assert_eq!(status(&inner)["status"], "stopped");
        assert!(run_script(&mut inner, &json!({ "text": "pause 1" })).ok);
    }

    #[test]
    fn scripts_in_named_slots_run_side_by_side_and_take_turns() {
        let mut inner = player();
        let healer = json!({ "text": "msg 'heal'", "loop": true, "slot": "healer" });
        assert!(run_script(&mut inner, &healer).ok);
        assert!(
            run_script(
                &mut inner,
                &json!({ "text": "msg 'work'", "loop": true, "slot": "task" })
            )
            .ok
        );
        assert!(!run_script(&mut inner, &healer).ok, "one script a slot");
        assert_eq!(running_slots(&inner), vec!["healer", "task"]);
        let mut spoken = Vec::new();
        for _ in 0..4 {
            ready_to_act(&mut inner);
            inner.scripting.resume_at = Instant::now();
            pump_script(&mut inner, Instant::now());
            spoken.push(inner.outbound.len());
        }
        let said = |words: &str| {
            let wide: Vec<u8> = words.encode_utf16().flat_map(u16::to_be_bytes).collect();
            inner
                .outbound
                .iter()
                .filter(|p| p.windows(wide.len()).any(|w| w == wide.as_slice()))
                .count()
        };
        assert_eq!(said("heal"), 2, "{spoken:?}");
        assert_eq!(said("work"), 2, "each tick the other goes first");
        let one = script_status(&inner, &json!({ "slot": "task" })).result;
        assert_eq!(one["status"], "running");
        assert!(stop_script(&mut inner, &json!({ "slot": "task" })).ok);
        assert_eq!(running_slots(&inner), vec!["healer"]);
        assert!(!stop_script(&mut inner, &json!({ "slot": "task" })).ok);
        let ended = script_status(&inner, &json!({ "slot": "task" })).result;
        assert_eq!(ended["status"], "stopped");
    }

    #[test]
    fn a_bounded_run_ends_after_its_iterations_or_its_time() {
        let mut inner = player();
        assert!(
            run_script(
                &mut inner,
                &json!({ "text": "msg 'once more'", "iterations": 2 })
            )
            .ok
        );
        tick(&mut inner, 4);
        let ended = status(&inner);
        assert_eq!(ended["status"], "done");
        assert_eq!(ended["ended_by"], ENDED_ITERATIONS);
        assert!(run_script(&mut inner, &json!({ "text": "pause 60000", "for": 0.001 })).ok);
        std::thread::sleep(Duration::from_millis(5));
        tick(&mut inner, 1);
        assert_eq!(status(&inner)["ended_by"], ENDED_TIME_UP);
    }

    #[test]
    fn one_script_holds_another_and_lets_it_go() {
        let mut inner = player();
        assert!(
            run_script(
                &mut inner,
                &json!({ "text": "pause 60000", "slot": "gather" })
            )
            .ok
        );
        start(&mut inner, "script 'suspend' 'gather'");
        tick(&mut inner, 1);
        let held = script_status(&inner, &json!({ "slot": "gather" })).result;
        assert_eq!(held["status"], "suspended");
        start(&mut inner, "script 'resume' 'gather'");
        tick(&mut inner, 1);
        let going = script_status(&inner, &json!({ "slot": "gather" })).result;
        assert_eq!(going["status"], "running");
    }

    #[test]
    fn a_looping_script_starts_again_at_its_end() {
        let mut inner = player();
        let started = run_script(&mut inner, &json!({ "text": "msg 'again'", "loop": true }));
        assert!(started.ok);
        tick(&mut inner, 3);
        let said = inner
            .outbound
            .iter()
            .filter(|p| p.first() == Some(&PKT_UNICODE_SPEECH))
            .count();
        assert_eq!(said, 3, "one pass a tick");
        assert_eq!(status(&inner)["status"], "running");
    }

    fn forbid(inner: &Inner, feature: AssistFeature) {
        inner.world.write().assist = uoterm_world::AssistRules::from_bits(1 << feature as u32);
    }

    #[test]
    fn a_shard_that_forbids_scripts_or_loops_gets_neither() {
        let mut inner = player();
        forbid(&inner, AssistFeature::ScriptMacros);
        assert!(!run_script(&mut inner, &json!({ "text": "pause 1" })).ok);
        let mut inner = player();
        forbid(&inner, AssistFeature::LoopedMacros);
        assert!(!run_script(&mut inner, &json!({ "text": "pause 1", "loop": true })).ok);
        assert!(!run_script(&mut inner, &json!({ "text": "pause 1\nreplay" })).ok);
        assert!(run_script(&mut inner, &json!({ "text": "pause 1" })).ok);
    }

    #[test]
    fn a_shard_that_forbids_potion_keys_gets_no_potion_drunk() {
        let mut inner = player();
        put_in_pack(&mut inner, POTION, GRAPHIC_POTION_HEAL);
        forbid(&inner, AssistFeature::PotionHotkeys);
        start(&mut inner, "drinkpotion 'heal'");
        tick(&mut inner, 1);
        assert!(!sent(&inner, &encode::double_click(POTION)));
        assert_eq!(status(&inner)["status"], "failed");
    }

    #[test]
    fn a_text_dialog_is_answered_and_then_closed() {
        let mut inner = player();
        let dialog = uoterm_protocol::TextEntryDialog {
            serial: Serial(0x0000_1234),
            parent: 1,
            button: 2,
            text: String::new(),
            can_cancel: true,
            style: 1,
            max_len: 20,
            description: "Name?".into(),
        };
        inner.world.write().text_entry = Some(dialog.clone());
        start(&mut inner, "textentrymsg 'Rowan'");
        tick(&mut inner, 1);
        assert!(sent(
            &inner,
            &encode::text_entry_response(&dialog, "Rowan", true)
        ));
        assert!(inner.world.read().text_entry.is_none());
    }

    #[test]
    fn a_prompt_answer_longer_than_the_shard_takes_is_refused() {
        let mut inner = player();
        inner.world.write().prompt = Some(uoterm_protocol::PromptRequest {
            serial: Serial(1),
            id: 1,
            unicode: false,
        });
        let long = "x".repeat(129);
        start(&mut inner, &format!("promptmsg '{long}'"));
        tick(&mut inner, 1);
        assert_eq!(status(&inner)["status"], "failed");
        assert!(
            inner.world.read().prompt.is_some(),
            "the prompt is still open"
        );
    }

    #[test]
    fn death_ends_a_script() {
        let mut inner = player();
        start(&mut inner, "pause 60000");
        inner.world.write().self_state.dead = true;
        tick(&mut inner, 1);
        assert_eq!(status(&inner)["status"], "stopped");
    }

    #[test]
    fn only_a_file_with_the_script_extension_is_a_script() {
        const SCRIPT: &str = "sysmsg 'heal'";
        let dir = std::env::temp_dir().join(format!("uoterm-scripts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("the test scripts folder is made");
        std::fs::write(dir.join("heal.TXT"), SCRIPT).expect("a script is written");
        std::fs::write(dir.join("notes.md"), SCRIPT).expect("a note is written");
        std::fs::write(dir.join("bank.json"), SCRIPT).expect("a data file is written");
        let dirs = [dir.clone()];
        let heal = find_script_in(&dirs, "heal");
        let notes = find_script_in(&dirs, "notes");
        let names: Vec<String> = script_files(&dirs).into_iter().map(|(n, _)| n).collect();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(heal.as_deref(), Some(SCRIPT));
        assert_eq!(notes, None);
        assert_eq!(names, vec!["heal"]);
    }

    #[test]
    fn a_context_menu_number_picks_an_entry_by_place_or_by_client_text() {
        const CHEST: Serial = Serial(0x4000_0D01);
        const OPEN_BANKBOX: u32 = 3_000_489;
        const THIRD_ENTRY: u16 = 3;
        for (line, choice) in [
            (
                "contextmenu 0x40000D01 3000489",
                MenuChoice::Cliloc(OPEN_BANKBOX),
            ),
            ("contextmenu 0x40000D01 3", MenuChoice::Index(THIRD_ENTRY)),
            (
                "contextmenu 0x40000D01 'Open'",
                MenuChoice::Text("Open".into()),
            ),
        ] {
            let mut inner = player();
            start(&mut inner, line);
            tick(&mut inner, 1);
            assert_eq!(inner.pending_context_menu, Some((CHEST, choice)), "{line}");
        }
    }

    #[test]
    fn a_script_that_does_not_parse_does_not_start() {
        let mut inner = player();
        let result = run_script(&mut inner, &json!({ "text": "if dead" }));
        assert!(!result.ok);
        assert!(result.error.unwrap_or_default().contains("line 1"));
    }

    /// The script guide the user reads.
    const GUIDE: &str = include_str!("../../../../docs/SCRIPTS.md");

    /// The first-column code of each table row in one part of the guide.
    fn guide_names(from: &str, to: &str) -> Vec<String> {
        let start = GUIDE.find(from).expect("the part starts");
        let end = GUIDE[start..].find(to).map_or(GUIDE.len(), |e| start + e);
        GUIDE[start..end]
            .lines()
            .filter_map(|line| line.strip_prefix("| `"))
            .flat_map(|cell| {
                let first = cell.split(" |").next().unwrap_or("");
                first
                    .split('`')
                    .step_by(2)
                    .filter_map(|code| code.split_whitespace().next())
                    .map(|w| w.trim_start_matches('@').trim_end_matches('!').to_string())
                    .collect::<Vec<_>>()
            })
            .filter(|w| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphabetic()))
            .collect()
    }

    #[test]
    fn every_example_in_the_guide_reads() {
        for block in GUIDE.split("```text").skip(1) {
            let code = block.split("```").next().unwrap_or("");
            assert!(Program::parse(code).is_ok(), "{code}");
        }
    }

    #[test]
    fn every_command_the_guide_names_is_known() {
        let names = guide_names("## Commands", "## Condition words");
        assert!(
            names.len() > 100,
            "the guide lists the commands: {}",
            names.len()
        );
        for name in names {
            let mut inner = player();
            let mut vars = Vars::default();
            let mut script = Script::new(Program::parse(&name).expect("one word"));
            script.tick(&mut Game { inner: &mut inner }, &mut vars, Instant::now());
            if let Status::Failed { message, .. } = script.status() {
                assert!(!message.starts_with("unknown command"), "{name}: {message}");
            }
        }
    }

    /// The command code the guide is checked against.
    const COMMANDS: &str = include_str!("scripting/commands.rs");

    /// Every command the code knows is in the guide, so a script writer can
    /// find each one.
    #[test]
    fn every_command_the_code_knows_is_in_the_guide() {
        let start = COMMANDS
            .find("pub(super) fn run(")
            .expect("the command match");
        let end = COMMANDS[start..]
            .find("unknown command")
            .map_or(COMMANDS.len(), |e| start + e);
        let known: Vec<&str> = COMMANDS[start..end]
            .lines()
            .map(str::trim_start)
            .filter(|line| line.starts_with('"') && line.contains(" =>"))
            .filter_map(|line| line.split(" =>").next())
            .flat_map(|arm| arm.split('|'))
            .map(|name| name.trim().trim_matches('"'))
            .filter(|name| !name.is_empty() && name.chars().all(|c| c.is_ascii_alphabetic()))
            .collect();
        assert!(
            known.len() > 100,
            "the code lists the commands: {}",
            known.len()
        );
        for name in known {
            assert!(
                GUIDE.contains(&format!("`{name}")) || GUIDE.contains(&format!(" {name}`")),
                "{name} is not in docs/SCRIPTS.md"
            );
        }
    }

    #[test]
    fn every_word_the_guide_names_is_known() {
        let words = guide_names("## Condition words", "## Running scripts");
        assert!(
            words.len() > 60,
            "the guide lists the words: {}",
            words.len()
        );
        for word in words {
            let mut inner = player();
            let mut vars = Vars::default();
            let text = format!("if {word}\nendif");
            let mut script = Script::new(Program::parse(&text).expect("a check"));
            script.tick(&mut Game { inner: &mut inner }, &mut vars, Instant::now());
            if let Status::Failed { message, .. } = script.status() {
                assert!(!message.starts_with("unknown word"), "{word}: {message}");
            }
        }
    }

    #[test]
    fn a_new_script_starts_with_no_output() {
        let mut inner = player();
        start(&mut inner, "sysmsg 'old'");
        tick(&mut inner, 1);
        start(&mut inner, "sysmsg 'new'");
        tick(&mut inner, 1);
        assert_eq!(status(&inner)["output"], json!(["new"]));
    }

    #[test]
    fn the_output_of_a_script_is_kept_for_its_status() {
        let mut inner = player();
        start(&mut inner, "sysmsg 'hello'\nwhere");
        tick(&mut inner, 1);
        let out = status(&inner)["output"].clone();
        assert_eq!(out[0], "hello");
    }
}
