//! The client files as the map needs them: what lies on each tile, and the
//! picture of each thing. The window and the web server read the files
//! themselves, so the session sends no pictures. Each picture is made on
//! the CPU, hues and all, as RGBA bytes.

use super::facet_maps::FacetMaps;
use super::figure::{self, FrameCache, Source};
use super::text::UoFonts;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use uoterm_nav::{
    land_is_ignored, shown_graphic, Action, AnimData, AnimRules, ArtCycles, ArtData, ArtPixels,
    ClilocData, CursorSet, GumpArt, HueData, HueRamp, ItemTile, LandTile, LightData, LightShape,
    MulMap, MultiData, MultiPiece, RadarColors, RadarTables, SeasonArt, TexmapData, TextPicture,
    TileData, TileFlagSet, TileQuery, GUMP_MAX_SIDE, TILE_PARTIAL_HUE,
};
use uoterm_view::art::{
    is_drawn, mount_item, radar_item, ArtRequest, Cell, CellStatic, GumpMask, MapBlockAt, Picture,
    Stretch, TextLook, TextMeasure,
};
use uoterm_view::frame::{WatchLiveMap, WatchLook, WatchMap};
use uoterm_view::geom::Vector;
use uoterm_view::map_lay::{map_tile_pixels, near_pixels, MAP_TILE_SIDE, SPAN};
use uoterm_view::model::map_item::land_rgba;

/// How many tiles the window remembers. A full window shows about four
/// thousand, so this is a few windows of walking.
const CELL_CACHE_CAP: usize = 24_000;
/// The side of a block of the map, in tiles.
const BLOCK_SIDE: u16 = 8;

/// One unit of height is this many pixels, as the normals of the land are
/// worked out.
const NORMAL_Z_PIXELS: f32 = 4.0;
/// Half the side of a tile, as the normals of the land are worked out.
const NORMAL_HALF_TILE: f32 = 22.0;
/// The sides of a black border round a cave wall.
const BORDER_RGBA: [u8; 4] = [0, 0, 0, u8::MAX];
const RGBA_BYTES: usize = 4;
/// The most characters words in a picture may have. A label, a line of
/// speech or a page of a gump is far shorter.
pub const TEXT_MAX_CHARS: usize = 4096;
/// The widest and the tallest a picture of words may be: as large as the
/// largest picture the client files hold.
pub const PICTURE_MAX_SIDE: usize = GUMP_MAX_SIDE;

/// The most worn items a figure may list: one for each layer number a
/// packet can name. A mobile wears one item on each layer at most.
pub const FIGURE_MAX_EQUIPMENT: usize = u8::MAX as usize + 1;

/// A request for a picture larger than any the client shows. A web page
/// can send any request, and a picture of gigabytes would stop UOTerm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtTooLarge;

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

pub struct ClientArt {
    uopath: PathBuf,
    art: ArtData,
    /// None when the client files hold no classic animation files.
    anim: Option<AnimData>,
    frames: FrameCache,
    hues: Option<HueData>,
    /// None when the client files hold no picture cycles.
    cycles: Option<Arc<ArtCycles>>,
    /// None when the client files hold no houses and boats.
    multis: Option<MultiData>,
    /// None when the client files hold no gump pictures.
    gumps: Option<GumpArt>,
    /// The art that each season swaps.
    seasons: Arc<SeasonArt>,
    /// None when the client files hold no colors for a world map.
    radar: Option<RadarColors>,
    /// The map files, with the UltimaLive blocks of the window's session.
    maps: FacetMaps,
    cells: HashMap<(u8, u16, u16), Cell>,
    /// None when the client files hold no tiledata.
    tiles: Option<Arc<TileData>>,
    /// None when the client files hold no land textures.
    textures: Option<TexmapData>,
    /// None when the client files hold no light shapes.
    lights: Option<LightData>,
    light_shapes: HashMap<u8, Option<LightShape>>,
    /// None when the client files hold no UO fonts.
    fonts: Option<UoFonts>,
    cursors: CursorSet,
    /// The text database, read the first time it is asked for: it is
    /// large, and the window does not read it. None inside when the client
    /// files hold none.
    cliloc: OnceLock<Option<Arc<ClilocData>>>,
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
            cycles: ArtCycles::open(uopath).ok().map(Arc::new),
            multis: MultiData::open(uopath).ok(),
            radar: RadarColors::open(uopath).ok(),
            gumps: GumpArt::open(uopath).ok(),
            seasons: Arc::new(SeasonArt::open(&uoterm_runtime::config::config_dir())),
            maps: FacetMaps::default(),
            cells: HashMap::new(),
            tiles: TileData::open(uopath).ok().map(Arc::new),
            textures: TexmapData::open(uopath).ok(),
            lights: LightData::open(uopath).ok(),
            light_shapes: HashMap::new(),
            fonts: UoFonts::open(uopath).ok(),
            cliloc: OnceLock::new(),
        })
    }

    /// Lays the map blocks an UltimaLive shard changed over the map files
    /// of the window's session. The tiles read before a change are read
    /// anew. Gives the blocks laid now.
    pub fn take_live_map(&mut self, live: &WatchLiveMap) -> Vec<MapBlockAt> {
        let laid = self.maps.take_live_map(&self.uopath, live);
        if !laid.is_empty() {
            self.cells.retain(|(map, _, _), _| *map != live.map);
        }
        laid
    }

    /// One tile of a facet. The tiles are read a block at a time.
    pub fn cell(&mut self, map_index: u8, x: u16, y: u16) -> Option<&Cell> {
        let key = (map_index, x, y);
        if !self.cells.contains_key(&key) {
            let (block_x, block_y) = (x / BLOCK_SIDE, y / BLOCK_SIDE);
            let block = self.cell_block(None, map_index, block_x, block_y);
            if self.cells.len() + block.len() > CELL_CACHE_CAP {
                self.cells.clear();
            }
            let tiles =
                (0..BLOCK_SIDE).flat_map(|row| (0..BLOCK_SIDE).map(move |column| (column, row)));
            for ((column, row), cell) in tiles.zip(block) {
                let at = (
                    map_index,
                    block_x * BLOCK_SIDE + column,
                    block_y * BLOCK_SIDE + row,
                );
                self.cells.insert(at, cell);
            }
        }
        self.cells.get(&key)
    }

    /// The 64 tiles of one block of a facet, row by row from its north west
    /// corner, with the UltimaLive blocks of `live` when it is given.
    /// Empty past the edge of the map, or with no map files.
    pub fn cell_block(
        &mut self,
        live: Option<&FacetMaps>,
        map_index: u8,
        block_x: u16,
        block_y: u16,
    ) -> Vec<Cell> {
        let Some(map) = self.maps.facet_under(live, &self.uopath, map_index) else {
            return Vec::new();
        };
        let (Some(left), Some(top)) = (
            block_x.checked_mul(BLOCK_SIDE),
            block_y.checked_mul(BLOCK_SIDE),
        ) else {
            return Vec::new();
        };
        if !map.in_bounds(left, top) {
            return Vec::new();
        }
        let tiles = self.tiles.as_deref();
        (0..BLOCK_SIDE)
            .flat_map(|row| (0..BLOCK_SIDE).map(move |column| (left + column, top + row)))
            .map(|(x, y)| read_cell(map, tiles, x, y))
            .collect()
    }

    /// The picture a request asks for, made from the client files. None
    /// when the files do not hold it, or when it is too large.
    pub fn picture(&self, request: &ArtRequest) -> Option<Picture> {
        self.checked_picture(request).ok().flatten()
    }

    /// The picture a request asks for, or why it is refused. Words have a
    /// size the request sets, and a figure a list of worn items: every
    /// other picture is as large as the client files make it, and they hold
    /// none too large. Ok(None) when the files do not hold the picture.
    pub fn checked_picture(&self, request: &ArtRequest) -> Result<Option<Picture>, ArtTooLarge> {
        match request {
            ArtRequest::Text { text, look } => self.check_words(text, look)?,
            ArtRequest::Figure { look, .. } if look.equipment.len() > FIGURE_MAX_EQUIPMENT => {
                return Err(ArtTooLarge)
            }
            _ => {}
        }
        Ok(self.make_picture(request))
    }

    /// Refuses words with too many characters, a width too large, or lines
    /// that would make a picture too large.
    fn check_words(&self, text: &str, look: &TextLook) -> Result<(), ArtTooLarge> {
        let too_wide = look
            .width
            .is_some_and(|width| width as usize > PICTURE_MAX_SIDE);
        if too_wide || text.chars().count() > TEXT_MAX_CHARS {
            return Err(ArtTooLarge);
        }
        match self.fonts.as_ref() {
            Some(fonts) if !fits_a_picture(fonts, text, &fitted(fonts, text, *look)) => {
                Err(ArtTooLarge)
            }
            _ => Ok(()),
        }
    }

    fn make_picture(&self, request: &ArtRequest) -> Option<Picture> {
        match request {
            ArtRequest::Land { land_id, hue } => {
                let art = self.art.land(*land_id)?;
                Some(picture_of(&art, self.ramp(*hue, false)))
            }
            ArtRequest::Texture { texture_id, hue } => {
                let texture = self.textures.as_ref()?.texture(*texture_id)?;
                Some(picture_of(&texture, self.ramp(*hue, false)))
            }
            ArtRequest::Item {
                graphic,
                hue,
                whole_hue,
                border,
            } => {
                let art = self.art.item(*graphic)?;
                let partial = !whole_hue && self.item_flags(*graphic) & TILE_PARTIAL_HUE != 0;
                let mut picture = picture_of(&art, self.ramp(*hue, partial));
                if *border {
                    draw_border(&mut picture);
                }
                Some(picture)
            }
            ArtRequest::Gump { gump, hue, partial } => {
                let art = self.gumps.as_ref()?.gump(*gump)?;
                Some(picture_of(&art, self.ramp(*hue, *partial)))
            }
            ArtRequest::Cursor { shape, war, hue } => {
                let cursor = self.cursors.get(*shape, *war)?;
                Some(Picture {
                    anchor: Vector::new(cursor.hot_x as f32, cursor.hot_y as f32),
                    ..picture_of(&cursor.pixels, self.ramp(*hue, false))
                })
            }
            ArtRequest::Text { text, look } => {
                let fonts = self.fonts.as_ref()?;
                let picture = fonts.render(text, &fitted(fonts, text, *look))?;
                Some(text_picture(picture))
            }
            ArtRequest::Figure { look, pose, paint } => {
                figure::compose(&self.figure_source()?, look, *pose, *paint)
            }
        }
    }

    fn ramp(&self, hue: u16, partial: bool) -> Option<HueRamp<'_>> {
        self.hues.as_ref()?.ramp(hue, partial)
    }

    /// Where the pictures of figures come from. None when the client files
    /// hold no classic animation files or no tiledata.
    fn figure_source(&self) -> Option<Source<'_>> {
        Some(Source {
            anim: self.anim.as_ref()?,
            hues: self.hues.as_ref(),
            tiledata: self.tiles.as_deref()?,
            cache: &self.frames,
        })
    }

    /// How many frames the body of this look has for an action.
    pub fn frame_count(&self, look: &WatchLook, action: Action) -> Option<usize> {
        let mounted = mount_item(look).is_some();
        self.body_frame_count(look.body, look.direction, action, mounted)
    }

    /// How many frames a body has for an action, facing `direction` and on
    /// a mount or not. None when the client files hold no animation files.
    pub fn body_frame_count(
        &self,
        body: u16,
        direction: u8,
        action: Action,
        mounted: bool,
    ) -> Option<usize> {
        Some(
            self.figure_source()?
                .cycle(body, direction, action, mounted),
        )
    }

    /// The tables of the animation files that hold no pictures.
    pub fn anim_rules(&self) -> Option<&AnimRules> {
        self.anim.as_ref().map(AnimData::rules)
    }

    /// The folder of the client files.
    pub fn uopath(&self) -> &Path {
        &self.uopath
    }

    /// The whole tiledata file. None when the client files hold none.
    pub fn tiledata_tables(&self) -> Option<Arc<TileData>> {
        self.tiles.clone()
    }

    /// Every color of a map of the world. None when the client files hold
    /// none.
    pub fn radar_tables(&self) -> Option<RadarTables> {
        self.radar.as_ref().map(RadarColors::tables)
    }

    /// The art each season swaps.
    pub fn season_tables(&self) -> Arc<SeasonArt> {
        Arc::clone(&self.seasons)
    }

    /// The text database of the client files, read the first time it is
    /// asked for. None when the client files hold none.
    pub fn cliloc_table(&self) -> Option<Arc<ClilocData>> {
        self.cliloc
            .get_or_init(|| ClilocData::open(&self.uopath).ok().map(Arc::new))
            .clone()
    }

    /// The picture cycles of the items. None when the client files hold
    /// none.
    pub fn art_cycles_table(&self) -> Option<Arc<ArtCycles>> {
        self.cycles.clone()
    }

    /// The tiledata record of an item graphic.
    pub fn item_tile(&self, graphic: u16) -> Option<&ItemTile> {
        self.tiles.as_ref()?.item(graphic)
    }

    /// The tiledata record of a land tile.
    pub fn land_tile(&self, land_id: u16) -> Option<&LandTile> {
        self.tiles.as_ref()?.land(land_id)
    }

    /// One light shape of the light files, read the first time it is
    /// asked for.
    pub fn light_shape(&mut self, shape: u8) -> Option<&LightShape> {
        let lights = self.lights.as_ref();
        self.light_shapes
            .entry(shape)
            .or_insert_with(|| lights.and_then(|lights| lights.shape(shape)))
            .as_ref()
    }

    /// The height of one line of words in a UO font.
    pub fn line_height(&self, look: &TextLook) -> Option<u32> {
        Some(self.fonts.as_ref()?.line_height(look))
    }

    /// The lines words break into in a UO font, as the words over heads of
    /// the classic client wrap. Empty when the files hold no UO fonts.
    pub fn text_lines(&self, text: &str, look: &TextLook) -> Vec<String> {
        self.fonts
            .as_ref()
            .map(|fonts| fonts.lines(text, &fitted(fonts, text, *look)))
            .unwrap_or_default()
    }

    /// The lines of words and the height of one, for a web page that lays
    /// out words it draws. Refused as a picture of the words is. Ok(None)
    /// when the files hold no UO fonts.
    pub fn measured_text(
        &self,
        text: &str,
        look: &TextLook,
    ) -> Result<Option<TextMeasure>, ArtTooLarge> {
        self.check_words(text, look)?;
        Ok(self.line_height(look).map(|line_height| TextMeasure {
            lines: self.text_lines(text, look),
            line_height,
        }))
    }

    fn item_flags(&self, graphic: u16) -> u32 {
        self.item_tile(graphic)
            .map_or(0, |tile| tile.flags.low_bits())
    }

    /// True when the client files hold the pictures of gumps.
    pub fn has_gump_art(&self) -> bool {
        self.gumps.is_some()
    }

    /// True when the gump draws the pixel at `x`, `y` of its picture, for
    /// a click that must fall on what a gump shows, not on its box.
    pub fn gump_drawn_at(&self, gump: u16, x: usize, y: usize) -> bool {
        self.gumps
            .as_ref()
            .and_then(|gumps| gumps.gump(gump))
            .is_some_and(|art| art.is_drawn(x, y))
    }

    /// The picture of the land round `middle` in its radar colors, as the
    /// map of a start town shows it. None when no tile round it has a
    /// color.
    pub fn near_picture(
        &mut self,
        live: Option<&FacetMaps>,
        map_index: u8,
        middle: (u16, u16),
    ) -> Option<Picture> {
        let pixels = near_pixels(middle, |x, y| self.radar_rgb_under(live, map_index, x, y))?;
        Some(Picture {
            width: SPAN,
            height: SPAN,
            rgba: pixels.iter().flat_map(|pixel| pixel.to_array()).collect(),
            anchor: Vector::ZERO,
        })
    }

    /// One tile of the whole-world picture of a map, in its radar colors,
    /// as the world map of a web page lays it. None for a tile past the
    /// picture, or with no known land.
    pub fn map_tile_picture(
        &mut self,
        live: Option<&FacetMaps>,
        map_index: u8,
        tile: (usize, usize),
    ) -> Option<Picture> {
        let pixels = map_tile_pixels(map_index, tile, |x, y| {
            self.radar_rgb_under(live, map_index, x, y)
        })?;
        Some(Picture {
            width: MAP_TILE_SIDE,
            height: MAP_TILE_SIDE,
            rgba: pixels.iter().flat_map(|pixel| pixel.to_array()).collect(),
            anchor: Vector::ZERO,
        })
    }

    /// The land of a map item in its radar colors. None when no tile of it
    /// has a color.
    pub fn map_item_picture(
        &mut self,
        live: Option<&FacetMaps>,
        map: &WatchMap,
    ) -> Option<Picture> {
        let (width, height, rgba) =
            land_rgba(map, |facet, x, y| self.radar_rgb_under(live, facet, x, y))?;
        Some(Picture {
            width,
            height,
            rgba,
            anchor: Vector::ZERO,
        })
    }

    /// The color of one tile on a map of the world: the color of its
    /// highest item, or of its land. None past the edge of the map.
    pub fn radar_rgb(&mut self, map_index: u8, x: u16, y: u16) -> Option<[u8; 3]> {
        self.radar_rgb_under(None, map_index, x, y)
    }

    /// [`ClientArt::radar_rgb`] with the UltimaLive blocks of `live` when
    /// it is given.
    fn radar_rgb_under(
        &mut self,
        live: Option<&FacetMaps>,
        map_index: u8,
        x: u16,
        y: u16,
    ) -> Option<[u8; 3]> {
        let radar = self.radar.as_ref()?;
        let map = self.maps.facet_under(live, &self.uopath, map_index)?;
        if !map.in_bounds(x, y) {
            return None;
        }
        let statics = map.statics_at(x, y);
        match radar_item(statics.iter().map(|s| (s.graphic, s.z))) {
            Some(graphic) => radar.item(graphic),
            None => radar.land(map.column(x, y).land_id),
        }
    }

    /// The height of the land of one tile, as the classic client reads it
    /// for a target of a place. None past the edge of the map.
    pub fn land_z(&mut self, map_index: u8, x: u16, y: u16) -> Option<i8> {
        let map = self.maps.facet(&self.uopath, map_index)?;
        map.in_bounds(x, y)
            .then(|| map.column(x, y).land.north_west)
    }

    /// The pixels a gump draws, for a web page that must know where a
    /// click falls on what a gump shows. A gump of the files is never wider
    /// or taller than `GUMP_MAX_SIDE`, so its mask is small.
    pub fn gump_mask(&self, gump: u16) -> Option<GumpMask> {
        let art = self.gump_pixels(gump)?;
        Some(GumpMask::from_drawn(art.width, art.height, |x, y| {
            art.is_drawn(x, y)
        }))
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
    pub fn shown_graphic(&self, graphic: u16, time_ms: u64) -> u16 {
        shown_graphic(
            self.cycles.as_deref(),
            self.item_flags(graphic),
            graphic,
            time_ms,
        )
    }

    /// The color of words written in a hue.
    pub fn text_rgb(&self, hue: u16) -> Option<[u8; 3]> {
        self.hues.as_ref()?.text_rgb(hue)
    }
}

/// One tile as the window draws it, from the map files of its facet.
fn read_cell(map: &MulMap, tiles: Option<&TileData>, x: u16, y: u16) -> Cell {
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
    let land = tiles.and_then(|tiles| tiles.land(column.land_id));
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
    Cell {
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
    }
}

/// True when the words of a look make a picture no wider and no taller
/// than [`PICTURE_MAX_SIDE`]. The lines are measured as the font draws
/// them, before any is drawn.
fn fits_a_picture(fonts: &UoFonts, text: &str, look: &TextLook) -> bool {
    let block = fonts.measure(text, look);
    block.width as usize <= PICTURE_MAX_SIDE && block.height as usize <= PICTURE_MAX_SIDE
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

fn picture_of(art: &ArtPixels, ramp: Option<HueRamp<'_>>) -> Picture {
    Picture {
        width: art.width,
        height: art.height,
        rgba: art.rgba(ramp),
        anchor: Vector::ZERO,
    }
}

fn text_picture(text: TextPicture) -> Picture {
    Picture {
        width: text.width,
        height: text.height,
        rgba: text.rgba,
        anchor: Vector::ZERO,
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
    use uoterm_nav::fixtures::{write_two_items, TWO_ITEMS_RED};
    use uoterm_nav::CursorShape;
    use uoterm_protocol::types::Direction;

    #[test]
    fn an_item_request_gives_its_picture_and_a_missing_one_none() {
        let dir = std::env::temp_dir().join(format!("uoterm-client-art-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_two_items(&dir);
        let client = ClientArt::open(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let item = |graphic| ArtRequest::Item {
            graphic,
            hue: 0,
            whole_hue: false,
            border: false,
        };
        let picture = client.picture(&item(1)).unwrap();
        assert_eq!((picture.width, picture.height), (2, 1));
        assert_eq!(
            &picture.rgba[..RGBA_BYTES],
            &[0; RGBA_BYTES],
            "a clear first pixel"
        );
        let red = ArtPixels {
            width: 1,
            height: 1,
            colors: vec![TWO_ITEMS_RED],
        };
        assert_eq!(&picture.rgba[RGBA_BYTES..], red.rgba(None).as_slice());
        assert!(client.picture(&item(2)).is_none(), "no entry");
    }

    fn words(text: String, look: TextLook) -> ArtRequest {
        ArtRequest::Text { text, look }
    }

    #[test]
    fn words_too_long_or_too_wide_for_a_picture_are_refused() {
        let dir =
            std::env::temp_dir().join(format!("uoterm-client-art-words-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_two_items(&dir);
        let client = ClientArt::open(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let plain = TextLook::unicode(1, 0);
        let too_long = words("a".repeat(TEXT_MAX_CHARS + 1), plain);
        assert_eq!(client.checked_picture(&too_long), Err(ArtTooLarge));
        assert!(client.picture(&too_long).is_none());
        let too_wide = PICTURE_MAX_SIDE as u32 + 1;
        for look in [plain.cropped(too_wide), plain.wrap(too_wide)] {
            assert_eq!(
                client.checked_picture(&words("Hail".into(), look)),
                Err(ArtTooLarge)
            );
        }
        assert_eq!(
            client.checked_picture(&words("Hail".into(), plain)),
            Ok(None),
            "words that fit, with no fonts in the files"
        );
    }

    #[test]
    fn words_are_measured_with_the_font_before_they_are_drawn() {
        let dir =
            std::env::temp_dir().join(format!("uoterm-client-art-fonts-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_two_items(&dir);
        uoterm_nav::fixtures::write_small_fonts(&dir);
        let client = ClientArt::open(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let ascii = TextLook::ascii(0, 0);
        let hail = client
            .checked_picture(&words("Hail".into(), ascii))
            .unwrap();
        assert!(hail.is_some(), "short words are drawn");
        let one_long_line = words("a".repeat(TEXT_MAX_CHARS), ascii);
        assert_eq!(client.checked_picture(&one_long_line), Err(ArtTooLarge));
        let many_lines = words("\n".repeat(TEXT_MAX_CHARS), ascii);
        assert_eq!(client.checked_picture(&many_lines), Err(ArtTooLarge));
    }

    #[test]
    fn a_figure_with_more_worn_items_than_layers_is_refused() {
        let dir =
            std::env::temp_dir().join(format!("uoterm-client-art-figure-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_two_items(&dir);
        let client = ClientArt::open(&dir).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        let figure = |count: usize| ArtRequest::Figure {
            look: WatchLook {
                equipment: vec![uoterm_view::frame::WatchEquip::default(); count],
                ..WatchLook::default()
            },
            pose: uoterm_view::art::Pose {
                action: Action::Stand,
                tick: 0,
            },
            paint: uoterm_view::art::Paint {
                outline: [0; RGBA_BYTES],
                whole_hue: None,
            },
        };
        assert_eq!(
            client.checked_picture(&figure(FIGURE_MAX_EQUIPMENT + 1)),
            Err(ArtTooLarge)
        );
        assert_eq!(
            client.checked_picture(&figure(FIGURE_MAX_EQUIPMENT)),
            Ok(None),
            "no animation files"
        );
    }

    #[test]
    fn a_block_past_the_last_block_a_map_can_have_is_empty() {
        const FIRST_TOO_FAR: u16 = u16::MAX / BLOCK_SIDE + 1;
        let dir =
            std::env::temp_dir().join(format!("uoterm-client-art-far-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_two_items(&dir);
        uoterm_nav::fixtures::write_mini_client(&dir);
        let mut client = ClientArt::open(&dir).unwrap();
        assert_eq!(
            client.cell_block(None, 0, 0, 0).len(),
            usize::from(BLOCK_SIDE * BLOCK_SIDE)
        );
        assert!(client.cell_block(None, 0, FIRST_TOO_FAR, 0).is_empty());
        assert!(client.cell_block(None, 0, 0, FIRST_TOO_FAR).is_empty());
        assert!(client.cell_block(None, 0, u16::MAX, u16::MAX).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

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
            anchor: Vector::ZERO,
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
        let mut client = ClientArt::open(&dir).unwrap();
        let look = TextLook::unicode(1, 0x0035).bordered();
        let text = |look| ArtRequest::Text {
            text: "Hail".into(),
            look,
        };
        let words = client.picture(&text(look)).unwrap();
        assert!(words.width > 0 && words.height > 0);
        let wrapped = client.picture(&text(look.wrap(200))).unwrap();
        const LINE: &str = "a\n";
        let many_lines = ArtRequest::Text {
            text: LINE.repeat(TEXT_MAX_CHARS / LINE.len()),
            look,
        };
        assert_eq!(
            client.checked_picture(&many_lines),
            Err(ArtTooLarge),
            "too many lines for a picture"
        );
        assert_eq!(wrapped.width, words.width, "short words take no more room");
        assert_eq!(client.text_lines("Hail", &look.wrap(200)), ["Hail"]);
        let arrow = ArtRequest::Cursor {
            shape: CursorShape::Walk(Direction::North),
            war: false,
            hue: 0,
        };
        let pointer = client.picture(&arrow).unwrap();
        assert!(pointer.anchor.x < pointer.width as f32);
        assert!(pointer.anchor.y < pointer.height as f32);
        const FELUCCA: u8 = 0;
        let slopes = (0..SCAN)
            .filter(|dx| {
                client
                    .cell(FELUCCA, MINOC_MOUNTAINS.0 + dx, MINOC_MOUNTAINS.1)
                    .is_some_and(|cell| cell.stretch.is_some())
            })
            .count();
        assert!(slopes > 0, "hills have stretched land");
        assert!(client.light_shape(1).is_some());
    }

    #[test]
    fn real_files_give_a_block_of_cells_that_matches_each_cell() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        const TRAMMEL: u8 = 1;
        let (x, y) = (3484u16, 2570u16);
        let mut client = ClientArt::open(&dir).unwrap();
        let block = client.cell_block(None, TRAMMEL, x / BLOCK_SIDE, y / BLOCK_SIDE);
        assert_eq!(block.len(), usize::from(BLOCK_SIDE * BLOCK_SIDE));
        let at = usize::from((y % BLOCK_SIDE) * BLOCK_SIDE + x % BLOCK_SIDE);
        assert_eq!(client.cell(TRAMMEL, x, y), Some(&block[at]));
        assert!(client
            .cell_block(None, TRAMMEL, u16::MAX / BLOCK_SIDE, 0)
            .is_empty());
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
        let high = MulMap::open(&dir, TRAMMEL).unwrap().blocks_high();
        let block = u32::from(x / BLOCK_SIDE) * u32::from(high) + u32::from(y / BLOCK_SIDE);
        let mut land = Vec::new();
        for _ in 0..BLOCK_SIDE * BLOCK_SIDE {
            land.extend_from_slice(&3u16.to_le_bytes());
            land.push(RAISED as u8);
        }
        let live = WatchLiveMap {
            map: TRAMMEL,
            revision: 1,
            blocks: vec![uoterm_view::frame::WatchLiveBlock {
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
