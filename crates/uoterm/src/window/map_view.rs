//! The drawing the world maps of both styles share: the pictures of the
//! land from the radar colors of the client, how a view lays tiles on its
//! field (turned as the play field is turned, or north up), the marker and
//! zone files, the named places of the session, and the marks drawn over
//! the land. The Modern map panel and the Classic world map gump each draw
//! their own frame round it.

use super::model::reads::Readings;
use super::model::world_map::{self, Marker, MarkerFile, ZoneFile};
use super::scene::Scene;
use super::settings::{Profile, WorldMapOptions};
use super::theme;
use crate::view::WatchFrame;
use eframe::egui::{
    self, epaint::Vertex, Align2, Color32, ColorImage, CornerRadius, FontId, Mesh, Painter, Pos2,
    Rect, Shape, Stroke, TextureHandle, TextureId, TextureOptions, Vec2,
};
use serde_json::{json, Value};
use uoterm_runtime::tools::TOOL_FIND_LANDMARKS;

/// How many tiles one side of the picture near a place covers.
pub const SPAN: usize = 256;
/// The least zoom of a Modern map: the whole picture fits the field.
pub const ZOOM_MIN: f32 = 1.0;
/// How much one notch of the wheel changes the zoom of a Modern map.
const ZOOM_PER_NOTCH: f32 = 1.15;
/// The wheel gives its step in points. This many points are one notch.
const WHEEL_NOTCH: f32 = 50.0;
/// The near picture is made again when its middle is this far away.
const REDRAW_TILES: u16 = 48;
const UNKNOWN: Color32 = Color32::from_rgb(10, 12, 18);
/// The whole-world picture is at most this many pixels on its longer side.
const WORLD_PICTURE_SIDE: u16 = 1024;
/// The whole-world picture grows this many rows in each frame, so the
/// window does not stop while the facet is read.
const WORLD_ROWS_PER_FRAME: usize = 8;
const NEAR_TEXTURE: &str = "world-map";
const WORLD_TEXTURE: &str = "whole-world-map";
/// The named places of the session are read again this seldom, in seconds.
const LANDMARKS_MAX_AGE: f64 = 60.0;
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
const WHOLE_UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

/// Where a tile is on the turned map, as a step from the middle. One tile
/// east goes right and down, one tile south goes left and down.
fn turned(tiles: Vec2, unit: f32) -> Vec2 {
    Vec2::new(tiles.x - tiles.y, tiles.x + tiles.y) * unit
}

/// The tiles from the middle for a step on the turned map.
fn unturned(on_screen: Vec2, unit: f32) -> Vec2 {
    let (a, b) = (on_screen.x / unit, on_screen.y / unit);
    Vec2::new((a + b) / HALF, (b - a) / HALF)
}

/// How a view lays tiles on the field: turned round a middle tile, as the
/// play field is turned, or north up round a middle tile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lay {
    Turned {
        center: Pos2,
        from: Vec2,
        /// Half the points one tile takes along a side of the diamond.
        unit: f32,
    },
    NorthUp {
        center: Pos2,
        middle: Vec2,
        /// Points for each tile.
        scale: f32,
    },
}

impl Lay {
    pub fn screen(self, x: f32, y: f32) -> Pos2 {
        match self {
            Self::Turned { center, from, unit } => center + turned(Vec2::new(x, y) - from, unit),
            Self::NorthUp {
                center,
                middle,
                scale,
            } => center + (Vec2::new(x, y) - middle) * scale,
        }
    }

    pub fn tile(self, at: Pos2) -> Vec2 {
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

/// The notches the wheel turned over a field this frame: up is positive.
pub fn wheel_notches(ui: &egui::Ui, response: &egui::Response) -> f32 {
    if response.hovered() {
        ui.input(|input| input.raw_scroll_delta.y) / WHEEL_NOTCH
    } else {
        0.0
    }
}

/// A tile held inside the range of the map.
pub fn whole_tile(tile: Vec2) -> (u16, u16) {
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
fn landmarks(answer: &Value, map: u8) -> Vec<Marker> {
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

/// The named places of the session on a map, while the World Map page
/// shows markers. The session is read again now and then.
pub fn session_markers(readings: &mut Readings, profile: &Profile, map: u8) -> Vec<Marker> {
    if !profile.world_map.show_markers {
        return Vec::new();
    }
    readings
        .want(
            TOOL_FIND_LANDMARKS,
            json!({ "map": map }),
            LANDMARKS_MAX_AGE,
        )
        .map(|answer| landmarks(answer, map))
        .unwrap_or_default()
}

/// Draws a texture over the tiles from `from` to `to`, laid by `lay`.
fn draw_texture(painter: &Painter, lay: Lay, texture: TextureId, from: Vec2, to: Vec2) {
    let corners = [
        (from.x, from.y),
        (to.x, from.y),
        (to.x, to.y),
        (from.x, to.y),
    ];
    let uvs = [
        WHOLE_UV.left_top(),
        WHOLE_UV.right_top(),
        WHOLE_UV.right_bottom(),
        WHOLE_UV.left_bottom(),
    ];
    let mut mesh = Mesh::with_texture(texture);
    for ((x, y), uv) in corners.into_iter().zip(uvs) {
        mesh.vertices.push(Vertex {
            pos: lay.screen(x, y),
            uv,
            color: Color32::WHITE,
        });
    }
    mesh.indices.extend([0, 1, 2, 0, 2, 3]);
    painter.add(Shape::mesh(mesh));
}

/// The land round one tile, one pixel for each tile.
struct NearPicture {
    map: u8,
    /// The tile in the middle of the picture.
    middle: (u16, u16),
    texture: TextureHandle,
}

/// The whole facet, drawn a few rows at a time.
struct WorldPicture {
    map: u8,
    /// How many tiles one pixel stands for.
    step: u16,
    image: ColorImage,
    next_row: usize,
    texture: Option<TextureHandle>,
}

/// The pictures of the land a map draws.
#[derive(Default)]
pub struct MapPictures {
    near: Option<NearPicture>,
    world: Option<WorldPicture>,
}

impl MapPictures {
    /// Makes the picture of the land round a middle tile, when there is none
    /// near enough. False when the client files have no map.
    pub fn make_near(
        &mut self,
        ctx: &egui::Context,
        scene: &mut Scene,
        map: u8,
        middle: (u16, u16),
    ) -> bool {
        let fresh = self.near.as_ref().is_some_and(|picture| {
            picture.map == map
                && picture.middle.0.abs_diff(middle.0) < REDRAW_TILES
                && picture.middle.1.abs_diff(middle.1) < REDRAW_TILES
        });
        if fresh {
            return true;
        }
        let half = (SPAN / 2) as i32;
        let mut image = ColorImage::new([SPAN, SPAN], UNKNOWN);
        let mut any = false;
        for row in 0..SPAN {
            for column in 0..SPAN {
                let x = i32::from(middle.0) + column as i32 - half;
                let y = i32::from(middle.1) + row as i32 - half;
                let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                    continue;
                };
                if let Some([r, g, b]) = scene.radar_rgb(map, x, y) {
                    image.pixels[row * SPAN + column] = Color32::from_rgb(r, g, b);
                    any = true;
                }
            }
        }
        if !any {
            self.near = None;
            return false;
        }
        self.near = Some(NearPicture {
            map,
            middle,
            texture: ctx.load_texture(NEAR_TEXTURE, image, TextureOptions::NEAREST),
        });
        true
    }

    /// Draws the near picture on the field by the lay.
    pub fn draw_near(&self, painter: &Painter, lay: Lay) {
        let Some(picture) = &self.near else {
            return;
        };
        let half = SPAN as f32 / HALF;
        let middle = Vec2::new(f32::from(picture.middle.0), f32::from(picture.middle.1));
        draw_texture(
            painter,
            lay,
            picture.texture.id(),
            middle - Vec2::splat(half),
            middle + Vec2::splat(half),
        );
    }

    /// Reads a few more rows of the whole facet. True while rows are left.
    pub fn grow_world(&mut self, ctx: &egui::Context, scene: &mut Scene, map: u8) -> bool {
        let (width, height) = world_map::facet_size(map);
        if self.world.as_ref().is_none_or(|world| world.map != map) {
            let step = width.max(height).div_ceil(WORLD_PICTURE_SIDE);
            let size = [
                usize::from(width.div_ceil(step)),
                usize::from(height.div_ceil(step)),
            ];
            self.world = Some(WorldPicture {
                map,
                step,
                image: ColorImage::new(size, UNKNOWN),
                next_row: 0,
                texture: None,
            });
        }
        let Some(world) = self.world.as_mut() else {
            return false;
        };
        let [columns, rows] = world.image.size;
        if world.next_row >= rows {
            return false;
        }
        let last = (world.next_row + WORLD_ROWS_PER_FRAME).min(rows);
        for row in world.next_row..last {
            for column in 0..columns {
                let x = column as u16 * world.step;
                let y = row as u16 * world.step;
                if let Some([r, g, b]) = scene.radar_rgb(map, x, y) {
                    world.image.pixels[row * columns + column] = Color32::from_rgb(r, g, b);
                }
            }
        }
        world.next_row = last;
        world.texture =
            Some(ctx.load_texture(WORLD_TEXTURE, world.image.clone(), TextureOptions::LINEAR));
        world.next_row < rows
    }

    /// Draws the whole facet on the field by the lay. False while no row
    /// of it is read.
    pub fn draw_world(&self, painter: &Painter, lay: Lay) -> bool {
        let Some(world) = &self.world else {
            return false;
        };
        let Some(texture) = &world.texture else {
            return false;
        };
        let [columns, rows] = world.image.size;
        let step = f32::from(world.step);
        draw_texture(
            painter,
            lay,
            texture.id(),
            Vec2::ZERO,
            Vec2::new(step * columns as f32, step * rows as f32),
        );
        true
    }
}

/// The marker and zone files, with the hidden lists they were read with.
pub struct MapFiles {
    pub markers: Vec<MarkerFile>,
    pub zones: Vec<ZoneFile>,
    hidden_markers: Vec<String>,
    hidden_zones: Vec<String>,
}

impl MapFiles {
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
}

/// The marker and zone files, read once and again when the hidden lists
/// of the World Map page change or a file changed.
#[derive(Default)]
pub struct MapFilesCache {
    files: Option<MapFiles>,
}

impl MapFilesCache {
    pub fn get(&mut self, profile: &Profile) -> &MapFiles {
        let options = &profile.world_map;
        let stale = self.files.as_ref().is_none_or(|files| {
            files.hidden_markers != options.hidden_marker_files
                || files.hidden_zones != options.hidden_zone_files
        });
        if stale {
            self.files = None;
        }
        self.files.get_or_insert_with(|| {
            let dir = world_map::map_dir();
            MapFiles {
                markers: world_map::load_markers(&dir, &options.hidden_marker_files),
                zones: world_map::load_zones(&dir, &options.hidden_zone_files),
                hidden_markers: options.hidden_marker_files.clone(),
                hidden_zones: options.hidden_zone_files.clone(),
            }
        })
    }

    /// Reads the files again at the next use.
    pub fn reload(&mut self) {
        self.files = None;
    }
}

/// The colors and the font of the marks of one style.
pub struct MarkLook {
    pub font: FontId,
    /// Words get a dark shadow, so they read on any land.
    pub shadowed: bool,
    /// Dots are small squares, as the classic client draws them, or round.
    pub square_dots: bool,
    /// A marker whose file names no color.
    pub marker: Color32,
    pub waypoint: Color32,
    pub multi: Color32,
    pub party: Color32,
    pub guild: Color32,
    /// Where the character walks to.
    pub goal: Color32,
    /// The place the player looked for.
    pub looking: Color32,
    pub me: Color32,
    pub grid: Color32,
    pub mobile: fn(u8) -> Color32,
    /// Draws a small health bar: its track and its share from 0 to 1.
    pub health_bar: fn(&Painter, Rect, f32),
}

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

fn write(
    painter: &Painter,
    look: &MarkLook,
    at: Pos2,
    anchor: Align2,
    words: &str,
    color: Color32,
) {
    if look.shadowed {
        theme::shadowed_text(painter, at, anchor, words, look.font.clone(), color);
    } else {
        painter.text(at, anchor, words, look.font.clone(), color);
    }
}

fn dot(painter: &Painter, look: &MarkLook, at: Pos2, radius: f32, color: Color32) {
    if look.square_dots {
        painter.rect_filled(
            Rect::from_center_size(at, Vec2::splat(radius * HALF)),
            CornerRadius::ZERO,
            color,
        );
    } else {
        painter.circle_filled(at, radius, color);
    }
}

/// A small health bar under a dot of the map.
fn health_bar(painter: &Painter, look: &MarkLook, at: Pos2, share: f32) {
    let track = Rect::from_min_size(
        at + Vec2::new(-GROUP_BAR_WIDTH / HALF, SELF_RADIUS * HALF),
        Vec2::new(GROUP_BAR_WIDTH, GROUP_BAR_HEIGHT),
    );
    (look.health_bar)(painter, track, share);
}

/// Grid lines every few tiles, over the part of the map the field shows.
fn grid(painter: &Painter, field: Rect, lay: Lay, color: Color32) {
    let corners = [
        lay.tile(field.left_top()),
        lay.tile(field.right_top()),
        lay.tile(field.left_bottom()),
        lay.tile(field.right_bottom()),
    ];
    let low = corners.iter().fold(Vec2::splat(f32::MAX), |a, b| a.min(*b));
    let high = corners.iter().fold(Vec2::splat(f32::MIN), |a, b| a.max(*b));
    let stroke = Stroke::new(GRID_STROKE, color);
    let first = |from: f32| (from / GRID_TILES).floor() * GRID_TILES;
    let mut x = first(low.x);
    while x <= high.x {
        painter.line_segment([lay.screen(x, low.y), lay.screen(x, high.y)], stroke);
        x += GRID_TILES;
    }
    let mut y = first(low.y);
    while y <= high.y {
        painter.line_segment([lay.screen(low.x, y), lay.screen(high.x, y)], stroke);
        y += GRID_TILES;
    }
}

/// Everything a map carries over the land: the grid, the zones, the
/// markers, the marks of the shard, the houses, the mobiles, the party and
/// the guild, the goal, the place looked for, and the character.
pub fn overlays(painter: &Painter, field: Rect, lay: Lay, marks: &Marks<'_>, look: &MarkLook) {
    let Marks {
        frame,
        map,
        profile,
        files,
        session,
        looking_at,
    } = *marks;
    let options = &profile.world_map;
    if options.grid_when_zoomed && lay.tile_points() * GRID_TILES >= GRID_MIN_GAP {
        grid(painter, field, lay, look.grid);
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
        let color = theme::with_alpha(Color32::from_rgb(r, g, b), ZONE_ALPHA);
        let points: Vec<Pos2> = zone
            .corners
            .iter()
            .map(|(x, y)| lay.screen(f32::from(*x), f32::from(*y)))
            .collect();
        if let Some(first) = points.first().copied() {
            painter.add(Shape::closed_line(points, Stroke::new(ZONE_STROKE, color)));
            write(painter, look, first, Align2::LEFT_TOP, &zone.label, color);
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
                .map_or(look.marker, |[r, g, b]| Color32::from_rgb(r, g, b));
            dot(painter, look, at, DOT_RADIUS, color);
            if options.show_marker_names {
                let beside = at + Vec2::new(DOT_RADIUS * HALF, 0.0);
                write(
                    painter,
                    look,
                    beside,
                    Align2::LEFT_CENTER,
                    &marker.name,
                    color,
                );
            }
        }
    }
    if let Some((x, y)) = looking_at {
        painter.circle_stroke(
            lay.screen(f32::from(x), f32::from(y)),
            SELF_RADIUS * HALF,
            Stroke::new(GOAL_STROKE, look.looking),
        );
    }
    if map == frame.map {
        world_round_me(painter, lay, frame, options, look);
    }
}

/// The marks of the world round the character: the marks of the shard,
/// the houses, the mobiles, the party and the guild, the goal, and the
/// character himself.
fn world_round_me(
    painter: &Painter,
    lay: Lay,
    frame: &WatchFrame,
    options: &WorldMapOptions,
    look: &MarkLook,
) {
    // The marks the shard put on the map, each with its name.
    for mark in frame.waypoints.iter().filter(|mark| mark.map == frame.map) {
        let at = lay.screen(f32::from(mark.x), f32::from(mark.y));
        dot(painter, look, at, DOT_RADIUS, look.waypoint);
        let beside = at + Vec2::new(DOT_RADIUS * HALF, 0.0);
        write(
            painter,
            look,
            beside,
            Align2::LEFT_CENTER,
            &mark.name,
            look.waypoint,
        );
    }
    if options.show_multis {
        for multi in &frame.multis {
            let at = lay.screen(f32::from(multi.x), f32::from(multi.y));
            painter.rect_filled(
                Rect::from_center_size(at, Vec2::splat(MULTI_SIDE)),
                CornerRadius::ZERO,
                look.multi,
            );
        }
    }
    if options.show_mobiles {
        for mobile in &frame.mobiles {
            let at = lay.screen(f32::from(mobile.x), f32::from(mobile.y));
            dot(
                painter,
                look,
                at,
                DOT_RADIUS,
                (look.mobile)(mobile.notoriety),
            );
        }
    }
    if options.show_party {
        for member in world_map::group_on_map(frame) {
            let at = lay.screen(f32::from(member.x), f32::from(member.y));
            let color = if member.guild { look.guild } else { look.party };
            dot(painter, look, at, SELF_RADIUS, color);
            if options.show_group_names {
                let beside = at + Vec2::new(SELF_RADIUS * HALF, 0.0);
                write(
                    painter,
                    look,
                    beside,
                    Align2::LEFT_CENTER,
                    &member.name,
                    color,
                );
            }
            if let (true, Some(percent)) = (options.show_group_bars, member.hits_percent) {
                health_bar(painter, look, at, f32::from(percent) / PERCENT);
            }
        }
    }
    if let (Some(x), Some(y)) = (frame.dest_x, frame.dest_y) {
        painter.circle_stroke(
            lay.screen(f32::from(x), f32::from(y)),
            SELF_RADIUS,
            Stroke::new(GOAL_STROKE, look.goal),
        );
    }
    let me = lay.screen(f32::from(frame.x), f32::from(frame.y));
    dot(painter, look, me, SELF_RADIUS, look.me);
    if options.show_player_name {
        let beside = me + Vec2::new(SELF_RADIUS * HALF, 0.0);
        write(
            painter,
            look,
            beside,
            Align2::LEFT_CENTER,
            &frame.name,
            look.me,
        );
    }
    if options.show_player_bar && frame.hits_max > 0 {
        health_bar(
            painter,
            look,
            me,
            f32::from(frame.hits) / f32::from(frame.hits_max),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let east = turned(Vec2::new(1.0, 0.0), UNIT);
        assert!(east.x > 0.0 && east.y > 0.0, "east goes right and down");
        let south = turned(Vec2::new(0.0, 1.0), UNIT);
        assert!(south.x < 0.0 && south.y > 0.0, "south goes left and down");
        let tiles = Vec2::new(37.0, -12.0);
        let back = unturned(turned(tiles, UNIT), UNIT);
        assert!((back - tiles).length() < 0.001);
    }

    #[test]
    fn each_view_finds_the_tile_it_drew() {
        let views = [
            Lay::Turned {
                center: Pos2::new(200.0, 200.0),
                from: Vec2::new(1000.0, 1200.0),
                unit: 1.5,
            },
            Lay::NorthUp {
                center: Pos2::new(200.0, 200.0),
                middle: Vec2::new(3000.0, 2000.0),
                scale: 0.25,
            },
        ];
        for lay in views {
            let at = lay.screen(1010.0, 1195.0);
            assert!((lay.tile(at) - Vec2::new(1010.0, 1195.0)).length() < 0.01);
        }
        assert_eq!(whole_tile(Vec2::new(-4.0, 70_000.0)), (0, u16::MAX));
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
}
