//! The one large texture that holds every picture the map has drawn, so the
//! whole map is one mesh and one draw call. Where each picture goes is
//! `uoterm_view::atlas`.

use eframe::egui::{self, Color32, ColorImage, TextureHandle, TextureOptions};
use uoterm_view::art::Picture;
use uoterm_view::atlas::{Placement, ATLAS_SIDE};

const TEXTURE_NAME: &str = "uoterm-watch-art";
const OPTIONS: TextureOptions = TextureOptions::NEAREST;

pub struct Atlas {
    texture: TextureHandle,
}

impl Atlas {
    pub fn new(ctx: &egui::Context) -> Self {
        Self {
            texture: ctx.load_texture(TEXTURE_NAME, blank(), OPTIONS),
        }
    }

    pub fn texture_id(&self) -> egui::TextureId {
        self.texture.id()
    }

    /// Puts a picture at its place.
    pub fn put(&mut self, placement: Placement, picture: &Picture) {
        let image =
            ColorImage::from_rgba_unmultiplied([picture.width, picture.height], &picture.rgba);
        self.texture
            .set_partial([placement.x, placement.y], image, OPTIONS);
    }

    /// Clears every picture.
    pub fn clear(&mut self) {
        self.texture.set(blank(), OPTIONS);
    }
}

fn blank() -> ColorImage {
    ColorImage::new([ATLAS_SIDE, ATLAS_SIDE], Color32::TRANSPARENT)
}
