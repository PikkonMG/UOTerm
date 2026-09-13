//! Compiled reflex tick. Combat and gather do not wait on an LLM.

use crate::banks::nearest_bank;
use crate::persona::Persona;
use crate::tools::Goal;
use std::time::{Duration, Instant};
use uoterm_protocol::types::*;
use uoterm_world::{AssistFeature, World};

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
/// How long the character answers an attacker for after the last harm it did
/// him. Long enough to cover the gap between two swings of a slow weapon, and
/// short enough that he stops when the thing is dead or has walked away.
const FIGHT_BACK_MEMORY: Duration = Duration::from_secs(FIGHT_BACK_MEMORY_SECS);
const FIGHT_BACK_MEMORY_SECS: u64 = 30;

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
            if let Some(bank) = bank_near(world) {
                return bank;
            }
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
    // Bandages, potions and an answer to whoever is hitting her come before
    // the goal, because none of them is a goal. A wound goes on working while
    // she stands still, and so does the thing that made it.
    if let Some(care) = self_care(world) {
        return care;
    }
    if let Some(answer) = fight_back(world) {
        return answer;
    }
    match goal {
        Goal::Idle => ReflexAction::None,
        Goal::Social => ReflexAction::Say(SAY_SOCIAL),
        Goal::Shop => shop_action(world),
        Goal::Ress | Goal::Bank => bank_near(world).unwrap_or(ReflexAction::None),
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
    }
}

/// The care a character takes of himself, whatever else he is doing.
///
/// Poison and a wound are not a hunting matter: both go on working while he
/// stands still, and the bandage and the potion are the same two answers
/// whatever he was told to do. A shard can forbid either one done by itself,
/// and then it is left to the agent.
fn self_care(world: &World) -> Option<ReflexAction> {
    let hurt = world.self_state.hits_max > 0
        && stat_pct(world.self_state.hits, world.self_state.hits_max) < BANDAGE_HP_PCT;
    if hurt
        && world.assist.allows(AssistFeature::AutoBandage)
        && world.find_item_graphic(GRAPHIC_BANDAGE).is_some()
    {
        return Some(ReflexAction::BandageSelf);
    }
    if !world.assist.allows(AssistFeature::PotionHotkeys) {
        return None;
    }
    if world.self_state.poisoned {
        if let Some(serial) = drinkable_potion(world, GRAPHIC_POTION_CURE, None) {
            return Some(ReflexAction::Use(serial));
        }
    }
    if hurt {
        if let Some(serial) = drinkable_potion(world, GRAPHIC_POTION_HEAL, None) {
            return Some(ReflexAction::Use(serial));
        }
    }
    if world.self_state.stam_max > 0
        && stat_pct(world.self_state.stam, world.self_state.stam_max) < POTION_STAM_PCT
    {
        if let Some(serial) = drinkable_potion(world, GRAPHIC_POTION_REFRESH, None) {
            return Some(ReflexAction::Use(serial));
        }
    }
    None
}

/// Answers whoever is hitting the character, whatever the goal is.
///
/// Nothing else does. A fight was only ever started by the hunt goal, so a
/// character standing idle was beaten from full health down to a tenth of it
/// without striking back once, and an operator had to make her invulnerable to
/// stop it. Being beaten to death while idle is not a goal.
///
/// A fight the server already holds is left alone: that is a fight already,
/// and the goal drives it. Only a target the server will let her hurt is
/// answered, by the one test that decides that anywhere.
fn fight_back(world: &World) -> Option<ReflexAction> {
    if world.fighting() {
        return None;
    }
    let serial = world.recent_attacker(Instant::now(), FIGHT_BACK_MEMORY)?;
    let attacker = world.mobiles.get(&serial)?;
    if !can_be_harmed(attacker.notoriety, attacker.flags) {
        return None;
    }
    if !world.self_state.war {
        return Some(ReflexAction::WarMode(true));
    }
    if world.self_state.location.chebyshev(attacker.location) > u32::from(world.attack_range()) {
        return Some(ReflexAction::MoveTo {
            x: attacker.location.x,
            y: attacker.location.y,
            z: attacker.location.z,
        });
    }
    Some(ReflexAction::Attack(serial))
}

fn hunt_action(world: &World) -> ReflexAction {
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

pub(crate) fn has_free_hand(world: &World) -> bool {
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
pub(crate) fn can_be_harmed(notoriety: u8, flags: u8) -> bool {
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
    bank_near(world).unwrap_or(ReflexAction::None)
}

/// A walk to the bank, when the client knows one near the character. Far
/// from it, or on a map without it, there is none: the search for a way
/// that is not there costs seconds.
fn bank_near(world: &World) -> Option<ReflexAction> {
    nearest_bank(world.self_state.map, world.self_state.location).map(|bank| ReflexAction::MoveTo {
        x: bank.at.x,
        y: bank.at.y,
        z: bank.at.z,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_protocol::Point3;

    /// Serial of the only mobile standing next to us in the hunt tests.
    const TARGET: Serial = Serial(0x0000_1234);
    /// The character's own serial, told apart from every mobile around her.
    const SELF: Serial = Serial(0x0000_00AB);
    /// One second past whatever it is added to.
    const ONE_SECOND: Duration = Duration::from_secs(1);

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

    /// Somebody swings at her while she stands idle. Measured on a live shard:
    /// something took her from full health to a tenth of it and she never
    /// struck back once, because a fight was only ever started by the hunt
    /// goal. An operator had to make her invulnerable to stop it.
    #[test]
    fn she_answers_whoever_hits_her_whatever_the_goal_is() {
        for goal in [Goal::Idle, Goal::Gather, Goal::Bank] {
            let mut w = world_with_one_mobile(NOTO_GREY, 0);
            w.self_state.serial = SELF;
            w.harmed_by = Some(uoterm_world::Harm {
                by: TARGET,
                at: Instant::now(),
            });
            assert_eq!(
                tick(&w, &Persona::lumberjack_yew(), &goal),
                ReflexAction::Attack(TARGET),
                "she must answer her attacker while the goal is {}",
                goal.name()
            );
        }
    }

    /// The memory ends. Nothing on the wire says a fight is over, so a
    /// character who answers for ever answers a corpse.
    #[test]
    fn an_old_attacker_is_forgotten() {
        let mut w = world_with_one_mobile(NOTO_GREY, 0);
        w.self_state.serial = SELF;
        w.harmed_by = Some(uoterm_world::Harm {
            by: TARGET,
            at: Instant::now() - FIGHT_BACK_MEMORY - ONE_SECOND,
        });
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Idle),
            ReflexAction::None
        );
    }

    /// The one test of what may be hurt decides this too. Swinging at a thing
    /// no server will let her hurt is refused in silence, for ever.
    #[test]
    fn she_does_not_answer_an_attacker_she_cannot_harm() {
        let mut w = world_with_one_mobile(NOTO_INVULNERABLE, 0);
        w.self_state.serial = SELF;
        w.harmed_by = Some(uoterm_world::Harm {
            by: TARGET,
            at: Instant::now(),
        });
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Idle),
            ReflexAction::None
        );
    }

    /// The fight the server already holds is a fight already: the goal drives
    /// it, and a second attack packet on the same serial says nothing new.
    #[test]
    fn she_does_not_answer_again_while_the_server_holds_the_fight() {
        let mut w = world_with_one_mobile(NOTO_GREY, 0);
        w.self_state.serial = SELF;
        w.combatant = Some(TARGET);
        w.harmed_by = Some(uoterm_world::Harm {
            by: TARGET,
            at: Instant::now(),
        });
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Idle),
            ReflexAction::None
        );
    }

    /// A wound is not a hunting matter. She bandages herself whatever she was
    /// told to do, because the wound goes on working while she stands still.
    /// A shard list that forbids one assistant feature.
    fn forbidding(feature: AssistFeature) -> uoterm_world::AssistRules {
        uoterm_world::AssistRules::from_bits(1 << feature as u32)
    }

    /// Half her hits gone, and bandages in her pack.
    fn hurt_with_bandages() -> World {
        const BANDAGES: Serial = Serial(0x4000_0401);
        let mut w = World::new();
        w.logged_in = true;
        w.self_state.hits = 50;
        w.self_state.hits_max = 100;
        w.items.insert(
            BANDAGES,
            uoterm_world::Item {
                serial: BANDAGES,
                graphic: GRAPHIC_BANDAGE,
                amount: 1,
                hue: 0,
                location: Point3::new(0, 0, 0),
                parent: Some(Serial(0x4000_0002)),
                layer: None,
                grid: 0,
                name: "bandages".into(),
            },
        );
        w
    }

    #[test]
    fn she_bandages_herself_whatever_the_goal_is() {
        assert_eq!(
            tick(
                &hurt_with_bandages(),
                &Persona::lumberjack_yew(),
                &Goal::Idle
            ),
            ReflexAction::BandageSelf
        );
    }

    /// A shard that forbids bandaging by itself leaves the bandage to the
    /// agent.
    #[test]
    fn she_does_not_bandage_by_herself_where_the_shard_forbids_it() {
        let mut w = hurt_with_bandages();
        w.assist = forbidding(AssistFeature::AutoBandage);
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Idle),
            ReflexAction::None
        );
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

    /// A spot in Britain, a short walk from its bank.
    const IN_BRITAIN: Point3 = Point3 {
        x: 1430,
        y: 1700,
        z: 0,
    };

    #[test]
    fn shop_social_ress_return_actions() {
        let mut w = World::new();
        w.self_state.location = IN_BRITAIN;
        let p = Persona::lumberjack_yew();
        assert!(matches!(
            tick(&w, &p, &Goal::Shop),
            ReflexAction::MoveTo { .. }
        ));
        assert!(matches!(tick(&w, &p, &Goal::Social), ReflexAction::Say(_)));
        let mut dead = w.clone();
        dead.self_state.dead = true;
        assert!(matches!(
            tick(&dead, &p, &Goal::Ress),
            ReflexAction::MoveTo { .. }
        ));
    }

    /// Far from every bank, no walk to one is started: a search for a way
    /// that is not there costs seconds. In a town, the walk goes to its bank.
    #[test]
    fn no_walk_to_a_bank_that_is_not_near() {
        const MOONGLOW: Point3 = Point3 {
            x: 4474,
            y: 1285,
            z: 0,
        };
        const OPEN_SEA: Point3 = Point3 {
            x: 2000,
            y: 3900,
            z: 0,
        };
        const ILSHENAR: u8 = 2;
        let p = Persona::lumberjack_yew();
        let mut far = World::new();
        far.self_state.location = OPEN_SEA;
        assert_eq!(tick(&far, &p, &Goal::Bank), ReflexAction::None);
        let mut moonglow = World::new();
        moonglow.self_state.location = MOONGLOW;
        assert!(matches!(
            tick(&moonglow, &p, &Goal::Bank),
            ReflexAction::MoveTo {
                x: 4471,
                y: 1156,
                ..
            }
        ));
        let mut other_map = World::new();
        other_map.self_state.location = IN_BRITAIN;
        other_map.self_state.map = ILSHENAR;
        assert_eq!(tick(&other_map, &p, &Goal::Bank), ReflexAction::None);
        let mut near = World::new();
        near.self_state.location = IN_BRITAIN;
        assert!(matches!(
            tick(&near, &p, &Goal::Bank),
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

    const CURE: Serial = Serial(0x4000_0F07);

    /// Poisoned, with a cure potion in her pack and a grey beside her.
    fn poisoned_with_a_cure() -> World {
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
        w
    }

    #[test]
    fn hunt_drinks_a_cure_potion_when_poisoned() {
        assert_eq!(
            tick(
                &poisoned_with_a_cure(),
                &Persona::lumberjack_yew(),
                &Goal::Hunt
            ),
            ReflexAction::Use(CURE)
        );
    }

    /// A shard that forbids drinking potions by key leaves them to the agent.
    #[test]
    fn she_does_not_drink_by_herself_where_the_shard_forbids_it() {
        let mut w = poisoned_with_a_cure();
        w.assist = forbidding(AssistFeature::PotionHotkeys);
        assert_eq!(
            tick(&w, &Persona::lumberjack_yew(), &Goal::Idle),
            ReflexAction::None
        );
    }
}
