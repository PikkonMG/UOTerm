//! Empty a corpse: walk in, open it, lift each stack, drop into the pack.

use uoterm_protocol::{Serial, RANGE_LOOT};
use uoterm_world::World;

/// A lift of nothing takes one item, not the stack. Send at least this.
const LIFT_AT_LEAST: u16 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct LootJob {
    pub corpse: Serial,
    pub backpack: Serial,
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
    pub fn new(corpse: Serial, backpack: Serial) -> Self {
        Self { corpse, backpack }
    }

    pub fn step(&self, world: &World, action_ready: bool) -> LootStep {
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
        let Some(item) = container.items.iter().find_map(|serial| {
            world
                .items
                .get(serial)
                .filter(|item| item.parent == Some(self.corpse))
        }) else {
            return LootStep::Done;
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
        let job = LootJob::new(CORPSE, BACKPACK);
        match job.step(&w, READY) {
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
        let job = LootJob::new(CORPSE, BACKPACK);
        assert_eq!(job.step(&w, NOT_READY), LootStep::Wait);
        assert_eq!(job.step(&w, READY), LootStep::Open(CORPSE));
    }

    #[test]
    fn loot_lifts_the_stack_amount_not_zero() {
        let mut w = world_with_corpse(0);
        open_corpse(&mut w);
        let job = LootJob::new(CORPSE, BACKPACK);
        match job.step(&w, READY) {
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
        let job = LootJob::new(CORPSE, BACKPACK);
        assert_eq!(
            job.step(&w, NOT_READY),
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
        let job = LootJob::new(CORPSE, BACKPACK);
        assert_eq!(job.step(&w, READY), LootStep::Done);
    }

    #[test]
    fn loot_fails_when_the_corpse_is_gone() {
        let w = World::new();
        let job = LootJob::new(CORPSE, BACKPACK);
        assert_eq!(job.step(&w, READY), LootStep::Fail("corpse is gone"));
    }
}
