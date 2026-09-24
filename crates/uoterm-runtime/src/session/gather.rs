//! The gather goal: chop the trees or mine the rock round the character, one
//! swing at a time, as a player does.
//!
//! A tree and a rock face are part of the map: the client files hold them as
//! statics and land, and a shard never sends them as items. So the spots come
//! from the map, and a swing is the tool used and its cursor aimed at the
//! tile. A shard that plants a tree as an item gets it chopped as an object.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use uoterm_assist::harvest::{Harvest, HARVEST_REACH};
use uoterm_nav::TileQuery;
use uoterm_protocol::types::{Point3, Serial};
use uoterm_world::World;

use super::{
    job_failed, queue_aim, queue_move, queue_target, send_double_click, Aim, Goal, Inner,
    BARE_LAND_GRAPHIC, CLILOC_PREFIX, TARGET_QUEUE_LIFETIME,
};

/// How far round the character a spot is looked for, in tiles.
const SPOT_SEARCH: u16 = 12;
/// How long a spot the shard says is empty is left alone. A shard fills a
/// spot again ten to twenty minutes after it runs out.
const EXHAUSTED_FOR: Duration = Duration::from_secs(20 * 60);

/// One place to gather from: a tile, the graphic the cursor names there, and
/// the item that stands there when the shard sent the tree as an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Spot {
    pub at: Point3,
    pub graphic: u16,
    pub object: Option<Serial>,
}

/// What the gather goal remembers between swings.
#[derive(Debug, Default)]
pub(super) struct Gathering {
    /// The spot of the last swing.
    spot: Option<Spot>,
    /// The spots the shard said have nothing left, and when they may be tried
    /// again.
    exhausted: HashMap<(u16, u16), Instant>,
    /// When the swing on its way is over.
    next_swing_at: Option<Instant>,
}

/// One tick of the gather goal.
pub(super) fn gather_tick(inner: &mut Inner, resource: Harvest) {
    let now = Instant::now();
    if inner.gathering.next_swing_at.is_some_and(|at| now < at) || inner.movement.walking() {
        return;
    }
    inner.gathering.exhausted.retain(|_, until| *until > now);
    let (tool, here, mounted) = {
        let world = inner.world.read();
        (
            tool(&world, resource),
            world.self_state.location,
            crate::movement::is_mounted(&world.self_state.equipment),
        )
    };
    let Some(tool) = tool else {
        return give_up(inner, resource.no_tool());
    };
    if let Some(why) = resource.refused_while_mounted().filter(|_| mounted) {
        return give_up(inner, why);
    }
    let spot = {
        let tiles = inner.tiles();
        let world = inner.world.read();
        nearest_spot(&tiles, &world, resource, here, &inner.gathering.exhausted)
    };
    let Some(spot) = spot else {
        return give_up(inner, resource.nothing_near());
    };
    if here.chebyshev(spot.at) > u32::from(HARVEST_REACH) {
        let stand = stand_by(&inner.tiles(), here, spot.at);
        match stand {
            Some(stand) => {
                let _ = queue_move(inner, stand);
            }
            // Nothing beside it can be stood on: a tree in a wall.
            None => {
                inner
                    .gathering
                    .exhausted
                    .insert((spot.at.x, spot.at.y), now + EXHAUSTED_FOR);
            }
        }
        return;
    }
    if !send_double_click(inner, tool) {
        return;
    }
    match spot.object {
        Some(serial) => queue_target(inner, serial, now),
        None => queue_aim(
            inner,
            Aim::Ground {
                at: spot.at,
                graphic: spot.graphic,
            },
            TARGET_QUEUE_LIFETIME,
            now,
        ),
    }
    inner.gathering.spot = Some(spot);
    inner.gathering.next_swing_at = Some(now + Duration::from_millis(resource.swing_ms()));
}

/// The shard said a line. When it says the spot of the last swing has nothing
/// left, that spot is left alone until it has had time to fill again.
pub(super) fn heard(inner: &mut Inner, text: &str) {
    let Goal::Gather { resource } = inner.goal else {
        return;
    };
    let Some(spot) = inner.gathering.spot else {
        return;
    };
    let number = format!("{CLILOC_PREFIX}{}", resource.exhausted_cliloc());
    let exhausted = text.starts_with(&number)
        || inner
            .cliloc
            .as_deref()
            .and_then(|table| table.text(resource.exhausted_cliloc()))
            .is_some_and(|words| text.eq_ignore_ascii_case(words));
    if exhausted {
        inner.gathering.spot = None;
        inner.gathering.next_swing_at = None;
        inner
            .gathering
            .exhausted
            .insert((spot.at.x, spot.at.y), Instant::now() + EXHAUSTED_FOR);
    }
}

/// Ends the goal and says why, once.
fn give_up(inner: &mut Inner, why: &str) {
    job_failed(inner, "gather", why);
    inner.goal = Goal::Idle;
    inner.world.write().goal = Goal::Idle.name().into();
}

/// A tool the character carries that gathers it.
fn tool(world: &World, resource: Harvest) -> Option<Serial> {
    resource
        .tools()
        .iter()
        .find_map(|graphic| world.find_item_graphic(*graphic))
        .map(|item| item.serial)
}

/// The spot nearest the character, of the map or of the items the shard sent,
/// that no swing has found empty.
pub(super) fn nearest_spot(
    tiles: &dyn TileQuery,
    world: &World,
    resource: Harvest,
    here: Point3,
    exhausted: &HashMap<(u16, u16), Instant>,
) -> Option<Spot> {
    let open = |x: u16, y: u16| !exhausted.contains_key(&(x, y));
    let items = world
        .items
        .values()
        .filter(|item| item.parent.is_none() && resource.from_static(item.graphic))
        .filter(|item| here.chebyshev(item.location) <= u32::from(SPOT_SEARCH))
        .map(|item| Spot {
            at: item.location,
            graphic: item.graphic,
            object: Some(item.serial),
        });
    let r = i32::from(SPOT_SEARCH);
    let map = (-r..=r)
        .flat_map(|dy| (-r..=r).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| {
            let x = u16::try_from(i32::from(here.x) + dx).ok()?;
            let y = u16::try_from(i32::from(here.y) + dy).ok()?;
            tiles.in_bounds(x, y).then_some((x, y))
        })
        .filter_map(|(x, y)| map_spot(tiles, resource, x, y));
    items
        .chain(map)
        .filter(|spot| open(spot.at.x, spot.at.y))
        .min_by_key(|spot| here.chebyshev(spot.at))
}

/// The spot on one tile of the map: a static the resource grows in, or the
/// land when the resource lies in it.
fn map_spot(tiles: &dyn TileQuery, resource: Harvest, x: u16, y: u16) -> Option<Spot> {
    if let Some(found) = tiles
        .statics_at(x, y)
        .into_iter()
        .find(|s| resource.from_static(s.graphic))
    {
        return Some(Spot {
            at: Point3::new(x, y, found.z),
            graphic: found.graphic,
            object: None,
        });
    }
    let tile = tiles.tile(x, y);
    resource.from_land(tile.land_id).then_some(Spot {
        at: Point3::new(x, y, tile.land_z),
        graphic: BARE_LAND_GRAPHIC,
        object: None,
    })
}

/// A tile in reach of the spot that a person can stand on, the nearest to the
/// character. A tree stands on a tile nobody can enter, so the walk ends
/// beside it.
pub(super) fn stand_by(tiles: &dyn TileQuery, here: Point3, spot: Point3) -> Option<Point3> {
    let reach = i32::from(HARVEST_REACH);
    (-reach..=reach)
        .flat_map(|dy| (-reach..=reach).map(move |dx| (dx, dy)))
        .filter_map(|(dx, dy)| {
            let x = u16::try_from(i32::from(spot.x) + dx).ok()?;
            let y = u16::try_from(i32::from(spot.y) + dy).ok()?;
            let z = tiles.surface_near(here.z, x, y)?;
            Some(Point3::new(x, y, z))
        })
        .min_by_key(|stand| here.chebyshev(*stand))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_nav::MockMap;

    const HERE: Point3 = Point3 { x: 20, y: 20, z: 0 };
    const A_TREE: u16 = 0x0CCA;

    fn world_with_tree_item(at: Point3) -> World {
        let mut world = World::new();
        world.items.insert(
            Serial(0x4000_0010),
            uoterm_world::Item {
                serial: Serial(0x4000_0010),
                graphic: A_TREE,
                amount: 1,
                hue: 0,
                location: at,
                parent: None,
                layer: None,
                grid: 0,
                name: String::new(),
                flags: 0,
            },
        );
        world
    }

    /// A tree the shard sent as an item is a spot, and it is chopped as the
    /// object it is.
    #[test]
    fn a_tree_item_is_the_nearest_spot() {
        let map = MockMap::new(64, 64);
        let tree_at = Point3::new(24, 20, 0);
        let world = world_with_tree_item(tree_at);
        let spot = nearest_spot(&map, &world, Harvest::Lumber, HERE, &HashMap::new())
            .expect("the tree is near");
        assert_eq!(spot.at, tree_at);
        assert_eq!(spot.object, Some(Serial(0x4000_0010)));
        assert!(
            nearest_spot(&map, &world, Harvest::Ore, HERE, &HashMap::new()).is_none(),
            "a tree is no rock"
        );
    }

    /// A spot the shard said is empty is not chosen again.
    #[test]
    fn an_exhausted_spot_is_left_alone() {
        let map = MockMap::new(64, 64);
        let tree_at = Point3::new(24, 20, 0);
        let world = world_with_tree_item(tree_at);
        let mut exhausted = HashMap::new();
        exhausted.insert((tree_at.x, tree_at.y), Instant::now() + EXHAUSTED_FOR);
        assert!(nearest_spot(&map, &world, Harvest::Lumber, HERE, &exhausted).is_none());
    }

    /// The walk to a tree ends on a tile beside it, in reach, that a person
    /// can stand on, and the nearest such tile to the character.
    #[test]
    fn the_walk_ends_in_reach_beside_the_spot() {
        let mut map = MockMap::new(64, 64);
        let tree_at = Point3::new(30, 20, 0);
        map.set_block(tree_at.x, tree_at.y, true);
        let stand = stand_by(&map, HERE, tree_at).expect("somewhere to stand");
        assert!(stand.chebyshev(tree_at) <= u32::from(HARVEST_REACH));
        assert!(stand != tree_at);
        assert_eq!(
            stand.x,
            tree_at.x - HARVEST_REACH,
            "the near side of the tree"
        );
    }
}
