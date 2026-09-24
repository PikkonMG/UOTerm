use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uoterm_protocol::{Direction, Point3};

use crate::step::{body_fits, check_step, LandCorners, TileColumn, TilePiece, PERSON_HEIGHT};

pub const TILE_IMPASSABLE: u32 = 0x0000_0040;
pub const TILE_WET: u32 = 0x0000_0080;
pub const TILE_SURFACE: u32 = 0x0000_0200;
pub const TILE_BRIDGE: u32 = 0x0000_0400;
pub const TILE_PARTIAL_HUE: u32 = 0x0004_0000;
pub const TILE_DOOR: u32 = 0x2000_0000;

/// Every flag a tiledata record can carry, by the name the client gives it.
/// Files older than High Seas hold the low 32 bits only.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileFlagSet(pub u64);

impl TileFlagSet {
    pub const NONE: Self = Self(0);
    pub const BACKGROUND: Self = Self(0x0000_0001);
    pub const WEAPON: Self = Self(0x0000_0002);
    pub const TRANSPARENT: Self = Self(0x0000_0004);
    pub const TRANSLUCENT: Self = Self(0x0000_0008);
    pub const WALL: Self = Self(0x0000_0010);
    pub const DAMAGING: Self = Self(0x0000_0020);
    pub const IMPASSABLE: Self = Self::low(TILE_IMPASSABLE);
    pub const WET: Self = Self::low(TILE_WET);
    pub const SURFACE: Self = Self::low(TILE_SURFACE);
    pub const BRIDGE: Self = Self::low(TILE_BRIDGE);
    /// Items of this graphic stack into one pile.
    pub const STACKABLE: Self = Self(0x0000_0800);
    pub const WINDOW: Self = Self::low(crate::sight::TILE_WINDOW);
    pub const NO_SHOOT: Self = Self::low(crate::sight::TILE_NO_SHOOT);
    pub const ARTICLE_A: Self = Self(0x0000_4000);
    pub const ARTICLE_AN: Self = Self(0x0000_8000);
    pub const INTERNAL: Self = Self(0x0001_0000);
    pub const FOLIAGE: Self = Self(0x0002_0000);
    pub const PARTIAL_HUE: Self = Self::low(TILE_PARTIAL_HUE);
    pub const NO_HOUSE: Self = Self(0x0008_0000);
    pub const MAP: Self = Self(0x0010_0000);
    pub const CONTAINER: Self = Self(0x0020_0000);
    pub const WEARABLE: Self = Self(0x0040_0000);
    pub const LIGHT_SOURCE: Self = Self(0x0080_0000);
    pub const ANIMATION: Self = Self::low(crate::animdata::TILE_ANIMATED);
    pub const NO_DIAGONAL: Self = Self(0x0200_0000);
    pub const ARMOR: Self = Self(0x0800_0000);
    pub const ROOF: Self = Self(0x1000_0000);
    pub const DOOR: Self = Self::low(TILE_DOOR);
    pub const STAIR_BACK: Self = Self(0x4000_0000);
    pub const STAIR_RIGHT: Self = Self(0x8000_0000);
    pub const ALPHA_BLEND: Self = Self(0x0001_0000_0000);
    pub const USE_NEW_ART: Self = Self(0x0002_0000_0000);
    pub const ART_USED: Self = Self(0x0004_0000_0000);
    pub const NO_SHADOW: Self = Self(0x0010_0000_0000);
    pub const PIXEL_BLEED: Self = Self(0x0020_0000_0000);
    pub const PLAY_ANIM_ONCE: Self = Self(0x0040_0000_0000);
    pub const MULTI_MOVABLE: Self = Self(0x0100_0000_0000);

    const fn low(bits: u32) -> Self {
        Self(bits as u64)
    }

    /// True when every flag of `other` is set here.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// The low 32 bits, which every file holds and the movement rules read.
    pub const fn low_bits(self) -> u32 {
        self.0 as u32
    }
}

impl std::ops::BitOr for TileFlagSet {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct TileInfo {
    pub land_id: u16,
    pub z: i8,
    pub land_z: i8,
    pub flags: u32,
    pub statics_impassable: bool,
    pub door: bool,
    pub wet: bool,
}

impl TileInfo {
    pub fn walkable(self) -> bool {
        self.door || (!self.statics_impassable && self.flags & TILE_IMPASSABLE == 0)
    }

    pub fn radar_char(self) -> char {
        if self.door {
            '+'
        } else if self.walkable() {
            '.'
        } else if self.wet {
            '~'
        } else {
            '#'
        }
    }
}

/// One static item on a tile, with the tiledata name that tells an agent what
/// the item is. `TileInfo` says if a tile is walkable; this says what is there.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StaticView {
    pub graphic: u16,
    pub name: String,
    pub z: i8,
    pub height: u8,
    pub flags: u32,
    /// The color the map files give this static. Zero is the art's own color.
    #[serde(default)]
    pub hue: u16,
}

impl StaticView {
    pub fn impassable(&self) -> bool {
        self.flags & TILE_IMPASSABLE != 0
    }

    pub fn surface(&self) -> bool {
        self.flags & TILE_SURFACE != 0
    }

    pub fn bridge(&self) -> bool {
        self.flags & TILE_BRIDGE != 0
    }

    pub fn door(&self) -> bool {
        self.flags & TILE_DOOR != 0
    }
}

/// True while a floor at `stand` belongs to the same storey as a person whose
/// feet are at `from`.
///
/// This answers "which floor is his", not "can he climb there". A person is
/// sixteen units tall, so a floor further than his own height above or below
/// him is a floor of another storey and no business of his. How far he climbs
/// in one step is [`crate::STEP_HEIGHT`], which is a much smaller number and
/// belongs to the step check alone.
pub fn z_reachable(from_z: i8, to_z: i8) -> bool {
    same_storey(i16::from(from_z), i16::from(to_z))
}

fn same_storey(from: i16, stand: i16) -> bool {
    (stand - from).abs() <= i16::from(PERSON_HEIGHT)
}

/// The floor of this tile a person can stand on, and the highest one when the
/// tile holds more than one.
///
/// `from_z` names the storey the answer must belong to. Without it every floor
/// of the tile is on offer and the topmost wins, which is what a map drawn
/// from above shows.
fn standing_height(column: &TileColumn, from_z: Option<i8>) -> Option<i16> {
    let mut best: Option<i16> = None;
    let mut consider = |stand: i16| {
        if let Some(from) = from_z {
            if !same_storey(i16::from(from), stand) {
                return;
            }
        }
        if !body_fits(column, stand, stand + i16::from(PERSON_HEIGHT)) {
            return;
        }
        best = Some(best.map_or(stand, |found: i16| found.max(stand)));
    };
    if column.consider_land() && !column.land_blocks() {
        consider(column.land.center());
    }
    for piece in column.pieces.iter().filter(|p| p.is_standing_surface()) {
        consider(piece.stands_at());
    }
    best
}

/// Reads one tile as the person named by `from_z` finds it, or as a map drawn
/// from above shows it when no height is named.
fn resolve_tile(column: &TileColumn, from_z: Option<i8>) -> TileInfo {
    let door = column.has_door();
    let wet =
        column.land_flags & TILE_WET != 0 || column.pieces.iter().any(|p| p.flags & TILE_WET != 0);
    // A doorway is a way through. Its leaf is opened, not walked around, so a
    // person crossing it stands on the ground under it.
    let land_center = column.land.center();
    let stand = standing_height(column, from_z).or_else(|| match from_z {
        _ if !door => None,
        Some(from) if !z_reachable(from, clamp_z(land_center)) => None,
        _ => Some(land_center),
    });
    let mut flags = column.land_flags;
    if stand.is_none() {
        flags |= TILE_IMPASSABLE;
    } else if flags & TILE_IMPASSABLE != 0 {
        flags &= !TILE_IMPASSABLE;
        flags |= TILE_SURFACE;
    }
    TileInfo {
        land_id: column.land_id,
        z: clamp_z(stand.unwrap_or(land_center)),
        land_z: column.land.z(),
        flags,
        statics_impassable: stand.is_none() && !door,
        door,
        wet,
    }
}

/// Heights on the wire are one signed byte. A tile the client files describe
/// never leaves that range, and a sum of two of them can.
fn clamp_z(z: i16) -> i8 {
    z.clamp(i16::from(i8::MIN), i16::from(i8::MAX)) as i8
}

pub trait TileQuery {
    /// The land under one tile and every item standing on it, which is all the
    /// movement rules read.
    fn column(&self, x: u16, y: u16) -> TileColumn;
    fn width(&self) -> u16;
    fn height(&self) -> u16;

    /// Every static on the tile, lowest z first. Maps without static data
    /// report none.
    fn statics_at(&self, _x: u16, _y: u16) -> Vec<StaticView> {
        Vec::new()
    }

    /// The tiledata name of the land tile. Maps without tiledata report blank.
    fn land_name(&self, _x: u16, _y: u16) -> String {
        String::new()
    }

    /// The tiledata name of an item graphic, such as "barrel". Maps without
    /// tiledata report blank.
    fn item_name(&self, _graphic: u16) -> String {
        String::new()
    }

    /// The tiledata flags and height of an item graphic, which is what an
    /// item on the ground is to the movement rules. Maps without tiledata
    /// know none.
    fn item_stat(&self, _graphic: u16) -> Option<(u32, u8)> {
        None
    }

    fn in_bounds(&self, x: u16, y: u16) -> bool {
        x < self.width() && y < self.height()
    }

    fn tile(&self, x: u16, y: u16) -> TileInfo {
        resolve_tile(&self.column(x, y), None)
    }

    /// The tile as a person who stands at `from_z` finds it. A building with
    /// more than one floor gives one answer for each floor at the same x,y.
    /// Only the height of the person tells which floor is his. A map with one
    /// floor answers the same as `tile`.
    fn tile_from(&self, from_z: i8, x: u16, y: u16) -> TileInfo {
        resolve_tile(&self.column(x, y), Some(from_z))
    }

    /// The height of the walkable surface on the tile nearest to `near_z`,
    /// at any height: the ground, a floor, a porch or a roof. A person who
    /// points at a spot names no height; this is the spot he means. None
    /// when nothing on the tile can be stood on.
    fn surface_near(&self, near_z: i8, x: u16, y: u16) -> Option<i8> {
        if !self.in_bounds(x, y) {
            return None;
        }
        let column = self.column(x, y);
        let tops = column
            .pieces
            .iter()
            .map(|p| i16::from(p.z) + i16::from(p.height));
        std::iter::once(column.land.center())
            .chain(tops)
            .filter_map(|top| {
                let tile = resolve_tile(&column, Some(clamp_z(top)));
                tile.walkable().then_some(tile.z)
            })
            .min_by_key(|z| (i16::from(*z) - i16::from(near_z)).abs())
    }

    fn can_walk(&self, x: u16, y: u16) -> bool {
        self.in_bounds(x, y) && self.tile(x, y).walkable()
    }

    /// `can_walk` for a person who stands at `from_z`. Ask this, not
    /// `can_walk`, whenever the height of the person is known.
    fn can_walk_from(&self, from_z: i8, x: u16, y: u16) -> bool {
        self.in_bounds(x, y) && self.tile_from(from_z, x, y).walkable()
    }

    /// The height a person standing at `from` reaches by stepping onto the
    /// tile beside him, or `None` when that step is not one he can take.
    ///
    /// This is the whole movement test, corner rule included, so a caller
    /// needs nothing else to know whether one step is legal. The ground of the
    /// tile he leaves is read at the edge he crosses, which is the way the
    /// client works it out and the stricter of the two: a step this refuses is
    /// never one a shard would have allowed him to take from that edge.
    fn can_step(&self, from: Point3, to_x: u16, to_y: u16) -> Option<i8> {
        let dx = i32::from(to_x) - i32::from(from.x);
        let dy = i32::from(to_y) - i32::from(from.y);
        if (dx, dy) == (0, 0) || dx.abs() > 1 || dy.abs() > 1 {
            return None;
        }
        check_step(self, from, Direction::from_delta(dx, dy))
    }
}

/// A map with the things on it that its files do not hold: the pieces of the
/// houses and boats in view, and the items on the ground a person stands on or
/// cannot pass. Each tile reads the column of the map with these pieces added,
/// so the movement rules weigh them exactly as they weigh a static.
pub struct Overlay<'a> {
    base: &'a dyn TileQuery,
    pieces: HashMap<(u16, u16), Vec<TilePiece>>,
}

impl<'a> Overlay<'a> {
    pub fn new(base: &'a dyn TileQuery) -> Self {
        Self {
            base,
            pieces: HashMap::new(),
        }
    }

    /// Puts one more piece on a tile.
    pub fn add(&mut self, x: u16, y: u16, piece: TilePiece) {
        self.pieces.entry((x, y)).or_default().push(piece);
    }

    /// True when anything was put on that tile.
    pub fn has_pieces(&self, x: u16, y: u16) -> bool {
        self.pieces.contains_key(&(x, y))
    }
}

impl TileQuery for Overlay<'_> {
    fn column(&self, x: u16, y: u16) -> TileColumn {
        let mut column = self.base.column(x, y);
        if let Some(extra) = self.pieces.get(&(x, y)) {
            column.pieces.extend_from_slice(extra);
        }
        column
    }

    fn width(&self) -> u16 {
        self.base.width()
    }

    fn height(&self) -> u16 {
        self.base.height()
    }

    fn statics_at(&self, x: u16, y: u16) -> Vec<StaticView> {
        self.base.statics_at(x, y)
    }

    fn land_name(&self, x: u16, y: u16) -> String {
        self.base.land_name(x, y)
    }

    fn item_name(&self, graphic: u16) -> String {
        self.base.item_name(graphic)
    }

    fn item_stat(&self, graphic: u16) -> Option<(u32, u8)> {
        self.base.item_stat(graphic)
    }
}

/// How tall the leaf of a mock door stands.
const MOCK_DOOR_HEIGHT: u8 = 20;

#[derive(Clone, Debug)]
pub struct MockMap {
    width: u16,
    height: u16,
    blocked: Vec<bool>,
    doors: Vec<bool>,
    wet: Vec<bool>,
    z: Vec<i8>,
    item_stats: HashMap<u16, (u32, u8)>,
}

impl MockMap {
    pub fn new(width: u16, height: u16) -> Self {
        let n = width as usize * height as usize;
        Self {
            width,
            height,
            blocked: vec![false; n],
            doors: vec![false; n],
            wet: vec![false; n],
            z: vec![0; n],
            item_stats: HashMap::new(),
        }
    }

    /// Gives an item graphic the tiledata flags and height a test needs.
    pub fn set_item_stat(&mut self, graphic: u16, flags: u32, height: u8) {
        self.item_stats.insert(graphic, (flags, height));
    }

    fn idx(&self, x: u16, y: u16) -> usize {
        y as usize * self.width as usize + x as usize
    }

    pub fn set_block(&mut self, x: u16, y: u16, blocked: bool) {
        let i = self.idx(x, y);
        self.blocked[i] = blocked;
    }

    pub fn set_door(&mut self, x: u16, y: u16, door: bool) {
        let i = self.idx(x, y);
        self.doors[i] = door;
    }

    pub fn set_wet(&mut self, x: u16, y: u16, wet: bool) {
        let i = self.idx(x, y);
        self.wet[i] = wet;
    }

    pub fn set_z(&mut self, x: u16, y: u16, z: i8) {
        let i = self.idx(x, y);
        self.z[i] = z;
    }
}

impl TileQuery for MockMap {
    /// A mock cell is flat: it has one height, not four corners to average.
    fn column(&self, x: u16, y: u16) -> TileColumn {
        if !self.in_bounds(x, y) {
            return TileColumn::off_map();
        }
        let i = self.idx(x, y);
        let z = self.z[i];
        let mut land_flags = if self.blocked[i] {
            TILE_IMPASSABLE
        } else {
            TILE_SURFACE
        };
        if self.wet[i] {
            land_flags |= TILE_WET;
        }
        TileColumn {
            land_id: 0,
            land_flags,
            land: LandCorners::flat(z),
            // A door is a leaf standing on the ground, and a shut leaf fills
            // the space a body would. It is the door flag alone that lets a
            // person through it.
            pieces: if self.doors[i] {
                vec![TilePiece {
                    z,
                    height: MOCK_DOOR_HEIGHT,
                    flags: TILE_DOOR | TILE_IMPASSABLE,
                }]
            } else {
                Vec::new()
            },
        }
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        self.height
    }

    fn item_stat(&self, graphic: u16) -> Option<(u32, u8)> {
        self.item_stats.get(&graphic).copied()
    }
}
