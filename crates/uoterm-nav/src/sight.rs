//! Line of sight, as a shard works it out: a line from the eye to the target,
//! stepped one unit at a time, and each point of it tested against the ground
//! and against every piece on its tile that stops a shot.
//!
//! A spell or an arrow at a target out of sight is refused, so an archer or a
//! mage asks this before it aims.

use serde::{Deserialize, Serialize};
use uoterm_protocol::Point3;

use crate::step::land_is_ignored;
use crate::tiles::TileQuery;

/// The tiledata flag of a wall, which Sphere judges sight by.
const TILE_WALL: u32 = crate::tiles::TileFlagSet::WALL.low_bits();

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

/// The rules a shard family judges sight by. They agree on open ground and
/// on a plain wall; they part on windows, on the tiles a line crosses and on
/// the height it is judged at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SightMode {
    /// RunUO, ServUO and ModernUO: a line stepped one unit at a time in all
    /// three axes, stopped by the ground and by windows and pieces that stop
    /// a shot.
    #[default]
    RunUo,
    /// POL: the same stepped line, stopped by the ground and by pieces that
    /// stop a shot. A window lets the line through.
    Pol,
    /// Sphere: one point on each tile the line crosses, at the height the
    /// line has there, stopped by walls and pieces that stop a shot. A window
    /// lets the line through.
    Sphere,
}

/// The mode names a caller writes, each with its mode.
const SIGHT_MODE_NAMES: [(&str, SightMode); 5] = [
    ("runuo", SightMode::RunUo),
    ("modernuo", SightMode::RunUo),
    ("servuo", SightMode::RunUo),
    ("pol", SightMode::Pol),
    ("sphere", SightMode::Sphere),
];

impl SightMode {
    /// The mode of a name, in any case: runuo, modernuo, servuo, pol or
    /// sphere.
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        SIGHT_MODE_NAMES
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, mode)| *mode)
    }
}

/// What stops a line at one point of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SightBlocker {
    /// The ends are further apart than a line of sight reaches.
    TooFar,
    /// The point is off the map.
    OffMap,
    /// The black edge of the world.
    EdgeOfWorld,
    /// The ground, from its lowest corner to its highest.
    Land { low: i16, high: i16 },
    /// A piece standing on the tile, with its tiledata flags.
    Piece { bottom: i16, top: i16, flags: u32 },
}

/// One point of a line of sight, and what stops the line there, if anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SightPoint {
    pub x: i32,
    pub y: i32,
    pub z: i16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker: Option<SightBlocker>,
}

/// Every point of a line of sight, and whether it gets through.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SightTrace {
    pub in_sight: bool,
    pub points: Vec<SightPoint>,
}

/// True when nothing stops a line from `from` to `to`, by the rules of
/// RunUO and its heirs. Both points are where the line starts and ends, so a
/// caller aims from the eyes of a mobile with [`eyes_at`], and at a mobile's
/// eyes or an item's [`middle_of`].
pub fn line_of_sight<M: TileQuery + ?Sized>(map: &M, from: Point3, to: Point3) -> bool {
    sight_trace(map, from, to, SightMode::RunUo).in_sight
}

/// The line from `from` to `to` point by point, by the rules of `mode`, with
/// what stops it at each point.
pub fn sight_trace<M: TileQuery + ?Sized>(
    map: &M,
    from: Point3,
    to: Point3,
    mode: SightMode,
) -> SightTrace {
    if from.chebyshev(to) > SIGHT_RANGE {
        return SightTrace {
            in_sight: false,
            points: vec![SightPoint {
                x: i32::from(to.x),
                y: i32::from(to.y),
                z: i16::from(to.z),
                blocker: Some(SightBlocker::TooFar),
            }],
        };
    }
    let path = match mode {
        SightMode::RunUo | SightMode::Pol => stepped_line(from, to),
        SightMode::Sphere => tile_line(from, to),
    };
    let points: Vec<SightPoint> = path
        .into_iter()
        .map(|(x, y, z)| SightPoint {
            x,
            y,
            z,
            blocker: blocker_at(map, (x, y, z), from, to, mode),
        })
        .collect();
    SightTrace {
        in_sight: points.iter().all(|p| p.blocker.is_none()),
        points,
    }
}

/// The points of the stepped line of RunUO and POL. It is always walked the
/// same way round, from the lower end, so a line asked from either end is the
/// same line.
fn stepped_line(from: Point3, to: Point3) -> Vec<(i32, i32, i16)> {
    let (org, dest) = if (from.x, from.y, from.z) > (to.x, to.y, to.z) {
        (to, from)
    } else {
        (from, to)
    };
    if org == dest {
        return Vec::new();
    }
    line(org, dest)
}

/// The tiles a line crosses between its two ends, one point on each at the
/// height the line has there. The two end tiles are where the looker and the
/// target stand, and are not judged.
fn tile_line(from: Point3, to: Point3) -> Vec<(i32, i32, i16)> {
    let steps = from.chebyshev(to) as i32;
    let (fx, fy, fz) = (i32::from(from.x), i32::from(from.y), i32::from(from.z));
    let (dx, dy, dz) = (
        i32::from(to.x) - fx,
        i32::from(to.y) - fy,
        i32::from(to.z) - fz,
    );
    let share =
        |delta: i32, i: i32| (f64::from(delta) * f64::from(i) / f64::from(steps)).round() as i32;
    (1..steps)
        .map(|i| {
            (
                fx + share(dx, i),
                fy + share(dy, i),
                (fz + share(dz, i)) as i16,
            )
        })
        .collect()
}

/// What stops the line at one point, by the rules of `mode`.
fn blocker_at<M: TileQuery + ?Sized>(
    map: &M,
    (x, y, z): (i32, i32, i16),
    from: Point3,
    end: Point3,
    mode: SightMode,
) -> Option<SightBlocker> {
    let (Ok(tx), Ok(ty)) = (u16::try_from(x), u16::try_from(y)) else {
        return Some(SightBlocker::OffMap);
    };
    if !map.in_bounds(tx, ty) {
        return Some(SightBlocker::OffMap);
    }
    let column = map.column(tx, ty);
    let (land_low, land_high) = (column.land.low(), column.land.high());
    if mode == SightMode::Sphere {
        let at_an_end = (tx, ty) == (from.x, from.y) || (tx, ty) == (end.x, end.y);
        if at_an_end {
            return None;
        }
        if !land_is_ignored(column.land_id) && z < land_low {
            return Some(SightBlocker::Land {
                low: land_low,
                high: land_high,
            });
        }
        return column
            .pieces
            .iter()
            .find(|piece| {
                let stops = piece.flags & (TILE_WALL | TILE_NO_SHOOT) != 0
                    && piece.flags & TILE_WINDOW == 0;
                let bottom = i16::from(piece.z);
                stops && bottom <= z && z < bottom + piece.calc_height()
            })
            .map(|piece| piece_blocker(piece.z, piece.calc_height(), piece.flags));
    }
    let point_top = z + 1;
    let at_end = tx == end.x && ty == end.y;
    let end_top = i16::from(end.z) + 1;
    let ground_in_the_way = land_low <= point_top
        && land_high >= z
        && (!at_end || land_low > end_top || land_high < i16::from(end.z))
        && !land_is_ignored(column.land_id);
    if ground_in_the_way {
        return Some(SightBlocker::Land {
            low: land_low,
            high: land_high,
        });
    }
    if column.land_id == LAND_OF_THE_EDGE && column.pieces.is_empty() {
        return Some(SightBlocker::EdgeOfWorld);
    }
    let stopping = match mode {
        SightMode::Pol => TILE_NO_SHOOT,
        SightMode::RunUo | SightMode::Sphere => TILE_WINDOW | TILE_NO_SHOOT,
    };
    column
        .pieces
        .iter()
        .find(|piece| {
            if piece.flags & stopping == 0 {
                return false;
            }
            let bottom = i16::from(piece.z);
            let top = bottom + piece.calc_height();
            bottom <= point_top
                && top >= z
                && (!at_end || bottom > end_top || top < i16::from(end.z))
        })
        .map(|piece| piece_blocker(piece.z, piece.calc_height(), piece.flags))
}

fn piece_blocker(z: i8, height: i16, flags: u32) -> SightBlocker {
    let bottom = i16::from(z);
    SightBlocker::Piece {
        bottom,
        top: bottom + height,
        flags,
    }
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

    #[test]
    fn a_window_stops_the_line_only_by_the_runuo_rules() {
        let map = MockMap::new(GRID, GRID);
        let mut glazed = Overlay::new(&map);
        wall_at(&mut glazed, 14, 10, TILE_WINDOW);
        let trace = |mode| sight_trace(&glazed, eyes_at(ARCHER), eyes_at(TARGET), mode);
        let runuo = trace(SightMode::RunUo);
        assert!(!runuo.in_sight);
        let stopped = runuo.points.iter().find(|p| p.blocker.is_some()).unwrap();
        assert_eq!((stopped.x, stopped.y), (14, 10));
        assert!(matches!(
            stopped.blocker,
            Some(SightBlocker::Piece {
                flags: TILE_WINDOW,
                ..
            })
        ));
        assert!(trace(SightMode::Pol).in_sight);
        assert!(trace(SightMode::Sphere).in_sight);
    }

    #[test]
    fn a_wall_stops_the_line_by_every_rule_and_open_ground_by_none() {
        let map = MockMap::new(GRID, GRID);
        let mut walled = Overlay::new(&map);
        wall_at(&mut walled, 14, 10, TILE_NO_SHOOT | TILE_WALL);
        for mode in [SightMode::RunUo, SightMode::Pol, SightMode::Sphere] {
            assert!(!sight_trace(&walled, eyes_at(ARCHER), eyes_at(TARGET), mode).in_sight);
            let open = sight_trace(&map, eyes_at(ARCHER), eyes_at(TARGET), mode);
            assert!(open.in_sight, "{mode:?}");
            assert!(!open.points.is_empty(), "{mode:?}");
        }
        let far = Point3::new(ARCHER.x + SIGHT_RANGE as u16 + 1, ARCHER.y, 0);
        let too_far = sight_trace(&map, eyes_at(ARCHER), eyes_at(far), SightMode::Sphere);
        assert_eq!(too_far.points[0].blocker, Some(SightBlocker::TooFar));
    }

    #[test]
    fn a_mode_is_read_from_its_name() {
        assert_eq!(SightMode::from_name("ModernUO"), Some(SightMode::RunUo));
        assert_eq!(SightMode::from_name(" pol "), Some(SightMode::Pol));
        assert_eq!(SightMode::from_name("sphere"), Some(SightMode::Sphere));
        assert_eq!(SightMode::from_name("uox"), None);
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
