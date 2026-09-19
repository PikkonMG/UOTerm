//! Session jobs that run on the tick and hand control back with a reason.
//!
//! The first job is melee hunt: kill, loot corpses this character made, flee
//! when hurt or when a blocked creature fights us. It does not bandage.

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use crate::loot::{LootJob, LootStep};
use crate::reflex::can_be_harmed;
use uoterm_protocol::types::{LAYER_BACKPACK, NOTO_ATTACKABLE};
use uoterm_protocol::{Point3, Serial, FLAG_WAR};
use uoterm_world::{Mobile, World};

pub const JOB_HUNT: &str = "hunt";
pub const JOB_WALK: &str = "walk";

pub const REASON_EMPTY: &str = "empty";
pub const REASON_UNREACHABLE: &str = "unreachable";
pub const REASON_AVOIDED: &str = "avoided";
pub const REASON_DEAD: &str = "dead";
pub const REASON_STOPPED: &str = "stopped";
pub const REASON_ARRIVED: &str = "arrived";
pub const REASON_HOSTILE: &str = "hostile";

pub const JOB_UNKNOWN: &str = "unknown job";
pub const JOB_ALREADY_RUNNING: &str = "that job is already running";
pub const JOB_NONE_RUNNING: &str = "no job is running";
pub const JOB_TERM_NEEDS_AXIS: &str =
    "each include or avoid term must start with species:, name:, graphic:, or any:";
pub const JOB_LISTS_TOGETHER: &str = "pass include or avoid, not both";
pub const JOB_NEEDS_NAME: &str = "job_start needs job";
pub const JOB_WALK_NEEDS_SPOT: &str = "walk needs x and y, or a place name";

const ACQUIRE_RANGE: u16 = 10;
const FLEE_HP_PCT: u32 = 45;
const RESUME_HP_PCT: u32 = 80;
const SWARM_COUNT: usize = 4;
const SWARM_RANGE: u32 = 2;
const SWARM_HP_PCT: u32 = 60;
const AGGRO_RANGE: u32 = 12;
const AVOID_IDLE_RANGE: u32 = 2;
const HANDOFF_CLEAR: Duration = Duration::from_secs(15);
const FLEE_DISTANCE: u16 = 15;
const HP_FLEE_DISTANCE: u16 = 25;
const MELEE_TILES: u32 = 1;
const NOPATH_STRIKES: u8 = 2;
const BLACKLIST: Duration = Duration::from_secs(5 * 60);
const CORPSE_WAIT: Duration = Duration::from_secs(3);
const PERCENT: u32 = 100;
const MAX_AGGRO_TO_LOOT: usize = 2;
const LIFT_AT_LEAST: u16 = 1;
pub const GRAPHIC_CORPSE: u16 = 0x2006;
const WALK_STOP_RANGE: u32 = 15;
const WALK_ARRIVE: u32 = 1;

const PREFIX_SPECIES: &str = "species:";
const PREFIX_NAME: &str = "name:";
const PREFIX_GRAPHIC: &str = "graphic:";
const PREFIX_ANY: &str = "any:";
const ARTICLE_A: &str = "a ";
const ARTICLE_AN: &str = "an ";
const ARTICLE_THE: &str = "the ";

#[derive(Clone, Debug, PartialEq, Eq)]
enum Axis {
    Species,
    Name,
    Graphic(u16),
    Any,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchTerm {
    axis: Axis,
    value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HuntLists {
    pub include: Vec<MatchTerm>,
    pub avoid: Vec<MatchTerm>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HuntPhase {
    Kill,
    Loot,
    Escape,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EscapeKind {
    Hp,
    Avoid,
    Swarm,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HuntAction {
    None,
    WarOn,
    Attack(Serial),
    MoveTo { x: u16, y: u16, z: i8 },
    Open(Serial),
    Lift { serial: Serial, amount: u16 },
    Drop { serial: Serial, dest: Serial },
    End(&'static str),
}

#[derive(Clone, Debug)]
pub struct HuntJob {
    pub lists: HuntLists,
    phase: HuntPhase,
    escape: Option<EscapeKind>,
    target: Option<Serial>,
    last_target_at: Option<Point3>,
    pending_corpse_at: Option<(Point3, Instant)>,
    corpses: VecDeque<Serial>,
    looted: HashSet<Serial>,
    loot: Option<LootJob>,
    avoided: Option<Serial>,
    avoided_at: Option<Point3>,
    avoided_gone_since: Option<Instant>,
    nopath: HashMap<Serial, u8>,
    blacklist_until: HashMap<Serial, Instant>,
    seen_wanted: bool,
}

impl HuntLists {
    pub fn parse(include: &[String], avoid: &[String]) -> Result<Self, &'static str> {
        if !include.is_empty() && !avoid.is_empty() {
            return Err(JOB_LISTS_TOGETHER);
        }
        Ok(Self {
            include: parse_terms(include)?,
            avoid: parse_terms(avoid)?,
        })
    }

    pub fn as_strings(terms: &[MatchTerm]) -> Vec<String> {
        terms.iter().map(MatchTerm::display).collect()
    }
}

impl MatchTerm {
    fn display(&self) -> String {
        match self.axis {
            Axis::Species => format!("{PREFIX_SPECIES}{}", self.value),
            Axis::Name => format!("{PREFIX_NAME}{}", self.value),
            Axis::Graphic(n) => format!("{PREFIX_GRAPHIC}{n}"),
            Axis::Any => format!("{PREFIX_ANY}{}", self.value),
        }
    }
}

pub fn parse_term(raw: &str) -> Result<MatchTerm, &'static str> {
    let raw = raw.trim();
    let lower = raw.to_ascii_lowercase();
    if let Some(rest) = strip_prefix_ci(&lower, PREFIX_SPECIES) {
        return Ok(MatchTerm {
            axis: Axis::Species,
            value: rest.to_string(),
        });
    }
    if let Some(rest) = strip_prefix_ci(&lower, PREFIX_NAME) {
        return Ok(MatchTerm {
            axis: Axis::Name,
            value: rest.to_string(),
        });
    }
    if let Some(rest) = strip_prefix_ci(&lower, PREFIX_GRAPHIC) {
        let graphic = parse_graphic(rest).ok_or(JOB_TERM_NEEDS_AXIS)?;
        return Ok(MatchTerm {
            axis: Axis::Graphic(graphic),
            value: rest.to_string(),
        });
    }
    if let Some(rest) = strip_prefix_ci(&lower, PREFIX_ANY) {
        return Ok(MatchTerm {
            axis: Axis::Any,
            value: rest.to_string(),
        });
    }
    Err(JOB_TERM_NEEDS_AXIS)
}

fn parse_terms(raw: &[String]) -> Result<Vec<MatchTerm>, &'static str> {
    raw.iter().map(|s| parse_term(s)).collect()
}

fn strip_prefix_ci<'a>(lower: &'a str, prefix: &str) -> Option<&'a str> {
    lower.strip_prefix(prefix)
}

fn parse_graphic(s: &str) -> Option<u16> {
    if let Some(hex) = s.strip_prefix("0x") {
        u16::from_str_radix(hex, 16).ok()
    } else {
        s.parse().ok()
    }
}

/// The species word for a mobile: the name without a leading article.
pub fn species_of(name: &str) -> String {
    let n = name.trim().to_ascii_lowercase();
    let n = n
        .strip_prefix(ARTICLE_A)
        .or_else(|| n.strip_prefix(ARTICLE_AN))
        .or_else(|| n.strip_prefix(ARTICLE_THE))
        .unwrap_or(&n);
    n.trim().to_string()
}

fn term_matches(term: &MatchTerm, name: &str, species: &str, body: u16) -> bool {
    match term.axis {
        Axis::Species => species == term.value,
        Axis::Name => name.to_ascii_lowercase() == term.value,
        Axis::Graphic(g) => body == g,
        Axis::Any => {
            name.to_ascii_lowercase().contains(&term.value) || species.contains(&term.value)
        }
    }
}

fn listed(terms: &[MatchTerm], mobile: &Mobile) -> bool {
    let name = mobile.name.to_ascii_lowercase();
    let species = species_of(&mobile.name);
    terms
        .iter()
        .any(|t| term_matches(t, &name, &species, mobile.body))
}

fn is_threat(mobile: &Mobile) -> bool {
    can_be_harmed(mobile.notoriety, mobile.flags) && NOTO_ATTACKABLE.contains(&mobile.notoriety)
}

fn is_blocked(lists: &HuntLists, mobile: &Mobile) -> bool {
    if !lists.include.is_empty() {
        return !listed(&lists.include, mobile);
    }
    if !lists.avoid.is_empty() {
        return listed(&lists.avoid, mobile);
    }
    false
}

fn hp_pct(cur: u16, max: u16) -> u32 {
    if max == 0 {
        PERCENT
    } else {
        u32::from(cur) * PERCENT / u32::from(max)
    }
}

fn pack_serial(world: &World) -> Serial {
    world
        .self_state
        .equipment
        .iter()
        .find(|item| item.layer == LAYER_BACKPACK)
        .map(|item| item.serial)
        .unwrap_or(world.self_state.serial)
}

fn corpse_near(world: &World, at: Point3) -> Option<Serial> {
    world.items.values().find_map(|item| {
        if item.parent.is_some() || item.graphic != GRAPHIC_CORPSE {
            return None;
        }
        (item.location.chebyshev(at) <= MELEE_TILES).then_some(item.serial)
    })
}

fn away_from(me: Point3, threat: Point3, dist: u16) -> Point3 {
    let mut dx = i32::from(me.x) - i32::from(threat.x);
    let dy = i32::from(me.y) - i32::from(threat.y);
    if dx == 0 && dy == 0 {
        dx = 1;
    }
    let span = dx.abs().max(dy.abs()).max(1);
    let scale = i32::from(dist);
    let nx = dx * scale / span;
    let ny = dy * scale / span;
    Point3::new(
        (i32::from(me.x) + nx).clamp(0, i32::from(u16::MAX)) as u16,
        (i32::from(me.y) + ny).clamp(0, i32::from(u16::MAX)) as u16,
        me.z,
    )
}

impl HuntJob {
    pub fn new(lists: HuntLists) -> Self {
        Self {
            lists,
            phase: HuntPhase::Kill,
            escape: None,
            target: None,
            last_target_at: None,
            pending_corpse_at: None,
            corpses: VecDeque::new(),
            looted: HashSet::new(),
            loot: None,
            avoided: None,
            avoided_at: None,
            avoided_gone_since: None,
            nopath: HashMap::new(),
            blacklist_until: HashMap::new(),
            seen_wanted: false,
        }
    }

    pub fn phase_name(&self) -> &'static str {
        match self.phase {
            HuntPhase::Kill => "kill",
            HuntPhase::Loot => "loot",
            HuntPhase::Escape => "escape",
        }
    }

    pub fn target(&self) -> Option<Serial> {
        self.target
    }

    pub fn note_nopath(&mut self, serial: Serial, now: Instant) {
        let strikes = self.nopath.entry(serial).or_insert(0);
        *strikes = strikes.saturating_add(1);
        if *strikes >= NOPATH_STRIKES {
            self.blacklist_until.insert(serial, now + BLACKLIST);
            if self.target == Some(serial) {
                self.target = None;
            }
        }
    }

    pub fn tick(&mut self, world: &World, action_ready: bool, now: Instant) -> HuntAction {
        if world.self_state.dead {
            return HuntAction::End(REASON_DEAD);
        }
        self.expire_blacklist(now);
        self.note_vanished_target(world, now);
        self.collect_pending_corpse(world, now);
        self.choose_phase(world, now);
        match self.phase {
            HuntPhase::Escape => self.tick_escape(world, now),
            HuntPhase::Loot => self.tick_loot(world, action_ready, now),
            HuntPhase::Kill => self.tick_kill(world, now),
        }
    }

    fn expire_blacklist(&mut self, now: Instant) {
        self.blacklist_until.retain(|_, until| now < *until);
    }

    fn blacklisted(&self, serial: Serial, now: Instant) -> bool {
        self.blacklist_until
            .get(&serial)
            .is_some_and(|until| now < *until)
    }

    fn note_vanished_target(&mut self, world: &World, now: Instant) {
        let Some(serial) = self.target else {
            return;
        };
        if world.mobiles.contains_key(&serial) {
            if let Some(mobile) = world.mobiles.get(&serial) {
                self.last_target_at = Some(mobile.location);
            }
            return;
        }
        self.target = None;
        if let Some(at) = self.last_target_at {
            self.pending_corpse_at = Some((at, now));
        }
    }

    fn collect_pending_corpse(&mut self, world: &World, now: Instant) {
        let Some((at, since)) = self.pending_corpse_at else {
            return;
        };
        if let Some(serial) = corpse_near(world, at) {
            if !self.looted.contains(&serial) && !self.corpses.contains(&serial) {
                self.corpses.push_back(serial);
            }
            self.pending_corpse_at = None;
            return;
        }
        if now.saturating_duration_since(since) >= CORPSE_WAIT {
            self.pending_corpse_at = None;
        }
    }

    fn choose_phase(&mut self, world: &World, now: Instant) {
        let here = world.self_state.location;
        let hp = hp_pct(world.self_state.hits, world.self_state.hits_max);
        let blocked_aggro = self.nearest_blocked_aggro(world);
        let swarm = self.threats_within(world, SWARM_RANGE).len() >= SWARM_COUNT;
        if hp < FLEE_HP_PCT {
            self.enter_escape(EscapeKind::Hp, blocked_aggro);
            return;
        }
        if let Some(mobile) = blocked_aggro {
            self.enter_escape(EscapeKind::Avoid, Some(mobile));
            return;
        }
        if swarm && hp <= SWARM_HP_PCT {
            self.enter_escape(EscapeKind::Swarm, None);
            return;
        }
        if self.escape == Some(EscapeKind::Hp) && hp < RESUME_HP_PCT {
            self.phase = HuntPhase::Escape;
            return;
        }
        if self.escape == Some(EscapeKind::Avoid) {
            self.phase = HuntPhase::Escape;
            let gone = blocked_aggro.is_none()
                && self
                    .avoided
                    .is_none_or(|serial| !world.mobiles.contains_key(&serial));
            if gone {
                if self.avoided_gone_since.is_none() {
                    self.avoided_gone_since = Some(now);
                }
            } else {
                self.avoided_gone_since = None;
            }
            return;
        }
        self.escape = None;
        let aggro = self.aggro_count(world, here);
        if (self.loot.is_some() || !self.corpses.is_empty()) && aggro < MAX_AGGRO_TO_LOOT {
            self.phase = HuntPhase::Loot;
            return;
        }
        self.phase = HuntPhase::Kill;
    }

    fn enter_escape(&mut self, kind: EscapeKind, from: Option<&Mobile>) {
        self.phase = HuntPhase::Escape;
        self.escape = Some(kind);
        if kind == EscapeKind::Avoid {
            if let Some(mobile) = from {
                self.avoided = Some(mobile.serial);
                self.avoided_at = Some(mobile.location);
            }
            self.avoided_gone_since = None;
        }
    }

    fn tick_escape(&mut self, world: &World, now: Instant) -> HuntAction {
        let here = world.self_state.location;
        let hp = hp_pct(world.self_state.hits, world.self_state.hits_max);
        if self.escape == Some(EscapeKind::Avoid) {
            if let Some(since) = self.avoided_gone_since {
                if now.saturating_duration_since(since) >= HANDOFF_CLEAR && hp >= RESUME_HP_PCT {
                    return HuntAction::End(REASON_AVOIDED);
                }
            }
        }
        let threat_at = self
            .nearest_blocked_aggro(world)
            .map(|m| m.location)
            .or(self.avoided_at)
            .or_else(|| self.nearest_threat(world).map(|m| m.location));
        let dist = if self.escape == Some(EscapeKind::Hp) {
            HP_FLEE_DISTANCE
        } else {
            FLEE_DISTANCE
        };
        let dest = match threat_at {
            Some(at) => away_from(here, at, dist),
            None => Point3::new(here.x.saturating_sub(dist), here.y, here.z),
        };
        HuntAction::MoveTo {
            x: dest.x,
            y: dest.y,
            z: dest.z,
        }
    }

    fn tick_loot(&mut self, world: &World, action_ready: bool, now: Instant) -> HuntAction {
        if self.loot.is_none() {
            while let Some(corpse) = self.corpses.pop_front() {
                if self.looted.contains(&corpse) || !world.items.contains_key(&corpse) {
                    continue;
                }
                self.loot = Some(LootJob::new(corpse, pack_serial(world), now));
                break;
            }
        }
        let Some(job) = self.loot.clone() else {
            self.phase = HuntPhase::Kill;
            return self.tick_kill(world, now);
        };
        match job.step(world, action_ready, now) {
            LootStep::Walk { x, y, z } => HuntAction::MoveTo { x, y, z },
            LootStep::Open(serial) => HuntAction::Open(serial),
            LootStep::Lift { serial, amount } => {
                if let Some(loot) = self.loot.as_mut() {
                    loot.limits.note_lift(serial);
                }
                HuntAction::Lift {
                    serial,
                    amount: amount.max(LIFT_AT_LEAST),
                }
            }
            LootStep::Drop { serial, dest } => HuntAction::Drop { serial, dest },
            LootStep::Wait => HuntAction::None,
            LootStep::Done | LootStep::Fail(_) => {
                if let Some(loot) = self.loot.take() {
                    self.looted.insert(loot.corpse);
                }
                HuntAction::None
            }
        }
    }

    fn tick_kill(&mut self, world: &World, now: Instant) -> HuntAction {
        let wanted = self.wanted(world, now);
        if wanted.is_empty() {
            if !self.seen_wanted {
                return HuntAction::None;
            }
            let in_range = self.allowed_in_acquire(world);
            if in_range.iter().any(|m| self.blacklisted(m.serial, now)) {
                return HuntAction::End(REASON_UNREACHABLE);
            }
            return HuntAction::End(REASON_EMPTY);
        }
        self.seen_wanted = true;
        let target = self.lock_target(world, &wanted);
        let Some(mobile) = world.mobiles.get(&target) else {
            return HuntAction::None;
        };
        if !world.self_state.war {
            return HuntAction::WarOn;
        }
        let dist = world.self_state.location.chebyshev(mobile.location);
        if dist > MELEE_TILES {
            return HuntAction::MoveTo {
                x: mobile.location.x,
                y: mobile.location.y,
                z: mobile.location.z,
            };
        }
        if world.combatant != Some(target) {
            return HuntAction::Attack(target);
        }
        HuntAction::None
    }

    fn lock_target(&mut self, world: &World, wanted: &[&Mobile]) -> Serial {
        if let Some(serial) = self.target {
            if wanted.iter().any(|m| m.serial == serial) {
                return serial;
            }
        }
        let mut sorted = wanted.to_vec();
        sorted.sort_by_key(|m| (world.self_state.location.chebyshev(m.location), m.serial.0));
        let serial = sorted[0].serial;
        self.target = Some(serial);
        self.last_target_at = Some(sorted[0].location);
        serial
    }

    fn wanted<'w>(&self, world: &'w World, now: Instant) -> Vec<&'w Mobile> {
        self.allowed_in_acquire(world)
            .into_iter()
            .filter(|m| !self.blacklisted(m.serial, now))
            .collect()
    }

    fn allowed_in_acquire<'w>(&self, world: &'w World) -> Vec<&'w Mobile> {
        let here = world.self_state.location;
        world
            .mobiles
            .values()
            .filter(|m| m.serial != world.self_state.serial)
            .filter(|m| is_threat(m) && !is_blocked(&self.lists, m))
            .filter(|m| here.chebyshev(m.location) <= u32::from(ACQUIRE_RANGE))
            .collect()
    }

    fn nearest_blocked_aggro<'w>(&self, world: &'w World) -> Option<&'w Mobile> {
        let here = world.self_state.location;
        world
            .mobiles
            .values()
            .filter(|m| m.serial != world.self_state.serial)
            .filter(|m| is_threat(m) && is_blocked(&self.lists, m))
            .filter(|m| {
                let dist = here.chebyshev(m.location);
                let war = m.flags & FLAG_WAR != 0;
                (war && dist <= AGGRO_RANGE) || dist <= AVOID_IDLE_RANGE
            })
            .min_by_key(|m| here.chebyshev(m.location))
    }

    fn nearest_threat<'w>(&self, world: &'w World) -> Option<&'w Mobile> {
        let here = world.self_state.location;
        world
            .mobiles
            .values()
            .filter(|m| m.serial != world.self_state.serial)
            .filter(|m| is_threat(m))
            .min_by_key(|m| here.chebyshev(m.location))
    }

    fn threats_within<'w>(&self, world: &'w World, range: u32) -> Vec<&'w Mobile> {
        let here = world.self_state.location;
        world
            .mobiles
            .values()
            .filter(|m| m.serial != world.self_state.serial)
            .filter(|m| is_threat(m))
            .filter(|m| here.chebyshev(m.location) <= range)
            .collect()
    }

    fn aggro_count(&self, world: &World, here: Point3) -> usize {
        world
            .mobiles
            .values()
            .filter(|m| m.serial != world.self_state.serial)
            .filter(|m| is_threat(m) && m.flags & FLAG_WAR != 0)
            .filter(|m| here.chebyshev(m.location) <= AGGRO_RANGE)
            .count()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WalkPhase {
    Walk,
    Watch,
    Flee,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WalkAction {
    None,
    MoveTo { x: u16, y: u16, z: i8 },
    Hold,
    End(&'static str),
}

#[derive(Clone, Debug)]
pub struct WalkJob {
    pub dest: Point3,
    pub watch: bool,
    phase: WalkPhase,
}

impl WalkJob {
    pub fn new(dest: Point3, watch: bool) -> Self {
        Self {
            dest,
            watch,
            phase: WalkPhase::Walk,
        }
    }

    pub fn phase_name(&self) -> &'static str {
        match self.phase {
            WalkPhase::Walk => "walk",
            WalkPhase::Watch => "watch",
            WalkPhase::Flee => "flee",
        }
    }

    pub fn fleeing(&self) -> bool {
        self.phase == WalkPhase::Flee
    }

    pub fn tick(&mut self, world: &World) -> WalkAction {
        if world.self_state.dead {
            return WalkAction::End(REASON_DEAD);
        }
        let here = world.self_state.location;
        if let Some(threat) = nearest_walk_threat(world, WALK_STOP_RANGE) {
            self.phase = WalkPhase::Flee;
            let dest = away_from(here, threat.location, FLEE_DISTANCE);
            return WalkAction::MoveTo {
                x: dest.x,
                y: dest.y,
                z: dest.z,
            };
        }
        if self.phase == WalkPhase::Flee {
            return WalkAction::End(REASON_HOSTILE);
        }
        if here.chebyshev(self.dest) <= WALK_ARRIVE {
            if self.watch {
                self.phase = WalkPhase::Watch;
                return WalkAction::Hold;
            }
            return WalkAction::End(REASON_ARRIVED);
        }
        self.phase = WalkPhase::Walk;
        WalkAction::MoveTo {
            x: self.dest.x,
            y: self.dest.y,
            z: self.dest.z,
        }
    }
}

fn nearest_walk_threat(world: &World, range: u32) -> Option<&Mobile> {
    let here = world.self_state.location;
    world
        .mobiles
        .values()
        .filter(|m| m.serial != world.self_state.serial)
        .filter(|m| is_threat(m))
        .filter(|m| here.chebyshev(m.location) <= range)
        .min_by_key(|m| here.chebyshev(m.location))
}

fn string_args(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub fn lists_from_args(args: &serde_json::Value) -> Result<HuntLists, &'static str> {
    HuntLists::parse(
        &string_args(args.get("include")),
        &string_args(args.get("avoid")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_protocol::{
        ContainerItem, EquipItem, GroundItem, Inbound, GRAPHIC_BACKPACK, NOTO_GREY, NOTO_INNOCENT,
    };

    const SELF: Serial = Serial(0x0000_00AB);
    const ZOMBIE: Serial = Serial(0x0000_1234);
    const CASTER: Serial = Serial(0x0000_1235);
    const BACKPACK: Serial = Serial(0x4000_0100);
    const CORPSE: Serial = Serial(0x4000_0200);
    const GOLD: Serial = Serial(0x4000_0201);
    const GRAPHIC_GOLD: u16 = 0x0EED;
    const BODY_ZOMBIE: u16 = 3;
    const BODY_LICH: u16 = 24;
    const HERE_X: u16 = 100;
    const HERE_Y: u16 = 100;
    const NEXT_X: u16 = 101;
    const FAR_X: u16 = 108;
    const FULL: u16 = 100;
    const LOW: u16 = 40;
    const ONE: u16 = 1;
    const CORPSE_GUMP: u16 = 9;
    const READY: bool = true;

    fn base_world() -> World {
        let mut w = World::new();
        w.logged_in = true;
        w.self_state.serial = SELF;
        w.self_state.location = Point3::new(HERE_X, HERE_Y, 0);
        w.self_state.hits = FULL;
        w.self_state.hits_max = FULL;
        w.self_state.war = false;
        w.apply(&Inbound::Equipped {
            owner: SELF,
            item: EquipItem {
                serial: BACKPACK,
                graphic: GRAPHIC_BACKPACK,
                layer: LAYER_BACKPACK,
                hue: 0,
            },
        });
        w
    }

    fn put_mobile(
        w: &mut World,
        serial: Serial,
        name: &str,
        body: u16,
        x: u16,
        noto: u8,
        flags: u8,
    ) {
        w.mobiles.insert(
            serial,
            Mobile {
                serial,
                name: name.into(),
                title: String::new(),
                body,
                hue: 0,
                location: Point3::new(x, HERE_Y, 0),
                direction: 0,
                running: false,
                notoriety: noto,
                flags,
                hits: Some(FULL),
                hits_max: Some(FULL),
                equipment: Vec::new(),
            },
        );
    }

    fn hunt(include: &[&str], avoid: &[&str]) -> HuntJob {
        let include: Vec<String> = include.iter().map(|s| (*s).to_string()).collect();
        let avoid: Vec<String> = avoid.iter().map(|s| (*s).to_string()).collect();
        HuntJob::new(HuntLists::parse(&include, &avoid).unwrap())
    }

    #[test]
    fn a_term_must_name_its_axis() {
        assert_eq!(parse_term("zombie"), Err(JOB_TERM_NEEDS_AXIS));
        assert!(parse_term("species:zombie").is_ok());
        assert!(parse_term("graphic:0x3").is_ok());
        assert_eq!(parse_term("graphic:3").unwrap().axis, Axis::Graphic(3));
    }

    #[test]
    fn species_strips_the_article() {
        assert_eq!(species_of("a zombie"), "zombie");
        assert_eq!(species_of("an orc"), "orc");
        assert_eq!(species_of("the lich"), "lich");
    }

    #[test]
    fn include_and_avoid_together_are_refused() {
        let include = vec!["species:zombie".into()];
        let avoid = vec!["species:lich".into()];
        assert_eq!(HuntLists::parse(&include, &avoid), Err(JOB_LISTS_TOGETHER));
    }

    #[test]
    fn empty_ground_ends_the_job() {
        let mut w = base_world();
        put_mobile(
            &mut w,
            ZOMBIE,
            "a zombie",
            BODY_ZOMBIE,
            NEXT_X,
            NOTO_GREY,
            0,
        );
        w.self_state.war = true;
        let mut job = hunt(&[], &[]);
        assert_eq!(
            job.tick(&w, READY, Instant::now()),
            HuntAction::Attack(ZOMBIE)
        );
        w.mobiles.remove(&ZOMBIE);
        assert_eq!(
            job.tick(&w, READY, Instant::now()),
            HuntAction::End(REASON_EMPTY)
        );
    }

    #[test]
    fn hunt_waits_when_no_target_has_been_seen() {
        let w = base_world();
        let mut job = hunt(&[], &[]);
        assert_eq!(job.tick(&w, READY, Instant::now()), HuntAction::None);
    }

    #[test]
    fn hunt_turns_war_on_then_attacks() {
        let mut w = base_world();
        put_mobile(
            &mut w,
            ZOMBIE,
            "a zombie",
            BODY_ZOMBIE,
            NEXT_X,
            NOTO_GREY,
            0,
        );
        let mut job = hunt(&[], &[]);
        assert_eq!(job.tick(&w, READY, Instant::now()), HuntAction::WarOn);
        w.self_state.war = true;
        assert_eq!(
            job.tick(&w, READY, Instant::now()),
            HuntAction::Attack(ZOMBIE)
        );
    }

    #[test]
    fn hunt_walks_into_melee_range() {
        let mut w = base_world();
        put_mobile(&mut w, ZOMBIE, "a zombie", BODY_ZOMBIE, FAR_X, NOTO_GREY, 0);
        w.self_state.war = true;
        let mut job = hunt(&[], &[]);
        match job.tick(&w, READY, Instant::now()) {
            HuntAction::MoveTo { x, y, .. } => {
                assert_eq!(x, FAR_X);
                assert_eq!(y, HERE_Y);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn include_fights_only_the_listed_species() {
        let mut w = base_world();
        put_mobile(&mut w, CASTER, "a lich", BODY_LICH, FAR_X, NOTO_GREY, 0);
        let mut job = hunt(&["species:zombie"], &[]);
        assert_eq!(job.tick(&w, READY, Instant::now()), HuntAction::None);
        put_mobile(
            &mut w,
            ZOMBIE,
            "a zombie",
            BODY_ZOMBIE,
            NEXT_X,
            NOTO_GREY,
            0,
        );
        w.self_state.war = true;
        assert_eq!(
            job.tick(&w, READY, Instant::now()),
            HuntAction::Attack(ZOMBIE)
        );
    }

    #[test]
    fn an_innocent_is_not_a_hunt_target() {
        let mut w = base_world();
        put_mobile(
            &mut w,
            ZOMBIE,
            "a guard",
            BODY_ZOMBIE,
            NEXT_X,
            NOTO_INNOCENT,
            0,
        );
        let mut job = hunt(&[], &[]);
        assert_eq!(job.tick(&w, READY, Instant::now()), HuntAction::None);
    }

    #[test]
    fn low_health_flees_instead_of_attacking() {
        let mut w = base_world();
        put_mobile(
            &mut w,
            ZOMBIE,
            "a zombie",
            BODY_ZOMBIE,
            NEXT_X,
            NOTO_GREY,
            0,
        );
        w.self_state.hits = LOW;
        w.self_state.war = true;
        let mut job = hunt(&[], &[]);
        match job.tick(&w, READY, Instant::now()) {
            HuntAction::MoveTo { .. } => {}
            other => panic!("expected flee, got {other:?}"),
        }
        assert_eq!(job.phase, HuntPhase::Escape);
    }

    #[test]
    fn an_avoided_creature_in_war_hands_back_when_clear() {
        let mut w = base_world();
        put_mobile(
            &mut w, CASTER, "a lich", BODY_LICH, NEXT_X, NOTO_GREY, FLAG_WAR,
        );
        w.self_state.war = true;
        let mut job = hunt(&[], &["species:lich"]);
        match job.tick(&w, READY, Instant::now()) {
            HuntAction::MoveTo { .. } => {}
            other => panic!("{other:?}"),
        }
        w.mobiles.remove(&CASTER);
        let start = Instant::now();
        job.tick(&w, READY, start);
        assert_eq!(
            job.tick(&w, READY, start + HANDOFF_CLEAR),
            HuntAction::End(REASON_AVOIDED)
        );
    }

    #[test]
    fn a_vanished_target_queues_its_corpse_and_opens_it() {
        let mut w = base_world();
        put_mobile(
            &mut w,
            ZOMBIE,
            "a zombie",
            BODY_ZOMBIE,
            NEXT_X,
            NOTO_GREY,
            0,
        );
        w.self_state.war = true;
        w.combatant = Some(ZOMBIE);
        let mut job = hunt(&[], &[]);
        let _ = job.tick(&w, READY, Instant::now());
        w.mobiles.remove(&ZOMBIE);
        w.combatant = None;
        w.apply(&Inbound::WorldItem(GroundItem {
            serial: CORPSE,
            graphic: GRAPHIC_CORPSE,
            amount: ONE,
            x: NEXT_X,
            y: HERE_Y,
            z: 0,
            hue: 0,
            multi: false,
        }));
        let now = Instant::now();
        let _ = job.tick(&w, READY, now);
        assert_eq!(job.tick(&w, READY, now), HuntAction::Open(CORPSE));
    }

    #[test]
    fn loot_lifts_from_a_kill_corpse() {
        let mut w = base_world();
        w.self_state.war = true;
        w.apply(&Inbound::WorldItem(GroundItem {
            serial: CORPSE,
            graphic: GRAPHIC_CORPSE,
            amount: ONE,
            x: HERE_X,
            y: HERE_Y,
            z: 0,
            hue: 0,
            multi: false,
        }));
        w.apply(&Inbound::OpenContainer {
            serial: CORPSE,
            gump: CORPSE_GUMP,
        });
        w.apply(&Inbound::ContainerContents {
            items: vec![ContainerItem {
                serial: GOLD,
                graphic: GRAPHIC_GOLD,
                amount: ONE,
                x: 0,
                y: 0,
                grid: 0,
                container: CORPSE,
                hue: 0,
            }],
        });
        let mut job = hunt(&[], &[]);
        job.corpses.push_back(CORPSE);
        match job.tick(&w, READY, Instant::now()) {
            HuntAction::Lift { serial, amount } => {
                assert_eq!(serial, GOLD);
                assert_eq!(amount, ONE);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn two_nopath_strikes_make_spawn_unreachable() {
        let mut w = base_world();
        put_mobile(
            &mut w,
            ZOMBIE,
            "a zombie",
            BODY_ZOMBIE,
            NEXT_X,
            NOTO_GREY,
            0,
        );
        w.self_state.war = true;
        let mut job = hunt(&[], &[]);
        let now = Instant::now();
        assert_eq!(job.tick(&w, READY, now), HuntAction::Attack(ZOMBIE));
        job.note_nopath(ZOMBIE, now);
        job.note_nopath(ZOMBIE, now);
        assert_eq!(
            job.tick(&w, READY, now),
            HuntAction::End(REASON_UNREACHABLE)
        );
    }

    const DEST_X: u16 = 120;
    const DEST_Y: u16 = 100;

    fn walk_to(x: u16, y: u16, watch: bool) -> WalkJob {
        WalkJob::new(Point3::new(x, y, 0), watch)
    }

    #[test]
    fn walk_moves_toward_the_spot() {
        let w = base_world();
        let mut job = walk_to(DEST_X, DEST_Y, false);
        match job.tick(&w) {
            WalkAction::MoveTo { x, y, .. } => {
                assert_eq!(x, DEST_X);
                assert_eq!(y, DEST_Y);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn walk_ends_when_it_arrives() {
        let w = base_world();
        let mut job = walk_to(HERE_X, HERE_Y, false);
        assert_eq!(job.tick(&w), WalkAction::End(REASON_ARRIVED));
    }

    #[test]
    fn walk_watch_holds_after_arrival() {
        let w = base_world();
        let mut job = walk_to(HERE_X, HERE_Y, true);
        assert_eq!(job.tick(&w), WalkAction::Hold);
        assert_eq!(job.phase_name(), "watch");
    }

    #[test]
    fn walk_flees_a_hostile_then_hands_back() {
        let mut w = base_world();
        put_mobile(
            &mut w,
            ZOMBIE,
            "a zombie",
            BODY_ZOMBIE,
            NEXT_X,
            NOTO_GREY,
            0,
        );
        let mut job = walk_to(DEST_X, DEST_Y, false);
        match job.tick(&w) {
            WalkAction::MoveTo { .. } => {}
            other => panic!("{other:?}"),
        }
        w.mobiles.remove(&ZOMBIE);
        assert!(job.fleeing());
        assert_eq!(job.tick(&w), WalkAction::End(REASON_HOSTILE));
    }
}
