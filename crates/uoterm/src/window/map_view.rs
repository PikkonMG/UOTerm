//! The drawing the world maps of both styles share: the pictures of the
//! land from the radar colors of the client, the marker and zone files, the
//! named places of the session, and the marks painted over the land. How a
//! view lays tiles and where each mark goes are `uoterm_view::map_lay`. The
//! Modern map panel and the Classic world map gump each draw their own
//! frame round it.

pub use uoterm_view::map_lay::*;

use super::bridge;
use super::model::host;
use super::model::reads::{ReadCache, ReadKey};
use super::model::world_map::{self, Marker};
use super::scene::Scene;
use super::settings::Profile;
use super::theme;
use eframe::egui::{
    self, epaint::Vertex, Align2, Color32, ColorImage, CornerRadius, FontId, Mesh, Painter, Pos2,
    Rect, Shape, Stroke, TextureHandle, TextureId, TextureOptions, Vec2,
};
use serde_json::json;
use uoterm_runtime::tools::TOOL_FIND_LANDMARKS;

/// The wheel gives its step in points. This many points are one notch.
const WHEEL_NOTCH: f32 = 50.0;
/// The near picture is made again when its middle is this far away.
const REDRAW_TILES: u16 = 48;
/// The whole-world picture is at most this many pixels on its longer side.
const WORLD_PICTURE_SIDE: u16 = 1024;
/// The whole-world picture grows this many rows in each frame, so the
/// window does not stop while the facet is read.
const WORLD_ROWS_PER_FRAME: usize = 8;
const NEAR_TEXTURE: &str = "world-map";
const WORLD_TEXTURE: &str = "whole-world-map";
/// The named places of the session are read again this seldom, in seconds.
const LANDMARKS_MAX_AGE: f64 = 60.0;
const HALF: f32 = 2.0;
const WHOLE_UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

/// The notches the wheel turned over a field this frame: up is positive.
pub fn wheel_notches(ui: &egui::Ui, response: &egui::Response) -> f32 {
    if response.hovered() {
        ui.input(|input| input.raw_scroll_delta.y) / WHEEL_NOTCH
    } else {
        0.0
    }
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
            pos: bridge::pos2(lay.screen(x, y)),
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
        let Some(pixels) = near_pixels(middle, |x, y| scene.radar_rgb(map, x, y)) else {
            self.near = None;
            return false;
        };
        let image = ColorImage {
            size: [SPAN, SPAN],
            pixels: pixels.into_iter().map(bridge::color).collect(),
        };
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
        let half = Vec2::splat(SPAN as f32 / HALF);
        let middle = Vec2::new(f32::from(picture.middle.0), f32::from(picture.middle.1));
        draw_texture(
            painter,
            lay,
            picture.texture.id(),
            middle - half,
            middle + half,
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
                image: ColorImage::new(size, bridge::color(UNKNOWN_LAND)),
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

/// The marker and zone files, read once and again when the hidden lists
/// of the World Map page change or a file changed.
#[derive(Default)]
pub struct MapFilesCache {
    files: Option<MapFiles>,
}

impl MapFilesCache {
    pub fn get(&mut self, profile: &Profile) -> &MapFiles {
        let options = &profile.world_map;
        let stale = self
            .files
            .as_ref()
            .is_none_or(|files| !files.read_for(options));
        if stale {
            self.files = None;
        }
        self.files.get_or_insert_with(|| {
            let dir = host::world_map::map_dir();
            MapFiles {
                markers: host::world_map::load_markers(&dir, &options.hidden_marker_files),
                zones: host::world_map::load_zones(&dir, &options.hidden_zone_files),
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

/// How one style paints the marks: their colors, the font, whether words
/// get a shadow and dots are square, and its small health bar.
pub struct MarkStyle {
    pub look: MarkLook,
    pub font: FontId,
    /// Words get a dark shadow, so they read on any land.
    pub shadowed: bool,
    /// Dots are small squares, as the classic client draws them, or round.
    pub square_dots: bool,
    /// Draws a small health bar: its track and its share from 0 to 1.
    pub health_bar: fn(&Painter, Rect, f32),
}

/// Paints everything a map carries over the land, as
/// [`mark_layout`] places it.
pub fn overlays(painter: &Painter, field: Rect, lay: Lay, marks: &Marks<'_>, style: &MarkStyle) {
    for mark in mark_layout(bridge::area(field), lay, marks, &style.look) {
        match mark {
            MarkPlace::Line {
                from,
                to,
                width,
                color,
            } => {
                let stroke = Stroke::new(width, bridge::color(color));
                painter.line_segment([bridge::pos2(from), bridge::pos2(to)], stroke);
            }
            MarkPlace::Outline {
                points,
                width,
                color,
            } => {
                let points = points.into_iter().map(bridge::pos2).collect();
                let stroke = Stroke::new(width, bridge::color(color));
                painter.add(Shape::closed_line(points, stroke));
            }
            MarkPlace::Words {
                at,
                anchor,
                words,
                color,
            } => {
                let anchor = match anchor {
                    WordsAnchor::LeftTop => Align2::LEFT_TOP,
                    WordsAnchor::LeftCenter => Align2::LEFT_CENTER,
                };
                let (at, color) = (bridge::pos2(at), bridge::color(color));
                if style.shadowed {
                    theme::shadowed_text(painter, at, anchor, &words, style.font.clone(), color);
                } else {
                    painter.text(at, anchor, words, style.font.clone(), color);
                }
            }
            MarkPlace::Dot { at, radius, color } => {
                let (at, color) = (bridge::pos2(at), bridge::color(color));
                if style.square_dots {
                    let square = Rect::from_center_size(at, Vec2::splat(radius * HALF));
                    painter.rect_filled(square, CornerRadius::ZERO, color);
                } else {
                    painter.circle_filled(at, radius, color);
                }
            }
            MarkPlace::Square { at, side, color } => {
                let square = Rect::from_center_size(bridge::pos2(at), Vec2::splat(side));
                painter.rect_filled(square, CornerRadius::ZERO, bridge::color(color));
            }
            MarkPlace::Ring {
                at,
                radius,
                width,
                color,
            } => {
                let stroke = Stroke::new(width, bridge::color(color));
                painter.circle_stroke(bridge::pos2(at), radius, stroke);
            }
            MarkPlace::HealthBar { track, share } => {
                (style.health_bar)(painter, bridge::rect(track), share);
            }
        }
    }
}
