//! Player houses and boats, and the items on the ground: what a walk must
//! weigh that stands in no client map file.
//!
//! A building is built on the server and stands in no client map file, so the
//! map calls the ground under a house open and a route runs straight into its
//! wall. What the client does get is an ordinary item packet whose graphic
//! names a multi, and the shape of that multi is in its own files, or the
//! design its owner built arrives on the wire. The same is true of a crate or a
//! table dropped on the ground. This module puts every piece of them on an
//! [`Overlay`] of the map, where the movement rules weigh each one as they
//! weigh a static: floors and stairs to stand on, walls and furniture to go
//! around, each at the height it stands at.

use uoterm_nav::{
    MultiPiece, Overlay, TilePiece, TileQuery, PERSON_HEIGHT, TILE_DOOR, TILE_IMPASSABLE,
    TILE_NO_SHOOT, TILE_SURFACE, TILE_WINDOW,
};
use uoterm_protocol::types::{Point3, ITEM_FLAG_MOVABLE, MULTI_ID_MASK};
use uoterm_protocol::GroundItem;
use uoterm_world::{DoorItem, HouseTile, Item};

/// The multi shape the item names, or `None` when it is an ordinary object.
///
/// Which of the two world item packets carried the item, and how each one says
/// "this is a building", is settled in the decoder. What arrives here is one
/// flag, so all this does is take the multi bit off the graphic and leave the
/// index of the shape in the client multi files.
pub fn multi_id(item: &GroundItem) -> Option<u16> {
    item.multi.then_some(item.graphic & MULTI_ID_MASK)
}

/// The shape of one building in view: the pieces of its multi, or the tiles
/// its owner designed, which take the place of the multi's own.
pub enum Shape<'a> {
    Multi(&'a [MultiPiece]),
    Designed(&'a [HouseTile]),
}

/// One piece of a building, as an offset from the building and the tiledata
/// flags and height of its graphic.
struct Placed {
    dx: i32,
    dy: i32,
    dz: i32,
    flags: u32,
    height: u8,
}

/// Puts every piece of the buildings in view on the overlay, at the height it
/// stands at.
///
/// A door item owns its doorway: a solid piece that fills the room a person
/// takes in that doorway is left out, because the door logic opens the door
/// and a building that closed its own doorway would be sealed for good.
pub fn add_buildings(
    overlay: &mut Overlay<'_>,
    buildings: &[(Point3, Shape<'_>)],
    doors: &[DoorItem],
) {
    for (at, shape) in buildings {
        let placed: Vec<Placed> = match shape {
            Shape::Multi(pieces) => pieces
                .iter()
                .map(|piece| Placed {
                    dx: i32::from(piece.dx),
                    dy: i32::from(piece.dy),
                    dz: i32::from(piece.dz),
                    flags: piece.flags,
                    height: piece.height,
                })
                .collect(),
            Shape::Designed(tiles) => tiles
                .iter()
                .filter_map(|tile| {
                    let (flags, height) = overlay.item_stat(tile.graphic)?;
                    Some(Placed {
                        dx: tile.dx,
                        dy: tile.dy,
                        dz: tile.dz,
                        flags,
                        height,
                    })
                })
                .collect(),
        };
        for piece in placed {
            let x = i32::from(at.x) + piece.dx;
            let y = i32::from(at.y) + piece.dy;
            let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                continue;
            };
            let z = clamp_z(i32::from(at.z) + piece.dz);
            let tile = TilePiece {
                z,
                height: piece.height,
                flags: piece.flags,
            };
            if !weighs(tile.flags) || closes_a_doorway(doors, x, y, tile) {
                continue;
            }
            overlay.add(x, y, tile);
        }
    }
}

/// Puts the items on the ground that a walk must weigh on the overlay.
///
/// A shard weighs them so: an item that cannot be picked up and is a surface
/// is a floor to stand on, and an impassable item blocks whether it can be
/// picked up or not. A surface a player could carry off holds nobody up.
pub fn add_ground_items<'i>(overlay: &mut Overlay<'_>, items: impl Iterator<Item = &'i Item>) {
    for item in items {
        let Some((mut flags, height)) = overlay.item_stat(item.graphic) else {
            continue;
        };
        if item.flags & ITEM_FLAG_MOVABLE != 0 {
            flags &= !TILE_SURFACE;
        }
        if !weighs(flags) {
            continue;
        }
        overlay.add(
            item.location.x,
            item.location.y,
            TilePiece {
                z: item.location.z,
                height,
                flags,
            },
        );
    }
}

/// True when a piece matters to a walk: something to stand on or something
/// in the way. Any other piece is only drawn.
fn weighs(flags: u32) -> bool {
    flags & (TILE_IMPASSABLE | TILE_SURFACE | TILE_DOOR | TILE_WINDOW | TILE_NO_SHOOT) != 0
}

/// True when a solid piece fills the room a person takes in a doorway a door
/// item stands in.
fn closes_a_doorway(doors: &[DoorItem], x: u16, y: u16, piece: TilePiece) -> bool {
    if piece.flags & TILE_IMPASSABLE == 0 || piece.flags & TILE_DOOR != 0 {
        return false;
    }
    let bottom = i16::from(piece.z);
    let top = bottom + i16::from(piece.height);
    doors.iter().any(|door| {
        let feet = i16::from(door.location.z);
        door.location.x == x
            && door.location.y == y
            && top > feet
            && bottom < feet + i16::from(PERSON_HEIGHT)
    })
}

/// Heights on the wire are one signed byte, and a building's own height plus
/// the offset of one of its pieces can leave that range.
fn clamp_z(z: i32) -> i8 {
    z.clamp(i32::from(i8::MIN), i32::from(i8::MAX)) as i8
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_nav::{pathfind, MockMap, Obstacles, TILE_BRIDGE};
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
            flags: 0,
            direction: 0,
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

    /// The step up to the doorway: a stair half as high as the footing, so a
    /// person on the ground climbs onto it and from it onto the floor.
    const DOORSTEP_HEIGHT: u8 = 5;
    const GRAPHIC_STEP: u16 = 0x0751;
    /// A table nobody can pick up, a crate a player could, and a platform
    /// fixed to the ground. Each has the tiledata it has in the client files.
    const GRAPHIC_TABLE: u16 = 0x0B90;
    const GRAPHIC_CRATE: u16 = 0x0E3D;
    const GRAPHIC_PLATFORM: u16 = 0x0708;
    const TABLE_HEIGHT: u8 = 6;
    const PLATFORM_HEIGHT: u8 = 1;

    /// The stone house with its doorstep, as a person meets it.
    fn house_with_a_step() -> Vec<MultiPiece> {
        let mut pieces = stone_house();
        pieces.push(piece(
            GRAPHIC_STEP,
            DOORWAY_DX,
            DOORWAY_DY + 1,
            FOOTING_DZ,
            DOORSTEP_HEIGHT,
            TILE_SURFACE | TILE_BRIDGE,
        ));
        pieces
    }

    fn overlay_of<'m>(map: &'m MockMap, pieces: &[MultiPiece], doors: &[DoorItem]) -> Overlay<'m> {
        let mut overlay = Overlay::new(map);
        add_buildings(&mut overlay, &[(HOUSE_AT, Shape::Multi(pieces))], doors);
        overlay
    }

    fn at(dx: i16, dy: i16, z: i8) -> Point3 {
        let (x, y) = tile(dx, dy);
        Point3::new(x, y, z)
    }

    fn inside_a_wall(step: &uoterm_nav::Step) -> bool {
        let dx = i32::from(step.x) - i32::from(HOUSE_AT.x);
        let dy = i32::from(step.y) - i32::from(HOUSE_AT.y);
        let half = i32::from(HOUSE_HALF);
        (dx.abs() == half || dy.abs() == half)
            && dx.abs() <= half
            && dy.abs() <= half
            && (dx, dy) != (i32::from(DOORWAY_DX), i32::from(DOORWAY_DY))
    }

    /// The whole point of reading the shapes: a route planned past a house
    /// the character can see goes round it, and the map alone sends him
    /// straight through the wall.
    #[test]
    fn a_route_goes_round_a_house_and_not_through_it() {
        let map = MockMap::new(MOCK_GRID, MOCK_GRID);
        let straight = pathfind(&map, WEST_OF_THE_HOUSE, EAST_OF_THE_HOUSE, &Obstacles::NONE)
            .expect("the map alone calls the way open");
        assert!(
            straight.steps.iter().any(inside_a_wall),
            "the map knows no house, so it walks through one: {:?}",
            straight.steps
        );
        let overlay = overlay_of(&map, &stone_house(), NO_DOORS);
        let around = pathfind(
            &overlay,
            WEST_OF_THE_HOUSE,
            EAST_OF_THE_HOUSE,
            &Obstacles::NONE,
        )
        .expect("a way round the house");
        assert!(
            !around.steps.iter().any(inside_a_wall),
            "no step of the walk stands in a wall: {:?}",
            around.steps
        );
        assert_eq!(
            around.steps.last().map(|s| (s.x, s.y)),
            Some((EAST_OF_THE_HOUSE.x, EAST_OF_THE_HOUSE.y)),
            "and it comes out the far side: {:?}",
            around.steps
        );
    }

    /// The footing of a house runs unbroken round all four sides, doorway
    /// included, so a rule that only asked whether a solid piece stands on a
    /// tile would seal every house on the shard. The height keeps the doorway
    /// open: the footing is under the floor a person steps up onto from the
    /// doorstep, and a route walks in over it.
    #[test]
    fn a_route_climbs_the_doorstep_and_walks_into_the_house() {
        let house = house_with_a_step();
        assert!(
            house.iter().any(|p| p.dx == DOORWAY_DX
                && p.dy == DOORWAY_DY
                && p.dz == FOOTING_DZ
                && p.blocks()),
            "the doorway must stand on the solid footing, or the test proves nothing"
        );
        let map = MockMap::new(MOCK_GRID, MOCK_GRID);
        let overlay = overlay_of(&map, &house, NO_DOORS);
        let outside = at(DOORWAY_DX, DOORWAY_DY + 2, GROUND_Z);
        let middle = at(0, 0, INSIDE_Z);
        let walk =
            pathfind(&overlay, outside, middle, &Obstacles::NONE).expect("the doorway lets him in");
        let (door_x, door_y) = tile(DOORWAY_DX, DOORWAY_DY);
        assert!(
            walk.steps
                .iter()
                .any(|s| (s.x, s.y, s.z) == (door_x, door_y, INSIDE_Z)),
            "he crosses the doorway on the floor of the house: {:?}",
            walk.steps
        );
        assert_eq!(walk.steps.last().map(|s| s.z), Some(INSIDE_Z));
    }

    /// A door item stands in its own doorway, and the door logic opens it. A
    /// building must never close that tile, whatever its shape says, or the
    /// character walks around the door he has just opened.
    #[test]
    fn a_tile_a_door_item_stands_on_is_never_a_building_wall() {
        let map = MockMap::new(MOCK_GRID, MOCK_GRID);
        let (wall_dx, wall_dy) = (HOUSE_HALF, 0);
        let (x, y) = tile(wall_dx, wall_dy);
        assert!(
            !overlay_of(&map, &stone_house(), NO_DOORS)
                .tile_from(INSIDE_Z, x, y)
                .walkable(),
            "that tile is a wall while no door stands on it"
        );
        let door = DoorItem {
            serial: Serial(0x4000_0001),
            graphic: GRAPHIC_DOOR,
            location: Point3::new(x, y, INSIDE_Z),
        };
        assert!(
            overlay_of(&map, &stone_house(), &[door])
                .tile_from(INSIDE_Z, x, y)
                .walkable(),
            "and the door logic owns it once the server sends the door"
        );
    }

    /// A house with two storeys puts the wall of the room above over the
    /// heads of people on the ground: they walk round it by the same way as
    /// round a house of one storey, and the floor inside stays open.
    #[test]
    fn the_wall_of_the_storey_above_leaves_the_ground_floor_open() {
        let map = MockMap::new(MOCK_GRID, MOCK_GRID);
        let one = overlay_of(&map, &stone_house(), NO_DOORS);
        let two = overlay_of(&map, &two_storey_house(), NO_DOORS);
        let around_one = pathfind(&one, WEST_OF_THE_HOUSE, EAST_OF_THE_HOUSE, &Obstacles::NONE)
            .expect("round the small house");
        let around_two = pathfind(&two, WEST_OF_THE_HOUSE, EAST_OF_THE_HOUSE, &Obstacles::NONE)
            .expect("round the tall house");
        assert_eq!(around_one.steps.len(), around_two.steps.len());
        let (x, y) = tile(1, 1);
        assert!(
            two.tile_from(INSIDE_Z, x, y).walkable(),
            "the floor below is open"
        );
        assert!(
            two.tile_from(UPSTAIRS_Z, x, y).walkable(),
            "and so is the floor above"
        );
    }

    fn ground_item(graphic: u16, x: u16, y: u16, flags: u8) -> Item {
        Item {
            serial: Serial(0x4000_0200),
            graphic,
            amount: 1,
            hue: 0,
            location: Point3::new(x, y, GROUND_Z),
            parent: None,
            layer: None,
            grid: 0,
            name: String::new(),
            flags,
        }
    }

    /// An item on the ground weighs as a shard weighs it: an impassable one
    /// blocks whether a player could carry it off or not, a fixed surface is
    /// a floor, and a surface a player could carry off holds nobody up.
    #[test]
    fn items_on_the_ground_block_or_hold_as_the_shard_rules() {
        let mut map = MockMap::new(MOCK_GRID, MOCK_GRID);
        map.set_item_stat(GRAPHIC_TABLE, TILE_IMPASSABLE, TABLE_HEIGHT);
        map.set_item_stat(GRAPHIC_CRATE, TILE_IMPASSABLE, TABLE_HEIGHT);
        map.set_item_stat(GRAPHIC_PLATFORM, TILE_SURFACE, PLATFORM_HEIGHT);
        const FIXED: u8 = 0;
        let items = [
            ground_item(GRAPHIC_TABLE, 10, 10, FIXED),
            ground_item(GRAPHIC_CRATE, 11, 10, ITEM_FLAG_MOVABLE),
            ground_item(GRAPHIC_PLATFORM, 12, 10, FIXED),
            ground_item(GRAPHIC_PLATFORM, 13, 10, ITEM_FLAG_MOVABLE),
        ];
        let mut overlay = Overlay::new(&map);
        add_ground_items(&mut overlay, items.iter());
        assert!(
            !overlay.can_walk_from(GROUND_Z, 10, 10),
            "a fixed table blocks"
        );
        assert!(
            !overlay.can_walk_from(GROUND_Z, 11, 10),
            "and so does a crate"
        );
        let raised = GROUND_Z + PLATFORM_HEIGHT as i8;
        assert_eq!(
            overlay.surface_near(raised, 12, 10),
            Some(raised),
            "a fixed platform is a floor to stand on"
        );
        assert_eq!(
            overlay.surface_near(raised, 13, 10),
            Some(GROUND_Z),
            "but one a player could carry off holds nobody up"
        );
    }
}
