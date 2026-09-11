//! Empty a corpse: walk in, open it, lift each stack, drop into the pack.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use uoterm_protocol::{Serial, RANGE_LOOT};
use uoterm_world::World;

/// A lift of nothing takes one item, not the stack. Send at least this.
const LIFT_AT_LEAST: u16 = 1;
/// The longest a loot or bank job runs. A job that cannot finish, such as a
/// walk that finds no way or a shard that refuses a lift, must end: while it
/// runs, the character does nothing else.
pub(crate) const JOB_TIME_LIMIT: Duration = Duration::from_secs(60);
/// How often a job lifts one item before it leaves it where it is.
const MAX_LIFTS_PER_ITEM: u8 = 3;
pub const JOB_TOOK_TOO_LONG: &str = "the job took too long";
pub const JOB_LIFTS_REFUSED: &str = "the shard refused every lift";

/// How long a loot or bank job may still run, and how often it has lifted
/// each item.
#[derive(Clone, Debug, PartialEq)]
pub struct MoveLimits {
    until: Instant,
    lifts: HashMap<Serial, u8>,
}

impl MoveLimits {
    pub fn new(now: Instant) -> Self {
        Self {
            until: now + JOB_TIME_LIMIT,
            lifts: HashMap::new(),
        }
    }

    pub fn timed_out(&self, now: Instant) -> bool {
        now >= self.until
    }

    pub fn may_lift(&self, item: Serial) -> bool {
        self.lifts
            .get(&item)
            .map_or(true, |&n| n < MAX_LIFTS_PER_ITEM)
    }

    pub fn note_lift(&mut self, item: Serial) {
        *self.lifts.entry(item).or_default() += 1;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LootJob {
    pub corpse: Serial,
    pub backpack: Serial,
    pub limits: MoveLimits,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LootStep {
    Walk { x: u16, y: u16, z: i8 },
    Open(Serial),
    Lift { serial: Serial, amount: u16 },
    Drop { serial: Serial, dest: Serial },
    Wait,
    Done,
    Fail(&'static str),
}

impl LootJob {
    pub fn new(corpse: Serial, backpack: Serial, now: Instant) -> Self {
        Self {
            corpse,
            backpack,
            limits: MoveLimits::new(now),
        }
    }

    pub fn step(&self, world: &World, action_ready: bool, now: Instant) -> LootStep {
        if self.limits.timed_out(now) {
            return LootStep::Fail(JOB_TOOK_TOO_LONG);
        }
        if let Some(held) = world.holding {
            return LootStep::Drop {
                serial: held,
                dest: self.backpack,
            };
        }
        let Some(corpse) = world.items.get(&self.corpse) else {
            return LootStep::Fail("corpse is gone");
        };
        let dist = world.self_state.location.chebyshev(corpse.location);
        if dist > u32::from(RANGE_LOOT) {
            return LootStep::Walk {
                x: corpse.location.x,
                y: corpse.location.y,
                z: corpse.location.z,
            };
        }
        let Some(container) = world.containers.get(&self.corpse) else {
            if action_ready {
                return LootStep::Open(self.corpse);
            }
            return LootStep::Wait;
        };
        let left: Vec<_> = container
            .items
            .iter()
            .filter_map(|serial| world.items.get(serial))
            .filter(|item| item.parent == Some(self.corpse))
            .collect();
        if left.is_empty() {
            return LootStep::Done;
        }
        let Some(item) = left.into_iter().find(|i| self.limits.may_lift(i.serial)) else {
            return LootStep::Fail(JOB_LIFTS_REFUSED);
        };
        if !action_ready {
            return LootStep::Wait;
        }
        LootStep::Lift {
            serial: item.serial,
            amount: item.amount.max(LIFT_AT_LEAST),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_protocol::{
        ContainerItem, EquipItem, GroundItem, Inbound, Point3, GRAPHIC_BACKPACK, LAYER_BACKPACK,
    };
    use uoterm_world::World;

    const SELF: Serial = Serial(0x0000_00AB);
    const BACKPACK: Serial = Serial(0x4000_0100);
    const CORPSE: Serial = Serial(0x4000_0200);
    const GOLD: Serial = Serial(0x4000_0201);
    const BONE: Serial = Serial(0x4000_0202);
    const GRAPHIC_GOLD: u16 = 0x0EED;
    const GRAPHIC_BONE: u16 = 0x0F7E;
    const GOLD_AMOUNT: u16 = 54;
    const ONE: u16 = 1;
    const CORPSE_GUMP: u16 = 9;
    const READY: bool = true;
    const NOT_READY: bool = false;

    fn in_container(container: Serial, serial: Serial, graphic: u16, amount: u16) -> ContainerItem {
        ContainerItem {
            serial,
            graphic,
            amount,
            x: 0,
            y: 0,
            grid: 0,
            container,
            hue: 0,
        }
    }

    fn world_with_corpse(dist: u16) -> World {
        let mut w = World::new();
        w.self_state.serial = SELF;
        w.self_state.location = Point3::new(10, 20, 1);
        w.apply(&Inbound::Equipped(EquipItem {
            serial: BACKPACK,
            graphic: GRAPHIC_BACKPACK,
            layer: LAYER_BACKPACK,
            hue: 0,
        }));
        w.apply(&Inbound::WorldItem(GroundItem {
            serial: CORPSE,
            graphic: 0x2006,
            amount: ONE,
            x: 10 + dist,
            y: 20,
            z: 1,
            hue: 0,
            multi: false,
        }));
        w
    }

    fn open_corpse(w: &mut World) {
        w.apply(&Inbound::OpenContainer {
            serial: CORPSE,
            gump: CORPSE_GUMP,
        });
        w.apply(&Inbound::ContainerContents {
            items: vec![
                in_container(CORPSE, GOLD, GRAPHIC_GOLD, GOLD_AMOUNT),
                in_container(CORPSE, BONE, GRAPHIC_BONE, ONE),
            ],
        });
    }

    #[test]
    fn loot_walks_until_within_two_tiles() {
        let w = world_with_corpse(RANGE_LOOT + 1);
        let job = LootJob::new(CORPSE, BACKPACK, Instant::now());
        match job.step(&w, READY, Instant::now()) {
            LootStep::Walk { x, y, .. } => {
                assert_eq!(x, 10 + RANGE_LOOT + 1);
                assert_eq!(y, 20);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn loot_opens_when_in_range_and_the_budget_allows() {
        let w = world_with_corpse(RANGE_LOOT);
        let job = LootJob::new(CORPSE, BACKPACK, Instant::now());
        assert_eq!(job.step(&w, NOT_READY, Instant::now()), LootStep::Wait);
        assert_eq!(job.step(&w, READY, Instant::now()), LootStep::Open(CORPSE));
    }

    #[test]
    fn loot_lifts_the_stack_amount_not_zero() {
        let mut w = world_with_corpse(0);
        open_corpse(&mut w);
        let job = LootJob::new(CORPSE, BACKPACK, Instant::now());
        match job.step(&w, READY, Instant::now()) {
            LootStep::Lift { serial, amount } => {
                assert_eq!(serial, GOLD);
                assert_eq!(amount, GOLD_AMOUNT);
                assert_ne!(amount, 0);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn loot_drops_what_it_is_holding_into_the_pack() {
        let mut w = world_with_corpse(0);
        open_corpse(&mut w);
        w.holding = Some(GOLD);
        let job = LootJob::new(CORPSE, BACKPACK, Instant::now());
        assert_eq!(
            job.step(&w, NOT_READY, Instant::now()),
            LootStep::Drop {
                serial: GOLD,
                dest: BACKPACK
            }
        );
    }

    #[test]
    fn loot_is_done_when_the_corpse_is_empty() {
        let mut w = world_with_corpse(0);
        w.apply(&Inbound::OpenContainer {
            serial: CORPSE,
            gump: CORPSE_GUMP,
        });
        let job = LootJob::new(CORPSE, BACKPACK, Instant::now());
        assert_eq!(job.step(&w, READY, Instant::now()), LootStep::Done);
    }

    /// A shard that refuses a lift leaves the item in the corpse. The job
    /// tries each item three times, then gives up, so the character can move
    /// again.
    #[test]
    fn a_refused_lift_is_tried_three_times_then_the_job_gives_up() {
        const ITEMS_IN_CORPSE: usize = 2;
        let mut w = world_with_corpse(1);
        open_corpse(&mut w);
        let now = Instant::now();
        let mut job = LootJob::new(CORPSE, BACKPACK, now);
        let mut lifts = 0;
        while let LootStep::Lift { serial, .. } = job.step(&w, READY, now) {
            job.limits.note_lift(serial);
            lifts += 1;
        }
        assert_eq!(lifts, ITEMS_IN_CORPSE * usize::from(MAX_LIFTS_PER_ITEM));
        assert_eq!(job.step(&w, READY, now), LootStep::Fail(JOB_LIFTS_REFUSED));
    }

    #[test]
    fn a_job_that_runs_too_long_gives_up() {
        let w = world_with_corpse(10);
        let now = Instant::now();
        let job = LootJob::new(CORPSE, BACKPACK, now);
        assert!(matches!(job.step(&w, READY, now), LootStep::Walk { .. }));
        assert_eq!(
            job.step(&w, READY, now + JOB_TIME_LIMIT),
            LootStep::Fail(JOB_TOOK_TOO_LONG)
        );
    }

    #[test]
    fn loot_fails_when_the_corpse_is_gone() {
        let w = World::new();
        let job = LootJob::new(CORPSE, BACKPACK, Instant::now());
        assert_eq!(
            job.step(&w, READY, Instant::now()),
            LootStep::Fail("corpse is gone")
        );
    }
}
