//! Player houses and boats, and the tiles their walls close to a route.
//!
//! A building is built on the server and stands in no client map file, so the
//! map calls the ground under a house open and a route runs straight into its
//! wall. What the client does get is an ordinary item packet whose graphic
//! names a multi, and the shape of that multi is in its own files. This module
//! reads that packet, and turns the shape into the tiles a walk must go
//! around.

use std::collections::HashMap;

use uoterm_nav::{z_reachable, MultiData, MultiPiece, PERSON_HEIGHT, TILE_BRIDGE, TILE_DOOR};
use uoterm_protocol::types::{Point3, MULTI_ID_MASK};
use uoterm_protocol::GroundItem;
use uoterm_world::{DoorItem, MultiItem};

use crate::movement::door_on_tile;

/// The multi shape the item names, or `None` when it is an ordinary object.
///
/// Which of the two world item packets carried the item, and how each one says
/// "this is a building", is settled in the decoder. What arrives here is one
/// flag, so all this does is take the multi bit off the graphic and leave the
/// index of the shape in the client multi files.
pub fn multi_id(item: &GroundItem) -> Option<u16> {
    item.multi.then_some(item.graphic & MULTI_ID_MASK)
}

/// One piece of a building on one tile, at the height it really stands at.
///
/// The heights are counted in `i32` because a piece stands at the height of
/// the multi item plus its own offset, and that sum leaves the range a single
/// tile height is written in.
#[derive(Clone, Copy, Debug)]
struct PieceAt {
    z: i32,
    height: i32,
    flags: u32,
    blocks: bool,
    surface: bool,
}

impl PieceAt {
    fn new(multi_z: i8, piece: &MultiPiece) -> Self {
        Self {
            z: i32::from(multi_z) + i32::from(piece.dz),
            height: i32::from(piece.height),
            flags: piece.flags,
            blocks: piece.blocks(),
            surface: piece.surface(),
        }
    }

    /// The height a person ends up at once he has climbed onto this piece. A
    /// bridge is a stair or a gangplank, and a person meets it half way up.
    fn stands_at(&self) -> i32 {
        if self.flags & TILE_BRIDGE != 0 {
            self.z + self.height / 2
        } else {
            self.z + self.height
        }
    }

    /// True while this piece closes the space a person standing at `stand`
    /// fills.
    ///
    /// A door is left out: the leaf of a door is opened, not walked around,
    /// and the door logic owns the tile it stands on. This is the test the map
    /// reader makes of a static, which is private to it and cannot be shared.
    fn shuts_out(&self, stand: i32) -> bool {
        if !self.blocks || self.flags & TILE_DOOR != 0 {
            return false;
        }
        self.z + self.height > stand && self.z < stand + i32::from(PERSON_HEIGHT)
    }
}

/// Every height a person whose feet are at `feet_z` could stand at on one tile
/// of a building: his own floor, and every floor of the building he could step
/// up or down onto from it.
fn standing_heights(pieces: &[PieceAt], feet_z: i8) -> Vec<i32> {
    let mut heights = vec![i32::from(feet_z)];
    for piece in pieces.iter().filter(|p| p.surface) {
        let Ok(stand) = i8::try_from(piece.stands_at()) else {
            continue;
        };
        if z_reachable(feet_z, stand) {
            heights.push(i32::from(stand));
        }
    }
    heights
}

/// The tiles the buildings the character can see close to him on his own
/// floor.
///
/// Height is what keeps the storeys apart. A person is only shut out of a tile
/// when every floor of the building he could reach from where he stands is
/// closed to him, so the wall of an upper storey leaves the ground floor open
/// and the wall of the ground floor leaves the storey above open. It is also
/// what keeps a doorway passable: the footing that runs unbroken round a
/// house is below the floor boards a person steps up onto in the doorway, and
/// it shuts him out of no tile he can stand on.
///
/// A tile a door item stands on is never given back, whatever the shape says.
/// Those tiles belong to the door logic, which opens them; a building that
/// closed its own doorway would be sealed for good.
pub fn building_tiles(
    shapes: &MultiData,
    buildings: &[MultiItem],
    feet_z: i8,
    doors: &[DoorItem],
) -> Vec<Point3> {
    let shaped: Vec<(Point3, &[MultiPiece])> = buildings
        .iter()
        .map(|building| (building.location, shapes.pieces(building.multi_id)))
        .collect();
    shut_out_tiles(&shaped, feet_z, doors)
}

/// The same, from the shape of each building rather than its multi id, so the
/// rule can be read and tested without the client files.
fn shut_out_tiles(
    buildings: &[(Point3, &[MultiPiece])],
    feet_z: i8,
    doors: &[DoorItem],
) -> Vec<Point3> {
    let mut by_tile: HashMap<(u16, u16), Vec<PieceAt>> = HashMap::new();
    for (at, pieces) in buildings {
        for piece in *pieces {
            let x = i32::from(at.x) + i32::from(piece.dx);
            let y = i32::from(at.y) + i32::from(piece.dy);
            let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                continue;
            };
            by_tile
                .entry((x, y))
                .or_default()
                .push(PieceAt::new(at.z, piece));
        }
    }
    by_tile
        .into_iter()
        .filter(|((x, y), pieces)| {
            standing_heights(pieces, feet_z)
                .into_iter()
                .all(|stand| pieces.iter().any(|piece| piece.shuts_out(stand)))
                && door_on_tile(doors, *x, *y, feet_z).is_none()
        })
        .map(|((x, y), _)| Point3::new(x, y, feet_z))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_nav::{pathfind, MockMap, Obstacles, TILE_IMPASSABLE, TILE_SURFACE};
    use uoterm_protocol::types::Serial;

    /// The small stone house, multi id 0x0064, as the client files on a live
    /// machine hold it: 145 pieces of which 56 are solid, on 7 by 7 tiles.
    const STONE_HOUSE_ID: u16 = 0x0064;
    /// How far the wall of that house stands from the multi item itself.
    const HOUSE_HALF: i16 = 3;
    /// The doorway of that house: the middle of its south wall.
    const DOORWAY_DX: i16 = 0;
    const DOORWAY_DY: i16 = HOUSE_HALF;
    /// The footing of the house, measured from the client files: it runs
    /// unbroken round all four sides, under the doorway as well, and it is
    /// impassable and no surface.
    const FOOTING_DZ: i16 = 0;
    const FOOTING_HEIGHT: u8 = 5;
    /// The floor of the house stands on that footing, and it is the floor a
    /// person inside walks on.
    const FLOOR_DZ: i16 = 7;
    const FLOOR_HEIGHT: u8 = 0;
    /// The wall proper stands on the floor.
    const WALL_HEIGHT: u8 = 20;
    /// The storey above, and its own floor and wall.
    const UPPER_DZ: i16 = 27;
    /// Graphics only name the pieces apart in a failure message.
    const GRAPHIC_FOOTING: u16 = 0x0063;
    const GRAPHIC_FLOOR: u16 = 0x04AE;
    const GRAPHIC_WALL: u16 = 0x0203;
    const GRAPHIC_DOOR: u16 = 0x06A5;
    /// Where the test puts the house on the mock grid.
    const HOUSE_AT: Point3 = Point3 {
        x: 40,
        y: 40,
        z: GROUND_Z,
    };
    const GROUND_Z: i8 = 0;
    /// The floor a person inside the house stands on, and the one above it.
    const INSIDE_Z: i8 = FLOOR_DZ as i8;
    const UPSTAIRS_Z: i8 = UPPER_DZ as i8;
    const MOCK_GRID: u16 = 96;
    /// A walk that passes the house from one side to the other.
    const WEST_OF_THE_HOUSE: Point3 = Point3 {
        x: HOUSE_AT.x - HOUSE_HALF as u16 - 1,
        y: HOUSE_AT.y,
        z: GROUND_Z,
    };
    const EAST_OF_THE_HOUSE: Point3 = Point3 {
        x: HOUSE_AT.x + HOUSE_HALF as u16 + 1,
        y: HOUSE_AT.y,
        z: GROUND_Z,
    };
    /// A house standing on nobody's floor, so no door item is in the way.
    const NO_DOORS: &[DoorItem] = &[];

    fn piece(graphic: u16, dx: i16, dy: i16, dz: i16, height: u8, flags: u32) -> MultiPiece {
        MultiPiece {
            graphic,
            dx,
            dy,
            dz,
            height,
            flags,
        }
    }

    /// The shape of the small stone house, piece for piece as the client files
    /// describe it: a footing that runs unbroken round every side, a floor on
    /// top of it, and a wall on the floor with one gap for the doorway.
    fn stone_house() -> Vec<MultiPiece> {
        let mut pieces = Vec::new();
        for dy in -HOUSE_HALF..=HOUSE_HALF {
            for dx in -HOUSE_HALF..=HOUSE_HALF {
                let edge = dx.abs() == HOUSE_HALF || dy.abs() == HOUSE_HALF;
                if edge {
                    pieces.push(piece(
                        GRAPHIC_FOOTING,
                        dx,
                        dy,
                        FOOTING_DZ,
                        FOOTING_HEIGHT,
                        TILE_IMPASSABLE,
                    ));
                }
                pieces.push(piece(
                    GRAPHIC_FLOOR,
                    dx,
                    dy,
                    FLOOR_DZ,
                    FLOOR_HEIGHT,
                    TILE_SURFACE,
                ));
                if edge && (dx, dy) != (DOORWAY_DX, DOORWAY_DY) {
                    pieces.push(piece(
                        GRAPHIC_WALL,
                        dx,
                        dy,
                        FLOOR_DZ,
                        WALL_HEIGHT,
                        TILE_IMPASSABLE,
                    ));
                }
            }
        }
        pieces
    }

    /// The same house with a storey on top of it, so a test can prove that the
    /// wall up there leaves the ground floor open.
    fn two_storey_house() -> Vec<MultiPiece> {
        let mut pieces = stone_house();
        for dy in -HOUSE_HALF..=HOUSE_HALF {
            for dx in -HOUSE_HALF..=HOUSE_HALF {
                pieces.push(piece(
                    GRAPHIC_FLOOR,
                    dx,
                    dy,
                    UPPER_DZ,
                    FLOOR_HEIGHT,
                    TILE_SURFACE,
                ));
                if dx.abs() == HOUSE_HALF || dy.abs() == HOUSE_HALF {
                    pieces.push(piece(
                        GRAPHIC_WALL,
                        dx,
                        dy,
                        UPPER_DZ,
                        WALL_HEIGHT,
                        TILE_IMPASSABLE,
                    ));
                }
            }
        }
        pieces
    }

    /// The tile a piece at `dx`,`dy` of the test house stands on.
    fn tile(dx: i16, dy: i16) -> (u16, u16) {
        (
            (i32::from(HOUSE_AT.x) + i32::from(dx)) as u16,
            (i32::from(HOUSE_AT.y) + i32::from(dy)) as u16,
        )
    }

    fn walls(pieces: &[MultiPiece], feet_z: i8, doors: &[DoorItem]) -> Vec<Point3> {
        shut_out_tiles(&[(HOUSE_AT, pieces)], feet_z, doors)
    }

    fn shuts_out(tiles: &[Point3], dx: i16, dy: i16) -> bool {
        let (x, y) = tile(dx, dy);
        tiles.iter().any(|t| t.x == x && t.y == y)
    }

    /// The graphic of a house on the wire, and the graphic of an ordinary
    /// item. A server sets the multi bit on the graphic of a house on the legacy
    /// world item packet and masks every other item below that bit.
    const HOUSE_ON_THE_WIRE: u16 = STONE_HOUSE_ID | uoterm_protocol::types::ITEM_GRAPHIC_MULTI;
    /// A pile of logs, which is no building at all.
    const LOGS_ON_THE_WIRE: u16 = 0x1BDD;
    /// One item as the decoder hands it over, with only the fields this
    /// module reads.
    fn item(graphic: u16, multi: bool) -> GroundItem {
        GroundItem {
            serial: uoterm_protocol::Serial(1),
            graphic,
            amount: 1,
            x: 0,
            y: 0,
            z: 0,
            hue: 0,
            multi,
        }
    }

    /// Which packet carried the item, and how each one says "this is a
    /// building", is settled in the decoder. By the time an item reaches this
    /// module the answer is already one flag, so all this has to do is name
    /// the shape under it.
    #[test]
    fn a_multi_item_names_its_shape_and_an_ordinary_item_names_none() {
        assert_eq!(
            multi_id(&item(HOUSE_ON_THE_WIRE, true)),
            Some(STONE_HOUSE_ID),
            "the multi bit is masked off, and what is left is the shape"
        );
        assert_eq!(
            multi_id(&item(STONE_HOUSE_ID, true)),
            Some(STONE_HOUSE_ID),
            "a graphic that never carried the bit names the same shape"
        );
        assert_eq!(
            multi_id(&item(LOGS_ON_THE_WIRE, false)),
            None,
            "a pile of logs is no building"
        );
        assert_eq!(
            multi_id(&item(HOUSE_ON_THE_WIRE, false)),
            None,
            "and the flag decides it, not the graphic"
        );
    }

    #[test]
    fn the_solid_pieces_of_a_house_shut_a_route_out_of_its_walls() {
        let house = stone_house();
        let shut = walls(&house, GROUND_Z, NO_DOORS);
        for dy in -HOUSE_HALF..=HOUSE_HALF {
            for dx in -HOUSE_HALF..=HOUSE_HALF {
                let is_wall = (dx.abs() == HOUSE_HALF || dy.abs() == HOUSE_HALF)
                    && (dx, dy) != (DOORWAY_DX, DOORWAY_DY);
                assert_eq!(
                    shuts_out(&shut, dx, dy),
                    is_wall,
                    "the piece at {dx},{dy} of the house"
                );
            }
        }
        assert_eq!(
            shut.len(),
            walls(&house, INSIDE_Z, NO_DOORS).len(),
            "the wall is the same wall to a person on the floor of the house"
        );
    }

    /// The failure this guards against: the footing of a house runs unbroken
    /// round all four sides, doorway included, so a rule that only asked
    /// whether a solid piece stands on the tile would seal every house on the
    /// shard. What keeps the doorway open is the height: the footing is under
    /// the floor boards a person steps up onto there.
    #[test]
    fn the_doorway_of_a_house_stays_passable() {
        let house = stone_house();
        assert!(
            house.iter().any(|p| p.dx == DOORWAY_DX
                && p.dy == DOORWAY_DY
                && p.dz == FOOTING_DZ
                && p.blocks()),
            "the doorway must stand on the solid footing, or the test proves nothing"
        );
        for feet_z in [GROUND_Z, INSIDE_Z] {
            assert!(
                !shuts_out(&walls(&house, feet_z, NO_DOORS), DOORWAY_DX, DOORWAY_DY),
                "the doorway is open to a person whose feet are at {feet_z}"
            );
        }
    }

    /// A door item stands in its own doorway, and the door logic opens it. A
    /// building must never claim that tile, whatever its shape says, or the
    /// character walks around the door he has just opened.
    #[test]
    fn a_tile_a_door_item_stands_on_is_never_a_building_wall() {
        let house = stone_house();
        let wall_dx = HOUSE_HALF;
        let wall_dy = 0;
        let (x, y) = tile(wall_dx, wall_dy);
        assert!(
            shuts_out(&walls(&house, GROUND_Z, NO_DOORS), wall_dx, wall_dy),
            "that tile is a wall while no door stands on it"
        );
        let door = DoorItem {
            serial: Serial(0x4000_0001),
            graphic: GRAPHIC_DOOR,
            location: Point3::new(x, y, INSIDE_Z),
        };
        assert!(
            !shuts_out(&walls(&house, GROUND_Z, &[door]), wall_dx, wall_dy),
            "and the door logic owns it once the server sends the door"
        );
    }

    /// A house with two storeys must not put the wall of the bedroom in front
    /// of a person walking past the house on the ground.
    #[test]
    fn the_wall_of_the_storey_above_leaves_the_ground_floor_open() {
        let house = two_storey_house();
        let corner = (HOUSE_HALF, HOUSE_HALF);
        assert!(
            shuts_out(&walls(&house, UPSTAIRS_Z, NO_DOORS), corner.0, corner.1),
            "the wall of the storey above shuts a person upstairs out of it"
        );
        assert!(
            shuts_out(&walls(&house, UPSTAIRS_Z, NO_DOORS), DOORWAY_DX, DOORWAY_DY),
            "the wall up there runs over the doorway below, and shuts him out of it"
        );
        let ground = walls(&house, GROUND_Z, NO_DOORS);
        let one_storey = walls(&stone_house(), GROUND_Z, NO_DOORS);
        assert_eq!(
            ground.len(),
            one_storey.len(),
            "the storey above shuts a person on the ground out of nothing at all"
        );
        assert!(
            !shuts_out(&ground, DOORWAY_DX, DOORWAY_DY),
            "so the doorway underneath it is still open to him"
        );
    }

    /// The whole point of reading the shapes: a route planned around a house
    /// the character can see goes round it, and the map alone sends him
    /// straight through the wall.
    #[test]
    fn a_route_goes_round_a_house_and_not_through_it() {
        let map = MockMap::new(MOCK_GRID, MOCK_GRID);
        let straight = pathfind(&map, WEST_OF_THE_HOUSE, EAST_OF_THE_HOUSE, &Obstacles::NONE)
            .expect("the map alone calls the way open");
        let (through_x, through_y) = tile(0, 0);
        assert!(
            straight
                .steps
                .iter()
                .any(|s| s.x == through_x && s.y == through_y),
            "the map knows no house, so it walks through the middle of one: {:?}",
            straight.steps
        );

        let shut = walls(&stone_house(), GROUND_Z, NO_DOORS);
        // A wall is proven shut, which is what makes it hard.
        let around = pathfind(
            &map,
            WEST_OF_THE_HOUSE,
            EAST_OF_THE_HOUSE,
            &Obstacles {
                soft: &[],
                hard: &shut,
                moves: &[],
            },
        )
        .expect("a way round the house");
        for step in &around.steps {
            assert!(
                !shut.iter().any(|w| w.x == step.x && w.y == step.y),
                "no step of the walk stands in a wall: {:?}",
                around.steps
            );
        }
        assert_eq!(
            around.steps.last().map(|s| (s.x, s.y)),
            Some((EAST_OF_THE_HOUSE.x, EAST_OF_THE_HOUSE.y)),
            "and it comes out the far side: {:?}",
            around.steps
        );
        assert!(
            around.steps.len() > straight.steps.len(),
            "going round costs more than going through: {:?}",
            around.steps
        );
    }
}
