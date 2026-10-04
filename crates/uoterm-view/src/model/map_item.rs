//! The picture of a map item, such as a treasure map or a city map: the land
//! between its start and its end, drawn from the radar colors of the client
//! files, and the way between tiles of the world and pixels of the picture.

use crate::frame::WatchMap;
use crate::geom::{Area, Point, Rgba, Vector};

/// The largest picture made for a map item. A larger map is drawn into this
/// and shown at its own size.
pub const PICTURE_MAX: usize = 400;
/// Where a web page gets the land of a map item:
/// `{MAP_ITEM_PREFIX}/{facet}/{start_x}/{start_y}/{end_x}/{end_y}`.
pub const MAP_ITEM_PREFIX: &str = "/v1/map-item";
/// The color of land the client files do not have.
pub const UNKNOWN: Rgba = Rgba::from_rgba_premultiplied(28, 30, 34, OPAQUE);
const OPAQUE: u8 = u8::MAX;

/// The path of the land of a map item.
pub fn map_item_path(map: &WatchMap) -> String {
    format!(
        "{MAP_ITEM_PREFIX}/{}/{}/{}/{}/{}",
        map.facet, map.start_x, map.start_y, map.end_x, map.end_y
    )
}

/// The map item a path names the land of: its facet and its corners.
/// None when its end is not past its start.
pub fn map_of_path(facet: u8, start: (u16, u16), end: (u16, u16)) -> Option<WatchMap> {
    (end.0 > start.0 && end.1 > start.1).then(|| WatchMap {
        facet,
        start_x: start.0,
        start_y: start.1,
        end_x: end.0,
        end_y: end.1,
        ..WatchMap::default()
    })
}

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
pub fn pixel_at(map: &WatchMap, picture: Area, point: Point) -> (u16, u16) {
    let along = |from: f32, of: f32, pixels: u16| {
        let share = (from / of.max(1.0)).clamp(0.0, 1.0);
        (share * f32::from(pixels)).round() as u16
    };
    let on = point - picture.min;
    (
        along(on.x, picture.width(), map.width),
        along(on.y, picture.height(), map.height),
    )
}

/// The point of the picture drawn into `picture` that shows a pixel of the
/// map.
pub fn point_of(map: &WatchMap, picture: Area, pixel: (u16, u16)) -> Point {
    picture.min
        + Vector::new(
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
/// gives for a facet and a tile: the width and the height of the picture,
/// and its pixels as red, green, blue and alpha, row by row. None when it
/// gives no color at all.
pub fn land_rgba(
    map: &WatchMap,
    mut radar: impl FnMut(u8, u16, u16) -> Option<[u8; 3]>,
) -> Option<(usize, usize, Vec<u8>)> {
    let (across, down) = picture_size(map);
    let mut pixels = Vec::with_capacity(across * down * UNKNOWN.0.len());
    let mut any = false;
    for row in 0..down {
        for column in 0..across {
            let (x, y) = tile_of(map, (across, down), (column, row));
            let color = match radar(map.facet, x, y) {
                Some([r, g, b]) => {
                    any = true;
                    [r, g, b, OPAQUE]
                }
                None => UNKNOWN.to_array(),
            };
            pixels.extend_from_slice(&color);
        }
    }
    any.then_some((across, down, pixels))
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
    fn the_path_of_the_land_names_its_corners() {
        let map = treasure_map();
        assert_eq!(map_item_path(&map), "/v1/map-item/0/1000/1200/1400/1600");
        let read = map_of_path(0, (1000, 1200), (1400, 1600)).unwrap();
        assert_eq!(picture_size(&read), picture_size(&map));
        assert!(map_of_path(0, (5, 5), (5, 9)).is_none());
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
        let picture = Area::from_min_size(Point::new(10.0, 20.0), Vector::new(400.0, 400.0));
        assert_eq!(pixel_at(&map, picture, Point::new(210.0, 120.0)), (100, 50));
        assert_eq!(pixel_at(&map, picture, Point::new(0.0, 900.0)), (0, 200));
        assert_eq!(point_of(&map, picture, (100, 50)), Point::new(210.0, 120.0));
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
        let (across, down, pixels) = land_rgba(&small, |_, x, _| (x == 1001).then_some([1, 2, 3]))
            .expect("one tile has a color");
        assert_eq!((across, down), (2, 1));
        let [r, g, b, a] = UNKNOWN.to_array();
        assert_eq!(pixels, vec![r, g, b, a, 1, 2, 3, OPAQUE]);
        assert!(land_rgba(&small, |_, _, _| None).is_none());
    }
}
