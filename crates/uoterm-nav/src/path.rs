use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use thiserror::Error;
use uoterm_protocol::{Direction, Point3};

use crate::tiles::TileQuery;

pub const ORTHO_COST: u32 = 10;
pub const DIAG_COST: u32 = 14;
pub const TURN_COST: u32 = 2;
pub const DOOR_COST: u32 = 6;
pub const MAX_EXPAND: usize = 24_000;

pub(crate) const DIRS: [Direction; 8] = [
    Direction::North,
    Direction::Northeast,
    Direction::East,
    Direction::Southeast,
    Direction::South,
    Direction::Southwest,
    Direction::West,
    Direction::Northwest,
];

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PathError {
    #[error("start is not walkable")]
    BadStart,
    #[error("goal is not walkable")]
    BadGoal,
    #[error("goal stands on a tile proven shut")]
    BlockedGoal,
    #[error("no path")]
    Unreachable,
}

/// What stands in the way of a walk, in the two kinds a walk must tell apart.
///
/// A mobile is soft: it may step off its tile at any moment, so a walk plans
/// around it on the way and never gives up because one stands where the walk
/// ends. By the time the walker arrives it has moved, and refusing the walk
/// would leave a character unable to go anywhere near anybody.
///
/// A tile the server has already refused, and a solid piece of a building,
/// are hard: each is proven impassable, and no waiting changes it. A walk
/// plans around a hard tile on the way, and a walk that ends on one is
/// refused with [`PathError::BlockedGoal`] so the caller can choose somewhere
/// else. Without that a character walks into the very tile he was refused at,
/// over and over, because the tile he was told to walk to is the one his own
/// memory says he cannot enter.
///
/// The tile the walker stands on is exempt from both kinds. He is standing
/// there whatever the map, the memory or another mobile says, and a walk that
/// cannot start is a walk that never happens.
#[derive(Clone, Copy, Debug)]
pub struct Obstacles<'a> {
    /// Tiles a mobile stands on.
    pub soft: &'a [Point3],
    /// Tiles proven impassable: the ones the server refused, and the walls of
    /// the buildings the walker can see.
    pub hard: &'a [Point3],
}

impl Obstacles<'_> {
    /// Nothing is in the way.
    pub const NONE: Self = Self {
        soft: &[],
        hard: &[],
    };

    /// True when either kind stands on that tile. Height says nothing here: a
    /// walk goes around the tile itself.
    pub fn blocks(&self, at: Point3) -> bool {
        self.soft
            .iter()
            .chain(self.hard)
            .any(|p| p.x == at.x && p.y == at.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Step {
    pub x: u16,
    pub y: u16,
    pub z: i8,
    pub direction: Direction,
}

#[derive(Clone, Debug)]
pub struct Path {
    pub steps: Vec<Step>,
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    f: u32,
    g: u32,
    x: u16,
    y: u16,
    z: i8,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f).then_with(|| other.g.cmp(&self.g))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn key(x: u16, y: u16) -> u32 {
    ((x as u32) << 16) | y as u32
}

pub fn pathfind<M: TileQuery + ?Sized>(
    map: &M,
    start: Point3,
    goal: Point3,
    obstacles: &Obstacles,
) -> Result<Path, PathError> {
    pathfind_mode(map, start, goal, obstacles, true)
}

/// A* that ignores the height difference between one step and the next.
///
/// This is the fallback for a walk the height-aware search cannot plan. It
/// reads every tile from the height the walker starts at, so no cliff between
/// two tiles stops it and it still never routes through a wall that stands on
/// the walker's own floor. Each step of the route it gives back carries the
/// height of the ground that step lands on, because the walker really does
/// land there and the caller records where he arrives.
pub fn pathfind_flat<M: TileQuery + ?Sized>(
    map: &M,
    start: Point3,
    goal: Point3,
    obstacles: &Obstacles,
) -> Result<Path, PathError> {
    pathfind_mode(map, start, goal, obstacles, false)
}

/// Where one step puts the walker.
///
/// The two heights are the same in the height-aware mode, where the walker
/// climbs with the ground and reads the next tile from wherever he now stands.
/// They part in the flat mode: that mode goes on reading from the height the
/// walker started at, which is what keeps it out of the walls of his own
/// floor, while the ground he lands on is the height of the tile itself.
struct Landing {
    /// The height of the ground the step lands on.
    ground: i8,
    /// The height the search reads the tile after this one from.
    read_from: i8,
}

/// Where a step to `to_x`,`to_y` puts a walker standing at `from`, or `None`
/// when that is not a step he can take.
///
/// The height-aware mode puts the whole movement test to the step, corner rule
/// included. The flat mode asks only whether a person of the walker's own
/// storey can stand on the tile. Asked with no height at all, a tile inside a
/// building with more than one floor answers for the floor above, and the flat
/// mode then walks the walker into a wall of his own floor.
fn step_landing<M: TileQuery + ?Sized>(
    map: &M,
    from: Point3,
    to_x: u16,
    to_y: u16,
    use_z: bool,
) -> Option<Landing> {
    if use_z {
        return map.can_step(from, to_x, to_y).map(|z| Landing {
            ground: z,
            read_from: z,
        });
    }
    if !map.in_bounds(to_x, to_y) {
        return None;
    }
    let tile = map.tile_from(from.z, to_x, to_y);
    tile.walkable().then_some(Landing {
        ground: tile.z,
        read_from: from.z,
    })
}

fn pathfind_mode<M: TileQuery + ?Sized>(
    map: &M,
    start: Point3,
    goal: Point3,
    obstacles: &Obstacles,
    use_z: bool,
) -> Result<Path, PathError> {
    // The walker is on his own tile, so nothing the map believes about that
    // tile can stop him leaving it. His footing there is worked out from where
    // he is, and when nothing holds him at that height the ground of the tile
    // holds him instead. Reading the tile the way a stranger reads it froze
    // every route of a character whose height did not match the ground the
    // client files draw under him. Only a tile off the map refuses a start.
    //
    // The goal is read from the height of the tile it names, not from the
    // walker's. With the walker's height, a tile at the top of a ramp or a
    // stair answers for a floor he has not climbed to yet, so every goal
    // further than one storey above him is refused before the search begins.
    if !map.in_bounds(start.x, start.y) {
        return Err(PathError::BadStart);
    }
    if !map.can_walk_from(goal.z, goal.x, goal.y) {
        return Err(PathError::BadGoal);
    }
    if start.x == goal.x && start.y == goal.y {
        return Ok(Path { steps: Vec::new() });
    }
    let start_k = key(start.x, start.y);
    let goal_k = key(goal.x, goal.y);
    // The walker stands on the start tile, so nothing recorded on it stops him
    // leaving it. Everything else that is proven shut stays shut, the goal
    // included: a walk that ends on one of those tiles is a walk the server
    // has already refused, and the caller is told so instead of walking it
    // again.
    let mut blocked: HashSet<u32> = obstacles
        .hard
        .iter()
        .map(|p| key(p.x, p.y))
        .filter(|k| *k != start_k)
        .collect();
    if blocked.contains(&goal_k) {
        return Err(PathError::BlockedGoal);
    }
    // A mobile is off its tile in a moment, so one standing where the walk
    // ends is no reason to refuse the walk.
    blocked.extend(
        obstacles
            .soft
            .iter()
            .map(|p| key(p.x, p.y))
            .filter(|k| *k != start_k && *k != goal_k),
    );

    let mut open = BinaryHeap::new();
    let mut g_score: HashMap<u32, u32> = HashMap::new();
    let mut came: HashMap<u32, (u16, u16, Direction, i8)> = HashMap::new();
    g_score.insert(start_k, 0);
    open.push(Node {
        f: heuristic(start.x, start.y, goal.x, goal.y),
        g: 0,
        x: start.x,
        y: start.y,
        z: start.z,
    });
    let mut expansions = 0usize;
    while let Some(Node { x, y, z, g, .. }) = open.pop() {
        expansions += 1;
        if expansions > MAX_EXPAND {
            break;
        }
        if x == goal.x && y == goal.y {
            return Ok(Path {
                steps: reconstruct(&came, start, goal),
            });
        }
        let current_k = key(x, y);
        if g_score.get(&current_k).copied().unwrap_or(u32::MAX) < g {
            continue;
        }
        let incoming = came.get(&current_k).map(|c| c.2);
        let from = Point3::new(x, y, z);
        for dir in DIRS {
            let (dx, dy) = dir.delta();
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let nx = nx as u16;
            let ny = ny as u16;
            let Some(landing) = step_landing(map, from, nx, ny, use_z) else {
                continue;
            };
            if blocked.contains(&key(nx, ny)) {
                continue;
            }
            // A step past a corner needs both tiles beside it. The
            // height-aware check does that itself, the shard's way: each of
            // the two must pass the whole movement test. The flat mode has
            // only the walker's own height to read them from.
            if !use_z && dx != 0 && dy != 0 {
                let sx = (x as i32 + dx) as u16;
                let sy = (y as i32 + dy) as u16;
                if !map.can_walk_from(from.z, x, sy) || !map.can_walk_from(from.z, sx, y) {
                    continue;
                }
            }
            let mut step = if dx != 0 && dy != 0 {
                DIAG_COST
            } else {
                ORTHO_COST
            };
            if let Some(prev) = incoming {
                if prev != dir {
                    step += TURN_COST;
                }
            }
            if map.tile_from(z, nx, ny).door {
                step += DOOR_COST;
            }
            let tentative = g + step;
            let nk = key(nx, ny);
            if tentative < g_score.get(&nk).copied().unwrap_or(u32::MAX) {
                g_score.insert(nk, tentative);
                came.insert(nk, (x, y, dir, landing.ground));
                let f = tentative + heuristic(nx, ny, goal.x, goal.y);
                open.push(Node {
                    f,
                    g: tentative,
                    x: nx,
                    y: ny,
                    z: landing.read_from,
                });
            }
        }
    }
    Err(PathError::Unreachable)
}

fn heuristic(x: u16, y: u16, gx: u16, gy: u16) -> u32 {
    let dx = (x as i32 - gx as i32).unsigned_abs();
    let dy = (y as i32 - gy as i32).unsigned_abs();
    let ortho = dx.abs_diff(dy);
    let diag = dx.min(dy);
    diag * DIAG_COST + ortho * ORTHO_COST
}

fn reconstruct(
    came: &HashMap<u32, (u16, u16, Direction, i8)>,
    start: Point3,
    goal: Point3,
) -> Vec<Step> {
    let mut cur = (goal.x, goal.y);
    let mut steps = Vec::new();
    while cur != (start.x, start.y) {
        let Some(&(px, py, dir, z)) = came.get(&key(cur.0, cur.1)) else {
            break;
        };
        steps.push(Step {
            x: cur.0,
            y: cur.1,
            z,
            direction: dir,
        });
        cur = (px, py);
    }
    steps.reverse();
    steps
}
