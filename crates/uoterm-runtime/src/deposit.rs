//! Put things in the bank: open the bank box, lift each stack out of the pack
//! and drop it in the box.

use std::time::Instant;

use uoterm_protocol::Serial;
use uoterm_world::World;

use crate::loot::{MoveLimits, JOB_LIFTS_REFUSED, JOB_TOOK_TOO_LONG};

/// A lift of nothing takes one item, not the stack. Send at least this.
const LIFT_AT_LEAST: u16 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct DepositJob {
    pub backpack: Serial,
    /// The one graphic to bank, or every graphic in the pack when it is None.
    pub only: Option<u16>,
    pub limits: MoveLimits,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DepositStep {
    Open(Serial),
    Lift { serial: Serial, amount: u16 },
    Drop { serial: Serial, dest: Serial },
    Wait,
    Done,
    Fail(&'static str),
}

impl DepositJob {
    pub fn new(backpack: Serial, only: Option<u16>, now: Instant) -> Self {
        Self {
            backpack,
            only,
            limits: MoveLimits::new(now),
        }
    }

    /// The next thing to do to move the pack into the bank box.
    ///
    /// The character wears his bank box on a layer of its own, and the server
    /// only fills and opens it beside a banker. So the box is opened first:
    /// nothing can go into a container the server has not handed over.
    pub fn step(&self, world: &World, action_ready: bool, now: Instant) -> DepositStep {
        if self.limits.timed_out(now) {
            return DepositStep::Fail(JOB_TOOK_TOO_LONG);
        }
        let Some(bank) = world.bank_box() else {
            return DepositStep::Fail("no bank box");
        };
        if !world.containers.contains_key(&bank) {
            if action_ready {
                return DepositStep::Open(bank);
            }
            return DepositStep::Wait;
        }
        if let Some(held) = world.holding {
            return DepositStep::Drop {
                serial: held,
                dest: bank,
            };
        }
        let Some(pack) = world.containers.get(&self.backpack) else {
            return DepositStep::Done;
        };
        let left: Vec<_> = pack
            .items
            .iter()
            .filter_map(|serial| world.items.get(serial))
            .filter(|item| item.parent == Some(self.backpack))
            .filter(|item| self.only.map_or(true, |graphic| item.graphic == graphic))
            .collect();
        if left.is_empty() {
            return DepositStep::Done;
        }
        let Some(item) = left.into_iter().find(|i| self.limits.may_lift(i.serial)) else {
            return DepositStep::Fail(JOB_LIFTS_REFUSED);
        };
        if !action_ready {
            return DepositStep::Wait;
        }
        DepositStep::Lift {
            serial: item.serial,
            amount: item.amount.max(LIFT_AT_LEAST),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loot::JOB_TIME_LIMIT;
    use uoterm_protocol::{
        ContainerItem, EquipItem, Inbound, Point3, GRAPHIC_BACKPACK, LAYER_BACKPACK, LAYER_BANK,
    };
    use uoterm_world::World;

    const SELF: Serial = Serial(0x0000_00AB);
    const BACKPACK: Serial = Serial(0x4000_0100);
    const BANK: Serial = Serial(0x4000_0300);
    const GOLD: Serial = Serial(0x4000_0301);
    const BONE: Serial = Serial(0x4000_0302);
    const GRAPHIC_BANK: u16 = 0x2436;
    const GRAPHIC_GOLD: u16 = 0x0EED;
    const GRAPHIC_BONE: u16 = 0x0F7E;
    const GOLD_AMOUNT: u16 = 54;
    const ONE: u16 = 1;
    const BANK_GUMP: u16 = 0x3C;
    const READY: bool = true;
    const NOT_READY: bool = false;
    const EVERYTHING: Option<u16> = None;

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

    /// A character wearing a pack with gold and a bone in it. The bank box is
    /// worn too unless `banked` says otherwise, because the server sends it on
    /// its own layer like any other worn thing.
    fn world_with_pack(banked: bool) -> World {
        let mut w = World::new();
        w.self_state.serial = SELF;
        w.self_state.location = Point3::new(10, 20, 1);
        w.apply(&Inbound::Equipped(EquipItem {
            serial: BACKPACK,
            graphic: GRAPHIC_BACKPACK,
            layer: LAYER_BACKPACK,
            hue: 0,
        }));
        if banked {
            w.apply(&Inbound::Equipped(EquipItem {
                serial: BANK,
                graphic: GRAPHIC_BANK,
                layer: LAYER_BANK,
                hue: 0,
            }));
        }
        w.apply(&Inbound::ContainerContents {
            items: vec![
                in_container(BACKPACK, GOLD, GRAPHIC_GOLD, GOLD_AMOUNT),
                in_container(BACKPACK, BONE, GRAPHIC_BONE, ONE),
            ],
        });
        w
    }

    fn open_bank(w: &mut World) {
        w.apply(&Inbound::OpenContainer {
            serial: BANK,
            gump: BANK_GUMP,
        });
    }

    /// A bank job that cannot finish gives up at its time limit, so the
    /// character is free to walk again.
    #[test]
    fn a_bank_job_that_runs_too_long_gives_up() {
        let w = world_with_pack(true);
        let now = Instant::now();
        let job = DepositJob::new(BACKPACK, EVERYTHING, now);
        assert_ne!(
            job.step(&w, READY, now),
            DepositStep::Fail(JOB_TOOK_TOO_LONG)
        );
        assert_eq!(
            job.step(&w, READY, now + JOB_TIME_LIMIT),
            DepositStep::Fail(JOB_TOOK_TOO_LONG)
        );
    }

    #[test]
    fn deposit_fails_when_the_server_has_sent_no_bank_box() {
        let w = world_with_pack(false);
        let job = DepositJob::new(BACKPACK, EVERYTHING, Instant::now());
        assert_eq!(
            job.step(&w, READY, Instant::now()),
            DepositStep::Fail("no bank box")
        );
    }

    #[test]
    fn deposit_opens_the_bank_box_before_anything_goes_in() {
        let w = world_with_pack(true);
        let job = DepositJob::new(BACKPACK, EVERYTHING, Instant::now());
        assert_eq!(job.step(&w, NOT_READY, Instant::now()), DepositStep::Wait);
        assert_eq!(job.step(&w, READY, Instant::now()), DepositStep::Open(BANK));
    }

    #[test]
    fn deposit_lifts_the_stack_amount_not_zero() {
        let mut w = world_with_pack(true);
        open_bank(&mut w);
        let job = DepositJob::new(BACKPACK, EVERYTHING, Instant::now());
        match job.step(&w, READY, Instant::now()) {
            DepositStep::Lift { serial, amount } => {
                assert_eq!(serial, GOLD);
                assert_eq!(amount, GOLD_AMOUNT);
                assert_ne!(amount, 0, "a lift of nothing takes one coin, not the pile");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn deposit_drops_what_it_is_holding_into_the_bank() {
        let mut w = world_with_pack(true);
        open_bank(&mut w);
        w.holding = Some(GOLD);
        let job = DepositJob::new(BACKPACK, EVERYTHING, Instant::now());
        assert_eq!(
            job.step(&w, NOT_READY, Instant::now()),
            DepositStep::Drop {
                serial: GOLD,
                dest: BANK
            }
        );
    }

    #[test]
    fn deposit_banks_only_the_graphic_it_was_asked_for() {
        let mut w = world_with_pack(true);
        open_bank(&mut w);
        let job = DepositJob::new(BACKPACK, Some(GRAPHIC_BONE), Instant::now());
        match job.step(&w, READY, Instant::now()) {
            DepositStep::Lift { serial, .. } => assert_eq!(serial, BONE, "the gold stays put"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn deposit_is_done_when_the_pack_holds_nothing_of_that_kind() {
        let mut w = world_with_pack(true);
        open_bank(&mut w);
        const NOT_IN_THE_PACK: u16 = 0x1234;
        let job = DepositJob::new(BACKPACK, Some(NOT_IN_THE_PACK), Instant::now());
        assert_eq!(job.step(&w, READY, Instant::now()), DepositStep::Done);
    }
}
