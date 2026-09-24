//! Awareness: what an agent must not miss while it drives the character.
//!
//! An agent that polls `observe` compares snapshots and misses what happens
//! between them. `next_event` waits until something important happens and
//! gives it, with a short state block, so any agent can run one loop:
//! wait, decide, act. Events wait in the world's event log, so an agent that
//! is slow gets them all, in order, the next time it asks.

use std::collections::{HashMap, HashSet};

use uoterm_world::{Event, EventKind};

use super::*;

/// Health under this share of the maximum is low.
const LOW_HEALTH_PCT: u32 = 50;
const PERCENT: u32 = 100;
/// A mobile the character may fight counts as near within this many tiles.
const ENEMY_NEAR_TILES: u32 = 10;
/// How long an enemy_near for one creature is not raised again after it fires.
/// A creature that drifts in and out across the near edge, as a swarm does,
/// then fires once and not on every crossing, so a dense room cannot flood the
/// event log with the same creatures over and over.
const ENEMY_NEAR_COOLDOWN: Duration = Duration::from_secs(10);
/// The most events one `next_event` answer carries. The rest wait for the
/// next call.
const NEXT_EVENT_BATCH: usize = 50;
/// The most near enemies the state block lists, nearest first.
const STATE_ENEMIES: usize = 5;
/// The argument that names the ambient kinds a caller wants too, and the
/// word for all of them.
const ARG_AMBIENT: &str = "ambient";
const AMBIENT_ALL: &str = "all";
const BAD_AMBIENT: &str =
    "ambient lists sound, effect, animation, item_deleted, member_positions or all";

/// What the awareness check saw at the last tick, to tell what is new.
#[derive(Default)]
pub(super) struct Awareness {
    low_health: bool,
    /// The enemies near at the last tick, to fire only on the crossing in.
    enemies_near: HashSet<Serial>,
    /// When each enemy last raised an event, so one that flaps across the near
    /// edge is not announced again until the cooldown passes.
    enemies_announced: HashMap<Serial, Instant>,
    /// The last event a `next_event` answer carried.
    delivered: u64,
}

/// A caller waiting for the next important event.
pub(super) struct EventWaiter {
    gives_up_at: Instant,
    /// The ambient kinds this caller wants besides the important events.
    ambient: Vec<EventKind>,
    reply: oneshot::Sender<ToolResult>,
}

/// The ambient kinds a call asks for: none, some by name, or all.
fn ambient_kinds(args: &Value) -> std::result::Result<Vec<EventKind>, &'static str> {
    let Some(names) = args.get(ARG_AMBIENT) else {
        return Ok(Vec::new());
    };
    let names = names.as_array().ok_or(BAD_AMBIENT)?;
    let mut kinds = Vec::new();
    for name in names {
        if name.as_str() == Some(AMBIENT_ALL) {
            return Ok(uoterm_world::AMBIENT_EVENT_KINDS.to_vec());
        }
        let kind: EventKind = serde_json::from_value(name.clone()).map_err(|_| BAD_AMBIENT)?;
        if !kind.is_ambient() {
            return Err(BAD_AMBIENT);
        }
        kinds.push(kind);
    }
    Ok(kinds)
}

/// Notes the changes no packet names by itself: health falling under the
/// low mark, and an enemy coming near.
pub(super) fn pump_awareness(inner: &mut Inner) {
    let (low, near, names) = {
        let world = inner.world.read();
        let s = &world.self_state;
        let low =
            s.hits_max > 0 && u32::from(s.hits) * PERCENT < u32::from(s.hits_max) * LOW_HEALTH_PCT;
        let near: Vec<(Serial, String)> = enemies_near(inner, &world)
            .into_iter()
            .map(|(m, _)| (m.serial, m.name.clone()))
            .collect();
        (low, near, format!("{}/{}", s.hits, s.hits_max))
    };
    let mut events = Vec::new();
    if low && !inner.aware.low_health {
        events.push(Event::new(EventKind::LowHealth, None, names));
    }
    inner.aware.low_health = low;
    let now = Instant::now();
    for (serial, name) in &near {
        let newly_near = !inner.aware.enemies_near.contains(serial);
        let off_cooldown = inner
            .aware
            .enemies_announced
            .get(serial)
            .is_none_or(|&when| now.duration_since(when) >= ENEMY_NEAR_COOLDOWN);
        if newly_near && off_cooldown {
            events.push(Event::new(
                EventKind::EnemyNear,
                Some(*serial),
                name.clone(),
            ));
            inner.aware.enemies_announced.insert(*serial, now);
        }
    }
    let near_now: HashSet<Serial> = near.iter().map(|(s, _)| *s).collect();
    // Forget cooldowns for enemies gone from range whose window has passed, so
    // the map cannot grow without bound in a room that churns through mobiles.
    inner.aware.enemies_announced.retain(|serial, when| {
        near_now.contains(serial) || now.duration_since(*when) < ENEMY_NEAR_COOLDOWN
    });
    inner.aware.enemies_near = near_now;
    if !events.is_empty() {
        let mut world = inner.world.write();
        for event in events {
            world.push_event(event);
        }
    }
}

/// The mobiles near the character that it may fight, nearest first. Party
/// members and friends are never enemies.
fn enemies_near<'w>(inner: &Inner, world: &'w World) -> Vec<(&'w uoterm_world::Mobile, u32)> {
    let here = world.self_state.location;
    let mut near: Vec<_> = world
        .mobiles
        .values()
        .filter(|m| m.serial != world.self_state.serial)
        .filter(|m| crate::reflex::can_be_harmed(m.notoriety, m.flags))
        .filter(|m| !world.party.contains(&m.serial) && !inner.agents.is_friend(world, m.serial))
        .map(|m| (m, here.chebyshev(m.location)))
        .filter(|&(_, d)| d <= ENEMY_NEAR_TILES)
        .collect();
    near.sort_by_key(|&(m, d)| (d, m.serial.0));
    near
}

/// True for an event an agent must act on or know of. An ambient event
/// counts only when the caller asked for its kind.
fn important(inner: &Inner, event: &Event, world: &World, ambient: &[EventKind]) -> bool {
    if event.kind.is_ambient() {
        return ambient.contains(&event.kind);
    }
    match event.kind {
        // A gump of the ignore list: every gump open from that object is
        // one the agent does not hear of.
        EventKind::GumpOpened => {
            let mut from_it = world
                .gumps
                .iter()
                .filter(|g| Some(g.serial) == event.serial)
                .peekable();
            from_it.peek().is_none()
                || !from_it.all(|g| super::actions::gump_ignored(inner, g.gump_id))
        }
        EventKind::SpokenTo => !super::actions::line_ignored(inner, "", &event.text),
        EventKind::Damaged => event.serial == Some(world.self_state.serial),
        EventKind::ItemAdded => event.serial.is_some_and(|item| {
            backpack_serial(world).is_some_and(|pack| world.is_inside(item, pack))
        }),
        EventKind::Speech | EventKind::LoggedIn | EventKind::ContainerOpened => false,
        _ => true,
    }
}

/// Answers at once when important events wait; otherwise keeps the caller
/// until one comes or its time runs out.
pub(super) fn wait_for_event(
    inner: &mut Inner,
    args: &Value,
    reply: oneshot::Sender<ToolResult>,
    now: Instant,
) {
    let ambient = match ambient_kinds(args) {
        Ok(kinds) => kinds,
        Err(why) => {
            let _ = reply.send(ToolResult::err(why));
            return;
        }
    };
    match take_events(inner, &ambient) {
        Some(answer) => {
            let _ = reply.send(answer);
        }
        None => inner.event_waiters.push(EventWaiter {
            gives_up_at: wait_deadline(args, now),
            ambient,
            reply,
        }),
    }
}

/// Answers the waiting callers when important events have come, and those
/// whose time ran out with none, with the state alone.
pub(super) fn answer_event_waiters(inner: &mut Inner, now: Instant) {
    if inner.event_waiters.is_empty() {
        return;
    }
    let waiters = std::mem::take(&mut inner.event_waiters);
    let mut answered = false;
    for waiter in waiters {
        let ready = if answered {
            None
        } else {
            take_events(inner, &waiter.ambient)
        };
        if let Some(ready) = ready {
            answered = true;
            let _ = waiter.reply.send(ready);
        } else if waiter.gives_up_at <= now {
            let _ = waiter.reply.send(event_answer(inner, Vec::new(), 0));
        } else {
            inner.event_waiters.push(waiter);
        }
    }
}

/// The important events not yet given to the agent, with the state, or None
/// when there are none.
fn take_events(inner: &mut Inner, ambient: &[EventKind]) -> Option<ToolResult> {
    let (events, last, missed) = {
        let world = inner.world.read();
        let after = inner.aware.delivered;
        let oldest = world.events.first().map_or(0, |e| e.seq);
        let missed = if after > 0 {
            oldest.saturating_sub(after + 1)
        } else {
            0
        };
        let newer: Vec<&Event> = world.events.iter().filter(|e| e.seq > after).collect();
        let events: Vec<Event> = newer
            .iter()
            .filter(|e| important(inner, e, &world, ambient))
            .take(NEXT_EVENT_BATCH)
            .map(|e| (*e).clone())
            .collect();
        // Past the batch's last event, or past everything seen when no
        // event mattered, so plain events are not read again.
        let last = match events.last() {
            Some(e) if events.len() == NEXT_EVENT_BATCH => e.seq,
            _ => newer.last().map_or(after, |e| e.seq),
        };
        (events, last, missed)
    };
    inner.aware.delivered = last;
    (!events.is_empty()).then(|| event_answer(inner, events, missed))
}

/// What the character is doing now, so an agent's words match its acts:
/// its goal, where it walks, whom it follows or plays along with, and a
/// loot or bank job under way.
pub(super) fn doing(inner: &Inner, world: &World) -> Value {
    // How the last walk ended: there, or with the reason no way was found.
    let last_walk = world
        .events
        .iter()
        .rev()
        .find(|e| matches!(e.kind, EventKind::Arrived | EventKind::PathFailed))
        .map(|e| json!({ "arrived": e.kind == EventKind::Arrived, "words": e.text, "unix_ms": e.unix_ms }));
    json!({
        "goal": inner.goal.name(),
        "walking_to": inner.movement.goal,
        "last_walk": last_walk,
        "following": inner.follow.map(|s| world.name_of(s)),
        "playing_along_with": inner.play_along.map(|run| world.name_of(run.with)),
        "looting": inner.loot.as_ref().map(|job| job.corpse),
        "banking": inner.deposit.is_some(),
        "script": scripting::running_slots(inner).first(),
        "scripts": scripting::running_slots(inner),
        "job": inner.hunt.as_ref().map(|job| json!({
            "name": crate::jobs::JOB_HUNT,
            "phase": job.phase_name(),
        })).or_else(|| inner.walk.as_ref().map(|job| json!({
            "name": crate::jobs::JOB_WALK,
            "phase": job.phase_name(),
            "watch": job.watch,
        }))),
    })
}

/// The events, and the state an agent needs to act on them.
fn event_answer(inner: &Inner, events: Vec<Event>, missed: u64) -> ToolResult {
    // Read before the guard below: the world lock is not reentrant.
    let gumps = super::gump_views(inner);
    let world = inner.world.read();
    let s = &world.self_state;
    let enemies: Vec<Value> = enemies_near(inner, &world)
        .into_iter()
        .take(STATE_ENEMIES)
        .map(|(m, dist)| json!({ "serial": m.serial, "name": m.name, "dist": dist }))
        .collect();
    let pack = backpack_serial(&world);
    let pack_items = pack.map_or(0, |p| world.items_inside(p, true).len());
    let mut state = json!({
        "hits": s.hits,
        "hits_max": s.hits_max,
        "mana": s.mana,
        "mana_max": s.mana_max,
        "stam": s.stam,
        "stam_max": s.stam_max,
        "war": s.war,
        "dead": s.dead,
        "location": s.location,
        "combatant": world.combatant.map(|c| world.name_of(c)),
        "enemies_near": enemies,
        "unanswered": world.spoken_to.unanswered(uoterm_world::unix_now_ms()),
        "chat_mode": world.chat_mode(),
        "target_cursor": world.pending_target.is_some(),
        "pack": { "items": pack_items, "weight": s.weight, "weight_max": s.weight_max },
        "doing": doing(inner, &world),
        "human_control": inner.human.active(),
    });
    if !gumps.is_empty() {
        state["gumps"] = json!(gumps);
    }
    ToolResult::ok(json!({ "events": events, "missed": missed, "state": state }))
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::{armed_session, PACK};
    use super::*;
    use uoterm_protocol::{MobileView, SpeechLine, NOTO_GREY, NOTO_INNOCENT, SPEECH_REGULAR};

    const ME: Serial = Serial(0x0000_0001);
    const ANN: Serial = Serial(0x0000_0F01);
    const RAT: Serial = Serial(0x0000_0F02);
    const FULL_HEALTH: u16 = 100;
    const LOW: u16 = 20;

    fn player() -> Inner {
        let inner = armed_session();
        {
            let mut w = inner.world.write();
            w.logged_in = true;
            w.answer_when_named = true;
            w.self_state.serial = ME;
            w.self_state.name = "Mara".into();
            w.self_state.hits = FULL_HEALTH;
            w.self_state.hits_max = FULL_HEALTH;
        }
        inner
    }

    fn mobile(inner: &Inner, serial: Serial, notoriety: u8, x: u16) {
        inner
            .world
            .write()
            .apply(&Inbound::MobileIncoming(MobileView {
                serial,
                body: 0x190,
                x,
                y: 0,
                z: 0,
                direction: 0,
                hue: 0,
                flags: 0,
                notoriety,
                hits: None,
                hits_max: None,
                equipment: Vec::new(),
            }));
    }

    /// Calls `next_event` and returns its answer, or None while it waits.
    fn next_event(inner: &mut Inner) -> Option<ToolResult> {
        let (tx, mut rx) = oneshot::channel();
        wait_for_event(inner, &json!({ "timeout_ms": 0 }), tx, Instant::now());
        rx.try_recv().ok()
    }

    fn kinds(answer: &ToolResult) -> Vec<String> {
        answer.result["events"]
            .as_array()
            .expect("events")
            .iter()
            .map(|e| e["kind"].as_str().unwrap_or_default().to_string())
            .collect()
    }

    #[test]
    fn a_named_line_comes_at_once_with_the_state() {
        let mut inner = player();
        mobile(&inner, ANN, NOTO_INNOCENT, 1);
        inner.world.write().apply(&Inbound::Speech(SpeechLine {
            affix: None,
            serial: ANN,
            graphic: 0x190,
            kind: SPEECH_REGULAR,
            hue: 0,
            name: "Ann".into(),
            text: "mara, hi".into(),
        }));
        inner.follow = Some(ANN);
        let answer = next_event(&mut inner).expect("an answer at once");
        assert_eq!(kinds(&answer), vec!["spoken_to"]);
        assert_eq!(answer.result["state"]["doing"]["following"], "Ann");
        assert_eq!(answer.result["state"]["unanswered"][0]["name"], "Ann");
        assert_eq!(answer.result["state"]["hits"], FULL_HEALTH);
        assert!(next_event(&mut inner).is_none(), "the event is given once");
        answer_event_waiters(&mut inner, Instant::now() + Duration::from_secs(1));
    }

    #[test]
    fn low_health_and_an_enemy_near_are_noted_once() {
        let mut inner = player();
        mobile(&inner, RAT, NOTO_GREY, 3);
        mobile(&inner, ANN, NOTO_INNOCENT, 2);
        inner.world.write().self_state.hits = LOW;
        pump_awareness(&mut inner);
        pump_awareness(&mut inner);
        let answer = next_event(&mut inner).expect("events");
        assert_eq!(kinds(&answer), vec!["low_health", "enemy_near"]);
        let enemies = answer.result["state"]["enemies_near"]
            .as_array()
            .expect("a list")
            .clone();
        assert_eq!(enemies.len(), 1, "an innocent is no enemy");
        assert_eq!(enemies[0]["serial"], RAT.0);
    }

    /// A creature that leaves the near ring and comes back within the cooldown
    /// does not raise a second enemy_near, so a swarm churning at the edge does
    /// not flood the event log.
    #[test]
    fn a_flapping_enemy_is_announced_once_within_the_cooldown() {
        let mut inner = player();
        mobile(&inner, RAT, NOTO_GREY, 3);
        pump_awareness(&mut inner);
        let first = next_event(&mut inner).expect("the first enemy_near");
        assert_eq!(kinds(&first), vec!["enemy_near"]);
        // It drifts out of range and back in, all inside the cooldown.
        mobile(&inner, RAT, NOTO_GREY, 50);
        pump_awareness(&mut inner);
        mobile(&inner, RAT, NOTO_GREY, 3);
        pump_awareness(&mut inner);
        assert!(
            next_event(&mut inner).is_none(),
            "the same creature is not announced again within the cooldown"
        );
    }

    #[test]
    fn only_the_characters_own_hurts_and_pack_items_matter() {
        const LOOSE: Serial = Serial(0x4000_0F03);
        const KEPT: Serial = Serial(0x4000_0F04);
        let mut inner = player();
        let add = |serial: Serial, container: Serial| {
            Inbound::AddItem(uoterm_protocol::ContainerItem {
                serial,
                graphic: GRAPHIC_GOLD_COINS,
                amount: 1,
                x: 0,
                y: 0,
                grid: 0,
                container,
                hue: 0,
            })
        };
        {
            let mut w = inner.world.write();
            w.apply(&Inbound::Damage {
                serial: RAT,
                amount: 5,
            });
            w.apply(&add(LOOSE, Serial(0x4000_0F05)));
            w.apply(&add(KEPT, PACK));
        }
        let answer = next_event(&mut inner).expect("the pack item");
        assert_eq!(kinds(&answer), vec!["item_added"]);
        assert_eq!(answer.result["events"][0]["serial"], KEPT.0);
    }

    /// A sound is ambient: a plain call skips it, and a call that asks for
    /// sounds gets it. A skill change always comes.
    #[test]
    fn ambient_events_come_only_to_a_caller_who_asks() {
        let mut inner = player();
        let sound = || Inbound::SoundEffect {
            sound: 0x2A,
            volume: 0,
            x: 1,
            y: 2,
            z: 0,
        };
        inner.world.write().apply(&sound());
        assert!(
            next_event(&mut inner).is_none(),
            "a plain call skips a sound"
        );
        inner.world.write().apply(&sound());
        let (tx, mut rx) = oneshot::channel();
        let asks = json!({ "timeout_ms": 0, "ambient": ["sound"] });
        wait_for_event(&mut inner, &asks, tx, Instant::now());
        let answer = rx.try_recv().expect("the sound comes");
        assert_eq!(kinds(&answer), vec!["sound"]);
        let (tx, mut rx) = oneshot::channel();
        let wrong = json!({ "ambient": ["died"] });
        wait_for_event(&mut inner, &wrong, tx, Instant::now());
        assert!(
            !rx.try_recv().expect("an answer").ok,
            "died is no ambient kind"
        );
        assert_eq!(
            ambient_kinds(&json!({ "ambient": ["all"] })).unwrap(),
            uoterm_world::AMBIENT_EVENT_KINDS.to_vec()
        );
    }
}
