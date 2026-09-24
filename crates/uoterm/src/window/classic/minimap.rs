//! The minimap of the classic client: the land
//! round the character, turned as the game view is turned, drawn into the
//! dark window of its gump picture, two pixels to a tile. The mobiles
//! blink as small dots in the colors of their notoriety, and the character
//! as a white dot in the middle. A double click changes the small picture
//! for the large one and back.

use super::canvas::Canvas;
use super::registry::{well_known, GumpBody, GumpContext, GumpKind, GumpRules};
use crate::view::WatchFrame;
use crate::window::atlas::Sprite;
use crate::window::look::notoriety_hue;
use crate::window::scene::Scene;
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, TextureHandle, TextureOptions};
use uoterm_nav::ArtPixels;

pub const MINIMAP: GumpKind = GumpKind {
    id: well_known::MINIMAP,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(Minimap::default()),
};

const SMALL_GUMP: u16 = 5010;
const LARGE_GUMP: u16 = 5011;
/// The dark pixel of the gump picture the land is drawn into, with the
/// bit that says the pixel is drawn.
const MAP_WINDOW_PIXEL: u16 = 0x8421;
/// The land reaches this share of the gump's width each way from the
/// middle.
const REACH_SHARE: i32 = 4;
/// A tile is drawn this many pixels high.
const PIXELS_PER_TILE: i32 = 2;
/// The dots blink on and off this often, in seconds.
const BLINK_SECONDS: f64 = 0.5;
const BLINKS_PER_CYCLE: f64 = 2.0;
const DOT: i32 = 2;
const NO_HUE: u16 = 0;
const HALF: i32 = 2;
const TEXTURE_NAME: &str = "classic-minimap";
const WHOLE_UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

/// Where one tile a step from the character lands on the gump picture:
/// east goes right and down, south goes left and down.
fn tile_pixel(dx: i32, dy: i32, width: i32) -> (i32, i32) {
    let px = dx + width / HALF;
    (px - dy, px + dy)
}

/// The picture of the minimap: the gump picture, with the land drawn into
/// its dark window round the character.
fn map_picture(
    art: &ArtPixels,
    center: (u16, u16),
    mut land: impl FnMut(u16, u16) -> Option<[u8; 3]>,
) -> ColorImage {
    let (width, height) = (art.width as i32, art.height as i32);
    let rgba = art.rgba(None);
    let mut image = ColorImage::from_rgba_unmultiplied([art.width, art.height], &rgba);
    let reach_x = width / REACH_SHARE + PIXELS_PER_TILE;
    let reach_y = height / REACH_SHARE + PIXELS_PER_TILE;
    for dx in -reach_x - reach_y..=reach_x + reach_y {
        for dy in -reach_x - reach_y..=reach_x + reach_y {
            let (gx, gy) = tile_pixel(dx, dy, width);
            if gx < 0 || gx >= width || gy + PIXELS_PER_TILE <= 0 || gy >= height {
                continue;
            }
            let (Ok(x), Ok(y)) = (
                u16::try_from(i32::from(center.0) + dx),
                u16::try_from(i32::from(center.1) + dy),
            ) else {
                continue;
            };
            let mut color = None;
            for row in gy..gy + PIXELS_PER_TILE {
                if row < 0 || row >= height {
                    continue;
                }
                let at = (row * width + gx) as usize;
                if art.colors[at] != MAP_WINDOW_PIXEL {
                    continue;
                }
                let [r, g, b] = match color {
                    Some(rgb) => rgb,
                    None => match land(x, y) {
                        Some(rgb) => *color.insert(rgb),
                        None => break,
                    },
                };
                image.pixels[at] = Color32::from_rgb(r, g, b);
            }
        }
    }
    image
}

/// The picture made last, and where it was made for.
struct Made {
    map: u8,
    center: (u16, u16),
    large: bool,
    texture: TextureHandle,
    width: f32,
    height: f32,
}

#[derive(Default)]
pub struct Minimap {
    made: Option<Made>,
}

impl Minimap {
    /// The picture for the character's place, made again when he moved.
    fn picture(
        &mut self,
        ctx: &egui::Context,
        scene: &mut Scene,
        frame: &WatchFrame,
        large: bool,
    ) -> Option<&Made> {
        let center = (frame.x, frame.y);
        let fresh = self.made.as_ref().is_some_and(|made| {
            made.map == frame.map && made.center == center && made.large == large
        });
        if !fresh {
            let gump = if large { LARGE_GUMP } else { SMALL_GUMP };
            let art = scene.gump_pixels(gump)?;
            let image = map_picture(&art, center, |x, y| scene.radar_rgb(frame.map, x, y));
            self.made = Some(Made {
                map: frame.map,
                center,
                large,
                texture: ctx.load_texture(TEXTURE_NAME, image, TextureOptions::NEAREST),
                width: art.width as f32,
                height: art.height as f32,
            });
        }
        self.made.as_ref()
    }
}

impl GumpBody for Minimap {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if g.body_double_click() {
            cx.profile.world_map.minimap_large = !cx.profile.world_map.minimap_large;
            cx.profile_changed();
        }
        let frame = cx.frame;
        let ctx = g.ctx().clone();
        let large = cx.profile.world_map.minimap_large;
        let Some(made) = self.picture(&ctx, g.scene, frame, large) else {
            return;
        };
        let (width, height) = (made.width, made.height);
        let sprite = Sprite {
            uv: WHOLE_UV,
            width,
            height,
            anchor: egui::Vec2::ZERO,
        };
        g.sprite(0, 0, made.texture.id(), sprite);
        let time = ctx.input(|i| i.time);
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(BLINK_SECONDS));
        let shown = (time / BLINK_SECONDS) % BLINKS_PER_CYCLE < 1.0;
        if !shown {
            return;
        }
        let (width, height) = (width as i32, height as i32);
        for mobile in &frame.mobiles {
            let dx = i32::from(mobile.x) - i32::from(frame.x);
            let dy = i32::from(mobile.y) - i32::from(frame.y);
            let (gx, gy) = tile_pixel(dx, dy, width);
            if gx < 0 || gy < 0 || gx + DOT > width || gy + DOT > height {
                continue;
            }
            // A mobile of no notoriety keeps the red of the dot.
            match notoriety_hue(&cx.profile.combat, mobile.notoriety) {
                NO_HUE => g.fill(gx, gy, DOT, DOT, Color32::RED),
                hue => g.hue_box(gx, gy, DOT, DOT, hue),
            }
        }
        let (me_x, me_y) = (width / HALF, height / HALF);
        g.fill(me_x, me_y, DOT, DOT, Color32::WHITE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tile_east_goes_right_and_down_and_a_tile_south_left_and_down() {
        const WIDTH: i32 = 100;
        assert_eq!(tile_pixel(0, 0, WIDTH), (50, 50));
        assert_eq!(tile_pixel(1, 0, WIDTH), (51, 51));
        assert_eq!(tile_pixel(0, 1, WIDTH), (49, 51));
    }

    #[test]
    fn the_land_fills_only_the_dark_window_of_the_gump() {
        const SIDE: usize = 20;
        const FRAME_PIXEL: u16 = 0xFFFF;
        let mut colors = vec![FRAME_PIXEL; SIDE * SIDE];
        let window = SIDE / 2 * SIDE + SIDE / 2;
        colors[window] = MAP_WINDOW_PIXEL;
        let art = ArtPixels {
            width: SIDE,
            height: SIDE,
            colors,
        };
        let image = map_picture(&art, (100, 100), |_, _| Some([10, 200, 30]));
        assert_eq!(image.pixels[window], Color32::from_rgb(10, 200, 30));
        assert_ne!(image.pixels[0], Color32::from_rgb(10, 200, 30));
        let empty = map_picture(&art, (100, 100), |_, _| None);
        assert_ne!(empty.pixels[window], Color32::from_rgb(10, 200, 30));
    }

    #[test]
    fn the_minimap_draws_with_the_client_files() {
        use super::super::manager::GumpManager;
        use super::super::registry::GumpId;
        use super::super::testing::draw_frames;
        use crate::window::settings::Profile;
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        manager.open(GumpId::one(well_known::MINIMAP), &mut profile);
        let frame = WatchFrame {
            x: 1434,
            y: 1699,
            map: 1,
            ..WatchFrame::default()
        };
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert!(manager.is_open(&GumpId::one(well_known::MINIMAP)));
    }
}
