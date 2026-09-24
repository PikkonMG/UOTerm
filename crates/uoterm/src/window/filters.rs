//! The statics the play window treats in a way of its own, as the reference client
//! does: trees that become stumps, plants that hide, cave walls that are
//! marked, fields that stop moving, and the chairs a person sits on.

use eframe::egui::Vec2;
use std::ops::RangeInclusive;
use uoterm_nav::TileFlagSet;

/// The stump a tree shows as.
pub const STUMP_GRAPHIC: u16 = 0x0E59;
/// The flat tile a field shows as.
pub const FIELD_TILE_GRAPHIC: u16 = 0x1826;

const CAVE_TILES: RangeInclusive<u16> = 0x053B..=0x0553;
/// The one tile in the cave range that is no cave wall.
const NOT_CAVE: u16 = 0x0550;

/// The trees of the classic client. One that blocks the way is a tree and
/// becomes a stump; one that does not is only a plant.
const TREE_TILES: [u16; 62] = [
    0x0C95, 0x0C96, 0x0C99, 0x0C9B, 0x0C9C, 0x0C9D, 0x0C9E, 0x0CA6, 0x0CA8, 0x0CAA, 0x0CAB, 0x0CC9,
    0x0CCA, 0x0CCB, 0x0CCC, 0x0CCD, 0x0CD0, 0x0CD3, 0x0CD6, 0x0CD8, 0x0CDA, 0x0CDD, 0x0CE0, 0x0CE3,
    0x0CE6, 0x0CF8, 0x0CFB, 0x0CFE, 0x0D01, 0x0D37, 0x0D38, 0x0D41, 0x0D42, 0x0D43, 0x0D44, 0x0D57,
    0x0D58, 0x0D59, 0x0D5A, 0x0D5B, 0x0D6E, 0x0D6F, 0x0D70, 0x0D71, 0x0D72, 0x0D84, 0x0D85, 0x0D86,
    0x0D94, 0x0D98, 0x0D9C, 0x0DA0, 0x0DA4, 0x0DA8, 0x12B6, 0x12B7, 0x12B8, 0x12B9, 0x12BA, 0x12BB,
    0x12BC, 0x12BD,
];

/// The plants the player may hide. One that blocks the way stays.
const VEGETATION_TILES: [u16; 178] = [
    0x0C37, 0x0C38, 0x0C45, 0x0C46, 0x0C47, 0x0C48, 0x0C49, 0x0C4A, 0x0C4B, 0x0C4C, 0x0C4D, 0x0C4E,
    0x0C83, 0x0C84, 0x0C85, 0x0C86, 0x0C87, 0x0C88, 0x0C89, 0x0C8A, 0x0C8B, 0x0C8C, 0x0C8D, 0x0C8E,
    0x0C93, 0x0C94, 0x0C98, 0x0C9F, 0x0CA0, 0x0CA1, 0x0CA2, 0x0CA3, 0x0CA4, 0x0CA7, 0x0CAC, 0x0CAD,
    0x0CAE, 0x0CAF, 0x0CB0, 0x0CB1, 0x0CB2, 0x0CB3, 0x0CB4, 0x0CB5, 0x0CB6, 0x0CB9, 0x0CBA, 0x0CBC,
    0x0CBD, 0x0CBE, 0x0CBF, 0x0CC0, 0x0CC1, 0x0CC3, 0x0CC5, 0x0CC6, 0x0CC7, 0x0CE9, 0x0CF3, 0x0CF4,
    0x0CF5, 0x0CF6, 0x0CF7, 0x0D04, 0x0D06, 0x0D07, 0x0D08, 0x0D09, 0x0D0A, 0x0D0B, 0x0D0C, 0x0D0D,
    0x0D0E, 0x0D0F, 0x0D10, 0x0D11, 0x0D12, 0x0D13, 0x0D14, 0x0D15, 0x0D16, 0x0D17, 0x0D18, 0x0D19,
    0x0D28, 0x0D29, 0x0D2A, 0x0D2B, 0x0D2D, 0x0D2F, 0x0D32, 0x0D33, 0x0D34, 0x0D36, 0x0D3F, 0x0D40,
    0x0D45, 0x0D46, 0x0D47, 0x0D48, 0x0D49, 0x0D4A, 0x0D4B, 0x0D4C, 0x0D4D, 0x0D4E, 0x0D4F, 0x0D50,
    0x0D51, 0x0D52, 0x0D53, 0x0D54, 0x0D5C, 0x0D5D, 0x0D5E, 0x0D5F, 0x0D60, 0x0D61, 0x0D62, 0x0D63,
    0x0D64, 0x0D65, 0x0D66, 0x0D67, 0x0D68, 0x0D69, 0x0D6D, 0x0D73, 0x0D74, 0x0D75, 0x0D76, 0x0D77,
    0x0D78, 0x0D79, 0x0D7A, 0x0D7B, 0x0D7C, 0x0D7D, 0x0D7E, 0x0D7F, 0x0D80, 0x0D83, 0x0D87, 0x0D88,
    0x0D89, 0x0D8A, 0x0D8B, 0x0D8C, 0x0D8D, 0x0D8E, 0x0D8F, 0x0D90, 0x0D91, 0x0D93, 0x0DAE, 0x0DAF,
    0x0DBA, 0x0DBB, 0x0DBC, 0x0DBD, 0x0DBE, 0x0DC1, 0x0DC2, 0x0DC3, 0x12B6, 0x12B7, 0x12BC, 0x12BD,
    0x12BE, 0x12BF, 0x12C0, 0x12C1, 0x12C2, 0x12C3, 0x12C4, 0x12C5, 0x12C6, 0x12C7,
];

const ROCK_TILES: [u16; 9] = [4945, 4948, 4950, 4953, 4955, 4958, 4959, 4960, 4962];
const ROCK_RANGE: RangeInclusive<u16> = 6001..=6012;

/// Each kind of field, and the hue its flat tile takes.
const FIELDS: [(RangeInclusive<u16>, u16); 5] = [
    (0x398C..=0x399F, 0x0020),
    (0x3967..=0x397A, 0x0058),
    (0x3946..=0x3964, 0x0070),
    (0x3914..=0x3929, 0x0044),
    (0x0082..=0x0082, 0x038A),
];

/// A cave wall, which the player may mark with a border.
pub fn is_cave(graphic: u16) -> bool {
    CAVE_TILES.contains(&graphic) && graphic != NOT_CAVE
}

/// A tree that becomes a stump.
pub fn is_tree(graphic: u16, flags: TileFlagSet) -> bool {
    flags.contains(TileFlagSet::IMPASSABLE) && TREE_TILES.binary_search(&graphic).is_ok()
}

/// A plant the player may hide: it does not block the way.
pub fn is_vegetation(graphic: u16, flags: TileFlagSet) -> bool {
    !flags.contains(TileFlagSet::IMPASSABLE)
        && (VEGETATION_TILES.binary_search(&graphic).is_ok()
            || TREE_TILES.binary_search(&graphic).is_ok())
}

/// A rock, which throws a shadow as a tree does.
pub fn is_rock(graphic: u16) -> bool {
    ROCK_TILES.contains(&graphic) || ROCK_RANGE.contains(&graphic)
}

/// The hue of the flat tile a field shows as, or None for no field.
pub fn field_tile_hue(graphic: u16) -> Option<u16> {
    FIELDS
        .iter()
        .find(|(range, _)| range.contains(&graphic))
        .map(|(_, hue)| *hue)
}

/// How a person sits on one chair: the way he may face for each quarter,
/// or -1 when that quarter turns him to the next, and how far he sinks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Chair {
    graphic: u16,
    /// For a person who faces north, east, south and west.
    faces: [i8; 4],
    drop: i8,
    mirrored_drop: i8,
}

const fn chair(graphic: u16, faces: [i8; 4], drop: i8, mirrored_drop: i8) -> Chair {
    Chair {
        graphic,
        faces,
        drop,
        mirrored_drop,
    }
}

/// The chairs of the classic client, by graphic.
const CHAIRS: [Chair; 171] = [
    chair(0x0459, [0, -1, 4, -1], 2, 2),
    chair(0x045A, [-1, 2, -1, 6], 2, 2),
    chair(0x045B, [0, -1, 4, -1], 2, 2),
    chair(0x045C, [-1, 2, -1, 6], 2, 2),
    chair(0x0A2A, [0, 2, 4, 6], -4, -4),
    chair(0x0A2B, [0, 2, 4, 6], -8, -8),
    chair(0x0B2C, [-1, 2, -1, 6], 2, 2),
    chair(0x0B2D, [0, -1, 4, -1], 2, 2),
    chair(0x0B2E, [4, 4, 4, 4], 0, 0),
    chair(0x0B2F, [2, 2, 2, 2], 6, 6),
    chair(0x0B30, [6, 6, 6, 6], -8, 8),
    chair(0x0B31, [0, 0, 0, 0], 0, 4),
    chair(0x0B32, [4, 4, 4, 4], 0, 0),
    chair(0x0B33, [2, 2, 2, 2], 0, 0),
    chair(0x0B4E, [2, 2, 2, 2], 0, 0),
    chair(0x0B4F, [4, 4, 4, 4], 0, 0),
    chair(0x0B50, [0, 0, 0, 0], 0, 0),
    chair(0x0B51, [6, 6, 6, 6], 0, 0),
    chair(0x0B52, [2, 2, 2, 2], 0, 0),
    chair(0x0B53, [4, 4, 4, 4], 0, 0),
    chair(0x0B54, [0, 0, 0, 0], 0, 0),
    chair(0x0B55, [6, 6, 6, 6], 0, 0),
    chair(0x0B56, [2, 2, 2, 2], 4, 4),
    chair(0x0B57, [4, 4, 4, 4], 4, 4),
    chair(0x0B58, [6, 6, 6, 6], 0, 8),
    chair(0x0B59, [0, 0, 0, 0], 0, 8),
    chair(0x0B5A, [2, 2, 2, 2], 8, 8),
    chair(0x0B5B, [4, 4, 4, 4], 8, 8),
    chair(0x0B5C, [0, 0, 0, 0], 0, 8),
    chair(0x0B5D, [6, 6, 6, 6], 0, 8),
    chair(0x0B5E, [0, 2, 4, 6], -8, -8),
    chair(0x0B5F, [-1, 2, -1, 6], 3, 14),
    chair(0x0B60, [-1, 2, -1, 6], 3, 14),
    chair(0x0B61, [-1, 2, -1, 6], 3, 14),
    chair(0x0B62, [-1, 2, -1, 6], 3, 10),
    chair(0x0B63, [-1, 2, -1, 6], 3, 10),
    chair(0x0B64, [-1, 2, -1, 6], 3, 10),
    chair(0x0B65, [0, -1, 4, -1], 3, 10),
    chair(0x0B66, [0, -1, 4, -1], 3, 10),
    chair(0x0B67, [0, -1, 4, -1], 3, 10),
    chair(0x0B68, [0, -1, 4, -1], 3, 10),
    chair(0x0B69, [0, -1, 4, -1], 3, 10),
    chair(0x0B6A, [0, -1, 4, -1], 3, 10),
    chair(0x0B91, [4, 4, 4, 4], 6, 6),
    chair(0x0B92, [4, 4, 4, 4], 6, 6),
    chair(0x0B93, [2, 2, 2, 2], 6, 6),
    chair(0x0B94, [2, 2, 2, 2], 6, 6),
    chair(0x0CF3, [-1, 2, -1, 6], 2, 8),
    chair(0x0CF4, [-1, 2, -1, 6], 2, 8),
    chair(0x0CF6, [0, -1, 4, -1], 2, 8),
    chair(0x0CF7, [0, -1, 4, -1], 2, 8),
    chair(0x0E50, [4, 4, 4, 4], 4, 4),
    chair(0x0E51, [4, 4, 4, 4], 4, 4),
    chair(0x0E52, [2, 2, 2, 2], 0, 0),
    chair(0x0E53, [2, 2, 2, 2], 0, 0),
    chair(0x1049, [-1, 2, -1, 6], 2, 2),
    chair(0x104A, [0, -1, 4, -1], 2, 2),
    chair(0x11FC, [0, 2, 4, 6], 2, 7),
    chair(0x1207, [0, -1, 4, -1], 3, 10),
    chair(0x1208, [0, -1, 4, -1], 3, 10),
    chair(0x1209, [0, -1, 4, -1], 3, 10),
    chair(0x120A, [0, -1, 4, -1], 3, 10),
    chair(0x120B, [0, -1, 4, -1], 3, 10),
    chair(0x120C, [0, -1, 4, -1], 3, 10),
    chair(0x1218, [4, 4, 4, 4], 4, 4),
    chair(0x1219, [2, 2, 2, 2], 4, 4),
    chair(0x121A, [0, 0, 0, 0], 0, 8),
    chair(0x121B, [6, 6, 6, 6], 0, 8),
    chair(0x1527, [2, 2, 2, 2], 0, 0),
    chair(0x1771, [0, 2, 4, 6], 0, 0),
    chair(0x1776, [0, 2, 4, 6], 0, 0),
    chair(0x1779, [0, 2, 4, 6], 0, 0),
    chair(0x1DC7, [-1, 2, -1, 6], 3, 10),
    chair(0x1DC8, [-1, 2, -1, 6], 3, 10),
    chair(0x1DC9, [-1, 2, -1, 6], 3, 10),
    chair(0x1DCA, [0, -1, 4, -1], 3, 10),
    chair(0x1DCB, [0, -1, 4, -1], 3, 10),
    chair(0x1DCC, [0, -1, 4, -1], 3, 10),
    chair(0x1DCD, [-1, 2, -1, 6], 3, 10),
    chair(0x1DCE, [-1, 2, -1, 6], 3, 10),
    chair(0x1DCF, [-1, 2, -1, 6], 3, 10),
    chair(0x1DD0, [0, -1, 4, -1], 3, 10),
    chair(0x1DD1, [0, -1, 4, -1], 3, 10),
    chair(0x1DD2, [-1, 2, -1, 6], 3, 10),
    chair(0x2A58, [4, 4, 4, 4], 0, 0),
    chair(0x2A59, [2, 2, 2, 2], 0, 0),
    chair(0x2A5A, [0, 2, 4, 6], 0, 0),
    chair(0x2A5B, [0, 2, 4, 6], 10, 10),
    chair(0x2A7F, [0, 2, 4, 6], 0, 0),
    chair(0x2A80, [0, 2, 4, 6], 0, 0),
    chair(0x2DDF, [0, 2, 4, 6], 2, 2),
    chair(0x2DE0, [0, 2, 4, 6], 2, 2),
    chair(0x2DE3, [2, 2, 2, 2], 4, 4),
    chair(0x2DE4, [4, 4, 4, 4], 4, 4),
    chair(0x2DE5, [6, 6, 6, 6], 4, 4),
    chair(0x2DE6, [0, 0, 0, 0], 4, 4),
    chair(0x2DEB, [0, 0, 0, 0], 4, 4),
    chair(0x2DEC, [4, 4, 4, 4], 4, 4),
    chair(0x2DED, [2, 2, 2, 2], 4, 4),
    chair(0x2DEE, [6, 6, 6, 6], 4, 4),
    chair(0x2DF5, [0, 2, 4, 6], 4, 4),
    chair(0x2DF6, [0, 2, 4, 6], 4, 4),
    chair(0x3088, [0, 2, 4, 6], 4, 4),
    chair(0x3089, [0, 2, 4, 6], 4, 4),
    chair(0x308A, [0, 2, 4, 6], 4, 4),
    chair(0x308B, [0, 2, 4, 6], 4, 4),
    chair(0x319A, [-1, 2, -1, 6], 2, 2),
    chair(0x319B, [0, -1, 4, -1], 2, 2),
    chair(0x35ED, [0, 2, 4, 6], 0, 0),
    chair(0x35EE, [0, 2, 4, 6], 0, 0),
    chair(0x3DFF, [0, -1, 4, -1], 2, 2),
    chair(0x3E00, [-1, 2, -1, 6], 2, 2),
    chair(0x4023, [4, 4, 4, 4], 4, 4),
    chair(0x4024, [2, 2, 2, 2], 0, 0),
    chair(0x4027, [4, 4, 4, 4], 4, 4),
    chair(0x4028, [4, 4, 4, 4], 4, 4),
    chair(0x4029, [2, 2, 2, 2], 0, 0),
    chair(0x402A, [2, 2, 2, 2], 0, 0),
    chair(0x4BDC, [4, 4, 4, 4], 4, 4),
    chair(0x4C1B, [4, 4, 4, 4], 4, 4),
    chair(0x4C1E, [2, 2, 2, 2], 6, 6),
    chair(0x4C80, [4, 4, 4, 4], 4, 4),
    chair(0x4C81, [2, 2, 2, 2], 0, 0),
    chair(0x4C82, [4, 4, 4, 4], 4, 4),
    chair(0x4C83, [4, 4, 4, 4], 4, 4),
    chair(0x4C84, [2, 2, 2, 2], 0, 0),
    chair(0x4C85, [2, 2, 2, 2], 0, 0),
    chair(0x4C86, [4, 4, 4, 4], 4, 4),
    chair(0x4C87, [4, 4, 4, 4], 4, 4),
    chair(0x4C88, [2, 2, 2, 2], 0, 0),
    chair(0x4C89, [2, 2, 2, 2], 0, 0),
    chair(0x4C8A, [2, 2, 2, 2], 0, 0),
    chair(0x4C8B, [2, 2, 2, 2], 0, 0),
    chair(0x4C8C, [2, 2, 2, 2], 0, 0),
    chair(0x4C8D, [4, 4, 4, 4], 4, 4),
    chair(0x4C8E, [4, 4, 4, 4], 4, 4),
    chair(0x4C8F, [4, 4, 4, 4], 4, 4),
    chair(0x4DE0, [2, 2, 2, 2], 0, 0),
    chair(0x63BC, [0, -1, 4, -1], 3, 10),
    chair(0x63BD, [0, -1, 4, -1], 3, 10),
    chair(0x63C3, [-1, 2, -1, 6], 3, 14),
    chair(0x63C4, [-1, 2, -1, 6], 3, 14),
    chair(0x996C, [4, 4, 4, 4], 4, 4),
    chair(0x9977, [2, 2, 2, 2], 0, 0),
    chair(0x9C57, [6, 6, 6, 6], 6, 4),
    chair(0x9C58, [6, 6, 6, 6], 6, 4),
    chair(0x9C59, [0, 0, 0, 0], 4, 4),
    chair(0x9C5A, [0, 0, 0, 0], 4, 4),
    chair(0x9C5D, [6, 6, 6, 6], 6, 4),
    chair(0x9C5E, [6, 6, 6, 6], 6, 4),
    chair(0x9C5F, [6, 6, 6, 6], 6, 4),
    chair(0x9C60, [0, 0, 0, 0], 4, 4),
    chair(0x9C61, [0, 0, 0, 0], 4, 4),
    chair(0x9C62, [0, 0, 0, 0], 4, 4),
    chair(0x9E8E, [0, 0, 0, 0], 4, 4),
    chair(0x9E8F, [6, 6, 6, 6], 6, 4),
    chair(0x9E90, [2, 2, 2, 2], 0, 0),
    chair(0x9E91, [4, 4, 4, 4], 4, 4),
    chair(0x9E9F, [0, 0, 0, 0], 4, 4),
    chair(0x9EA0, [6, 6, 6, 6], 6, 4),
    chair(0x9EA1, [4, 4, 4, 4], 4, 4),
    chair(0x9EA2, [2, 2, 2, 2], 0, 0),
    chair(0xA05C, [6, 6, 6, 6], 6, 4),
    chair(0xA05D, [4, 4, 4, 4], 4, 4),
    chair(0xA05E, [0, 0, 0, 0], 4, 4),
    chair(0xA05F, [2, 2, 2, 2], 0, 0),
    chair(0xA211, [0, 2, 4, 6], -4, -4),
    chair(0xA4EA, [4, 4, 4, 4], 4, 4),
    chair(0xA4EB, [2, 2, 2, 2], 0, 0),
    chair(0xA586, [4, 4, 4, 4], 4, 4),
    chair(0xA587, [2, 2, 2, 2], 0, 0),
];

/// No way of a chair.
const NO_FACE: i8 = -1;
const QUARTERS: usize = 4;
const WAYS: u8 = 8;
/// A person sits lower than he stands.
const SIT_DROP: f32 = 4.0;
const SIT_SIDE: f32 = 8.0;
/// The reference client's pixel steps for each of the four ways a person sits.
const SIT_NORTH_DROP: f32 = 25.0;
const SIT_NORTH_SIDE: f32 = 4.0;
const SIT_EAST_DROP: f32 = 9.0;
const SIT_SOUTH_DROP: f32 = 10.0;
const SIT_SOUTH_SIDE: f32 = SIT_SIDE + 1.0;
const SIT_WEST_DROP: f32 = 23.0;
const SIT_WEST_SIDE: f32 = 3.0;
const FACING_NORTH: u8 = 0;
const FACING_EAST: u8 = 2;
const FACING_SOUTH: u8 = 4;
const FACING_WEST: u8 = 6;

/// How a person sits on a chair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seat {
    /// The way he faces: north, east, south or west.
    pub facing: u8,
    /// How far his picture moves from where he would stand.
    pub offset: Vec2,
    /// Seen from the back he takes the pose of a rider; seen from the front
    /// his standing picture is folded at the waist and the knees.
    pub from_back: bool,
}

/// The seat a person facing `direction` takes on the chair `graphic`, or
/// None when it is no chair.
pub fn seat(graphic: u16, direction: u8) -> Option<Seat> {
    let index = CHAIRS.binary_search_by_key(&graphic, |c| c.graphic).ok()?;
    let chair = CHAIRS[index];
    // Each quarter covers its way and the diagonal before it. A quarter
    // with no way turns a diagonal back to the way before it, and a straight
    // way on to the next, as the classic client does.
    let direction = usize::from(direction % WAYS);
    let quarter = direction.div_ceil(2) % QUARTERS;
    let face = chair.faces[quarter];
    let facing = if face != NO_FACE {
        face
    } else if direction % 2 == 1 {
        chair.faces[(quarter + QUARTERS - 1) % QUARTERS]
    } else {
        chair.faces[(quarter + 1) % QUARTERS]
    };
    let facing = u8::try_from(facing).ok()?;
    let (drop, side) = match facing {
        FACING_NORTH => (
            SIT_NORTH_DROP + f32::from(chair.mirrored_drop),
            SIT_NORTH_SIDE,
        ),
        FACING_EAST => (SIT_EAST_DROP + f32::from(chair.drop), 0.0),
        FACING_SOUTH => (SIT_SOUTH_DROP + f32::from(chair.drop), -SIT_SOUTH_SIDE),
        FACING_WEST => (
            SIT_WEST_DROP + f32::from(chair.mirrored_drop),
            -SIT_WEST_SIDE,
        ),
        _ => return None,
    };
    Some(Seat {
        facing,
        offset: Vec2::new(side, drop + SIT_DROP),
        from_back: matches!(facing, FACING_NORTH | FACING_WEST),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WOODEN_CHAIR: u16 = 0x0459;

    #[test]
    fn the_lists_are_sorted_so_a_search_finds_each_one() {
        assert!(TREE_TILES.windows(2).all(|w| w[0] < w[1]));
        assert!(VEGETATION_TILES.windows(2).all(|w| w[0] < w[1]));
        assert!(CHAIRS.windows(2).all(|w| w[0].graphic < w[1].graphic));
    }

    #[test]
    fn a_blocking_tree_is_a_tree_and_an_open_one_is_a_plant() {
        const OAK: u16 = 0x0CCA;
        assert!(is_tree(OAK, TileFlagSet::IMPASSABLE));
        assert!(!is_vegetation(OAK, TileFlagSet::IMPASSABLE));
        assert!(!is_tree(OAK, TileFlagSet::NONE));
        assert!(is_vegetation(OAK, TileFlagSet::NONE));
    }

    #[test]
    fn caves_rocks_and_fields_are_known() {
        assert!(is_cave(0x053B) && !is_cave(NOT_CAVE) && !is_cave(0x0554));
        assert!(is_rock(4945) && is_rock(6005) && !is_rock(4946));
        assert_eq!(field_tile_hue(0x3990), Some(0x0020));
        assert_eq!(field_tile_hue(0x0E75), None);
    }

    #[test]
    fn a_person_turns_to_a_way_the_chair_has() {
        // This chair seats a person facing north or south.
        let north = seat(WOODEN_CHAIR, FACING_NORTH).unwrap();
        assert_eq!(north.facing, FACING_NORTH);
        assert!(north.from_back);
        // East has no seat: he turns to the next way, south.
        assert_eq!(
            seat(WOODEN_CHAIR, FACING_EAST).unwrap().facing,
            FACING_SOUTH
        );
        // North-west has no own way and turns back to north.
        assert_eq!(seat(WOODEN_CHAIR, 7).unwrap().facing, FACING_NORTH);
        let south = seat(WOODEN_CHAIR, FACING_SOUTH).unwrap();
        assert!(!south.from_back && south.offset.y > SIT_DROP);
        assert!(seat(0x0E75, FACING_SOUTH).is_none());
    }
}
