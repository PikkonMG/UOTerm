//! The resurrection goal: a ghost finds a healer and comes back to life, as a
//! player does.
//!
//! A healer offers to bring a ghost back when the ghost steps within reach of
//! him from farther off, and the offer is a gump that asks whether to go on.
//! So the ghost walks up to a healer he can see, or to the nearest healer the
//! marker file names, or else to the nearest bank, where towns keep one; and
//! when the offer comes, he takes it.

use std::time::{Duration, Instant};

use uoterm_nav::TileQuery;
use uoterm_protocol::types::{Direction, Point3};
use uoterm_world::{Mobile, World};

use super::{answer_gump, job_ended, job_failed, queue_move, Goal, Inner};

/// How near a healer a ghost must come for the healer to offer: two tiles.
const HEALER_REACH: u32 = 2;
/// How far off the ghost steps back to come in again when no offer came.
const STEP_BACK: u16 = 4;
/// How long the ghost waits beside a healer for the offer before he steps
/// back and comes in again: a healer offers once every two seconds at most.
const OFFER_WAIT: Duration = Duration::from_secs(4);
/// The line number of the CONTINUE button of a resurrection offer, and the
/// button that takes it.
const CLILOC_CONTINUE: &str = "1011011";
const BUTTON_CONTINUE: u32 = 1;
/// The word a healer's name or title holds.
const HEALER_WORD: &str = "healer";
/// Why the goal ends.
const JOB_RESS: &str = "ress";
const ALIVE_AGAIN: &str = "alive";
const NO_HEALER_KNOWN: &str = "no healer in view, in the marker file, or at a bank near";

/// What the goal remembers between ticks.
#[derive(Debug, Default)]
pub(super) struct Reviving {
    /// When the ghost came within reach of the healer, for the wait.
    beside_since: Option<Instant>,
    /// The gump of an offer already answered.
    answered: Option<u32>,
}

/// One tick of the resurrection goal.
pub(super) fn ress_tick(inner: &mut Inner) {
    if !inner.world.read().self_state.dead {
        inner.reviving = Reviving::default();
        inner.goal = Goal::Idle;
        inner.world.write().goal = Goal::Idle.name().into();
        job_ended(inner, JOB_RESS, ALIVE_AGAIN);
        return;
    }
    if take_the_offer(inner) || inner.movement.walking() {
        return;
    }
    let (here, map, healer) = {
        let world = inner.world.read();
        let here = world.self_state.location;
        (
            here,
            world.self_state.map,
            nearest_healer(&world, here).map(|m| m.location),
        )
    };
    let Some(healer) = healer else {
        inner.reviving.beside_since = None;
        return walk_to_a_known_healer(inner, here, map);
    };
    let now = Instant::now();
    if here.chebyshev(healer) > HEALER_REACH {
        inner.reviving.beside_since = None;
        if let Some(stand) = beside(&inner.tiles(), here, healer) {
            let _ = queue_move(inner, stand);
        }
        return;
    }
    let since = *inner.reviving.beside_since.get_or_insert(now);
    if now.saturating_duration_since(since) >= OFFER_WAIT {
        // The offer comes to a ghost who steps into reach, so he steps out
        // and comes in again.
        inner.reviving.beside_since = None;
        let back = crate::jobs::away_from(here, healer, STEP_BACK);
        let _ = queue_move(inner, back);
    }
}

/// Answers a resurrection offer with CONTINUE, once. True when one was open.
fn take_the_offer(inner: &mut Inner) -> bool {
    let offer = inner
        .world
        .read()
        .gumps
        .iter()
        .find(|gump| is_resurrection_offer(&gump.layout))
        .cloned();
    let Some(gump) = offer else {
        return false;
    };
    if inner.reviving.answered == Some(gump.gump_id) {
        return true;
    }
    inner.reviving.answered = Some(gump.gump_id);
    answer_gump(inner, &gump, BUTTON_CONTINUE, &[], &[]);
    true
}

/// True for the layout of a resurrection offer: a CONTINUE button that
/// answers with its own number.
fn is_resurrection_offer(layout: &str) -> bool {
    layout
        .split(|c: char| !c.is_ascii_digit())
        .any(|number| number == CLILOC_CONTINUE)
}

/// The nearest living healer in view.
fn nearest_healer(world: &World, here: Point3) -> Option<&Mobile> {
    world
        .mobiles
        .values()
        .filter(|m| !uoterm_world::is_ghost_body(m.body))
        .filter(|m| m.answers_to(HEALER_WORD))
        .min_by_key(|m| here.chebyshev(m.location))
}

/// A tile beside the healer to walk onto, the nearest one to the ghost.
fn beside(tiles: &dyn TileQuery, here: Point3, healer: Point3) -> Option<Point3> {
    Direction::ALL
        .iter()
        .filter_map(|dir| healer.neighbour(*dir))
        .filter_map(|tile| {
            let z = tiles.surface_near(here.z, tile.x, tile.y)?;
            Some(Point3::new(tile.x, tile.y, z))
        })
        .min_by_key(|tile| here.chebyshev(*tile))
}

/// With no healer in view: the nearest healer the marker file names on this
/// map, or else the nearest bank.
fn walk_to_a_known_healer(inner: &mut Inner, here: Point3, map: u8) {
    let from_markers = inner.landmarks.as_ref().and_then(|marks| {
        marks
            .find(Some(HEALER_WORD), Some(map))
            .into_iter()
            .map(|mark| mark.at)
            .min_by_key(|at| here.chebyshev(*at))
    });
    let to = from_markers.or_else(|| crate::banks::nearest_bank(map, here).map(|bank| bank.at));
    match to {
        Some(to) => {
            let _ = queue_move(inner, to);
        }
        None => {
            job_failed(inner, JOB_RESS, NO_HEALER_KNOWN);
            inner.goal = Goal::Idle;
            inner.world.write().goal = Goal::Idle.name().into();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::test_session;
    use super::*;
    use uoterm_protocol::{Inbound, MobileView, OpenGump, Serial, NOTO_INVULNERABLE};

    const HEALER: Serial = Serial(0x0000_0E01);
    const OFFER_GUMP: u32 = 0x0000_3A3A;

    /// A ghost walks to the healer he sees, and takes the offer when it
    /// comes.
    #[test]
    fn a_ghost_walks_to_the_healer_and_takes_the_offer() {
        let mut inner = test_session();
        let here = Point3::new(100, 100, 0);
        {
            let mut world = inner.world.write();
            world.self_state.location = here;
            world.self_state.dead = true;
            world.apply(&Inbound::MobileIncoming(MobileView {
                serial: HEALER,
                body: 0x0190,
                x: here.x + 6,
                y: here.y,
                z: 0,
                direction: 0,
                hue: 0,
                flags: 0,
                notoriety: NOTO_INVULNERABLE,
                hits: None,
                hits_max: None,
                equipment: Vec::new(),
            }));
            world.mobiles.get_mut(&HEALER).unwrap().title = "the healer".into();
        }
        inner.goal = Goal::Ress;
        ress_tick(&mut inner);
        let goal = inner.movement.goal.expect("he walks");
        assert!(
            goal.chebyshev(Point3::new(here.x + 6, here.y, 0)) <= 1,
            "to the healer"
        );

        inner.movement.clear();
        inner.world.write().apply(&Inbound::Gump(OpenGump {
            serial: HEALER,
            gump_id: OFFER_GUMP,
            x: 0,
            y: 0,
            layout: "{ xmfhtmlgumpcolor 100 230 110 35 1011011 0 0 2124 }{ button 65 227 4005 4007 1 0 1 }".into(),
            text: Vec::new(),
        }));
        ress_tick(&mut inner);
        assert!(inner
            .outbound
            .contains(&uoterm_protocol::encode::gump_response(
                HEALER,
                OFFER_GUMP,
                BUTTON_CONTINUE,
                &[],
                &[]
            )));

        inner.world.write().self_state.dead = false;
        ress_tick(&mut inner);
        assert_eq!(inner.goal, Goal::Idle, "alive, the goal is over");
    }

    /// The offer is found by its CONTINUE button, whatever else the layout
    /// holds, and no other gump is taken for it.
    #[test]
    fn the_offer_is_known_by_its_continue_button() {
        let offer = "{ page 0 }{ xmfhtmlgumpcolor 100 230 110 35 1011011 0 0 2124 }\
            { button 65 227 4005 4007 1 0 1 }";
        assert!(is_resurrection_offer(offer));
        assert!(!is_resurrection_offer(
            "{ button 65 227 4005 4007 1 0 1 }{ text 1 1 0 0 }"
        ));
        assert!(!is_resurrection_offer(
            "{ xmfhtmlgump 1 1 1 1 10110110 0 0 }"
        ));
    }
}
