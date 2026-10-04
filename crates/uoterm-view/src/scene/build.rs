//! The world of one frame, row by row from the far edge: the land, the
//! statics, the houses, the items, the corpses, the mobiles and the
//! character, each as the Options screen asks, with the lights they throw
//! and the names over them.

use super::canvas::{corners, ellipse, Canvas};
use super::ceiling::FadeKey;
use super::glide::turned_to;
use super::pick::{figure_area, Pick};
use super::plates::Plate;
use super::{
    faded, SceneState, CORPSE_GRAPHIC, DIRECTION_MASK, HALF_TILE, PAWN_RING_RX, PAWN_RING_RY,
    PAWN_RING_WIDTH, PERCENT_MAX, STANDING,
};
use crate::art::{
    deed_action, is_drawn, is_mounted, stance_action, Art, ArtRequest, Cell, ItemPaint, Paint,
    Pose, Sprite, WorldArt,
};
use crate::clicks::PickKind;
use crate::filters::{self, Seat};
use crate::frame::{
    WatchFrame, WatchItem, WatchLook, WatchMobile, SYM_BLOCK, SYM_DOOR, SYM_WALK, SYM_WATER,
};
use crate::geom::{Area, Point, Rgba, Vector};
use crate::lights::{
    flicker, light_cells, shown_light_color, world_light, LightCells, LightRules, LightSource,
};
use crate::look::{self, MobileState, PlateOf};
use crate::model::house_design::{kind_of, piece_look, PieceLook};
use crate::settings::{CircleStyle, FieldStyle};
use crate::ui::theme;
use std::collections::HashMap;
use uoterm_nav::{item_light, Action, Deed, Facing, LightHolder, TileFlagSet};

/// A piece on a see-through storey of the house being designed shows at
/// this share of its opacity.
const DESIGN_SEE_THROUGH: f32 = 0.5;
/// A wall in front of a light this far over it hides the light.
const LIGHT_WALL_RISE: i16 = 5;
/// The season from which plants lose their leaves.
const SEASON_WINTER: u8 = 3;
/// The ring of color under the feet: its size, and how far over the feet
/// its middle is.
const AURA_RADIUS: f32 = 40.0;
const AURA_LIFT: f32 = 5.0;
/// A person seen from the back on a chair takes this pose of a rider.
const SIT_FROM_BACK_GROUP: u8 = 25;
/// A chair is under a person this near his feet.
const SEAT_REACH: i16 = 1;
/// The Video page counts terrain shadows in tenths.
const TERRAIN_SHADOWS_STEP: f32 = 0.1;
/// A circle of transparency is cut in cells of this many points.
const CIRCLE_CELL: f32 = 8.0;
const ITEM_NAME_HUE: u16 = 0x03B2;
const MS_PER_SECOND: f64 = 1000.0;

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
/// The grid lines of the flat map are this thick.
const HAIRLINE: f32 = 1.0;

const PAWN_RING_FILL_ALPHA: f32 = 0.28;
const PAWN_FOOT_HALF: f32 = 7.0;
const PAWN_SHOULDER_HALF: f32 = 5.0;
const PAWN_FOOT_Y: f32 = -3.0;
const PAWN_SHOULDER_Y: f32 = -36.0;
const PAWN_HEAD_Y: f32 = -44.0;
const PAWN_HEAD_RADIUS: f32 = 7.0;
const PAWN_OUTLINE: f32 = 1.5;
const PAWN_HIDDEN_ALPHA: f32 = 0.45;
const PAWN_OUTLINE_COLOR: Rgba = Rgba::from_rgba_premultiplied(4, 6, 10, 235);
const FACING_NEAR: f32 = 1.15;
const FACING_FAR: f32 = 1.9;
const FACING_HALF_WIDTH: f32 = 0.32;
const GHOST_ALPHA: f32 = 0.35;
const TARGET_RING_GROW: f32 = 1.45;

/// How one mobile is drawn: what he looks like, what he does, his color.
struct Looks<'a> {
    look: &'a WatchLook,
    pose: Pose,
    color: Rgba,
    alpha: f32,
    /// One hue over all of him, from the choices of the Options screen.
    hue: Option<u16>,
    /// The hue of the ring of color under his feet, when it shows.
    aura: Option<u16>,
    shadow: bool,
    seat: Option<Seat>,
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

/// A thing that stands on one tile and is drawn in height order.
pub(super) enum Standing<'a> {
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

/// True when the body is a person, by the tables of the animation files.
/// With no animation files nobody is.
pub(super) fn is_person(art: &dyn WorldArt, body: u16) -> bool {
    art.has_anim() && art.anim().is_person(body)
}

/// How far a thing at `x`, `y` is from the character, in tiles.
fn tiles_from(frame: &WatchFrame, x: u16, y: u16) -> u16 {
    x.abs_diff(frame.x).max(y.abs_diff(frame.y))
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
fn held_light_offset(direction: u8) -> Vector {
    const SIDE: f32 = 22.0;
    const HAND: f32 = 33.0;
    const LOW_HAND: f32 = 55.0;
    match direction & DIRECTION_MASK {
        1 => Vector::new(SIDE, HAND),
        2 => Vector::new(SIDE, LOW_HAND),
        3 => Vector::new(0.0, LOW_HAND),
        4 => Vector::new(-SIDE, LOW_HAND),
        5 => Vector::new(-SIDE, HAND),
        _ => Vector::ZERO,
    }
}

/// The flicker seed of a light on a tile: each tile has its own.
fn tile_seed((x, y): (u16, u16)) -> u32 {
    const Y_BITS: u32 = u16::BITS;
    (u32::from(x) << Y_BITS) | u32::from(y)
}

/// The way the character faces, as one step on the tile grid.
fn facing_step(facing: &str) -> Option<(f32, f32)> {
    let (dx, dy) = uoterm_protocol::types::Direction::from_name(facing)?.delta();
    Some((dx as f32, dy as f32))
}

fn radar_symbol(frame: &WatchFrame, column: i32, row: i32) -> Option<char> {
    let line = frame.radar.get(usize::try_from(row).ok()?)?;
    let symbol = line.chars().nth(usize::try_from(column).ok()?)?;
    Some(match symbol {
        SYM_BLOCK | SYM_DOOR | SYM_WATER => symbol,
        _ => SYM_WALK,
    })
}

/// The color of words in a hue.
fn hue_color(art: &dyn WorldArt, hue: u16) -> Rgba {
    let [red, green, blue] = art.text_rgb(hue);
    Rgba::from_rgb(red, green, blue)
}

impl SceneState {
    /// Lays the world on the canvas, row by row from the far edge, and the
    /// plates of the things that have them.
    pub(super) fn build_world(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        self.picks.clear();
        self.light_sources.clear();
        let mut standing = self.standing(art, frame);
        self.ceiling = self.ceiling_over(art, frame, &standing);
        self.circle = self.circle_round(view);
        let half = HALF_TILE * self.zoom;
        let rows = ((view.height() / 2.0 + self.peek.y.abs()) / half).ceil() as i32;
        let columns =
            ((view.width() / 2.0 + self.peek.x.abs()) / half).ceil() as i32 + MARGIN_COLUMNS;
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
                if art.has_art() {
                    self.real_tile(art, view, frame, (tile_x, tile_y), canvas);
                } else {
                    let symbol = radar_symbol(
                        frame,
                        radar_half + i32::from(tile_x) - i32::from(frame.x),
                        radar_half + i32::from(tile_y) - i32::from(frame.y),
                    );
                    self.flat_tile(view, (tile_x, tile_y), symbol, canvas);
                }
                self.things(art, view, frame, (tile_x, tile_y), things, canvas, plates);
            }
        }
        let shown = self.frame_count;
        self.fades.retain(|_, (_, seen)| *seen == shown);
    }

    /// The things that stand on each tile this frame.
    pub(super) fn standing<'a>(
        &self,
        art: &mut dyn WorldArt,
        frame: &'a WatchFrame,
    ) -> HashMap<(i32, i32), Vec<Standing<'a>>> {
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
        for multi in &frame.multis {
            if designed.contains_key(&multi.serial) {
                continue;
            }
            let pieces = art.multi_pieces(multi.multi_id).ready();
            for piece in pieces.unwrap_or_default() {
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

    fn real_tile(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        (x, y): (u16, u16),
        canvas: &mut Canvas,
    ) {
        let Some(cell) = art.cell(frame.map, x, y).ready().cloned() else {
            return;
        };
        if let Some(land_id) = cell.land_id {
            self.land(art, view, frame, (x, y), &cell, land_id, canvas);
        }
        for piece in &cell.statics {
            let shown = StaticArt {
                tile: (x, y),
                z: f32::from(piece.z),
                graphic: piece.graphic,
                hue: piece.hue,
                flags: piece.flags,
                height: piece.height,
                piece: false,
                faded: false,
            };
            self.static_art(art, view, frame, shown, canvas);
        }
    }

    /// The land of one tile: its texture stretched over a slope and lit by
    /// the way each corner faces, or its own flat picture.
    #[allow(clippy::too_many_arguments)]
    fn land(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
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
        let projection = self.projection(view);
        let points = [
            projection.project([fx - 0.5, fy - 0.5, top]),
            projection.project([fx + 0.5, fy - 0.5, right]),
            projection.project([fx + 0.5, fy + 0.5, bottom]),
            projection.project([fx - 0.5, fy + 0.5, left]),
        ];
        let lit = |light: f32| faded(Rgba::WHITE.with_alpha(light).to_opaque(), alpha);
        if let Some(stretch) = cell.stretch {
            let request = ArtRequest::Texture {
                texture_id: stretch.texture_id,
                hue,
            };
            match art.sprite(&request) {
                Art::Ready(sprite) => {
                    let bright =
                        f32::from(self.look.video.terrain_shadows_level) * TERRAIN_SHADOWS_STEP;
                    let colors = stretch
                        .normals
                        .map(|normal| lit(land_light(normal, bright)));
                    canvas.quad_colors(points, corners(sprite.uv), colors);
                    return;
                }
                Art::Pending => return,
                Art::Missing => {}
            }
        }
        let land_id = art.season_land(self.season, land_id);
        let Some(sprite) = art.sprite(&ArtRequest::Land { land_id, hue }).ready() else {
            return;
        };
        let uv = sprite.uv;
        let middle = uv.center();
        let diamond = [
            Point::new(middle.x, uv.min.y),
            Point::new(uv.max.x, middle.y),
            Point::new(middle.x, uv.max.y),
            Point::new(uv.min.x, middle.y),
        ];
        let flat = cell.corners.iter().all(|z| *z == cell.corners[0]);
        if flat {
            canvas.quad(points, diamond, lit(1.0));
            if self.look.video.animated_water && cell.wet {
                let area = Area::from_points(&points);
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
    fn static_art(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        shown: StaticArt,
        canvas: &mut Canvas,
    ) {
        let flags = shown.flags;
        let general = &self.look.general;
        let movable = flags.contains(TileFlagSet::MULTI_MOVABLE);
        let foliage = flags.contains(TileFlagSet::FOLIAGE);
        let bare = foliage && !movable && self.season >= SEASON_WINTER;
        let stumped = foliage && general.trees_to_stumps && (!shown.piece || !movable);
        let hidden_plant =
            !movable && general.hide_vegetation && filters::is_vegetation(shown.graphic, flags);
        if flags.contains(TileFlagSet::INTERNAL) || bare || stumped || hidden_plant {
            return;
        }
        let tree = filters::is_tree(shown.graphic, flags);
        let graphic = if tree && general.trees_to_stumps {
            filters::STUMP_GRAPHIC
        } else {
            shown.graphic
        };
        let border = general.mark_cave_tiles && filters::is_cave(graphic);
        let (x, y) = shown.tile;
        let key = FadeKey::Tile {
            x,
            y,
            z: shown.z as i8,
            graphic: shown.graphic,
        };
        let alpha = self.alpha_of(key, self.target_alpha(shown.z as i16, flags));
        if alpha <= 0.0 {
            return;
        }
        let alpha = if shown.faded {
            alpha * DESIGN_SEE_THROUGH
        } else {
            alpha
        };
        let out_of_range = tiles_from(frame, x, y) > look::VIEW_RANGE;
        let paint = self.paint(frame.dead, false, out_of_range, shown.hue, border);
        let at = [f32::from(x), f32::from(y), shown.z];
        let Some(sprite) = self.item_sprite(art, graphic, paint, true).ready() else {
            return;
        };
        let center = self.screen_of(view, at);
        let area = self.art_area(center, sprite);
        let see_through = !tree
            && !foliage
            && look::see_through(
                shown.z as i8,
                shown.height,
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
            let z = shown.z as i16;
            let holder = LightHolder::MapStatic;
            self.add_ground_light(art, frame.map, (x, y), z, shown.graphic, holder, center);
        }
    }

    /// Where the picture of an item goes. Its bottom edge lies on the bottom
    /// point of its tile, and it is centered on the tile.
    fn art_area(&self, tile_center: Point, sprite: Sprite) -> Area {
        let size = Vector::new(sprite.width, sprite.height) * self.zoom;
        let bottom = tile_center.y + HALF_TILE * self.zoom;
        Area::from_min_size(
            Point::new(tile_center.x - size.x / 2.0, bottom - size.y),
            size,
        )
    }

    fn flat_tile(&self, view: Area, (x, y): (u16, u16), symbol: Option<char>, canvas: &mut Canvas) {
        let (fx, fy) = (f32::from(x), f32::from(y));
        let projection = self.projection(view);
        let corner = |dx: f32, dy: f32, lift: f32| {
            projection.project([fx + dx, fy + dy, self.camera[2]])
                - Vector::new(0.0, lift * self.zoom)
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
        let grid = Rgba::WHITE.with_alpha(FLAT_GRID_ALPHA);
        let [top, right, _, left] = diamond(height);
        let hair = Vector::new(0.0, HAIRLINE);
        if i32::from(y) % FLAT_GRID_EVERY == 0 {
            canvas.fill(&[top, right, right + hair, top + hair], grid);
        }
        if i32::from(x) % FLAT_GRID_EVERY == 0 {
            canvas.fill(&[left, top, top + hair, left + hair], grid);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn things(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        tile: (u16, u16),
        mut things: Vec<Standing<'_>>,
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        things.sort_by(|a, b| a.z().total_cmp(&b.z()));
        for thing in things {
            match thing {
                Standing::Art(item) => self.ground_item(art, view, frame, item, canvas, plates),
                Standing::Piece { graphic, z, faded } => {
                    let (flags, height) = art
                        .item_tile(graphic)
                        .map_or((TileFlagSet::NONE, 0), |tile| (tile.flags, tile.height));
                    let shown = StaticArt {
                        tile,
                        z,
                        graphic,
                        hue: 0,
                        flags,
                        height,
                        piece: true,
                        faded,
                    };
                    self.static_art(art, view, frame, shown, canvas);
                }
                Standing::Corpse(item) => self.corpse(art, view, frame, item, canvas, plates),
                Standing::Mobile { mobile, at } => {
                    self.mobile(art, view, frame, mobile, at, canvas, plates);
                }
                Standing::Character { at } => {
                    self.character(art, view, frame, at, canvas, plates);
                }
            }
        }
    }

    /// One item on the ground: its hue, the way fields show, its name and
    /// its light.
    fn ground_item(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        item: &WatchItem,
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        let flags = art
            .item_tile(item.graphic)
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
        let center = self.screen_of(view, at);
        let area = match self.item_sprite(art, graphic, paint, animate) {
            Art::Ready(sprite) => {
                let area = self.art_area(center, sprite);
                canvas.sprite(sprite, area, faded(Rgba::WHITE, alpha));
                area
            }
            Art::Missing if !art.has_art() => {
                let radius = Vector::new(FLAT_ITEM_RADIUS * 2.0, FLAT_ITEM_RADIUS) * self.zoom;
                canvas.fill(&ellipse(center, radius), theme::FLAT_ITEM);
                Area::from_center_size(center, radius * 2.0)
            }
            Art::Missing | Art::Pending => return,
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
        let z = i16::from(item.z);
        self.add_ground_light(art, frame.map, tile, z, item.graphic, holder, center);
    }

    /// A corpse is the fallen body, lying the way the shard says. The amount
    /// of a corpse item is the body it was. With no picture of the death, a
    /// ring marks the place.
    fn corpse(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
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
        let center = self.screen_of(
            view,
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
        let area = match corpse_sprite(art, look, paint) {
            Art::Ready(sprite) => {
                let area = Area::from_min_size(
                    center - sprite.anchor * self.zoom,
                    Vector::new(sprite.width, sprite.height) * self.zoom,
                );
                canvas.sprite(sprite, area, faded(Rgba::WHITE, alpha));
                area
            }
            Art::Missing => {
                let radius = Vector::new(PAWN_RING_RX, PAWN_RING_RY) * self.zoom;
                canvas.ring(center, radius, PAWN_RING_WIDTH * self.zoom, theme::CORPSE);
                Area::from_center_size(center, radius * 2.0)
            }
            Art::Pending => return,
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
    #[allow(clippy::too_many_arguments)]
    fn mobile(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
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
            person: is_person(art, mobile.look.body),
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
        let seat = seat_of(art, frame, look, at, resting);
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
        let foot = self.screen_of(view, at);
        let Some(area) = self.figure(art, canvas, looks, foot, target) else {
            return;
        };
        self.held_lights(art, mobile.serial, look, area);
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
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        at: [f32; 3],
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        let foot = self.screen_of(view, at);
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
                let radius = Vector::new(PAWN_RING_RX, PAWN_RING_RY) * self.zoom;
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
        let seat = seat_of(art, frame, look, at, resting);
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
        let Some(area) = self.figure(art, canvas, looks, foot, false) else {
            return;
        };
        self.held_lights(art, frame.serial, look, area);
        // The Modern style shows the character's name on its own bar.
        if !classic {
            return;
        }
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
        plates.push(plate);
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

    /// The ring round a figure in the Modern style. The Classic style draws
    /// none.
    fn outline(&self, color: Rgba) -> [u8; 4] {
        if self.look.classic() {
            Rgba::TRANSPARENT.to_array()
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

    /// The picture of an item as it shows now, in the season of the world.
    /// `animate` lets a fire or a fountain go through its pictures.
    pub fn item_sprite(
        &self,
        art: &mut dyn WorldArt,
        graphic: u16,
        paint: ItemPaint,
        animate: bool,
    ) -> Art<Sprite> {
        let now_ms = (self.now * MS_PER_SECOND) as u64;
        let graphic = art.season_item(self.season, graphic);
        let shown = if animate {
            art.shown_graphic(graphic, now_ms)
        } else {
            graphic
        };
        art.sprite(&ArtRequest::item(shown, paint))
    }

    /// Lays one picture of the world on the canvas as `lay` says.
    fn lay(&self, canvas: &mut Canvas, sprite: Sprite, area: Area, lay: Lay) {
        let color = faded(Rgba::WHITE, lay.alpha);
        if lay.shadow {
            canvas.shadow(sprite, area, self.zoom, lay.alpha);
        }
        match self.circle.filter(|_| lay.see_through) {
            Some(circle) if circle.style == CircleStyle::Gradient => {
                let middle = area.center_bottom() - Vector::new(0.0, HALF_TILE * self.zoom);
                let ratio = middle.distance(circle.center) / circle.radius;
                let shown = look::circle_alpha(CircleStyle::Gradient, ratio);
                canvas.sprite(sprite, area, faded(Rgba::WHITE, lay.alpha * shown));
            }
            Some(circle) if area.distance_to(circle.center) < circle.radius => {
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
        art: &dyn WorldArt,
        graphic: u16,
        holder: LightHolder,
        center: Point,
        seed: u32,
    ) -> Option<LightSource> {
        let tile = art.item_tile(graphic)?;
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
    #[allow(clippy::too_many_arguments)]
    fn add_ground_light(
        &mut self,
        art: &mut dyn WorldArt,
        map: u8,
        tile: (u16, u16),
        z: i16,
        graphic: u16,
        holder: LightHolder,
        center: Point,
    ) {
        if let Some(source) = self.light_of(art, graphic, holder, center, tile_seed(tile)) {
            if !self.light_hidden(art, map, tile, z) {
                self.light_sources.push(source);
            }
        }
    }

    /// The lights mobile `serial` holds, such as a torch or a lantern,
    /// placed as the classic client places them for the way he faces.
    fn held_lights(&mut self, art: &dyn WorldArt, serial: u32, look: &WatchLook, area: Area) {
        let mirrored = Facing::from_direction(look.direction).mirrored;
        let base = Point::new(if mirrored { area.max.x } else { area.min.x }, area.min.y);
        let center = base + held_light_offset(look.direction) * self.zoom;
        for item in &look.equipment {
            if let Some(source) =
                self.light_of(art, item.graphic, LightHolder::Held, center, serial)
            {
                self.light_sources.push(source);
            }
        }
    }

    /// True when a wall in front hides the light of a thing at this tile.
    fn light_hidden(&self, art: &mut dyn WorldArt, map: u8, (x, y): (u16, u16), z: i16) -> bool {
        let max_z = self.ceiling.max_z;
        let Some(cell) = art
            .cell(map, x.saturating_add(1), y.saturating_add(1))
            .ready()
        else {
            return false;
        };
        cell.statics.iter().any(|piece| {
            !piece.flags.contains(TileFlagSet::TRANSPARENT)
                && (z + LIGHT_WALL_RISE..max_z).contains(&i16::from(piece.z))
        })
    }

    /// The light of the world over `view`: the light level, and each lamp,
    /// torch and lit spell in view. `effects` are the places and the
    /// graphics of the spells that show now. None when the world shows in
    /// its own light.
    pub fn light_cells(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        effects: &[([f32; 3], u16)],
    ) -> Option<LightCells> {
        if frame.dead && self.look.video.black_and_white_when_dead {
            return None;
        }
        let rules = LightRules::from(&self.look.video);
        let light = world_light(frame.light, frame.personal_light, &rules);
        let projection = self.projection(view);
        for (place, graphic) in effects {
            let tile = (place[0] as u16, place[1] as u16);
            let center = projection.project(*place);
            let holder = LightHolder::MapStatic;
            if let Some(source) = self.light_of(art, *graphic, holder, center, tile_seed(tile)) {
                self.light_sources.push(source);
            }
        }
        if light.level >= 1.0 && !rules.alternative {
            return None;
        }
        let sources = std::mem::take(&mut self.light_sources);
        let mut shapes = HashMap::new();
        for source in &sources {
            if let Some(shape) = art.light_shape(source.shape).ready() {
                shapes.entry(source.shape).or_insert_with(|| shape.clone());
            }
        }
        Some(light_cells(
            view,
            light,
            rules.alternative,
            &sources,
            self.zoom,
            |shape| shapes.get(&shape),
        ))
    }

    /// One mobile: his aura, his shadow and his real picture, in the hue
    /// the Options screen puts over him. The Modern style adds a ring on the
    /// ground and an outline in his color. When the client files hold no
    /// picture of him, a plain figure stands in. Gives the area he covers,
    /// or None while his picture is on its way.
    fn figure(
        &self,
        art: &mut dyn WorldArt,
        canvas: &mut Canvas,
        looks: Looks<'_>,
        foot: Point,
        target: bool,
    ) -> Option<Area> {
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
            let middle = foot - Vector::new(0.0, AURA_LIFT * zoom);
            canvas.glow(middle, AURA_RADIUS * zoom, hue_color(art, aura));
        }
        if !self.look.classic() {
            let ring = Vector::new(PAWN_RING_RX, PAWN_RING_RY) * zoom;
            if target {
                canvas.ring(
                    foot,
                    ring * TARGET_RING_GROW,
                    PAWN_RING_WIDTH * zoom,
                    faded(theme::ALARM, alpha),
                );
            }
            canvas.fill(
                &ellipse(foot, ring),
                faded(color, alpha * PAWN_RING_FILL_ALPHA),
            );
            canvas.ring(foot, ring, PAWN_RING_WIDTH * zoom, faded(color, alpha));
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
        let pose = match seat {
            Some(seat) if seat.from_back => Pose {
                action: Action::Shown(SIT_FROM_BACK_GROUP),
                tick: 0,
            },
            Some(_) => STANDING,
            None => Pose {
                action: stance_action(art.anim(), look, pose.action),
                ..pose
            },
        };
        let sprite = match figure_sprite(art, look, pose, paint) {
            Art::Ready(sprite) => sprite,
            Art::Missing => {
                self.plain_figure(canvas, foot, color, alpha);
                return Some(figure_area(foot, zoom));
            }
            Art::Pending => return None,
        };
        let foot = foot + seat.map_or(Vector::ZERO, |seat| seat.offset * zoom);
        let area = Area::from_min_size(
            foot - sprite.anchor * zoom,
            Vector::new(sprite.width, sprite.height) * zoom,
        );
        if shadow {
            canvas.shadow(sprite, area, zoom, alpha);
        }
        let tint = faded(Rgba::WHITE, alpha);
        match seat {
            Some(seat) if !seat.from_back => {
                let mirrored = Facing::from_direction(seat.facing).mirrored;
                canvas.sitting(sprite, area, tint, mirrored, zoom);
            }
            _ => canvas.sprite(sprite, area, tint),
        }
        Some(area)
    }

    /// A body and a head in one color, for a mobile with no real picture.
    fn plain_figure(&self, canvas: &mut Canvas, foot: Point, color: Rgba, alpha: f32) {
        let zoom = self.zoom;
        let body = |grow: f32| {
            [
                foot + Vector::new(-PAWN_FOOT_HALF - grow, PAWN_FOOT_Y + grow) * zoom,
                foot + Vector::new(PAWN_FOOT_HALF + grow, PAWN_FOOT_Y + grow) * zoom,
                foot + Vector::new(PAWN_SHOULDER_HALF + grow, PAWN_SHOULDER_Y - grow) * zoom,
                foot + Vector::new(-PAWN_SHOULDER_HALF - grow, PAWN_SHOULDER_Y - grow) * zoom,
            ]
        };
        let head = |grow: f32| {
            ellipse(
                foot + Vector::new(0.0, PAWN_HEAD_Y) * zoom,
                Vector::splat((PAWN_HEAD_RADIUS + grow) * zoom),
            )
        };
        let outline = faded(PAWN_OUTLINE_COLOR, alpha);
        canvas.fill(&body(PAWN_OUTLINE), outline);
        canvas.fill(&head(PAWN_OUTLINE), outline);
        canvas.fill(&body(0.0), faded(color, alpha));
        canvas.fill(&head(0.0), faded(color, alpha));
    }

    /// A wedge on the ground that points the way the character faces.
    fn facing_mark(&self, canvas: &mut Canvas, foot: Point, facing: &str) {
        let Some((dx, dy)) = facing_step(facing) else {
            return;
        };
        let length = (dx * dx + dy * dy).sqrt();
        let (dx, dy) = (dx / length, dy / length);
        let on_ground = |along: f32, across: f32| {
            let (x, y) = (dx * along - dy * across, dy * along + dx * across);
            foot + Vector::new((x - y) * HALF_TILE, (x + y) * HALF_TILE) * self.zoom
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
}

/// The seat a person takes on a chair under him, while he rests.
fn seat_of(
    art: &mut dyn WorldArt,
    frame: &WatchFrame,
    look: &WatchLook,
    at: [f32; 3],
    resting: bool,
) -> Option<Seat> {
    if !resting || is_mounted(look) || !is_person(art, look.body) {
        return None;
    }
    let (x, y) = (at[0].round() as u16, at[1].round() as u16);
    let z = at[2].round() as i16;
    let near = |thing_z: i16| (thing_z - z).abs() <= SEAT_REACH;
    let statics: Vec<u16> = art
        .cell(frame.map, x, y)
        .ready()
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

/// The picture of one mobile in one pose. Two ticks that show the same
/// frame share one picture.
fn figure_sprite(
    art: &mut dyn WorldArt,
    look: &WatchLook,
    pose: Pose,
    paint: Paint,
) -> Art<Sprite> {
    let frames = match art.frame_count(look, pose.action) {
        Art::Ready(frames) => frames,
        Art::Pending => return Art::Pending,
        Art::Missing => return Art::Missing,
    };
    let request = ArtRequest::Figure {
        look: look.clone(),
        pose: Pose {
            tick: pose.tick % frames.max(1),
            ..pose
        },
        paint,
    };
    art.sprite(&request)
}

/// The picture of a fallen body: the last picture of its death.
fn corpse_sprite(art: &mut dyn WorldArt, look: WatchLook, paint: Paint) -> Art<Sprite> {
    let Some(action) = deed_action(art.anim(), &look, Deed::Die) else {
        return Art::Missing;
    };
    let frames = match art.frame_count(&look, action) {
        Art::Ready(frames) => frames,
        Art::Pending => return Art::Pending,
        Art::Missing => return Art::Missing,
    };
    let last = Pose {
        action,
        tick: frames.saturating_sub(1),
    };
    figure_sprite(art, &look, last, paint)
}

#[cfg(test)]
mod tests {
    use super::super::test_art::NoFiles;
    use super::*;
    use crate::frame::{WatchDesigning, WatchMulti};
    use crate::model::house_design::{StoreyLook, STOREYS};
    use uoterm_nav::{HousePart, HousePartKind};

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
        let without_files = SceneState::new();
        let frame = WatchFrame {
            multis: vec![WatchMulti {
                serial: 50,
                multi_id: 100,
                x: 10,
                y: 10,
                z: 0,
            }],
            ..WatchFrame::default()
        };
        let standing = without_files.standing(&mut NoFiles::default(), &frame);
        assert_eq!(standing.len(), 1, "only the character stands");
    }

    /// While the player designs, a hidden storey leaves its pieces out and a
    /// see-through one fades them; other houses show as they are.
    #[test]
    fn the_storeys_of_the_house_being_designed_show_as_the_designer_asks() {
        const WALL: u16 = 0x0064;
        let mut scene = SceneState::new();
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
        let faded_at = |scene: &SceneState, x| {
            scene
                .standing(&mut NoFiles::default(), &frame)
                .get(&(x, 10))
                .and_then(|things| {
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
        assert_eq!(held_light_offset(0), Vector::ZERO);
        assert!(held_light_offset(2).x > 0.0 && held_light_offset(4).x < 0.0);
        assert_eq!(held_light_offset(3 | 0x80), held_light_offset(3));
    }

    #[test]
    fn a_thing_with_no_picture_is_a_flat_mark_the_mouse_can_pick() {
        const VIEW: Area = Area {
            min: Point { x: 0.0, y: 0.0 },
            max: Point { x: 400.0, y: 300.0 },
        };
        const BARREL: u16 = 0x0E77;
        let frame = WatchFrame {
            x: 100,
            y: 100,
            items: vec![WatchItem {
                serial: 7,
                graphic: BARREL,
                x: 101,
                y: 100,
                name: "a barrel".into(),
                ..WatchItem::default()
            }],
            ..WatchFrame::default()
        };
        let mut scene = SceneState::new();
        let mut art = NoFiles::default();
        scene.follow(&art, &frame, 0.0);
        let mut canvas = Canvas::new(Point::new(0.5, 0.5));
        let mut plates = Vec::new();
        scene.build_world(&mut art, VIEW, &frame, &mut canvas, &mut plates);
        let barrel = scene.screen_of(VIEW, [101.0, 100.0, 0.0]);
        assert_eq!(scene.thing_at(barrel).map(|pick| pick.serial), Some(7));
        assert_eq!(plates.len(), 1, "the barrel has a plate");
        assert_eq!(plates[0].of, PlateOf::Item);
    }
}
