//! The test a shard applies to one step.
//!
//! Every emulator this client must work with decides a step the same way, and
//! it is not the rough test a client is tempted to write. A person climbs two
//! height units in one step and no more, he is sixteen units tall, a thing he
//! can stand on is a surface that is not impassable, and a step across a
//! corner is only legal when both tiles beside it are legal steps of their
//! own.
//!
//! The ground is the part a client must be most careful with. A map file holds
//! one height per cell, and that height is a CORNER of the square the cell
//! draws, not the middle of it. So the ground of one cell is four numbers: its
//! own and those of three neighbours. Standing in the middle of the cell a
//! person is at the average of one pair of opposite corners, and that is the
//! height both the client and the shard report for the tile. Leaving the cell
//! he is at the edge he crosses, and that height depends on the direction he
//! walks: [`LandCorners::toward`].
//!
//! The rules here are written for the character this client drives: a living
//! player on foot, who neither swims nor flies, and who opens a door instead
//! of walking around it.

use uoterm_protocol::{Direction, Point3};

use crate::tiles::{TileQuery, TILE_BRIDGE, TILE_DOOR, TILE_IMPASSABLE, TILE_SURFACE};

/// How far up a person climbs in one step.
///
/// This is the number the whole of walking turns on. A person steps up two
/// height units, which is one step of a stair and one tile of a slope. Any
/// larger number lets a walk plan a climb the shard then refuses, and the
/// character stops dead in front of it.
pub const STEP_HEIGHT: i8 = 2;

/// How tall a person is.
///
/// This says whether a body FITS in the space over a tile, which is a
/// different question from how far he climbs to get there.
pub const PERSON_HEIGHT: i8 = 16;

/// The land ids that draw nothing at all. The ground of such a cell is not
/// ground: only the statics on it hold a person up.
const LAND_NODRAW: u16 = 2;
const LAND_NODRAW_ALT: u16 = 0x1DB;
const LAND_NODRAW_RANGE: std::ops::RangeInclusive<u16> = 0x1AE..=0x1B5;

/// True when the land of a cell draws nothing and holds nobody up.
pub(crate) fn land_is_ignored(land_id: u16) -> bool {
    land_id == LAND_NODRAW || land_id == LAND_NODRAW_ALT || LAND_NODRAW_RANGE.contains(&land_id)
}

/// A person can stand on a thing with these flags.
///
/// It must be a surface, and it must not be impassable. A table carries both
/// flags: it is furniture to walk around, not a floor to walk on.
pub fn is_standing_surface(flags: u32) -> bool {
    flags & TILE_SURFACE != 0 && flags & TILE_IMPASSABLE == 0
}

/// One item standing on a tile, as the movement rules read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TilePiece {
    pub z: i8,
    pub height: u8,
    pub flags: u32,
}

impl TilePiece {
    /// The height the piece adds to the one a person stands at. A bridge is a
    /// ramp, so a person meets it half way up.
    pub(crate) fn calc_height(&self) -> i16 {
        let height = i16::from(self.height);
        if self.flags & TILE_BRIDGE != 0 {
            height / 2
        } else {
            height
        }
    }

    /// Where a person's feet rest when he stands on this piece.
    pub(crate) fn stands_at(&self) -> i16 {
        i16::from(self.z) + self.calc_height()
    }

    /// The top of the piece as a solid body. A bridge is walked up, so it
    /// closes nothing over its own foot.
    fn solid_top(&self) -> i16 {
        if self.flags & TILE_BRIDGE != 0 {
            i16::from(self.z)
        } else {
            i16::from(self.z) + i16::from(self.height)
        }
    }

    pub(crate) fn surface(&self) -> bool {
        self.flags & TILE_SURFACE != 0
    }

    pub(crate) fn impassable(&self) -> bool {
        self.flags & TILE_IMPASSABLE != 0
    }

    pub(crate) fn door(&self) -> bool {
        self.flags & TILE_DOOR != 0
    }

    /// A person can stand on this piece.
    pub(crate) fn is_standing_surface(&self) -> bool {
        is_standing_surface(self.flags)
    }

    /// This piece takes up space a body would fill. A floor takes up space
    /// too, which is how the ceiling of a low room shuts a person out of it.
    fn is_solid(&self) -> bool {
        self.surface() || self.impassable()
    }
}

/// The four corners of the square one land cell draws.
///
/// The map file writes one height per cell and that height is the north west
/// corner of its square. The other three corners are the heights written in
/// the cells to the east, to the south and to the south east, so no cell can
/// be read on its own: the ground of a cell is these four numbers, and every
/// height a person walks on is worked out from them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LandCorners {
    /// The height written in the cell itself.
    pub north_west: i8,
    /// The height written in the cell to the east.
    pub north_east: i8,
    /// The height written in the cell to the south.
    pub south_west: i8,
    /// The height written in the cell to the south east.
    pub south_east: i8,
}

impl LandCorners {
    /// A cell whose four corners are all at the same height, which is flat
    /// ground and the whole of what a map with one height per tile can say.
    pub fn flat(z: i8) -> Self {
        Self {
            north_west: z,
            north_east: z,
            south_west: z,
            south_east: z,
        }
    }

    /// The height written in the cell itself, which names the land and no
    /// person's footing.
    pub fn z(self) -> i8 {
        self.north_west
    }

    /// The lowest of the four corners. A body that cannot reach this cannot
    /// reach the cell at all.
    pub fn low(self) -> i16 {
        let (nw, ne, sw, se) = self.heights();
        nw.min(ne).min(sw).min(se)
    }

    /// Where a person standing in the middle of the cell has his feet.
    ///
    /// The square is drawn as two triangles, and which pair of opposite
    /// corners forms their shared edge is decided by which pair differs least.
    /// A person stands on that edge, half way between its two corners. This is
    /// the height a tile reports, and the one both the client and the shard
    /// give for a person who is on the tile rather than leaving it.
    pub fn center(self) -> i16 {
        let (nw, ne, sw, se) = self.heights();
        if (nw - se).abs() > (sw - ne).abs() {
            floor_average(sw, ne)
        } else {
            floor_average(nw, se)
        }
    }

    /// The height of the ground where a person leaves the cell walking in
    /// `direction`.
    ///
    /// He walks out over a corner of the square or over an edge of it. Over a
    /// corner he is at that one corner, which is why half the directions read
    /// a single number and no average at all. Over an edge he is half way
    /// between the two corners of that edge.
    ///
    /// This is what makes the ground of a cell a different height for each way
    /// out of it, and it is the height that decides how far the next step may
    /// climb.
    pub fn toward(self, direction: Direction) -> i16 {
        let (nw, ne, sw, se) = self.heights();
        match direction {
            Direction::Northwest => nw,
            Direction::North => floor_average(nw, ne),
            Direction::Northeast => ne,
            Direction::East => floor_average(ne, se),
            Direction::Southeast => se,
            Direction::South => floor_average(se, sw),
            Direction::Southwest => sw,
            Direction::West => floor_average(sw, nw),
        }
    }

    fn heights(self) -> (i16, i16, i16, i16) {
        (
            i16::from(self.north_west),
            i16::from(self.north_east),
            i16::from(self.south_west),
            i16::from(self.south_east),
        )
    }
}

/// One map tile as the movement rules read it: the land under it and every
/// item standing on it.
#[derive(Clone, Debug, Default)]
pub struct TileColumn {
    pub land_id: u16,
    pub land_flags: u32,
    pub land: LandCorners,
    pub pieces: Vec<TilePiece>,
}

impl TileColumn {
    /// A tile off the edge of the map, which no step may enter.
    pub(crate) fn off_map() -> Self {
        Self {
            land_flags: TILE_IMPASSABLE,
            ..Self::default()
        }
    }

    /// The land holds nobody up: it is impassable, and this character does not
    /// swim.
    pub(crate) fn land_blocks(&self) -> bool {
        self.land_flags & TILE_IMPASSABLE != 0
    }

    /// The land of this cell draws something, so it can hold a person up.
    pub(crate) fn consider_land(&self) -> bool {
        !land_is_ignored(self.land_id)
    }

    /// A door leaf stands on this tile.
    pub(crate) fn has_door(&self) -> bool {
        self.pieces.iter().any(TilePiece::door)
    }
}

/// The average of two heights, rounded down, below sea level as well as above.
fn floor_average(a: i16, b: i16) -> i16 {
    let sum = a + b;
    if sum < 0 {
        (sum - 1) / 2
    } else {
        sum / 2
    }
}

/// True while a body standing at `our_z` and reaching up to `our_top` fits in
/// the space over the tile.
///
/// A door leaf is left out. This client opens a door and walks through it, so
/// a shut door is a thing to unlatch and not a wall to walk around.
pub(crate) fn body_fits(column: &TileColumn, our_z: i16, our_top: i16) -> bool {
    !column.pieces.iter().any(|piece| {
        if !piece.is_solid() || piece.door() {
            return false;
        }
        let check_z = i16::from(piece.z);
        let check_top = check_z + piece.calc_height();
        check_top > our_z && our_top > check_z
    })
}

/// The height a person is standing at, and the height his footing reaches up
/// to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Footing {
    /// The foot of the thing he stands on.
    pub low: i16,
    /// Where his feet rest.
    pub center: i16,
    /// The top of the thing he stands on, never below him.
    pub top: i16,
}

/// Works out the footing of a person who is at `at_z` on this tile and about
/// to leave it in `direction`.
///
/// The land reaches up to the height of the edge it is left over, not to its
/// highest corner: a person on a slope steps off the side he walks out of, and
/// how far he may climb is measured from there. That is what makes a footing a
/// different answer for each way out of one tile.
///
/// The last rule is the one that matters most: a person who is on a tile is on
/// it, whatever the map says about it. The height a client holds for a
/// character is its own guess between one word from the server and the next,
/// and a guess that has fallen behind the ground leaves him with nothing under
/// his feet. The ground of the tile is then the best account of where he
/// stands, and taking it lets him walk off the tile instead of standing frozen
/// on it while the world moves away from him. His own height is the last
/// resort, for the tile whose land holds nobody up at all.
pub(crate) fn footing(column: &TileColumn, at_z: i8, direction: Direction) -> Footing {
    let at = i16::from(at_z);
    let land_holds = column.consider_land() && !column.land_blocks();
    let ground = || {
        (
            column.land.low(),
            column.land.center(),
            column.land.toward(direction),
        )
    };
    let mut resting = (land_holds && at >= column.land.center()).then(ground);

    for piece in &column.pieces {
        let piece_center = piece.stands_at();
        if at < piece_center || !piece.surface() {
            continue;
        }
        if let Some((_, center, _)) = resting {
            if piece_center < center {
                continue;
            }
        }
        let piece_top = i16::from(piece.z) + i16::from(piece.height);
        let top = match resting {
            Some((_, _, top)) => top.max(piece_top),
            None => piece_top,
        };
        resting = Some((i16::from(piece.z), piece_center, top));
    }

    let (low, center, top) = resting
        .or_else(|| land_holds.then(ground))
        .unwrap_or((at, at, at));
    Footing {
        low,
        center,
        top: top.max(at),
    }
}

/// Where a person ends up on one tile, or `None` when he cannot be there.
///
/// `from` is the height the walker is standing at, which decides which of
/// several floors on one tile is his: the one nearest that height wins, and
/// the lower of two equally near ones. `start_low` and `start_top` are his
/// footing on the tile he leaves, which set how far he reaches up and how much
/// room his body needs.
fn check_tile(column: &TileColumn, from: i16, start_low: i16, start_top: i16) -> Option<i16> {
    let step_top = start_top + i16::from(STEP_HEIGHT);
    let check_top = start_low + i16::from(PERSON_HEIGHT);
    let consider_land = column.consider_land();
    let land_low = column.land.low();
    let land_center = column.land.center();
    let mut arrival: Option<i16> = None;

    for piece in &column.pieces {
        if !piece.is_standing_surface() {
            continue;
        }
        let our_z = piece.stands_at();
        if let Some(best) = arrival {
            if is_further(from, our_z, best) {
                continue;
            }
        }
        let test_top = check_top.max(our_z + i16::from(PERSON_HEIGHT));
        if step_top < piece.solid_top() {
            continue;
        }
        let land_check = i16::from(piece.z) + i16::from(piece.height).min(i16::from(STEP_HEIGHT));
        if consider_land && land_check < land_center && land_center > our_z && test_top > land_low {
            continue;
        }
        if body_fits(column, our_z, test_top) {
            arrival = Some(our_z);
        }
    }

    if !consider_land || column.land_blocks() || step_top < land_low {
        return arrival;
    }

    let test_top = check_top.max(land_center + i16::from(PERSON_HEIGHT));
    let land_is_nearer = match arrival {
        Some(best) => !is_further(from, land_center, best),
        None => true,
    };
    if land_is_nearer && body_fits(column, land_center, test_top) {
        arrival = Some(land_center);
    }
    arrival
}

/// True when `candidate` is a worse floor for a walker at `from` than `best`:
/// further from his own height, or the same distance and higher up.
fn is_further(from: i16, candidate: i16, best: i16) -> bool {
    let gap = (candidate - from).abs() - (best - from).abs();
    gap > 0 || (gap == 0 && candidate > best)
}

/// The height a walker standing at `from` reaches by stepping one tile in
/// `direction`, or `None` when the step is not one he can take.
///
/// The ground he leaves is read at the edge he crosses, which is what the
/// client does and is the stricter of the two readings. A step refused here is
/// therefore never one the shard would have granted him from that edge, and a
/// character who walks only these steps is never refused for his height.
///
/// A step along a diagonal is a step past a corner, and the two tiles beside
/// that corner must pass the whole test in their own right. Only a game master
/// is let through when just one of them passes, so for this client both must.
///
/// Another mobile standing on the tile ahead is not asked about. The shard
/// tests mobiles for a wild creature and never for a player, and a walk that
/// went around every character it met could not cross a bank.
pub(crate) fn check_step<M: TileQuery + ?Sized>(
    map: &M,
    from: Point3,
    direction: Direction,
) -> Option<i8> {
    let arrival = reach(map, from, direction)?;
    if is_diagonal(direction) {
        for side in [turn(direction, -1), turn(direction, 1)] {
            reach(map, from, side)?;
        }
    }
    i8::try_from(arrival).ok()
}

/// The height a walker at `from` reaches on the tile one step away in
/// `direction`, or `None` when nothing there holds him up.
///
/// The footing is worked out for this direction and no other, because the
/// ground of the tile he leaves is a different height at each way out of it.
/// The two tiles beside a diagonal are each asked this question in their own
/// right, so each is judged from the edge that faces it.
fn reach<M: TileQuery + ?Sized>(map: &M, from: Point3, direction: Direction) -> Option<i16> {
    let (dx, dy) = direction.delta();
    let ahead = offset(map, from, dx, dy)?;
    let start = footing(&map.column(from.x, from.y), from.z, direction);
    // A walker whose height has fallen below the floor he is standing on is
    // lifted onto it before the tile ahead is read, so the floor nearest him
    // there is the one nearest his feet and not the one nearest a stale guess.
    let standing = i16::from(from.z).max(start.center);
    check_tile(
        &map.column(ahead.0, ahead.1),
        standing,
        start.low,
        start.top,
    )
}

/// The eight directions run round the compass, so every other one is a
/// diagonal.
fn is_diagonal(direction: Direction) -> bool {
    direction as u8 % 2 == 1
}

/// The direction `steps` eighths of a turn clockwise from this one.
fn turn(direction: Direction, steps: i8) -> Direction {
    const DIRECTION_COUNT: i8 = 8;
    let turned = (direction as i8 + steps).rem_euclid(DIRECTION_COUNT);
    Direction::from_byte(turned as u8)
}

/// The tile `dx`,`dy` from `at`, or `None` when it is off the map.
fn offset<M: TileQuery + ?Sized>(map: &M, at: Point3, dx: i32, dy: i32) -> Option<(u16, u16)> {
    let x = u16::try_from(i32::from(at.x) + dx).ok()?;
    let y = u16::try_from(i32::from(at.y) + dy).ok()?;
    map.in_bounds(x, y).then_some((x, y))
}
