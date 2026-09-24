//! Line of sight, as a shard works it out: a line from the eye to the target,
//! stepped one unit at a time, and each point of it tested against the ground
//! and against every piece on its tile that stops a shot.
//!
//! A spell or an arrow at a target out of sight is refused, so an archer or a
//! mage asks this before it aims.

use uoterm_protocol::Point3;

use crate::step::land_is_ignored;
use crate::tiles::TileQuery;

/// Tiledata flags of a piece a shot cannot pass: a window, and anything that
/// stops an arrow.
pub const TILE_WINDOW: u32 = 0x0000_1000;
pub const TILE_NO_SHOOT: u32 = 0x0000_2000;
/// How far above his feet a mobile's eyes are. A shard aims from there and at
/// there.
pub const EYE_HEIGHT: i8 = 14;
/// How far a line of sight reaches: the widest range a shard updates, and one
/// more.
const SIGHT_RANGE: u32 = 25;
/// The land a shard fills the black edges of the world with. With nothing on
/// it, no line passes it.
const LAND_OF_THE_EDGE: u16 = 0x0244;
/// How far a stepped point may run past either end and still be on the line.
const LINE_SLACK: f64 = 0.5;

/// The eyes of a mobile standing at `feet`.
pub fn eyes_at(feet: Point3) -> Point3 {
    Point3::new(feet.x, feet.y, feet.z.saturating_add(EYE_HEIGHT))
}

/// Where a shot at an item standing at `at`, as tall as `height`, aims: half
/// way up it.
pub fn middle_of(at: Point3, height: u8) -> Point3 {
    let lift = i8::try_from(height / 2 + 1).unwrap_or(i8::MAX);
    Point3::new(at.x, at.y, at.z.saturating_add(lift))
}

/// True when nothing stops a line from `from` to `to`. Both points are where
/// the line starts and ends, so a caller aims from the eyes of a mobile with
/// [`eyes_at`], and at a mobile's eyes or an item's [`middle_of`].
pub fn line_of_sight<M: TileQuery + ?Sized>(map: &M, from: Point3, to: Point3) -> bool {
    if from.chebyshev(to) > SIGHT_RANGE {
        return false;
    }
    let end = to;
    // The line is always walked the same way round, from the lower end.
    let (org, dest) = if (from.x, from.y, from.z) > (to.x, to.y, to.z) {
        (to, from)
    } else {
        (from, to)
    };
    if org == dest {
        return true;
    }
    let path = line(org, dest);
    let end_top = i16::from(end.z) + 1;
    path.iter().all(|&(x, y, z)| {
        let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
            return false;
        };
        if !map.in_bounds(x, y) {
            return false;
        }
        let point_top = z + 1;
        let at_end = x == end.x && y == end.y;
        let column = map.column(x, y);
        let (land_low, land_top) = (column.land.low(), column.land.high());
        let ground_in_the_way = land_low <= point_top
            && land_top >= z
            && (!at_end || land_low > end_top || land_top < i16::from(end.z))
            && !land_is_ignored(column.land_id);
        if ground_in_the_way {
            return false;
        }
        if column.land_id == LAND_OF_THE_EDGE && column.pieces.is_empty() {
            return false;
        }
        !column.pieces.iter().any(|piece| {
            if piece.flags & (TILE_WINDOW | TILE_NO_SHOOT) == 0 {
                return false;
            }
            let bottom = i16::from(piece.z);
            let top = bottom + piece.calc_height();
            bottom <= point_top
                && top >= z
                && (!at_end || bottom > end_top || top < i16::from(end.z))
        })
    })
}

/// The points of the line from `org` to `dest`, one unit apart in all three
/// axes together, each rounded to the tile and the height it falls in, with
/// `dest` last.
fn line(org: Point3, dest: Point3) -> Vec<(i32, i32, i16)> {
    let (ox, oy, oz) = (f64::from(org.x), f64::from(org.y), f64::from(org.z));
    let (dx, dy, dz) = (
        f64::from(dest.x) - ox,
        f64::from(dest.y) - oy,
        f64::from(dest.z) - oz,
    );
    let length = (dx * dx + dy * dy + dz * dz).sqrt();
    let (run, rise, climb) = (dx / length, dy / length, dz / length);
    let within = |v: f64, a: f64, b: f64| v < a.max(b) + LINE_SLACK && v > a.min(b) - LINE_SLACK;
    let (tx, ty, tz) = (ox + dx, oy + dy, oz + dz);
    let mut points: Vec<(i32, i32, i16)> = Vec::new();
    let (mut x, mut y, mut z) = (ox, oy, oz);
    while within(x, ox, tx) && within(y, oy, ty) && within(z, oz, tz) {
        let point = (x.round() as i32, y.round() as i32, z.round() as i16);
        if points.last() != Some(&point) {
            points.push(point);
        }
        x += run;
        y += rise;
        z += climb;
    }
    let last = (i32::from(dest.x), i32::from(dest.y), i16::from(dest.z));
    if points.last() != Some(&last) {
        points.push(last);
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::step::TilePiece;
    use crate::tiles::{MockMap, Overlay};

    const GRID: u16 = 40;
    const ARCHER: Point3 = Point3 { x: 10, y: 10, z: 0 };
    const TARGET: Point3 = Point3 { x: 18, y: 10, z: 0 };
    const WALL_HEIGHT: u8 = 20;

    fn wall_at(overlay: &mut Overlay<'_>, x: u16, y: u16, flags: u32) {
        overlay.add(
            x,
            y,
            TilePiece {
                z: 0,
                height: WALL_HEIGHT,
                flags,
            },
        );
    }

    #[test]
    fn open_ground_is_in_sight_and_a_wall_is_not() {
        let map = MockMap::new(GRID, GRID);
        assert!(line_of_sight(&map, eyes_at(ARCHER), eyes_at(TARGET)));
        let mut walled = Overlay::new(&map);
        wall_at(&mut walled, 14, 10, TILE_NO_SHOOT);
        assert!(!line_of_sight(&walled, eyes_at(ARCHER), eyes_at(TARGET)));
        let mut glazed = Overlay::new(&map);
        wall_at(&mut glazed, 14, 10, TILE_WINDOW);
        assert!(
            !line_of_sight(&glazed, eyes_at(ARCHER), eyes_at(TARGET)),
            "a window stops it too"
        );
    }

    /// A piece that stops nothing, such as a low table, leaves the line clear;
    /// so does a wall on the target's own tile, which is where it stands.
    #[test]
    fn only_what_stops_a_shot_blocks_and_not_on_the_target_tile() {
        let map = MockMap::new(GRID, GRID);
        let mut table = Overlay::new(&map);
        wall_at(&mut table, 14, 10, 0);
        assert!(line_of_sight(&table, eyes_at(ARCHER), eyes_at(TARGET)));
        let mut at_target = Overlay::new(&map);
        wall_at(&mut at_target, TARGET.x, TARGET.y, TILE_NO_SHOOT);
        assert!(line_of_sight(
            &at_target,
            eyes_at(ARCHER),
            middle_of(TARGET, WALL_HEIGHT)
        ));
    }

    #[test]
    fn a_target_past_the_sight_range_is_out_of_sight() {
        let map = MockMap::new(GRID, GRID);
        let far = Point3::new(ARCHER.x + SIGHT_RANGE as u16 + 1, ARCHER.y, 0);
        assert!(!line_of_sight(&map, eyes_at(ARCHER), eyes_at(far)));
    }

    /// The line is the same line whichever end it is asked from.
    #[test]
    fn sight_goes_both_ways() {
        let map = MockMap::new(GRID, GRID);
        let mut walled = Overlay::new(&map);
        wall_at(&mut walled, 13, 11, TILE_NO_SHOOT);
        let high = Point3::new(16, 12, 0);
        assert_eq!(
            line_of_sight(&walled, eyes_at(ARCHER), eyes_at(high)),
            line_of_sight(&walled, eyes_at(high), eyes_at(ARCHER))
        );
    }
}
