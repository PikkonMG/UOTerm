//! The design of a house a player made, as tiles on the map. The shard
//! sends it in packed planes; this turns those planes into walls, floors
//! and doors with their places.
//!
//! A plane is packed in one of three ways. Mode 0 gives each tile its own
//! offsets. Mode 1 gives each tile an x and a y, and the level of the plane
//! gives the height. Mode 2 gives the tiles with no places at all: they lie
//! in rows across the ground of the house, and the bounds of the foundation
//! say where each row starts.

use serde::{Deserialize, Serialize};
use uoterm_protocol::{CustomHouse, HousePlane, Serial};

/// How high one level of a house is.
const LEVEL_HEIGHT: i32 = 20;
/// The floor of a level lies this far above its foundation.
const LEVEL_FLOOR: i32 = 7;
const LEVELS: u8 = 4;
const MODE_WITH_PLACES: u8 = 0;
const MODE_ONE_LEVEL: u8 = 1;
const MODE_ROWS: u8 = 2;
const TILE_WITH_PLACES: usize = 5;
const TILE_ONE_LEVEL: usize = 4;
const TILE_ROWS: usize = 2;
const WORD: usize = 2;

/// One piece of a house: a wall, a floor, a door, a teleporter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseTile {
    pub graphic: u16,
    /// From the tile the foundation stands on.
    pub dx: i32,
    pub dy: i32,
    pub dz: i32,
}

/// The corners of the foundation, from its multi. `min_x` and `min_y` are
/// the lowest offsets of its pieces, and `max_y` the highest `y`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl HouseBounds {
    /// The bounds of the pieces of a multi. None for a multi with none.
    pub fn of(pieces: impl IntoIterator<Item = (i16, i16)>) -> Option<Self> {
        let mut pieces = pieces.into_iter();
        let (x, y) = pieces.next()?;
        let mut bounds = Self {
            min_x: i32::from(x),
            min_y: i32::from(y),
            max_y: i32::from(y),
        };
        for (x, y) in pieces {
            bounds.min_x = bounds.min_x.min(i32::from(x));
            bounds.min_y = bounds.min_y.min(i32::from(y));
            bounds.max_y = bounds.max_y.max(i32::from(y));
        }
        Some(bounds)
    }
}

fn graphic_at(data: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([data[at], data[at + 1]])
}

/// The height of the floor of a level. Level 0 is the ground.
fn level_height(z_index: u8) -> i32 {
    match z_index {
        0 => 0,
        level => i32::from((level - 1) % LEVELS) * LEVEL_HEIGHT + LEVEL_FLOOR,
    }
}

/// Where the rows of a mode 2 plane start, and how long each row is.
fn row_shape(z_index: u8, bounds: HouseBounds) -> (i32, i32, i32) {
    let span = bounds.max_y - bounds.min_y;
    match z_index {
        0 => (bounds.min_x, bounds.min_y, span + 2),
        level if level <= LEVELS => (bounds.min_x + 1, bounds.min_y + 1, span),
        _ => (bounds.min_x, bounds.min_y, span + 1),
    }
}

fn plane_tiles(plane: &HousePlane, bounds: HouseBounds, out: &mut Vec<HouseTile>) {
    let data = &plane.data;
    let dz = level_height(plane.z_index);
    let step = match plane.mode {
        MODE_WITH_PLACES => TILE_WITH_PLACES,
        MODE_ONE_LEVEL => TILE_ONE_LEVEL,
        MODE_ROWS => TILE_ROWS,
        _ => return,
    };
    let (row_x, row_y, row_len) = row_shape(plane.z_index, bounds);
    if plane.mode == MODE_ROWS && row_len <= 0 {
        return;
    }
    for (i, tile) in data.chunks_exact(step).enumerate() {
        let graphic = graphic_at(tile, 0);
        if graphic == 0 {
            continue;
        }
        let place = match plane.mode {
            MODE_WITH_PLACES => (
                i32::from(tile[WORD] as i8),
                i32::from(tile[WORD + 1] as i8),
                dz + i32::from(tile[WORD + 2] as i8),
            ),
            MODE_ONE_LEVEL => (
                i32::from(tile[WORD] as i8),
                i32::from(tile[WORD + 1] as i8),
                dz,
            ),
            _ => (row_x + i as i32 / row_len, row_y + i as i32 % row_len, dz),
        };
        out.push(HouseTile {
            graphic,
            dx: place.0,
            dy: place.1,
            dz: place.2,
        });
    }
}

/// The tiles of a house the shard sent, from the tile its foundation
/// stands on.
pub fn house_tiles(house: &CustomHouse, bounds: HouseBounds) -> Vec<HouseTile> {
    let mut tiles = Vec::new();
    for plane in &house.planes {
        plane_tiles(plane, bounds, &mut tiles);
    }
    tiles
}

/// A house a player designed, ready to draw.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignedHouse {
    pub serial: Serial,
    pub revision: u32,
    pub tiles: Vec<HouseTile>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const WALL: u16 = 0x0064;
    const FLOOR: u16 = 0x0031;

    fn plane(mode: u8, z_index: u8, data: Vec<u8>) -> HousePlane {
        HousePlane {
            z_index,
            mode,
            data,
        }
    }

    fn house(planes: Vec<HousePlane>) -> CustomHouse {
        CustomHouse {
            serial: Serial(1),
            revision: 1,
            planes,
        }
    }

    const BOUNDS: HouseBounds = HouseBounds {
        min_x: -3,
        min_y: -3,
        max_y: 3,
    };

    #[test]
    fn a_plane_with_places_keeps_the_place_of_each_tile() {
        let mut data = WALL.to_be_bytes().to_vec();
        data.extend([1u8, 2, 7]);
        data.extend(0u16.to_be_bytes());
        data.extend([9u8, 9, 9]);
        let tiles = house_tiles(&house(vec![plane(MODE_WITH_PLACES, 0, data)]), BOUNDS);
        assert_eq!(
            tiles,
            vec![HouseTile {
                graphic: WALL,
                dx: 1,
                dy: 2,
                dz: 7
            }],
            "a tile with no graphic is no tile"
        );
    }

    #[test]
    fn a_plane_of_one_level_takes_the_height_of_that_level() {
        let mut data = WALL.to_be_bytes().to_vec();
        data.extend([(-2i8) as u8, 3]);
        let tiles = house_tiles(&house(vec![plane(MODE_ONE_LEVEL, 2, data)]), BOUNDS);
        assert_eq!(tiles[0].dz, LEVEL_HEIGHT + LEVEL_FLOOR);
        assert_eq!((tiles[0].dx, tiles[0].dy), (-2, 3));
        assert_eq!(level_height(0), 0);
        assert_eq!(level_height(1), LEVEL_FLOOR);
    }

    #[test]
    fn a_plane_of_rows_lays_its_tiles_across_the_ground() {
        // The ground level of a house 7 tiles deep has rows of 8.
        let (row_x, row_y, row_len) = row_shape(0, BOUNDS);
        assert_eq!((row_x, row_y, row_len), (-3, -3, 8));
        let mut data = Vec::new();
        for i in 0..10u16 {
            data.extend(if i == 3 { 0 } else { FLOOR }.to_be_bytes());
        }
        let tiles = house_tiles(&house(vec![plane(MODE_ROWS, 0, data)]), BOUNDS);
        assert_eq!(tiles.len(), 9, "the empty tile is left out");
        assert_eq!((tiles[0].dx, tiles[0].dy), (-3, -3));
        assert_eq!((tiles[1].dx, tiles[1].dy), (-3, -2));
        // The ninth tile starts the next row.
        assert_eq!((tiles[8].dx, tiles[8].dy), (-2, -2));
        assert_eq!(row_shape(2, BOUNDS), (-2, -2, 6), "a level is one smaller");
    }

    #[test]
    fn the_bounds_are_the_corners_of_the_pieces_of_the_foundation() {
        let bounds = HouseBounds::of([(-3, -4), (2, 5), (0, 0)]).unwrap();
        assert_eq!(
            bounds,
            HouseBounds {
                min_x: -3,
                min_y: -4,
                max_y: 5
            }
        );
        assert_eq!(HouseBounds::of([]), None);
    }
}
