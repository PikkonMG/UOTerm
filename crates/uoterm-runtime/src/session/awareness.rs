//! Awareness: what an agent must not miss while it drives the character.
//!
//! An agent that polls `observe` compares snapshots and misses what happens
//! between them. `next_event` waits until something important happens and
//! gives it, with a short state block, so any agent can run one loop:
//! wait, decide, act. Events wait in the world's event log, so an agent that
//! is slow gets them all, in order, the next time it asks.

use std::collections::HashSet;

use uoterm_world::{Event, EventKind};

use super::*;

/// Health under this share of the maximum is low.
const LOW_HEALTH_PCT: u32 = 50;
const PERCENT: u32 = 100;
/// A mobile the character may fight counts as near within this many tiles.
const ENEMY_NEAR_TILES: u32 = 10;
/// The most events one `next_event` answer carries. The rest wait for the
/// next call.
const NEXT_EVENT_BATCH: usize = 50;
/// The most near enemies the state block lists, nearest first.
const STATE_ENEMIES: usize = 5;

/// What the awareness check saw at the last tick, to tell what is new.
#[derive(Default)]
pub(super) struct Awareness {
    low_health: bool,
    enemies_near: HashSet<Serial>,
    /// The last event a `next_event` answer carried.
    delivered: u64,
}

/// A caller waiting for the next important event.
pub(super) struct EventWaiter {
    gives_up_at: Instant,
    reply: oneshot::Sender<ToolResult>,
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
    for (serial, name) in &near {
        if !inner.aware.enemies_near.contains(serial) {
            events.push(Event::new(
                EventKind::EnemyNear,
                Some(*serial),
                name.clone(),
            ));
        }
    }
    inner.aware.enemies_near = near.into_iter().map(|(s, _)| s).collect();
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

/// True for an event an agent must act on or know of.
fn important(event: &Event, world: &World) -> bool {
    match event.kind {
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
    match take_events(inner) {
        Some(answer) => {
            let _ = reply.send(answer);
        }
        None => inner.event_waiters.push(EventWaiter {
            gives_up_at: wait_deadline(args, now),
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
    let mut answer = take_events(inner);
    for waiter in waiters {
        if let Some(ready) = answer.take() {
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
fn take_events(inner: &mut Inner) -> Option<ToolResult> {
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
            .filter(|e| important(e, &world))
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
fn doing(inner: &Inner, world: &World) -> Value {
    json!({
        "goal": inner.goal.name(),
        "walking_to": inner.movement.goal,
        "following": inner.follow.map(|s| world.name_of(s)),
        "playing_along_with": inner.play_along.map(|run| world.name_of(run.with)),
        "looting": inner.loot.as_ref().map(|job| job.corpse),
        "banking": inner.deposit.is_some(),
        "script": scripting::running_name(inner),
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

    #[test]
    fn only_the_characters_own_hurts_and_pack_items_matter() {
        const LOOSE: Serial = Serial(0x4000_0F03);
        const KEPT: Serial = Serial(0x4000_0F04);
        let mut inner = player();
        let add = |serial: Serial, container: Serial| {
            Inbound::AddItem(uoterm_protocol::ContainerItem {
                serial,
                graphic: 0x0EED,
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
}
