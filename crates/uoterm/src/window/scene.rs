//! The map behind the panels: the land, the things on it, the people, and
//! the way to the walk goal. It draws the real pictures when the client
//! files are there, and flat colors from the radar when they are not.

use super::atlas::{Atlas, Sprite};
use super::audio::Step;
use super::classic::text::TextLook;
use super::client_art::{is_drawn, Cell, ClientArt, ItemPaint};
use super::figure::{is_mounted, Paint, Pose};
use super::filters::{self, Seat};
use super::lights::{flicker, shown_light_color, world_light, LightMap, LightRules, LightSource};
use super::link::POLL_MS_SAME_PROGRAM;
use super::look::{self, HitsShown, MobileState, PlateOf, WorldLook};
use super::model::house_design::{kind_of, piece_look, PieceLook, StoreyLook, STOREYS};
use super::predict::WalkPrediction;
use super::settings::{CircleStyle, FieldStyle, Profile};
use super::theme;
use crate::view::{
    WatchCueKind, WatchFrame, WatchItem, WatchLook, WatchMobile, WatchStride, SYM_BLOCK, SYM_DOOR,
    SYM_WALK, SYM_WATER,
};
use eframe::egui::{
    self,
    epaint::{Mesh, Vertex},
    Align2, Color32, Galley, Painter, Pos2, Rect, Shape, Stroke, Vec2,
};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use uoterm_nav::{
    item_light, Action, CursorShape, Deed, Facing, LightHolder, TileFlagSet,
    UNICODE_PICTURE_PADDING,
};

/// Half the side of a tile picture. One tile step moves this far on each
/// screen axis.
const HALF_TILE: f32 = 22.0;
/// A tile's edge lies this far from its middle, in tiles.
const HALF_TILE_STEP: f32 = 0.5;
/// The width of the range circle's line.
const RANGE_LINE_WIDTH: f32 = 2.0;
/// A piece on a see-through storey of the house being designed shows at
/// this share of its opacity.
const DESIGN_SEE_THROUGH: f32 = 0.5;
/// One unit of height lifts a thing this many pixels.
const Z_PIXELS: f32 = 4.0;
/// The classic client cuts the world over the character from this far
/// above his feet, and takes him to be this tall.
const HEAD_ROOM: i16 = 14;
const BODY_HEIGHT: i16 = 16;
/// Nothing is cut away over the character.
const NO_CEILING: i16 = 127;
/// A wall in front of a light this far over it hides the light.
const LIGHT_WALL_RISE: i16 = 5;
/// The season from which plants lose their leaves.
const SEASON_WINTER: u8 = 3;
/// The longest step of time one frame fades things by.
const MAX_FRAME_SECONDS: f64 = 0.25;
/// How long the death screen shows, and what it says.
const DEATH_SCREEN_SECONDS: f64 = 1.5;
const DEATH_WORDS: &str = "You are dead.";
const DEATH_FONT: u8 = 3;
const DEATH_HUE: u16 = 0;
/// The ring of color under the feet: its size, and how far over the feet
/// its middle is.
const AURA_RADIUS: f32 = 40.0;
const AURA_LIFT: f32 = 5.0;
/// A shadow is this dark, and its foot is this far over the foot of the
/// picture.
const SHADOW_ALPHA: f32 = 0.4;
const SHADOW_RISE: f32 = 10.0;
const SHADOW_SQUASH: f32 = 0.5;
/// A person on a chair leans his upper body this far, and is folded at
/// these shares of his height. His feet under the last one do not show.
const SIT_LEAN: f32 = 8.0;
const SIT_FOLDS: [f32; 4] = [0.0, 0.35, 0.60, 0.94];
/// A person seen from the back on a chair takes this pose of a rider.
const SIT_FROM_BACK_GROUP: u8 = 25;
/// A chair is under a person this near his feet.
const SEAT_REACH: i16 = 1;
/// Water that moves grows and shrinks by these shares.
const WATER_GROW: f32 = 1.1;
const WATER_SWAY: f32 = 0.1;
const WATER_SWAY_DOWN: f32 = 0.05;
/// The Video page counts terrain shadows in tenths.
const TERRAIN_SHADOWS_STEP: f32 = 0.1;
/// A circle of transparency is cut in cells of this many points.
const CIRCLE_CELL: f32 = 8.0;
/// The hit points line of the classic client, under the feet.
const HITS_BACK_GUMP: u16 = 0x1068;
const HITS_FILL_GUMP: u16 = 0x1069;
const HITS_LINE_SIZE: Vec2 = Vec2::new(34.0, 8.0);
const HITS_LINE_DROP: f32 = 5.0;
const HITS_LOST_HUE: u16 = 0x0021;
const HITS_FILL_HUE: u16 = 0x005A;
const HITS_POISON_HUE: u16 = 0x003F;
const HITS_YELLOW_HUE: u16 = 0x0035;
/// A line of a mobile the character does not fight is this faint.
const HITS_PASSIVE_ALPHA: f32 = 0.5;
/// The hit points in words over a head, in an ASCII font.
const HITS_FONT: u8 = 3;
const HITS_RISE: f32 = 8.0;
const NAMEPLATE_FONT: u8 = 1;
const NAMEPLATE_PAD: f32 = 2.0;
const ITEM_NAME_HUE: u16 = 0x03B2;
const PERCENT_MAX: u8 = 100;
/// The bits of a direction byte that name the way.
const DIRECTION_MASK: u8 = 0x07;
const CORPSE_GRAPHIC: u16 = 0x2006;
/// Where the quest arrow stands, and the way it points. A place in view
/// keeps its own spot; one out of view is held at the edge of the window,
/// as a compass needle is.
fn arrow_at(rect: Rect, at: Pos2) -> (Pos2, Vec2) {
    let room = rect.shrink(ARROW_EDGE);
    let point = Pos2::new(
        at.x.clamp(room.left(), room.right()),
        at.y.clamp(room.top(), room.bottom()),
    );
    let away = at - rect.center();
    let away = if away.length() > f32::EPSILON {
        away.normalized()
    } else {
        Vec2::new(0.0, -1.0)
    };
    (point, away)
}

/// The room the quest arrow keeps from the edge of the window.
const ARROW_EDGE: f32 = 28.0;
const ARROW_LENGTH: f32 = 22.0;
const ARROW_WIDTH: f32 = 7.0;
const ARROW_EDGE_WIDTH: f32 = 1.5;
/// A paperdoll faces the watcher.
pub const DOLL_FACING: u8 = 4;

const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 3.0;
const ZOOM_START: f32 = 1.0;
const ZOOM_PER_SCROLL_POINT: f32 = 0.0015;
/// The paces of the game: the time of one tile on foot and on a mount.
const STEP_SECONDS_FOOT_WALK: f64 = 0.4;
const STEP_SECONDS_FOOT_RUN: f64 = 0.2;
const STEP_SECONDS_MOUNT_WALK: f64 = 0.2;
const STEP_SECONDS_MOUNT_RUN: f64 = 0.1;
/// How many tiles wait while a move is under way.
const GLIDE_QUEUE_CAP: usize = 4;
/// A move with no tile queued takes this much longer than its pace.
const GLIDE_STRETCH_ALONE: f64 = 1.02;
/// A gap of this many paces, or more, is a rest between two walks.
const NEWS_REST_FACTOR: f64 = 2.0;
/// How much of each new gap goes into the learned rhythm.
const NEWS_LEARN_SHARE: f64 = 0.35;
/// Each queued tile makes the next move this much faster. A step the
/// session sent catches up by as much for each step it is late.
const GLIDE_CATCH_UP_PER_TILE: f64 = 0.08;
/// A step the session sent starts where the one before it ends when its
/// slot comes this near that end. The clock of the window reads the slot a
/// little off; the shortest real pause between two steps, a turn, is five
/// times longer.
const STRIDE_JOIN_SECONDS: f64 = 0.02;
/// The news of a step reaches the window up to one poll of the session
/// after the step, and then this much later at most: the call itself, and
/// the wait for the next frame of the window. The character takes each step
/// that long after its slot, so the news of the next step always comes
/// before this one ends, and the steps meet end to end.
const STRIDE_READ_MARGIN_SECONDS: f64 = 0.05;
/// The next place comes a moment after a move ends. A mobile keeps his walk
/// for this long, so he does not stand still for one frame between two tiles.
const STEP_LINGER_SECONDS: f64 = 0.12;
/// How long one picture of each action shows.
const STAND_FRAME_SECONDS: f64 = 0.35;
/// The least time between two footstep sounds of one walker, as the game
/// client has it.
const STEP_GAP_ON_FOOT: f64 = 0.52;
const STEP_GAP_MOUNT_RUN: f64 = 0.195;
const STEP_GAP_MOUNT_WALK: f64 = 0.455;
/// A walker who made no step for this long is forgotten.
const STEP_MEMORY_SECONDS: f64 = 5.0;
/// The character has no serial in the frame. No mobile has this one.
const SELF_STEP_KEY: u32 = 0;
/// One picture of a walk or a run shows for this long at the full pace of
/// the game. The legs go by the ground that was covered, not by the clock:
/// a slow mobile has slow legs, and a mobile that stops has still legs.
const STEP_FRAME_SECONDS: f64 = 0.08;
/// A jump longer than this is a teleport. The camera does not slide over it.
const TELEPORT_TILES: f32 = 12.0;

// Rows of tiles drawn past the window edge. Tall things and high ground
// reach up into the window from below it.
const MARGIN_ROWS_TOP: i32 = 4;
const MARGIN_ROWS_BOTTOM: i32 = 14;
const MARGIN_COLUMNS: i32 = 2;

const LAND_LIGHT_BASE: f32 = 0.84;
const LAND_LIGHT_SIDE: f32 = 0.030;
const LAND_LIGHT_FRONT: f32 = 0.015;
const LAND_LIGHT_MIN: f32 = 0.45;

const FLAT_BLOCK_HEIGHT: f32 = 16.0;
const FLAT_DOOR_HEIGHT: f32 = 5.0;
const FLAT_ITEM_RADIUS: f32 = 4.0;
const FLAT_GRID_EVERY: i32 = 8;
const FLAT_GRID_ALPHA: f32 = 0.05;

const PAWN_RING_RX: f32 = 15.0;
const PAWN_RING_RY: f32 = 7.5;
const PAWN_RING_WIDTH: f32 = 2.0;
const PAWN_RING_FILL_ALPHA: f32 = 0.28;
const PAWN_FOOT_HALF: f32 = 7.0;
const PAWN_SHOULDER_HALF: f32 = 5.0;
const PAWN_FOOT_Y: f32 = -3.0;
const PAWN_SHOULDER_Y: f32 = -36.0;
const PAWN_HEAD_Y: f32 = -44.0;
const PAWN_HEAD_RADIUS: f32 = 7.0;
const PAWN_OUTLINE: f32 = 1.5;
const PAWN_TOP_Y: f32 = -56.0;
const PAWN_HIDDEN_ALPHA: f32 = 0.45;
const PAWN_OUTLINE_COLOR: Color32 = Color32::from_rgba_premultiplied(4, 6, 10, 235);
const FACING_NEAR: f32 = 1.15;
const FACING_FAR: f32 = 1.9;
const FACING_HALF_WIDTH: f32 = 0.32;
/// The second ring of the character, drawn over everything so that a wall or
/// a neighbor never hides where he is.
const FOCUS_RING_GROW: f32 = 1.5;
const GHOST_ALPHA: f32 = 0.35;
const TARGET_RING_GROW: f32 = 1.45;
const ELLIPSE_POINTS: usize = 24;

// The search for the tile under the mouse. A floor above the character is
// drawn higher on the screen, so its tile is some rows further down the map.
const PICK_ROW_STEP: f32 = 0.5;
const PICK_ROWS_UP: i32 = -12;
const PICK_ROWS_DOWN: i32 = 40;

const PLATE_BAR_WIDTH: f32 = 46.0;
const PLATE_GAP: f32 = 3.0;
const PLATE_PAD: f32 = 3.0;
const PERCENT: f32 = 100.0;
const HAIRLINE: f32 = 1.0;

const PATH_DASH: f32 = 9.0;
const PATH_GAP: f32 = 7.0;
const PATH_WIDTH: f32 = 2.0;
const BEACON_HEIGHT: f32 = 46.0;
const BEACON_RING: f32 = 0.5;
const EDGE_INSET: f32 = 28.0;
const EDGE_MARK_RADIUS: f32 = 6.0;

pub const NOTE_NO_UOPATH: &str = "No client files. Give --uopath to see the real map.";

pub struct Scene {
    client: Option<ClientArt>,
    atlas: Option<Atlas>,
    note: String,
    /// Where the character is drawn now. The map is drawn round this point.
    camera: [f32; 3],
    camera_glide: Option<Glide>,
    camera_map: Option<u8>,
    zoom: f32,
    /// How far the camera looks away from the character, in points, while
    /// a look key is held.
    peek: Vec2,
    /// Where each mobile is drawn now, under its serial.
    actors: HashMap<u32, [f32; 3]>,
    glides: HashMap<u32, Glide>,
    /// The things drawn this frame, in paint order. The last one is on top.
    picks: Vec<Pick>,
    /// The time of the frame that is drawn now.
    now: f64,
    /// The footsteps the sound has not taken yet.
    steps: Vec<Step>,
    /// When each walker last made a footstep sound.
    last_step: HashMap<u32, f64>,
    /// The action each mobile shows now, and when it began.
    shows: HashMap<u32, (Action, f64)>,
    /// The last cue that was taken. None until the first picture.
    last_cue: Option<u64>,
    /// The panels of the last frame. The wheel over one does not zoom.
    panels: Vec<Rect>,
    /// How many screen pixels one point of the window has.
    pixels_per_point: f32,
    /// The season of the world. It swaps some of the art.
    season: u8,
    /// The choices of the Options screen, as this frame draws them.
    look: WorldLook,
    /// The light map, and the lights this frame found.
    lights: LightMap,
    light_sources: Vec<LightSource>,
    /// How much of each fading thing shows, and the frame it last showed in.
    fades: HashMap<FadeKey, (f32, u64)>,
    frame_count: u64,
    /// The time from the last frame to this one.
    frame_seconds: f32,
    /// The thing under the mouse in the last frame.
    hovered: Option<u32>,
    ctrl_shift: bool,
    /// Ctrl was held in the last frame.
    ctrl_before: bool,
    /// The heights the world is cut at over the character.
    ceiling: Ceiling,
    circle: Option<Circle>,
    walk: WalkPrediction,
    /// How long the character waits after the slot of a step he was sent
    /// before he takes it. See [`STRIDE_READ_MARGIN_SECONDS`].
    stride_lag: f64,
    /// The slot of the newest step whose move has started.
    stride_taken: Option<std::time::Instant>,
    /// How each storey of the house being designed shows.
    storey_looks: [StoreyLook; STOREYS],
    /// When the shard last showed the death screen.
    died_at: Option<f64>,
    /// The mobiles and corpses in view. None until the first picture.
    in_view: Option<HashSet<u32>>,
    arrivals: Vec<u32>,
    /// A drag the human started on the map, until the gumps take it.
    map_drag: Option<MapDrag>,
}

/// A left drag the human started on the map: where it started, and the
/// mobile it started on. The classic gumps take it to open a health bar
/// or to select health bars by a box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapDrag {
    pub from: Pos2,
    pub mobile: Option<u32>,
}

/// What kind of thing the mouse is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    Mobile,
    Item,
    Corpse,
}

/// One thing on the map that the mouse can point at.
#[derive(Clone, Debug)]
pub struct Pick {
    area: Rect,
    pub serial: u32,
    pub name: String,
    pub kind: PickKind,
}

/// What floats over one thing: its name and its hit points.
struct Plate {
    top: Pos2,
    foot: Pos2,
    name: String,
    /// The color of the Modern style.
    color: Color32,
    /// The notoriety hue of the Classic style.
    hue: u16,
    of: PlateOf,
    hits_percent: Option<u8>,
    target: bool,
    hits: HitsShown,
    /// The name plate shows.
    named: bool,
    poisoned: bool,
    yellow_hits: bool,
}

/// How one mobile is drawn: what he looks like, what he does, his color.
struct Looks<'a> {
    look: &'a WatchLook,
    pose: Pose,
    color: Color32,
    alpha: f32,
    /// One hue over all of him, from the choices of the Options screen.
    hue: Option<u16>,
    /// The hue of the ring of color under his feet, when it shows.
    aura: Option<u16>,
    shadow: bool,
    seat: Option<Seat>,
}

/// Words ready to draw: a picture in a UO font, or the window's own font.
pub enum Words {
    Picture(egui::TextureId, Sprite),
    Font(Arc<Galley>, Color32),
}

impl Words {
    pub fn size(&self) -> Vec2 {
        match self {
            Self::Picture(_, sprite) => Vec2::new(sprite.width, sprite.height),
            Self::Font(galley, _) => galley.size(),
        }
    }

    /// Draws the words with their top left corner at `min`.
    pub fn paint(&self, painter: &Painter, min: Pos2, alpha: f32) {
        match self {
            Self::Picture(texture, sprite) => {
                painter.image(
                    *texture,
                    Rect::from_min_size(min, self.size()),
                    sprite.uv,
                    Color32::WHITE.gamma_multiply(alpha),
                );
            }
            Self::Font(galley, color) => {
                painter.galley_with_override_text_color(
                    min,
                    galley.clone(),
                    color.gamma_multiply(alpha),
                );
            }
        }
    }
}

/// A thing whose alpha fades: a tile of the map, a piece of land, or an
/// object of the shard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FadeKey {
    Land { x: u16, y: u16 },
    Tile { x: u16, y: u16, z: i8, graphic: u16 },
    Serial(u32),
}

/// The heights the world is cut at over the character, as the reference
/// client works them out: things from `max_z` up fade away, and so does land over
/// `max_ground_z`. Under a roof, the roofs fade too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ceiling {
    max_z: i16,
    max_ground_z: i16,
    no_draw_roofs: bool,
}

impl Ceiling {
    fn open(draw_roofs: bool) -> Self {
        Self {
            max_z: NO_CEILING,
            max_ground_z: NO_CEILING,
            no_draw_roofs: !draw_roofs,
        }
    }
}

/// One thing on a tile, for the ceiling.
#[derive(Clone, Copy, Debug)]
struct Over {
    z: i16,
    flags: TileFlagSet,
}

/// The ceiling over a character at `own_z`, from the land and the things
/// on his tile and on the tile in front of him.
fn ceiling_of(
    own_z: i16,
    land: Option<i16>,
    here: &[Over],
    ahead: &[Over],
    draw_roofs: bool,
) -> Ceiling {
    let head = own_z + HEAD_ROOM;
    let top = own_z + BODY_HEIGHT;
    let mut ceiling = Ceiling::open(draw_roofs);
    if land.is_some_and(|land| top <= land) {
        // He is under the ground, in a cave or a dungeon.
        ceiling.max_ground_z = top;
        ceiling.max_z = top;
    } else {
        let solid = TileFlagSet::FOLIAGE | TileFlagSet::TRANSPARENT;
        for over in here {
            let blocks = over.flags.0 & solid.0 == 0
                && (!over.flags.contains(TileFlagSet::ROOF)
                    || over.flags.contains(TileFlagSet::SURFACE));
            if over.z > head && ceiling.max_z > over.z && blocks {
                ceiling.max_z = over.z;
                ceiling.no_draw_roofs = true;
            }
        }
    }
    let open_roof = TileFlagSet::TRANSPARENT | TileFlagSet::SURFACE;
    let mut near = ceiling.max_z;
    for over in ahead {
        let roof = over.flags.0 & open_roof.0 == 0 && over.flags.contains(TileFlagSet::ROOF);
        if over.z > head && ceiling.max_z > over.z && roof {
            ceiling.max_z = over.z;
            near = over.z;
            ceiling.no_draw_roofs = true;
        }
    }
    ceiling.max_z = near.max(top);
    ceiling
}

/// The circle of transparency round the character.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Circle {
    center: Pos2,
    radius: f32,
    style: CircleStyle,
}

/// How one picture of the world is laid.
#[derive(Clone, Copy, Debug)]
struct Lay {
    alpha: f32,
    /// The circle of transparency may show through it.
    see_through: bool,
    shadow: bool,
    /// It is water, which moves.
    water: bool,
}

/// One static of the map, or one piece of a house or a boat.
#[derive(Clone, Copy, Debug)]
struct StaticArt {
    tile: (u16, u16),
    z: f32,
    graphic: u16,
    hue: u16,
    flags: TileFlagSet,
    height: u8,
    /// A piece of a house or a boat.
    piece: bool,
    /// A piece of the house being designed on a see-through storey.
    faded: bool,
}

/// How far a thing at `x`, `y` is from the character, in tiles.
fn tiles_from(frame: &WatchFrame, x: u16, y: u16) -> u16 {
    x.abs_diff(frame.x).max(y.abs_diff(frame.y))
}

/// The four corners of a picture, clockwise from the top left.
fn corners(rect: Rect) -> [Pos2; 4] {
    [
        rect.left_top(),
        rect.right_top(),
        rect.right_bottom(),
        rect.left_bottom(),
    ]
}

/// The light of a corner of a stretched land tile with this normal, as the
/// shader of the classic client works it out. `bright` is the terrain
/// shadows level: it makes slopes darker and brighter.
fn land_light(normal: [f32; 3], bright: f32) -> f32 {
    const LIGHT_DIRECTION: [f32; 3] = [
        0.0,
        std::f32::consts::FRAC_1_SQRT_2,
        std::f32::consts::FRAC_1_SQRT_2,
    ];
    // Flat land is lit at 45 degrees: half of cos 45, plus a half.
    const FLAT_LIGHT: f32 = 0.853_553_4;
    let dot: f32 = normal.iter().zip(LIGHT_DIRECTION).map(|(n, l)| n * l).sum();
    let base = dot.max(0.0) / 2.0 + 0.5;
    (base + bright * (base - FLAT_LIGHT) - (base - FLAT_LIGHT)).clamp(0.0, 1.0)
}

/// Where a mobile holds a light, from the top of his picture, for the way
/// he faces.
fn held_light_offset(direction: u8) -> Vec2 {
    const SIDE: f32 = 22.0;
    const HAND: f32 = 33.0;
    const LOW_HAND: f32 = 55.0;
    match direction & DIRECTION_MASK {
        1 => Vec2::new(SIDE, HAND),
        2 => Vec2::new(SIDE, LOW_HAND),
        3 => Vec2::new(0.0, LOW_HAND),
        4 => Vec2::new(-SIDE, LOW_HAND),
        5 => Vec2::new(-SIDE, HAND),
        _ => Vec2::ZERO,
    }
}

/// The flicker seed of a light on a tile: each tile has its own.
fn tile_seed((x, y): (u16, u16)) -> u32 {
    const Y_BITS: u32 = u16::BITS;
    (u32::from(x) << Y_BITS) | u32::from(y)
}

/// How much of a building that waits for its place shows.
const PLACING_ALPHA: f32 = 0.55;
const MS_PER_SECOND: f64 = 1000.0;
/// A fire or a fountain shows its next picture this often, so the window
/// draws again at least this often.
const ART_CYCLE_SECONDS: f64 = 0.1;

/// How long a shown action plays, and how long each of its pictures stays.
const SHOW_SECONDS: f64 = 0.9;
const SHOW_FRAME_SECONDS: f64 = 0.1;

/// The pose of a mobile the window has not seen move.
const STANDING: Pose = Pose {
    action: Action::Stand,
    tick: 0,
};

/// A thing that stands on one tile and is drawn in height order.
enum Standing<'a> {
    Art(&'a WatchItem),
    /// One piece of a house or a boat, see-through while its storey of the
    /// house being designed shows so.
    Piece {
        graphic: u16,
        z: f32,
        faded: bool,
    },
    Corpse(&'a WatchItem),
    Mobile {
        mobile: &'a WatchMobile,
        at: [f32; 3],
    },
    Character {
        at: [f32; 3],
    },
}

impl Standing<'_> {
    fn z(&self) -> f32 {
        match self {
            Self::Art(item) | Self::Corpse(item) => f32::from(item.z),
            Self::Piece { z, .. } => *z,
            Self::Mobile { at, .. } | Self::Character { at } => at[2],
        }
    }
}

struct Canvas {
    mesh: Mesh,
    white: Pos2,
}

impl Canvas {
    fn vertex(&mut self, pos: Pos2, uv: Pos2, color: Color32) -> u32 {
        self.mesh.vertices.push(Vertex { pos, uv, color });
        self.mesh.vertices.len() as u32 - 1
    }

    fn quad(&mut self, points: [Pos2; 4], uvs: [Pos2; 4], color: Color32) {
        self.quad_colors(points, uvs, [color; 4]);
    }

    /// A picture laid on four points, each with its own color.
    fn quad_colors(&mut self, points: [Pos2; 4], uvs: [Pos2; 4], colors: [Color32; 4]) {
        let first = self.vertex(points[0], uvs[0], colors[0]);
        for i in 1..points.len() {
            self.vertex(points[i], uvs[i], colors[i]);
        }
        self.mesh
            .indices
            .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }

    /// A picture cut in a grid of cells. Each corner of a cell takes the
    /// share of `color` that `shown` gives for its place.
    fn sprite_grid(
        &mut self,
        sprite: Sprite,
        rect: Rect,
        color: Color32,
        cell: f32,
        shown: impl Fn(Pos2) -> f32,
    ) {
        let columns = (rect.width() / cell).ceil().max(1.0) as u32;
        let rows = (rect.height() / cell).ceil().max(1.0) as u32;
        let first = self.mesh.vertices.len() as u32;
        for row in 0..=rows {
            for column in 0..=columns {
                let share = Vec2::new(column as f32 / columns as f32, row as f32 / rows as f32);
                let pos = rect.min + rect.size() * share;
                let uv = sprite.uv.min + sprite.uv.size() * share;
                self.vertex(pos, uv, color.gamma_multiply(shown(pos)));
            }
        }
        let stride = columns + 1;
        for row in 0..rows {
            for column in 0..columns {
                let top_left = first + row * stride + column;
                let bottom_left = top_left + stride;
                self.mesh.indices.extend([
                    top_left,
                    top_left + 1,
                    bottom_left + 1,
                    top_left,
                    bottom_left + 1,
                    bottom_left,
                ]);
            }
        }
    }

    /// The shadow of a picture, laid flat and slanted on the ground, as the
    /// classic client draws it.
    fn shadow(&mut self, sprite: Sprite, area: Rect, zoom: f32, alpha: f32) {
        let width = area.width();
        let height = area.height() * SHADOW_SQUASH;
        let top = area.top() + height - SHADOW_RISE * zoom;
        // The classic client slants it by its own height.
        let slant = height;
        let points = [
            Pos2::new(area.left() + slant, top),
            Pos2::new(area.left() + slant + width, top),
            Pos2::new(area.left() + width, top + height),
            Pos2::new(area.left(), top + height),
        ];
        let color = Color32::from_black_alpha((SHADOW_ALPHA * alpha * f32::from(u8::MAX)) as u8);
        self.quad(points, corners(sprite.uv), color);
    }

    /// Water that moves: the picture again over itself, grown and shrunk
    /// with the time.
    fn water(&mut self, sprite: Sprite, area: Rect, color: Color32, time: f64) {
        let time = time as f32;
        let grow = Vec2::new(
            WATER_GROW + time.sin() * WATER_SWAY,
            WATER_GROW + time.cos() * WATER_SWAY_DOWN,
        );
        let grown = Rect::from_min_size(area.min, area.size() * grow);
        self.sprite(sprite, grown, color);
    }

    /// A round glow of `color` that fades to nothing at its edge.
    fn glow(&mut self, center: Pos2, radius: f32, color: Color32) {
        let white = self.white;
        let middle = self.vertex(center, white, color);
        let edge: Vec<u32> = ellipse(center, Vec2::splat(radius))
            .into_iter()
            .map(|point| self.vertex(point, white, Color32::TRANSPARENT))
            .collect();
        for i in 0..edge.len() {
            let next = (i + 1) % edge.len();
            self.mesh.indices.extend([middle, edge[i], edge[next]]);
        }
    }

    /// A person on a chair, as the classic client draws him: his standing
    /// picture folded at the waist and the knees, the upper body leaning
    /// forward, and the feet left out.
    fn sitting(&mut self, sprite: Sprite, area: Rect, color: Color32, mirrored: bool, zoom: f32) {
        let lean = if mirrored { -SIT_LEAN } else { SIT_LEAN } * zoom;
        let leans = [lean, lean, 0.0, 0.0];
        let at = |fold: usize| {
            let share = SIT_FOLDS[fold];
            let y = area.top() + area.height() * share;
            let v = sprite.uv.top() + sprite.uv.height() * share;
            (y, v, leans[fold])
        };
        for fold in 0..SIT_FOLDS.len() - 1 {
            let (top, top_v, top_lean) = at(fold);
            let (bottom, bottom_v, bottom_lean) = at(fold + 1);
            self.quad(
                [
                    Pos2::new(area.left() + top_lean, top),
                    Pos2::new(area.right() + top_lean, top),
                    Pos2::new(area.right() + bottom_lean, bottom),
                    Pos2::new(area.left() + bottom_lean, bottom),
                ],
                [
                    Pos2::new(sprite.uv.left(), top_v),
                    Pos2::new(sprite.uv.right(), top_v),
                    Pos2::new(sprite.uv.right(), bottom_v),
                    Pos2::new(sprite.uv.left(), bottom_v),
                ],
                color,
            );
        }
    }

    /// A filled shape with no picture. The points must go round a convex
    /// shape.
    fn fill(&mut self, points: &[Pos2], color: Color32) {
        let white = self.white;
        let first = self.vertex(points[0], white, color);
        for point in &points[1..] {
            self.vertex(*point, white, color);
        }
        for i in 1..points.len() as u32 - 1 {
            self.mesh.indices.extend([first, first + i, first + i + 1]);
        }
    }

    fn ring(&mut self, center: Pos2, radius: Vec2, width: f32, color: Color32) {
        let outer = ellipse(center, radius);
        let inner = ellipse(center, radius - Vec2::splat(width));
        for i in 0..ELLIPSE_POINTS {
            let next = (i + 1) % ELLIPSE_POINTS;
            self.fill(&[outer[i], outer[next], inner[next], inner[i]], color);
        }
    }

    fn sprite(&mut self, sprite: Sprite, rect: Rect, color: Color32) {
        let uv = sprite.uv;
        self.quad(
            [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ],
            [
                uv.left_top(),
                uv.right_top(),
                uv.right_bottom(),
                uv.left_bottom(),
            ],
            color,
        );
    }
}

fn ellipse(center: Pos2, radius: Vec2) -> Vec<Pos2> {
    (0..ELLIPSE_POINTS)
        .map(|i| {
            let angle = i as f32 / ELLIPSE_POINTS as f32 * std::f32::consts::TAU;
            center + Vec2::new(angle.cos() * radius.x, angle.sin() * radius.y)
        })
        .collect()
}

/// The way the character faces, as one step on the tile grid.
fn facing_step(facing: &str) -> Option<(f32, f32)> {
    let (dx, dy) = uoterm_protocol::types::Direction::from_name(facing)?.delta();
    Some((dx as f32, dy as f32))
}

/// A steady move from one place to the next. The session tells where a
/// thing is a few times each second. A thing that walks changes its tile at
/// an even pace, so the move to the new tile takes as long as the last tile
/// took. The thing then moves without a stop between two tiles.
#[derive(Clone, Copy, Debug)]
struct Glide {
    from: [f32; 3],
    to: [f32; 3],
    started: f64,
    seconds: f64,
    /// The tiles that came while the move to `to` was under way.
    queue: [Waiting; GLIDE_QUEUE_CAP],
    queued: usize,
    /// How long one tile takes at the pace of the mobile.
    tile_seconds: f64,
    /// How long one tile took of late, by when the news of each tile came.
    /// The news of a live shard comes at uneven times. A move that is a
    /// little faster than the news stops after each tile.
    news_seconds: f64,
    last_news: f64,
    /// The tiles of the moves that ended, for the pictures of the legs.
    tiles_done: f32,
    running: bool,
}

/// A tile that waits for the move under way to end.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Waiting {
    at: [f32; 3],
    stride: Option<Stride>,
}

/// The timing of a move the session sent as a step, on the clock of the
/// window: when the move may start, and how long each tile of it lasts. The
/// steps of the character take exactly as long as the session gave them,
/// so they meet end to end at the pace they were sent, as in the classic
/// client. The news of a mobile has no such timing and is only heard.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Stride {
    starts: f64,
    tile_seconds: f64,
}

impl Stride {
    /// When the move starts after a move that ends at `ended`: right then
    /// when the step was sent on the cadence of the one before, and at its
    /// own time after a pause.
    fn start_after(self, ended: f64) -> f64 {
        if self.starts <= ended + STRIDE_JOIN_SECONDS {
            ended
        } else {
            self.starts
        }
    }

    /// The timing of a move of `tiles` tiles whose last tile is this step:
    /// its first tile started that many steps before.
    fn over(self, tiles: f32) -> Self {
        Self {
            starts: self.starts - self.tile_seconds * f64::from((tiles - 1.0).max(0.0)),
            ..self
        }
    }
}

/// How long one tile takes. These are the paces of the game itself, so the
/// move on the screen does not depend on when the news of it came.
fn tile_seconds(mounted: bool, running: bool) -> f64 {
    match (mounted, running) {
        (false, false) => STEP_SECONDS_FOOT_WALK,
        (false, true) => STEP_SECONDS_FOOT_RUN,
        (true, false) => STEP_SECONDS_MOUNT_WALK,
        (true, true) => STEP_SECONDS_MOUNT_RUN,
    }
}

fn tiles_between(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
}

impl Glide {
    fn resting(at: [f32; 3], time: f64) -> Self {
        Self {
            from: at,
            to: at,
            started: time,
            seconds: 0.0,
            queue: [Waiting { at, stride: None }; GLIDE_QUEUE_CAP],
            queued: 0,
            tile_seconds: STEP_SECONDS_FOOT_WALK,
            news_seconds: STEP_SECONDS_FOOT_WALK,
            last_news: time,
            tiles_done: 0.0,
            running: false,
        }
    }

    fn at(&self, time: f64) -> [f32; 3] {
        if self.seconds <= 0.0 {
            return self.to;
        }
        let share = ((time - self.started) / self.seconds).clamp(0.0, 1.0) as f32;
        [0, 1, 2].map(|i| self.from[i] + (self.to[i] - self.from[i]) * share)
    }

    fn moving(&self, time: f64) -> bool {
        self.queued > 0 || (self.from != self.to && time - self.started < self.seconds)
    }

    /// The way the mobile moves now, 0 for north and then clockwise to 7.
    /// None when he rests. The shard turns him when it takes his next step,
    /// but the window still draws the step before it. With the way of the
    /// shard he would slide sideways, as on ice.
    fn heading(&self, time: f64) -> Option<u8> {
        /// The ways, by the step south (north, none, south) and then by the
        /// step east (west, none, east). The middle is no move.
        const WAYS: [[u8; 3]; 3] = [[7, 0, 1], [6, 0, 2], [5, 4, 3]];
        let place = |step: f32| match step {
            step if step < 0.0 => 0,
            step if step > 0.0 => 2,
            _ => 1,
        };
        let east = place(self.to[0] - self.from[0]);
        let south = place(self.to[1] - self.from[1]);
        let on_the_move = time - self.started < self.seconds + STEP_LINGER_SECONDS;
        (on_the_move && (east, south) != (1, 1)).then(|| WAYS[south][east])
    }

    /// How many tiles the mobile covered since he last rested.
    fn tiles_covered(&self, time: f64) -> f32 {
        if self.seconds <= 0.0 {
            return self.tiles_done;
        }
        let share = ((time - self.started) / self.seconds).clamp(0.0, 1.0) as f32;
        self.tiles_done + tiles_between(self.from, self.to) * share
    }

    /// What the mobile does now, and which picture of it shows. The legs of
    /// a walk go by the ground he covered. For a short time after a move he
    /// keeps his last picture, so he does not stand up between two tiles.
    fn pose(&self, time: f64) -> Pose {
        let stepping = self.queued > 0
            || (self.from != self.to && time - self.started < self.seconds + STEP_LINGER_SECONDS);
        if !stepping {
            return Pose {
                action: Action::Stand,
                tick: (time / STAND_FRAME_SECONDS) as usize,
            };
        }
        let pictures_per_tile = self.tile_seconds / STEP_FRAME_SECONDS;
        Pose {
            action: if self.running {
                Action::Run
            } else {
                Action::Walk
            },
            tick: (f64::from(self.tiles_covered(time)) * pictures_per_tile) as usize,
        }
    }

    /// The last tile the mobile is on his way to.
    fn goal(&self) -> [f32; 3] {
        match self.queued {
            0 => self.to,
            queued => self.queue[queued - 1].at,
        }
    }

    /// How long the move to the next tile takes, once it has started.
    ///
    /// A step the session sent takes exactly the time the session gave it,
    /// and a little less while it is late for its slot, so the character
    /// catches up without a jump.
    ///
    /// Heard news has no such time. With nothing queued the move is a little
    /// slow, so the next tile comes before this one ends and the mobile does
    /// not stop between two tiles. With tiles queued it is fast, so the
    /// mobile catches up.
    fn leg_seconds(&self, from: [f32; 3], to: [f32; 3], stride: Option<Stride>) -> f64 {
        let tiles = f64::from(tiles_between(from, to).max(1.0));
        if let Some(stride) = stride {
            let late = ((self.started - stride.starts) / stride.tile_seconds).max(0.0);
            return stride.tile_seconds * tiles / (1.0 + GLIDE_CATCH_UP_PER_TILE * late);
        }
        let pace = match self.queued {
            0 => GLIDE_STRETCH_ALONE,
            queued => 1.0 / (1.0 + GLIDE_CATCH_UP_PER_TILE * queued as f64),
        };
        self.tile_seconds.max(self.news_seconds) * tiles * pace
    }

    /// Takes the tile the mobile is on now, from the news of the shard. A
    /// new tile starts a move, or waits in the queue for the move under way
    /// to end. A jump is not a move.
    fn aim(&mut self, goal: [f32; 3], time: f64, mounted: bool, running: bool) {
        self.take(goal, time, mounted, running, None);
    }

    /// Takes the tile of a step the session sent, with the timing it gave
    /// the step.
    fn aim_stride(
        &mut self,
        goal: [f32; 3],
        time: f64,
        mounted: bool,
        running: bool,
        stride: Stride,
    ) {
        self.take(goal, time, mounted, running, Some(stride));
    }

    fn take(
        &mut self,
        goal: [f32; 3],
        time: f64,
        mounted: bool,
        running: bool,
        stride: Option<Stride>,
    ) {
        self.tile_seconds = tile_seconds(mounted, running);
        self.running = running;
        if goal != self.goal() {
            if far_apart(self.goal(), goal) {
                *self = Self::resting(goal, time);
                return;
            }
            let stride = stride.map(|stride| stride.over(tiles_between(self.goal(), goal)));
            if stride.is_none() {
                self.note_news(goal, time);
            }
            let at_rest = self.queued == 0 && time - self.started >= self.seconds;
            if at_rest {
                // A walk after a rest starts its legs from the first picture.
                let rested = time - self.started >= self.seconds + STEP_LINGER_SECONDS;
                self.tiles_done = if rested {
                    0.0
                } else {
                    self.tiles_done + tiles_between(self.from, self.to)
                };
                let ended = self.started + self.seconds;
                self.from = self.to;
                self.to = goal;
                // Late news starts now: a move never jumps ahead.
                self.started = stride.map_or(time, |stride| stride.start_after(ended).max(time));
                self.seconds = self.leg_seconds(self.from, goal, stride);
            } else {
                // A full queue loses its last tile. The move then cuts a corner.
                let place = self.queued.min(GLIDE_QUEUE_CAP - 1);
                self.queue[place] = Waiting { at: goal, stride };
                self.queued = place + 1;
            }
        }
        self.settle(time);
    }

    /// Learns the rhythm of the news. A gap that is much longer than the
    /// pace is a rest, not a rhythm.
    fn note_news(&mut self, goal: [f32; 3], time: f64) {
        let tiles = f64::from(tiles_between(self.goal(), goal).max(1.0));
        let took = (time - self.last_news) / tiles;
        self.last_news = time;
        let in_rhythm = took < self.tile_seconds * NEWS_REST_FACTOR;
        self.news_seconds = if in_rhythm {
            self.news_seconds + (took - self.news_seconds) * NEWS_LEARN_SHARE
        } else {
            self.tile_seconds
        };
    }

    /// Starts the move to the next queued tile at the moment the last move
    /// ended, so no time is lost between two tiles. A step the session sent
    /// after a pause, such as a turn, waits for its own time.
    fn settle(&mut self, time: f64) {
        while self.queued > 0 && time - self.started >= self.seconds {
            let next = self.queue[0];
            self.queue.copy_within(1.., 0);
            self.queued -= 1;
            let ended = self.started + self.seconds;
            self.started = next
                .stride
                .map_or(ended, |stride| stride.start_after(ended));
            self.tiles_done += tiles_between(self.from, self.to);
            self.from = self.to;
            self.to = next.at;
            self.seconds = self.leg_seconds(self.from, next.at, next.stride);
        }
    }
}

/// The look of a mobile turned the way he moves. None when he faces that
/// way already, or when he rests.
fn turned_to(look: &WatchLook, heading: Option<u8>) -> Option<WatchLook> {
    let heading = heading.filter(|way| *way != look.direction)?;
    Some(WatchLook {
        direction: heading,
        ..look.clone()
    })
}

fn stride_lag(poll_every: Duration) -> f64 {
    poll_every.as_secs_f64() + STRIDE_READ_MARGIN_SECONDS
}

fn far_apart(a: [f32; 3], b: [f32; 3]) -> bool {
    tiles_between(a, b) > TELEPORT_TILES
}

impl Scene {
    pub fn new(uopath: Option<&Path>) -> Self {
        let (client, note) = match uopath.map(ClientArt::open) {
            Some(Ok(client)) => (Some(client), String::new()),
            Some(Err(e)) => (None, format!("No real map: {e}.")),
            None => (None, NOTE_NO_UOPATH.to_string()),
        };
        Self {
            client,
            atlas: None,
            note,
            camera: [0.0; 3],
            camera_glide: None,
            camera_map: None,
            zoom: ZOOM_START,
            peek: Vec2::ZERO,
            actors: HashMap::new(),
            glides: HashMap::new(),
            picks: Vec::new(),
            now: 0.0,
            steps: Vec::new(),
            last_step: HashMap::new(),
            shows: HashMap::new(),
            last_cue: None,
            panels: Vec::new(),
            pixels_per_point: 1.0,
            season: 0,
            look: WorldLook::of(&Profile::default()),
            lights: LightMap::default(),
            light_sources: Vec::new(),
            fades: HashMap::new(),
            frame_count: 0,
            frame_seconds: 0.0,
            hovered: None,
            ctrl_shift: false,
            ctrl_before: false,
            ceiling: Ceiling::open(true),
            circle: None,
            walk: WalkPrediction::default(),
            stride_lag: stride_lag(Duration::from_millis(POLL_MS_SAME_PROGRAM)),
            stride_taken: None,
            storey_looks: [StoreyLook::Normal; STOREYS],
            died_at: None,
            in_view: None,
            arrivals: Vec::new(),
            map_drag: None,
        }
    }

    /// Why the map shows flat colors. Empty when it shows the real map.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Draws the map into `rect`, as the profile says. True while something
    /// still moves, so the window must draw the next frame at once.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        time: f64,
        profile: &Profile,
    ) -> bool {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::VOID);
        self.look = WorldLook::of(profile);
        self.read_input(ui, rect);
        self.pixels_per_point = ui.ctx().pixels_per_point();
        self.season = frame.season;
        self.frame_seconds = (time - self.now).clamp(0.0, MAX_FRAME_SECONDS) as f32;
        self.now = time;
        self.frame_count += 1;
        self.take_cues(frame, time);
        if let Some(client) = self.client.as_mut() {
            client.take_live_map(&frame.live_map);
        }
        self.note_arrivals(frame);
        let moving = self.follow(frame, time) || !self.shows.is_empty();
        // A mobile that stands still moves a little, and a fire burns. Wake
        // for the next picture.
        if self.client.is_some() {
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(ART_CYCLE_SECONDS));
        }
        if self.death_screen(&painter, rect, frame, time) {
            return true;
        }
        let atlas = self.atlas.get_or_insert_with(|| Atlas::new(ui.ctx()));
        let mut canvas = Canvas {
            mesh: Mesh::with_texture(atlas.texture_id()),
            white: atlas.white_uv(),
        };
        let mut plates = Vec::new();
        self.build(rect, frame, &mut canvas, &mut plates);
        painter.add(Shape::mesh(canvas.mesh));
        self.draw_walk_goal(&painter, rect, frame);
        let character = self.project(rect, self.camera);
        if self.look.classic() {
            self.draw_overheads(&painter, plates);
        } else {
            self.focus_ring(&painter, character);
            draw_plates(&painter, plates, self.figure_rect(character), self.zoom);
        }
        moving
    }

    /// The death screen: for a moment after the shard says the character
    /// died, the world is black and says so. True while it shows.
    fn death_screen(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        time: f64,
    ) -> bool {
        let showing = frame.dead
            && self.look.video.death_screen
            && self
                .died_at
                .is_some_and(|at| time - at < DEATH_SCREEN_SECONDS);
        if !showing {
            return false;
        }
        painter.rect_filled(rect, 0.0, Color32::BLACK);
        let words = self.words(painter, DEATH_WORDS, TextLook::ascii(DEATH_FONT, DEATH_HUE));
        words.paint(painter, rect.center() - words.size() / 2.0, 1.0);
        painter
            .ctx()
            .request_repaint_after(Duration::from_secs_f64(ART_CYCLE_SECONDS));
        true
    }

    /// Notes the mobiles and corpses that came into view since the last
    /// frame. The classic client asks for the name of each as it comes.
    fn note_arrivals(&mut self, frame: &WatchFrame) {
        let general = &self.look.general;
        let mobiles = frame.mobiles.iter().map(|mobile| (mobile.serial, true));
        let corpses = frame
            .items
            .iter()
            .filter(|item| item.graphic == CORPSE_GRAPHIC)
            .map(|item| (item.serial, false));
        let now: HashMap<u32, bool> = mobiles.chain(corpses).collect();
        if let Some(before) = &self.in_view {
            let wanted = |mobile: bool| {
                if mobile {
                    general.show_incoming_mobiles
                } else {
                    general.show_incoming_corpses
                }
            };
            self.arrivals.extend(
                now.iter()
                    .filter(|(serial, mobile)| !before.contains(serial) && wanted(**mobile))
                    .map(|(serial, _)| *serial),
            );
        }
        self.in_view = Some(now.into_keys().collect());
    }

    /// The mobiles and corpses that came into view, whose names the window
    /// asks for.
    pub fn take_arrivals(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.arrivals)
    }

    /// The map tells of a left drag the human started on it.
    pub fn start_map_drag(&mut self, drag: MapDrag) {
        self.map_drag = Some(drag);
    }

    /// The drag the human started on the map, once.
    pub fn take_map_drag(&mut self) -> Option<MapDrag> {
        self.map_drag.take()
    }

    /// The mobiles drawn in a box of the screen, first drawn first.
    pub fn mobiles_in(&self, area: Rect) -> Vec<u32> {
        self.picks
            .iter()
            .filter(|pick| pick.kind == PickKind::Mobile && pick.area.intersects(area))
            .map(|pick| pick.serial)
            .collect()
    }

    /// How each storey of the house being designed shows, as the designer
    /// sets it.
    pub fn set_storey_looks(&mut self, looks: [StoreyLook; STOREYS]) {
        self.storey_looks = looks;
    }

    /// Reads the mouse and the keys for this frame: what the mouse is on,
    /// Ctrl and Shift, and the wheel. In the Classic style the wheel zooms
    /// while Ctrl is held when the Video page allows it, and the zoom may go
    /// back to its default when Ctrl is let go. The Modern style zooms with
    /// the wheel alone.
    fn read_input(&mut self, ui: &egui::Ui, rect: Rect) {
        let (pointer, scroll, zoom_delta, ctrl, shift) = ui.input(|i| {
            (
                i.pointer.hover_pos(),
                i.smooth_scroll_delta.y,
                i.zoom_delta(),
                i.modifiers.ctrl,
                i.modifiers.shift,
            )
        });
        let on_world = pointer.filter(|p| self.is_on_world(rect, *p));
        self.hovered = on_world
            .and_then(|p| self.thing_at(p))
            .map(|pick| pick.serial);
        self.ctrl_shift = ctrl && shift;
        let video = &self.look.video;
        let zoom = if !self.look.classic() {
            let scroll = if on_world.is_some() { scroll } else { 0.0 };
            self.zoom * (1.0 + scroll * ZOOM_PER_SCROLL_POINT)
        } else if video.wheel_zoom && ctrl && on_world.is_some() {
            self.zoom * zoom_delta
        } else if video.wheel_zoom && video.ctrl_release_restores_zoom && self.ctrl_before && !ctrl
        {
            video.default_zoom
        } else {
            self.zoom
        };
        self.ctrl_before = ctrl;
        self.zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
    }

    /// True when a point of the window shows the world and no panel.
    pub fn is_on_world(&self, rect: Rect, point: Pos2) -> bool {
        rect.contains(point) && !self.panels.iter().any(|panel| panel.contains(point))
    }

    /// Moves the camera and every mobile toward where the frame says it is.
    /// While a human steers, the character goes on the steps the session
    /// has sent, ahead of the shard, each over the time the session gave it.
    fn follow(&mut self, frame: &WatchFrame, time: f64) -> bool {
        let same_map = self.camera_map == Some(frame.map);
        self.camera_map = Some(frame.map);
        let steered = same_map && frame.human_control && !frame.paralyzed;
        let sent = frame.stepping_to.filter(|_| steered);
        let shown = self.walk.show((frame.x, frame.y, frame.z), sent);
        let goal = [
            f32::from(shown.spot.0),
            f32::from(shown.spot.1),
            f32::from(shown.spot.2),
        ];
        let (mounted, running) = (is_mounted(&frame.look), frame.look.running);
        let glide = match self
            .camera_glide
            .filter(|_| same_map && !shown.snapped_back)
        {
            Some(mut glide) => {
                let stride = frame
                    .stride
                    .filter(|stride| steered && goal != glide.goal() && self.is_fresh(stride));
                match stride {
                    Some(stride) => {
                        self.stride_taken = Some(stride.slot);
                        let stride = self.on_window_clock(stride, time);
                        glide.aim_stride(goal, time, mounted, running, stride);
                    }
                    None => glide.aim(goal, time, mounted, running),
                }
                glide
            }
            None => Glide::resting(goal, time),
        };
        if self
            .camera_glide
            .is_some_and(|before| before.goal() != glide.goal())
            && !frame.dead
            && !frame.hidden
        {
            self.step(SELF_STEP_KEY, &frame.look, &glide, 0.0, time);
        }
        self.camera_glide = Some(glide);
        self.camera = glide.at(time);
        let mut moving = glide.moving(time);
        let mut glides = HashMap::with_capacity(frame.mobiles.len());
        self.actors.clear();
        for mobile in &frame.mobiles {
            let goal = [
                f32::from(mobile.x),
                f32::from(mobile.y),
                f32::from(mobile.z),
            ];
            let glide = match self.glides.get(&mobile.serial).filter(|_| same_map) {
                Some(known) => {
                    let mut glide = *known;
                    glide.aim(goal, time, is_mounted(&mobile.look), mobile.look.running);
                    if glide.goal() != known.goal() {
                        self.step(
                            mobile.serial,
                            &mobile.look,
                            &glide,
                            f32::from(mobile.dist),
                            time,
                        );
                    }
                    glide
                }
                None => Glide::resting(goal, time),
            };
            moving |= glide.moving(time);
            self.actors.insert(mobile.serial, glide.at(time));
            glides.insert(mobile.serial, glide);
        }
        self.glides = glides;
        self.last_step
            .retain(|_, at| time - *at < STEP_MEMORY_SECONDS);
        moving
    }

    /// True for the newest step when it is the news of this move: a step
    /// whose move has not started, sent no longer ago than it lasts and the
    /// news of it takes. An older one is not what moved the character.
    fn is_fresh(&self, stride: &WatchStride) -> bool {
        let fresh_for = stride.lasts.as_secs_f64() + self.stride_lag;
        self.stride_taken != Some(stride.slot) && stride.slot.elapsed().as_secs_f64() <= fresh_for
    }

    /// The timing of a sent step on the clock of the window, which `time`
    /// reads now.
    fn on_window_clock(&self, stride: WatchStride, time: f64) -> Stride {
        Stride {
            starts: time - stride.slot.elapsed().as_secs_f64() + self.stride_lag,
            tile_seconds: stride.lasts.as_secs_f64(),
        }
    }

    /// Takes the new animation cues. A cue from before the window opened
    /// does not play.
    fn take_cues(&mut self, frame: &WatchFrame, time: f64) {
        let newest = frame.cues.iter().map(|cue| cue.seq).max();
        if let Some(seen) = self.last_cue {
            for cue in frame.cues.iter().filter(|cue| cue.seq > seen) {
                if cue.kind == WatchCueKind::DeathScreen {
                    self.died_at = Some(time);
                }
                let action = match cue.kind {
                    WatchCueKind::Animation(group) => u8::try_from(group).ok().map(Action::Shown),
                    WatchCueKind::Deed(kind, action) => {
                        self.deed_of(frame, cue.serial, kind, action)
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    self.shows.insert(cue.serial, (action, time));
                }
            }
        }
        self.last_cue = newest.or(self.last_cue).or(Some(0));
        self.shows
            .retain(|_, (_, began)| time - *began < SHOW_SECONDS);
    }

    /// The pictures for a deed of the newer animation packet. They depend
    /// on the body of the mobile.
    fn deed_of(&self, frame: &WatchFrame, serial: u32, kind: u16, action: u16) -> Option<Action> {
        let look = if serial == frame.serial {
            &frame.look
        } else {
            &frame.mobiles.iter().find(|m| m.serial == serial)?.look
        };
        self.client
            .as_ref()?
            .deed_action(look, Deed::from_packet(kind, action)?)
    }

    /// The pose of a mobile that shows an action now.
    fn shown_pose(&self, serial: u32) -> Option<Pose> {
        let (action, began) = self.shows.get(&serial)?;
        Some(Pose {
            action: *action,
            tick: ((self.now - began) / SHOW_FRAME_SECONDS) as usize,
        })
    }

    /// Draws the outline of a building where the mouse points, while the
    /// shard waits for its place. Its pieces show as a pale shape, so the
    /// human sees what the building covers before he puts it there.
    pub fn draw_placing(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        mouse: Pos2,
    ) -> bool {
        let Some(placing) = frame.placing else {
            return false;
        };
        let (x, y, z) = self.tile_at(rect, frame, mouse);
        let pieces: Vec<(i16, i16, i16, u16)> = self
            .client
            .as_ref()
            .map(|client| {
                client
                    .multi_pieces(placing.multi_id)
                    .iter()
                    .map(|piece| (piece.dx, piece.dy, piece.dz, piece.graphic))
                    .collect()
            })
            .unwrap_or_default();
        let mut mesh = Mesh::with_texture(match self.atlas.as_ref() {
            Some(atlas) => atlas.texture_id(),
            None => return false,
        });
        let white = self.atlas.as_ref().map(Atlas::white_uv).unwrap_or_default();
        let mut canvas = Canvas { mesh, white };
        for (dx, dy, dz, graphic) in pieces {
            if !is_drawn(graphic) {
                continue;
            }
            let at = [
                f32::from(x) + f32::from(dx),
                f32::from(y) + f32::from(dy),
                f32::from(z) + f32::from(dz),
            ];
            let paint = ItemPaint {
                hue: placing.hue,
                ..ItemPaint::default()
            };
            let Some(sprite) = self.item_sprite(frame.map, graphic, paint, true) else {
                continue;
            };
            let foot = self.project(rect, at);
            let area = Rect::from_min_size(
                foot - sprite.anchor * self.zoom,
                Vec2::new(sprite.width, sprite.height) * self.zoom,
            );
            canvas.sprite(
                sprite,
                area,
                theme::with_alpha(Color32::WHITE, PLACING_ALPHA),
            );
        }
        mesh = canvas.mesh;
        if !mesh.is_empty() {
            painter.add(Shape::mesh(mesh));
        }
        // The tile the building goes on, so the human sees the exact spot.
        let center = self.project(rect, [f32::from(x), f32::from(y), f32::from(z)]);
        painter.circle_stroke(
            center,
            PAWN_RING_RX * self.zoom,
            Stroke::new(PAWN_RING_WIDTH * self.zoom, theme::GOAL),
        );
        true
    }

    /// The arrow the shard points at a place. It stands at the edge of the
    /// window when the place is out of view, as a compass needle does.
    /// Draws the arrow the shard points at a place. Gives its box, which
    /// the player clicks.
    pub fn draw_quest_arrow(
        &self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
    ) -> Option<Rect> {
        let (x, y) = frame.quest_arrow?;
        let at = self.project(rect, [f32::from(x), f32::from(y), self.camera[2]]);
        let (point, away) = arrow_at(rect, at);
        let tip = point + away * ARROW_LENGTH / 2.0;
        let back = point - away * ARROW_LENGTH / 2.0;
        let side = egui::vec2(-away.y, away.x) * ARROW_WIDTH;
        let corners = vec![tip, back + side, back - side];
        let area = Rect::from_points(&corners);
        painter.add(Shape::convex_polygon(
            corners,
            theme::GOAL,
            Stroke::new(ARROW_EDGE_WIDTH, theme::TEXT),
        ));
        Some(area)
    }

    /// The picture of a mobile as he stands and faces the watcher, for a
    /// paperdoll. It carries what he wears.
    pub fn doll_picture(
        &mut self,
        map: u8,
        look: &crate::view::WatchLook,
    ) -> Option<(egui::TextureId, Sprite)> {
        self.standing_picture(map, look, DOLL_FACING, Paint::outlined(theme::SELF_FIGURE))
    }

    /// The picture of a mobile as he stands turned to `direction`, with no
    /// ring round it, as the figure of a new character shows.
    pub fn turned_picture(
        &mut self,
        map: u8,
        look: &crate::view::WatchLook,
        direction: u8,
    ) -> Option<(egui::TextureId, Sprite)> {
        self.standing_picture(map, look, direction, Paint::outlined(Color32::TRANSPARENT))
    }

    /// The picture of a creature as a shopkeeper shows it for sale:
    /// standing, facing the watcher, with no ring round it.
    pub fn creature_picture(
        &mut self,
        map: u8,
        body: u16,
        hue: u16,
    ) -> Option<(egui::TextureId, Sprite)> {
        let look = crate::view::WatchLook {
            body,
            hue,
            ..crate::view::WatchLook::default()
        };
        self.turned_picture(map, &look, DOLL_FACING)
    }

    fn standing_picture(
        &mut self,
        map: u8,
        look: &crate::view::WatchLook,
        direction: u8,
        paint: Paint,
    ) -> Option<(egui::TextureId, Sprite)> {
        let facing = crate::view::WatchLook {
            direction,
            ..look.clone()
        };
        let pose = Pose {
            action: Action::Stand,
            tick: 0,
        };
        self.client.as_mut()?.open_map(map);
        let atlas = self.atlas.as_mut()?;
        let sprite = self
            .client
            .as_ref()?
            .figure_sprite(atlas, map, &facing, pose, paint)?;
        Some((atlas.texture_id(), sprite))
    }

    /// Makes the texture the pictures go into, as the first draw of the
    /// world does, for gumps drawn with no world under them, as the login
    /// screens are.
    pub fn make_atlas(&mut self, ctx: &egui::Context) {
        self.atlas.get_or_insert_with(|| Atlas::new(ctx));
    }

    /// True when gumps can show in their own pictures.
    pub fn has_gump_art(&self) -> bool {
        self.client.as_ref().is_some_and(ClientArt::has_gump_art)
    }

    /// A picture of a gump, for a window that is not the map.
    pub fn gump_picture(&mut self, gump: u16, hue: u16) -> Option<(egui::TextureId, Sprite)> {
        self.hued_gump_picture(gump, hue, false)
    }

    /// A picture of a gump in a hue that colors its grey pixels only, as a
    /// body or a worn item on a paperdoll takes it when `partial` is set.
    pub fn hued_gump_picture(
        &mut self,
        gump: u16,
        hue: u16,
        partial: bool,
    ) -> Option<(egui::TextureId, Sprite)> {
        let atlas = self.atlas.as_mut()?;
        let sprite = self
            .client
            .as_ref()?
            .gump_sprite(atlas, gump, hue, partial)?;
        Some((atlas.texture_id(), sprite))
    }

    /// True when the gump draws the pixel at `x`, `y` of its picture.
    pub fn gump_drawn_at(&self, gump: u16, x: usize, y: usize) -> bool {
        self.client
            .as_ref()
            .is_some_and(|client| client.gump_drawn_at(gump, x, y))
    }

    /// What `Equipconv.def` puts in the place of a worn item on a body.
    pub fn equip_conv(&self, body: u16, worn_anim: u16) -> Option<uoterm_nav::EquipConv> {
        self.client.as_ref()?.equip_conv(body, worn_anim)
    }

    /// The color of one tile on a map of the world.
    pub fn radar_rgb(&mut self, map: u8, x: u16, y: u16) -> Option<[u8; 3]> {
        self.client.as_mut()?.radar_rgb(map, x, y)
    }

    /// The height of the land of one tile of a map of the world.
    pub fn land_z(&mut self, map: u8, x: u16, y: u16) -> Option<i8> {
        self.client.as_mut()?.land_z(map, x, y)
    }

    /// The pixels of a gump picture as the files hold them.
    pub fn gump_pixels(&self, gump: u16) -> Option<uoterm_nav::ArtPixels> {
        self.client.as_ref()?.gump_pixels(gump)
    }

    /// Where a place of the world is on the screen.
    pub fn screen_of(&self, rect: Rect, place: [f32; 3]) -> Pos2 {
        self.project(rect, place)
    }

    /// The corners of the ground within `tiles` of the character, the
    /// top one first. UO counts range as the larger of the two distances,
    /// so the tiles in range make a square, which the map shows as a
    /// diamond.
    fn range_corners(&self, rect: Rect, tiles: u8) -> [Pos2; 4] {
        let reach = f32::from(tiles) + HALF_TILE_STEP;
        let [x, y, z] = self.camera;
        [
            (-reach, -reach),
            (reach, -reach),
            (reach, reach),
            (-reach, reach),
        ]
        .map(|(dx, dy)| self.project(rect, [x + dx, y + dy, z]))
    }

    /// Outlines the ground within `tiles` of the character, as the Combat
    /// page's range circle shows how far a spell reaches.
    pub fn draw_range_circle(&self, painter: &Painter, rect: Rect, tiles: u8, color: Color32) {
        let corners = self.range_corners(rect, tiles).to_vec();
        let stroke = Stroke::new(RANGE_LINE_WIDTH * self.zoom, color);
        painter.add(Shape::closed_line(corners, stroke));
    }

    /// Where a mobile is drawn now. None for one that is not in view.
    pub fn place_of(&self, frame: &WatchFrame, serial: u32) -> Option<[f32; 3]> {
        if serial == frame.serial {
            return Some(self.camera);
        }
        self.actors.get(&serial).copied()
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Tells how often the window reads the session. The news of a step
    /// comes that much later.
    pub fn set_poll_every(&mut self, every: Duration) {
        self.stride_lag = stride_lag(every);
    }

    /// Sets the zoom, inside the range the wheel allows.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
    }

    /// Moves the camera this far from the character, in points. Zero puts
    /// the character back in the middle.
    pub fn set_peek(&mut self, peek: Vec2) {
        self.peek = peek;
    }

    /// The point of the screen the camera looks at.
    fn view_center(&self, rect: Rect) -> Pos2 {
        rect.center() - self.peek
    }

    /// The window tells where its panels are, for the next frame.
    pub fn set_panels(&mut self, panels: Vec<Rect>) {
        self.panels = panels;
    }

    /// Notes one footstep for the sound, when the walker is a person and his
    /// last footstep is long enough ago. A jump makes no footstep.
    fn step(&mut self, walker: u32, look: &WatchLook, glide: &Glide, tiles_away: f32, time: f64) {
        let person = self
            .client
            .as_ref()
            .is_some_and(|client| client.is_person(look.body));
        if !person || glide.from == glide.to {
            return;
        }
        let mounted = is_mounted(look);
        let running = glide.running;
        let gap = match (mounted, running) {
            (true, true) => STEP_GAP_MOUNT_RUN,
            (true, false) => STEP_GAP_MOUNT_WALK,
            (false, _) => STEP_GAP_ON_FOOT,
        };
        if self
            .last_step
            .get(&walker)
            .is_some_and(|at| time - at < gap)
        {
            return;
        }
        self.last_step.insert(walker, time);
        self.steps.push(Step {
            tiles_away,
            mounted,
            running,
        });
    }

    /// The footsteps since the last call, for the sound.
    pub fn take_steps(&mut self) -> Vec<Step> {
        std::mem::take(&mut self.steps)
    }

    fn land_sprite(&mut self, land_id: u16, hue: u16) -> Option<Sprite> {
        let client = self.client.as_ref()?;
        let land_id = client.season_land(self.season, land_id);
        client.land_sprite(self.atlas.as_mut()?, land_id, hue)
    }

    fn texture_sprite(&mut self, texture_id: u16, hue: u16) -> Option<Sprite> {
        let client = self.client.as_ref()?;
        client.texture_sprite(self.atlas.as_mut()?, texture_id, hue)
    }

    /// The picture of an item as it shows now. `animate` lets a fire or a
    /// fountain go through its pictures.
    fn item_sprite(
        &mut self,
        map: u8,
        graphic: u16,
        paint: ItemPaint,
        animate: bool,
    ) -> Option<Sprite> {
        let client = self.client.as_ref()?;
        let now_ms = (self.now * MS_PER_SECOND) as u64;
        let graphic = client.season_item(self.season, graphic);
        let shown = if animate {
            client.shown_graphic(map, graphic, now_ms)
        } else {
            graphic
        };
        client.item_sprite(self.atlas.as_mut()?, map, shown, paint)
    }

    /// Where a place of the world is in the window. The camera moves in
    /// whole screen pixels, and each place lands on a whole screen pixel.
    /// The art is drawn with sharp pixels, so a part of a pixel would make
    /// the pixels of the art crawl while the map scrolls.
    fn project(&self, rect: Rect, at: [f32; 3]) -> Pos2 {
        let on_plane = |place: [f32; 3]| {
            Vec2::new(
                (place[0] - place[1]) * HALF_TILE,
                (place[0] + place[1]) * HALF_TILE - place[2] * Z_PIXELS,
            ) * self.zoom
        };
        let snap = |v: Vec2| (v * self.pixels_per_point).round() / self.pixels_per_point;
        let camera = snap(on_plane(self.camera));
        self.view_center(rect) + snap(on_plane(at) - camera)
    }

    /// The things that stand on each tile this frame.
    fn standing<'a>(&self, frame: &'a WatchFrame) -> HashMap<(i32, i32), Vec<Standing<'a>>> {
        let mut out: HashMap<(i32, i32), Vec<Standing<'a>>> = HashMap::new();
        let tile = |at: [f32; 3]| (at[0].round() as i32, at[1].round() as i32);
        for item in frame.items.iter().filter(|i| is_drawn(i.graphic)) {
            let thing = if item.graphic == CORPSE_GRAPHIC {
                Standing::Corpse(item)
            } else {
                Standing::Art(item)
            };
            out.entry((i32::from(item.x), i32::from(item.y)))
                .or_default()
                .push(thing);
        }
        // A house a player designed is drawn from its own tiles, not from
        // the plain multi its foundation names.
        let designed: HashMap<u32, &uoterm_world::DesignedHouse> = frame
            .designed_houses
            .iter()
            .map(|house| (house.serial.0, house))
            .collect();
        // The house being designed shows each storey as the designer asks.
        let designing = frame.designing.map(|designing| designing.serial);
        for (multi, house) in frame
            .multis
            .iter()
            .filter_map(|multi| Some((multi, *designed.get(&multi.serial)?)))
        {
            for tile in house.tiles.iter().filter(|t| is_drawn(t.graphic)) {
                let look = if designing == Some(multi.serial) {
                    let kind = kind_of(&frame.house_parts, tile.graphic);
                    piece_look(&self.storey_looks, kind, tile.dz)
                } else {
                    PieceLook::Shown
                };
                if look == PieceLook::Hidden {
                    continue;
                }
                let at = (i32::from(multi.x) + tile.dx, i32::from(multi.y) + tile.dy);
                out.entry(at).or_default().push(Standing::Piece {
                    graphic: tile.graphic,
                    z: f32::from(multi.z) + tile.dz as f32,
                    faded: look == PieceLook::SeeThrough,
                });
            }
        }
        for (multi, piece) in frame.multis.iter().flat_map(|multi| {
            let designed = designed.contains_key(&multi.serial);
            let pieces = self
                .client
                .as_ref()
                .filter(|_| !designed)
                .map_or(&[][..], |client| client.multi_pieces(multi.multi_id));
            pieces.iter().map(move |piece| (multi, piece))
        }) {
            if !is_drawn(piece.graphic) {
                continue;
            }
            let tile = (
                i32::from(multi.x) + i32::from(piece.dx),
                i32::from(multi.y) + i32::from(piece.dy),
            );
            out.entry(tile).or_default().push(Standing::Piece {
                graphic: piece.graphic,
                z: f32::from(multi.z) + f32::from(piece.dz),
                faded: false,
            });
        }
        for mobile in &frame.mobiles {
            let at = self.actors[&mobile.serial];
            out.entry(tile(at))
                .or_default()
                .push(Standing::Mobile { mobile, at });
        }
        out.entry(tile(self.camera))
            .or_default()
            .push(Standing::Character { at: self.camera });
        out
    }

    /// The heights the world is cut at over the character, from what stands
    /// on his tile and on the tile in front of him.
    fn ceiling(
        &mut self,
        frame: &WatchFrame,
        standing: &HashMap<(i32, i32), Vec<Standing<'_>>>,
    ) -> Ceiling {
        let draw_roofs = !self.look.general.hide_roofs;
        let Some(client) = self.client.as_mut() else {
            return Ceiling::open(draw_roofs);
        };
        let mut over = |x: i32, y: i32| -> (Option<i16>, Vec<Over>) {
            let (Ok(tile_x), Ok(tile_y)) = (u16::try_from(x), u16::try_from(y)) else {
                return (None, Vec::new());
            };
            let mut land = None;
            let mut found = Vec::new();
            if let Some(cell) = client.cell(frame.map, tile_x, tile_y) {
                land = cell.land_id.map(|_| i16::from(cell.average_z));
                found.extend(cell.statics.iter().map(|s| Over {
                    z: i16::from(s.z),
                    flags: s.flags,
                }));
            }
            for thing in standing.get(&(x, y)).into_iter().flatten() {
                if let Standing::Piece { graphic, z, .. } = thing {
                    found.push(Over {
                        z: *z as i16,
                        flags: client
                            .item_tile(*graphic)
                            .map_or(TileFlagSet::NONE, |tile| tile.flags),
                    });
                }
            }
            (land, found)
        };
        let (x, y) = (self.camera[0].round() as i32, self.camera[1].round() as i32);
        let (land, here) = over(x, y);
        let (_, ahead) = over(x + 1, y + 1);
        ceiling_of(
            self.camera[2].round() as i16,
            land,
            &here,
            &ahead,
            draw_roofs,
        )
    }

    /// The circle of transparency round the character this frame, when the
    /// General page turns it on.
    fn circle(&self, rect: Rect) -> Option<Circle> {
        let general = &self.look.general;
        general.circle_of_transparency.then(|| Circle {
            center: self.project(rect, self.camera) - Vec2::new(0.0, HALF_TILE * self.zoom),
            radius: f32::from(general.circle_radius),
            style: general.circle_style,
        })
    }

    /// How much of a fading thing shows this frame, as it moves toward
    /// `target`. A thing that shows whole is forgotten.
    fn alpha_of(&mut self, key: FadeKey, target: f32) -> f32 {
        let before = self.fades.get(&key).map_or(1.0, |(alpha, _)| *alpha);
        let alpha = look::faded(
            before,
            target,
            self.frame_seconds,
            self.look.general.object_fading,
        );
        if alpha >= 1.0 {
            self.fades.remove(&key);
        } else {
            self.fades.insert(key, (alpha, self.frame_count));
        }
        alpha
    }

    /// How much of a thing at `z` should show: none over the ceiling or on
    /// a hidden roof, a part of a translucent one, and all of the rest.
    fn target_alpha(&self, z: i16, flags: TileFlagSet) -> f32 {
        let ceiling = self.ceiling;
        if z >= ceiling.max_z || ceiling.no_draw_roofs && flags.contains(TileFlagSet::ROOF) {
            0.0
        } else if flags.contains(TileFlagSet::TRANSLUCENT) {
            look::TRANSLUCENT_ALPHA
        } else {
            1.0
        }
    }

    fn build(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        self.picks.clear();
        self.light_sources.clear();
        let mut standing = self.standing(frame);
        self.ceiling = self.ceiling(frame, &standing);
        self.circle = self.circle(rect);
        let half = HALF_TILE * self.zoom;
        let rows = ((rect.height() / 2.0 + self.peek.y.abs()) / half).ceil() as i32;
        let columns =
            ((rect.width() / 2.0 + self.peek.x.abs()) / half).ceil() as i32 + MARGIN_COLUMNS;
        let center = (self.camera[0].round() as i32, self.camera[1].round() as i32);
        let radar_half = frame.radar.len() as i32 / 2;
        for row in -rows - MARGIN_ROWS_TOP..=rows + MARGIN_ROWS_BOTTOM {
            for column in (-columns..=columns).filter(|c| (c + row) % 2 == 0) {
                let (dx, dy) = ((row + column) / 2, (row - column) / 2);
                let (x, y) = (center.0 + dx, center.1 + dy);
                let (Ok(tile_x), Ok(tile_y)) = (u16::try_from(x), u16::try_from(y)) else {
                    continue;
                };
                let things = standing.remove(&(x, y)).unwrap_or_default();
                if self.client.is_some() {
                    self.real_tile(rect, frame, (tile_x, tile_y), canvas);
                } else {
                    let symbol = radar_symbol(
                        frame,
                        radar_half + i32::from(tile_x) - i32::from(frame.x),
                        radar_half + i32::from(tile_y) - i32::from(frame.y),
                    );
                    self.flat_tile(rect, (tile_x, tile_y), symbol, canvas);
                }
                self.things(rect, frame, (tile_x, tile_y), things, canvas, plates);
            }
        }
        let shown = self.frame_count;
        self.fades.retain(|_, (_, seen)| *seen == shown);
    }

    fn real_tile(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        (x, y): (u16, u16),
        canvas: &mut Canvas,
    ) {
        let Some(cell) = self
            .client
            .as_mut()
            .and_then(|client| client.cell(frame.map, x, y).cloned())
        else {
            return;
        };
        if let Some(land_id) = cell.land_id {
            self.land(rect, frame, (x, y), &cell, land_id, canvas);
        }
        for piece in &cell.statics {
            let art = StaticArt {
                tile: (x, y),
                z: f32::from(piece.z),
                graphic: piece.graphic,
                hue: piece.hue,
                flags: piece.flags,
                height: piece.height,
                piece: false,
                faded: false,
            };
            self.static_art(rect, frame, art, canvas);
        }
    }

    /// The land of one tile: its texture stretched over a slope and lit by
    /// the way each corner faces, or its own flat picture.
    fn land(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        (x, y): (u16, u16),
        cell: &Cell,
        land_id: u16,
        canvas: &mut Canvas,
    ) {
        let over_ground = i16::from(cell.average_z) > self.ceiling.max_ground_z;
        let alpha = self.alpha_of(FadeKey::Land { x, y }, if over_ground { 0.0 } else { 1.0 });
        if alpha <= 0.0 {
            return;
        }
        let out_of_range = tiles_from(frame, x, y) > look::VIEW_RANGE;
        let hue = look::thing_hue(&self.look, frame.dead, false, out_of_range).unwrap_or(0);
        let (fx, fy) = (f32::from(x), f32::from(y));
        let [top, right, bottom, left] = cell.corners.map(f32::from);
        let points = [
            self.project(rect, [fx - 0.5, fy - 0.5, top]),
            self.project(rect, [fx + 0.5, fy - 0.5, right]),
            self.project(rect, [fx + 0.5, fy + 0.5, bottom]),
            self.project(rect, [fx - 0.5, fy + 0.5, left]),
        ];
        let lit =
            |light: f32| theme::with_alpha(Color32::WHITE.gamma_multiply(light).to_opaque(), alpha);
        if let Some(stretch) = cell.stretch {
            if let Some(sprite) = self.texture_sprite(stretch.texture_id, hue) {
                let bright =
                    f32::from(self.look.video.terrain_shadows_level) * TERRAIN_SHADOWS_STEP;
                let colors = stretch
                    .normals
                    .map(|normal| lit(land_light(normal, bright)));
                canvas.quad_colors(points, corners(sprite.uv), colors);
                return;
            }
        }
        let Some(sprite) = self.land_sprite(land_id, hue) else {
            return;
        };
        let uv = sprite.uv;
        let diamond = [
            Pos2::new(uv.center().x, uv.top()),
            Pos2::new(uv.right(), uv.center().y),
            Pos2::new(uv.center().x, uv.bottom()),
            Pos2::new(uv.left(), uv.center().y),
        ];
        let flat = cell.corners.iter().all(|z| *z == cell.corners[0]);
        if flat {
            canvas.quad(points, diamond, lit(1.0));
            if self.look.video.animated_water && cell.wet {
                let area = Rect::from_points(&points);
                canvas.water(sprite, area, lit(1.0), self.now);
            }
            return;
        }
        // A slope with no texture bends its flat picture, lit by its tilt.
        let light = (LAND_LIGHT_BASE
            + LAND_LIGHT_SIDE * (right - left)
            + LAND_LIGHT_FRONT * (top - bottom))
            .clamp(LAND_LIGHT_MIN, 1.0);
        canvas.quad(points, diamond, lit(light));
    }

    /// One static of the map or one piece of a house, by the choices of the
    /// General and Video pages: stumps, plants, cave marks, the fading of
    /// roofs, the circle of transparency, shadows, water, and its light.
    fn static_art(&mut self, rect: Rect, frame: &WatchFrame, art: StaticArt, canvas: &mut Canvas) {
        let flags = art.flags;
        let general = &self.look.general;
        let movable = flags.contains(TileFlagSet::MULTI_MOVABLE);
        let foliage = flags.contains(TileFlagSet::FOLIAGE);
        let bare = foliage && !movable && self.season >= SEASON_WINTER;
        let stumped = foliage && general.trees_to_stumps && (!art.piece || !movable);
        let hidden_plant =
            !movable && general.hide_vegetation && filters::is_vegetation(art.graphic, flags);
        if flags.contains(TileFlagSet::INTERNAL) || bare || stumped || hidden_plant {
            return;
        }
        let tree = filters::is_tree(art.graphic, flags);
        let graphic = if tree && general.trees_to_stumps {
            filters::STUMP_GRAPHIC
        } else {
            art.graphic
        };
        let border = general.mark_cave_tiles && filters::is_cave(graphic);
        let (x, y) = art.tile;
        let key = FadeKey::Tile {
            x,
            y,
            z: art.z as i8,
            graphic: art.graphic,
        };
        let alpha = self.alpha_of(key, self.target_alpha(art.z as i16, flags));
        if alpha <= 0.0 {
            return;
        }
        let alpha = if art.faded {
            alpha * DESIGN_SEE_THROUGH
        } else {
            alpha
        };
        let out_of_range = tiles_from(frame, x, y) > look::VIEW_RANGE;
        let paint = self.paint(frame.dead, false, out_of_range, art.hue, border);
        let at = [f32::from(x), f32::from(y), art.z];
        let Some(sprite) = self.item_sprite(frame.map, graphic, paint, true) else {
            return;
        };
        let center = self.project(rect, at);
        let area = self.art_rect(center, sprite);
        let see_through = !tree
            && !foliage
            && look::see_through(
                art.z as i8,
                art.height,
                flags.contains(TileFlagSet::SURFACE),
                flags.contains(TileFlagSet::BACKGROUND),
                flags.contains(TileFlagSet::ROOF) || flags.contains(TileFlagSet::WALL),
                self.camera[2].round() as i8,
            );
        let video = &self.look.video;
        let lay = Lay {
            alpha,
            see_through,
            shadow: video.shadows
                && video.statics_shadows
                && (tree || foliage || filters::is_rock(graphic)),
            water: video.animated_water && flags.contains(TileFlagSet::WET),
        };
        self.lay(canvas, sprite, area, lay);
        if flags.contains(TileFlagSet::LIGHT_SOURCE) {
            let z = art.z as i16;
            let holder = LightHolder::MapStatic;
            self.add_ground_light(frame.map, (x, y), z, art.graphic, holder, center);
        }
    }

    /// Where the picture of an item goes. Its bottom edge lies on the bottom
    /// point of its tile, and it is centered on the tile.
    fn art_rect(&self, tile_center: Pos2, sprite: Sprite) -> Rect {
        let size = Vec2::new(sprite.width, sprite.height) * self.zoom;
        let bottom = tile_center.y + HALF_TILE * self.zoom;
        Rect::from_min_size(
            Pos2::new(tile_center.x - size.x / 2.0, bottom - size.y),
            size,
        )
    }

    fn flat_tile(&self, rect: Rect, (x, y): (u16, u16), symbol: Option<char>, canvas: &mut Canvas) {
        let (fx, fy) = (f32::from(x), f32::from(y));
        let corner = |dx: f32, dy: f32, lift: f32| {
            self.project(rect, [fx + dx, fy + dy, self.camera[2]])
                - Vec2::new(0.0, lift * self.zoom)
        };
        let diamond = |lift: f32| {
            [
                corner(-0.5, -0.5, lift),
                corner(0.5, -0.5, lift),
                corner(0.5, 0.5, lift),
                corner(-0.5, 0.5, lift),
            ]
        };
        let (color, height) = match symbol {
            None => (theme::FLAT_UNKNOWN, 0.0),
            Some(SYM_BLOCK) => (theme::FLAT_BLOCK_TOP, FLAT_BLOCK_HEIGHT),
            Some(SYM_DOOR) => (theme::FLAT_DOOR, FLAT_DOOR_HEIGHT),
            Some(SYM_WATER) => (theme::FLAT_WATER, 0.0),
            _ if (x + y) % 2 == 0 => (theme::FLAT_WALK, 0.0),
            _ => (theme::FLAT_WALK_ALT, 0.0),
        };
        if height > 0.0 {
            let [_, right, bottom, left] = diamond(0.0);
            let [_, top_right, top_bottom, top_left] = diamond(height);
            canvas.fill(
                &[top_left, top_bottom, bottom, left],
                theme::FLAT_BLOCK_LEFT,
            );
            canvas.fill(
                &[top_bottom, top_right, right, bottom],
                theme::FLAT_BLOCK_RIGHT,
            );
        }
        canvas.fill(&diamond(height), color);
        let grid = Color32::WHITE.gamma_multiply(FLAT_GRID_ALPHA);
        let [top, right, _, left] = diamond(height);
        let hair = Vec2::new(0.0, HAIRLINE);
        if i32::from(y) % FLAT_GRID_EVERY == 0 {
            canvas.fill(&[top, right, right + hair, top + hair], grid);
        }
        if i32::from(x) % FLAT_GRID_EVERY == 0 {
            canvas.fill(&[left, top, top + hair, left + hair], grid);
        }
    }

    fn things(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        tile: (u16, u16),
        mut things: Vec<Standing<'_>>,
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        things.sort_by(|a, b| a.z().total_cmp(&b.z()));
        for thing in things {
            match thing {
                Standing::Art(item) => self.ground_item(rect, frame, item, canvas, plates),
                Standing::Piece { graphic, z, faded } => {
                    let (flags, height) = self
                        .client
                        .as_ref()
                        .and_then(|client| client.item_tile(graphic))
                        .map_or((TileFlagSet::NONE, 0), |tile| (tile.flags, tile.height));
                    let art = StaticArt {
                        tile,
                        z,
                        graphic,
                        hue: 0,
                        flags,
                        height,
                        piece: true,
                        faded,
                    };
                    self.static_art(rect, frame, art, canvas);
                }
                Standing::Corpse(item) => self.corpse(rect, frame, item, canvas, plates),
                Standing::Mobile { mobile, at } => {
                    self.mobile(rect, frame, mobile, at, canvas, plates);
                }
                Standing::Character { at } => self.character(rect, frame, at, canvas, plates),
            }
        }
    }

    /// One item on the ground: its hue, the way fields show, its name and
    /// its light.
    fn ground_item(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        item: &WatchItem,
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        let flags = self
            .client
            .as_ref()
            .and_then(|client| client.item_tile(item.graphic))
            .map_or(TileFlagSet::NONE, |tile| tile.flags);
        let bare = flags.contains(TileFlagSet::FOLIAGE)
            && !flags.contains(TileFlagSet::MULTI_MOVABLE)
            && self.season >= SEASON_WINTER;
        if flags.contains(TileFlagSet::INTERNAL) || bare {
            return;
        }
        let key = FadeKey::Serial(item.serial);
        let alpha = self.alpha_of(key, self.target_alpha(i16::from(item.z), flags));
        if alpha <= 0.0 {
            return;
        }
        let field_style = self.look.general.field_style;
        let field_hue = filters::field_tile_hue(item.graphic)
            .filter(|_| flags.contains(TileFlagSet::ANIMATION));
        let (graphic, own_hue) = match field_hue {
            Some(hue) if field_style == FieldStyle::Tile => (filters::FIELD_TILE_GRAPHIC, hue),
            _ => (item.graphic, item.hue),
        };
        let animate = field_hue.is_none() || field_style == FieldStyle::Normal;
        let hovered = self.hovered == Some(item.serial);
        let out_of_range = tiles_from(frame, item.x, item.y) > look::VIEW_RANGE;
        let paint = self.paint(frame.dead, hovered, out_of_range, own_hue, false);
        let at = [f32::from(item.x), f32::from(item.y), f32::from(item.z)];
        let center = self.project(rect, at);
        let area = match self.item_sprite(frame.map, graphic, paint, animate) {
            Some(sprite) => {
                let area = self.art_rect(center, sprite);
                canvas.sprite(sprite, area, theme::with_alpha(Color32::WHITE, alpha));
                area
            }
            None if self.client.is_none() => {
                let radius = Vec2::new(FLAT_ITEM_RADIUS * 2.0, FLAT_ITEM_RADIUS) * self.zoom;
                canvas.fill(&ellipse(center, radius), theme::FLAT_ITEM);
                Rect::from_center_size(center, radius * 2.0)
            }
            None => return,
        };
        self.picks.push(Pick {
            area,
            serial: item.serial,
            name: item.name.clone(),
            kind: PickKind::Item,
        });
        plates.push(self.plate(PlateOf::Item, &item.name, area, center, ITEM_NAME_HUE, None));
        let holder = LightHolder::GroundItem {
            packet_light: item.direction,
        };
        let tile = (item.x, item.y);
        self.add_ground_light(
            frame.map,
            tile,
            i16::from(item.z),
            item.graphic,
            holder,
            center,
        );
    }

    /// A corpse is the fallen body, lying the way the shard says. The amount
    /// of a corpse item is the body it was. With no picture of the death, a
    /// ring marks the place.
    fn corpse(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        item: &WatchItem,
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        let key = FadeKey::Serial(item.serial);
        let alpha = self.alpha_of(key, self.target_alpha(i16::from(item.z), TileFlagSet::NONE));
        if alpha <= 0.0 {
            return;
        }
        let center = self.project(
            rect,
            [f32::from(item.x), f32::from(item.y), f32::from(item.z)],
        );
        let hovered = self.hovered == Some(item.serial);
        let out_of_range = tiles_from(frame, item.x, item.y) > look::VIEW_RANGE;
        let look = WatchLook {
            body: item.amount,
            hue: item.hue,
            direction: item.direction & DIRECTION_MASK,
            ..WatchLook::default()
        };
        let paint = Paint {
            outline: self.outline(theme::CORPSE),
            whole_hue: look::thing_hue(&self.look, frame.dead, hovered, out_of_range),
        };
        let sprite = self
            .client
            .as_ref()
            .zip(self.atlas.as_mut())
            .and_then(|(client, atlas)| client.corpse_sprite(atlas, frame.map, &look, paint));
        let area = match sprite {
            Some(sprite) => {
                let area = Rect::from_min_size(
                    center - sprite.anchor * self.zoom,
                    Vec2::new(sprite.width, sprite.height) * self.zoom,
                );
                canvas.sprite(sprite, area, theme::with_alpha(Color32::WHITE, alpha));
                area
            }
            None => {
                let radius = Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * self.zoom;
                canvas.ring(center, radius, PAWN_RING_WIDTH * self.zoom, theme::CORPSE);
                Rect::from_center_size(center, radius * 2.0)
            }
        };
        self.picks.push(Pick {
            area,
            serial: item.serial,
            name: item.name.clone(),
            kind: PickKind::Corpse,
        });
        plates.push(self.plate(
            PlateOf::Corpse,
            &item.name,
            area,
            center,
            ITEM_NAME_HUE,
            None,
        ));
    }

    /// One mobile, with the hue, the ring, the shadow and the seat the
    /// Options screen asks for.
    fn mobile(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        mobile: &WatchMobile,
        at: [f32; 3],
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        let key = FadeKey::Serial(mobile.serial);
        let alpha = self.alpha_of(
            key,
            self.target_alpha(at[2].round() as i16, TileFlagSet::NONE),
        );
        if alpha <= 0.0 {
            return;
        }
        let target = !frame.combatant.is_empty() && mobile.name == frame.combatant;
        let hovered = self.hovered == Some(mobile.serial);
        let dead = uoterm_world::is_ghost_body(mobile.look.body);
        let state = MobileState {
            own: false,
            hovered,
            marked: target || frame.target_cursor && hovered,
            out_of_range: mobile.dist > look::VIEW_RANGE,
            hidden: mobile.hidden,
            dead,
            person: self.is_person(mobile.look.body),
            poisoned: mobile.poisoned,
            paralyzed: mobile.paralyzed,
            yellow_hits: mobile.yellow_hits,
            notoriety: mobile.notoriety,
        };
        let party = frame
            .party_members
            .iter()
            .any(|member| member.serial == mobile.serial);
        let glide = self.glides.get(&mobile.serial).copied();
        let pose = self
            .shown_pose(mobile.serial)
            .unwrap_or_else(|| glide.map_or(STANDING, |glide| glide.pose(self.now)));
        let moving_look = turned_to(
            &mobile.look,
            glide.and_then(|glide| glide.heading(self.now)),
        );
        let look = moving_look.as_ref().unwrap_or(&mobile.look);
        let resting = glide.is_none_or(|glide| !glide.moving(self.now));
        let seat = self.seat_of(frame, look, at, resting);
        let shown = look::without_hidden_layers(&self.look.general, look, false);
        let looks = Looks {
            look: shown.as_ref().unwrap_or(look),
            pose,
            color: theme::notoriety_color(mobile.notoriety),
            alpha,
            hue: look::mobile_hue(&self.look, frame.dead, state),
            aura: self.aura(frame, mobile.notoriety, party),
            shadow: self.look.video.shadows && !dead && !mobile.hidden,
            seat,
        };
        let foot = self.project(rect, at);
        let area = self.figure(canvas, frame.map, looks, foot, target);
        self.held_lights(mobile.serial, look, area);
        self.picks.push(Pick {
            area,
            serial: mobile.serial,
            name: mobile.name.clone(),
            kind: PickKind::Mobile,
        });
        let mut plate = self.plate(
            PlateOf::Mobile,
            &mobile.name,
            area,
            foot,
            self.look.notoriety_hue(mobile.notoriety),
            mobile.hits_percent,
        );
        plate.color = theme::notoriety_color(mobile.notoriety);
        plate.target = target;
        plate.hits = look::hits_shown(&self.look.general, mobile.hits_percent, target, dead);
        plate.poisoned = mobile.poisoned;
        plate.yellow_hits = mobile.yellow_hits;
        plates.push(plate);
    }

    /// The character of the window, drawn at the place the camera follows.
    fn character(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        at: [f32; 3],
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        let foot = self.project(rect, at);
        let classic = self.look.classic();
        let alpha = if frame.dead {
            GHOST_ALPHA
        } else if frame.hidden && !classic {
            PAWN_HIDDEN_ALPHA
        } else {
            1.0
        };
        if !classic {
            if frame.dead {
                let radius = Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * self.zoom;
                canvas.ring(foot, radius, PAWN_RING_WIDTH * self.zoom, theme::CORPSE);
            } else {
                self.facing_mark(canvas, foot, &frame.facing);
            }
        }
        let glide = self.camera_glide;
        let pose = self
            .shown_pose(frame.serial)
            .unwrap_or_else(|| glide.map_or(STANDING, |glide| glide.pose(self.now)));
        let moving_look = turned_to(&frame.look, glide.and_then(|glide| glide.heading(self.now)));
        let look = moving_look.as_ref().unwrap_or(&frame.look);
        let resting = glide.is_none_or(|glide| !glide.moving(self.now));
        let seat = self.seat_of(frame, look, at, resting);
        let shown = look::without_hidden_layers(&self.look.general, look, true);
        let state = MobileState {
            own: true,
            hidden: frame.hidden,
            dead: frame.dead,
            person: true,
            poisoned: frame.poisoned,
            paralyzed: frame.paralyzed,
            notoriety: frame.notoriety,
            ..MobileState::default()
        };
        let looks = Looks {
            look: shown.as_ref().unwrap_or(look),
            pose,
            color: theme::SELF_FIGURE,
            alpha,
            hue: look::mobile_hue(&self.look, frame.dead, state).filter(|_| classic),
            aura: self.aura(frame, frame.notoriety, false),
            shadow: self.look.video.shadows && !frame.dead && !frame.hidden,
            seat,
        };
        let area = self.figure(canvas, frame.map, looks, foot, false);
        self.held_lights(frame.serial, look, area);
        let hits_percent = (frame.hits_max > 0).then(|| {
            (u32::from(frame.hits) * u32::from(PERCENT_MAX) / u32::from(frame.hits_max)) as u8
        });
        let mut plate = self.plate(
            PlateOf::Mobile,
            &frame.name,
            area,
            foot,
            self.look.notoriety_hue(frame.notoriety),
            hits_percent,
        );
        plate.target = true;
        plate.hits = look::hits_shown(&self.look.general, hits_percent, false, frame.dead);
        plate.poisoned = frame.poisoned;
        // The Modern style shows the character's name on its own bar.
        if classic {
            plates.push(plate);
        }
    }

    /// What floats over one thing, as the Nameplates page says.
    fn plate(
        &self,
        of: PlateOf,
        name: &str,
        area: Rect,
        foot: Pos2,
        hue: u16,
        hits_percent: Option<u8>,
    ) -> Plate {
        let full_health = hits_percent.is_none_or(|percent| percent >= PERCENT_MAX);
        Plate {
            top: area.center_top(),
            foot,
            name: name.to_string(),
            color: theme::TEXT,
            hue,
            of,
            hits_percent,
            target: false,
            hits: HitsShown::default(),
            named: look::plate_shows(&self.look.nameplates, of, self.ctrl_shift, full_health)
                && !name.is_empty(),
            poisoned: false,
            yellow_hits: false,
        }
    }

    /// The hue of the ring under the feet of a mobile, when it shows.
    fn aura(&self, frame: &WatchFrame, notoriety: u8, party: bool) -> Option<u16> {
        let general = &self.look.general;
        look::aura_shows(general.aura_under_feet, frame.war, self.ctrl_shift).then(|| {
            if party && general.party_aura {
                general.party_aura_hue
            } else {
                self.look.notoriety_hue(notoriety)
            }
        })
    }

    /// The seat a person takes on a chair under him, while he rests.
    fn seat_of(
        &mut self,
        frame: &WatchFrame,
        look: &WatchLook,
        at: [f32; 3],
        resting: bool,
    ) -> Option<Seat> {
        if !resting || is_mounted(look) || !self.is_person(look.body) {
            return None;
        }
        let (x, y) = (at[0].round() as u16, at[1].round() as u16);
        let z = at[2].round() as i16;
        let near = |thing_z: i16| (thing_z - z).abs() <= SEAT_REACH;
        let statics: Vec<u16> = self
            .client
            .as_mut()
            .and_then(|client| client.cell(frame.map, x, y))
            .map(|cell| {
                cell.statics
                    .iter()
                    .filter(|piece| near(i16::from(piece.z)))
                    .map(|piece| piece.graphic)
                    .collect()
            })
            .unwrap_or_default();
        let items = frame
            .items
            .iter()
            .filter(|item| item.x == x && item.y == y && near(i16::from(item.z)))
            .map(|item| item.graphic);
        statics
            .into_iter()
            .chain(items)
            .find_map(|graphic| filters::seat(graphic, look.direction))
    }

    /// True when the body is a person.
    fn is_person(&self, body: u16) -> bool {
        self.client
            .as_ref()
            .is_some_and(|client| client.is_person(body))
    }

    /// The ring round a figure in the Modern style. The Classic style draws
    /// none.
    fn outline(&self, color: Color32) -> [u8; 4] {
        if self.look.classic() {
            Color32::TRANSPARENT.to_array()
        } else {
            color.to_array()
        }
    }

    /// How an item picture is painted: in the hue the Options screen puts
    /// over it, or in its own.
    fn paint(
        &self,
        dead_world: bool,
        hovered: bool,
        out_of_range: bool,
        hue: u16,
        border: bool,
    ) -> ItemPaint {
        let over = look::thing_hue(&self.look, dead_world, hovered, out_of_range);
        ItemPaint {
            hue: over.unwrap_or(hue),
            whole_hue: over.is_some(),
            border,
        }
    }

    /// Lays one picture of the world on the canvas as `lay` says.
    fn lay(&self, canvas: &mut Canvas, sprite: Sprite, area: Rect, lay: Lay) {
        let color = theme::with_alpha(Color32::WHITE, lay.alpha);
        if lay.shadow {
            canvas.shadow(sprite, area, self.zoom, lay.alpha);
        }
        match self.circle.filter(|_| lay.see_through) {
            Some(circle) if circle.style == CircleStyle::Gradient => {
                let middle = area.center_bottom() - Vec2::new(0.0, HALF_TILE * self.zoom);
                let ratio = middle.distance(circle.center) / circle.radius;
                let shown = look::circle_alpha(CircleStyle::Gradient, ratio);
                canvas.sprite(
                    sprite,
                    area,
                    theme::with_alpha(Color32::WHITE, lay.alpha * shown),
                );
            }
            Some(circle) if area.distance_to_pos(circle.center) < circle.radius => {
                canvas.sprite_grid(sprite, area, color, CIRCLE_CELL, |point| {
                    look::circle_alpha(
                        CircleStyle::Full,
                        point.distance(circle.center) / circle.radius,
                    )
                });
            }
            _ => canvas.sprite(sprite, area, color),
        }
        if lay.water {
            canvas.water(sprite, area, color, self.now);
        }
    }

    /// The light a thing of `graphic` throws, when it throws one. `seed`
    /// tells its flicker from the flicker of the next light.
    fn light_of(
        &self,
        graphic: u16,
        holder: LightHolder,
        center: Pos2,
        seed: u32,
    ) -> Option<LightSource> {
        let tile = self.client.as_ref()?.item_tile(graphic)?;
        let light = item_light(graphic, tile, holder)?;
        let rules = LightRules::from(&self.look.video);
        Some(LightSource {
            center,
            shape: light.shape,
            color: shown_light_color(&rules, graphic, light.color),
            strength: if rules.candle_flicker {
                flicker(seed, self.now)
            } else {
                1.0
            },
        })
    }

    /// Adds the light a thing on the ground throws, unless a wall in front
    /// of it hides it.
    fn add_ground_light(
        &mut self,
        map: u8,
        tile: (u16, u16),
        z: i16,
        graphic: u16,
        holder: LightHolder,
        center: Pos2,
    ) {
        if let Some(source) = self.light_of(graphic, holder, center, tile_seed(tile)) {
            if !self.light_hidden(map, tile, z) {
                self.light_sources.push(source);
            }
        }
    }

    /// The lights mobile `serial` holds, such as a torch or a lantern,
    /// placed as the classic client places them for the way he faces.
    fn held_lights(&mut self, serial: u32, look: &WatchLook, area: Rect) {
        let mirrored = Facing::from_direction(look.direction).mirrored;
        let base = Pos2::new(
            if mirrored { area.right() } else { area.left() },
            area.top(),
        );
        let center = base + held_light_offset(look.direction) * self.zoom;
        let held: Vec<LightSource> = look
            .equipment
            .iter()
            .filter_map(|item| self.light_of(item.graphic, LightHolder::Held, center, serial))
            .collect();
        self.light_sources.extend(held);
    }

    /// True when a wall in front hides the light of a thing at this tile.
    fn light_hidden(&mut self, map: u8, (x, y): (u16, u16), z: i16) -> bool {
        let max_z = self.ceiling.max_z;
        let Some(cell) = self
            .client
            .as_mut()
            .and_then(|client| client.cell(map, x.saturating_add(1), y.saturating_add(1)))
        else {
            return false;
        };
        cell.statics.iter().any(|piece| {
            !piece.flags.contains(TileFlagSet::TRANSPARENT)
                && (z + LIGHT_WALL_RISE..max_z).contains(&i16::from(piece.z))
        })
    }

    /// Lays the light of the world over the map: the light level, and each
    /// lamp, torch and lit spell in view. `effects` are the places and the
    /// graphics of the spells that show now.
    pub fn draw_lights(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        effects: &[([f32; 3], u16)],
    ) {
        if frame.dead && self.look.video.black_and_white_when_dead {
            return;
        }
        let rules = LightRules::from(&self.look.video);
        let light = world_light(frame.light, frame.personal_light, &rules);
        let spells: Vec<LightSource> = effects
            .iter()
            .filter_map(|(place, graphic)| {
                let tile = (place[0] as u16, place[1] as u16);
                let center = self.project(rect, *place);
                self.light_of(*graphic, LightHolder::MapStatic, center, tile_seed(tile))
            })
            .collect();
        self.light_sources.extend(spells);
        if light.level >= 1.0 && !rules.alternative {
            return;
        }
        let sources = std::mem::take(&mut self.light_sources);
        if let Some(client) = self.client.as_mut() {
            for source in &sources {
                client.load_light_shape(source.shape);
            }
        }
        let client = self.client.as_ref();
        self.lights.draw(
            painter,
            rect,
            light,
            rules.alternative,
            &sources,
            self.zoom,
            |shape| client.and_then(|client| client.light_shape(shape)),
        );
    }

    /// One mobile: his aura, his shadow and his real picture, in the hue
    /// the Options screen puts over him. The Modern style adds a ring on the
    /// ground and an outline in his color. When the client files hold no
    /// picture of him, a plain figure stands in. Gives the area he covers.
    fn figure(
        &mut self,
        canvas: &mut Canvas,
        map: u8,
        looks: Looks<'_>,
        foot: Pos2,
        target: bool,
    ) -> Rect {
        let Looks {
            look,
            pose,
            color,
            alpha,
            hue,
            aura,
            shadow,
            seat,
        } = looks;
        let zoom = self.zoom;
        if let Some(aura) = aura {
            let middle = foot - Vec2::new(0.0, AURA_LIFT * zoom);
            canvas.glow(middle, AURA_RADIUS * zoom, self.words_color(aura));
        }
        if !self.look.classic() {
            let ring = Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * zoom;
            if target {
                canvas.ring(
                    foot,
                    ring * TARGET_RING_GROW,
                    PAWN_RING_WIDTH * zoom,
                    theme::with_alpha(theme::ALARM, alpha),
                );
            }
            canvas.fill(
                &ellipse(foot, ring),
                theme::with_alpha(color, alpha * PAWN_RING_FILL_ALPHA),
            );
            canvas.ring(
                foot,
                ring,
                PAWN_RING_WIDTH * zoom,
                theme::with_alpha(color, alpha),
            );
        }
        let seated_look = seat.map(|seat| WatchLook {
            direction: seat.facing,
            ..look.clone()
        });
        let look = seated_look.as_ref().unwrap_or(look);
        let paint = Paint {
            outline: self.outline(color),
            whole_hue: hue,
        };
        let sprite = self
            .client
            .as_ref()
            .zip(self.atlas.as_mut())
            .and_then(|(client, atlas)| {
                let pose = match seat {
                    Some(seat) if seat.from_back => Pose {
                        action: Action::Shown(SIT_FROM_BACK_GROUP),
                        tick: 0,
                    },
                    Some(_) => STANDING,
                    None => Pose {
                        action: client.stance_action(look, pose.action),
                        ..pose
                    },
                };
                client.figure_sprite(atlas, map, look, pose, paint)
            });
        let Some(sprite) = sprite else {
            self.plain_figure(canvas, foot, color, alpha);
            return self.figure_rect(foot);
        };
        let foot = foot + seat.map_or(Vec2::ZERO, |seat| seat.offset * zoom);
        let area = Rect::from_min_size(
            foot - sprite.anchor * zoom,
            Vec2::new(sprite.width, sprite.height) * zoom,
        );
        if shadow {
            canvas.shadow(sprite, area, zoom, alpha);
        }
        let tint = theme::with_alpha(Color32::WHITE, alpha);
        match seat {
            Some(seat) if !seat.from_back => {
                let mirrored = Facing::from_direction(seat.facing).mirrored;
                canvas.sitting(sprite, area, tint, mirrored, zoom);
            }
            _ => canvas.sprite(sprite, area, tint),
        }
        area
    }

    /// A body and a head in one color, for a mobile with no real picture.
    fn plain_figure(&self, canvas: &mut Canvas, foot: Pos2, color: Color32, alpha: f32) {
        let zoom = self.zoom;
        let body = |grow: f32| {
            [
                foot + Vec2::new(-PAWN_FOOT_HALF - grow, PAWN_FOOT_Y + grow) * zoom,
                foot + Vec2::new(PAWN_FOOT_HALF + grow, PAWN_FOOT_Y + grow) * zoom,
                foot + Vec2::new(PAWN_SHOULDER_HALF + grow, PAWN_SHOULDER_Y - grow) * zoom,
                foot + Vec2::new(-PAWN_SHOULDER_HALF - grow, PAWN_SHOULDER_Y - grow) * zoom,
            ]
        };
        let head = |grow: f32| {
            ellipse(
                foot + Vec2::new(0.0, PAWN_HEAD_Y) * zoom,
                Vec2::splat((PAWN_HEAD_RADIUS + grow) * zoom),
            )
        };
        let outline = theme::with_alpha(PAWN_OUTLINE_COLOR, alpha);
        canvas.fill(&body(PAWN_OUTLINE), outline);
        canvas.fill(&head(PAWN_OUTLINE), outline);
        canvas.fill(&body(0.0), theme::with_alpha(color, alpha));
        canvas.fill(&head(0.0), theme::with_alpha(color, alpha));
    }

    /// The screen area of one figure, from the top of its head to its feet.
    fn figure_rect(&self, foot: Pos2) -> Rect {
        Rect::from_min_max(
            foot + Vec2::new(-PAWN_RING_RX, PAWN_TOP_Y) * self.zoom,
            foot + Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * self.zoom,
        )
    }

    fn focus_ring(&self, painter: &Painter, foot: Pos2) {
        let radius = Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * FOCUS_RING_GROW * self.zoom;
        painter.add(Shape::closed_line(
            ellipse(foot, radius),
            Stroke::new(PAWN_RING_WIDTH * self.zoom, theme::SELF_FIGURE),
        ));
    }

    /// The picture of an item, for a window that is not the map.
    pub fn item_picture(
        &mut self,
        map: u8,
        graphic: u16,
        hue: u16,
    ) -> Option<(egui::TextureId, Sprite)> {
        self.painted_item_picture(
            map,
            graphic,
            ItemPaint {
                hue,
                ..ItemPaint::default()
            },
        )
    }

    /// The picture of an item in a hue that covers every pixel, as a gump
    /// marks the item under the mouse.
    pub fn item_picture_whole_hue(
        &mut self,
        map: u8,
        graphic: u16,
        hue: u16,
    ) -> Option<(egui::TextureId, Sprite)> {
        self.painted_item_picture(
            map,
            graphic,
            ItemPaint {
                hue,
                whole_hue: true,
                ..ItemPaint::default()
            },
        )
    }

    fn painted_item_picture(
        &mut self,
        map: u8,
        graphic: u16,
        paint: ItemPaint,
    ) -> Option<(egui::TextureId, Sprite)> {
        let sprite = self.item_sprite(map, graphic, paint, true)?;
        Some((self.atlas.as_ref()?.texture_id(), sprite))
    }

    /// The tiledata record of an item graphic, when the files have one.
    pub fn item_tile(&self, graphic: u16) -> Option<&uoterm_nav::ItemTile> {
        self.client.as_ref()?.item_tile(graphic)
    }

    /// The point over a mobile or a thing that is drawn now, where the
    /// words it says float.
    pub fn head_of(&self, rect: Rect, frame: &WatchFrame, serial: u32) -> Option<Pos2> {
        if serial == frame.serial {
            let foot = self.project(rect, self.camera);
            return Some(self.figure_rect(foot).center_top());
        }
        self.picks
            .iter()
            .find(|pick| pick.serial == serial)
            .map(|pick| pick.area.center_top())
    }

    /// Words in a UO font, as `look` says, ready to draw. With no UO fonts
    /// in the client files they come in the window's own font.
    pub fn words(&mut self, painter: &Painter, text: &str, look: TextLook) -> Words {
        let picture = self
            .client
            .as_ref()
            .zip(self.atlas.as_mut())
            .and_then(|(client, atlas)| {
                let sprite = client.text_sprite(atlas, text, look)?;
                Some((atlas.texture_id(), sprite))
            });
        match picture {
            Some((texture, sprite)) => Words::Picture(texture, sprite),
            None => {
                let color = self.words_color(look.hue);
                let font = theme::title_font(theme::SIZE_PLATE);
                let galley = match look.width {
                    Some(wrap) => painter.layout(text.to_string(), font, color, wrap as f32),
                    None => painter.layout_no_wrap(text.to_string(), font, color),
                };
                Words::Font(galley, color)
            }
        }
    }

    /// How many lines words take in a UO font, as `look` breaks them. One
    /// when the client files hold no UO fonts.
    pub fn text_lines(&mut self, text: &str, look: TextLook) -> usize {
        let lines = self
            .client
            .as_ref()
            .zip(self.atlas.as_mut())
            .and_then(|(client, atlas)| {
                let sprite = client.text_sprite(atlas, text, look)?;
                let line = client.line_height(&look)?.max(1) as f32;
                let drawn = sprite.height - UNICODE_PICTURE_PADDING as f32;
                Some((drawn / line).round() as usize)
            });
        lines.unwrap_or(1).max(1)
    }

    /// The picture of a mouse pointer of the classic client, with its point
    /// as the anchor. None without client files.
    pub fn cursor_picture(
        &mut self,
        shape: CursorShape,
        war: bool,
        hue: u16,
    ) -> Option<(egui::TextureId, Sprite)> {
        let atlas = self.atlas.as_mut()?;
        let sprite = self
            .client
            .as_ref()?
            .cursor_sprite(atlas, shape, war, hue)?;
        Some((atlas.texture_id(), sprite))
    }

    /// The hit points and the name plates of the Classic style, over each
    /// thing that has them.
    fn draw_overheads(&mut self, painter: &Painter, mut plates: Vec<Plate>) {
        // From the bottom of the window up, so a plate that must make room
        // moves up and away from the ones under it.
        plates.sort_by(|a, b| b.top.y.total_cmp(&a.top.y));
        let mut taken = Vec::new();
        for plate in &plates {
            if plate.hits.line {
                self.hits_line(painter, plate);
            }
            let mut top = plate.top - Vec2::new(0.0, HITS_RISE * self.zoom);
            if let (true, Some(percent)) = (plate.hits.percent, plate.hits_percent) {
                let text = format!("[{percent}%]");
                let words = self.words(
                    painter,
                    &text,
                    TextLook::ascii(HITS_FONT, look::hits_hue(percent)),
                );
                let size = words.size();
                top.y -= size.y;
                words.paint(painter, top - Vec2::new(size.x / 2.0, 0.0), 1.0);
            }
            if plate.named {
                self.nameplate(painter, plate, top, &mut taken);
            }
        }
    }

    /// The name of a thing on a dark plate, as the Nameplates page says: its
    /// opacity, a line of its hit points, and plates that keep apart.
    fn nameplate(&mut self, painter: &Painter, plate: &Plate, top: Pos2, taken: &mut Vec<Rect>) {
        let options = self.look.nameplates.clone();
        let look = TextLook::unicode(NAMEPLATE_FONT, plate.hue).bordered();
        let words = self.words(painter, &plate.name, look);
        let bar = plate
            .hits_percent
            .filter(|_| options.health_bar && plate.of == PlateOf::Mobile);
        let bar_room = if bar.is_some() {
            theme::PIP_HEIGHT + NAMEPLATE_PAD
        } else {
            0.0
        };
        let size = words.size() + Vec2::new(NAMEPLATE_PAD * 2.0, NAMEPLATE_PAD * 2.0 + bar_room);
        let mut area = Rect::from_min_size(top - Vec2::new(size.x / 2.0, size.y), size);
        if options.avoid_overlap {
            while let Some(hit) = taken.iter().find(|r| r.intersects(area)) {
                area = area.translate(Vec2::new(0.0, hit.top() - area.bottom() - 1.0));
            }
            taken.push(area);
        }
        let opacity = f32::from(options.opacity) / PERCENT;
        painter.rect_filled(
            area,
            0.0,
            Color32::from_black_alpha((opacity * f32::from(u8::MAX)) as u8),
        );
        words.paint(painter, area.min + Vec2::splat(NAMEPLATE_PAD), 1.0);
        if let Some(percent) = bar {
            let track = Rect::from_min_max(
                Pos2::new(
                    area.left() + NAMEPLATE_PAD,
                    area.bottom() - NAMEPLATE_PAD - theme::PIP_HEIGHT,
                ),
                Pos2::new(area.right() - NAMEPLATE_PAD, area.bottom() - NAMEPLATE_PAD),
            );
            painter.rect_filled(track, 0.0, self.words_color(HITS_LOST_HUE));
            let mut fill = track;
            fill.set_width(track.width() * f32::from(percent.min(PERCENT_MAX)) / PERCENT);
            painter.rect_filled(fill, 0.0, self.words_color(plate.hue));
        }
    }

    /// The line of hit points under the feet, from the gump art of the
    /// classic client: the notoriety hue behind, the lost part in red, and
    /// the rest in blue, green for poison or yellow for the blessed.
    fn hits_line(&mut self, painter: &Painter, plate: &Plate) {
        let Some(percent) = plate.hits_percent else {
            return;
        };
        let area = Rect::from_center_size(
            plate.foot + Vec2::new(0.0, HITS_LINE_DROP * self.zoom),
            HITS_LINE_SIZE,
        );
        let alpha = if plate.target {
            1.0
        } else {
            HITS_PASSIVE_ALPHA
        };
        let left = HITS_LINE_SIZE.x * f32::from(percent.min(PERCENT_MAX)) / PERCENT;
        let fill_hue = if plate.poisoned {
            HITS_POISON_HUE
        } else if plate.yellow_hits {
            HITS_YELLOW_HUE
        } else {
            HITS_FILL_HUE
        };
        let parts = [
            (HITS_BACK_GUMP, plate.hue, area),
            (
                HITS_FILL_GUMP,
                HITS_LOST_HUE,
                Rect::from_min_max(Pos2::new(area.left() + left, area.top()), area.max),
            ),
            (
                HITS_FILL_GUMP,
                fill_hue,
                Rect::from_min_size(area.min, Vec2::new(left, area.height())),
            ),
        ];
        for (gump, hue, part) in parts {
            if part.width() <= 0.0 {
                continue;
            }
            match self.gump_picture(gump, hue) {
                Some((texture, sprite)) => {
                    painter.image(
                        texture,
                        part,
                        sprite.uv,
                        Color32::WHITE.gamma_multiply(alpha),
                    );
                }
                None => {
                    painter.rect_filled(part, 0.0, self.words_color(hue).gamma_multiply(alpha));
                }
            }
        }
    }

    /// The color of words in a hue. Without client files, the plain color.
    pub fn words_color(&self, hue: u16) -> Color32 {
        self.client
            .as_ref()
            .and_then(|client| client.text_rgb(hue))
            .map_or(theme::TEXT, |[r, g, b]| Color32::from_rgb(r, g, b))
    }

    /// The thing on top under the mouse.
    pub fn thing_at(&self, mouse: Pos2) -> Option<&Pick> {
        self.picks
            .iter()
            .rev()
            .find(|pick| pick.area.contains(mouse))
    }

    /// The tile under the mouse, and the height of its floor. The ground is
    /// not flat, so the tile is the nearest one whose own floor lies under
    /// the mouse. With no client files the floor of the character serves.
    pub fn tile_at(&mut self, rect: Rect, frame: &WatchFrame, mouse: Pos2) -> (u16, u16, i8) {
        let on_plane = (mouse - self.view_center(rect)) / (HALF_TILE * self.zoom);
        let camera = self.camera;
        let tile_on_row = |rows_down: f32| {
            let sum = on_plane.y + rows_down;
            let x = camera[0] + (on_plane.x + sum) / 2.0;
            let y = camera[1] + (sum - on_plane.x) / 2.0;
            (x.round().max(0.0) as u16, y.round().max(0.0) as u16)
        };
        let level = (tile_on_row(0.0), frame.z);
        let mut best = level;
        for step in PICK_ROWS_UP..=PICK_ROWS_DOWN {
            let rows_down = step as f32 * PICK_ROW_STEP;
            let (x, y) = tile_on_row(rows_down);
            let Some(floor) = self.floor_near(frame, x, y) else {
                continue;
            };
            let lift_rows = (f32::from(floor) - camera[2]) * Z_PIXELS / HALF_TILE;
            if (lift_rows - rows_down).abs() <= PICK_ROW_STEP {
                best = ((x, y), floor);
            }
        }
        (best.0 .0, best.0 .1, best.1)
    }

    /// The floor of a tile that is nearest to the height of the character.
    fn floor_near(&mut self, frame: &WatchFrame, x: u16, y: u16) -> Option<i8> {
        let cell = self.client.as_mut()?.cell(frame.map, x, y)?;
        let land = cell.land_id.map(|_| i16::from(cell.corners[0]));
        let here = i16::from(frame.z);
        cell.statics
            .iter()
            .filter_map(|s| s.floor)
            .chain(land)
            .min_by_key(|floor| (floor - here).abs())
            .map(|floor| floor.clamp(i16::from(i8::MIN), i16::from(i8::MAX)) as i8)
    }

    /// A wedge on the ground that points the way the character faces.
    fn facing_mark(&self, canvas: &mut Canvas, foot: Pos2, facing: &str) {
        let Some((dx, dy)) = facing_step(facing) else {
            return;
        };
        let length = (dx * dx + dy * dy).sqrt();
        let (dx, dy) = (dx / length, dy / length);
        let on_ground = |along: f32, across: f32| {
            let (x, y) = (dx * along - dy * across, dy * along + dx * across);
            foot + Vec2::new((x - y) * HALF_TILE, (x + y) * HALF_TILE) * self.zoom
        };
        canvas.fill(
            &[
                on_ground(FACING_FAR, 0.0),
                on_ground(FACING_NEAR, FACING_HALF_WIDTH),
                on_ground(FACING_NEAR, -FACING_HALF_WIDTH),
            ],
            theme::SELF_FIGURE,
        );
    }

    /// A dashed line on the ground to the walk goal, and a beacon on it.
    /// When the goal is outside the window, a mark on the edge points to it.
    fn draw_walk_goal(&self, painter: &Painter, rect: Rect, frame: &WatchFrame) {
        let (Some(x), Some(y)) = (frame.dest_x, frame.dest_y) else {
            return;
        };
        let from = self.project(rect, self.camera);
        let goal = self.project(rect, [f32::from(x), f32::from(y), self.camera[2]]);
        let inside = rect.shrink(EDGE_INSET);
        let end = if inside.contains(goal) {
            goal
        } else {
            clamp_to(inside, from, goal)
        };
        let stroke = Stroke::new(PATH_WIDTH, theme::GOAL);
        painter.extend(Shape::dashed_line(
            &[from, end],
            stroke,
            PATH_DASH,
            PATH_GAP,
        ));
        if !inside.contains(goal) {
            painter.circle_filled(end, EDGE_MARK_RADIUS, theme::GOAL);
            return;
        }
        let half = HALF_TILE * self.zoom * BEACON_RING;
        let diamond = vec![
            goal + Vec2::new(0.0, -half),
            goal + Vec2::new(half, 0.0),
            goal + Vec2::new(0.0, half),
            goal + Vec2::new(-half, 0.0),
        ];
        painter.add(Shape::closed_line(diamond, stroke));
        painter.line_segment(
            [goal, goal - Vec2::new(0.0, BEACON_HEIGHT * self.zoom)],
            stroke,
        );
    }
}

/// The point where the line from `from` to `to` leaves `rect`.
fn clamp_to(rect: Rect, from: Pos2, to: Pos2) -> Pos2 {
    let delta = to - from;
    let reach = |delta: f32, low: f32, high: f32| {
        if delta > 0.0 {
            high / delta
        } else if delta < 0.0 {
            low / delta
        } else {
            f32::INFINITY
        }
    };
    let t = reach(delta.x, rect.left() - from.x, rect.right() - from.x)
        .min(reach(delta.y, rect.top() - from.y, rect.bottom() - from.y))
        .clamp(0.0, 1.0);
    from + delta * t
}

fn radar_symbol(frame: &WatchFrame, column: i32, row: i32) -> Option<char> {
    let line = frame.radar.get(usize::try_from(row).ok()?)?;
    let symbol = line.chars().nth(usize::try_from(column).ok()?)?;
    Some(match symbol {
        SYM_BLOCK | SYM_DOOR | SYM_WATER => symbol,
        _ => SYM_WALK,
    })
}

/// Draws the names from the top of the window down. A name that would lie on
/// one already drawn, or on the character, moves up until it is clear.
fn draw_plates(painter: &Painter, mut plates: Vec<Plate>, keep_clear: Rect, zoom: f32) {
    plates.retain(|plate| plate.of == PlateOf::Mobile);
    plates.sort_by(|a, b| a.top.y.total_cmp(&b.top.y));
    let font = theme::title_font(theme::SIZE_PLATE);
    let mut taken = vec![keep_clear];
    for plate in plates {
        let galley = painter.layout_no_wrap(plate.name.clone(), font.clone(), plate.color);
        let bar_height = if plate.hits_percent.is_some() {
            theme::PIP_HEIGHT + PLATE_GAP
        } else {
            0.0
        };
        let size = Vec2::new(
            galley.size().x.max(PLATE_BAR_WIDTH),
            galley.size().y + bar_height,
        ) + Vec2::splat(PLATE_PAD * 2.0);
        let mut area = Rect::from_center_size(
            plate.top - Vec2::new(0.0, size.y / 2.0 + PLATE_GAP * zoom),
            size,
        );
        while let Some(hit) = taken.iter().find(|r| r.intersects(area)) {
            area = area.translate(Vec2::new(0.0, hit.top() - area.bottom() - 1.0));
        }
        taken.push(area);
        painter.rect_filled(area, theme::BAR_RADIUS, theme::PLATE_BACK);
        let name_at = Pos2::new(area.center().x, area.top() + PLATE_PAD);
        let color = if plate.target {
            theme::ALARM
        } else {
            plate.color
        };
        theme::shadowed_text(
            painter,
            name_at,
            Align2::CENTER_TOP,
            &plate.name,
            font.clone(),
            color,
        );
        if let Some(percent) = plate.hits_percent {
            let track = Rect::from_min_size(
                Pos2::new(
                    area.center().x - PLATE_BAR_WIDTH / 2.0,
                    area.bottom() - PLATE_PAD - theme::PIP_HEIGHT,
                ),
                Vec2::new(PLATE_BAR_WIDTH, theme::PIP_HEIGHT),
            );
            painter.rect_filled(track.expand(1.0), theme::BAR_RADIUS, theme::TEXT_SHADOW);
            let mut fill = track;
            fill.set_width(PLATE_BAR_WIDTH * f32::from(percent) / PERCENT);
            painter.rect_filled(fill, theme::BAR_RADIUS, plate.color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));
    const MIDDLE: Pos2 = Pos2::new(50.0, 50.0);

    #[test]
    fn the_quest_arrow_is_held_at_the_edge_when_its_place_is_out_of_view() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0));
        // A place far to the right is held at the right edge.
        let (point, away) = arrow_at(rect, Pos2::new(5000.0, 400.0));
        assert_eq!(point.x, rect.right() - ARROW_EDGE);
        assert!(away.x > 0.9, "it points to the right");
        // A place in view keeps its own spot.
        let inside = Pos2::new(700.0, 500.0);
        assert_eq!(arrow_at(rect, inside).0, inside);
        // A place under the character points up, not nowhere.
        let (_, away) = arrow_at(rect, rect.center());
        assert_eq!(away, Vec2::new(0.0, -1.0));
    }

    #[test]
    fn a_piece_of_a_house_is_drawn_in_height_order_with_the_rest() {
        let floor = Standing::Piece {
            graphic: 0x0500,
            z: 7.0,
            faded: false,
        };
        let character = Standing::Character {
            at: [0.0, 0.0, 12.0],
        };
        assert!(floor.z() < character.z());
        let without_files = Scene::new(None);
        let frame = WatchFrame {
            multis: vec![crate::view::WatchMulti {
                serial: 50,
                multi_id: 100,
                x: 10,
                y: 10,
                z: 0,
            }],
            ..WatchFrame::default()
        };
        let standing = without_files.standing(&frame);
        assert_eq!(standing.len(), 1, "only the character stands");
    }

    /// While the player designs, a hidden storey leaves its pieces out and a
    /// see-through one fades them; other houses show as they are.
    #[test]
    fn the_storeys_of_the_house_being_designed_show_as_the_designer_asks() {
        use crate::view::{WatchDesigning, WatchMulti};
        use uoterm_nav::{HousePart, HousePartKind};
        const WALL: u16 = 0x0064;
        let mut scene = Scene::new(None);
        let house = |serial| uoterm_world::DesignedHouse {
            serial: uoterm_protocol::types::Serial(serial),
            revision: 1,
            tiles: vec![uoterm_world::HouseTile {
                graphic: WALL,
                dx: 1,
                dy: 0,
                dz: 7,
            }],
        };
        let multi = |serial, x| WatchMulti {
            serial,
            multi_id: 100,
            x,
            y: 10,
            z: 0,
        };
        let frame = WatchFrame {
            multis: vec![multi(50, 10), multi(51, 30)],
            designed_houses: vec![house(50), house(51)],
            designing: Some(WatchDesigning {
                serial: 50,
                floor: 1,
                plot: None,
            }),
            house_parts: vec![HousePart {
                kind: HousePartKind::Wall,
                name: "Stone".into(),
                pieces: vec![WALL],
            }],
            ..WatchFrame::default()
        };
        let faded_at = |scene: &Scene, x| {
            scene.standing(&frame).get(&(x, 10)).and_then(|things| {
                things.iter().find_map(|thing| match thing {
                    Standing::Piece { faded, .. } => Some(*faded),
                    _ => None,
                })
            })
        };
        assert_eq!(faded_at(&scene, 11), Some(false));
        let mut looks = [StoreyLook::Normal; STOREYS];
        looks[0] = StoreyLook::SeeThroughContent;
        scene.set_storey_looks(looks);
        assert_eq!(faded_at(&scene, 11), Some(true));
        looks[0] = StoreyLook::HideAll;
        scene.set_storey_looks(looks);
        assert_eq!(faded_at(&scene, 11), None, "hidden");
        assert_eq!(faded_at(&scene, 31), Some(false), "another house");
    }

    #[test]
    fn a_new_animation_cue_plays_once_and_then_ends() {
        const ORC: u32 = 9;
        const SWING: u16 = 9;
        let mut scene = Scene::new(None);
        let mut frame = WatchFrame::default();
        scene.take_cues(&frame, 0.0);
        frame.cues.push(crate::view::WatchCue {
            seq: 1,
            serial: ORC,
            kind: WatchCueKind::Animation(SWING),
        });
        scene.take_cues(&frame, 1.0);
        scene.now = 1.25;
        let pose = scene.shown_pose(ORC).unwrap();
        assert_eq!((pose.action, pose.tick), (Action::Shown(9), 2));
        scene.take_cues(&frame, 1.0 + SHOW_SECONDS * 2.0);
        assert!(scene.shown_pose(ORC).is_none());
    }

    #[test]
    fn a_goal_outside_the_window_is_marked_on_the_edge() {
        let far_right = Pos2::new(250.0, 50.0);
        assert_eq!(clamp_to(WINDOW, MIDDLE, far_right), Pos2::new(100.0, 50.0));
        let far_up_left = Pos2::new(-50.0, -150.0);
        assert_eq!(clamp_to(WINDOW, MIDDLE, far_up_left), Pos2::new(25.0, 0.0));
    }

    #[test]
    fn north_points_up_and_to_the_right_on_the_screen() {
        let (dx, dy) = facing_step("north").unwrap();
        assert!(dx - dy > 0.0 && dx + dy < 0.0);
        assert!(facing_step("up").is_none());
    }

    #[test]
    fn radar_cells_outside_the_radar_draw_nothing() {
        let frame = WatchFrame {
            radar: vec!["#.".into(), "m~".into()],
            ..WatchFrame::default()
        };
        assert_eq!(radar_symbol(&frame, 0, 0), Some(SYM_BLOCK));
        assert_eq!(radar_symbol(&frame, 0, 1), Some(SYM_WALK));
        assert_eq!(radar_symbol(&frame, 1, 1), Some(SYM_WATER));
        assert_eq!(radar_symbol(&frame, 2, 0), None);
        assert_eq!(radar_symbol(&frame, -1, 0), None);
    }

    #[test]
    fn the_range_circle_bounds_the_tiles_in_range() {
        const TILES: u8 = 3;
        let mut scene = Scene::new(None);
        let frame = WatchFrame {
            x: 100,
            y: 100,
            z: 5,
            ..WatchFrame::default()
        };
        scene.follow(&frame, 0.0);
        let [top, right, bottom, left] = scene.range_corners(WINDOW, TILES);
        let reach = (f32::from(TILES) + HALF_TILE_STEP) * HALF_TILE * 2.0;
        assert_eq!(top, MIDDLE - Vec2::new(0.0, reach));
        assert_eq!(bottom, MIDDLE + Vec2::new(0.0, reach));
        assert_eq!(right, MIDDLE + Vec2::new(reach, 0.0));
        assert_eq!(left, MIDDLE - Vec2::new(reach, 0.0));
    }

    #[test]
    fn the_tile_under_the_mouse_is_the_tile_that_is_drawn_there() {
        let mut scene = Scene::new(None);
        let frame = WatchFrame {
            x: 100,
            y: 100,
            z: 5,
            ..WatchFrame::default()
        };
        scene.follow(&frame, 0.0);
        assert_eq!(scene.tile_at(WINDOW, &frame, MIDDLE), (100, 100, 5));
        let one_east_one_north = MIDDLE + Vec2::new(HALF_TILE * 2.0, 0.0);
        assert_eq!(
            scene.tile_at(WINDOW, &frame, one_east_one_north),
            (101, 99, 5)
        );
        let drawn_at = scene.project(WINDOW, [97.0, 104.0, 5.0]);
        assert_eq!(scene.tile_at(WINDOW, &frame, drawn_at), (97, 104, 5));
    }

    #[test]
    fn real_files_pick_the_floor_of_a_building_and_not_the_land_under_it() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        const BANK_FLOOR: i8 = 20;
        let mut scene = Scene::new(Some(&dir));
        let frame = WatchFrame {
            x: 3484,
            y: 2570,
            z: BANK_FLOOR,
            map: 1,
            ..WatchFrame::default()
        };
        scene.follow(&frame, 0.0);
        let drawn_at = scene.project(WINDOW, [3486.0, 2572.0, f32::from(BANK_FLOOR)]);
        assert_eq!(
            scene.tile_at(WINDOW, &frame, drawn_at),
            (3486, 2572, BANK_FLOOR)
        );
    }

    const ON_FOOT: bool = false;
    const WALKS: bool = false;
    const RUNS: bool = true;
    const CLOSE: f32 = 0.001;

    fn east(tiles: f32) -> [f32; 3] {
        [tiles, 0.0, 0.0]
    }

    #[test]
    fn a_mobile_walks_or_runs_by_what_the_shard_says_and_then_stands() {
        let mut walker = Glide::resting(east(0.0), 0.0);
        assert_eq!(walker.pose(0.0).action, Action::Stand);
        walker.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        assert_eq!(walker.pose(0.1).action, Action::Walk);
        // Five pictures for one tile on foot, spread over the move.
        let end_of_move = walker.seconds;
        assert_eq!(walker.pose(end_of_move / 2.0).tick, 2);
        assert_eq!(walker.pose(end_of_move).tick, 5);
        let end = walker.seconds;
        assert_eq!(
            walker.pose(end + STEP_LINGER_SECONDS / 2.0).action,
            Action::Walk
        );
        assert_eq!(walker.pose(end + 2.0).action, Action::Stand);
        let mut runner = Glide::resting(east(0.0), 0.0);
        runner.aim(east(1.0), 0.0, ON_FOOT, RUNS);
        assert_eq!(runner.pose(0.05).action, Action::Run);
        assert!(runner.seconds < walker.seconds);
    }

    #[test]
    fn tiles_that_come_early_wait_and_the_move_does_not_stop_between_them() {
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        let first_leg = glide.seconds;
        assert!((first_leg - STEP_SECONDS_FOOT_WALK * GLIDE_STRETCH_ALONE).abs() < 1e-9);
        // The news of the next tile comes at an uneven time. It waits.
        glide.aim(east(2.0), 0.33, ON_FOOT, WALKS);
        assert_eq!(glide.queued, 1);
        assert!(glide.at(0.33)[0] < 1.0);
        // The next move starts at the moment the first one ended.
        glide.aim(east(2.0), first_leg + 0.05, ON_FOOT, WALKS);
        assert_eq!((glide.queued, glide.started), (0, first_leg));
        let speed = 1.0 / glide.seconds as f32;
        assert!((glide.at(first_leg + 0.05)[0] - (1.0 + 0.05 * speed)).abs() < CLOSE);
        assert!(glide.moving(first_leg + 0.05));
    }

    #[test]
    fn news_that_comes_late_each_time_makes_the_move_slower_so_it_does_not_stop() {
        const LATE: f64 = STEP_SECONDS_FOOT_WALK * 1.2;
        let mut glide = Glide::resting(east(0.0), 0.0);
        for tile in 1..=8 {
            glide.aim(east(tile as f32), LATE * f64::from(tile), ON_FOOT, WALKS);
        }
        assert!(glide.news_seconds > STEP_SECONDS_FOOT_WALK * 1.15);
        assert!(glide.seconds >= LATE, "the move lasts until the next news");
        // A rest does not count as a slow rhythm.
        glide.aim(east(9.0), 100.0, ON_FOOT, WALKS);
        assert_eq!(glide.news_seconds, STEP_SECONDS_FOOT_WALK);
    }

    #[test]
    fn uneven_news_gives_a_move_that_never_stops_between_tiles() {
        const FRAME: f64 = 1.0 / 60.0;
        // The session spaces its steps like a person: some early, some late.
        let gaps = [0.46, 0.35, 0.44, 0.37, 0.45, 0.34, 0.46, 0.40, 0.43, 0.36];
        let mut glide = Glide::resting(east(0.0), 0.0);
        let (mut news_at, mut tile, mut gap) = (0.0, 0.0, 0);
        let (mut time, mut before, mut stalls) = (0.0, 0.0, 0);
        while tile < 40.0 {
            if time >= news_at {
                tile += 1.0;
                news_at += gaps[gap % gaps.len()];
                gap += 1;
            }
            glide.aim(east(tile), time, ON_FOOT, WALKS);
            let at = glide.at(time)[0];
            // The first tiles teach the rhythm.
            if tile > 6.0 && at <= before {
                stalls += 1;
            }
            before = at;
            time += FRAME;
        }
        assert_eq!(stalls, 0);
        assert!(tile - before < 2.0, "the move stays near the news");
    }

    #[test]
    fn the_legs_go_by_the_ground_and_stop_when_the_mobile_stops() {
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, RUNS);
        let end = glide.seconds;
        let at_end = glide.pose(end).tick;
        // After the move the legs keep their picture, then he stands.
        assert_eq!(glide.pose(end + STEP_LINGER_SECONDS / 2.0).tick, at_end);
        assert_eq!(glide.pose(end + 1.0).action, Action::Stand);
        // A slow move has the same pictures for each tile as a fast one.
        let mut slow = Glide::resting(east(0.0), 0.0);
        slow.aim(east(1.0), 0.0, ON_FOOT, RUNS);
        slow.seconds *= 3.0;
        assert_eq!(slow.pose(slow.seconds).tick, at_end);
        // The next tile carries the legs on from where they were.
        glide.aim(east(2.0), end, ON_FOOT, RUNS);
        assert!(glide.pose(end + glide.seconds).tick > at_end);
    }

    #[test]
    fn a_mobile_faces_the_way_he_moves_and_not_the_way_of_his_next_step() {
        const NORTH: u8 = 0;
        const EAST: u8 = 2;
        const SOUTH_WEST: u8 = 5;
        let mut glide = Glide::resting(east(0.0), 0.0);
        assert_eq!(glide.heading(0.0), None);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        assert_eq!(glide.heading(0.1), Some(EAST));
        // The shard turned him north for his next step. He still goes east.
        let shard_look = WatchLook {
            direction: NORTH,
            ..WatchLook::default()
        };
        let drawn = turned_to(&shard_look, glide.heading(0.1)).unwrap();
        assert_eq!(drawn.direction, EAST);
        assert!(turned_to(&shard_look, Some(NORTH)).is_none());
        assert_eq!(glide.heading(glide.seconds + 1.0), None);
        let mut back = Glide::resting([5.0, 5.0, 0.0], 0.0);
        back.aim([4.0, 6.0, 0.0], 0.0, ON_FOOT, WALKS);
        assert_eq!(back.heading(0.1), Some(SOUTH_WEST));
    }

    #[test]
    fn a_mobile_that_is_behind_catches_up() {
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        let alone = glide.seconds;
        for tile in 2..=8 {
            glide.aim(east(tile as f32), 0.01, ON_FOOT, WALKS);
        }
        assert_eq!(glide.queued, GLIDE_QUEUE_CAP);
        assert_eq!(
            glide.goal(),
            east(8.0),
            "a full queue keeps the newest tile"
        );
        glide.aim(east(8.0), alone, ON_FOOT, WALKS);
        assert!(glide.seconds < STEP_SECONDS_FOOT_WALK);
    }

    /// How often the window reads the session in these tests.
    const POLL: f64 = 0.033;
    /// Each read reaches the window this much after the session made it, in
    /// turn: the call, and the wait for the next frame of the window.
    const READ_DELAYS: [f64; 5] = [0.004, 0.019, 0.001, 0.027, 0.012];
    const FRAME: f64 = 1.0 / 60.0;
    /// The pause a turn puts between two steps, as the session times it.
    const TURN: f64 = 0.1;
    /// How far off the pace a frame of a steady walk may move.
    const PACE_TOLERANCE: f64 = 0.01;

    /// One step the session sent east: its slot and how long it lasts.
    type Sent = (f64, f64);

    /// Steps sent one after the other from `first`, each lasting `lasts`.
    fn sent_steps(first: f64, lasts: f64, count: usize) -> Vec<Sent> {
        (0..count)
            .map(|step| (first + lasts * step as f64, lasts))
            .collect()
    }

    /// The character a human steers, drawn at 60 frames a second while the
    /// window reads the session each poll, as the scene takes the reads:
    /// the tile the sent steps lead to, and the newest step once. Gives the
    /// place drawn in each frame, and how long the step under way lasts.
    fn drawn_steps(steps: &[Sent]) -> Vec<(f32, f64)> {
        let lag = stride_lag(Duration::from_secs_f64(POLL));
        let last_ends = steps.last().map_or(0.0, |(slot, lasts)| slot + lasts);
        let mut glide = Glide::resting(east(0.0), 0.0);
        let mut taken = None;
        let (mut poll, mut read, mut shown) = (0.0, 0, (0usize, None::<Sent>));
        let mut drawn = Vec::new();
        let mut time = 0.0;
        while time < last_ends + lag + 1.0 {
            let delay = READ_DELAYS[read % READ_DELAYS.len()];
            if poll + delay <= time {
                let sent = steps.iter().filter(|(slot, _)| *slot <= poll).count();
                shown = (sent, sent.checked_sub(1).map(|newest| steps[newest]));
                poll += POLL;
                read += 1;
            }
            let goal = east(shown.0 as f32);
            match shown
                .1
                .filter(|step| goal != glide.goal() && taken != Some(step.0))
            {
                Some((slot, lasts)) => {
                    taken = Some(slot);
                    let stride = Stride {
                        starts: slot + lag,
                        tile_seconds: lasts,
                    };
                    glide.aim_stride(goal, time, ON_FOOT, WALKS, stride);
                }
                None => glide.aim(goal, time, ON_FOOT, WALKS),
            }
            let lasts = steps
                .iter()
                .rev()
                .find(|(slot, _)| *slot + lag <= time)
                .map_or(STEP_SECONDS_FOOT_WALK, |(_, lasts)| *lasts);
            drawn.push((glide.at(time)[0], lasts));
            time += FRAME;
        }
        drawn
    }

    /// The pace of each frame from the first move to the last: 1 is the
    /// pace the session gave the step, 0 is a stop.
    fn paces(drawn: &[(f32, f64)]) -> Vec<f64> {
        let moves: Vec<f64> = drawn
            .windows(2)
            .map(|pair| f64::from(pair[1].0 - pair[0].0) / FRAME * pair[1].1)
            .collect();
        let first = moves.iter().position(|pace| *pace > 0.0).unwrap_or(0);
        let last = moves.iter().rposition(|pace| *pace > 0.0).unwrap_or(0);
        // The first and the last frame hold only part of a move.
        moves[first + 1..last].to_vec()
    }

    fn is_on_pace(pace: f64) -> bool {
        (pace - 1.0).abs() <= PACE_TOLERANCE
    }

    /// The measured fault: the steps of the character went out on the
    /// session's tick and the window learned their rhythm like news, so his
    /// speed swung from 0.86 to 1.08 of the pace while he ran and stopped
    /// for a frame between steps. Sent steps now meet end to end: every
    /// frame moves him at the pace, however late each read comes.
    #[test]
    fn a_steered_character_moves_at_one_even_pace_over_his_sent_steps() {
        for (lasts, count) in [
            (STEP_SECONDS_FOOT_WALK, 12),
            (STEP_SECONDS_FOOT_RUN, 24),
            (STEP_SECONDS_MOUNT_RUN, 40),
        ] {
            let paces = paces(&drawn_steps(&sent_steps(0.5, lasts, count)));
            let off: Vec<&f64> = paces.iter().filter(|pace| !is_on_pace(**pace)).collect();
            assert!(off.is_empty(), "a step of {lasts} s: {off:?} of {paces:?}");
            let drawn = drawn_steps(&sent_steps(0.5, lasts, count));
            assert_eq!(drawn.last().map(|(at, _)| *at), Some(count as f32));
        }
    }

    /// A turn holds the next step back. The character stands for that
    /// long and no longer, and walks on at the pace.
    #[test]
    fn a_turn_between_two_sent_steps_is_one_short_stop() {
        let lasts = STEP_SECONDS_FOOT_RUN;
        let mut steps = sent_steps(0.5, lasts, 6);
        let after_turn = steps.last().map_or(0.0, |(slot, _)| slot + lasts + TURN);
        steps.extend(sent_steps(after_turn, lasts, 6));
        let paces = paces(&drawn_steps(&steps));
        let stops = paces.iter().filter(|pace| **pace == 0.0).count();
        let stop_frames = (TURN / FRAME).round() as usize;
        assert!(
            stops.abs_diff(stop_frames) <= 1,
            "{stops} frames stood still"
        );
        // Two frames hold part of a move and part of the stop.
        let off = paces
            .iter()
            .filter(|pace| **pace != 0.0 && !is_on_pace(**pace))
            .count();
        assert!(off <= 2, "{paces:?}");
    }

    /// The human lets the mouse go further and the walk becomes a run. The
    /// character walks at the walk and runs at the run, with no stop.
    #[test]
    fn a_walk_that_turns_into_a_run_keeps_each_pace_and_does_not_stop() {
        let mut steps = sent_steps(0.5, STEP_SECONDS_FOOT_WALK, 5);
        let run_from = steps.last().map_or(0.0, |(slot, lasts)| slot + lasts);
        steps.extend(sent_steps(run_from, STEP_SECONDS_FOOT_RUN, 10));
        let paces = paces(&drawn_steps(&steps));
        let off: Vec<&f64> = paces.iter().filter(|pace| !is_on_pace(**pace)).collect();
        // One frame holds the end of the walk and the start of the run.
        assert!(off.len() <= 1, "{off:?} of {paces:?}");
    }

    /// A read that comes later than the lag leaves the character nowhere
    /// ahead of it: he waits for it, then catches up a little on each step,
    /// and never jumps.
    #[test]
    fn a_late_read_is_caught_up_without_a_jump() {
        let lag = stride_lag(Duration::from_secs_f64(POLL));
        let lasts = STEP_SECONDS_FOOT_RUN;
        let mut glide = Glide::resting(east(0.0), 0.0);
        let late = lag + FRAME * 3.0;
        glide.aim_stride(
            east(1.0),
            late,
            ON_FOOT,
            RUNS,
            Stride {
                starts: lag,
                tile_seconds: lasts,
            },
        );
        assert_eq!(glide.at(late)[0], 0.0, "no jump");
        assert!(glide.seconds < lasts, "and a quicker step to catch up");
        assert!(glide.seconds > lasts / (1.0 + GLIDE_CATCH_UP_PER_TILE));
    }

    #[test]
    fn a_step_is_a_steady_move_and_a_teleport_is_not() {
        let leg = STEP_SECONDS_FOOT_WALK * GLIDE_STRETCH_ALONE;
        let mut scene = Scene::new(None);
        let mut frame = WatchFrame {
            x: 100,
            y: 100,
            ..WatchFrame::default()
        };
        scene.follow(&frame, 0.0);
        frame.x += 1;
        assert!(scene.follow(&frame, 1.0));
        assert_eq!(scene.camera[0], 100.0);
        assert!(scene.follow(&frame, 1.0 + leg / 2.0));
        assert!((scene.camera[0] - 100.5).abs() < CLOSE);
        assert!(!scene.follow(&frame, 1.0 + leg * 2.0));
        assert_eq!(scene.camera[0], 101.0);
        frame.x = 900;
        assert!(!scene.follow(&frame, 3.0));
        assert_eq!(scene.camera[0], 900.0);
    }

    const ROOF: TileFlagSet = TileFlagSet::ROOF;

    fn over(z: i16, flags: TileFlagSet) -> Over {
        Over { z, flags }
    }

    #[test]
    fn open_sky_cuts_nothing_and_a_floor_overhead_cuts_the_world_there() {
        let open = ceiling_of(0, Some(0), &[], &[], true);
        assert_eq!(open, Ceiling::open(true));
        let upstairs = ceiling_of(0, Some(0), &[over(20, TileFlagSet::SURFACE)], &[], true);
        assert_eq!(upstairs.max_z, 20);
        assert!(upstairs.no_draw_roofs, "under a floor the roofs go too");
        // A low thing over the feet cuts nothing.
        let table = ceiling_of(0, Some(0), &[over(10, TileFlagSet::SURFACE)], &[], true);
        assert_eq!(table.max_z, NO_CEILING);
    }

    #[test]
    fn a_roof_in_front_cuts_the_world_and_roofs_may_always_hide() {
        let porch = ceiling_of(0, Some(0), &[], &[over(30, ROOF)], true);
        assert_eq!((porch.max_z, porch.no_draw_roofs), (30, true));
        let always = ceiling_of(0, Some(0), &[], &[], false);
        assert!(always.no_draw_roofs);
        // Under the ground of a cave, the world is cut at the head.
        let cave = ceiling_of(0, Some(40), &[], &[], true);
        assert_eq!((cave.max_z, cave.max_ground_z), (BODY_HEIGHT, BODY_HEIGHT));
    }

    #[test]
    fn flat_stretched_land_is_lit_as_the_classic_client_lights_it() {
        let flat = [0.0, 0.0, 1.0];
        assert!((land_light(flat, 1.5) - 0.853_553_4).abs() < 1e-4);
        let facing_the_light = [
            0.0,
            std::f32::consts::FRAC_1_SQRT_2,
            std::f32::consts::FRAC_1_SQRT_2,
        ];
        assert!(land_light(facing_the_light, 1.5) > land_light(flat, 1.5));
        let away = [0.0, -1.0, 0.0];
        assert!(land_light(away, 1.5) < land_light(flat, 1.5));
    }

    #[test]
    fn a_held_light_follows_the_way_the_mobile_faces() {
        assert_eq!(held_light_offset(0), Vec2::ZERO);
        assert!(held_light_offset(2).x > 0.0 && held_light_offset(4).x < 0.0);
        assert_eq!(held_light_offset(3 | 0x80), held_light_offset(3));
    }

    #[test]
    fn the_death_screen_shows_when_the_shard_says_so() {
        let mut scene = Scene::new(None);
        let mut frame = WatchFrame::default();
        scene.take_cues(&frame, 0.0);
        frame.cues.push(crate::view::WatchCue {
            seq: 1,
            serial: frame.serial,
            kind: WatchCueKind::DeathScreen,
        });
        scene.take_cues(&frame, 2.0);
        assert_eq!(scene.died_at, Some(2.0));
    }

    #[test]
    fn a_thing_faded_out_is_kept_and_a_whole_one_is_forgotten() {
        let mut scene = Scene::new(None);
        scene.frame_seconds = 10.0;
        let key = FadeKey::Serial(7);
        assert_eq!(scene.alpha_of(key, 0.0), 0.0);
        assert!(scene.fades.contains_key(&key));
        assert_eq!(scene.alpha_of(key, 1.0), 1.0);
        assert!(!scene.fades.contains_key(&key));
    }

    /// A step the session sent shows before the shard takes it, and a step
    /// the shard refused puts the character back at once, with no glide.
    #[test]
    fn a_sent_step_shows_before_the_shard_takes_it() {
        let mut scene = Scene::new(None);
        let resting = WatchFrame {
            x: 100,
            y: 100,
            human_control: true,
            ..WatchFrame::default()
        };
        scene.follow(&resting, 0.0);
        let stepping = WatchFrame {
            stepping_to: Some((101, 100, 0)),
            ..resting.clone()
        };
        scene.follow(&stepping, 0.1);
        assert_eq!(scene.camera_glide.unwrap().goal()[0], 101.0);
        scene.follow(&resting, 0.2);
        assert_eq!(scene.camera, [100.0, 100.0, 0.0], "refused: back at once");
    }

    /// The newest step the watch tells times the move to its tile, once:
    /// the character takes it the lag after its slot, over the time it
    /// lasts. A step from long ago is not the news of a move.
    #[test]
    fn the_newest_sent_step_times_the_move_of_the_character_once() {
        const LASTS: Duration = Duration::from_millis(400);
        /// How far the slot read off the clock may be from the slot given.
        const CLOCK_SLACK: f64 = 0.01;
        let stepping = |slot: std::time::Instant| WatchFrame {
            x: 100,
            y: 100,
            human_control: true,
            stepping_to: Some((101, 100, 0)),
            stride: Some(WatchStride { slot, lasts: LASTS }),
            ..WatchFrame::default()
        };
        let mut scene = Scene::new(None);
        let lag = scene.stride_lag;
        scene.follow(
            &WatchFrame {
                stepping_to: None,
                ..stepping(std::time::Instant::now())
            },
            0.0,
        );
        let sent = stepping(std::time::Instant::now());
        scene.follow(&sent, 0.0);
        let glide = scene.camera_glide.unwrap();
        assert!(
            (glide.started - lag).abs() < CLOCK_SLACK,
            "{}",
            glide.started
        );
        assert!((glide.seconds - LASTS.as_secs_f64()).abs() < CLOCK_SLACK);
        assert_eq!(scene.stride_taken, sent.stride.map(|stride| stride.slot));

        let long_ago = std::time::Instant::now() - LASTS * 10;
        let mut heard = Scene::new(None);
        heard.follow(
            &WatchFrame {
                stepping_to: None,
                ..stepping(long_ago)
            },
            0.0,
        );
        heard.follow(&stepping(long_ago), 0.0);
        assert_eq!(heard.camera_glide.unwrap().started, 0.0, "heard, not timed");
        assert_eq!(heard.stride_taken, None);
    }
}
