//! The picture of a map item, such as a treasure map or a city map: the land
//! between its start and its end, drawn from the radar colors of the client
//! files, and the way between tiles of the world and pixels of the picture.

use crate::view::WatchMap;
use eframe::egui::{
    self, Color32, ColorImage, Pos2, Rect, TextureHandle, TextureId, TextureOptions, Vec2,
};

/// The largest picture made for a map item. A larger map is drawn into this
/// and shown at its own size.
pub const PICTURE_MAX: usize = 400;
/// The color of land the client files do not have.
pub const UNKNOWN: Color32 = Color32::from_rgb(28, 30, 34);
const TEXTURE_NAME: &str = "map-item";

/// The pixel of a map picture that a tile of the world lies on.
pub fn pixel_of(map: &WatchMap, x: u16, y: u16) -> (u16, u16) {
    let along = |start: u16, end: u16, of: u16, at: u16| {
        let span = u32::from(end.saturating_sub(start)).max(1);
        let from_start = u32::from(at.saturating_sub(start));
        (from_start * u32::from(of) / span) as u16
    };
    (
        along(map.start_x, map.end_x, map.width, x),
        along(map.start_y, map.end_y, map.height, y),
    )
}

/// The pixel of a map a point of its picture shows, when the picture is
/// drawn into `picture`. A point off the picture lands on its edge.
pub fn pixel_at(map: &WatchMap, picture: Rect, point: Pos2) -> (u16, u16) {
    let along = |from: f32, of: f32, pixels: u16| {
        let share = (from / of.max(1.0)).clamp(0.0, 1.0);
        (share * f32::from(pixels)).round() as u16
    };
    let on = point - picture.left_top();
    (
        along(on.x, picture.width(), map.width),
        along(on.y, picture.height(), map.height),
    )
}

/// The point of the picture drawn into `picture` that shows a pixel of the
/// map.
pub fn point_of(map: &WatchMap, picture: Rect, pixel: (u16, u16)) -> Pos2 {
    picture.left_top()
        + Vec2::new(
            f32::from(pixel.0) * picture.width() / f32::from(map.width.max(1)),
            f32::from(pixel.1) * picture.height() / f32::from(map.height.max(1)),
        )
}

/// How large the picture is in pixels, and how many tiles each side covers.
pub fn picture_size(map: &WatchMap) -> (usize, usize) {
    let across = usize::from(map.end_x.saturating_sub(map.start_x)).max(1);
    let down = usize::from(map.end_y.saturating_sub(map.start_y)).max(1);
    (across.min(PICTURE_MAX), down.min(PICTURE_MAX))
}

/// The tile of the world at one pixel of the picture.
pub fn tile_of(map: &WatchMap, pixels: (usize, usize), pixel: (usize, usize)) -> (u16, u16) {
    let span = |start: u16, end: u16, pixels: usize, at: usize| {
        let span = u32::from(end.saturating_sub(start));
        start.saturating_add((span * at as u32 / pixels.max(1) as u32) as u16)
    };
    (
        span(map.start_x, map.end_x, pixels.0, pixel.0),
        span(map.start_y, map.end_y, pixels.1, pixel.1),
    )
}

/// The land of a map from the radar color of each tile, which `radar`
/// gives for a facet and a tile. None when it gives no color at all.
pub fn land_image(
    map: &WatchMap,
    mut radar: impl FnMut(u8, u16, u16) -> Option<[u8; 3]>,
) -> Option<ColorImage> {
    let (across, down) = picture_size(map);
    let mut image = ColorImage::new([across, down], UNKNOWN);
    let mut any = false;
    for row in 0..down {
        for column in 0..across {
            let (x, y) = tile_of(map, (across, down), (column, row));
            if let Some([r, g, b]) = radar(map.facet, x, y) {
                image.pixels[row * across + column] = Color32::from_rgb(r, g, b);
                any = true;
            }
        }
    }
    any.then_some(image)
}

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
            self.made = land_image(map, radar).map(|image| {
                let texture = ctx.load_texture(TEXTURE_NAME, image, TextureOptions::LINEAR);
                (map.serial, texture)
            });
        }
        self.made.as_ref().map(|(_, texture)| texture.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn treasure_map() -> WatchMap {
        WatchMap {
            serial: 60,
            start_x: 1000,
            start_y: 1200,
            end_x: 1400,
            end_y: 1600,
            width: 200,
            height: 200,
            ..WatchMap::default()
        }
    }

    #[test]
    fn the_picture_covers_the_land_of_the_map() {
        let map = treasure_map();
        let (across, down) = picture_size(&map);
        assert_eq!((across, down), (400, 400));
        assert_eq!(tile_of(&map, (across, down), (0, 0)), (1000, 1200));
        assert_eq!(tile_of(&map, (across, down), (200, 200)), (1200, 1400));
        assert_eq!(tile_of(&map, (across, down), (399, 399)), (1399, 1599));
    }

    #[test]
    fn a_point_of_a_scaled_picture_finds_its_pixel_and_back() {
        let map = treasure_map();
        let picture = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::splat(400.0));
        assert_eq!(pixel_at(&map, picture, Pos2::new(210.0, 120.0)), (100, 50));
        assert_eq!(pixel_at(&map, picture, Pos2::new(0.0, 900.0)), (0, 200));
        assert_eq!(point_of(&map, picture, (100, 50)), Pos2::new(210.0, 120.0));
    }

    #[test]
    fn a_map_larger_than_the_picture_is_drawn_into_it() {
        let wide = WatchMap {
            end_x: 6000,
            end_y: 6000,
            ..treasure_map()
        };
        let (across, down) = picture_size(&wide);
        assert_eq!((across, down), (PICTURE_MAX, PICTURE_MAX));
        assert_eq!(tile_of(&wide, (across, down), (0, 0)), (1000, 1200));
        let (x, _) = tile_of(&wide, (across, down), (PICTURE_MAX / 2, 0));
        assert_eq!(x, 1000 + (6000 - 1000) / 2);
    }

    #[test]
    fn a_tile_of_the_world_finds_its_pixel_on_the_map() {
        let map = treasure_map();
        assert_eq!(pixel_of(&map, 1000, 1200), (0, 0));
        assert_eq!(pixel_of(&map, 1200, 1400), (100, 100));
        assert_eq!(pixel_of(&map, 1400, 1600), (200, 200));
        // A tile off the map lands on its edge.
        assert_eq!(pixel_of(&map, 900, 1100), (0, 0));
    }

    #[test]
    fn a_map_with_no_land_of_its_own_still_has_a_picture_size() {
        let empty = WatchMap {
            start_x: 500,
            end_x: 500,
            start_y: 500,
            end_y: 500,
            ..treasure_map()
        };
        assert_eq!(picture_size(&empty), (1, 1));
        assert_eq!(tile_of(&empty, (1, 1), (0, 0)), (500, 500));
    }

    #[test]
    fn the_land_takes_the_radar_colors_and_none_without_them() {
        let small = WatchMap {
            end_x: 1002,
            end_y: 1201,
            ..treasure_map()
        };
        let image = land_image(&small, |_, x, _| (x == 1001).then_some([1, 2, 3]))
            .expect("one tile has a color");
        assert_eq!(image.size, [2, 1]);
        assert_eq!(image.pixels, vec![UNKNOWN, Color32::from_rgb(1, 2, 3)]);
        assert!(land_image(&small, |_, _, _| None).is_none());
    }
}
