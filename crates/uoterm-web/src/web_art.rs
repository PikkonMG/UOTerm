//! The art of the browser: the pictures and the tables of the client
//! files, which the page fetches from the server. Each answer is pending
//! until the page gives it. A picture is asked for once by its request;
//! a table once by its path. The page keeps the pixels of each picture
//! under the key given here, and copies them into the texture at the place
//! the packer gives.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use uoterm_nav::{
    shown_graphic, Action, AnimRules, ArtCycle, ArtCycles, ItemTile, LandTile, LightShape,
    MultiPiece, RadarTables, SeasonArt, TileData,
};
use uoterm_view::art::{
    is_mounted, radar_item, text_rgb_or_plain, Art, ArtRequest, Cell, GumpMask, MapBlockAt, Sprite,
    TextLook, TextMeasure, WorldArt, NO_FONT_LINE_HEIGHT,
};
use uoterm_view::atlas::{ShelfPacker, ATLAS_SIDE};
use uoterm_view::frame::{WatchLiveMap, WatchLook};
use uoterm_view::geom::{Point, Vector};

/// A block the scene has not read for this many frames is dropped. The
/// scene reads each block it draws in each frame, whatever the size of the
/// view and the zoom.
pub const BLOCK_KEEP_FRAMES: u64 = 120;
/// At most this many measures of words are kept; then they start again.
pub const MEASURES_KEPT: usize = 1024;
/// The side of a map block, in tiles, and its tiles.
const BLOCK_SIDE: u16 = 8;
const BLOCK_CELLS: usize = (BLOCK_SIDE * BLOCK_SIDE) as usize;

const TILEDATA_PATH: &str = "/v1/data/tiledata";
const RADAR_PATH: &str = "/v1/data/radarcol";
const SEASONS_PATH: &str = "/v1/data/seasons";
const CYCLES_PATH: &str = "/v1/data/animdata";
const ANIM_RULES_PATH: &str = "/v1/data/anim-rules";
const MULTI_PREFIX: &str = "/v1/data/multis/";
const LIGHT_PREFIX: &str = "/v1/data/lights/";
const FRAMES_PREFIX: &str = "/v1/data/frames/";
const TEXT_RGB_PREFIX: &str = "/v1/data/hues-text/";
const GUMP_MASK_PREFIX: &str = "/v1/gump-mask/";
const MAP_PREFIX: &str = "/v1/map/";
/// The routes a page posts to: the lines of words in a UO font, and the
/// live map of a picture.
pub const MEASURE_PATH: &str = "/v1/text/measure";
pub const LIVE_MAP_PATH: &str = "/v1/map/live";
const PATH_SEPARATOR: char = '/';

/// One picture the page gets from the server: the key it keeps the
/// pixels under, and the request it posts.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Wanted {
    pub key: String,
    pub request: ArtRequest,
}

/// One picture placed in the texture this frame. The page copies its
/// pixels, from its own cache by `key`, to `x`, `y`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Upload {
    pub key: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}

/// A body the page posts to a route, and gives the answer back by `key`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Post {
    pub key: String,
    pub path: &'static str,
    pub body: Value,
}

/// A table the server sends: on its way, here, or not in the files.
enum Table<T> {
    Asked,
    Here(T),
    Missing,
}

impl<T> Table<T> {
    fn art(&self) -> Art<&T> {
        match self {
            Self::Asked => Art::Pending,
            Self::Here(value) => Art::Ready(value),
            Self::Missing => Art::Missing,
        }
    }
}

/// A picture: asked for, here with its size, or not in the files. A
/// picture that is here has a place in the texture until the texture
/// starts again.
#[derive(Clone, Copy)]
enum Picture {
    Asked,
    Arrived {
        width: usize,
        height: usize,
        anchor: Vector,
        sprite: Option<Sprite>,
    },
    Missing,
}

/// The paths to get, each one time until it is forgotten.
#[derive(Default)]
struct Wants {
    paths: RefCell<Vec<String>>,
    asked: RefCell<HashSet<String>>,
}

impl Wants {
    fn want(&self, path: String) {
        if self.asked.borrow_mut().insert(path.clone()) {
            self.paths.borrow_mut().push(path);
        }
    }

    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.paths.borrow_mut())
    }

    /// The path may be asked for again.
    fn forget(&self, path: &str) {
        self.asked.borrow_mut().remove(path);
    }
}

/// What a post is for, by its key.
enum PostFor {
    Measure(u64),
    LiveMap,
}

/// The tables of the client files, by what they answer.
enum DataPath {
    TileData,
    Radar,
    Seasons,
    Cycles,
    AnimRules,
    Multi(u16),
    Light(u8),
    Frames(FramesKey),
    TextRgb(u16),
    GumpMask(u16),
    Block(BlockKey),
}

/// A body facing a way, doing an action, on a mount or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FramesKey {
    body: u16,
    action: Action,
    direction: u8,
    mounted: bool,
}

/// One block of a map: the map, its column and its row of blocks.
type BlockKey = (u8, u16, u16);

impl DataPath {
    fn path(&self) -> String {
        match self {
            Self::TileData => TILEDATA_PATH.to_string(),
            Self::Radar => RADAR_PATH.to_string(),
            Self::Seasons => SEASONS_PATH.to_string(),
            Self::Cycles => CYCLES_PATH.to_string(),
            Self::AnimRules => ANIM_RULES_PATH.to_string(),
            Self::Multi(id) => format!("{MULTI_PREFIX}{id}"),
            Self::Light(id) => format!("{LIGHT_PREFIX}{id}"),
            Self::Frames(key) => format!(
                "{FRAMES_PREFIX}{}/{}/{}/{}",
                key.body, key.action, key.direction, key.mounted
            ),
            Self::TextRgb(hue) => format!("{TEXT_RGB_PREFIX}{hue}"),
            Self::GumpMask(id) => format!("{GUMP_MASK_PREFIX}{id}"),
            Self::Block((map, bx, by)) => format!("{MAP_PREFIX}{map}/{bx}/{by}"),
        }
    }

    /// The table a path names. None for a path this art never asks for.
    fn of(path: &str) -> Option<Self> {
        Some(match path {
            TILEDATA_PATH => Self::TileData,
            RADAR_PATH => Self::Radar,
            SEASONS_PATH => Self::Seasons,
            CYCLES_PATH => Self::Cycles,
            ANIM_RULES_PATH => Self::AnimRules,
            _ => {
                if let Some(id) = path.strip_prefix(MULTI_PREFIX) {
                    Self::Multi(id.parse().ok()?)
                } else if let Some(id) = path.strip_prefix(LIGHT_PREFIX) {
                    Self::Light(id.parse().ok()?)
                } else if let Some(rest) = path.strip_prefix(FRAMES_PREFIX) {
                    let parts: Vec<&str> = rest.split(PATH_SEPARATOR).collect();
                    let [body, action, direction, mounted] = parts.as_slice() else {
                        return None;
                    };
                    Self::Frames(FramesKey {
                        body: body.parse().ok()?,
                        action: action.parse().ok()?,
                        direction: direction.parse().ok()?,
                        mounted: mounted.parse().ok()?,
                    })
                } else if let Some(hue) = path.strip_prefix(TEXT_RGB_PREFIX) {
                    Self::TextRgb(hue.parse().ok()?)
                } else if let Some(id) = path.strip_prefix(GUMP_MASK_PREFIX) {
                    Self::GumpMask(id.parse().ok()?)
                } else if let Some(rest) = path.strip_prefix(MAP_PREFIX) {
                    let parts: Vec<&str> = rest.split(PATH_SEPARATOR).collect();
                    let [map, bx, by] = parts.as_slice() else {
                        return None;
                    };
                    Self::Block((map.parse().ok()?, bx.parse().ok()?, by.parse().ok()?))
                } else {
                    return None;
                }
            }
        })
    }
}

/// The mask of a gump as the server sends it, with its bits in base64.
#[derive(Deserialize)]
struct SentMask {
    width: usize,
    height: usize,
    bits: String,
}

/// The block of a tile.
fn block_of(map: u8, x: u16, y: u16) -> BlockKey {
    (map, x / BLOCK_SIDE, y / BLOCK_SIDE)
}

/// The place of a tile in the cells of its block, row by row.
fn cell_index(x: u16, y: u16) -> usize {
    usize::from(y % BLOCK_SIDE) * usize::from(BLOCK_SIDE) + usize::from(x % BLOCK_SIDE)
}

/// The key of words measured in a look.
fn measure_key(text: &str, look: &TextLook) -> u64 {
    ArtRequest::Text {
        text: text.to_string(),
        look: *look,
    }
    .key()
}

pub struct WebArt {
    packer: ShelfPacker,
    pictures: HashMap<u64, Picture>,
    /// The pictures the scene asked for since the texture last started
    /// again.
    used: HashSet<u64>,
    /// The pictures forgotten, whose pixels the page may drop.
    forgotten: Vec<String>,
    wanted: Vec<Wanted>,
    uploads: Vec<Upload>,
    /// Every picture lost its place this frame: the page clears its
    /// texture and lays the white square again.
    atlas_reset: bool,
    wants: Wants,
    tiles: Option<Table<TileData>>,
    radar: Option<Table<RadarTables>>,
    seasons: Option<Table<SeasonArt>>,
    cycles: Option<Table<ArtCycles>>,
    anim: Option<Table<AnimRules>>,
    multis: HashMap<u16, Table<Vec<MultiPiece>>>,
    lights: HashMap<u8, Table<LightShape>>,
    frames: HashMap<FramesKey, Table<usize>>,
    text_colors: HashMap<u16, Table<[u8; 3]>>,
    masks: HashMap<u16, Table<GumpMask>>,
    blocks: HashMap<BlockKey, Table<Vec<Cell>>>,
    /// The frame each block was last read in.
    block_read: HashMap<BlockKey, u64>,
    /// The frames that ended.
    frames_ended: u64,
    measures: RefCell<HashMap<u64, Table<TextMeasure>>>,
    /// The words to measure, by their key.
    to_measure: RefCell<Vec<(u64, String, TextLook)>>,
    posts: Vec<Post>,
    posted: HashMap<u64, PostFor>,
    next_post: u64,
    /// The live map last posted.
    live_map: Option<Value>,
    /// The tables when the server has no animation files.
    no_anim: AnimRules,
}

impl Default for WebArt {
    fn default() -> Self {
        let art = Self {
            packer: ShelfPacker::new(ATLAS_SIDE),
            pictures: HashMap::new(),
            used: HashSet::new(),
            forgotten: Vec::new(),
            wanted: Vec::new(),
            uploads: Vec::new(),
            atlas_reset: true,
            wants: Wants::default(),
            tiles: None,
            radar: None,
            seasons: None,
            cycles: None,
            anim: None,
            multis: HashMap::new(),
            lights: HashMap::new(),
            frames: HashMap::new(),
            text_colors: HashMap::new(),
            masks: HashMap::new(),
            blocks: HashMap::new(),
            block_read: HashMap::new(),
            frames_ended: 0,
            measures: RefCell::new(HashMap::new()),
            to_measure: RefCell::new(Vec::new()),
            posts: Vec::new(),
            posted: HashMap::new(),
            next_post: 0,
            live_map: None,
            no_anim: AnimRules::default(),
        };
        for table in [
            DataPath::TileData,
            DataPath::Radar,
            DataPath::Seasons,
            DataPath::Cycles,
            DataPath::AnimRules,
        ] {
            art.wants.want(table.path());
        }
        art
    }
}

/// The table of `key`, asked for by its path the first time.
fn asked_for<'a, K: Copy + Eq + std::hash::Hash, T>(
    tables: &'a mut HashMap<K, Table<T>>,
    wants: &Wants,
    key: K,
    path_of: impl FnOnce(K) -> DataPath,
) -> &'a Table<T> {
    tables.entry(key).or_insert_with(|| {
        wants.want(path_of(key).path());
        Table::Asked
    })
}

/// A table that comes once: pending until its path is answered.
fn table_art<T>(table: &Option<Table<T>>) -> Art<&T> {
    table.as_ref().map_or(Art::Pending, Table::art)
}

impl WebArt {
    /// The pictures to get, each one time.
    pub fn take_wanted(&mut self) -> Vec<Wanted> {
        std::mem::take(&mut self.wanted)
    }

    /// The paths of the tables to get, each one time.
    pub fn take_data_wanted(&mut self) -> Vec<String> {
        self.wants.take()
    }

    /// The bodies to post, each one time.
    pub fn take_posts(&mut self) -> Vec<Post> {
        let measured: Vec<(u64, String, TextLook)> =
            std::mem::take(&mut *self.to_measure.borrow_mut());
        for (key, text, look) in measured {
            let body = serde_json::json!({ "text": text, "look": look });
            self.post(MEASURE_PATH, body, PostFor::Measure(key));
        }
        std::mem::take(&mut self.posts)
    }

    /// The pictures placed in the texture since the last call.
    pub fn take_uploads(&mut self) -> Vec<Upload> {
        std::mem::take(&mut self.uploads)
    }

    /// The keys of the pictures forgotten since the last call: the page
    /// may drop their pixels. A picture forgotten is fetched again when the
    /// scene asks for it.
    pub fn take_forgotten(&mut self) -> Vec<String> {
        std::mem::take(&mut self.forgotten)
    }

    /// True once after the texture started again.
    pub fn take_atlas_reset(&mut self) -> bool {
        std::mem::take(&mut self.atlas_reset)
    }

    fn post(&mut self, path: &'static str, body: Value, about: PostFor) {
        self.next_post += 1;
        let key = self.next_post;
        self.posted.insert(key, about);
        self.posts.push(Post {
            key: key.to_string(),
            path,
            body,
        });
    }

    /// The picture of a request came: its size, and the point of it that
    /// goes on the tile.
    pub fn arrived(&mut self, key: u64, width: usize, height: usize, anchor_x: f32, anchor_y: f32) {
        self.pictures.insert(
            key,
            Picture::Arrived {
                width,
                height,
                anchor: Vector::new(anchor_x, anchor_y),
                sprite: None,
            },
        );
    }

    /// The server has no picture for a request.
    pub fn missing(&mut self, key: u64) {
        self.pictures.insert(key, Picture::Missing);
    }

    /// The tiles of a block came, row by row.
    pub fn block_arrived(&mut self, map: u8, bx: u16, by: u16, cells: Vec<Cell>) {
        let table = if cells.len() == BLOCK_CELLS {
            Table::Here(cells)
        } else {
            Table::Missing
        };
        let key = (map, bx, by);
        self.blocks.insert(key, table);
        self.block_read.insert(key, self.frames_ended);
    }

    /// Ends a frame: drops the blocks not read for [`BLOCK_KEEP_FRAMES`]
    /// frames. They are asked for again when the scene needs them.
    pub fn end_frame(&mut self) {
        self.frames_ended += 1;
        let now = self.frames_ended;
        let stale: Vec<BlockKey> = self
            .block_read
            .iter()
            .filter(|(_, read)| now - **read > BLOCK_KEEP_FRAMES)
            .map(|(key, _)| *key)
            .collect();
        self.drop_block_keys(stale);
    }

    fn drop_block_keys(&mut self, keys: impl IntoIterator<Item = BlockKey>) {
        for key in keys {
            self.blocks.remove(&key);
            self.block_read.remove(&key);
            self.wants.forget(&DataPath::Block(key).path());
        }
    }

    /// Drops changed blocks, so they are asked for again.
    fn drop_blocks(&mut self, changed: &[MapBlockAt]) {
        self.drop_block_keys(changed.iter().map(|block| (block.map, block.bx, block.by)));
    }

    /// The live map of a picture, the `live_map` value of the `watch`
    /// tool. A new one goes to the server, which answers the blocks it
    /// changed.
    pub fn see_live_map(&mut self, live: &Value) {
        let has_blocks = live
            .get("blocks")
            .and_then(Value::as_array)
            .is_some_and(|blocks| !blocks.is_empty());
        if !has_blocks || self.live_map.as_ref() == Some(live) {
            return;
        }
        self.live_map = Some(live.clone());
        self.post(LIVE_MAP_PATH, live.clone(), PostFor::LiveMap);
    }

    /// The answer to a post came.
    pub fn post_arrived(&mut self, key: u64, answer: &Value) {
        match self.posted.remove(&key) {
            Some(PostFor::Measure(measure)) => {
                let table =
                    serde_json::from_value(answer.clone()).map_or(Table::Missing, Table::Here);
                self.measures.borrow_mut().insert(measure, table);
            }
            Some(PostFor::LiveMap) => {
                let changed: Vec<MapBlockAt> =
                    serde_json::from_value(answer.clone()).unwrap_or_default();
                self.drop_blocks(&changed);
            }
            None => {}
        }
    }

    /// A post had no answer.
    pub fn post_missing(&mut self, key: u64) {
        if let Some(PostFor::Measure(measure)) = self.posted.remove(&key) {
            self.measures.borrow_mut().insert(measure, Table::Missing);
        }
    }

    /// The answer of a path came. False for a path this art never asks
    /// for.
    pub fn data_arrived(&mut self, path: &str, answer: &Value) -> bool {
        let Some(data) = DataPath::of(path) else {
            return false;
        };
        fn read<T: serde::de::DeserializeOwned>(answer: &Value) -> Table<T> {
            serde_json::from_value(answer.clone()).map_or(Table::Missing, Table::Here)
        }
        match data {
            DataPath::TileData => self.tiles = Some(read(answer)),
            DataPath::Radar => self.radar = Some(read(answer)),
            DataPath::Seasons => self.seasons = Some(read(answer)),
            DataPath::Cycles => {
                let cycles: Table<BTreeMap<u16, ArtCycle>> = read(answer);
                self.cycles = Some(match cycles {
                    Table::Here(by_graphic) => Table::Here(by_graphic.into_iter().collect()),
                    Table::Asked | Table::Missing => Table::Missing,
                });
            }
            DataPath::AnimRules => self.anim = Some(read(answer)),
            DataPath::Multi(id) => {
                self.multis.insert(id, read(answer));
            }
            DataPath::Light(id) => {
                self.lights.insert(id, read(answer));
            }
            DataPath::Frames(key) => {
                self.frames.insert(key, read(answer));
            }
            DataPath::TextRgb(hue) => {
                self.text_colors.insert(hue, read(answer));
            }
            DataPath::GumpMask(id) => {
                let sent: Table<SentMask> = read(answer);
                let mask = match sent {
                    Table::Here(sent) => BASE64.decode(&sent.bits).map_or(Table::Missing, |bits| {
                        Table::Here(GumpMask {
                            width: sent.width,
                            height: sent.height,
                            bits,
                        })
                    }),
                    Table::Asked | Table::Missing => Table::Missing,
                };
                self.masks.insert(id, mask);
            }
            DataPath::Block((map, bx, by)) => {
                let cells: Vec<Cell> = serde_json::from_value(answer.clone()).unwrap_or_default();
                self.block_arrived(map, bx, by, cells);
            }
        }
        true
    }

    /// The server has no answer for a path. False for a path this art
    /// never asks for.
    pub fn data_missing(&mut self, path: &str) -> bool {
        let Some(data) = DataPath::of(path) else {
            return false;
        };
        match data {
            DataPath::TileData => self.tiles = Some(Table::Missing),
            DataPath::Radar => self.radar = Some(Table::Missing),
            DataPath::Seasons => self.seasons = Some(Table::Missing),
            DataPath::Cycles => self.cycles = Some(Table::Missing),
            DataPath::AnimRules => self.anim = Some(Table::Missing),
            DataPath::Multi(id) => {
                self.multis.insert(id, Table::Missing);
            }
            DataPath::Light(id) => {
                self.lights.insert(id, Table::Missing);
            }
            DataPath::Frames(key) => {
                self.frames.insert(key, Table::Missing);
            }
            DataPath::TextRgb(hue) => {
                self.text_colors.insert(hue, Table::Missing);
            }
            DataPath::GumpMask(id) => {
                self.masks.insert(id, Table::Missing);
            }
            DataPath::Block(key) => {
                self.blocks.insert(key, Table::Missing);
            }
        }
        true
    }

    /// The page lost the texture with its pixels (its WebGL context): every
    /// picture takes a place again, as when the texture is full.
    pub fn atlas_lost(&mut self) {
        self.start_again();
    }

    /// Forgets every place in the texture. The pictures used since the
    /// last start are placed again when the scene next asks for them, with
    /// the pixels the page keeps; the others are forgotten, so the page
    /// keeps no more pixels than the texture holds.
    fn start_again(&mut self) {
        self.packer.reset();
        self.uploads.clear();
        self.atlas_reset = true;
        let used = std::mem::take(&mut self.used);
        let forgotten = &mut self.forgotten;
        self.pictures.retain(|key, picture| {
            let kept = used.contains(key);
            match picture {
                Picture::Arrived { sprite, .. } if kept => *sprite = None,
                Picture::Arrived { .. } => forgotten.push(key.to_string()),
                Picture::Asked | Picture::Missing => {}
            }
            kept
        });
    }

    /// Gives a picture that came a place in the texture. None when it is
    /// larger than the texture.
    fn place(&mut self, key: u64, width: usize, height: usize, anchor: Vector) -> Option<Sprite> {
        if !self.packer.fits(width, height) {
            return None;
        }
        let placement = match self.packer.place(width, height) {
            Some(placement) => placement,
            None => {
                self.start_again();
                self.packer.place(width, height)?
            }
        };
        self.used.insert(key);
        self.uploads.push(Upload {
            key: key.to_string(),
            x: placement.x,
            y: placement.y,
            width,
            height,
        });
        Some(Sprite {
            uv: self.packer.uv(placement),
            width: width as f32,
            height: height as f32,
            anchor,
        })
    }

    fn cells(&mut self, map: u8, x: u16, y: u16) -> Art<&Vec<Cell>> {
        let key = block_of(map, x, y);
        self.block_read.insert(key, self.frames_ended);
        asked_for(&mut self.blocks, &self.wants, key, DataPath::Block).art()
    }

    fn tile_data(&self) -> Option<&TileData> {
        table_art(&self.tiles).ready()
    }
}

impl WorldArt for WebArt {
    fn has_art(&self) -> bool {
        self.tile_data().is_some()
    }

    fn has_anim(&self) -> bool {
        table_art(&self.anim).ready().is_some()
    }

    fn sprite(&mut self, request: &ArtRequest) -> Art<Sprite> {
        let key = request.key();
        self.used.insert(key);
        match self.pictures.get(&key).copied() {
            None => {
                self.pictures.insert(key, Picture::Asked);
                self.wanted.push(Wanted {
                    key: key.to_string(),
                    request: request.clone(),
                });
                Art::Pending
            }
            Some(Picture::Asked) => Art::Pending,
            Some(Picture::Missing) => Art::Missing,
            Some(Picture::Arrived {
                sprite: Some(sprite),
                ..
            }) => Art::Ready(sprite),
            Some(Picture::Arrived {
                width,
                height,
                anchor,
                sprite: None,
            }) => match self.place(key, width, height, anchor) {
                Some(sprite) => {
                    self.pictures.insert(
                        key,
                        Picture::Arrived {
                            width,
                            height,
                            anchor,
                            sprite: Some(sprite),
                        },
                    );
                    Art::Ready(sprite)
                }
                None => {
                    self.pictures.insert(key, Picture::Missing);
                    Art::Missing
                }
            },
        }
    }

    fn white_uv(&self) -> Point {
        self.packer.white_uv()
    }

    fn cell(&mut self, map: u8, x: u16, y: u16) -> Art<&Cell> {
        match self.cells(map, x, y) {
            Art::Ready(cells) => cells.get(cell_index(x, y)).into(),
            Art::Pending => Art::Pending,
            Art::Missing => Art::Missing,
        }
    }

    /// The web view gives the live map of each picture to
    /// [`WebArt::see_live_map`] as the server reads it, so there is nothing
    /// to lay here.
    fn take_live_map(&mut self, _: &WatchLiveMap) {}

    fn item_tile(&self, graphic: u16) -> Option<&ItemTile> {
        self.tile_data()?.item(graphic)
    }

    fn land_tile(&self, land_id: u16) -> Option<&LandTile> {
        self.tile_data()?.land(land_id)
    }

    fn multi_pieces(&mut self, multi: u16) -> Art<&[MultiPiece]> {
        match asked_for(&mut self.multis, &self.wants, multi, DataPath::Multi).art() {
            Art::Ready(pieces) => Art::Ready(pieces.as_slice()),
            Art::Pending => Art::Pending,
            Art::Missing => Art::Missing,
        }
    }

    fn shown_graphic(&self, graphic: u16, time_ms: u64) -> u16 {
        let flags = self
            .item_tile(graphic)
            .map_or(0, |tile| tile.flags.low_bits());
        shown_graphic(table_art(&self.cycles).ready(), flags, graphic, time_ms)
    }

    fn season_land(&self, season: u8, land_id: u16) -> u16 {
        table_art(&self.seasons)
            .ready()
            .map_or(land_id, |seasons| seasons.land(season, land_id))
    }

    fn season_item(&self, season: u8, graphic: u16) -> u16 {
        table_art(&self.seasons)
            .ready()
            .map_or(graphic, |seasons| seasons.item(season, graphic))
    }

    fn radar_rgb(&mut self, map: u8, x: u16, y: u16) -> Option<[u8; 3]> {
        // Asks for the block the first time.
        self.cell(map, x, y).ready()?;
        let cells = self.blocks.get(&block_of(map, x, y))?.art().ready()?;
        let cell = cells.get(cell_index(x, y))?;
        let radar = table_art(&self.radar).ready()?;
        match radar_item(cell.statics.iter().map(|item| (item.graphic, item.z))) {
            Some(graphic) => radar.item(graphic),
            None => radar.land(cell.land_id?),
        }
    }

    fn land_z(&mut self, map: u8, x: u16, y: u16) -> Option<i8> {
        self.cell(map, x, y).ready().map(|cell| cell.corners[0])
    }

    fn light_shape(&mut self, id: u8) -> Art<&LightShape> {
        asked_for(&mut self.lights, &self.wants, id, DataPath::Light).art()
    }

    fn anim(&self) -> &AnimRules {
        table_art(&self.anim).ready().unwrap_or(&self.no_anim)
    }

    fn frame_count(&mut self, look: &WatchLook, action: Action) -> Art<usize> {
        let key = FramesKey {
            body: look.body,
            action,
            direction: look.direction,
            mounted: is_mounted(look),
        };
        match asked_for(&mut self.frames, &self.wants, key, DataPath::Frames).art() {
            Art::Ready(count) => Art::Ready(*count),
            Art::Pending => Art::Pending,
            Art::Missing => Art::Missing,
        }
    }

    fn line_height(&self, look: &TextLook) -> f32 {
        self.measured("", look)
            .map_or(NO_FONT_LINE_HEIGHT, |measure| measure.line_height as f32)
    }

    fn text_lines(&self, text: &str, look: &TextLook) -> Vec<String> {
        self.measured(text, look)
            .map(|measure| measure.lines)
            .unwrap_or_default()
    }

    fn text_rgb(&self, hue: u16) -> [u8; 3] {
        let found = match self.text_colors.get(&hue) {
            Some(table) => table.art().ready().copied(),
            None => {
                self.wants.want(DataPath::TextRgb(hue).path());
                None
            }
        };
        text_rgb_or_plain(found)
    }

    fn gump_drawn_at(&self, gump: u16, x: usize, y: usize) -> bool {
        match self.masks.get(&gump) {
            Some(table) => table.art().ready().is_some_and(|mask| mask.drawn_at(x, y)),
            None => {
                self.wants.want(DataPath::GumpMask(gump).path());
                false
            }
        }
    }

    /// The server sends gump pictures when it has client files.
    fn has_gump_art(&self) -> bool {
        self.has_art()
    }
}

impl WebArt {
    /// The lines and the line height of words in a look, when the server
    /// measured them. The words are measured once.
    fn measured(&self, text: &str, look: &TextLook) -> Option<TextMeasure> {
        let key = measure_key(text, look);
        let mut measures = self.measures.borrow_mut();
        match measures.get(&key) {
            Some(table) => table.art().ready().cloned(),
            None => {
                if measures.len() >= MEASURES_KEPT {
                    measures.clear();
                }
                measures.insert(key, Table::Asked);
                self.to_measure
                    .borrow_mut()
                    .push((key, text.to_string(), *look));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uoterm_view::art::{Art, ArtRequest, WorldArt};

    #[test]
    fn a_missing_picture_is_asked_one_time() {
        let mut art = WebArt::default();
        let request = ArtRequest::Item {
            graphic: 9,
            hue: 0,
            whole_hue: false,
            border: false,
        };
        assert!(matches!(art.sprite(&request), Art::Pending));
        assert_eq!(art.take_wanted().len(), 1);
        art.missing(request.key());
        assert!(matches!(art.sprite(&request), Art::Missing));
        assert!(art.take_wanted().is_empty());
    }

    #[test]
    fn an_arrived_picture_gets_an_atlas_place() {
        let mut art = WebArt::default();
        let request = ArtRequest::Land { land_id: 3, hue: 0 };
        let _ = art.sprite(&request);
        art.take_wanted();
        art.arrived(request.key(), 44, 44, 22.0, 22.0);
        assert!(matches!(art.sprite(&request), Art::Ready(_)));
        assert_eq!(art.take_uploads().len(), 1);
    }

    #[test]
    fn a_block_not_read_for_a_while_is_dropped() {
        let mut art = WebArt::default();
        art.block_arrived(0, 0, 0, vec![Cell::default(); 64]);
        for _ in 0..=BLOCK_KEEP_FRAMES {
            art.end_frame();
        }
        assert!(matches!(art.cell(0, 0, 0), Art::Pending));
        assert!(art
            .take_data_wanted()
            .contains(&"/v1/map/0/0/0".to_string()));
    }

    #[test]
    fn a_block_read_each_frame_stays() {
        let mut art = WebArt::default();
        art.block_arrived(0, 0, 0, vec![Cell::default(); 64]);
        for _ in 0..BLOCK_KEEP_FRAMES * 2 {
            assert!(matches!(art.cell(0, 0, 0), Art::Ready(_)));
            art.end_frame();
        }
    }

    #[test]
    fn a_picture_not_used_since_the_last_restart_is_forgotten() {
        const SIDE: usize = 2000;
        /// Four of this side fill the texture with the white square.
        const FILL: u16 = 4;
        let mut art = WebArt::default();
        let land = |land_id: u16| ArtRequest::Land { land_id, hue: 0 };
        let show = |art: &mut WebArt, land_id: u16| {
            let request = land(land_id);
            if matches!(art.sprite(&request), Art::Pending) {
                art.arrived(request.key(), SIDE, SIDE, 0.0, 0.0);
            }
            assert!(matches!(art.sprite(&request), Art::Ready(_)));
        };
        for land_id in 0..=FILL {
            show(&mut art, land_id);
        }
        assert!(art.take_forgotten().is_empty(), "all were used");
        show(&mut art, 0);
        for land_id in FILL + 1..FILL * 2 + 1 {
            show(&mut art, land_id);
        }
        let forgotten = art.take_forgotten();
        // The picture that started the texture again took a place in it,
        // so it counts as used.
        let gone: Vec<String> = (1..FILL).map(|id| land(id).key().to_string()).collect();
        assert_eq!(forgotten.len(), gone.len(), "{forgotten:?}");
        assert!(gone.iter().all(|key| forgotten.contains(key)));
        art.take_wanted();
        assert!(matches!(art.sprite(&land(1)), Art::Pending), "asked again");
        assert_eq!(art.take_wanted().len(), 1);
    }

    #[test]
    fn the_measures_kept_stay_few() {
        let art = WebArt::default();
        let look = TextLook::unicode(1, 0);
        for line in 0..=MEASURES_KEPT {
            art.text_lines(&line.to_string(), &look);
        }
        assert!(art.measures.borrow().len() <= MEASURES_KEPT);
    }

    #[test]
    fn a_lost_texture_starts_again_and_places_its_pictures_with_no_fetch() {
        let mut art = WebArt::default();
        art.take_atlas_reset();
        let request = ArtRequest::Land { land_id: 3, hue: 0 };
        let _ = art.sprite(&request);
        art.take_wanted();
        art.arrived(request.key(), 44, 44, 22.0, 22.0);
        assert!(matches!(art.sprite(&request), Art::Ready(_)));
        art.take_uploads();
        art.atlas_lost();
        assert!(art.take_atlas_reset());
        assert!(matches!(art.sprite(&request), Art::Ready(_)));
        assert_eq!(art.take_uploads().len(), 1, "placed again");
        assert!(
            art.take_wanted().is_empty(),
            "with the pixels the page keeps"
        );
        assert!(art.take_forgotten().is_empty());
    }

    #[test]
    fn a_full_texture_starts_again_and_places_the_pictures_anew() {
        /// Four pictures of this side fill the texture with the white
        /// square, and the fifth starts it again.
        const SIDE: usize = 2000;
        let mut art = WebArt::default();
        assert!(art.take_atlas_reset(), "the first frame lays the texture");
        let requests: Vec<ArtRequest> = (0..5)
            .map(|land_id| ArtRequest::Land { land_id, hue: 0 })
            .collect();
        for request in &requests {
            let _ = art.sprite(request);
            art.arrived(request.key(), SIDE, SIDE, 0.0, 0.0);
            assert!(matches!(art.sprite(request), Art::Ready(_)));
        }
        assert!(art.take_atlas_reset(), "four of this size fill it");
        assert_eq!(art.take_uploads().len(), 1, "the uploads before are gone");
        assert!(matches!(art.sprite(&requests[0]), Art::Ready(_)));
        assert_eq!(art.take_uploads().len(), 1, "placed again with no fetch");
        assert_eq!(art.take_wanted().len(), requests.len());
    }

    #[test]
    fn a_tile_is_read_from_its_block_row_by_row() {
        const X: u16 = 8 * 3 + 2;
        const Y: u16 = 8 * 5 + 1;
        const LAND: u16 = 7;
        let mut art = WebArt::default();
        assert!(matches!(art.cell(1, X, Y), Art::Pending));
        let paths = art.take_data_wanted();
        assert!(paths.contains(&"/v1/map/1/3/5".to_string()), "{paths:?}");
        let mut cells = vec![Cell::default(); BLOCK_CELLS];
        cells[usize::from(BLOCK_SIDE) + 2].land_id = Some(LAND);
        cells[usize::from(BLOCK_SIDE) + 2].corners = [4, 0, 0, 0];
        assert!(art.data_arrived("/v1/map/1/3/5", &json!(cells)));
        assert!(matches!(art.cell(1, X, Y), Art::Ready(cell) if cell.land_id == Some(LAND)));
        assert_eq!(art.land_z(1, X, Y), Some(4));
        assert!(art.take_data_wanted().is_empty(), "asked one time");
    }

    #[test]
    fn the_tables_come_once_and_a_missing_one_stays_missing() {
        let mut art = WebArt::default();
        let paths = art.take_data_wanted();
        assert_eq!(paths.len(), 5, "{paths:?}");
        assert!(!art.has_art(), "pending");
        assert!(art.data_arrived(TILEDATA_PATH, &json!(TileData::default())));
        assert!(art.has_art());
        assert!(art.data_missing(ANIM_RULES_PATH));
        assert!(!art.has_anim());
        assert!(!art.data_arrived("/v1/elsewhere", &json!(null)));
        let fire = ArtCycle {
            offsets: vec![0, 1],
            step_ms: 100,
        };
        let cycles: BTreeMap<u16, ArtCycle> = [(3, fire)].into_iter().collect();
        assert!(art.data_arrived(CYCLES_PATH, &json!(cycles)));
        assert_eq!(
            art.shown_graphic(3, 100),
            3,
            "tiledata does not mark it animated"
        );
    }

    #[test]
    fn a_body_asks_for_its_frames_by_its_words() {
        let mut art = WebArt::default();
        art.take_data_wanted();
        let look = WatchLook {
            body: 0x0190,
            direction: 2,
            ..WatchLook::default()
        };
        assert_eq!(art.frame_count(&look, Action::Walk), Art::Pending);
        assert_eq!(art.take_data_wanted(), ["/v1/data/frames/400/walk/2/false"]);
        assert!(art.data_arrived("/v1/data/frames/400/walk/2/false", &json!(10)));
        assert_eq!(art.frame_count(&look, Action::Walk), Art::Ready(10));
    }

    #[test]
    fn words_are_measured_by_a_post_and_a_live_map_drops_its_blocks() {
        let mut art = WebArt::default();
        let look = TextLook::unicode(1, 0);
        assert!(art.text_lines("Hail", &look).is_empty(), "pending");
        let posts = art.take_posts();
        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].path, MEASURE_PATH);
        let key: u64 = posts[0].key.parse().unwrap();
        art.post_arrived(key, &json!({ "lines": ["Hail"], "line_height": 14 }));
        assert_eq!(art.text_lines("Hail", &look), ["Hail"]);
        art.block_arrived(0, 1, 1, vec![Cell::default(); BLOCK_CELLS]);
        art.take_data_wanted();
        let live = json!({ "map": 0, "revision": 1, "blocks": [{ "block": 1, "changed": 1 }] });
        art.see_live_map(&live);
        art.see_live_map(&live);
        let posts = art.take_posts();
        assert_eq!(posts.len(), 1, "the same live map goes one time");
        let key: u64 = posts[0].key.parse().unwrap();
        art.post_arrived(key, &json!([{ "map": 0, "bx": 1, "by": 1 }]));
        assert!(matches!(art.cell(0, 8, 8), Art::Pending), "asked again");
        assert_eq!(art.take_data_wanted(), ["/v1/map/0/1/1"]);
    }

    #[test]
    fn a_gump_mask_comes_in_base64() {
        let mut art = WebArt::default();
        assert!(!art.gump_drawn_at(5, 0, 0));
        let paths = art.take_data_wanted();
        assert!(paths.contains(&"/v1/gump-mask/5".to_string()));
        let bits = BASE64.encode([0b0000_0010u8]);
        let mask = json!({ "width": 2, "height": 1, "bits": bits });
        assert!(art.data_arrived("/v1/gump-mask/5", &mask));
        assert!(!art.gump_drawn_at(5, 0, 0));
        assert!(art.gump_drawn_at(5, 1, 0));
    }
}
