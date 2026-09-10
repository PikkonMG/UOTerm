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
/// How many lines of script output the status keeps.
const OUTPUT_KEPT: usize = 50;

/// A script's argument names for the script tools.
const ARG_NAME: &str = "name";
const ARG_TEXT: &str = "text";
const ARG_LOOP: &str = "loop";

const A_SCRIPT_RUNS: &str = "a script is running; stop it first";
const NO_SCRIPT_NAMED: &str = "no script by that name in the scripts folder";
const RUN_NEEDS_SCRIPT: &str = "run_script needs name or text";

/// What the session keeps for scripts.
pub(super) struct Scripting {
    /// Aliases, lists and timers every script of the character shares.
    vars: Vars,
    running: Option<Running>,
    /// How the last script ended, for `script_status` after it is gone.
    last: Option<Report>,
    /// Lines scripts showed their user, newest last.
    output: VecDeque<String>,
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
    /// Lines to show later, each with the time it is due.
    later: Vec<(Instant, String)>,
    /// What the running script asked for after this tick: another script in
    /// its place, or to stop.
    next: Option<Next>,
    /// The name of the script running now.
    current: Option<String>,
}

/// What a script asks the session to do once its tick is over.
enum Next {
    Run(String),
    Stop,
}

struct Running {
    name: String,
    script: Script,
    /// The script as read, to start it again when it loops.
    program: Program,
    /// Start again from the top each time it reaches its end.
    looping: bool,
}

#[derive(Clone, Debug)]
struct Report {
    name: String,
    status: Status,
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
            running: None,
            last: None,
            output: VecDeque::new(),
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
            later: Vec::new(),
            next: None,
            current: None,
        }
    }

    fn keep_output(&mut self, lines: Vec<String>) {
        for line in lines {
            tracing::info!(line = %line, "script says");
            self.output.push_back(line);
        }
        while self.output.len() > OUTPUT_KEPT {
            self.output.pop_front();
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

/// The text of the script with this name. Any file whose name without its
/// extension matches, in any case, is the script.
fn find_script(name: &str) -> Option<String> {
    for dir in script_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if path.is_file() && stem.eq_ignore_ascii_case(name) {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    return Some(text);
                }
            }
        }
    }
    None
}

/// The names of the scripts in the scripts folders.
pub(super) fn script_names() -> Vec<String> {
    let mut names: Vec<String> = script_dirs()
        .into_iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flat_map(|entries| entries.flatten())
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter_map(|path| path.file_stem().and_then(|s| s.to_str()).map(String::from))
        .collect();
    names.sort_unstable_by_key(|n| n.to_ascii_lowercase());
    names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    names
}

/// Starts a script by name or from text. One script runs at a time.
pub(super) fn run_script(inner: &mut Inner, args: &Value) -> ToolResult {
    if inner.scripting.running.is_some() {
        return ToolResult::err(A_SCRIPT_RUNS);
    }
    if !shard_allows(inner, AssistFeature::ScriptMacros) {
        return ToolResult::err(forbidden(AssistFeature::ScriptMacros));
    }
    let name = args.get(ARG_NAME).and_then(|v| v.as_str()).map(str::trim);
    let text = args.get(ARG_TEXT).and_then(|v| v.as_str());
    let (name, source) = match (name, text) {
        (_, Some(text)) => (name.unwrap_or("text").to_string(), text.to_string()),
        (Some(name), None) if !name.is_empty() => match find_script(name) {
            Some(source) => (name.to_string(), source),
            None => return ToolResult::err(NO_SCRIPT_NAMED),
        },
        _ => return ToolResult::err(RUN_NEEDS_SCRIPT),
    };
    let looping = args
        .get(ARG_LOOP)
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    start_script(inner, name, &source, looping)
}

fn start_script(inner: &mut Inner, name: String, source: &str, looping: bool) -> ToolResult {
    let program = match Program::parse(source) {
        Ok(program) => program,
        Err(e) => return ToolResult::err(e.to_string()),
    };
    // A script that starts itself again is a looping macro.
    let loops = looping || program.ops.contains(&uoterm_script::Op::Replay);
    if loops && !shard_allows(inner, AssistFeature::LoopedMacros) {
        return ToolResult::err(forbidden(AssistFeature::LoopedMacros));
    }
    tracing::info!(script = %name, looping, "script starts");
    inner.scripting.running = Some(Running {
        script: Script::new(program.clone()),
        program,
        name: name.clone(),
        looping,
    });
    ToolResult::ok(json!({ "script": name, "loop": looping }))
}

/// The name of the script running now.
pub(super) fn running_name(inner: &Inner) -> Option<&str> {
    inner.scripting.running.as_ref().map(|r| r.name.as_str())
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

pub(super) fn stop_script(inner: &mut Inner) -> ToolResult {
    let Some(mut running) = inner.scripting.running.take() else {
        return ToolResult::err("no script is running");
    };
    running.script.stop();
    finish(inner, running);
    ToolResult::ok(json!({ "stopped": true }))
}

pub(super) fn script_status(inner: &Inner) -> ToolResult {
    let s = &inner.scripting;
    let output: Vec<&String> = s.output.iter().collect();
    let body = match (&s.running, &s.last) {
        (Some(running), _) => json!({
            "script": running.name,
            "status": "running",
            "loop": running.looping,
            "line": running.script.line(),
            "output": output,
        }),
        (None, Some(report)) => {
            let mut body = status_json(&report.status);
            body["script"] = json!(report.name);
            body["output"] = json!(output);
            body
        }
        (None, None) => json!({ "status": "none", "output": output }),
    };
    ToolResult::ok(body)
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

fn finish(inner: &mut Inner, mut running: Running) {
    let lines = running.script.take_output();
    inner.scripting.keep_output(lines);
    let status = running.script.status().clone();
    tracing::info!(script = %running.name, status = ?status, "script ends");
    inner.scripting.last = Some(Report {
        name: running.name,
        status,
    });
}

/// Runs the script a tick's worth. Death and a lost login end it.
pub(super) fn pump_script(inner: &mut Inner, now: Instant) {
    show_due_lines(inner, now);
    let Some(mut running) = inner.scripting.running.take() else {
        return;
    };
    let dead = {
        let world = inner.world.read();
        !world.logged_in || world.self_state.dead
    };
    if dead {
        running.script.stop();
        finish(inner, running);
        return;
    }
    let mut vars = std::mem::take(&mut inner.scripting.vars);
    inner.scripting.current = Some(running.name.clone());
    running.script.tick(&mut Game { inner }, &mut vars, now);
    inner.scripting.vars = vars;
    let lines = running.script.take_output();
    inner.scripting.keep_output(lines);
    match inner.scripting.next.take() {
        Some(Next::Run(name)) => {
            running.script.stop();
            finish(inner, running);
            match find_script(&name) {
                Some(source) => {
                    let result = start_script(inner, name, &source, false);
                    if let Some(error) = result.error {
                        inner.scripting.keep_output(vec![error]);
                    }
                }
                None => inner
                    .scripting
                    .keep_output(vec![format!("{name}: {NO_SCRIPT_NAMED}")]),
            }
        }
        Some(Next::Stop) => {
            running.script.stop();
            finish(inner, running);
        }
        None if *running.script.status() == Status::Running => {
            inner.scripting.running = Some(running);
        }
        // A looping script starts again from the top at the next tick.
        None if running.looping && *running.script.status() == Status::Done => {
            running.script = Script::new(running.program.clone());
            inner.scripting.running = Some(running);
        }
        None => finish(inner, running),
    }
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
        let world = self.world();
        match name.to_ascii_lowercase().as_str() {
            "self" => Some(world.self_state.serial),
            "backpack" => backpack_serial(&world),
            "bank" => world.bank_box(),
            "last" | "lasttarget" => last_target_for_cursor(self.inner),
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
            .filter(|i| i.graphic == graphic && color.map_or(true, |c| i.hue == c))
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
            .filter(|m| m.body == body && color.map_or(true, |c| m.hue == c))
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
        script_status(inner).result
    }

    #[test]
    fn a_spell_is_cast_by_name_on_a_target() {
        let mut inner = player();
        start(&mut inner, "cast 'Greater Heal' 'self'");
        tick(&mut inner, 1);
        assert!(sent(&inner, &encode::cast_spell(GREATER_HEAL)));
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
        assert!(sent(&inner, &encode::cast_spell(CURE)));
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
        assert!(stop_script(&mut inner).ok);
        assert_eq!(status(&inner)["status"], "stopped");
        assert!(run_script(&mut inner, &json!({ "text": "pause 1" })).ok);
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
    fn the_output_of_a_script_is_kept_for_its_status() {
        let mut inner = player();
        start(&mut inner, "sysmsg 'hello'\nwhere");
        tick(&mut inner, 1);
        let out = status(&inner)["output"].clone();
        assert_eq!(out[0], "hello");
    }
}
