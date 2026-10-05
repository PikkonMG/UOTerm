//! What the world maps of both styles share, apart from how they draw: how
//! a view lays tiles on its field (turned as the play field is turned, or
//! north up), the zoom of the wheel, how the pictures of the land sample
//! the radar colors, the marker and zone files, the named places of the
//! session, and where each mark goes over the land. The window paints the
//! marks.

use crate::frame::WatchFrame;
use crate::geom::{Area, Point, Rgba, Vector};
use crate::model::reads::{ReadCache, ReadKey};
use crate::model::world_map::{self, MapFolder, Marker, MarkerFile, ZoneFile};
use crate::settings::{Profile, WorldMapOptions};
use crate::ui::theme;
use serde_json::{json, Value};
use uoterm_world::tool_names::TOOL_FIND_LANDMARKS;

/// How many tiles one side of the picture near a place covers.
pub const SPAN: usize = 256;
/// Where a web page gets the picture of the land near a tile:
/// `{NEAR_MAP_PREFIX}/{map}/{x}/{y}`.
pub const NEAR_MAP_PREFIX: &str = "/v1/map/near";
/// A tile of a map picture whose color the client files do not give.
pub const UNKNOWN_LAND: Rgba = Rgba::from_rgb(10, 12, 18);
/// The whole-world picture is at most this many pixels on its longer side.
pub const WORLD_PICTURE_SIDE: u16 = 1024;
/// A web page gets the whole-world picture in square tiles of this many
/// pixels: `{MAP_PICTURE_PREFIX}/{map}/{tx}/{ty}`.
pub const MAP_TILE_SIDE: usize = 256;
pub const MAP_PICTURE_PREFIX: &str = "/v1/map-picture";
/// The least zoom of a Modern map: the whole picture fits the field.
pub const ZOOM_MIN: f32 = 1.0;
/// The picture near a place is made again when its middle is this many
/// tiles away.
pub const REDRAW_TILES: u16 = 48;
/// The named places of the session are read again this seldom, in seconds.
pub const LANDMARKS_MAX_AGE: f64 = 60.0;
/// How much one notch of the wheel changes the zoom of a Modern map.
const ZOOM_PER_NOTCH: f32 = 1.15;
const LANDMARK_KEY_NAME: &str = "name";
const LANDMARK_KEY_LOCATION: &str = "location";
const DOT_RADIUS: f32 = 2.5;
const SELF_RADIUS: f32 = 4.0;
const MULTI_SIDE: f32 = 5.0;
const GOAL_STROKE: f32 = 1.5;
const ZONE_STROKE: f32 = 1.5;
const ZONE_ALPHA: f32 = 0.6;
const GROUP_BAR_WIDTH: f32 = 24.0;
const GROUP_BAR_HEIGHT: f32 = 3.0;
const PERCENT: f32 = 100.0;
/// Grid lines show every this many tiles, when they are this many points
/// apart.
const GRID_TILES: f32 = 8.0;
const GRID_MIN_GAP: f32 = 24.0;
const GRID_STROKE: f32 = 0.5;
const HALF: f32 = 2.0;

/// The picture of the land round `middle`: [`SPAN`] tiles on a side, row
/// by row from the north west corner, each tile in the color `radar` gives
/// it, or [`UNKNOWN_LAND`]. None when `radar` gives no tile a color.
pub fn near_pixels(
    middle: (u16, u16),
    mut radar: impl FnMut(u16, u16) -> Option<[u8; 3]>,
) -> Option<Vec<Rgba>> {
    let half = (SPAN / 2) as i32;
    let mut pixels = vec![UNKNOWN_LAND; SPAN * SPAN];
    let mut any = false;
    for row in 0..SPAN {
        for column in 0..SPAN {
            let x = i32::from(middle.0) + column as i32 - half;
            let y = i32::from(middle.1) + row as i32 - half;
            let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                continue;
            };
            if let Some([r, g, b]) = radar(x, y) {
                pixels[row * SPAN + column] = Rgba::from_rgb(r, g, b);
                any = true;
            }
        }
    }
    any.then_some(pixels)
}

/// How many tiles of the world one pixel of the whole-world picture of
/// `map` stands for.
pub fn world_step(map: u8) -> u16 {
    let (width, height) = world_map::facet_size(map);
    width.max(height).div_ceil(WORLD_PICTURE_SIDE)
}

/// The size of the whole-world picture of `map`: its columns and its rows.
pub fn world_picture_size(map: u8) -> (usize, usize) {
    let (width, height) = world_map::facet_size(map);
    let step = world_step(map);
    (
        usize::from(width.div_ceil(step)),
        usize::from(height.div_ceil(step)),
    )
}

/// The tile of the world one pixel of the whole-world picture shows, by
/// its column and its row.
pub fn world_pixel_tile(step: u16, column: usize, row: usize) -> (u16, u16) {
    let at = |pixel: usize| u16::try_from(pixel * usize::from(step)).unwrap_or(u16::MAX);
    (at(column), at(row))
}

/// How many tiles of [`MAP_TILE_SIDE`] the whole-world picture of `map` is
/// across and down.
pub fn map_picture_tiles(map: u8) -> (usize, usize) {
    let (columns, rows) = world_picture_size(map);
    (
        columns.div_ceil(MAP_TILE_SIDE),
        rows.div_ceil(MAP_TILE_SIDE),
    )
}

/// The path of one tile of the whole-world picture of `map`.
pub fn map_picture_path(map: u8, tx: usize, ty: usize) -> String {
    format!("{MAP_PICTURE_PREFIX}/{map}/{tx}/{ty}")
}

/// The pixels of one tile of the whole-world picture of `map`, row by row:
/// each pixel in the color `radar` gives its tile, or [`UNKNOWN_LAND`],
/// also past the edge of the picture. None for a tile past the picture,
/// or when `radar` gives no pixel a color.
pub fn map_tile_pixels(
    map: u8,
    (tx, ty): (usize, usize),
    mut radar: impl FnMut(u16, u16) -> Option<[u8; 3]>,
) -> Option<Vec<Rgba>> {
    let (across, down) = map_picture_tiles(map);
    if tx >= across || ty >= down {
        return None;
    }
    let (columns, rows) = world_picture_size(map);
    let step = world_step(map);
    let mut pixels = vec![UNKNOWN_LAND; MAP_TILE_SIDE * MAP_TILE_SIDE];
    let mut any = false;
    for row in 0..MAP_TILE_SIDE {
        let picture_row = ty * MAP_TILE_SIDE + row;
        for column in 0..MAP_TILE_SIDE {
            let picture_column = tx * MAP_TILE_SIDE + column;
            if picture_column >= columns || picture_row >= rows {
                continue;
            }
            let (x, y) = world_pixel_tile(step, picture_column, picture_row);
            if let Some([r, g, b]) = radar(x, y) {
                pixels[row * MAP_TILE_SIDE + column] = Rgba::from_rgb(r, g, b);
                any = true;
            }
        }
    }
    any.then_some(pixels)
}

/// One tile of the whole-world picture that shows on a field.
#[derive(Clone, Debug, PartialEq)]
pub struct MapTileAt {
    pub path: String,
    /// Where it lies on the field.
    pub area: Area,
}

/// The tiles of the whole-world picture of `map` that show on `field`
/// laid north up by `lay`, each where it lies.
pub fn map_tiles_on(field: Area, lay: Lay, map: u8) -> Vec<MapTileAt> {
    let (across, down) = map_picture_tiles(map);
    let tile_tiles = (MAP_TILE_SIDE * usize::from(world_step(map))) as f32;
    let mut out = Vec::new();
    for ty in 0..down {
        for tx in 0..across {
            let (left, top) = (tx as f32 * tile_tiles, ty as f32 * tile_tiles);
            let area = Area::from_two_points(
                lay.screen(left, top),
                lay.screen(left + tile_tiles, top + tile_tiles),
            );
            if area.intersects(field) {
                out.push(MapTileAt {
                    path: map_picture_path(map, tx, ty),
                    area,
                });
            }
        }
    }
    out
}

/// True when a picture of the land round `drawn` of `drawn_map` still
/// serves for `middle` of `map`.
pub fn near_still_serves(drawn_map: u8, drawn: (u16, u16), map: u8, middle: (u16, u16)) -> bool {
    drawn_map == map
        && drawn.0.abs_diff(middle.0) < REDRAW_TILES
        && drawn.1.abs_diff(middle.1) < REDRAW_TILES
}

/// The named places of the session on a map, while the World Map page
/// shows markers. The session is read again now and then.
pub fn session_markers(reads: &mut ReadCache, profile: &Profile, map: u8) -> Vec<Marker> {
    if !profile.world_map.show_markers {
        return Vec::new();
    }
    let key = ReadKey::new(TOOL_FIND_LANDMARKS, &json!({ "map": map }));
    reads
        .want(key, LANDMARKS_MAX_AGE)
        .map(|answer| landmarks(answer, map))
        .unwrap_or_default()
}

/// The name of the query that names the session whose live map lies over
/// a path of the map: a block, the land near a tile, a tile of the world
/// picture, the land of a map item.
pub const SESSION_PARAM: &str = "session";
const QUERY_START: char = '?';
const QUERY_NEXT: char = '&';

/// `path` asking for the map as `session` sees it, after the query `path`
/// already has. The path as it is with no session.
pub fn with_session(path: &str, session: Option<&str>) -> String {
    match session {
        Some(session) => {
            let join = if path.contains(QUERY_START) {
                QUERY_NEXT
            } else {
                QUERY_START
            };
            format!("{path}{join}{SESSION_PARAM}={session}")
        }
        None => path.to_string(),
    }
}

/// The path of the picture of the land near tile `x`, `y` of `map`.
pub fn near_map_path(map: u8, x: u16, y: u16) -> String {
    format!("{NEAR_MAP_PREFIX}/{map}/{x}/{y}")
}

/// Where a tile is on the turned map, as a step from the middle. One tile
/// east goes right and down, one tile south goes left and down.
pub fn turned(tiles: Vector, unit: f32) -> Vector {
    Vector::new(tiles.x - tiles.y, tiles.x + tiles.y) * unit
}

/// The tiles from the middle for a step on the turned map.
pub fn unturned(on_screen: Vector, unit: f32) -> Vector {
    let (a, b) = (on_screen.x / unit, on_screen.y / unit);
    Vector::new((a + b) / HALF, (b - a) / HALF)
}

/// How a view lays tiles on the field: turned round a middle tile, as the
/// play field is turned, or north up round a middle tile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lay {
    Turned {
        center: Point,
        from: Vector,
        /// Half the points one tile takes along a side of the diamond.
        unit: f32,
    },
    NorthUp {
        center: Point,
        middle: Vector,
        /// Points for each tile.
        scale: f32,
    },
}

impl Lay {
    pub fn screen(self, x: f32, y: f32) -> Point {
        let tile = Vector::new(x, y);
        match self {
            Self::Turned { center, from, unit } => center + turned(tile - from, unit),
            Self::NorthUp {
                center,
                middle,
                scale,
            } => center + (tile - middle) * scale,
        }
    }

    pub fn tile(self, at: Point) -> Vector {
        match self {
            Self::Turned { center, from, unit } => from + unturned(at - center, unit),
            Self::NorthUp {
                center,
                middle,
                scale,
            } => middle + (at - center) / scale,
        }
    }

    /// Points on the field for one tile.
    pub fn tile_points(self) -> f32 {
        match self {
            Self::Turned { unit, .. } => unit * HALF,
            Self::NorthUp { scale, .. } => scale,
        }
    }
}

/// The zoom after `notches` of the wheel, held between the least zoom and
/// `max`.
pub fn zoomed(zoom: f32, notches: f32, max: f32) -> f32 {
    (zoom * ZOOM_PER_NOTCH.powf(notches)).clamp(ZOOM_MIN, max)
}

/// A tile held inside the range of the map.
pub fn whole_tile(tile: Vector) -> (u16, u16) {
    let clamp = |at: f32| at.round().clamp(0.0, f32::from(u16::MAX)) as u16;
    (clamp(tile.x), clamp(tile.y))
}

/// A place in words: its tiles, and its sextant place when the World Map
/// page asks for one and the facet has one.
pub fn place_words(profile: &Profile, map: u8, x: u16, y: u16) -> String {
    let sextant = profile
        .world_map
        .sextant_coordinates
        .then(|| world_map::sextant(map, x, y))
        .flatten();
    match sextant {
        Some(sextant) => format!("{x}, {y}  {sextant}"),
        None => format!("{x}, {y}"),
    }
}

/// The places the session names on a map, from its marker file.
pub fn landmarks(answer: &Value, map: u8) -> Vec<Marker> {
    answer
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|place| {
            let at = place.get(LANDMARK_KEY_LOCATION)?;
            let number = |key: &str| u16::try_from(at.get(key)?.as_u64()?).ok();
            Some(Marker {
                name: place.get(LANDMARK_KEY_NAME)?.as_str()?.to_string(),
                map,
                x: number("x")?,
                y: number("y")?,
                icon: String::new(),
                color: String::new(),
            })
        })
        .collect()
}

/// The marker and zone files, with the hidden lists they were read with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapFiles {
    pub markers: Vec<MarkerFile>,
    pub zones: Vec<ZoneFile>,
    pub hidden_markers: Vec<String>,
    pub hidden_zones: Vec<String>,
}

impl MapFiles {
    /// The files of the folder the World Map page does not hide.
    pub fn shown(folder: &MapFolder, options: &WorldMapOptions) -> Self {
        let hidden_markers = &options.hidden_marker_files;
        let hidden_zones = &options.hidden_zone_files;
        Self {
            markers: folder
                .markers
                .iter()
                .filter(|file| !world_map::is_hidden(hidden_markers, &file.name))
                .cloned()
                .collect(),
            zones: folder
                .zones
                .iter()
                .filter(|file| !world_map::is_hidden(hidden_zones, &file.name))
                .cloned()
                .collect(),
            hidden_markers: hidden_markers.clone(),
            hidden_zones: hidden_zones.clone(),
        }
    }

    /// The label of the zone a tile lies in, on a facet.
    pub fn zone_at(&self, map: u8, x: u16, y: u16) -> Option<String> {
        self.zones
            .iter()
            .filter(|file| file.map == map)
            .find_map(|file| {
                file.zones
                    .iter()
                    .find(|zone| world_map::inside(&zone.corners, x, y))
                    .map(|zone| zone.label.clone())
            })
    }

    /// True when the files were read with the hidden lists the World Map
    /// page has now.
    pub fn read_for(&self, options: &WorldMapOptions) -> bool {
        self.hidden_markers == options.hidden_marker_files
            && self.hidden_zones == options.hidden_zone_files
    }
}

/// The colors of the marks of one style.
#[derive(Clone, Copy, Debug)]
pub struct MarkLook {
    /// A marker whose file names no color.
    pub marker: Rgba,
    pub waypoint: Rgba,
    pub multi: Rgba,
    pub party: Rgba,
    pub guild: Rgba,
    /// Where the character walks to.
    pub goal: Rgba,
    /// The place the player looked for.
    pub looking: Rgba,
    pub me: Rgba,
    pub grid: Rgba,
    pub mobile: fn(u8) -> Rgba,
}

/// The colors of the marks of the Modern style.
pub const MODERN_LOOK: MarkLook = MarkLook {
    marker: theme::WAITING,
    waypoint: theme::WAITING,
    multi: theme::FLAT_DOOR,
    party: theme::GOAL,
    guild: theme::MANA,
    goal: theme::GOAL,
    looking: theme::WAITING,
    me: theme::SELF_FIGURE,
    grid: theme::GLASS_EDGE,
    mobile: theme::notoriety_color,
};

/// What the marks of one frame come from.
pub struct Marks<'a> {
    pub frame: &'a WatchFrame,
    /// The facet the map shows. The marks of the world round the
    /// character show only on his own facet.
    pub map: u8,
    pub profile: &'a Profile,
    pub files: &'a MapFiles,
    /// The named places of the session.
    pub session: &'a [Marker],
    pub looking_at: Option<(u16, u16)>,
}

/// Which corner or edge of words lies on their point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordsAnchor {
    LeftTop,
    LeftCenter,
}

/// One mark over the land, in the points of the field.
#[derive(Clone, Debug, PartialEq)]
pub enum MarkPlace {
    Line {
        from: Point,
        to: Point,
        width: f32,
        color: Rgba,
    },
    /// A closed line round a zone.
    Outline {
        points: Vec<Point>,
        width: f32,
        color: Rgba,
    },
    Words {
        at: Point,
        anchor: WordsAnchor,
        words: String,
        color: Rgba,
    },
    /// A dot, which a style may draw round or square.
    Dot {
        at: Point,
        radius: f32,
        color: Rgba,
    },
    Square {
        at: Point,
        side: f32,
        color: Rgba,
    },
    Ring {
        at: Point,
        radius: f32,
        width: f32,
        color: Rgba,
    },
    /// A small health bar: its track and its share from 0 to 1.
    HealthBar {
        track: Area,
        share: f32,
    },
}

/// A dot with its name beside it, when the name shows.
fn named_dot(out: &mut Vec<MarkPlace>, at: Point, radius: f32, name: Option<&str>, color: Rgba) {
    out.push(MarkPlace::Dot { at, radius, color });
    if let Some(name) = name {
        out.push(MarkPlace::Words {
            at: at + Vector::new(radius * HALF, 0.0),
            anchor: WordsAnchor::LeftCenter,
            words: name.to_string(),
            color,
        });
    }
}

/// A small health bar under a dot of the map.
fn health_bar(at: Point, share: f32) -> MarkPlace {
    let track = Area::from_min_size(
        at + Vector::new(-GROUP_BAR_WIDTH / HALF, SELF_RADIUS * HALF),
        Vector::new(GROUP_BAR_WIDTH, GROUP_BAR_HEIGHT),
    );
    MarkPlace::HealthBar { track, share }
}

/// Grid lines every few tiles, over the part of the map the field shows.
fn grid(out: &mut Vec<MarkPlace>, field: Area, lay: Lay, color: Rgba) {
    let corners = [
        lay.tile(field.min),
        lay.tile(Point::new(field.max.x, field.min.y)),
        lay.tile(Point::new(field.min.x, field.max.y)),
        lay.tile(field.max),
    ];
    let low = corners
        .iter()
        .fold(Vector::new(f32::MAX, f32::MAX), |a, b| {
            Vector::new(a.x.min(b.x), a.y.min(b.y))
        });
    let high = corners
        .iter()
        .fold(Vector::new(f32::MIN, f32::MIN), |a, b| {
            Vector::new(a.x.max(b.x), a.y.max(b.y))
        });
    let line = |from: Point, to: Point| MarkPlace::Line {
        from,
        to,
        width: GRID_STROKE,
        color,
    };
    let first = |from: f32| (from / GRID_TILES).floor() * GRID_TILES;
    let mut x = first(low.x);
    while x <= high.x {
        out.push(line(lay.screen(x, low.y), lay.screen(x, high.y)));
        x += GRID_TILES;
    }
    let mut y = first(low.y);
    while y <= high.y {
        out.push(line(lay.screen(low.x, y), lay.screen(high.x, y)));
        y += GRID_TILES;
    }
}

/// Everything a map carries over the land, in paint order: the grid, the
/// zones, the markers, the marks of the shard, the houses, the mobiles,
/// the party and the guild, the goal, the place looked for, and the
/// character.
pub fn mark_layout(field: Area, lay: Lay, marks: &Marks<'_>, look: &MarkLook) -> Vec<MarkPlace> {
    let Marks {
        frame,
        map,
        profile,
        files,
        session,
        looking_at,
    } = *marks;
    let options = &profile.world_map;
    let mut out = Vec::new();
    if options.grid_when_zoomed && lay.tile_points() * GRID_TILES >= GRID_MIN_GAP {
        grid(&mut out, field, lay, look.grid);
    }
    for zone in files
        .zones
        .iter()
        .filter(|file| file.map == map)
        .flat_map(|file| &file.zones)
    {
        let Some([r, g, b]) = world_map::color_of(&zone.color) else {
            continue;
        };
        let color = Rgba::from_rgb(r, g, b).with_alpha(ZONE_ALPHA);
        let points: Vec<Point> = zone
            .corners
            .iter()
            .map(|(x, y)| lay.screen(f32::from(*x), f32::from(*y)))
            .collect();
        if let Some(first) = points.first().copied() {
            out.push(MarkPlace::Outline {
                points,
                width: ZONE_STROKE,
                color,
            });
            out.push(MarkPlace::Words {
                at: first,
                anchor: WordsAnchor::LeftTop,
                words: zone.label.clone(),
                color,
            });
        }
    }
    if options.show_markers {
        let all = files
            .markers
            .iter()
            .flat_map(|file| file.markers.iter())
            .chain(session.iter())
            .filter(|marker| marker.map == map);
        for marker in all {
            let at = lay.screen(f32::from(marker.x), f32::from(marker.y));
            if !field.contains(at) {
                continue;
            }
            let color = world_map::color_of(&marker.color)
                .map_or(look.marker, |[r, g, b]| Rgba::from_rgb(r, g, b));
            let name = options.show_marker_names.then_some(marker.name.as_str());
            named_dot(&mut out, at, DOT_RADIUS, name, color);
        }
    }
    if let Some((x, y)) = looking_at {
        out.push(MarkPlace::Ring {
            at: lay.screen(f32::from(x), f32::from(y)),
            radius: SELF_RADIUS * HALF,
            width: GOAL_STROKE,
            color: look.looking,
        });
    }
    if map == frame.map {
        world_round_me(&mut out, lay, frame, options, look);
    }
    out
}

/// The marks of the world round the character: the marks of the shard,
/// the houses, the mobiles, the party and the guild, the goal, and the
/// character himself.
fn world_round_me(
    out: &mut Vec<MarkPlace>,
    lay: Lay,
    frame: &WatchFrame,
    options: &WorldMapOptions,
    look: &MarkLook,
) {
    // The marks the shard put on the map, each with its name.
    for mark in frame.waypoints.iter().filter(|mark| mark.map == frame.map) {
        let at = lay.screen(f32::from(mark.x), f32::from(mark.y));
        named_dot(out, at, DOT_RADIUS, Some(&mark.name), look.waypoint);
    }
    if options.show_multis {
        for multi in &frame.multis {
            out.push(MarkPlace::Square {
                at: lay.screen(f32::from(multi.x), f32::from(multi.y)),
                side: MULTI_SIDE,
                color: look.multi,
            });
        }
    }
    if options.show_mobiles {
        for mobile in &frame.mobiles {
            out.push(MarkPlace::Dot {
                at: lay.screen(f32::from(mobile.x), f32::from(mobile.y)),
                radius: DOT_RADIUS,
                color: (look.mobile)(mobile.notoriety),
            });
        }
    }
    if options.show_party {
        for member in world_map::group_on_map(frame) {
            let at = lay.screen(f32::from(member.x), f32::from(member.y));
            let color = if member.guild { look.guild } else { look.party };
            let name = options.show_group_names.then_some(member.name.as_str());
            named_dot(out, at, SELF_RADIUS, name, color);
            if let (true, Some(percent)) = (options.show_group_bars, member.hits_percent) {
                out.push(health_bar(at, f32::from(percent) / PERCENT));
            }
        }
    }
    if let (Some(x), Some(y)) = (frame.dest_x, frame.dest_y) {
        out.push(MarkPlace::Ring {
            at: lay.screen(f32::from(x), f32::from(y)),
            radius: SELF_RADIUS,
            width: GOAL_STROKE,
            color: look.goal,
        });
    }
    let me = lay.screen(f32::from(frame.x), f32::from(frame.y));
    let name = options.show_player_name.then_some(frame.name.as_str());
    named_dot(out, me, SELF_RADIUS, name, look.me);
    if options.show_player_bar && frame.hits_max > 0 {
        let share = f32::from(frame.hits) / f32::from(frame.hits_max);
        out.push(health_bar(me, share));
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_map_path_names_the_session_after_its_own_query() {
        assert_eq!(
            with_session("/v1/map/near/0/1/2", Some("s1")),
            "/v1/map/near/0/1/2?session=s1"
        );
        assert_eq!(
            with_session("/v1/map-picture/0/0/0?drawn=2", Some("s1")),
            "/v1/map-picture/0/0/0?drawn=2&session=s1"
        );
        assert_eq!(with_session("/v1/map/0/0/0", None), "/v1/map/0/0/0");
    }

    use super::*;
    use serde_json::json;

    #[test]
    fn the_land_near_a_place_is_its_radar_colors_with_the_place_in_the_middle() {
        const GRASS: [u8; 3] = [20, 120, 30];
        let middle = (1_000, 2_000);
        let pixels = near_pixels(middle, |x, y| ((x, y) == middle).then_some(GRASS)).unwrap();
        let half = SPAN / 2;
        assert_eq!(pixels.len(), SPAN * SPAN);
        assert_eq!(pixels[half * SPAN + half], Rgba::from_rgb(20, 120, 30));
        assert_eq!(pixels[0], UNKNOWN_LAND);
        assert_eq!(near_pixels(middle, |_, _| None), None);
        let corner = near_pixels((0, 0), |_, _| Some(GRASS)).unwrap();
        assert_eq!(corner[0], UNKNOWN_LAND, "no tile west of the edge");
    }

    #[test]
    fn the_wheel_zooms_between_the_two_bounds() {
        const MAX: f32 = 8.0;
        assert!(
            zoomed(ZOOM_MIN, 1.0, MAX) > ZOOM_MIN,
            "a notch up comes closer"
        );
        assert_eq!(zoomed(ZOOM_MIN, -5.0, MAX), ZOOM_MIN, "never below the fit");
        assert_eq!(zoomed(MAX, 20.0, MAX), MAX, "never above the bound");
        let twice = zoomed(zoomed(ZOOM_MIN, 1.0, MAX), -1.0, MAX);
        assert!((twice - ZOOM_MIN).abs() < 0.001, "up then down comes back");
    }

    #[test]
    fn a_click_on_the_turned_map_finds_its_tile_again() {
        const UNIT: f32 = 0.9;
        let east = turned(Vector::new(1.0, 0.0), UNIT);
        assert!(east.x > 0.0 && east.y > 0.0, "east goes right and down");
        let south = turned(Vector::new(0.0, 1.0), UNIT);
        assert!(south.x < 0.0 && south.y > 0.0, "south goes left and down");
        let tiles = Vector::new(37.0, -12.0);
        let back = unturned(turned(tiles, UNIT), UNIT);
        assert!((back - tiles).length() < 0.001);
    }

    #[test]
    fn each_view_finds_the_tile_it_drew() {
        let views = [
            Lay::Turned {
                center: Point::new(200.0, 200.0),
                from: Vector::new(1000.0, 1200.0),
                unit: 1.5,
            },
            Lay::NorthUp {
                center: Point::new(200.0, 200.0),
                middle: Vector::new(3000.0, 2000.0),
                scale: 0.25,
            },
        ];
        for lay in views {
            let at = lay.screen(1010.0, 1195.0);
            assert!((lay.tile(at) - Vector::new(1010.0, 1195.0)).length() < 0.01);
        }
        assert_eq!(whole_tile(Vector::new(-4.0, 70_000.0)), (0, u16::MAX));
    }

    #[test]
    fn the_world_picture_is_cut_in_tiles_of_its_sampled_pixels() {
        const FELUCCA: u8 = 0;
        let step = world_step(FELUCCA);
        let (columns, rows) = world_picture_size(FELUCCA);
        assert!(
            columns <= usize::from(WORLD_PICTURE_SIDE) && rows <= usize::from(WORLD_PICTURE_SIDE)
        );
        let (across, down) = map_picture_tiles(FELUCCA);
        assert_eq!(
            (across, down),
            (
                columns.div_ceil(MAP_TILE_SIDE),
                rows.div_ceil(MAP_TILE_SIDE)
            )
        );
        let mut asked = Vec::new();
        let pixels = map_tile_pixels(FELUCCA, (1, 0), |x, y| {
            asked.push((x, y));
            Some([1, 2, 3])
        })
        .unwrap();
        assert_eq!(pixels.len(), MAP_TILE_SIDE * MAP_TILE_SIDE);
        assert_eq!(asked[0], world_pixel_tile(step, MAP_TILE_SIDE, 0));
        assert!(map_tile_pixels(FELUCCA, (across, 0), |_, _| Some([1, 2, 3])).is_none());
        assert!(map_tile_pixels(FELUCCA, (0, 0), |_, _| None).is_none());
        let field = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(100.0, 100.0));
        let lay = Lay::NorthUp {
            center: field.center(),
            middle: Vector::new(0.0, 0.0),
            scale: 0.1,
        };
        let tiles = map_tiles_on(field, lay, FELUCCA);
        assert_eq!(tiles[0].path, "/v1/map-picture/0/0/0");
        assert_eq!(tiles[0].area.min, field.center());
    }

    #[test]
    fn the_files_the_world_map_page_hides_do_not_show() {
        let folder = MapFolder {
            markers: ["camps", "towns"]
                .map(|name| MarkerFile {
                    name: name.into(),
                    markers: Vec::new(),
                })
                .to_vec(),
            zones: vec![ZoneFile {
                name: "towns".into(),
                map: 0,
                zones: Vec::new(),
            }],
        };
        let options = WorldMapOptions {
            hidden_marker_files: vec!["camps".into()],
            hidden_zone_files: vec!["towns".into()],
            ..WorldMapOptions::default()
        };
        let files = MapFiles::shown(&folder, &options);
        assert_eq!(files.markers.len(), 1);
        assert_eq!(files.markers[0].name, "towns");
        assert!(files.zones.is_empty());
        assert!(files.read_for(&options));
    }

    #[test]
    fn the_places_of_the_session_become_markers_of_their_map() {
        let answer = json!([
            { "name": "Britain Bank", "map": 1, "location": { "x": 1434, "y": 1699 } },
            { "name": "no place" },
        ]);
        let found = landmarks(&answer, 1);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].x, found[0].y, found[0].map), (1434, 1699, 1));
        let mut profile = Profile::default();
        assert_eq!(place_words(&profile, 1, 1434, 1699), "1434, 1699");
        profile.world_map.sextant_coordinates = true;
        assert!(place_words(&profile, 1, 1434, 1699).ends_with("7o 48'E"));
    }

    fn white(_notoriety: u8) -> Rgba {
        Rgba::from_rgb(u8::MAX, u8::MAX, u8::MAX)
    }

    const LOOK: MarkLook = MarkLook {
        marker: Rgba::from_rgb(1, 0, 0),
        waypoint: Rgba::from_rgb(2, 0, 0),
        multi: Rgba::from_rgb(3, 0, 0),
        party: Rgba::from_rgb(4, 0, 0),
        guild: Rgba::from_rgb(5, 0, 0),
        goal: Rgba::from_rgb(6, 0, 0),
        looking: Rgba::from_rgb(7, 0, 0),
        me: Rgba::from_rgb(8, 0, 0),
        grid: Rgba::from_rgb(9, 0, 0),
        mobile: white,
    };

    #[test]
    fn the_character_and_his_goal_are_marked_on_his_own_facet_only() {
        let frame = WatchFrame {
            map: 1,
            x: 100,
            y: 100,
            name: "Mara".into(),
            dest_x: Some(110),
            dest_y: Some(100),
            ..WatchFrame::default()
        };
        let mut profile = Profile::default();
        profile.world_map.show_player_name = true;
        profile.world_map.grid_when_zoomed = false;
        let files = MapFiles::default();
        let field = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(400.0, 400.0));
        let lay = Lay::NorthUp {
            center: field.center(),
            middle: Vector::new(100.0, 100.0),
            scale: 1.0,
        };
        let marks = |map| Marks {
            frame: &frame,
            map,
            profile: &profile,
            files: &files,
            session: &[],
            looking_at: None,
        };
        let placed = mark_layout(field, lay, &marks(1), &LOOK);
        let me = field.center();
        assert!(placed.contains(&MarkPlace::Dot {
            at: me,
            radius: SELF_RADIUS,
            color: LOOK.me,
        }));
        assert!(placed.iter().any(|mark| matches!(
            mark,
            MarkPlace::Words { words, .. } if words == "Mara"
        )));
        assert!(placed.contains(&MarkPlace::Ring {
            at: me + Vector::new(10.0, 0.0),
            radius: SELF_RADIUS,
            width: GOAL_STROKE,
            color: LOOK.goal,
        }));
        assert!(mark_layout(field, lay, &marks(0), &LOOK).is_empty());
    }
}
