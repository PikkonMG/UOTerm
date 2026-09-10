//! Agents: the always-on helpers (loot, scavenge, bandage, remount, carve,
//! open corpses, buy, sell) and the run-once jobs (organize, restock,
//! dress, undress).
//!
//! Agents run on the session tick after the reflexes and before a script.
//! Each one acts only when the character may act, one move at a time, with
//! its own delay between moves, so agents never go faster than a player.

mod config;
mod work;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub(super) use config::AgentsConfig;
pub(super) use config::MoveList as AgentMoveList;
use config::{DressItem, DressList, MoveList, TargetFilter, DEFAULT_LIST};

use super::*;

/// The folder agent settings are kept in, below the user's config folder.
const AGENTS_DIR: &str = "agents";
const AGENTS_FILE_EXT: &str = "toml";
/// A dress list made from what the character wears now.
pub(super) const TEMP_DRESS_LIST: &str = "temp";
/// An item held longer than this is taken to have landed somewhere the
/// client did not hear of, so the agents do not wait for it for ever.
const HOLD_LIMIT: Duration = Duration::from_secs(10);

/// The agents that switch on and off.
const SWITCHED: [&str; 9] = [
    "autoloot",
    "scavenger",
    "buy",
    "sell",
    "bandage",
    "remount",
    "bone_cutter",
    "carver",
    "open_corpses",
];

/// A run-once job and how far it has got.
#[derive(Clone, Debug)]
pub(super) enum Job {
    Organize {
        list: String,
        done: HashSet<Serial>,
    },
    Restock {
        list: String,
    },
    Dress {
        list: String,
    },
    Undress {
        list: Option<String>,
    },
    /// The autoloot list, once, on the corpses in range.
    LootOnce,
}

impl Job {
    fn name(&self) -> &'static str {
        match self {
            Self::Organize { .. } => "organizer",
            Self::Restock { .. } => "restock",
            Self::Dress { .. } => "dress",
            Self::Undress { .. } => "undress",
            Self::LootOnce => "autoloot",
        }
    }
}

/// Damage the character's side dealt to each mobile while the meter runs.
#[derive(Clone, Debug, Default)]
struct DamageMeter {
    started: Option<Instant>,
    paused_for: Duration,
    paused_at: Option<Instant>,
    dealt: HashMap<Serial, u32>,
}

/// What the session keeps for agents.
pub(super) struct Agents {
    pub(super) config: AgentsConfig,
    /// Where the settings are saved. None when there is no character name.
    file: Option<PathBuf>,
    pub(super) job: Option<Job>,
    /// Corpses opened, carved, and items already moved or tried.
    opened: HashSet<Serial>,
    carved: HashSet<Serial>,
    tried: HashSet<Serial>,
    /// Items whose property list was asked for.
    asked_properties: HashSet<Serial>,
    /// No agent moves an item before this.
    next_move_at: Instant,
    /// When the character was seen without a mount, for the remount delay.
    unmounted_since: Option<Instant>,
    meter: DamageMeter,
    /// The last pick of each target filter, for next and previous.
    last_pick: HashMap<String, Serial>,
    /// When the item in hand was lifted, for [`HOLD_LIMIT`].
    holding_since: Option<Instant>,
    /// The contents of the last container the shard listed, in its order.
    /// A vendor's buy list prices them in that same order.
    last_contents: Option<(Serial, Vec<uoterm_protocol::ContainerItem>)>,
}

impl Agents {
    /// The agents of a character, with the settings saved for it.
    pub(super) fn load(character: &str) -> Self {
        let file = (!character.trim().is_empty()).then(|| {
            crate::config::config_dir()
                .join(AGENTS_DIR)
                .join(format!("{}.{AGENTS_FILE_EXT}", character.trim()))
        });
        let config = file
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| match toml::from_str(&text) {
                Ok(config) => Some(config),
                Err(e) => {
                    tracing::warn!(error = %e, "the agent settings file does not read; using defaults");
                    None
                }
            })
            .unwrap_or_default();
        Self {
            config,
            file,
            job: None,
            opened: HashSet::new(),
            carved: HashSet::new(),
            tried: HashSet::new(),
            asked_properties: HashSet::new(),
            next_move_at: Instant::now(),
            unmounted_since: None,
            meter: DamageMeter::default(),
            last_pick: HashMap::new(),
            holding_since: None,
            last_contents: None,
        }
    }

    pub(super) fn save(&self) -> std::result::Result<(), String> {
        let Some(path) = &self.file else {
            return Ok(());
        };
        let text = toml::to_string_pretty(&self.config).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    /// True when the character counts this mobile as a friend.
    pub(super) fn is_friend(&self, world: &World, serial: Serial) -> bool {
        let f = &self.config.friends;
        f.friends.contains(&serial) || (f.include_party && world.party.contains(&serial))
    }

    pub(super) fn switch(&mut self, agent: &str, on: bool) -> std::result::Result<(), String> {
        let c = &mut self.config;
        let flag = match agent {
            "autoloot" => &mut c.autoloot.enabled,
            "scavenger" => &mut c.scavenger.enabled,
            "buy" => &mut c.buy.enabled,
            "sell" => &mut c.sell.enabled,
            "bandage" => &mut c.bandage.enabled,
            "remount" => &mut c.remount.enabled,
            "bone_cutter" => &mut c.bone_cutter.enabled,
            "carver" => &mut c.carver.enabled,
            "open_corpses" => &mut c.open_corpses.enabled,
            other => {
                return Err(format!(
                    "'{other}' does not switch on and off; the agents that do: {}",
                    SWITCHED.join(", ")
                ))
            }
        };
        *flag = on;
        Ok(())
    }

    pub(super) fn enabled(&self, agent: &str) -> bool {
        let c = &self.config;
        match agent {
            "autoloot" => c.autoloot.enabled,
            "scavenger" => c.scavenger.enabled,
            "buy" => c.buy.enabled,
            "sell" => c.sell.enabled,
            "bandage" => c.bandage.enabled,
            "remount" => c.remount.enabled,
            "bone_cutter" => c.bone_cutter.enabled,
            "carver" => c.carver.enabled,
            "open_corpses" => c.open_corpses.enabled,
            _ => false,
        }
    }

    /// Starts a run-once job, in place of any job already running.
    pub(super) fn start(&mut self, job: Job) -> std::result::Result<(), String> {
        let c = &self.config;
        let known = match &job {
            Job::Organize { list, .. } => c.organizer.contains_key(list),
            Job::Restock { list } => c.restock.contains_key(list),
            Job::Dress { list } => c.dress.contains_key(list),
            Job::Undress { list } => list.as_ref().map_or(true, |l| c.dress.contains_key(l)),
            Job::LootOnce => true,
        };
        if !known {
            return Err(format!("there is no {} list by that name", job.name()));
        }
        tracing::info!(job = job.name(), "agent job starts");
        self.job = Some(job);
        Ok(())
    }

    /// Takes a dress list from what the character wears now.
    pub(super) fn dress_from_worn(&mut self, world: &World) {
        let items = world
            .self_state
            .equipment
            .iter()
            .filter(|e| !NOT_DRESSED.contains(&e.layer))
            .map(|e| DressItem {
                layer: e.layer,
                serial: e.serial,
            })
            .collect();
        self.config.dress.insert(
            TEMP_DRESS_LIST.into(),
            DressList {
                items,
                ..DressList::default()
            },
        );
    }

    /// Notes damage the shard reports on a mobile, while the meter runs.
    pub(super) fn note_damage(&mut self, me: Serial, serial: Serial, amount: u16) {
        let m = &mut self.meter;
        if serial != me && m.started.is_some() && m.paused_at.is_none() {
            *m.dealt.entry(serial).or_default() += u32::from(amount);
        }
    }

    /// Keeps the last container list, in order, for a vendor's buy list.
    pub(super) fn note_contents(&mut self, items: &[uoterm_protocol::ContainerItem]) {
        if let Some(first) = items.first() {
            self.last_contents = Some((first.container, items.to_vec()));
        }
    }
}

/// Layers a dress list leaves alone: the pack, the bank, hair, beard and a
/// mount are not clothes.
const NOT_DRESSED: [u8; 5] = [
    LAYER_BACKPACK,
    LAYER_BANK,
    LAYER_HAIR,
    LAYER_BEARD,
    LAYER_MOUNT,
];
const LAYER_HAIR: u8 = 11;
const LAYER_BEARD: u8 = 16;

/// Runs the agents for one tick.
pub(super) fn pump_agents(inner: &mut Inner, now: Instant) {
    let (alive, holding) = {
        let w = inner.world.read();
        (w.logged_in && !w.self_state.dead, w.holding)
    };
    if !alive {
        return;
    }
    // An item on the cursor has to land before the next lift.
    match holding {
        Some(_) => {
            let since = *inner.agents.holding_since.get_or_insert(now);
            if now.saturating_duration_since(since) < HOLD_LIMIT {
                return;
            }
            tracing::warn!("an item stayed on the cursor too long; the agents go on");
            inner.world.write().holding = None;
            inner.agents.holding_since = None;
        }
        None => inner.agents.holding_since = None,
    }
    if !action_ready(inner) || now < inner.agents.next_move_at {
        return;
    }
    let acted = work::remount(inner, now)
        || work::bandage(inner, now)
        || work::job(inner, now)
        || work::autoloot(inner, now, false)
        || work::scavenge(inner, now)
        || work::carve(inner, now)
        || work::cut_bones(inner, now)
        || work::open_corpses(inner, now);
    if acted {
        tracing::debug!("an agent acted");
    }
}

/// Answers a vendor's buy list with the buy agent's list.
pub(super) fn on_buy_list(
    inner: &mut Inner,
    container: Serial,
    entries: &[uoterm_protocol::VendorBuyEntry],
) {
    if inner.agents.config.buy.enabled {
        work::buy(inner, container, entries);
    }
}

/// Answers a vendor's sell list with the sell agent's list. Returns true
/// when the agent answered it.
pub(super) fn on_sell_list(
    inner: &mut Inner,
    vendor: Serial,
    entries: &[uoterm_protocol::VendorSellEntry],
) -> bool {
    inner.agents.config.sell.enabled && work::sell(inner, vendor, entries)
}

/// Joins a party a friend asked the character into, when the friends list
/// says to.
pub(super) fn on_party_invite(inner: &mut Inner, leader: Serial) {
    let world = inner.world.read().clone();
    let a = &inner.agents;
    if a.config.friends.accept_party && a.is_friend(&world, leader) {
        inner.outbound.push_back(encode::party_accept(leader));
    }
}

// ---------- tools ----------

const ARG_AGENT: &str = "agent";
const ARG_ON: &str = "on";
const ARG_LIST: &str = "list";
const ARG_SETTINGS: &str = "settings";
const ARG_ACTION: &str = "action";

fn agent_arg(args: &Value) -> std::result::Result<String, String> {
    args.get(ARG_AGENT)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "needs agent".to_string())
}

/// Every agent's settings, which are on, and the job running.
pub(super) fn agents_status(inner: &Inner) -> ToolResult {
    let a = &inner.agents;
    let on: Vec<&str> = SWITCHED.iter().copied().filter(|n| a.enabled(n)).collect();
    ToolResult::ok(json!({
        "on": on,
        "job": a.job.as_ref().map(Job::name),
        "settings": a.config,
        "file": a.file,
    }))
}

/// Replaces one agent's settings, or one named list of an agent that keeps
/// lists, and saves them.
pub(super) fn agent_set(inner: &mut Inner, args: &Value) -> ToolResult {
    let agent = match agent_arg(args) {
        Ok(a) => a,
        Err(e) => return ToolResult::err(e),
    };
    let Some(settings) = args.get(ARG_SETTINGS) else {
        return ToolResult::err("agent_set needs settings");
    };
    let settings = serials_as_numbers(settings.clone());
    let list = args
        .get(ARG_LIST)
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let c = &mut inner.agents.config;
    let result = match (agent.as_str(), list) {
        ("organizer", Some(list)) => from(settings).map(|v: MoveList| {
            c.organizer.insert(list, v);
        }),
        ("restock", Some(list)) => from(settings).map(|v: MoveList| {
            c.restock.insert(list, v);
        }),
        ("dress", Some(list)) => from(settings).map(|v: DressList| {
            c.dress.insert(list, v);
        }),
        ("targets", Some(list)) => from(settings).map(|v: TargetFilter| {
            c.targets.insert(list, v);
        }),
        ("organizer" | "restock" | "dress" | "targets", None) => {
            Err(format!("{agent} keeps named lists; give list"))
        }
        ("autoloot", Some(list)) => from(settings).map(|v| {
            c.autoloot.items.lists.insert(list, v);
        }),
        ("scavenger", Some(list)) => from(settings).map(|v| {
            c.scavenger.items.lists.insert(list, v);
        }),
        ("buy", Some(list)) => from(settings).map(|v| {
            c.buy.items.lists.insert(list, v);
        }),
        ("sell", Some(list)) => from(settings).map(|v| {
            c.sell.items.lists.insert(list, v);
        }),
        ("autoloot", None) => from(settings).map(|v| c.autoloot = v),
        ("scavenger", None) => from(settings).map(|v| c.scavenger = v),
        ("buy", None) => from(settings).map(|v| c.buy = v),
        ("sell", None) => from(settings).map(|v| c.sell = v),
        ("bandage", _) => from(settings).map(|v| c.bandage = v),
        ("friends", _) => from(settings).map(|v| c.friends = v),
        ("remount", _) => from(settings).map(|v| c.remount = v),
        ("bone_cutter", _) => from(settings).map(|v| c.bone_cutter = v),
        ("carver", _) => from(settings).map(|v| c.carver = v),
        ("open_corpses", _) => from(settings).map(|v| c.open_corpses = v),
        (other, _) => Err(format!("no agent named '{other}'")),
    };
    if let Err(e) = result.and_then(|()| inner.agents.save()) {
        return ToolResult::err(e);
    }
    ToolResult::ok(json!({ "agent": agent }))
}

fn from<T: serde::de::DeserializeOwned>(value: Value) -> std::result::Result<T, String> {
    serde_json::from_value(value).map_err(|e| format!("settings do not read: {e}"))
}

/// Turns every `0x...` text in the settings into its number, so a serial
/// or a graphic copied from `observe` reads as one.
fn serials_as_numbers(value: Value) -> Value {
    match value {
        Value::String(text) => match text
            .strip_prefix("0x")
            .or_else(|| text.strip_prefix("0X"))
            .and_then(|hex| u64::from_str_radix(hex, 16).ok())
        {
            Some(n) => json!(n),
            None => Value::String(text),
        },
        Value::Array(items) => Value::Array(items.into_iter().map(serials_as_numbers).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, serials_as_numbers(v)))
                .collect(),
        ),
        other => other,
    }
}

/// Switches an agent on or off, and picks the list it uses when one is
/// named.
pub(super) fn agent_on(inner: &mut Inner, args: &Value) -> ToolResult {
    let agent = match agent_arg(args) {
        Ok(a) => a,
        Err(e) => return ToolResult::err(e),
    };
    let on = args.get(ARG_ON).and_then(|v| v.as_bool()).unwrap_or(true);
    if let Some(list) = args.get(ARG_LIST).and_then(|v| v.as_str()) {
        if let Err(e) = use_list(&mut inner.agents.config, &agent, list) {
            return ToolResult::err(e);
        }
    }
    if let Err(e) = inner
        .agents
        .switch(&agent, on)
        .and_then(|()| inner.agents.save())
    {
        return ToolResult::err(e);
    }
    ToolResult::ok(json!({ "agent": agent, "on": on }))
}

/// Picks the named list an agent uses.
pub(super) fn use_list(
    config: &mut AgentsConfig,
    agent: &str,
    list: &str,
) -> std::result::Result<(), String> {
    let lists = match agent {
        "autoloot" => &mut config.autoloot.items,
        "scavenger" => &mut config.scavenger.items,
        "buy" => &mut config.buy.items,
        "sell" => &mut config.sell.items,
        other => return Err(format!("{other} keeps no item lists")),
    };
    if !lists.lists.contains_key(list) && list != DEFAULT_LIST {
        return Err(format!("{agent} has no list named '{list}'"));
    }
    lists.active = list.into();
    Ok(())
}

/// Runs a job once: organizer, restock, dress, undress, or autoloot.
pub(super) fn agent_run(inner: &mut Inner, args: &Value) -> ToolResult {
    let agent = match agent_arg(args) {
        Ok(a) => a,
        Err(e) => return ToolResult::err(e),
    };
    let list = args
        .get(ARG_LIST)
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let job = match (agent.as_str(), list) {
        ("organizer", Some(list)) => Job::Organize {
            list,
            done: HashSet::new(),
        },
        ("restock", Some(list)) => Job::Restock { list },
        ("dress", list) => Job::Dress {
            list: list.unwrap_or_else(|| TEMP_DRESS_LIST.into()),
        },
        ("undress", list) => Job::Undress { list },
        ("autoloot", _) => Job::LootOnce,
        ("organizer" | "restock", None) => return ToolResult::err(format!("{agent} needs list")),
        (other, _) => {
            return ToolResult::err(format!(
                "'{other}' is not a job; jobs: organizer, restock, dress, undress, autoloot"
            ))
        }
    };
    match inner.agents.start(job) {
        Ok(()) => ToolResult::ok(json!({ "job": agent })),
        Err(e) => ToolResult::err(e),
    }
}

pub(super) fn agent_stop(inner: &mut Inner) -> ToolResult {
    let stopped = inner.agents.job.take().map(|j| j.name());
    ToolResult::ok(json!({ "stopped": stopped }))
}

/// The damage meter: start, pause, resume, stop, or report.
pub(super) fn damage_meter(inner: &mut Inner, args: &Value) -> ToolResult {
    let action = args
        .get(ARG_ACTION)
        .and_then(|v| v.as_str())
        .unwrap_or("report");
    let now = Instant::now();
    let m = &mut inner.agents.meter;
    match action {
        "start" => {
            *m = DamageMeter {
                started: Some(now),
                ..DamageMeter::default()
            }
        }
        "pause" => m.paused_at = m.paused_at.or(Some(now)),
        "resume" => {
            if let Some(at) = m.paused_at.take() {
                m.paused_for += now.saturating_duration_since(at);
            }
        }
        "stop" => m.started = None,
        "report" => {}
        other => {
            return ToolResult::err(format!(
                "'{other}' is not start, pause, resume, stop or report"
            ))
        }
    }
    let running_for = m.started.map_or(Duration::ZERO, |s| {
        now.saturating_duration_since(s)
            .saturating_sub(m.paused_for)
            .saturating_sub(
                m.paused_at
                    .map_or(Duration::ZERO, |p| now.saturating_duration_since(p)),
            )
    });
    let secs = running_for.as_secs_f64().max(f64::MIN_POSITIVE);
    let world = inner.world.read();
    let mut rows: Vec<Value> = m
        .dealt
        .iter()
        .map(|(serial, total)| {
            json!({
                "serial": serial,
                "name": world.name_of(*serial),
                "damage": total,
                "per_second": f64::from(*total) / secs,
            })
        })
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse(r["damage"].as_u64().unwrap_or(0)));
    ToolResult::ok(json!({
        "running": m.started.is_some() && m.paused_at.is_none(),
        "seconds": running_for.as_secs_f64(),
        "mobiles": rows,
    }))
}

/// Picks a mobile with a named target filter.
pub(super) fn work_pick(
    inner: &mut Inner,
    name: &str,
) -> std::result::Result<Option<Serial>, String> {
    work::pick_by_filter(inner, name)
}

/// Picks a mobile with a named target filter and makes it the last target.
pub(super) fn target_filter(inner: &mut Inner, args: &Value) -> ToolResult {
    let Some(name) = args.get("name").and_then(|v| v.as_str()) else {
        return ToolResult::err("target_filter needs name");
    };
    match work::pick_by_filter(inner, name) {
        Ok(Some(serial)) => {
            inner.last_target = Some(serial);
            let who = inner.world.read().name_of(serial);
            ToolResult::ok(json!({ "serial": serial, "name": who }))
        }
        Ok(None) => ToolResult::err("no mobile passes that filter"),
        Err(e) => ToolResult::err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::{armed_session, ready_to_act, PACK};
    use super::config::{HealWhom, ItemLists, ItemRule, PropertyRule, Selector};
    use super::*;
    use uoterm_protocol::{MobileView, NOTO_GREY, NOTO_INNOCENT};

    const ME: Serial = Serial(0x0000_0001);
    const CORPSE: Serial = Serial(0x4000_0D01);
    const LOOT: Serial = Serial(0x4000_0D02);
    const OTHER: Serial = Serial(0x4000_0D03);
    const BAG: Serial = Serial(0x4000_0D04);
    const GOLD: u16 = 0x0EED;
    const RING: u16 = 0x108A;
    const CORPSE_GRAPHIC: u16 = 0x2006;
    const FRIEND: Serial = Serial(0x0000_0200);
    const HUMAN_MALE: u16 = 0x0190;

    fn player() -> Inner {
        let inner = armed_session();
        {
            let mut w = inner.world.write();
            w.logged_in = true;
            w.self_state.serial = ME;
            w.self_state.body = HUMAN_MALE;
            w.self_state.location = Point3::new(100, 100, 0);
            w.self_state.hits = 100;
            w.self_state.hits_max = 100;
            w.self_state.weight = 10;
            w.self_state.weight_max = 400;
        }
        inner
    }

    fn item(inner: &mut Inner, serial: Serial, graphic: u16, parent: Option<Serial>, at: Point3) {
        inner.world.write().items.insert(
            serial,
            uoterm_world::Item {
                serial,
                graphic,
                amount: 10,
                hue: 0,
                location: at,
                parent,
                layer: None,
                grid: 0,
                name: String::new(),
            },
        );
    }

    fn mobile(inner: &mut Inner, serial: Serial, x: u16, noto: u8, hits: Option<(u16, u16)>) {
        inner
            .world
            .write()
            .apply(&Inbound::MobileIncoming(MobileView {
                serial,
                body: 0x0190,
                x,
                y: 100,
                z: 0,
                direction: 0,
                hue: 0,
                flags: 0,
                notoriety: noto,
                equipment: Vec::new(),
                hits: hits.map(|h| h.0),
                hits_max: hits.map(|h| h.1),
            }));
    }

    fn list_of(rules: Vec<ItemRule>) -> ItemLists {
        let mut lists = ItemLists::default();
        lists.lists.insert(DEFAULT_LIST.into(), rules);
        lists
    }

    fn gold_rule() -> ItemRule {
        ItemRule {
            graphic: Some(GOLD),
            ..ItemRule::default()
        }
    }

    /// Runs the agents a tick, as if the last move's delay has passed.
    fn tick(inner: &mut Inner) {
        ready_to_act(inner);
        inner.agents.next_move_at = Instant::now();
        inner.world.write().holding = None;
        pump_agents(inner, Instant::now());
    }

    fn sent(inner: &Inner, packet: &[u8]) -> bool {
        inner.outbound.iter().any(|p| p.as_slice() == packet)
    }

    fn lifted(inner: &Inner, serial: Serial) -> bool {
        inner
            .outbound
            .iter()
            .any(|p| p.first() == Some(&PKT_LIFT) && p[1..5] == serial.0.to_be_bytes())
    }

    #[test]
    fn autoloot_opens_a_corpse_then_takes_what_its_list_wants() {
        let mut inner = player();
        item(
            &mut inner,
            CORPSE,
            CORPSE_GRAPHIC,
            None,
            Point3::new(101, 100, 0),
        );
        item(&mut inner, LOOT, GOLD, Some(CORPSE), Point3::new(0, 0, 0));
        item(&mut inner, OTHER, RING, Some(CORPSE), Point3::new(0, 0, 0));
        inner.agents.config.autoloot.enabled = true;
        inner.agents.config.autoloot.items = list_of(vec![gold_rule()]);
        tick(&mut inner);
        assert!(
            sent(&inner, &encode::double_click(CORPSE)),
            "the corpse is opened first"
        );
        tick(&mut inner);
        assert!(lifted(&inner, LOOT), "the gold is taken");
        tick(&mut inner);
        assert!(!lifted(&inner, OTHER), "the ring is not on the list");
    }

    #[test]
    fn autoloot_asks_for_properties_before_it_judges_an_item() {
        const FASTER_CASTING: u32 = 1_060_413;
        let mut inner = player();
        item(
            &mut inner,
            CORPSE,
            CORPSE_GRAPHIC,
            None,
            Point3::new(101, 100, 0),
        );
        item(&mut inner, LOOT, RING, Some(CORPSE), Point3::new(0, 0, 0));
        let a = &mut inner.agents;
        a.config.autoloot.enabled = true;
        a.config.autoloot.no_open_corpse = true;
        a.config.autoloot.items = list_of(vec![ItemRule {
            graphic: Some(RING),
            properties: vec![PropertyRule {
                name: format!("#{FASTER_CASTING}"),
                min: Some(1.0),
                max: None,
            }],
            ..ItemRule::default()
        }]);
        tick(&mut inner);
        assert!(sent(&inner, &encode::batch_query_properties(&[LOOT])));
        assert!(!lifted(&inner, LOOT), "not before the properties come");
        inner.world.write().apply(&Inbound::ObjectPropertyList {
            serial: LOOT,
            hash: 1,
            properties: vec![uoterm_protocol::ObjectProperty {
                cliloc: FASTER_CASTING,
                arguments: "1".into(),
            }],
        });
        tick(&mut inner);
        assert!(lifted(&inner, LOOT));
    }

    #[test]
    fn a_full_pack_stops_the_looting() {
        let mut inner = player();
        inner.world.write().self_state.weight = 398;
        item(
            &mut inner,
            CORPSE,
            CORPSE_GRAPHIC,
            None,
            Point3::new(101, 100, 0),
        );
        inner.agents.config.autoloot.enabled = true;
        inner.agents.config.autoloot.items = list_of(vec![gold_rule()]);
        tick(&mut inner);
        assert!(inner.outbound.is_empty());
    }

    #[test]
    fn the_scavenger_picks_up_only_what_it_can_reach() {
        let mut inner = player();
        item(&mut inner, LOOT, GOLD, None, Point3::new(102, 100, 0));
        item(&mut inner, OTHER, GOLD, None, Point3::new(110, 100, 0));
        inner.agents.config.scavenger.enabled = true;
        inner.agents.config.scavenger.items = list_of(vec![gold_rule()]);
        tick(&mut inner);
        tick(&mut inner);
        assert!(lifted(&inner, LOOT));
        assert!(!lifted(&inner, OTHER));
    }

    #[test]
    fn the_organizer_moves_its_list_then_finishes() {
        let mut inner = player();
        item(&mut inner, LOOT, GOLD, Some(PACK), Point3::new(0, 0, 0));
        item(&mut inner, OTHER, RING, Some(PACK), Point3::new(0, 0, 0));
        inner.agents.config.organizer.insert(
            "gold".into(),
            MoveList {
                destination: Some(BAG),
                items: vec![gold_rule()],
                ..MoveList::default()
            },
        );
        inner
            .agents
            .start(Job::Organize {
                list: "gold".into(),
                done: HashSet::new(),
            })
            .expect("a list");
        tick(&mut inner);
        assert!(lifted(&inner, LOOT));
        assert!(sent(
            &inner,
            &encode::drop_into_container(LOOT, BAG, drop_grid(&inner))
        ));
        tick(&mut inner);
        assert!(inner.agents.job.is_none(), "nothing more to move");
        assert!(!lifted(&inner, OTHER));
    }

    #[test]
    fn restock_tops_an_item_up_to_its_amount() {
        const BANK: Serial = Serial(0x4000_0D10);
        let mut inner = player();
        item(&mut inner, LOOT, GOLD, Some(BANK), Point3::new(0, 0, 0));
        inner.agents.config.restock.insert(
            "gold".into(),
            MoveList {
                source: Some(BANK),
                items: vec![ItemRule {
                    graphic: Some(GOLD),
                    amount: Some(4),
                    ..ItemRule::default()
                }],
                ..MoveList::default()
            },
        );
        inner
            .agents
            .start(Job::Restock {
                list: "gold".into(),
            })
            .expect("a list");
        tick(&mut inner);
        assert!(
            sent(&inner, &encode::lift(LOOT, 4)),
            "four, not the whole stack"
        );
    }

    #[test]
    fn the_bandage_agent_heals_the_most_hurt_friend() {
        const BANDAGES: Serial = Serial(0x4000_0D20);
        const OTHER_FRIEND: Serial = Serial(0x0000_0201);
        let mut inner = player();
        item(
            &mut inner,
            BANDAGES,
            GRAPHIC_BANDAGE,
            Some(PACK),
            Point3::new(0, 0, 0),
        );
        mobile(&mut inner, FRIEND, 101, NOTO_INNOCENT, Some((20, 100)));
        mobile(
            &mut inner,
            OTHER_FRIEND,
            101,
            NOTO_INNOCENT,
            Some((60, 100)),
        );
        let b = &mut inner.agents.config;
        b.friends.friends = vec![FRIEND, OTHER_FRIEND];
        b.bandage.enabled = true;
        b.bandage.whom = HealWhom::Friend;
        inner.next_bandage_at = Instant::now();
        tick(&mut inner);
        assert!(sent(&inner, &encode::bandage_target(BANDAGES, FRIEND)));
    }

    #[test]
    fn a_character_knocked_off_mounts_again_after_the_delay() {
        const MOUNT: Serial = Serial(0x0000_0300);
        let mut inner = player();
        inner.agents.config.remount.enabled = true;
        inner.agents.config.remount.mount = Some(MOUNT);
        inner.agents.config.remount.delay_ms = 0;
        tick(&mut inner);
        tick(&mut inner);
        assert!(sent(&inner, &encode::double_click(MOUNT)));
    }

    #[test]
    fn the_buy_agent_buys_by_the_shop_order() {
        const VENDOR: Serial = Serial(0x0000_0400);
        const SHOP: Serial = Serial(0x4000_0E00);
        const SHOP_GOLD: Serial = Serial(0x4000_0E01);
        const SHOP_RING: Serial = Serial(0x4000_0E02);
        let mut inner = player();
        inner.world.write().items.insert(
            SHOP,
            uoterm_world::Item {
                serial: SHOP,
                graphic: 0x0E75,
                amount: 1,
                hue: 0,
                location: Point3::new(0, 0, 0),
                parent: Some(VENDOR),
                layer: None,
                grid: 0,
                name: String::new(),
            },
        );
        let stock = |serial, graphic| uoterm_protocol::ContainerItem {
            serial,
            graphic,
            amount: 20,
            x: 0,
            y: 0,
            grid: 0,
            container: SHOP,
            hue: 0,
        };
        inner
            .agents
            .note_contents(&[stock(SHOP_RING, RING), stock(SHOP_GOLD, GOLD)]);
        inner.agents.config.buy.enabled = true;
        inner.agents.config.buy.items = list_of(vec![ItemRule {
            graphic: Some(GOLD),
            amount: Some(3),
            ..ItemRule::default()
        }]);
        let price = |p| uoterm_protocol::VendorBuyEntry {
            price: p,
            description: String::new(),
        };
        on_buy_list(&mut inner, SHOP, &[price(5), price(1)]);
        assert!(sent(&inner, &encode::vendor_buy(VENDOR, &[(SHOP_GOLD, 3)])));
    }

    #[test]
    fn the_sell_agent_sells_up_to_each_amount() {
        const VENDOR: Serial = Serial(0x0000_0400);
        let mut inner = player();
        inner.agents.config.sell.enabled = true;
        inner.agents.config.sell.items = list_of(vec![ItemRule {
            graphic: Some(GOLD),
            amount: Some(7),
            ..ItemRule::default()
        }]);
        let offer = |serial, amount| uoterm_protocol::VendorSellEntry {
            serial,
            graphic: GOLD,
            hue: 0,
            amount,
            price: 1,
            name: String::new(),
        };
        assert!(on_sell_list(
            &mut inner,
            VENDOR,
            &[offer(LOOT, 5), offer(OTHER, 5)]
        ));
        assert!(sent(
            &inner,
            &encode::vendor_sell(VENDOR, &[(LOOT, 5), (OTHER, 2)])
        ));
    }

    #[test]
    fn a_target_filter_picks_the_nearest_grey() {
        const NEAR: Serial = Serial(0x0000_0500);
        const FAR: Serial = Serial(0x0000_0501);
        const BLUE: Serial = Serial(0x0000_0502);
        let mut inner = player();
        mobile(&mut inner, FAR, 108, NOTO_GREY, None);
        mobile(&mut inner, NEAR, 103, NOTO_GREY, None);
        mobile(&mut inner, BLUE, 101, NOTO_INNOCENT, None);
        inner.agents.config.targets.insert(
            "greys".into(),
            TargetFilter {
                notorieties: vec!["gray".into()],
                selector: Selector::Nearest,
                ..TargetFilter::default()
            },
        );
        let result = target_filter(&mut inner, &json!({ "name": "greys" }));
        assert!(result.ok);
        assert_eq!(inner.last_target, Some(NEAR));
    }

    #[test]
    fn a_friend_is_not_attacked_when_the_list_says_so() {
        let mut inner = player();
        inner.agents.config.friends.friends = vec![FRIEND];
        inner.agents.config.friends.prevent_attack = true;
        send_attack(&mut inner, FRIEND);
        assert!(!sent(&inner, &encode::attack(FRIEND)));
    }

    #[test]
    fn an_agent_waits_while_an_item_is_on_the_cursor() {
        let mut inner = player();
        item(&mut inner, LOOT, GOLD, None, Point3::new(101, 100, 0));
        inner.agents.config.scavenger.enabled = true;
        inner.agents.config.scavenger.items = list_of(vec![gold_rule()]);
        inner.world.write().holding = Some(OTHER);
        ready_to_act(&mut inner);
        pump_agents(&mut inner, Instant::now());
        assert!(inner.outbound.is_empty());
    }

    #[test]
    fn settings_set_by_the_tool_read_hex_and_switch_on() {
        let mut inner = player();
        let set = agent_set(
            &mut inner,
            &json!({ "agent": "autoloot", "list": "gems", "settings": [{ "graphic": "0x0F10" }] }),
        );
        assert!(set.ok, "{:?}", set.error);
        let on = agent_on(
            &mut inner,
            &json!({ "agent": "autoloot", "list": "gems", "on": true }),
        );
        assert!(on.ok, "{:?}", on.error);
        let a = &inner.agents.config.autoloot;
        assert!(a.enabled);
        assert_eq!(a.items.rules()[0].graphic, Some(0x0F10));
    }

    #[test]
    fn hex_text_in_settings_becomes_numbers() {
        let v = serials_as_numbers(json!({
            "bag": "0x40001234",
            "lists": { "default": [{ "graphic": "0x0E21", "name": "bandages" }] }
        }));
        assert_eq!(v["bag"], json!(0x4000_1234u64));
        assert_eq!(v["lists"]["default"][0]["graphic"], json!(0x0E21));
        assert_eq!(v["lists"]["default"][0]["name"], json!("bandages"));
    }
}
