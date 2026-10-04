//! The light of the world over the map. The rules and the light map are
//! `uoterm_view::lights`; here the window uploads the map and lays it over
//! the world.

pub use uoterm_view::lights::*;

use super::bridge;
use eframe::egui::{Color32, ColorImage, Painter, Pos2, Rect, TextureHandle, TextureOptions, Vec2};

const TEXTURE_NAME: &str = "uoterm-light-map";

/// The texture of the light map of the last frame.
#[derive(Default)]
pub struct LightMap {
    texture: Option<TextureHandle>,
}

impl LightMap {
    /// Lays the light map over `rect`, from its top left corner.
    pub fn draw(&mut self, painter: &Painter, rect: Rect, light: &LightCells) {
        let image = ColorImage {
            size: [light.width, light.height],
            pixels: light
                .cells
                .iter()
                .map(|cell| bridge::color(*cell))
                .collect(),
        };
        let texture = self.texture.get_or_insert_with(|| {
            painter
                .ctx()
                .load_texture(TEXTURE_NAME, image.clone(), TextureOptions::LINEAR)
        });
        texture.set(image, TextureOptions::LINEAR);
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let full = Rect::from_min_size(
            rect.min,
            Vec2::new(light.width as f32, light.height as f32) * LIGHT_CELL,
        );
        painter.image(texture.id(), full, uv, Color32::WHITE);
    }
}
