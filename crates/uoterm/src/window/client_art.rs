//! The client files as the map needs them: what lies on each tile, and the
//! picture of each thing. The window reads the files itself, so the session
//! sends no pictures.

use super::atlas::{ArtKey, Atlas, Picture, Sprite};
use super::classic::text::{TextLook, UoFonts};
use super::figure::{self, FrameCache, Pose};
use crate::view::{WatchLiveMap, WatchLook};
use eframe::egui::Vec2;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use uoterm_nav::{
    land_is_ignored, Action, AnimData, ArtCycles, ArtData, ArtPixels, CursorSet, CursorShape, Deed,
    EquipConv, GumpArt, HueData, ItemTile, LightData, LightShape, MulMap, MultiData, MultiPiece,
    RadarColors, SeasonArt, Stance, TexmapData, TextPicture, TileData, TileFlagSet, TileQuery,
    TILE_ANIMATED, TILE_PARTIAL_HUE,
};
use uoterm_protocol::types::WEAPON_LAYERS;

/// How many tiles the window remembers. A full window shows about four
/// thousand, so this is a few windows of walking.
const CELL_CACHE_CAP: usize = 24_000;
/// Item graphics that the client never draws.
const NO_DRAW_GRAPHICS: [u16; 3] = [0x0001, 0x21BC, 0x63D3];
const NO_DRAW_RANGE: std::ops::RangeInclusive<u16> = 0x2198..=0x21A4;

/// One unit of height is this many pixels, as the normals of the land are
/// worked out.
const NORMAL_Z_PIXELS: f32 = 4.0;
/// Half the side of a tile, as the normals of the land are worked out.
const NORMAL_HALF_TILE: f32 = 22.0;
/// The sides of a black border round a cave wall.
const BORDER_RGBA: [u8; 4] = [0, 0, 0, u8::MAX];
const RGBA_BYTES: usize = 4;

#[derive(Clone, Copy, Debug)]
pub struct CellStatic {
    pub graphic: u16,
    pub hue: u16,
    pub z: i8,
    /// The height of its top, when a person can stand on it.
    pub floor: Option<i16>,
    pub flags: TileFlagSet,
    pub height: u8,
}

/// A land tile that is not flat. Its texture is stretched over the slope
/// and each corner is lit by the way it faces.
#[derive(Clone, Copy, Debug)]
pub struct Stretch {
    pub texture_id: u16,
    /// The normals at the top, right, bottom and left corners.
    pub normals: [[f32; 3]; 4],
}

/// One map tile as the window draws it.
#[derive(Clone, Debug)]
pub struct Cell {
    /// None when the land of the tile draws nothing.
    pub land_id: Option<u16>,
    /// The heights of the top, right, bottom and left points of the diamond.
    pub corners: [i8; 4],
    pub statics: Vec<CellStatic>,
    /// Some for a slope that has a texture.
    pub stretch: Option<Stretch>,
    /// The height a slope counts as, as the classic client takes it.
    pub average_z: i8,
    /// The land is water.
    pub wet: bool,
}

/// The normal of the land at one corner from the heights of the tile of
/// that corner and of the four tiles round it. None for flat land.
fn land_normal(tile: i8, top: i8, right: i8, bottom: i8, left: i8) -> Option<[f32; 3]> {
    if [top, right, bottom, left].iter().all(|z| *z == tile) {
        return None;
    }
    let rise = |z: i8| (f32::from(z) - f32::from(tile)) * NORMAL_Z_PIXELS;
    let h = NORMAL_HALF_TILE;
    let cross = |v: [f32; 3], u: [f32; 3]| {
        [
            v[1] * u[2] - v[2] * u[1],
            v[2] * u[0] - v[0] * u[2],
            v[0] * u[1] - v[1] * u[0],
        ]
    };
    let corners = [
        [-h, -h, rise(left)],
        [-h, h, rise(bottom)],
        [h, h, rise(right)],
        [h, -h, rise(top)],
    ];
    let mut sum = [0.0f32; 3];
    for (at, u) in corners.iter().enumerate() {
        let v = corners[(at + 1) % corners.len()];
        let part = cross(v, *u);
        for axis in 0..3 {
            sum[axis] += part[axis];
        }
    }
    let length = sum.iter().map(|c| c * c).sum::<f32>().sqrt();
    Some(sum.map(|c| c / length))
}

/// The height a slope counts as: the middle of its flatter diagonal.
fn average_z([top, right, bottom, left]: [i8; 4]) -> i8 {
    let middle = |a: i8, b: i8| ((i16::from(a) + i16::from(b)) >> 1) as i8;
    if (i16::from(top) - i16::from(bottom)).abs() <= (i16::from(left) - i16::from(right)).abs() {
        middle(top, bottom)
    } else {
        middle(left, right)
    }
}

/// How an item picture is painted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemPaint {
    pub hue: u16,
    /// The hue covers every pixel, as a hue the window puts on it does.
    pub whole_hue: bool,
    /// A black border marks a cave wall.
    pub border: bool,
}

pub struct ClientArt {
    uopath: PathBuf,
    art: ArtData,
    /// None when the client files hold no classic animation files.
    anim: Option<AnimData>,
    frames: FrameCache,
    hues: Option<HueData>,
    /// None when the client files hold no picture cycles.
    cycles: Option<ArtCycles>,
    /// None when the client files hold no houses and boats.
    multis: Option<MultiData>,
    /// None when the client files hold no gump pictures.
    gumps: Option<GumpArt>,
    /// The art that each season swaps.
    seasons: SeasonArt,
    /// None when the client files hold no colors for a world map.
    radar: Option<RadarColors>,
    /// None marks a map the client files do not hold.
    maps: HashMap<u8, Option<MulMap>>,
    cells: HashMap<(u8, u16, u16), Cell>,
    /// The UltimaLive blocks laid over the maps, each with the count of
    /// changes it was laid at.
    live_blocks: HashMap<(u8, u32), u64>,
    /// None when the client files hold no tiledata.
    tiles: Option<TileData>,
    /// None when the client files hold no land textures.
    textures: Option<TexmapData>,
    /// None when the client files hold no light shapes.
    lights: Option<LightData>,
    light_shapes: HashMap<u8, Option<LightShape>>,
    /// None when the client files hold no UO fonts.
    fonts: Option<UoFonts>,
    cursors: CursorSet,
}

/// The map files of a facet, opened the first time they are asked for.
fn facet_files<'a>(
    maps: &'a mut HashMap<u8, Option<MulMap>>,
    uopath: &Path,
    map_index: u8,
) -> Option<&'a MulMap> {
    maps.entry(map_index)
        .or_insert_with(|| MulMap::open(uopath, map_index).ok())
        .as_ref()
}

pub fn is_drawn(graphic: u16) -> bool {
    !NO_DRAW_GRAPHICS.contains(&graphic) && !NO_DRAW_RANGE.contains(&graphic)
}

impl ClientArt {
    pub fn open(uopath: &Path) -> Result<Self, String> {
        let art = ArtData::open(uopath).map_err(|e| e.to_string())?;
        Ok(Self {
            uopath: uopath.to_path_buf(),
            cursors: CursorSet::load(&art),
            art,
            anim: AnimData::open(uopath).ok(),
            frames: FrameCache::default(),
            hues: HueData::open(uopath).ok(),
            cycles: ArtCycles::open(uopath).ok(),
            multis: MultiData::open(uopath).ok(),
            radar: RadarColors::open(uopath).ok(),
            gumps: GumpArt::open(uopath).ok(),
            seasons: SeasonArt::open(&uoterm_runtime::config::config_dir()),
            maps: HashMap::new(),
            cells: HashMap::new(),
            live_blocks: HashMap::new(),
            tiles: TileData::open(uopath).ok(),
            textures: TexmapData::open(uopath).ok(),
            lights: LightData::open(uopath).ok(),
            light_shapes: HashMap::new(),
            fonts: UoFonts::open(uopath).ok(),
        })
    }

    /// Lays the map blocks an UltimaLive shard changed over the map files.
    /// A block is laid again only when it changed again, and the tiles read
    /// before a change are read anew.
    pub fn take_live_map(&mut self, live: &WatchLiveMap) {
        let fresh: Vec<_> = live
            .blocks
            .iter()
            .filter(|block| self.live_blocks.get(&(live.map, block.block)) != Some(&block.changed))
            .collect();
        if fresh.is_empty() {
            return;
        }
        let uopath = &self.uopath;
        let Some(map) = self
            .maps
            .entry(live.map)
            .or_insert_with(|| MulMap::open(uopath, live.map).ok())
            .as_ref()
        else {
            return;
        };
        for block in fresh {
            let number = u64::from(block.block);
            let land = block
                .land
                .as_ref()
                .map_or(Ok(()), |land| map.set_live_land(number, land));
            let statics = block
                .statics
                .as_ref()
                .map_or(Ok(()), |records| map.set_live_statics(number, records));
            if let Err(error) = land.and(statics) {
                tracing::warn!(%error, block = number, "an UltimaLive block was left out");
            }
            self.live_blocks
                .insert((live.map, block.block), block.changed);
        }
        self.cells.retain(|(map, _, _), _| *map != live.map);
    }

    /// Opens the map files of a facet, when they are not open yet. Figures
    /// need them too, for the tiledata of what a mobile wears.
    pub fn open_map(&mut self, map_index: u8) {
        facet_files(&mut self.maps, &self.uopath, map_index);
    }

    pub fn cell(&mut self, map_index: u8, x: u16, y: u16) -> Option<&Cell> {
        let key = (map_index, x, y);
        if !self.cells.contains_key(&key) {
            let map = facet_files(&mut self.maps, &self.uopath, map_index)?;
            if !map.in_bounds(x, y) {
                return None;
            }
            let column = map.column(x, y);
            let statics = map
                .statics_at(x, y)
                .into_iter()
                .filter(|s| is_drawn(s.graphic))
                .map(|s| CellStatic {
                    graphic: s.graphic,
                    hue: s.hue,
                    z: s.z,
                    floor: s.surface().then(|| i16::from(s.z) + i16::from(s.height)),
                    flags: TileFlagSet(u64::from(s.flags)),
                    height: s.height,
                })
                .collect();
            let corners = [
                column.land.north_west,
                column.land.north_east,
                column.land.south_east,
                column.land.south_west,
            ];
            let land = self
                .tiles
                .as_ref()
                .and_then(|tiles| tiles.land(column.land_id));
            let texture_id = land.map_or(0, |land| land.texture_id);
            let stretch = (texture_id != 0)
                .then(|| {
                    let z = |dx: i32, dy: i32| {
                        let at = (i32::from(x) + dx, i32::from(y) + dy);
                        match (u16::try_from(at.0), u16::try_from(at.1)) {
                            (Ok(x), Ok(y)) => map.column(x, y).land.north_west,
                            _ => corners[0],
                        }
                    };
                    let [z00, z10, z11, z01] = corners;
                    let found = [
                        land_normal(z00, z(0, -1), z10, z01, z(-1, 0)),
                        land_normal(z10, z(1, -1), z(2, 0), z11, z00),
                        land_normal(z11, z10, z(2, 1), z(1, 2), z01),
                        land_normal(z01, z00, z11, z(0, 2), z(-1, 1)),
                    ];
                    let flat = [0.0, 0.0, 1.0];
                    found.iter().any(Option::is_some).then(|| Stretch {
                        texture_id,
                        normals: found.map(|normal| normal.unwrap_or(flat)),
                    })
                })
                .flatten();
            let cell = Cell {
                land_id: (!land_is_ignored(column.land_id)).then_some(column.land_id),
                corners,
                statics,
                average_z: if stretch.is_some() {
                    average_z(corners)
                } else {
                    corners[0]
                },
                stretch,
                wet: land.is_some_and(|land| land.flags.contains(TileFlagSet::WET)),
            };
            if self.cells.len() >= CELL_CACHE_CAP {
                self.cells.clear();
            }
            self.cells.insert(key, cell);
        }
        self.cells.get(&key)
    }

    pub fn land_sprite(&self, atlas: &mut Atlas, land_id: u16, hue: u16) -> Option<Sprite> {
        atlas.sprite(ArtKey::Land { land_id, hue }, || {
            let ramp = self.hues.as_ref().and_then(|h| h.ramp(hue, false));
            self.art.land(land_id).map(|art| picture_of(&art, ramp))
        })
    }

    /// The texture a slope is drawn with.
    pub fn texture_sprite(&self, atlas: &mut Atlas, texture_id: u16, hue: u16) -> Option<Sprite> {
        atlas.sprite(ArtKey::Texture { texture_id, hue }, || {
            let ramp = self.hues.as_ref().and_then(|h| h.ramp(hue, false));
            let texture = self.textures.as_ref()?.texture(texture_id)?;
            Some(picture_of(&texture, ramp))
        })
    }

    /// The tiledata record of an item graphic.
    pub fn item_tile(&self, graphic: u16) -> Option<&ItemTile> {
        self.tiles.as_ref()?.item(graphic)
    }

    /// Reads one light shape of the light files, the first time it is
    /// asked for.
    pub fn load_light_shape(&mut self, shape: u8) {
        let lights = self.lights.as_ref();
        self.light_shapes
            .entry(shape)
            .or_insert_with(|| lights.and_then(|lights| lights.shape(shape)));
    }

    /// One light shape that was read before.
    pub fn light_shape(&self, shape: u8) -> Option<&LightShape> {
        self.light_shapes.get(&shape)?.as_ref()
    }

    /// The look words take: words that wrap take no more width than they
    /// need, as the words over heads of the classic client.
    fn fitted(fonts: &UoFonts, text: &str, look: TextLook) -> TextLook {
        match look.width.filter(|_| !look.crop) {
            Some(wrap) => TextLook {
                width: Some(fonts.width(look.font, text).min(wrap)),
                ..look
            },
            None => look,
        }
    }

    /// Words in a UO font as `look` says. None when the files hold no UO
    /// fonts.
    pub fn text_sprite(&self, atlas: &mut Atlas, text: &str, look: TextLook) -> Option<Sprite> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (text, look).hash(&mut hasher);
        atlas.sprite(ArtKey::Text(hasher.finish()), || {
            let fonts = self.fonts.as_ref()?;
            let picture = fonts.render(text, &Self::fitted(fonts, text, look))?;
            Some(text_picture(picture))
        })
    }

    /// The height of one line of words in a UO font.
    pub fn line_height(&self, look: &TextLook) -> Option<u32> {
        Some(self.fonts.as_ref()?.line_height(look))
    }

    /// A mouse pointer of the classic client. Its anchor is the pixel that
    /// points.
    pub fn cursor_sprite(
        &self,
        atlas: &mut Atlas,
        shape: CursorShape,
        war: bool,
        hue: u16,
    ) -> Option<Sprite> {
        atlas.sprite(ArtKey::Cursor { shape, war, hue }, || {
            let cursor = self.cursors.get(shape, war)?;
            let ramp = self.hues.as_ref().and_then(|h| h.ramp(hue, false));
            Some(Picture {
                anchor: Vec2::new(cursor.hot_x as f32, cursor.hot_y as f32),
                ..picture_of(&cursor.pixels, ramp)
            })
        })
    }

    fn item_flags(&self, map_index: u8, graphic: u16) -> u32 {
        self.tiledata(map_index)
            .map_or(0, |map| map.item_flags(graphic))
    }

    /// Each map holds the same tiledata, so the map of the character serves.
    fn tiledata(&self, map_index: u8) -> Option<&MulMap> {
        self.maps.get(&map_index).and_then(Option::as_ref)
    }

    pub fn item_sprite(
        &self,
        atlas: &mut Atlas,
        map_index: u8,
        graphic: u16,
        paint: ItemPaint,
    ) -> Option<Sprite> {
        let key = ArtKey::Item {
            graphic,
            hue: paint.hue,
            whole_hue: paint.whole_hue,
            border: paint.border,
        };
        atlas.sprite(key, || {
            let art = self.art.item(graphic)?;
            let partial =
                !paint.whole_hue && self.item_flags(map_index, graphic) & TILE_PARTIAL_HUE != 0;
            let ramp = self.hues.as_ref().and_then(|h| h.ramp(paint.hue, partial));
            let mut picture = picture_of(&art, ramp);
            if paint.border {
                draw_border(&mut picture);
            }
            Some(picture)
        })
    }

    /// True when the client files hold the pictures of gumps.
    pub fn has_gump_art(&self) -> bool {
        self.gumps.is_some()
    }

    /// A gump picture in a hue. A `partial` hue colors the grey pixels
    /// only.
    pub fn gump_sprite(
        &self,
        atlas: &mut Atlas,
        gump: u16,
        hue: u16,
        partial: bool,
    ) -> Option<Sprite> {
        atlas.sprite(ArtKey::Gump { gump, hue, partial }, || {
            let art = self.gumps.as_ref()?.gump(gump)?;
            let ramp = self.hues.as_ref().and_then(|h| h.ramp(hue, partial));
            Some(picture_of(&art, ramp))
        })
    }

    /// True when the gump draws the pixel at `x`, `y` of its picture, for
    /// a click that must fall on what a gump shows, not on its box.
    pub fn gump_drawn_at(&self, gump: u16, x: usize, y: usize) -> bool {
        self.gumps
            .as_ref()
            .and_then(|gumps| gumps.gump(gump))
            .is_some_and(|art| art.is_drawn(x, y))
    }

    /// What `Equipconv.def` puts in the place of a worn item on a body.
    pub fn equip_conv(&self, body: u16, worn_anim: u16) -> Option<EquipConv> {
        self.anim.as_ref()?.equip_conv(body, worn_anim)
    }

    /// The color of one tile on a map of the world: the color of its
    /// highest item, or of its land. None past the edge of the map.
    pub fn radar_rgb(&mut self, map_index: u8, x: u16, y: u16) -> Option<[u8; 3]> {
        let radar = self.radar.as_ref()?;
        let uopath = &self.uopath;
        let map = self
            .maps
            .entry(map_index)
            .or_insert_with(|| MulMap::open(uopath, map_index).ok())
            .as_ref()?;
        if !map.in_bounds(x, y) {
            return None;
        }
        let top = map
            .statics_at(x, y)
            .into_iter()
            .filter(|s| is_drawn(s.graphic))
            .max_by_key(|s| s.z);
        match top {
            Some(item) => radar.item(item.graphic),
            None => radar.land(map.column(x, y).land_id),
        }
    }

    /// The height of the land of one tile, as the classic client reads it
    /// for a target of a place. None past the edge of the map.
    pub fn land_z(&mut self, map_index: u8, x: u16, y: u16) -> Option<i8> {
        let uopath = &self.uopath;
        let map = self
            .maps
            .entry(map_index)
            .or_insert_with(|| MulMap::open(uopath, map_index).ok())
            .as_ref()?;
        map.in_bounds(x, y)
            .then(|| map.column(x, y).land.north_west)
    }

    /// The pixels of a gump picture as the files hold them, for a gump that
    /// draws into its own picture, as the minimap does.
    pub fn gump_pixels(&self, gump: u16) -> Option<ArtPixels> {
        self.gumps.as_ref()?.gump(gump)
    }

    /// The pieces of a house or a boat.
    pub fn multi_pieces(&self, multi_id: u16) -> &[MultiPiece] {
        self.multis
            .as_ref()
            .map_or(&[], |multis| multis.pieces(multi_id))
    }

    /// The land tile and the item that show in a season. In winter the
    /// grass shows as snow, and in autumn a green tree shows as a brown one.
    pub fn season_land(&self, season: u8, land_id: u16) -> u16 {
        self.seasons.land(season, land_id)
    }

    pub fn season_item(&self, season: u8, graphic: u16) -> u16 {
        self.seasons.item(season, graphic)
    }

    /// The picture an item shows now. A fire or a fountain goes through
    /// the pictures of its cycle.
    pub fn shown_graphic(&self, map_index: u8, graphic: u16, time_ms: u64) -> u16 {
        match &self.cycles {
            Some(cycles) if self.item_flags(map_index, graphic) & TILE_ANIMATED != 0 => {
                cycles.graphic_at(graphic, time_ms)
            }
            _ => graphic,
        }
    }

    /// The color of words written in a hue.
    pub fn text_rgb(&self, hue: u16) -> Option<[u8; 3]> {
        self.hues.as_ref()?.text_rgb(hue)
    }

    /// The action that shows a deed of a mobile, such as a swing or a cast.
    pub fn deed_action(&self, look: &WatchLook, deed: Deed) -> Option<Action> {
        self.anim
            .as_ref()?
            .deed_action(look.body, deed, figure::is_mounted(look))
    }

    /// The action of a mobile for the way he holds himself. A person on
    /// foot in war mode stands ready, and one with a weapon walks armed.
    pub fn stance_action(&self, look: &WatchLook, action: Action) -> Action {
        let Some(anim) = self.anim.as_ref().filter(|_| !figure::is_mounted(look)) else {
            return action;
        };
        let armed = look
            .equipment
            .iter()
            .any(|item| WEAPON_LAYERS.contains(&item.layer));
        let stance = Stance {
            armed,
            war: look.war,
        };
        anim.stance_action(look.body, action, stance)
    }

    /// The picture of a fallen body: the last picture of its death.
    pub fn corpse_sprite(
        &self,
        atlas: &mut Atlas,
        map_index: u8,
        look: &WatchLook,
        paint: figure::Paint,
    ) -> Option<Sprite> {
        let action = self.deed_action(look, Deed::Die)?;
        let source = figure::Source {
            anim: self.anim.as_ref()?,
            hues: self.hues.as_ref(),
            tiledata: self.tiledata(map_index)?,
            cache: &self.frames,
        };
        let last = source.cycle(look, action).saturating_sub(1);
        self.figure_sprite(atlas, map_index, look, Pose { action, tick: last }, paint)
    }

    /// True when the body is a person: a human, an elf, a gargoyle.
    pub fn is_person(&self, body: u16) -> bool {
        self.anim.as_ref().is_some_and(|anim| anim.is_person(body))
    }

    /// The picture of one mobile in one pose: his body, his mount and what
    /// he wears, painted as `paint` says.
    pub fn figure_sprite(
        &self,
        atlas: &mut Atlas,
        map_index: u8,
        look: &WatchLook,
        pose: Pose,
        paint: figure::Paint,
    ) -> Option<Sprite> {
        let source = figure::Source {
            anim: self.anim.as_ref()?,
            hues: self.hues.as_ref(),
            tiledata: self.tiledata(map_index)?,
            cache: &self.frames,
        };
        // Two ticks that show the same frame share one picture.
        let pose = Pose {
            tick: pose.tick % source.cycle(look, pose.action),
            ..pose
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (look, pose, paint).hash(&mut hasher);
        atlas.sprite(ArtKey::Figure(hasher.finish()), || {
            figure::compose(&source, look, pose, paint)
        })
    }
}

fn picture_of(art: &ArtPixels, ramp: Option<uoterm_nav::HueRamp<'_>>) -> Picture {
    Picture {
        width: art.width,
        height: art.height,
        rgba: art.rgba(ramp),
        anchor: Vec2::ZERO,
    }
}

fn text_picture(text: TextPicture) -> Picture {
    Picture {
        width: text.width,
        height: text.height,
        rgba: text.rgba,
        anchor: Vec2::ZERO,
    }
}

/// Puts a black border round the edge of a picture.
fn draw_border(picture: &mut Picture) {
    let (width, height) = (picture.width, picture.height);
    for y in 0..height {
        for x in 0..width {
            if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
                let at = (y * width + x) * RGBA_BYTES;
                picture.rgba[at..at + RGBA_BYTES].copy_from_slice(&BORDER_RGBA);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_protocol::types::Direction;

    #[test]
    fn flat_land_has_no_normal_and_a_slope_leans_away_from_its_high_side() {
        assert!(land_normal(5, 5, 5, 5, 5).is_none());
        // The tile on the left is higher: the normal leans away from it.
        let normal = land_normal(0, 0, 0, 0, 10).unwrap();
        let length: f32 = normal.iter().map(|c| c * c).sum::<f32>().sqrt();
        assert!((length - 1.0).abs() < 1e-5);
        assert!(normal[2] > 0.0, "it points up");
        assert!(normal[0] > 0.0, "it points away from the high left side");
    }

    #[test]
    fn a_slope_counts_as_the_middle_of_its_flatter_diagonal() {
        // Top 0 and bottom 10 differ by more than left 4 and right 6.
        assert_eq!(average_z([0, 6, 10, 4]), 5);
        assert_eq!(average_z([2, 0, 4, 20]), 3);
    }

    #[test]
    fn a_cave_wall_gets_a_black_border() {
        let mut picture = Picture {
            width: 3,
            height: 3,
            rgba: vec![u8::MAX; 3 * 3 * RGBA_BYTES],
            anchor: Vec2::ZERO,
        };
        draw_border(&mut picture);
        let pixel = |x: usize, y: usize| &picture.rgba[(y * 3 + x) * RGBA_BYTES..][..RGBA_BYTES];
        assert_eq!(pixel(0, 0), BORDER_RGBA);
        assert_eq!(pixel(2, 1), BORDER_RGBA);
        assert_eq!(pixel(1, 1), [u8::MAX; RGBA_BYTES]);
    }

    #[test]
    fn real_files_give_words_pointers_and_slopes() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        const MINOC_MOUNTAINS: (u16, u16) = (2450, 400);
        const SCAN: u16 = 200;
        const TEXTURE_SIDE_MAX: usize = 8192;
        let ctx = eframe::egui::Context::default();
        // A context with no screen allows small textures only until it is told
        // what the screen allows.
        let screen = eframe::egui::RawInput {
            max_texture_side: Some(TEXTURE_SIDE_MAX),
            ..Default::default()
        };
        let _ = ctx.run(screen, |_| {});
        let mut atlas = Atlas::new(&ctx);
        let mut client = ClientArt::open(&dir).unwrap();
        let look = TextLook::unicode(1, 0x0035).bordered();
        let words = client.text_sprite(&mut atlas, "Hail", look).unwrap();
        assert!(words.width > 0.0 && words.height > 0.0);
        let wrapped = client
            .text_sprite(&mut atlas, "Hail", look.wrap(200))
            .unwrap();
        assert_eq!(wrapped.width, words.width, "short words take no more room");
        let arrow = CursorShape::Walk(Direction::North);
        let pointer = client.cursor_sprite(&mut atlas, arrow, false, 0).unwrap();
        assert!(pointer.anchor.x < pointer.width && pointer.anchor.y < pointer.height);
        const FELUCCA: u8 = 0;
        let slopes = (0..SCAN)
            .filter(|dx| {
                client
                    .cell(FELUCCA, MINOC_MOUNTAINS.0 + dx, MINOC_MOUNTAINS.1)
                    .is_some_and(|cell| cell.stretch.is_some())
            })
            .count();
        assert!(slopes > 0, "hills have stretched land");
        client.load_light_shape(1);
        assert!(client.light_shape(1).is_some());
    }

    #[test]
    fn no_draw_graphics_are_left_out() {
        const BARREL: u16 = 0x0E77;
        assert!(is_drawn(BARREL));
        assert!(!is_drawn(NO_DRAW_GRAPHICS[0]));
        assert!(!is_drawn(*NO_DRAW_RANGE.start()));
    }

    /// A block an UltimaLive shard changed is drawn in its new form.
    #[test]
    fn a_live_block_changes_the_cells_drawn() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        const TRAMMEL: u8 = 1;
        const RAISED: i8 = 60;
        let (x, y) = (3484u16, 2570u16);
        let mut client = ClientArt::open(&dir).unwrap();
        assert!(client.cell(TRAMMEL, x, y).is_some(), "the map opens");
        let high = client.tiledata(TRAMMEL).unwrap().blocks_high();
        let block = u32::from(x / 8) * u32::from(high) + u32::from(y / 8);
        let mut land = Vec::new();
        for _ in 0..64 {
            land.extend_from_slice(&3u16.to_le_bytes());
            land.push(RAISED as u8);
        }
        let live = WatchLiveMap {
            map: TRAMMEL,
            revision: 1,
            blocks: vec![crate::view::WatchLiveBlock {
                block,
                changed: 1,
                land: Some(land),
                statics: Some(Vec::new()),
            }],
        };
        client.take_live_map(&live);
        let cell = client.cell(TRAMMEL, x, y).unwrap();
        assert_eq!(cell.corners[0], RAISED);
        assert!(cell.statics.is_empty(), "the bank is gone from the block");
    }

    #[test]
    fn real_files_give_the_cell_under_a_bank() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        const TRAMMEL: u8 = 1;
        let mut client = ClientArt::open(&dir).unwrap();
        let cell = client.cell(TRAMMEL, 3484, 2570).unwrap();
        assert!(!cell.statics.is_empty());
    }
}
