//! The picture of the land of a map item as an egui texture. The pixels
//! come from `uoterm_view::model::map_item::land_rgba`.

use crate::view::WatchMap;
use eframe::egui::{self, ColorImage, TextureHandle, TextureId, TextureOptions};
use uoterm_view::model::map_item::land_rgba;

const TEXTURE_NAME: &str = "map-item";

/// The picture of the land of one map, made once for each map.
#[derive(Default)]
pub struct LandPicture {
    made: Option<(u32, TextureHandle)>,
}

impl LandPicture {
    /// The picture of `map`, made the first time it is asked for. None when
    /// the client files have no colors for its land.
    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        map: &WatchMap,
        radar: impl FnMut(u8, u16, u16) -> Option<[u8; 3]>,
    ) -> Option<TextureId> {
        if self
            .made
            .as_ref()
            .is_none_or(|(serial, _)| *serial != map.serial)
        {
            self.made = land_rgba(map, radar).map(|(across, down, pixels)| {
                let image = ColorImage::from_rgba_premultiplied([across, down], &pixels);
                let texture = ctx.load_texture(TEXTURE_NAME, image, TextureOptions::LINEAR);
                (map.serial, texture)
            });
        }
        self.made.as_ref().map(|(_, texture)| texture.id())
    }
}
