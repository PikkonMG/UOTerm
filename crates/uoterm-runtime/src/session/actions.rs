//! The one-click acts a player has on his screen and an agent needs as
//! tools: skill and stat locks, a pet's name, a weapon move, a bow, flying,
//! the quest and guild buttons, a harvest tool aimed at a resource, an item
//! used by its type or on a mobile, the virtues, mounting, the nearest foe,
//! the catch bag and the ignore lists, and a record of the skills gained.
//!
//! An act a script command already does is run as that command, so a tool
//! and a script line do the same thing the same way, and a recording writes
//! the line down.

use std::collections::HashMap;

use uoterm_nav::TileFlagSet;

use super::recorder::quoted;
use super::*;

const ARG_NAME: &str = "name";
const ARG_SKILL: &str = "skill";
const ARG_LOCK: &str = "lock";
const ARG_STAT: &str = "stat";
const ARG_ABILITY: &str = "ability";
const ARG_ACTION: &str = "action";
const ARG_WHICH: &str = "which";
const ARG_TOOL: &str = "tool";
const ARG_RESOURCE: &str = "resource";
const ARG_GRAPHIC: &str = "graphic";
const ARG_SOURCE: &str = "source";
const ARG_RANGE: &str = "range";
const ARG_ITEM: &str = "item";
const ARG_LIST: &str = "list";
const ARG_VALUE: &str = "value";
const ARG_CLEAR: &str = "clear";
const ARG_DISTANCE: &str = "distance";

/// The script word for an item of any colour, and for the backpack.
const ANY_HUE: &str = "any";
const SOURCE_DEFAULT: &str = "backpack";
const LIST_GUMPS: &str = "gumps";
const LIST_JOURNAL: &str = "journal";
const DO_ADD: &str = "add";
const DO_REMOVE: &str = "remove";
const DO_CLEAR: &str = "clear";
const DO_SHOW: &str = "show";
const BUTTON_QUESTS: &str = "quests";
const BUTTON_GUILD: &str = "guild";

/// The gump the virtues are shown and invoked through. The shard reads a
/// press of each virtue by the number of its picture.
const VIRTUE_GUMP_ID: u32 = 0x0000_01CD;
/// The button that asks the shard to show a mobile's virtues.
const VIRTUE_GUMP_OPEN: u32 = 1;
/// Each virtue by name, the picture the virtue gump shows it by, and the
/// macro number of the three a macro invokes. The shard answers a virtue it
/// has no power for with "that virtue is not active yet".
pub(super) const VIRTUES: [(&str, u32, Option<u8>); 8] = [
    ("humility", 108, None),
    ("sacrifice", 110, Some(2)),
    ("compassion", 105, None),
    ("spirituality", 111, None),
    ("valor", 112, Some(3)),
    ("honor", 107, Some(1)),
    ("justice", 109, None),
    ("honesty", 106, None),
];

/// How many skill changes the record keeps, the newest last.
const GAINS_KEPT: usize = 200;
/// Skill values are in tenths of a point.
const TENTHS: f64 = 10.0;
const SECONDS_PER_HOUR: f64 = 3600.0;
/// A rate over less time than this says nothing yet.
const RATE_MIN_SECONDS: f64 = 60.0;

const NEEDS_TEXT: &str = "needs text";
const NO_SUCH_VIRTUE: &str =
    "virtue must be humility, sacrifice, compassion, spirituality, valor, honor, justice or honesty";
const BAD_BUTTON: &str = "which must be quests or guild";
const BAD_LIST: &str = "list must be gumps or journal";
const BAD_LIST_ACTION: &str = "action must be add, remove, clear or show";
const NEEDS_VALUE: &str = "needs value: a gump id for gumps, words for journal";
const NO_MOUNT: &str =
    "no mount: name one (serial), set the remount agent's mount, or stand near a pet you can ride";
const MOUNTED_ALREADY: &str = "the character rides already";
const NOT_MOUNTED: &str = "the character is not riding";
const FIGHTING_NO_MOUNT: &str =
    "war mode stays on in a fight, and a double-click in war mode attacks; end the fight first";
const NO_FOE: &str = "no foe in sight to attack";
const NOT_A_CONTAINER: &str = "the catch bag must be a container the character can see";
const NO_SUCH_SKILL: &str = "skill must be a skill number or name";

/// Runs one script command as this tool, and names the line it ran.
fn as_script(inner: &mut Inner, tool: &str, line: String) -> ToolResult {
    let done = scripting::run_now(inner, tool, &line);
    if !done.ok {
        return done;
    }
    let mut acted = ToolResult::action(tool);
    acted.result["output"] = done.result["output"].clone();
    acted.result["lines"] = json!(line);
    acted
}

/// A text argument quoted for a script line.
fn quoted_arg(args: &Value, key: &str) -> std::result::Result<String, Box<ToolResult>> {
    let text = args
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .ok_or_else(|| Box::new(ToolResult::err(format!("{key} {NEEDS_TEXT}"))))?;
    quoted(text).map_err(|_| {
        Box::new(ToolResult::err(format!(
            "{key} holds both quote marks or a line break"
        )))
    })
}

/// A serial argument as a script writes it.
fn serial_arg(args: &Value, key: &str, tool: &str) -> std::result::Result<String, Box<ToolResult>> {
    let serial = arg_serial(args, key);
    if serial.is_valid() {
        Ok(serial.0.to_string())
    } else {
        Err(Box::new(ToolResult::err(format!("{tool} needs {key}"))))
    }
}

macro_rules! take {
    ($e:expr) => {
        match $e {
            Ok(value) => value,
            Err(refused) => return *refused,
        }
    };
}

/// `skill_lock`: sets a skill's lock: up, down or locked.
pub(super) fn skill_lock(inner: &mut Inner, args: &Value) -> ToolResult {
    let skill = take!(quoted_arg(args, ARG_SKILL));
    let lock = take!(quoted_arg(args, ARG_LOCK));
    as_script(inner, TOOL_SKILL_LOCK, format!("setskill {skill} {lock}"))
}

/// `stat_lock`: sets a stat's lock: str, dex or int; up, down or locked.
pub(super) fn stat_lock(inner: &mut Inner, args: &Value) -> ToolResult {
    let stat = take!(quoted_arg(args, ARG_STAT));
    let lock = take!(quoted_arg(args, ARG_LOCK));
    as_script(inner, TOOL_STAT_LOCK, format!("setstatlock {stat} {lock}"))
}

/// `rename`: gives a pet a new name.
pub(super) fn rename(inner: &mut Inner, args: &Value) -> ToolResult {
    let pet = take!(serial_arg(args, ARG_SERIAL, TOOL_RENAME));
    let name = take!(quoted_arg(args, ARG_NAME));
    as_script(inner, TOOL_RENAME, format!("rename {pet} {name}"))
}

/// `set_ability`: arms or clears a weapon move: primary, secondary, stun or
/// disarm.
pub(super) fn set_ability(inner: &mut Inner, args: &Value) -> ToolResult {
    let ability = take!(quoted_arg(args, ARG_ABILITY));
    let on = if args.get(ARG_ON).and_then(Value::as_bool).unwrap_or(true) {
        "on"
    } else {
        "off"
    };
    as_script(
        inner,
        TOOL_SET_ABILITY,
        format!("setability {ability} '{on}'"),
    )
}

/// `emote_action`: plays a body action such as bow or salute.
pub(super) fn emote_action(inner: &mut Inner, args: &Value) -> ToolResult {
    let action = take!(quoted_arg(args, ARG_ACTION));
    as_script(inner, TOOL_EMOTE_ACTION, format!("emoteaction {action}"))
}

/// `fly`: a gargoyle takes off, or lands with on false.
pub(super) fn fly(inner: &mut Inner, args: &Value) -> ToolResult {
    let up = args.get(ARG_ON).and_then(Value::as_bool).unwrap_or(true);
    as_script(inner, TOOL_FLY, if up { "fly" } else { "land" }.into())
}

/// `menu_button`: presses the quests or the guild button of the paperdoll.
pub(super) fn menu_button(inner: &mut Inner, args: &Value) -> ToolResult {
    let which = args.get(ARG_WHICH).and_then(Value::as_str).map(str::trim);
    let command = match which {
        Some(w) if w.eq_ignore_ascii_case(BUTTON_QUESTS) => "questsbutton",
        Some(w) if w.eq_ignore_ascii_case(BUTTON_GUILD) => "guildbutton",
        _ => return ToolResult::err(BAD_BUTTON),
    };
    as_script(inner, TOOL_MENU_BUTTON, command.into())
}

/// `target_resource`: aims a harvest tool at a resource with no cursor.
pub(super) fn target_resource(inner: &mut Inner, args: &Value) -> ToolResult {
    let tool = take!(serial_arg(args, ARG_TOOL, TOOL_TARGET_RESOURCE));
    let resource = take!(quoted_arg(args, ARG_RESOURCE));
    as_script(
        inner,
        TOOL_TARGET_RESOURCE,
        format!("targetresource {tool} {resource}"),
    )
}

/// `use_type`: double-clicks the first item of a graphic, of a colour, in
/// a place: the backpack, the ground, the world or a container.
pub(super) fn use_type(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(graphic) = arg_number(args, ARG_GRAPHIC) else {
        return ToolResult::err(format!("{TOOL_USE_TYPE} needs graphic"));
    };
    let hue = arg_number(args, ARG_HUE).map_or(ANY_HUE.to_string(), |h| h.to_string());
    // A container by its serial, or a place by its word.
    let source = match arg_number(args, ARG_SOURCE) {
        Some(container) => container.to_string(),
        None if args.get(ARG_SOURCE).is_some() => take!(quoted_arg(args, ARG_SOURCE)),
        None => format!("'{SOURCE_DEFAULT}'"),
    };
    let mut line = format!("usetype {graphic} {hue} {source}");
    if let Some(range) = arg_number(args, ARG_RANGE) {
        line.push_str(&format!(" {range}"));
    }
    as_script(inner, TOOL_USE_TYPE, line)
}

/// `use_on`: uses an item on a mobile with no cursor, as a bandage is: a
/// bandage, a potion of a pet's, any item the shard lets be used so.
pub(super) fn use_on(inner: &mut Inner, args: &Value) -> ToolResult {
    let item = arg_serial(args, ARG_ITEM);
    let target = arg_serial(args, ARG_TARGET);
    if !item.is_valid() || !target.is_valid() {
        return ToolResult::err(format!("{TOOL_USE_ON} needs item and target"));
    }
    if !action_ready(inner) {
        return ToolResult::err(MUST_WAIT);
    }
    inner
        .outbound
        .push_back(encode::bandage_target(item, target));
    mark_action(inner);
    inner.last_object = Some(item);
    ToolResult::action(TOOL_USE_ON)
}

/// The picture and the macro number of a virtue, by its name.
pub(super) fn virtue_named(name: &str) -> Option<(u32, Option<u8>)> {
    VIRTUES
        .iter()
        .find(|(known, _, _)| known.eq_ignore_ascii_case(name.trim()))
        .map(|(_, picture, number)| (*picture, *number))
}

/// Invokes a virtue: with the macro for the three a macro invokes, which
/// every shard family reads, and else as a press on its picture in the
/// virtue gump.
pub(super) fn invoke_virtue(inner: &mut Inner, name: &str) -> std::result::Result<(), String> {
    let (picture, number) = virtue_named(name).ok_or(NO_SUCH_VIRTUE)?;
    let packet = match number {
        Some(number) => encode::invoke_virtue(number),
        None => {
            let me = inner.world.read().self_state.serial;
            encode::gump_response(me, VIRTUE_GUMP_ID, picture, &[], &[])
        }
    };
    inner.outbound.push_back(packet);
    Ok(())
}

/// `virtue`: invokes one of the eight virtues.
pub(super) fn virtue(inner: &mut Inner, args: &Value) -> ToolResult {
    let name = args
        .get(ARG_NAME)
        .and_then(Value::as_str)
        .unwrap_or_default();
    match invoke_virtue(inner, name) {
        Ok(()) => ToolResult::action(TOOL_VIRTUE),
        Err(why) => ToolResult::err(why),
    }
}

/// `virtue_gump`: asks the shard for the virtue gump of a mobile, the
/// character's own by default.
pub(super) fn virtue_gump(inner: &mut Inner, args: &Value) -> ToolResult {
    let me = inner.world.read().self_state.serial;
    let of = arg_serial_opt(args, ARG_SERIAL)
        .filter(|s| s.is_valid())
        .unwrap_or(me);
    inner.outbound.push_back(encode::gump_response(
        me,
        VIRTUE_GUMP_ID,
        VIRTUE_GUMP_OPEN,
        &[of.0],
        &[],
    ));
    ToolResult::action(TOOL_VIRTUE_GUMP)
}

/// The container loot and moves go into: the catch bag when it is set and
/// the character can still see it, else the backpack.
pub(super) fn catch_bag_or(inner: &Inner, pack: Serial) -> Serial {
    let world = inner.world.read();
    inner
        .agents
        .config
        .options
        .catch_bag
        .filter(|bag| world.items.contains_key(bag))
        .unwrap_or(pack)
}

/// `catch_bag`: sets the container loot and moves go into (serial), or
/// clears it (clear true). With neither, says which it is.
pub(super) fn catch_bag(inner: &mut Inner, args: &Value) -> ToolResult {
    let clear = args.get(ARG_CLEAR).and_then(Value::as_bool) == Some(true);
    let asked = arg_serial_opt(args, ARG_SERIAL).filter(|s| s.is_valid());
    if clear || asked.is_some() {
        if let Some(bag) = asked {
            let world = inner.world.read();
            let container = world.containers.contains_key(&bag)
                || world.items.get(&bag).is_some_and(|item| {
                    inner
                        .map_files()
                        .item_stat(item.graphic)
                        .is_some_and(|(flags, _)| flags & TileFlagSet::CONTAINER.low_bits() != 0)
                });
            if !container {
                return ToolResult::err(NOT_A_CONTAINER);
            }
        }
        inner.agents.config.options.catch_bag = asked;
        if let Err(e) = inner.agents.save() {
            return ToolResult::err(e);
        }
    }
    ToolResult::ok(json!({ "catch_bag": inner.agents.config.options.catch_bag }))
}

/// The pet to ride: the one named, the remount agent's, or the nearest pet
/// of the character's that can be ridden.
fn mount_to_ride(inner: &Inner, args: &Value) -> Option<Serial> {
    if let Some(named) = arg_serial_opt(args, ARG_SERIAL).filter(|s| s.is_valid()) {
        return Some(named);
    }
    if let Some(set) = inner.agents.config.remount.mount {
        return Some(set);
    }
    let world = inner.world.read();
    let here = world.self_state.location;
    world
        .mobiles
        .values()
        .filter(|m| world.is_renamable(m.serial) && uoterm_nav::is_mount_body(m.body))
        .min_by_key(|m| here.chebyshev(m.location))
        .map(|m| m.serial)
}

/// Takes war mode off before a double-click that must not be an attack.
/// False when a fight keeps it on.
fn peace_first(inner: &mut Inner) -> bool {
    let (war, fighting) = {
        let world = inner.world.read();
        (world.self_state.war, world.fighting())
    };
    if !war {
        return true;
    }
    if fighting {
        return false;
    }
    send_war_mode(inner, false);
    true
}

/// `mount`: rides a pet or an ethereal mount, with war mode off first so
/// the double-click is no attack.
pub(super) fn mount(inner: &mut Inner, args: &Value) -> ToolResult {
    if inner.world.read().worn(LAYER_MOUNT).is_some() {
        return ToolResult::err(MOUNTED_ALREADY);
    }
    let Some(ride) = mount_to_ride(inner, args) else {
        return ToolResult::err(NO_MOUNT);
    };
    if !action_ready(inner) {
        return ToolResult::err(MUST_WAIT);
    }
    if !peace_first(inner) {
        return ToolResult::err(FIGHTING_NO_MOUNT);
    }
    if !send_double_click(inner, ride) {
        return ToolResult::err(MUST_WAIT);
    }
    let mut acted = ToolResult::action(TOOL_MOUNT);
    acted.result["mount"] = json!(ride);
    acted
}

/// `dismount`: gets off the mount, with war mode off first.
pub(super) fn dismount(inner: &mut Inner) -> ToolResult {
    if inner.world.read().worn(LAYER_MOUNT).is_none() {
        return ToolResult::err(NOT_MOUNTED);
    }
    if !action_ready(inner) {
        return ToolResult::err(MUST_WAIT);
    }
    if !peace_first(inner) {
        return ToolResult::err(FIGHTING_NO_MOUNT);
    }
    let me = inner.world.read().self_state.serial;
    if !send_double_click(inner, me) {
        return ToolResult::err(MUST_WAIT);
    }
    ToolResult::action(TOOL_DISMOUNT)
}

/// `attack_nearest`: attacks the nearest mobile a shot or a spell can reach
/// that may be harmed without a crime: never an innocent, a friend, a pet
/// or party member of the character's, or one nobody can harm. notoriety,
/// species, name and distance narrow the choice.
pub(super) fn attack_nearest(inner: &mut Inner, args: &Value) -> ToolResult {
    if !shard_allows(inner, AssistFeature::ClosestTargets) {
        return ToolResult::err(forbidden(AssistFeature::ClosestTargets));
    }
    let ranks: Vec<u8> =
        args.get(ARG_NOTORIETY)
            .and_then(Value::as_str)
            .map_or(NOTO_ATTACKABLE.to_vec(), |words| {
                uoterm_assist::mobiles::notorieties(words)
                    .iter()
                    .copied()
                    .filter(|rank| NOTO_ATTACKABLE.contains(rank))
                    .collect()
            });
    let species = args
        .get(ARG_SPECIES)
        .and_then(Value::as_str)
        .map(str::to_lowercase);
    let name = args.get(ARG_NAME).and_then(Value::as_str);
    let within = arg_number(args, ARG_DISTANCE);
    let mode = inner.agents.config.options.sight_mode;
    let foe = {
        let tiles = inner.tiles();
        let world = inner.world.read();
        let here = world.self_state.location;
        let mut foes: Vec<&uoterm_world::Mobile> = world
            .mobiles
            .values()
            .filter(|m| m.serial != world.self_state.serial && ranks.contains(&m.notoriety))
            .filter(|m| !world.is_renamable(m.serial) && !world.party.contains(&m.serial))
            .filter(|m| !inner.agents.is_friend(&world, m.serial))
            .filter(|m| !uoterm_world::is_ghost_body(m.body))
            .filter(|m| name.is_none_or(|n| m.answers_to(n)))
            .filter(|m| {
                species
                    .as_ref()
                    .is_none_or(|wanted| crate::jobs::mobile_species(m).contains(wanted.as_str()))
            })
            .filter(|m| within.is_none_or(|w| here.chebyshev(m.location) <= w))
            .filter(|m| {
                uoterm_nav::sight_trace(
                    &tiles,
                    uoterm_nav::eyes_at(here),
                    uoterm_nav::eyes_at(m.location),
                    mode,
                )
                .in_sight
            })
            .collect();
        foes.sort_by_key(|m| (here.chebyshev(m.location), m.serial.0));
        foes.first().map(|m| (m.serial, m.name.clone()))
    };
    let Some((serial, name)) = foe else {
        return ToolResult::err(NO_FOE);
    };
    send_attack(inner, serial);
    inner.last_target = Some(serial);
    let mut acted = ToolResult::action(TOOL_ATTACK_NEAREST);
    acted.result["serial"] = json!(serial);
    acted.result["name"] = json!(name);
    acted
}

/// `ignore_list`: the gumps and the journal lines an agent does not hear
/// of. list gumps or journal; action add, remove, clear or show; value a
/// gump id or words.
pub(super) fn ignore_list(inner: &mut Inner, args: &Value) -> ToolResult {
    let list = args.get(ARG_LIST).and_then(Value::as_str).map(str::trim);
    let action = args
        .get(ARG_ACTION)
        .and_then(Value::as_str)
        .map_or(DO_SHOW, str::trim);
    let options = &mut inner.agents.config.options;
    let changed = match (list, action) {
        (_, a) if a.eq_ignore_ascii_case(DO_SHOW) => false,
        (Some(l), a) if l.eq_ignore_ascii_case(LIST_GUMPS) => {
            match (a.to_ascii_lowercase().as_str(), arg_number(args, ARG_VALUE)) {
                (DO_CLEAR, _) => options.ignore_gumps.clear(),
                (DO_ADD, Some(id)) if !options.ignore_gumps.contains(&id) => {
                    options.ignore_gumps.push(id)
                }
                (DO_ADD, Some(_)) => {}
                (DO_REMOVE, Some(id)) => options.ignore_gumps.retain(|kept| *kept != id),
                (DO_ADD | DO_REMOVE, None) => return ToolResult::err(NEEDS_VALUE),
                _ => return ToolResult::err(BAD_LIST_ACTION),
            }
            true
        }
        (Some(l), a) if l.eq_ignore_ascii_case(LIST_JOURNAL) => {
            let words = args
                .get(ARG_VALUE)
                .and_then(Value::as_str)
                .map(|w| w.trim().to_lowercase())
                .filter(|w| !w.is_empty());
            match (a.to_ascii_lowercase().as_str(), words) {
                (DO_CLEAR, _) => options.ignore_journal.clear(),
                (DO_ADD, Some(w)) if !options.ignore_journal.contains(&w) => {
                    options.ignore_journal.push(w)
                }
                (DO_ADD, Some(_)) => {}
                (DO_REMOVE, Some(w)) => options.ignore_journal.retain(|kept| *kept != w),
                (DO_ADD | DO_REMOVE, None) => return ToolResult::err(NEEDS_VALUE),
                _ => return ToolResult::err(BAD_LIST_ACTION),
            }
            true
        }
        _ => return ToolResult::err(BAD_LIST),
    };
    if changed {
        if let Err(e) = inner.agents.save() {
            return ToolResult::err(e);
        }
    }
    let options = &inner.agents.config.options;
    ToolResult::ok(json!({
        LIST_GUMPS: options.ignore_gumps,
        LIST_JOURNAL: options.ignore_journal,
    }))
}

/// True when an agent does not hear of this gump.
pub(super) fn gump_ignored(inner: &Inner, gump_id: u32) -> bool {
    inner.agents.config.options.ignore_gumps.contains(&gump_id)
}

/// True when an agent does not hear of this journal line: its speaker or
/// its words hold words of the ignore list.
pub(super) fn line_ignored(inner: &Inner, speaker: &str, text: &str) -> bool {
    let (speaker, text) = (speaker.to_lowercase(), text.to_lowercase());
    inner
        .agents
        .config
        .options
        .ignore_journal
        .iter()
        .any(|words| speaker.contains(words.as_str()) || text.contains(words.as_str()))
}

/// Hands the world the `listen_range` option, so a line said near the
/// character counts as said to him while the option is on.
pub(super) fn share_listen_range(inner: &Inner) {
    let range = inner.agents.config.options.listen_range;
    if inner.world.read().listen_range != range {
        inner.world.write().listen_range = range;
    }
}

/// One change of a skill's value.
#[derive(Clone, Copy, Debug)]
struct Gain {
    skill: u16,
    /// The new value and the change, in tenths.
    value: u16,
    delta: i32,
    unix_ms: u64,
}

/// The record of the skills the character gained: every change of a
/// skill's value since the session started, and when.
#[derive(Default)]
pub(super) struct SkillGains {
    /// The value of each skill the record last saw, in tenths.
    known: HashMap<u16, u16>,
    history: VecDeque<Gain>,
    /// When the record started, the first skill list.
    since_ms: Option<u64>,
}

/// Writes down every skill whose value moved since the last look. The
/// first value of a skill is where the record starts, not a gain.
pub(super) fn note_skills(inner: &mut Inner) {
    let world = inner.world.read();
    if world.self_state.skills.is_empty() {
        return;
    }
    let now = uoterm_world::unix_now_ms();
    let gains = &mut inner.gains;
    gains.since_ms.get_or_insert(now);
    for (&skill, value) in &world.self_state.skills {
        match gains.known.insert(skill, value.value) {
            Some(before) if before != value.value => {
                gains.history.push_back(Gain {
                    skill,
                    value: value.value,
                    delta: i32::from(value.value) - i32::from(before),
                    unix_ms: now,
                });
            }
            _ => {}
        }
    }
    while gains.history.len() > GAINS_KEPT {
        gains.history.pop_front();
    }
}

/// `skill_gains`: the skills gained this session, each with the points and
/// the gains an hour, and the newest changes. skill narrows it to one;
/// clear starts the record again.
pub(super) fn skill_gains(inner: &mut Inner, args: &Value) -> ToolResult {
    if args.get(ARG_CLEAR).and_then(Value::as_bool) == Some(true) {
        inner.gains.history.clear();
        inner.gains.since_ms = Some(uoterm_world::unix_now_ms());
    }
    let only = match args.get(ARG_SKILL) {
        None => None,
        Some(_) => match named_skill(inner, args) {
            Some(id) => Some(id),
            None => return ToolResult::err(NO_SUCH_SKILL),
        },
    };
    let now = uoterm_world::unix_now_ms();
    let gains = &inner.gains;
    let hours = gains
        .since_ms
        .map(|since| now.saturating_sub(since) as f64 / 1000.0)
        .filter(|seconds| *seconds >= RATE_MIN_SECONDS)
        .map(|seconds| seconds / SECONDS_PER_HOUR);
    let mut by_skill: Vec<(u16, i32, u32, u16)> = Vec::new();
    for gain in gains
        .history
        .iter()
        .filter(|g| only.is_none_or(|id| g.skill == id))
    {
        match by_skill.iter_mut().find(|(id, ..)| *id == gain.skill) {
            Some((_, total, count, value)) => {
                *total += gain.delta;
                *count += 1;
                *value = gain.value;
            }
            None => by_skill.push((gain.skill, gain.delta, 1, gain.value)),
        }
    }
    let name_of = |id: u16| {
        inner
            .scripting
            .skills
            .by_id(id)
            .map_or_else(|| id.to_string(), |s| s.name.clone())
    };
    let skills: Vec<Value> = by_skill
        .iter()
        .map(|(id, total, count, value)| {
            let points = f64::from(*total) / TENTHS;
            json!({
                "skill": id,
                "name": name_of(*id),
                "value": f64::from(*value) / TENTHS,
                "gained": points,
                "changes": count,
                "per_hour": hours.map(|h| points / h),
            })
        })
        .collect();
    let recent: Vec<Value> = gains
        .history
        .iter()
        .rev()
        .filter(|g| only.is_none_or(|id| g.skill == id))
        .take(PAGE_OF_GAINS)
        .map(|g| {
            json!({
                "skill": g.skill,
                "name": name_of(g.skill),
                "value": f64::from(g.value) / TENTHS,
                "change": f64::from(g.delta) / TENTHS,
                "unix_ms": g.unix_ms,
            })
        })
        .collect();
    ToolResult::ok(json!({
        "since_unix_ms": gains.since_ms,
        "skills": skills,
        "recent": recent,
    }))
}

/// How many of the newest skill changes `skill_gains` shows.
const PAGE_OF_GAINS: usize = 20;

#[cfg(test)]
mod tests {
    use super::super::relay_tests::{armed_session, ready_to_act};
    use super::*;

    const ME: Serial = Serial(0x0000_0001);
    const ORC: Serial = Serial(0x0000_0100);
    const TRAVELLER: Serial = Serial(0x0000_0101);
    const HORSE: Serial = Serial(0x0000_0102);
    const HORSE_BODY: u16 = 0x00CC;
    const ORC_BODY: u16 = 0x0011;
    const HUMAN_BODY: u16 = 0x0190;

    fn sent(inner: &Inner) -> Vec<Vec<u8>> {
        inner.outbound.iter().cloned().collect()
    }

    fn mobile(inner: &Inner, serial: Serial, body: u16, notoriety: u8, dx: u16) {
        let mut world = inner.world.write();
        let here = world.self_state.location;
        world.mobiles.insert(
            serial,
            uoterm_world::Mobile {
                serial,
                name: String::new(),
                title: String::new(),
                body,
                hue: 0,
                location: Point3::new(here.x + dx, here.y, here.z),
                direction: 0,
                running: false,
                notoriety,
                flags: 0,
                hits: None,
                hits_max: None,
                pools: Default::default(),
                equipment: Vec::new(),
            },
        );
    }

    #[test]
    fn every_virtue_is_invoked_the_way_a_shard_reads_it() {
        let mut inner = armed_session();
        inner.world.write().self_state.serial = ME;
        assert!(virtue(&mut inner, &json!({ "name": "Honor" })).ok);
        assert!(virtue(&mut inner, &json!({ "name": "compassion" })).ok);
        assert!(!virtue(&mut inner, &json!({ "name": "greed" })).ok);
        assert!(virtue_gump(&mut inner, &json!({})).ok);
        assert_eq!(
            sent(&inner),
            vec![
                encode::invoke_virtue(1),
                encode::gump_response(ME, VIRTUE_GUMP_ID, 105, &[], &[]),
                encode::gump_response(ME, VIRTUE_GUMP_ID, VIRTUE_GUMP_OPEN, &[ME.0], &[]),
            ]
        );
        assert_eq!(VIRTUES.len(), 8);
    }

    #[test]
    fn a_tool_of_a_script_command_runs_the_command_and_names_its_line() {
        let mut inner = armed_session();
        let locked = stat_lock(&mut inner, &json!({ "stat": "str", "lock": "locked" }));
        assert!(locked.ok, "{locked:?}");
        assert_eq!(locked.result["lines"], json!("setstatlock 'str' 'locked'"));
        assert_eq!(sent(&inner), vec![encode::stat_lock(0, SKILL_LOCK_LOCKED)]);
        assert!(!stat_lock(&mut inner, &json!({ "stat": "luck", "lock": "up" })).ok);
        assert!(!menu_button(&mut inner, &json!({ "which": "bank" })).ok);
        assert!(menu_button(&mut inner, &json!({ "which": "quests" })).ok);
        let bowed = emote_action(&mut inner, &json!({ "action": "bow" }));
        assert!(bowed.ok);
        assert_eq!(inner.outbound.back(), Some(&encode::emote_animation("bow")));
    }

    #[test]
    fn any_item_is_used_on_a_mobile_at_the_pace_of_an_action() {
        let mut inner = armed_session();
        ready_to_act(&mut inner);
        const POTION: Serial = Serial(0x4000_0333);
        assert!(use_on(&mut inner, &json!({ "item": POTION.0, "target": ORC.0 })).ok);
        assert_eq!(sent(&inner), vec![encode::bandage_target(POTION, ORC)]);
        assert!(!use_on(&mut inner, &json!({ "item": POTION.0, "target": ORC.0 })).ok);
        assert!(!use_on(&mut inner, &json!({ "item": POTION.0 })).ok);
    }

    #[test]
    fn the_nearest_foe_in_sight_is_attacked_and_an_innocent_never() {
        let mut inner = armed_session();
        mobile(&inner, TRAVELLER, HUMAN_BODY, NOTO_INNOCENT, 1);
        let none = attack_nearest(&mut inner, &json!({ "notoriety": "innocent" }));
        assert!(!none.ok, "an innocent is never a foe");
        mobile(&inner, ORC, ORC_BODY, NOTO_GREY, 3);
        let attacked = attack_nearest(&mut inner, &json!({}));
        assert!(attacked.ok, "{attacked:?}");
        assert_eq!(attacked.result["serial"], json!(ORC));
        assert_eq!(inner.last_target, Some(ORC));
    }

    #[test]
    fn a_mount_is_ridden_with_war_mode_off_first() {
        let mut inner = armed_session();
        ready_to_act(&mut inner);
        mobile(&inner, HORSE, HORSE_BODY, NOTO_INNOCENT, 1);
        assert!(!mount(&mut inner, &json!({})).ok, "not a pet of hers");
        inner.world.write().self_state.war = true;
        let rode = mount(&mut inner, &json!({ "serial": HORSE.0 }));
        assert!(rode.ok, "{rode:?}");
        assert_eq!(
            sent(&inner),
            vec![encode::war_mode(false), encode::double_click(HORSE)]
        );
        assert!(!dismount(&mut inner).ok, "not riding yet");
    }

    #[test]
    fn the_ignore_lists_add_remove_and_hide() {
        let mut inner = armed_session();
        let args = |action: &str, list: &str, value: Value| json!({ "action": action, "list": list, "value": value });
        assert!(ignore_list(&mut inner, &args("add", "gumps", json!(0x1234))).ok);
        assert!(ignore_list(&mut inner, &args("add", "journal", json!("Town Crier"))).ok);
        assert!(gump_ignored(&inner, 0x1234));
        assert!(line_ignored(&inner, "a town crier", "hear ye"));
        assert!(!line_ignored(&inner, "Ann", "hello"));
        assert!(ignore_list(&mut inner, &args("remove", "gumps", json!(0x1234))).ok);
        assert!(!gump_ignored(&inner, 0x1234));
        assert!(!ignore_list(&mut inner, &args("add", "gumps", Value::Null)).ok);
        assert!(!ignore_list(&mut inner, &args("add", "mail", json!(1))).ok);
    }

    #[test]
    fn a_line_is_said_in_the_colour_asked_and_a_yell_is_a_yell() {
        const PURPLE: u16 = 0x0021;
        const HUE_AT: usize = 4;
        const KIND_AT: usize = 3;
        let mut inner = armed_session();
        let said = handle_tool(
            &mut inner,
            ToolCall {
                name: TOOL_SAY.into(),
                args: json!({ "text": "good morning", "hue": PURPLE }),
            },
        );
        assert!(said.ok, "{said:?}");
        let line = inner.outbound.pop_back().unwrap();
        assert_eq!(u16::from_be_bytes([line[HUE_AT], line[HUE_AT + 1]]), PURPLE);
        let yelled = handle_tool(
            &mut inner,
            ToolCall {
                name: TOOL_SAY.into(),
                args: json!({ "text": "over here", "channel": "yell" }),
            },
        );
        assert!(yelled.ok, "{yelled:?}");
        let line = inner.outbound.pop_back().unwrap();
        assert_eq!(line[KIND_AT], SPEECH_YELL);
    }

    #[test]
    fn each_script_command_tool_writes_its_line() {
        const PET: Serial = Serial(0x0000_0555);
        const PICKAXE: Serial = Serial(0x4000_0556);
        let mut inner = armed_session();
        let line = |result: ToolResult| result.result["lines"].as_str().map(String::from);
        assert_eq!(
            line(rename(
                &mut inner,
                &json!({ "serial": PET.0, "name": "Rex" })
            )),
            Some(format!("rename {} 'Rex'", PET.0))
        );
        assert_eq!(
            line(fly(&mut inner, &json!({ "on": false }))),
            Some("land".into())
        );
        ready_to_act(&mut inner);
        assert_eq!(
            line(target_resource(
                &mut inner,
                &json!({ "tool": PICKAXE.0, "resource": "ore" })
            )),
            Some(format!("targetresource {} 'ore'", PICKAXE.0))
        );
        assert!(!set_ability(&mut inner, &json!({ "ability": "sneeze" })).ok);
        assert!(
            !skill_lock(&mut inner, &json!({ "skill": "hiding" })).ok,
            "needs a lock"
        );
        let used = use_type(
            &mut inner,
            &json!({ "graphic": 0x0E21, "source": "ground", "range": 2 }),
        );
        assert!(used.ok, "{used:?}");
        assert_eq!(line(used), Some("usetype 3617 any 'ground' 2".into()));
    }

    #[test]
    fn loot_goes_into_the_catch_bag_while_it_is_set() {
        const BAG: Serial = Serial(0x4000_0777);
        let mut inner = armed_session();
        assert!(
            !catch_bag(&mut inner, &json!({ "serial": BAG.0 })).ok,
            "not seen yet"
        );
        inner.world.write().containers.insert(
            BAG,
            uoterm_world::Container {
                serial: BAG,
                gump: 0,
                items: Vec::new(),
                opened: 0,
            },
        );
        inner.world.write().items.insert(
            BAG,
            uoterm_world::Item {
                serial: BAG,
                graphic: 0x0E76,
                amount: 1,
                hue: 0,
                location: Point3::new(0, 0, 0),
                parent: Some(super::super::relay_tests::PACK),
                layer: None,
                grid: 0,
                name: String::new(),
                flags: 0,
            },
        );
        assert!(catch_bag(&mut inner, &json!({ "serial": BAG.0 })).ok);
        let pack = super::super::relay_tests::PACK;
        assert_eq!(catch_bag_or(&inner, pack), BAG);
        assert!(catch_bag(&mut inner, &json!({ "clear": true })).ok);
        assert_eq!(catch_bag_or(&inner, pack), pack);
    }

    #[test]
    fn the_listen_range_option_reaches_the_world() {
        let mut inner = armed_session();
        const RANGE: u16 = 4;
        inner.agents.config.options.listen_range = Some(RANGE);
        share_listen_range(&inner);
        assert_eq!(inner.world.read().listen_range, Some(RANGE));
    }

    #[test]
    fn the_skills_gained_are_kept_with_their_rate() {
        let mut inner = armed_session();
        let set = |inner: &Inner, value: u16| {
            inner.world.write().self_state.skills.insert(
                40,
                uoterm_world::SkillValue {
                    value,
                    base: value,
                    cap: 1000,
                    lock: 0,
                },
            );
        };
        set(&inner, 500);
        note_skills(&mut inner);
        set(&inner, 501);
        note_skills(&mut inner);
        set(&inner, 503);
        note_skills(&mut inner);
        let gains = skill_gains(&mut inner, &json!({}));
        assert!(gains.ok, "{gains:?}");
        assert_eq!(gains.result["skills"][0]["gained"], json!(0.3));
        assert_eq!(gains.result["skills"][0]["changes"], json!(2));
        assert_eq!(gains.result["recent"][0]["value"], json!(50.3));
        let cleared = skill_gains(&mut inner, &json!({ "clear": true }));
        assert!(cleared.result["skills"].as_array().unwrap().is_empty());
    }
}
