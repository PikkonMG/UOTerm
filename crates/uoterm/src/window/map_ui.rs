//! The map of the world round the character, seen from far above and
//! turned as the play field is turned. A click on it walks the character
//! to that place, while the human has control.

use super::control::{Act, Hand};
use super::scene::Scene;
use super::theme::{self, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{
    self, epaint::Vertex, Align2, Color32, ColorImage, Id, Mesh, Pos2, Rect, Sense, Shape,
    TextureHandle, TextureOptions, Vec2,
};

/// How many tiles one side of the picture covers.
const SPAN: usize = 256;
/// The picture is made again when the character is this far from its middle.
const REDRAW_TILES: u16 = 48;
const PANEL_SIDE: f32 = 470.0;
const TITLE_ROW: f32 = 30.0;
const DOT_RADIUS: f32 = 2.5;
const SELF_RADIUS: f32 = 4.0;
const UNKNOWN: Color32 = Color32::from_rgb(10, 12, 18);

const WORDS_TITLE: &str = "Map";
const WORDS_NO_FILES: &str = "The map needs the client files.";
const HINT_WALK: &str = "Click: walk there.";

struct Picture {
    map: u8,
    /// The tile in the middle of the picture.
    middle: (u16, u16),
    texture: TextureHandle,
}

pub struct MapUi {
    open: bool,
    picture: Option<Picture>,
}

/// Where a tile is on the turned map, as a step from the character. One
/// tile east goes right and down, one tile south goes left and down.
fn turned(tiles: Vec2, unit: f32) -> Vec2 {
    Vec2::new(tiles.x - tiles.y, tiles.x + tiles.y) * unit
}

/// The tiles from the character for a step on the turned map.
fn unturned(on_screen: Vec2, unit: f32) -> Vec2 {
    let (a, b) = (on_screen.x / unit, on_screen.y / unit);
    Vec2::new((a + b) / 2.0, (b - a) / 2.0)
}

impl MapUi {
    pub fn starting(open: bool) -> Self {
        Self {
            open,
            picture: None,
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    fn picture_of(&mut self, ui: &egui::Ui, frame: &WatchFrame, scene: &mut Scene) -> bool {
        let fresh = self.picture.as_ref().is_some_and(|picture| {
            picture.map == frame.map
                && picture.middle.0.abs_diff(frame.x) < REDRAW_TILES
                && picture.middle.1.abs_diff(frame.y) < REDRAW_TILES
        });
        if fresh {
            return true;
        }
        let half = (SPAN / 2) as i32;
        let mut image = ColorImage::new([SPAN, SPAN], UNKNOWN);
        let mut any = false;
        for row in 0..SPAN {
            for column in 0..SPAN {
                let x = i32::from(frame.x) + column as i32 - half;
                let y = i32::from(frame.y) + row as i32 - half;
                let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                    continue;
                };
                if let Some([r, g, b]) = scene.radar_rgb(frame.map, x, y) {
                    image.pixels[row * SPAN + column] = Color32::from_rgb(r, g, b);
                    any = true;
                }
            }
        }
        if !any {
            self.picture = None;
            return false;
        }
        self.picture = Some(Picture {
            map: frame.map,
            middle: (frame.x, frame.y),
            texture: ui
                .ctx()
                .load_texture("world-map", image, TextureOptions::NEAREST),
        });
        true
    }

    /// Draws the map when it is open. Gives the place it covers.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
        hand: &Hand,
    ) -> Option<Rect> {
        if !self.open {
            return None;
        }
        let panel = Rect::from_center_size(
            rect.center(),
            Vec2::new(PANEL_SIDE, PANEL_SIDE + TITLE_ROW) + Vec2::splat(theme::PANEL_PAD * 2.0),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            WORDS_TITLE,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let field = Rect::from_min_max(inner.left_top() + Vec2::new(0.0, TITLE_ROW), inner.max);
        if !self.picture_of(ui, frame, scene) {
            ui.painter().text(
                field.center(),
                Align2::CENTER_CENTER,
                WORDS_NO_FILES,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
            return Some(panel);
        }
        let Some(picture) = &self.picture else {
            return Some(panel);
        };
        // The turned picture is a diamond as wide as the field.
        let unit = field.width() / (SPAN as f32 * 2.0);
        let center = field.center();
        let from_character = Vec2::new(
            f32::from(picture.middle.0) - f32::from(frame.x),
            f32::from(picture.middle.1) - f32::from(frame.y),
        );
        let half = SPAN as f32 / 2.0;
        let corners = [(-half, -half), (half, -half), (half, half), (-half, half)];
        let uvs = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let mut mesh = Mesh::with_texture(picture.texture.id());
        for ((dx, dy), (u, v)) in corners.into_iter().zip(uvs) {
            mesh.vertices.push(Vertex {
                pos: center + turned(from_character + Vec2::new(dx, dy), unit),
                uv: Pos2::new(u, v),
                color: Color32::WHITE,
            });
        }
        mesh.indices.extend([0, 1, 2, 0, 2, 3]);
        let painter = ui.painter().with_clip_rect(field);
        painter.add(Shape::mesh(mesh));
        for mobile in &frame.mobiles {
            let step = Vec2::new(
                f32::from(mobile.x) - f32::from(frame.x),
                f32::from(mobile.y) - f32::from(frame.y),
            );
            painter.circle_filled(
                center + turned(step, unit),
                DOT_RADIUS,
                theme::notoriety_color(mobile.notoriety),
            );
        }
        if let (Some(x), Some(y)) = (frame.dest_x, frame.dest_y) {
            let step = Vec2::new(
                f32::from(x) - f32::from(frame.x),
                f32::from(y) - f32::from(frame.y),
            );
            painter.circle_stroke(
                center + turned(step, unit),
                SELF_RADIUS,
                egui::Stroke::new(1.5, theme::GOAL),
            );
        }
        // The marks the shard put on the map, each with its name.
        for mark in frame.waypoints.iter().filter(|mark| mark.map == frame.map) {
            let step = Vec2::new(
                f32::from(mark.x) - f32::from(frame.x),
                f32::from(mark.y) - f32::from(frame.y),
            );
            let at = center + turned(step, unit);
            painter.circle_filled(at, DOT_RADIUS, theme::WAITING);
            painter.text(
                at + Vec2::new(DOT_RADIUS * 2.0, 0.0),
                Align2::LEFT_CENTER,
                &mark.name,
                text_font(theme::SIZE_SMALL),
                theme::WAITING,
            );
        }
        painter.circle_filled(center, SELF_RADIUS, theme::SELF_FIGURE);
        let response = ui.interact(field, Id::new("world-map"), Sense::click());
        if !frame.human_control {
            return Some(panel);
        }
        if let Some(mouse) = response.hover_pos() {
            super::tips::label(ui, HINT_WALK, "");
            if response.clicked() {
                let tiles = unturned(mouse - center, unit);
                let x = (f32::from(frame.x) + tiles.x).round().max(0.0) as u16;
                let y = (f32::from(frame.y) + tiles.y).round().max(0.0) as u16;
                hand.act(Act::WalkTo { x, y });
            }
        }
        Some(panel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
