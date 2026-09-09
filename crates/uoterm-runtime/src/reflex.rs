//! Compiled reflex tick. Combat and gather do not wait on an LLM.

use crate::persona::Persona;
use crate::tools::{Goal, BANK_X, BANK_Y, BANK_Z};
use uoterm_protocol::types::*;
use uoterm_world::World;

pub const BANDAGE_HP_PCT: u32 = 70;
const HUNT_RANGE: u16 = 8;
const SHOP_RANGE: u16 = 8;
const FLEE_HP_STEPS: u16 = 8;
const FLEE_XY_STEPS: u16 = 12;
const SAY_SOCIAL: &str = "yo";
const SAY_DEAD: &str = "i am dead";

fn stat_pct(cur: u16, max: u16) -> u32 {
    if max == 0 {
        100
    } else {
        u32::from(cur) * 100 / u32::from(max)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReflexAction {
    None,
    MoveTo { x: u16, y: u16, z: i8 },
    Attack(Serial),
    WarMode(bool),
    Use(Serial),
    Target(Serial),
    UseSkill(u16),
    BandageSelf,
    Say(&'static str),
}

pub fn tick(world: &World, persona: &Persona, goal: &Goal) -> ReflexAction {
    if world.self_state.dead {
        if matches!(goal, Goal::Ress) {
            return ReflexAction::MoveTo {
                x: BANK_X,
                y: BANK_Y,
                z: BANK_Z,
            };
        }
        return ReflexAction::Say(SAY_DEAD);
    }
    let hp_ratio = if world.self_state.hits_max == 0 {
        1.0
    } else {
        stat_pct(world.self_state.hits, world.self_state.hits_max) as f32 / 100.0
    };
    if hp_ratio < persona.hp_flee_ratio() && !matches!(goal, Goal::Flee | Goal::Ress) {
        let loc = world.self_state.location;
        return ReflexAction::MoveTo {
            x: loc.x.saturating_sub(FLEE_HP_STEPS),
            y: loc.y,
            z: loc.z,
        };
    }
    match goal {
        Goal::Idle => ReflexAction::None,
        Goal::Social => ReflexAction::Say(SAY_SOCIAL),
        Goal::Shop => shop_action(world),
        Goal::Ress => ReflexAction::MoveTo {
            x: BANK_X,
            y: BANK_Y,
            z: BANK_Z,
        },
        Goal::Travel { dest } => ReflexAction::MoveTo {
            x: dest.x,
            y: dest.y,
            z: dest.z,
        },
        Goal::Flee => ReflexAction::MoveTo {
            x: world.self_state.location.x.saturating_sub(FLEE_XY_STEPS),
            y: world.self_state.location.y.saturating_sub(FLEE_XY_STEPS),
            z: world.self_state.location.z,
        },
        Goal::Hunt => hunt_action(world),
        Goal::Gather => gather_action(world),
        Goal::Bank => ReflexAction::MoveTo {
            x: BANK_X,
            y: BANK_Y,
            z: BANK_Z,
        },
    }
}

fn hunt_action(world: &World) -> ReflexAction {
    if world.self_state.hits_max > 0
        && stat_pct(world.self_state.hits, world.self_state.hits_max) < BANDAGE_HP_PCT
        && world.find_item_graphic(GRAPHIC_BANDAGE).is_some()
    {
        return ReflexAction::BandageSelf;
    }
    let Some(f) = world
        .nearby_mobiles(HUNT_RANGE)
        .into_iter()
        .find(|m| can_be_harmed(m.notoriety, m.flags))
    else {
        return ReflexAction::None;
    };
    if !world.self_state.war {
        return ReflexAction::WarMode(true);
    }
    ReflexAction::Attack(f.serial)
}

/// True when a server will let us hurt this mobile.
///
/// Two separate marks say "you cannot touch this". Either one alone is enough
/// to make every swing be refused, and the refusal is silent: no packet comes
/// back and the client keeps swinging at nothing.
///
/// This is why an earlier build attacked a blessed target eight times and read
/// the failure as a broken packet. It tested `notoriety >= NOTO_GREY`, and
/// [`NOTO_INVULNERABLE`] sorts above every attackable rank.
fn can_be_harmed(notoriety: u8, flags: u8) -> bool {
    NOTO_ATTACKABLE.contains(&notoriety) && flags & FLAG_BLESSED == 0
}

fn gather_action(world: &World) -> ReflexAction {
    if world.pending_target.is_some() {
        if let Some(tree) = world.find_items(None, None, None).into_iter().find(|i| {
            i.parent.is_none() && (TREE_GRAPHIC_MIN..=TREE_GRAPHIC_MAX).contains(&i.graphic)
        }) {
            return ReflexAction::Target(tree.serial);
        }
    }
    if let Some(tool) = world.find_item_graphic(GRAPHIC_HATCHET) {
        return ReflexAction::Use(tool.serial);
    }
    ReflexAction::UseSkill(SKILL_LUMBERJACKING)
}

fn shop_action(world: &World) -> ReflexAction {
    let self_serial = world.self_state.serial;
    if let Some(vendor) = world
        .nearby_mobiles(SHOP_RANGE)
        .into_iter()
        .find(|m| m.serial != self_serial && m.notoriety == NOTO_INNOCENT)
    {
        return ReflexAction::Use(vendor.serial);
    }
    ReflexAction::MoveTo {
        x: BANK_X,
        y: BANK_Y,
        z: BANK_Z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_protocol::Point3;

    /// Serial of the only mobile standing next to us in the hunt tests.
    const TARGET: Serial = Serial(0x0000_1234);

    fn world_with_one_mobile(notoriety: u8, flags: u8) -> World {
        let mut w = World::new();
        w.logged_in = true;
        w.self_state.war = true;
        w.self_state.location = Point3::new(100, 100, 0);
        w.mobiles.insert(
            TARGET,
            uoterm_world::Mobile {
                serial: TARGET,
                name: String::new(),
                body: 3,
                hue: 0,
                location: Point3::new(101, 100, 0),
                direction: 0,
                running: false,
                notoriety,
                flags,
                hits: None,
                hits_max: None,
                equipment: Vec::new(),
            },
        );
        w
    }

    #[test]
    fn hunt_attacks_a_target_it_may_harm() {
        for rank in NOTO_ATTACKABLE {
            let w = world_with_one_mobile(rank, 0);
            assert_eq!(
                tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt),
                ReflexAction::Attack(TARGET),
                "rank {rank} should be attacked"
            );
        }
    }

    /// The defect that cost this project days. An invulnerable mobile sorts
    /// ABOVE every attackable rank, so the old test of `notoriety >= NOTO_GREY`
    /// let it through. The character then swung at something no server would
    /// ever let it hurt, and every refusal came back silent.
    #[test]
    fn hunt_never_picks_a_target_it_cannot_harm() {
        let w = world_with_one_mobile(NOTO_INVULNERABLE, 0);
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt),
            ReflexAction::None,
            "an invulnerable mobile must not be attacked"
        );

        let w = world_with_one_mobile(NOTO_GREY, FLAG_BLESSED);
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt),
            ReflexAction::None,
            "a blessed mobile must not be attacked whatever its rank"
        );

        for rank in [NOTO_INNOCENT, NOTO_FRIEND] {
            let w = world_with_one_mobile(rank, 0);
            assert_eq!(
                tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt),
                ReflexAction::None,
                "rank {rank} is not an enemy"
            );
        }
    }

    #[test]
    fn idle_does_nothing() {
        let w = World::new();
        let p = Persona::lumberjack_yew();
        assert!(matches!(tick(&w, &p, &Goal::Idle), ReflexAction::None));
    }

    #[test]
    fn gather_uses_hatchet_then_targets_tree() {
        use uoterm_protocol::{GroundItem, Inbound, TargetCursor};
        let mut w = World::new();
        w.logged_in = true;
        w.apply(&Inbound::WorldItem(GroundItem {
            serial: Serial(0x4000_0010),
            graphic: TREE_GRAPHIC_MIN,
            amount: 1,
            x: 10,
            y: 10,
            z: 0,
            hue: 0,
            multi: false,
        }));
        w.items.insert(
            Serial(0x4000_0001),
            uoterm_world::Item {
                serial: Serial(0x4000_0001),
                graphic: GRAPHIC_HATCHET,
                amount: 1,
                hue: 0,
                location: Point3::new(0, 0, 0),
                parent: Some(Serial(0x4000_0002)),
                layer: None,
                grid: 0,
                name: String::new(),
            },
        );
        let p = Persona::lumberjack_yew();
        match tick(&w, &p, &Goal::Gather) {
            ReflexAction::Use(s) => assert_eq!(s.0, 0x4000_0001),
            other => panic!("expected use hatchet, got {other:?}"),
        }
        w.apply(&Inbound::Target(TargetCursor {
            kind: 0,
            id: 1,
            flags: 0,
        }));
        match tick(&w, &p, &Goal::Gather) {
            ReflexAction::Target(s) => assert_eq!(s.0, 0x4000_0010),
            other => panic!("expected target tree, got {other:?}"),
        }
    }

    #[test]
    fn low_hp_flees() {
        let mut w = World::new();
        w.self_state.hits = 5;
        w.self_state.hits_max = 100;
        let p = Persona::lumberjack_yew();
        assert!(matches!(
            tick(&w, &p, &Goal::Hunt),
            ReflexAction::MoveTo { .. }
        ));
    }

    #[test]
    fn shop_social_ress_return_actions() {
        let w = World::new();
        let p = Persona::lumberjack_yew();
        assert!(matches!(
            tick(&w, &p, &Goal::Shop),
            ReflexAction::MoveTo { .. }
        ));
        assert!(matches!(tick(&w, &p, &Goal::Social), ReflexAction::Say(_)));
        let mut dead = World::new();
        dead.self_state.dead = true;
        assert!(matches!(
            tick(&dead, &p, &Goal::Ress),
            ReflexAction::MoveTo { .. }
        ));
    }
}
