//! Compiled reflex tick. Combat and gather do not wait on an LLM.

use crate::persona::Persona;
use crate::tools::{Goal, BANK_X, BANK_Y, BANK_Z};
use std::time::Instant;
use uoterm_protocol::types::*;
use uoterm_world::World;

pub const BANDAGE_HP_PCT: u32 = 70;
const HUNT_RANGE: u16 = 16;
const SHOP_RANGE: u16 = 8;
const FLEE_HP_STEPS: u16 = 8;
const FLEE_XY_STEPS: u16 = 12;
const SAY_SOCIAL: &str = "yo";
const SAY_DEAD: &str = "i am dead";
const POTION_STAM_PCT: u32 = 30;
const POTION_LESSER: &str = "lesser";
const POTION_GREATER: &str = "greater";
const HEAL_POTION_LESSER_MS: u64 = 3_000;
const HEAL_POTION_MS: u64 = 8_000;
const HEAL_POTION_GREATER_MS: u64 = 10_000;
const BANDAGE_SELF_BASE: f64 = 5.0;
const BANDAGE_SELF_DEX_STEP: f64 = 0.5;
const BANDAGE_SELF_DEX_CAP: f64 = 120.0;
const BANDAGE_SELF_DEX_DIV: f64 = 10.0;
const MS_PER_SECOND: f64 = 1000.0;

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
    if world.self_state.poisoned {
        if let Some(serial) = drinkable_potion(world, GRAPHIC_POTION_CURE, None) {
            return ReflexAction::Use(serial);
        }
    }
    if world.self_state.hits_max > 0
        && stat_pct(world.self_state.hits, world.self_state.hits_max) < BANDAGE_HP_PCT
    {
        if let Some(serial) = drinkable_potion(world, GRAPHIC_POTION_HEAL, None) {
            return ReflexAction::Use(serial);
        }
    }
    if world.self_state.stam_max > 0
        && stat_pct(world.self_state.stam, world.self_state.stam_max) < POTION_STAM_PCT
    {
        if let Some(serial) = drinkable_potion(world, GRAPHIC_POTION_REFRESH, None) {
            return ReflexAction::Use(serial);
        }
    }
    let Some(f) = locked_or_pick(world) else {
        return ReflexAction::None;
    };
    if !world.self_state.war {
        return ReflexAction::WarMode(true);
    }
    let dist = world.self_state.location.chebyshev(f.location);
    let range = world.attack_range();
    if dist > u32::from(range) || world.swing_is_late(Instant::now()) {
        return ReflexAction::MoveTo {
            x: f.location.x,
            y: f.location.y,
            z: f.location.z,
        };
    }
    if world.combatant == Some(f.serial) {
        return ReflexAction::None;
    }
    ReflexAction::Attack(f.serial)
}

/// Keep the serial the server already accepted. When none is live, pick the
/// lowest serial so a hash walk cannot change target every tick.
fn locked_or_pick(world: &World) -> Option<&uoterm_world::Mobile> {
    if let Some(serial) = world.combatant {
        if let Some(mobile) = world.mobiles.get(&serial) {
            if can_be_harmed(mobile.notoriety, mobile.flags) {
                return Some(mobile);
            }
        }
    }
    let mut found: Vec<&uoterm_world::Mobile> = world
        .nearby_mobiles(HUNT_RANGE)
        .into_iter()
        .filter(|m| can_be_harmed(m.notoriety, m.flags))
        .collect();
    found.sort_by_key(|m| m.serial.0);
    found.into_iter().next()
}

fn drinkable_potion(world: &World, graphic: u16, name_part: Option<&str>) -> Option<Serial> {
    if !has_free_hand(world) {
        return None;
    }
    world.items.values().find_map(|item| {
        if item.graphic != graphic {
            return None;
        }
        if let Some(part) = name_part {
            if !item.name.to_ascii_lowercase().contains(part) {
                return None;
            }
        }
        Some(item.serial)
    })
}

fn has_free_hand(world: &World) -> bool {
    let one = world
        .self_state
        .equipment
        .iter()
        .any(|eq| eq.layer == LAYER_ONE_HANDED);
    let two = world
        .self_state
        .equipment
        .iter()
        .find(|eq| eq.layer == LAYER_TWO_HANDED);
    if two.is_some_and(|eq| is_ranged_weapon(eq.graphic)) {
        return false;
    }
    !one || two.is_none()
}

/// Self-heal time in milliseconds: `5.0 + 0.5 * ((120 - dex) / 10)`.
pub fn bandage_self_ms(dex: u16) -> u64 {
    let seconds = BANDAGE_SELF_BASE
        + BANDAGE_SELF_DEX_STEP * ((BANDAGE_SELF_DEX_CAP - f64::from(dex)) / BANDAGE_SELF_DEX_DIV);
    (seconds * MS_PER_SECOND).max(0.0) as u64
}

pub fn heal_potion_lock_ms(name: &str) -> u64 {
    let n = name.to_ascii_lowercase();
    if n.contains(POTION_GREATER) {
        HEAL_POTION_GREATER_MS
    } else if n.contains(POTION_LESSER) {
        HEAL_POTION_LESSER_MS
    } else {
        HEAL_POTION_MS
    }
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

    #[test]
    fn hunt_walks_when_the_target_is_out_of_range() {
        let mut w = world_with_one_mobile(NOTO_GREY, 0);
        w.mobiles.get_mut(&TARGET).unwrap().location = Point3::new(110, 100, 0);
        match tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt) {
            ReflexAction::MoveTo { x, y, .. } => {
                assert_eq!(x, 110);
                assert_eq!(y, 100);
            }
            other => panic!("expected walk, got {other:?}"),
        }
    }

    #[test]
    fn hunt_does_not_resend_attack_while_the_server_holds_the_fight() {
        let mut w = world_with_one_mobile(NOTO_GREY, 0);
        w.combatant = Some(TARGET);
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt),
            ReflexAction::None
        );
    }

    #[test]
    fn hunt_locks_onto_one_serial() {
        const OTHER: Serial = Serial(0x0000_9999);
        let mut w = world_with_one_mobile(NOTO_GREY, 0);
        w.mobiles.insert(
            OTHER,
            uoterm_world::Mobile {
                serial: OTHER,
                name: String::new(),
                body: 3,
                hue: 0,
                location: Point3::new(101, 101, 0),
                direction: 0,
                running: false,
                notoriety: NOTO_ENEMY,
                flags: 0,
                hits: None,
                hits_max: None,
                equipment: Vec::new(),
            },
        );
        w.combatant = Some(OTHER);
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt),
            ReflexAction::None,
            "the locked serial is already the fight"
        );
        w.combatant = None;
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt),
            ReflexAction::Attack(TARGET),
            "with no fight, the lowest serial is picked every tick"
        );
    }

    #[test]
    fn hunt_never_asks_to_leave_war_mode() {
        let w = world_with_one_mobile(NOTO_GREY, 0);
        assert_ne!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt),
            ReflexAction::WarMode(false)
        );
    }

    #[test]
    fn bandage_self_time_follows_dexterity() {
        assert_eq!(bandage_self_ms(120), 5_000);
        assert_eq!(bandage_self_ms(100), 6_000);
        assert_eq!(bandage_self_ms(80), 7_000);
        assert_ne!(bandage_self_ms(80), 8_000);
    }

    #[test]
    fn heal_potion_tiers_share_a_lock_told_apart_by_name() {
        assert_eq!(heal_potion_lock_ms("a lesser heal potion"), 3_000);
        assert_eq!(heal_potion_lock_ms("a heal potion"), 8_000);
        assert_eq!(heal_potion_lock_ms("a greater heal potion"), 10_000);
    }

    #[test]
    fn hunt_drinks_a_cure_potion_when_poisoned() {
        const CURE: Serial = Serial(0x4000_0F07);
        let mut w = world_with_one_mobile(NOTO_GREY, 0);
        w.self_state.poisoned = true;
        w.items.insert(
            CURE,
            uoterm_world::Item {
                serial: CURE,
                graphic: GRAPHIC_POTION_CURE,
                amount: 1,
                hue: 0,
                location: Point3::new(0, 0, 0),
                parent: Some(Serial(0x4000_0002)),
                layer: None,
                grid: 0,
                name: "a lesser cure potion".into(),
            },
        );
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Hunt),
            ReflexAction::Use(CURE)
        );
    }
}
