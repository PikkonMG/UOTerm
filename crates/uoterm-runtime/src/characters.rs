//! The characters of an account, for an agent with no screen: the list with
//! its empty slots and start towns, a new character made to the rules of the
//! client version, one deleted, and a login as one of them. These tools
//! belong to the runtime, not to a session: before a login there is no
//! session to call.
//!
//! The account and the password come from a saved login (`profile`) or from
//! `account` and `password_env`, the name of an environment variable that
//! holds the password. A password is never taken as an argument, never
//! written to a log and never given back.

use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::{json, Value};
use uoterm_protocol::types::{ClientVersion, Era};
use uoterm_protocol::StartTown;

use crate::config::{
    client_version, era_from_str, load_app_config, load_profile, parse_encryption_mode,
    password_from_env, profile_path, AppConfig, CharacterRequest, ConnectOptions, LoginPicker,
    LoginQuestion, NewCharacterWish,
};
use crate::manager::Runtime;
use crate::tools::ToolResult;

pub const TOOL_CONNECT: &str = "connect";
pub const TOOL_DISCONNECT: &str = "disconnect";
pub const TOOL_CHARACTERS: &str = "characters";
pub const TOOL_CHARACTER_CREATE: &str = "character_create";
pub const TOOL_CHARACTER_DELETE: &str = "character_delete";

/// The runtime's own tools: each name, what it does, and what it needs.
pub const RUNTIME_TOOLS: [(&str, &str, &str); 5] = [
    (
        TOOL_CONNECT,
        "Logs in and starts a session: the character named (character), or the first of the account. Credentials: profile (a saved login of the profiles folder), or account and password_env (the name of an environment variable that holds the password; a password is never an argument). host, port, shard, era, version, encryption and proxy default to the saved login and the config file. Answers session_id.",
        "the shard is up",
    ),
    (
        TOOL_DISCONNECT,
        "Ends a session at once, with no logout: session_id.",
        "a session exists",
    ),
    (
        TOOL_CHARACTERS,
        "The character list of an account, with no login: each slot with its name, or empty, and the start towns a new character may pick. Same credentials as connect.",
        "the shard is up",
    ),
    (
        TOOL_CHARACTER_CREATE,
        "Makes a character and logs in as it: name, female, race (human, elf, gargoyle), str, dex, int (each 10 to 60; 80 in all, or 90 from client 7.0.16), skills ([{skill, value}]: 3, or 4 from client 7.0.16, each up to 50, 100 or 120 in all), skin_hue, hair, hair_hue, beard, beard_hue, shirt_hue, pants_hue, profession (0 for the numbers above), start_city (a town of characters). The rules of the client version are checked first. Same credentials as connect. Answers session_id.",
        "an empty slot on the account",
    ),
    (
        TOOL_CHARACTER_DELETE,
        "Deletes a character of the account, by name or slot, and answers the list after. The shard may refuse, for example a character played lately. Same credentials as connect.",
        "the shard is up",
    ),
];

const ARG_PROFILE: &str = "profile";
const ARG_ACCOUNT: &str = "account";
const ARG_PASSWORD_ENV: &str = "password_env";
const ARG_HOST: &str = "host";
const ARG_PORT: &str = "port";
const ARG_SHARD: &str = "shard";
const ARG_CHARACTER: &str = "character";
const ARG_ERA: &str = "era";
const ARG_VERSION: &str = "version";
const ARG_ENCRYPTION: &str = "encryption";
const ARG_PROXY: &str = "proxy";
const ARG_SESSION_ID: &str = "session_id";
const ARG_NAME: &str = "name";
const ARG_SLOT: &str = "slot";
const ARG_FEMALE: &str = "female";
const ARG_RACE: &str = "race";
const ARG_STR: &str = "str";
const ARG_DEX: &str = "dex";
const ARG_INT: &str = "int";
const ARG_SKILLS: &str = "skills";
const ARG_SKILL: &str = "skill";
const ARG_VALUE: &str = "value";
const ARG_SKIN_HUE: &str = "skin_hue";
const ARG_HAIR: &str = "hair";
const ARG_HAIR_HUE: &str = "hair_hue";
const ARG_BEARD: &str = "beard";
const ARG_BEARD_HUE: &str = "beard_hue";
const ARG_SHIRT_HUE: &str = "shirt_hue";
const ARG_PANTS_HUE: &str = "pants_hue";
const ARG_PROFESSION: &str = "profession";
const ARG_START_CITY: &str = "start_city";

/// The races by name, in the order the create request numbers them.
const RACES: [&str; 3] = ["human", "elf", "gargoyle"];
const RACE_HUMAN: u8 = 0;
const RACE_ELF: u8 = 1;
/// The first client versions that can make an elf and a gargoyle.
const ELF_FIRST: ClientVersion = ClientVersion::new(4, 0, 11, b'd' as u32);
const GARGOYLE_FIRST: ClientVersion = ClientVersion::new(6, 0, 14, 4);
/// A new character's stats: each in this range, and the sum the shard asks
/// of an older client and of one from 7.0.16.
const STAT_MIN: u8 = 10;
const STAT_MAX: u8 = 60;
const STATS_TOTAL_OLD: u16 = 80;
const STATS_TOTAL_NEW: u16 = 90;
/// A new character's skills: how many an older client and one from 7.0.16
/// send, the most one may start at, and the sums a shard takes.
const SKILLS_OLD: usize = 3;
const SKILLS_NEW: usize = 4;
const SKILL_START_MAX: u8 = 50;
const SKILL_TOTALS: [u16; 2] = [100, 120];
/// The highest skill number of the classic list.
const SKILL_ID_MAX: u8 = 57;
/// A character name: this long, of letters and these marks, and starting
/// with a letter.
const NAME_MIN: usize = 2;
const NAME_MAX: usize = 16;
const NAME_MARKS: [char; 4] = [' ', '-', '.', '\''];
/// The answer the login gets for a pick of the first shard or character.
const FIRST: usize = 0;

const NEEDS_ACCOUNT: &str =
    "needs profile, or account and password_env (the variable that holds the password)";
const NEEDS_SESSION: &str = "disconnect needs session_id";
const NEEDS_TARGET: &str = "character_delete needs name or slot";
const NO_SUCH_CHARACTER: &str = "no character by that name or slot on the account";
const NO_LIST: &str = "the shard sent no character list";
const NO_SUCH_TOWN: &str = "start_city is no start town of the shard's list; see characters";

/// What a scripted login does at the character list.
enum Plan {
    /// Look, and play none.
    List,
    Create(Box<NewCharacterWish>),
    Delete(Target),
}

/// A character of the account, by its name or its slot.
enum Target {
    Name(String),
    Slot(usize),
}

impl Target {
    fn slot_in(&self, names: &[String]) -> Option<usize> {
        match self {
            Self::Slot(slot) => names
                .get(*slot)
                .filter(|name| !name.is_empty())
                .map(|_| *slot),
            Self::Name(wanted) => names
                .iter()
                .position(|name| !name.is_empty() && name.eq_ignore_ascii_case(wanted)),
        }
    }
}

/// What a scripted login saw at the character list.
#[derive(Default)]
struct Seen {
    names: Vec<String>,
    towns: Vec<StartTown>,
    /// The words of the shard's last refusal.
    refused: Option<String>,
    /// Why the plan could not be carried out, when it could not.
    failed: Option<String>,
}

/// A login screen with nobody at it: it answers each question by the plan,
/// then leaves at the next list, and writes down what it saw.
fn scripted(plan: Plan) -> (LoginPicker, Arc<Mutex<Seen>>) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let seen = Arc::new(Mutex::new(Seen::default()));
    let record = seen.clone();
    tokio::spawn(async move {
        let mut plan = Some(plan);
        while let Some(question) = rx.recv().await {
            match question {
                LoginQuestion::Shard { reply, .. } | LoginQuestion::Character { reply, .. } => {
                    let _ = reply.send(FIRST);
                }
                LoginQuestion::Characters {
                    names,
                    refused,
                    choices,
                    reply,
                } => {
                    let mut seen = record.lock();
                    if !choices.towns.is_empty() {
                        seen.towns = choices.towns;
                    }
                    seen.refused = refused.or(seen.refused.take());
                    let request = match plan.take() {
                        None | Some(Plan::List) => CharacterRequest::Leave,
                        Some(Plan::Delete(target)) => match target.slot_in(&names) {
                            Some(slot) => CharacterRequest::Delete(slot),
                            None => {
                                seen.failed = Some(NO_SUCH_CHARACTER.into());
                                CharacterRequest::Leave
                            }
                        },
                        Some(Plan::Create(mut wish)) => {
                            let town_known = seen.towns.is_empty()
                                || seen
                                    .towns
                                    .iter()
                                    .any(|t| u16::from(t.index) == wish.start_city);
                            if town_known {
                                wish.slot = names
                                    .iter()
                                    .position(String::is_empty)
                                    .unwrap_or(names.len())
                                    as u16;
                                CharacterRequest::Make(wish)
                            } else {
                                seen.failed = Some(NO_SUCH_TOWN.into());
                                CharacterRequest::Leave
                            }
                        }
                    };
                    seen.names = names;
                    let _ = reply.send(request);
                }
            }
        }
    });
    (LoginPicker(tx), seen)
}

/// A text argument, trimmed, when it is there and not empty.
fn text<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

/// A number argument that fits `T`.
fn number<T: TryFrom<u64>>(args: &Value, key: &str) -> Option<T> {
    args.get(key)
        .and_then(Value::as_u64)
        .and_then(|n| T::try_from(n).ok())
}

/// The options of a login from the call, the saved login it names and the
/// config file. The password is read from its environment variable here and
/// goes nowhere but into the options.
fn login_options(cfg: &AppConfig, args: &Value) -> std::result::Result<ConnectOptions, String> {
    let profile = match text(args, ARG_PROFILE) {
        Some(name) => Some(load_profile(&profile_path(name)).map_err(|e| e.to_string())?),
        None => None,
    };
    let from_profile = |pick: fn(&crate::config::Profile) -> Option<&str>| {
        profile.as_ref().and_then(pick).map(str::to_string)
    };
    let account = text(args, ARG_ACCOUNT)
        .map(str::to_string)
        .or_else(|| from_profile(|p| Some(p.account.as_str())))
        .ok_or(NEEDS_ACCOUNT)?;
    let password_env = text(args, ARG_PASSWORD_ENV)
        .map(str::to_string)
        .or_else(|| from_profile(|p| Some(p.password_env.as_str())))
        .ok_or(NEEDS_ACCOUNT)?;
    let password = password_from_env(&password_env).map_err(|e| e.to_string())?;
    let era_name = text(args, ARG_ERA)
        .map(str::to_string)
        .or_else(|| from_profile(|p| p.era.as_deref()));
    let era: Era = match era_name {
        Some(name) => era_from_str(Some(&name)),
        None => cfg.era,
    };
    let version = client_version(
        text(args, ARG_VERSION)
            .map(str::to_string)
            .or_else(|| from_profile(|p| p.version.as_deref()))
            .as_deref(),
        era,
        cfg.uopath.as_deref(),
    );
    let proxy = match text(args, ARG_PROXY) {
        Some(url) => Some(url.parse()?),
        None => cfg.proxy.clone(),
    };
    let encryption = match text(args, ARG_ENCRYPTION) {
        Some(mode) => parse_encryption_mode(mode).map_err(|e| e.to_string())?,
        None => Default::default(),
    };
    Ok(ConnectOptions {
        host: text(args, ARG_HOST).map_or_else(|| cfg.host.clone(), str::to_string),
        port: number(args, ARG_PORT).unwrap_or(cfg.port),
        account,
        password,
        shard: text(args, ARG_SHARD)
            .map(str::to_string)
            .or_else(|| from_profile(|p| p.shard.as_deref())),
        character: text(args, ARG_CHARACTER)
            .map(str::to_string)
            .or_else(|| from_profile(|p| Some(p.character.as_str())))
            .unwrap_or_default(),
        version,
        era,
        uopath: cfg.uopath.clone(),
        markers: cfg.markers.clone(),
        encryption,
        obey_shard_rules: cfg.obey_shard_rules,
        answer_when_named: cfg.answer_when_named,
        play_along: cfg.play_along,
        reconnect: cfg.reconnect,
        proxy,
        ..ConnectOptions::default()
    })
}

/// The slots of a list as an agent reads them.
fn slots_json(names: &[String]) -> Value {
    json!(names
        .iter()
        .enumerate()
        .map(|(slot, name)| json!({ "slot": slot, "name": name, "empty": name.is_empty() }))
        .collect::<Vec<_>>())
}

fn towns_json(towns: &[StartTown]) -> Value {
    json!(towns
        .iter()
        .map(|t| json!({ "index": t.index, "town": t.name, "building": t.building }))
        .collect::<Vec<_>>())
}

/// Logs in with a screen that follows `plan`, and gives back what it saw
/// and how the login ended.
async fn scripted_login(
    runtime: &Runtime,
    mut opts: ConnectOptions,
    plan: Plan,
) -> (crate::error::Result<crate::SessionHandle>, Arc<Mutex<Seen>>) {
    let (picker, seen) = scripted(plan);
    opts.picker = Some(picker);
    // A login that stops at the list is no link lost.
    opts.reconnect = false;
    (runtime.connect(opts).await, seen)
}

/// The words for a login that ended at the character list with no list.
fn login_failed(ended: crate::error::Result<crate::SessionHandle>, seen: &Seen) -> String {
    seen.failed
        .clone()
        .or_else(|| seen.refused.clone())
        .unwrap_or_else(|| match ended {
            Ok(_) => NO_LIST.into(),
            Err(e) => e.to_string(),
        })
}

/// The name of the character a session plays, once the world has it.
fn played(handle: &crate::SessionHandle) -> String {
    handle.world.read().self_state.name.clone()
}

async fn connect(runtime: &Runtime, cfg: &AppConfig, args: &Value) -> ToolResult {
    let opts = match login_options(cfg, args) {
        Ok(opts) => opts,
        Err(why) => return ToolResult::err(why),
    };
    match runtime.connect(opts).await {
        Ok(handle) => ToolResult::ok(json!({
            "session_id": handle.id,
            "character": played(&handle),
        })),
        Err(e) => ToolResult::err(e.to_string()),
    }
}

async fn disconnect(runtime: &Runtime, args: &Value) -> ToolResult {
    let Some(id) = text(args, ARG_SESSION_ID) else {
        return ToolResult::err(NEEDS_SESSION);
    };
    if runtime.get(id).is_none() {
        return ToolResult::err(format!("no session {id}"));
    }
    match runtime.stop(id).await {
        Ok(()) => ToolResult::ok(json!({ "stopped": id })),
        Err(e) => ToolResult::err(e.to_string()),
    }
}

async fn characters(runtime: &Runtime, cfg: &AppConfig, args: &Value) -> ToolResult {
    let mut opts = match login_options(cfg, args) {
        Ok(opts) => opts,
        Err(why) => return ToolResult::err(why),
    };
    // No name, so the login asks the screen, which plays none.
    opts.character.clear();
    let (ended, seen) = scripted_login(runtime, opts, Plan::List).await;
    let seen = seen.lock();
    if seen.names.is_empty() {
        return ToolResult::err(login_failed(ended, &seen));
    }
    ToolResult::ok(json!({
        "slots": slots_json(&seen.names),
        "start_towns": towns_json(&seen.towns),
    }))
}

async fn character_delete(runtime: &Runtime, cfg: &AppConfig, args: &Value) -> ToolResult {
    let target = match (text(args, ARG_NAME), number::<usize>(args, ARG_SLOT)) {
        (Some(name), _) => Target::Name(name.to_string()),
        (None, Some(slot)) => Target::Slot(slot),
        (None, None) => return ToolResult::err(NEEDS_TARGET),
    };
    let mut opts = match login_options(cfg, args) {
        Ok(opts) => opts,
        Err(why) => return ToolResult::err(why),
    };
    opts.character.clear();
    let (ended, seen) = scripted_login(runtime, opts, Plan::Delete(target)).await;
    let seen = seen.lock();
    if seen.failed.is_some() || seen.refused.is_some() || seen.names.is_empty() {
        return ToolResult::err(login_failed(ended, &seen));
    }
    ToolResult::ok(json!({ "slots": slots_json(&seen.names) }))
}

async fn character_create(runtime: &Runtime, cfg: &AppConfig, args: &Value) -> ToolResult {
    let mut opts = match login_options(cfg, args) {
        Ok(opts) => opts,
        Err(why) => return ToolResult::err(why),
    };
    let wish = match new_character(args, opts.version, opts.era) {
        Ok(wish) => wish,
        Err(why) => return ToolResult::err(why),
    };
    // No name, so the login asks the screen, which makes the character.
    let name = wish.name.clone();
    let mut play_it = opts.clone();
    opts.character.clear();
    let (ended, seen) = scripted_login(runtime, opts, Plan::Create(Box::new(wish))).await;
    let made_but_listed = {
        let seen = seen.lock();
        seen.refused.is_none()
            && seen.failed.is_none()
            && seen.names.iter().any(|n| n.eq_ignore_ascii_case(&name))
    };
    let ended = match ended {
        // Most shards put a new character straight into the world.
        Ok(handle) => Ok(handle),
        // Others send the list with it on, and it is played from there.
        Err(_) if made_but_listed => {
            play_it.character = name;
            runtime.connect(play_it).await
        }
        Err(e) => Err(e),
    };
    match ended {
        Ok(handle) => ToolResult::ok(json!({
            "session_id": handle.id,
            "character": played(&handle),
        })),
        Err(e) => ToolResult::err(login_failed(Err(e), &seen.lock())),
    }
}

/// The character a call asks for, checked against the rules of the client
/// version and the era: the name, the race, the stats and the skills. A
/// profession names its own stats and skills, and those are not checked.
fn new_character(
    args: &Value,
    version: ClientVersion,
    era: Era,
) -> std::result::Result<NewCharacterWish, String> {
    let name = text(args, ARG_NAME).ok_or("character_create needs name")?;
    check_name(name)?;
    let race = match text(args, ARG_RACE) {
        None => RACE_HUMAN,
        Some(word) => RACES
            .iter()
            .position(|race| race.eq_ignore_ascii_case(word))
            .map(|race| race as u8)
            .ok_or("race must be human, elf or gargoyle")?,
    };
    let race_ok = match race {
        RACE_HUMAN => true,
        RACE_ELF => era != Era::T2a && version.at_least(ELF_FIRST),
        _ => era != Era::T2a && version.at_least(GARGOYLE_FIRST),
    };
    if !race_ok {
        return Err(format!(
            "a {} needs a newer client than {version}",
            RACES[usize::from(race)]
        ));
    }
    let profession: u8 = number(args, ARG_PROFESSION).unwrap_or(0);
    let stat = |key: &str| number::<u8>(args, key).unwrap_or(STAT_MIN);
    let (strength, dexterity, intelligence) = (stat(ARG_STR), stat(ARG_DEX), stat(ARG_INT));
    let mut skills: Vec<(u8, u8)> = Vec::new();
    for entry in args
        .get(ARG_SKILLS)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let skill = number::<u8>(entry, ARG_SKILL).ok_or("each skill needs skill (a number)")?;
        let value = number::<u8>(entry, ARG_VALUE).ok_or("each skill needs value")?;
        skills.push((skill, value));
    }
    let newer = version.has_three_starting_skills();
    if profession == 0 {
        check_stats([strength, dexterity, intelligence], newer)?;
        check_skills(&skills, newer)?;
    }
    let word = |key: &str| number::<u16>(args, key).unwrap_or(0);
    Ok(NewCharacterWish {
        name: name.to_string(),
        female: args
            .get(ARG_FEMALE)
            .and_then(Value::as_bool)
            .unwrap_or(false),
        race,
        strength,
        dexterity,
        intelligence,
        skills,
        skin_hue: word(ARG_SKIN_HUE),
        hair: word(ARG_HAIR),
        hair_hue: word(ARG_HAIR_HUE),
        beard: word(ARG_BEARD),
        beard_hue: word(ARG_BEARD_HUE),
        shirt_hue: word(ARG_SHIRT_HUE),
        pants_hue: word(ARG_PANTS_HUE),
        profession,
        start_city: word(ARG_START_CITY),
        slot: 0,
    })
}

fn check_name(name: &str) -> std::result::Result<(), String> {
    let count = name.chars().count();
    let letters_first = name.chars().next().is_some_and(char::is_alphabetic);
    let allowed = name
        .chars()
        .all(|c| c.is_alphabetic() || NAME_MARKS.contains(&c));
    if (NAME_MIN..=NAME_MAX).contains(&count) && letters_first && allowed {
        Ok(())
    } else {
        Err(format!(
            "a name has {NAME_MIN} to {NAME_MAX} letters, spaces, dashes, dots or quotes, and starts with a letter"
        ))
    }
}

fn check_stats(stats: [u8; 3], newer: bool) -> std::result::Result<(), String> {
    let total: u16 = stats.iter().map(|s| u16::from(*s)).sum();
    let wanted = if newer {
        STATS_TOTAL_NEW
    } else {
        STATS_TOTAL_OLD
    };
    if stats.iter().any(|s| !(STAT_MIN..=STAT_MAX).contains(s)) || total != wanted {
        return Err(format!(
            "str, dex and int are each {STAT_MIN} to {STAT_MAX} and {wanted} in all for this client"
        ));
    }
    Ok(())
}

fn check_skills(skills: &[(u8, u8)], newer: bool) -> std::result::Result<(), String> {
    let most = if newer { SKILLS_NEW } else { SKILLS_OLD };
    if skills.len() > most {
        return Err(format!("this client starts with at most {most} skills"));
    }
    let mut seen = Vec::new();
    for (skill, value) in skills {
        if *skill > SKILL_ID_MAX || *value > SKILL_START_MAX {
            return Err(format!(
                "a starting skill is a number up to {SKILL_ID_MAX} at most {SKILL_START_MAX}"
            ));
        }
        if seen.contains(skill) {
            return Err("a starting skill is named once".into());
        }
        seen.push(*skill);
    }
    let total: u16 = skills.iter().map(|(_, v)| u16::from(*v)).sum();
    if !SKILL_TOTALS.contains(&total) {
        return Err("the starting skills add up to 100 or 120".into());
    }
    Ok(())
}

/// True for a tool of the runtime, which no session answers.
pub fn is_runtime_tool(name: &str) -> bool {
    RUNTIME_TOOLS.iter().any(|(tool, _, _)| *tool == name)
}

/// Runs a tool of the runtime, with the defaults of the config file.
pub async fn call(runtime: &Runtime, name: &str, args: &Value) -> ToolResult {
    call_with(runtime, &load_app_config(None), name, args).await
}

async fn call_with(runtime: &Runtime, cfg: &AppConfig, name: &str, args: &Value) -> ToolResult {
    match name {
        TOOL_CONNECT => connect(runtime, cfg, args).await,
        TOOL_DISCONNECT => disconnect(runtime, args).await,
        TOOL_CHARACTERS => characters(runtime, cfg, args).await,
        TOOL_CHARACTER_CREATE => character_create(runtime, cfg, args).await,
        TOOL_CHARACTER_DELETE => character_delete(runtime, cfg, args).await,
        other => ToolResult::err(format!("unknown tool {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockServer, MOCK_CHAR};

    /// The variable the tests keep the mock account's password in. The mock
    /// takes any password.
    const PASSWORD_ENV: &str = "UOTERM_TEST_CHARACTERS_PASSWORD";
    const NEWCOMER: &str = "Lyra";

    fn login_args(server: &MockServer) -> Value {
        // SAFETY of the test: one variable, set to one value by every test.
        std::env::set_var(PASSWORD_ENV, "test");
        json!({
            "account": "test",
            "password_env": PASSWORD_ENV,
            "host": server.addr.ip().to_string(),
            "port": server.addr.port(),
            "era": "t2a",
        })
    }

    fn with(mut args: Value, extra: Value) -> Value {
        if let (Some(into), Value::Object(extra)) = (args.as_object_mut(), extra) {
            into.extend(extra);
        }
        args
    }

    fn fighter() -> Value {
        json!({
            "name": NEWCOMER,
            "female": true,
            "str": 40, "dex": 30, "int": 10,
            "skills": [{ "skill": 40, "value": 50 }, { "skill": 27, "value": 50 }],
            "hair": 0x203B, "hair_hue": 0x044E, "beard": 0, "skin_hue": 0x83EA,
            "shirt_hue": 0x0009, "pants_hue": 0x0010,
        })
    }

    #[test]
    fn a_new_character_keeps_the_rules_of_its_client() {
        let old = ClientVersion::T2A;
        let args = fighter();
        let wish = new_character(&args, old, Era::T2a).unwrap();
        assert_eq!((wish.strength, wish.shirt_hue, wish.pants_hue), (40, 9, 16));
        let stats_off = with(fighter(), json!({ "str": 45 }));
        assert!(new_character(&stats_off, old, Era::T2a).is_err());
        let too_high = with(
            fighter(),
            json!({ "skills": [{ "skill": 40, "value": 60 }, { "skill": 27, "value": 40 }] }),
        );
        assert!(new_character(&too_high, old, Era::T2a).is_err());
        let twice = with(
            fighter(),
            json!({ "skills": [{ "skill": 40, "value": 50 }, { "skill": 40, "value": 50 }] }),
        );
        assert!(new_character(&twice, old, Era::T2a).is_err());
        let elf = with(fighter(), json!({ "race": "elf" }));
        assert!(
            new_character(&elf, old, Era::T2a).is_err(),
            "no elves in T2A"
        );
        let modern = ClientVersion::MODERN;
        let new_stats = with(
            elf,
            json!({ "str": 50, "skills": [
            { "skill": 40, "value": 30 }, { "skill": 27, "value": 30 },
            { "skill": 17, "value": 30 }, { "skill": 5, "value": 30 }] }),
        );
        assert!(new_character(&new_stats, modern, Era::Modern).is_ok());
        assert!(new_character(&with(fighter(), json!({ "name": "x" })), old, Era::T2a).is_err());
        assert!(new_character(&with(fighter(), json!({ "name": "B0b" })), old, Era::T2a).is_err());
        let profession = with(fighter(), json!({ "profession": 2, "str": 0 }));
        assert!(new_character(&profession, old, Era::T2a).is_ok());
    }

    #[test]
    fn a_login_needs_a_password_variable_and_never_takes_the_password() {
        let cfg = AppConfig::default();
        assert!(login_options(&cfg, &json!({ "account": "a" })).is_err());
        let unset = json!({ "account": "a", "password_env": "UOTERM_TEST_UNSET_PASSWORD_VAR" });
        let refused = login_options(&cfg, &unset).unwrap_err();
        assert!(refused.contains("not set"));
        assert!(RUNTIME_TOOLS
            .iter()
            .all(|(_, about, _)| !about.contains("password:")));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_list_is_read_a_character_made_and_one_deleted() {
        let server = MockServer::start().await.unwrap();
        let rt = Runtime::new(4);
        let cfg = AppConfig::default();
        let args = login_args(&server);

        let listed = call_with(&rt, &cfg, TOOL_CHARACTERS, &args).await;
        assert!(listed.ok, "{listed:?}");
        assert_eq!(listed.result["slots"][0]["name"], json!(MOCK_CHAR));
        assert!(rt.list().is_empty(), "a list opens no session");

        let made = call_with(
            &rt,
            &cfg,
            TOOL_CHARACTER_CREATE,
            &with(args.clone(), fighter()),
        )
        .await;
        assert!(made.ok, "{made:?}");
        let id = made.result["session_id"].as_str().unwrap().to_string();
        assert!(rt.get(&id).is_some());
        assert_eq!(made.result["character"], json!(NEWCOMER));
        let gone = call_with(&rt, &cfg, TOOL_DISCONNECT, &json!({ "session_id": id })).await;
        assert!(gone.ok, "{gone:?}");

        let deleted = call_with(
            &rt,
            &cfg,
            TOOL_CHARACTER_DELETE,
            &with(args.clone(), json!({ "name": MOCK_CHAR })),
        )
        .await;
        assert!(deleted.ok, "{deleted:?}");
        let missing = call_with(
            &rt,
            &cfg,
            TOOL_CHARACTER_DELETE,
            &with(args.clone(), json!({ "name": "Nobody" })),
        )
        .await;
        assert!(!missing.ok);
        assert!(missing.error.unwrap().contains("no character"));

        let played = call_with(
            &rt,
            &cfg,
            TOOL_CONNECT,
            &with(args, json!({ "character": MOCK_CHAR })),
        )
        .await;
        assert!(played.ok, "{played:?}");
        assert_eq!(played.result["character"], json!(MOCK_CHAR));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_name_the_shard_refuses_comes_back_with_its_words() {
        let server = MockServer::start().await.unwrap();
        let rt = Runtime::new(4);
        let cfg = AppConfig::default();
        let taken = with(
            with(login_args(&server), fighter()),
            json!({ "name": MOCK_CHAR }),
        );
        let refused = call_with(&rt, &cfg, TOOL_CHARACTER_CREATE, &taken).await;
        assert!(!refused.ok);
        assert!(
            refused
                .error
                .as_deref()
                .unwrap()
                .contains("could not carry out"),
            "{refused:?}"
        );
        assert!(rt.list().is_empty());
    }
}
