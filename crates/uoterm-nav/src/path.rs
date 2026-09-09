use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use thiserror::Error;
use uoterm_protocol::{Direction, Point3};

use crate::step::PERSON_HEIGHT;
use crate::tiles::TileQuery;

pub const ORTHO_COST: u32 = 10;
pub const DIAG_COST: u32 = 14;
pub const TURN_COST: u32 = 2;
pub const DOOR_COST: u32 = 6;

/// The most nodes one search may ever open, however far the goal is.
///
/// This is the ceiling and not the budget itself: the budget grows with the
/// distance to the goal and stops here. The ceiling is there to keep a search
/// that cannot finish from running for ever, and for nothing else. A budget
/// small enough to be reached by a walk across a city is a cap on how far a
/// character may be sent, and it reports a goal every tile of which is open as
/// one nothing can reach.
pub const MAX_EXPAND: usize = 2_000_000;

/// Nodes granted for each unit of the square of the distance to the goal.
///
/// A penalty on the ground -- a door to open, a corner to turn -- costs the
/// search the straight line it would otherwise follow, and what it opens then
/// grows with the square of the distance rather than with the distance itself.
const EXPAND_PER_SQUARE: usize = 4;

/// Nodes granted whatever the distance, so that a walk to the far side of a
/// building still has room to search around the whole of it.
const EXPAND_BASE: usize = 65_536;

/// How far apart in height two spots may be and still be the same spot.
///
/// A body is [`PERSON_HEIGHT`] tall and a refusal is about the room a body
/// needs, so that is the window. Anything further above or below is another
/// storey: the ground under a bridge, or the room over the one the walker was
/// refused in. Neither is shut by what happened on the other, and a memory
/// that read the two ground coordinates alone shut both.
pub const SAME_SPOT_HEIGHT: i8 = PERSON_HEIGHT;

/// How far apart in height the ends of two moves may be and still be the same
/// move.
///
/// This is the tolerance of a memory and not a rule of movement, which is why
/// it is a number of its own. A move is remembered from the height the walker
/// believed he was at, and two heights this close on one tile are the same
/// step of the same stair.
pub const SAME_MOVE_HEIGHT: i8 = 2;

/// Where the two ground coordinates and the height sit in a search key.
const KEY_X_SHIFT: u32 = 32;
const KEY_Y_SHIFT: u32 = 16;
const KEY_COORD_MASK: u64 = 0xFFFF;

/// Where the tile left sits in the key of one move; the tile entered fills the
/// low half.
const MOVE_KEY_FROM_SHIFT: u32 = 32;

/// Where the first of the two ground coordinates sits in the key of a cell.
const CELL_KEY_X_SHIFT: u32 = 16;

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

/// One move proven shut: a step from one spot to the next that the server
/// refused, and only in that direction.
///
/// A refusal is not always about the tile it names. The step of a stair is
/// entered from the step below it and from nowhere else, so the server grants
/// the tile from the front and refuses the very same tile from the side.
/// Remembered as a shut tile, one such refusal shuts the whole stair and the
/// storey it leads to. Remembered as a move, it shuts the one way in that
/// failed and leaves the tile open from every other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockedMove {
    /// The spot the step was taken from.
    pub from: Point3,
    /// The spot the step was refused at.
    pub to: Point3,
}

impl BlockedMove {
    /// True when this is the move from `from` to `to`.
    ///
    /// Both tiles must be the tiles remembered, and both heights must be
    /// within [`SAME_MOVE_HEIGHT`] of the heights remembered.
    fn is(&self, from: Point3, to: Point3) -> bool {
        self.from.x == from.x
            && self.from.y == from.y
            && self.to.x == to.x
            && self.to.y == to.y
            && within(self.from.z, from.z, SAME_MOVE_HEIGHT)
            && within(self.to.z, to.z, SAME_MOVE_HEIGHT)
    }
}

/// What stands in the way of a walk, in the kinds a walk must tell apart.
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
/// Both kinds shut a spot, which is a tile at a height, and not a tile at
/// every height: see [`SAME_SPOT_HEIGHT`]. The third kind shuts no spot at
/// all, only the one way into it that failed: see [`BlockedMove`].
///
/// The spot the walker stands on is exempt from both kinds of shut spot. He is
/// standing there whatever the map, the memory or another mobile says, and a
/// walk that cannot start is a walk that never happens. A shut move is not
/// exempt anywhere: it is the record of a step, and the step it names is the
/// one that failed.
#[derive(Clone, Copy, Debug)]
pub struct Obstacles<'a> {
    /// Spots a mobile stands on.
    pub soft: &'a [Point3],
    /// Spots proven impassable: the ones the server refused, and the walls of
    /// the buildings the walker can see.
    pub hard: &'a [Point3],
    /// Single moves proven shut, each in the one direction it was refused in.
    pub moves: &'a [BlockedMove],
}

impl Obstacles<'_> {
    /// Nothing is in the way.
    pub const NONE: Self = Self {
        soft: &[],
        hard: &[],
        moves: &[],
    };

    /// True when either kind of shut spot stands where `at` is.
    ///
    /// A spot is a tile at a height. A tile shut on one storey leaves the
    /// storey above it open, and a shut spot on a bridge leaves the ground
    /// under the bridge open.
    pub fn blocks(&self, at: Point3) -> bool {
        self.soft
            .iter()
            .chain(self.hard)
            .any(|spot| same(*spot, at))
    }
}

/// True when two spots are the one spot: the same tile, within a body height.
fn same(a: Point3, b: Point3) -> bool {
    a.x == b.x && a.y == b.y && within(a.z, b.z, SAME_SPOT_HEIGHT)
}

/// True when two heights are no further apart than `window`.
fn within(a: i8, b: i8, window: i8) -> bool {
    (i16::from(a) - i16::from(b)).abs() <= i16::from(window)
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

/// How the search reached one spot.
#[derive(Clone, Copy)]
struct Came {
    /// The key of the spot the step was taken from.
    from: u64,
    /// The way the step went.
    direction: Direction,
    /// The height of the ground the step lands on, which is where the walker
    /// really arrives and what the route reports.
    ground: i8,
}

/// The spots each map cell is shut at, by the key of the cell.
type ShutSpots = HashMap<u32, Vec<i8>>;

/// The shut moves between one pair of cells, by the key of the pair. The key
/// is an index and no more: [`BlockedMove::is`] alone says what matches.
type ShutMoves = HashMap<u64, Vec<BlockedMove>>;

/// One spot of the search, packed into an integer.
///
/// The height belongs in the key. Keyed on the two ground coordinates alone,
/// the search shuts a whole column of a building at whichever floor it reached
/// first: it meets a switchback stair at the lower flight and then refuses to
/// look at the upper one, and every storey but the first is unreachable. The
/// key is an integer and not a struct because a long search holds hundreds of
/// thousands of them.
fn spot_key(x: u16, y: u16, z: i8) -> u64 {
    (u64::from(x) << KEY_X_SHIFT) | (u64::from(y) << KEY_Y_SHIFT) | u64::from(z as u8)
}

fn key_x(key: u64) -> u16 {
    ((key >> KEY_X_SHIFT) & KEY_COORD_MASK) as u16
}

fn key_y(key: u64) -> u16 {
    ((key >> KEY_Y_SHIFT) & KEY_COORD_MASK) as u16
}

/// One map cell, without the height: what a shut spot is filed under.
fn cell_key(x: u16, y: u16) -> u32 {
    (u32::from(x) << CELL_KEY_X_SHIFT) | u32::from(y)
}

/// One pair of cells, in the order they are walked: what a shut move is filed
/// under.
fn move_key(from: Point3, to: Point3) -> u64 {
    (u64::from(cell_key(from.x, from.y)) << MOVE_KEY_FROM_SHIFT) | u64::from(cell_key(to.x, to.y))
}

/// How many nodes the search from `start` to `goal` may open.
///
/// The budget grows with the square of the distance because that is how the
/// search itself grows once a penalty on the ground takes its straight line
/// away from it. A number fixed for every walk is a cap on how far a character
/// can be sent: near the goal it is more than the search will ever want, and
/// across a city it runs out with the way still open.
pub(crate) fn expand_budget(start: Point3, goal: Point3) -> usize {
    let distance = start.chebyshev(goal) as usize;
    distance
        .saturating_mul(distance)
        .saturating_mul(EXPAND_PER_SQUARE)
        .saturating_add(EXPAND_BASE)
        .min(MAX_EXPAND)
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
    // The walker stands on the start spot, so nothing recorded there stops him
    // leaving it. Everything else that is proven shut stays shut, the goal
    // included: a walk that ends on one of those spots is a walk the server
    // has already refused, and the caller is told so instead of walking it
    // again.
    let mut shut = ShutSpots::new();
    for spot in obstacles.hard.iter().copied().filter(|s| !same(*s, start)) {
        if same(spot, goal) {
            return Err(PathError::BlockedGoal);
        }
        note_shut(&mut shut, spot);
    }
    // A mobile is off its tile in a moment, so one standing where the walk
    // ends is no reason to refuse the walk.
    for spot in obstacles
        .soft
        .iter()
        .copied()
        .filter(|s| !same(*s, start) && !same(*s, goal))
    {
        note_shut(&mut shut, spot);
    }
    let mut shut_moves = ShutMoves::new();
    for refused in obstacles.moves {
        shut_moves
            .entry(move_key(refused.from, refused.to))
            .or_default()
            .push(*refused);
    }

    let budget = expand_budget(start, goal);
    let start_key = spot_key(start.x, start.y, start.z);
    let mut open = BinaryHeap::new();
    let mut g_score: HashMap<u64, u32> = HashMap::new();
    let mut came: HashMap<u64, Came> = HashMap::new();
    g_score.insert(start_key, 0);
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
        if expansions > budget {
            break;
        }
        let current_k = spot_key(x, y, z);
        if x == goal.x && y == goal.y {
            return Ok(Path {
                steps: reconstruct(&came, start_key, current_k),
            });
        }
        if g_score.get(&current_k).copied().unwrap_or(u32::MAX) < g {
            continue;
        }
        let incoming = came.get(&current_k).map(|step| step.direction);
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
            // The step is judged at the height it lands at, so a spot shut on
            // one storey leaves the storey above it open.
            let to = Point3::new(nx, ny, landing.ground);
            if is_shut(&shut, to) {
                continue;
            }
            if is_shut_move(&shut_moves, from, to) {
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
            let next_k = spot_key(nx, ny, landing.read_from);
            if tentative < g_score.get(&next_k).copied().unwrap_or(u32::MAX) {
                g_score.insert(next_k, tentative);
                came.insert(
                    next_k,
                    Came {
                        from: current_k,
                        direction: dir,
                        ground: landing.ground,
                    },
                );
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

/// Writes down the height one map cell is shut at.
fn note_shut(shut: &mut ShutSpots, spot: Point3) {
    shut.entry(cell_key(spot.x, spot.y))
        .or_default()
        .push(spot.z);
}

/// True when a spot is shut at the height the step lands at.
fn is_shut(shut: &ShutSpots, at: Point3) -> bool {
    shut.get(&cell_key(at.x, at.y))
        .is_some_and(|heights| heights.iter().any(|z| within(*z, at.z, SAME_SPOT_HEIGHT)))
}

/// True when this one move is shut. The move the other way is not, and the
/// spot the move ends on stays open to every other way in.
fn is_shut_move(shut: &ShutMoves, from: Point3, to: Point3) -> bool {
    shut.get(&move_key(from, to))
        .is_some_and(|refused| refused.iter().any(|one| one.is(from, to)))
}

fn heuristic(x: u16, y: u16, gx: u16, gy: u16) -> u32 {
    let dx = (x as i32 - gx as i32).unsigned_abs();
    let dy = (y as i32 - gy as i32).unsigned_abs();
    let ortho = dx.abs_diff(dy);
    let diag = dx.min(dy);
    diag * DIAG_COST + ortho * ORTHO_COST
}

fn reconstruct(came: &HashMap<u64, Came>, start_k: u64, goal_k: u64) -> Vec<Step> {
    let mut at = goal_k;
    let mut steps = Vec::new();
    while at != start_k {
        let Some(&Came {
            from,
            direction,
            ground,
        }) = came.get(&at)
        else {
            break;
        };
        steps.push(Step {
            x: key_x(at),
            y: key_y(at),
            z: ground,
            direction,
        });
        at = from;
    }
    steps.reverse();
    steps
}
