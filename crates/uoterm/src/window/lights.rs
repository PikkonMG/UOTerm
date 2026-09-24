//! The light of the world, as the reference client makes it. The world is as bright as
//! the light level of the shard, or of the player's own choice, and each
//! torch, lamp and lit window adds its pool of light. The pools are added
//! on a light map, and the map then darkens the world under it.
//!
//! The reference client multiplies the world by the light map on the graphics card.
//! egui blends with premultiplied alpha only, so the map darkens each spot
//! by its weakest color channel, and the rest of a colored light shows as a
//! glow of that color over it.

use super::settings::{LightLevelRule, VideoOptions};
use eframe::egui::{Color32, ColorImage, Painter, Pos2, Rect, TextureHandle, TextureOptions, Vec2};
use std::f64::consts::TAU;
use uoterm_nav::{LightShape, LIGHT_LEVEL_MAX};

/// The darkest level the shard may send.
pub const LIGHT_LEVEL_DARKEST: u8 = 0x1E;
/// A level counts in steps of one part in this many.
const LEVEL_STEPS: f32 = 32.0;
/// Dark nights take this much more light away.
const DARK_NIGHTS_DROP: f32 = 0.04;
/// The light map has one cell for each square of this many points.
const LIGHT_CELL: f32 = 4.0;
/// A light byte of a shape becomes a grey channel by this shift.
const LEVEL_TO_CHANNEL_SHIFT: u32 = 3;
const CHANNEL_MAX: f32 = 255.0;
/// The share of a colored light that shows as a glow over the dark.
const GLOW_BASE: f32 = 0.5;
/// Alternative lights brighten the world by this share of each light, and
/// do not darken it.
const ALTERNATIVE_GLOW: f32 = 0.5;
const TEXTURE_NAME: &str = "uoterm-light-map";
/// A flickering light dims by at most this share of its strength.
const FLICKER_DEPTH: f32 = 0.2;
/// The two waves of a flicker, in turns each second. They do not line up,
/// so the flicker does not repeat soon.
const FLICKER_WAVES: [f64; 2] = [7.3, 11.9];
/// Spreads the seeds of the lights over a whole turn, so lights side by
/// side do not flicker together.
const SEED_SPREAD: f64 = 0.618_033_988_75;

/// The light choices of the Video page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightRules {
    pub custom_level: Option<(u8, LightLevelRule)>,
    pub alternative: bool,
    pub dark_nights: bool,
    pub colored: bool,
    pub candle_flicker: bool,
}

impl From<&VideoOptions> for LightRules {
    fn from(video: &VideoOptions) -> Self {
        Self {
            custom_level: video
                .custom_light_level
                .then_some((video.light_level, video.light_level_rule)),
            alternative: video.alternative_lights,
            dark_nights: video.dark_nights,
            colored: video.colored_lights,
            candle_flicker: video.candle_flicker,
        }
    }
}

/// How bright the world is, and whether lights show at all.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldLight {
    /// From 0 for black to 1 for full day.
    pub level: f32,
    /// The world is darker than the character's own light: lamps show.
    pub lights_on: bool,
}

/// The light of the world for the light level of the shard and the
/// personal light of the character, under the player's own choice.
pub fn world_light(shard_level: u8, personal: u8, rules: &LightRules) -> WorldLight {
    let real_overall = shard_level.min(LIGHT_LEVEL_DARKEST);
    let real_personal = personal.min(LIGHT_LEVEL_DARKEST);
    let (overall, personal) = match rules.custom_level {
        Some((level, LightLevelRule::Minimum)) => (real_overall.min(level), 0),
        Some((level, LightLevelRule::Absolute)) => (level.min(LIGHT_LEVEL_DARKEST), 0),
        None => (real_overall, real_personal),
    };
    let reverted = LEVEL_STEPS - f32::from(overall);
    let mut level = f32::from(personal).max(reverted) / LEVEL_STEPS;
    if rules.dark_nights {
        level -= DARK_NIGHTS_DROP;
    }
    WorldLight {
        level: level.clamp(0.0, 1.0),
        lights_on: personal < overall,
    }
}

/// How a light color turns its light byte into each channel.
#[derive(Clone, Copy)]
enum Curve {
    Standard,
    Small,
    Dim,
    Flat,
    Medium,
    Halo,
}

const CURVE_STEPS: usize = LIGHT_LEVEL_MAX as usize + 1;

impl Curve {
    fn table(self) -> &'static [u8; CURVE_STEPS] {
        const STANDARD: [u8; CURVE_STEPS] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31,
        ];
        const SMALL: [u8; CURVE_STEPS] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 6, 8, 10, 12, 14, 16, 18,
            20, 22, 24, 26, 28,
        ];
        const DIM: [u8; CURVE_STEPS] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5,
            6, 7, 8,
        ];
        const FLAT: [u8; CURVE_STEPS] = [
            0, 1, 2, 4, 6, 8, 11, 14, 17, 20, 23, 26, 29, 30, 31, 31, 31, 31, 31, 31, 31, 31, 31,
            31, 31, 31, 31, 31, 31, 31, 31, 31,
        ];
        const MEDIUM: [u8; CURVE_STEPS] = [
            0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 17,
            19, 21, 23, 25, 27,
        ];
        const HALO: [u8; CURVE_STEPS] = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 5, 10, 15, 20, 25, 30, 30, 18,
            18, 18, 18, 18, 18, 18,
        ];
        match self {
            Self::Standard => &STANDARD,
            Self::Small => &SMALL,
            Self::Dim => &DIM,
            Self::Flat => &FLAT,
            Self::Medium => &MEDIUM,
            Self::Halo => &HALO,
        }
    }
}

/// One light color of the classic client: its full color and the curve of
/// its red, green and blue.
struct LightColor {
    id: u8,
    rgb: [u8; 3],
    curves: [Curve; 3],
}

const fn light_color(id: u8, rgb: u32, curves: [Curve; 3]) -> LightColor {
    const RED_SHIFT: u32 = 16;
    const GREEN_SHIFT: u32 = 8;
    const BYTE: u32 = 0xFF;
    LightColor {
        id,
        rgb: [
            ((rgb >> RED_SHIFT) & BYTE) as u8,
            ((rgb >> GREEN_SHIFT) & BYTE) as u8,
            (rgb & BYTE) as u8,
        ],
        curves,
    }
}

/// The light colors that are not plain white, as the reference client makes them.
const LIGHT_COLORS: [LightColor; 14] = {
    use Curve::*;
    [
        light_color(1, 0x00FF00, [Standard, Small, Standard]),
        light_color(2, 0x7F7FFF, [Standard, Standard, Standard]),
        light_color(6, 0xFF00FF, [Dim, Standard, Small]),
        light_color(10, 0x3F3FFF, [Standard, Standard, Standard]),
        light_color(20, 0x00FF00, [Standard, Standard, Standard]),
        light_color(30, 0xFF7F00, [Flat, Flat, Standard]),
        light_color(31, 0xFF7F00, [Small, Small, Standard]),
        light_color(32, 0xFF00FF, [Standard, Standard, Standard]),
        light_color(40, 0xFF0000, [Standard, Standard, Standard]),
        light_color(50, 0xFFFF00, [Standard, Standard, Standard]),
        light_color(60, 0xFFFF00, [Small, Small, Standard]),
        light_color(61, 0xFFFF00, [Medium, Medium, Standard]),
        light_color(62, 0xFFFFFF, [Medium, Medium, Medium]),
        light_color(63, 0xFFFFFF, [Halo, Halo, Halo]),
    ]
};

/// The light of one light byte in one light color, from 0 to 1 for each
/// channel. A color the table does not name is plain white.
pub fn light_rgb(color: Option<u8>, level: u8) -> [f32; 3] {
    let step = usize::from(level.min(LIGHT_LEVEL_MAX));
    let grey = f32::from(level.min(LIGHT_LEVEL_MAX) << LEVEL_TO_CHANNEL_SHIFT) / CHANNEL_MAX;
    let Some(entry) = color.and_then(|color| LIGHT_COLORS.iter().find(|c| c.id == color)) else {
        return [grey; 3];
    };
    [0, 1, 2].map(|channel| {
        let curved = f32::from(entry.curves[channel].table()[step]) / f32::from(LIGHT_LEVEL_MAX);
        curved * f32::from(entry.rgb[channel]) / CHANNEL_MAX
    })
}

/// The light colors the reference client gives some graphics of its own, whatever the
/// files say. The later lists win over the earlier ones.
const FIRST_COLORS: [(u16, u16, u8); 11] = [
    (0x09FB, 0x0A14, 30),
    (0x0A15, 0x0A29, 0),
    (0x0B1A, 0x0B28, 0),
    (0x0DE1, 0x0DEA, 31),
    (0x1849, 0x1850, 61),
    (0x1853, 0x185A, 61),
    (0x197A, 0x19A9, 60),
    (0x19AB, 0x19B6, 60),
    (0x1ECD, 0x1ED2, 1),
    (0x088C, 0x088C, 31),
    (0x0FAC, 0x0FAC, 30),
];
const LONE_COLORS: [(u16, u8); 5] = [
    (0x0FB1, 60),
    (0x1647, 61),
    (0x19BB, 40),
    (0x1F2B, 40),
    (0x9F66, 0),
];
const BLUE_FLAMES: [u16; 2] = [0x1FD4, 0x0F6C];
const BLUE_FLAME_COLOR: u8 = 2;
const LAST_COLORS: [(u16, u16, u8); 19] = [
    (0x0E2D, 0x0E30, 62),
    (0x0E31, 0x0E33, 40),
    (0x0E5C, 0x0E6A, 6),
    (0x12EE, 0x134D, 31),
    (0x306A, 0x329B, 31),
    (0x343B, 0x346C, 31),
    (0x3547, 0x354C, 31),
    (0x3914, 0x3929, 1),
    (0x3946, 0x3964, 6),
    (0x3967, 0x397A, 6),
    (0x398C, 0x399F, 31),
    (0x3E02, 0x3E0B, 1),
    (0x3E27, 0x3E3A, 31),
    (0x40FE, 0x40FE, 40),
    (0x40FF, 0x40FF, 10),
    (0x4100, 0x4100, 20),
    (0x4101, 0x4101, 32),
    (0x983B, 0x983D, 30),
    (0x983F, 0x9841, 30),
];

fn in_ranges(graphic: u16, ranges: &[(u16, u16, u8)]) -> Option<u8> {
    ranges
        .iter()
        .find(|(from, to, _)| (*from..=*to).contains(&graphic))
        .map(|(_, _, color)| *color)
}

/// The light color the reference client gives a graphic of its own, or None.
fn graphic_light_color(graphic: u16) -> Option<u8> {
    in_ranges(graphic, &LAST_COLORS)
        .or_else(|| BLUE_FLAMES.contains(&graphic).then_some(BLUE_FLAME_COLOR))
        .or_else(|| in_ranges(graphic, &FIRST_COLORS))
        .or_else(|| {
            LONE_COLORS
                .iter()
                .find(|(lone, _)| *lone == graphic)
                .map(|(_, color)| *color)
        })
}

/// The color a light of `graphic` shows in, from the color its light number
/// names. Plain white when colored lights are off. Color zero is white.
pub fn shown_light_color(rules: &LightRules, graphic: u16, named: Option<u8>) -> Option<u8> {
    if !rules.colored {
        return None;
    }
    graphic_light_color(graphic)
        .or(named)
        .filter(|color| *color != 0)
}

/// How strong a flickering light is at `time` seconds, from 1 for full
/// down to `1 - FLICKER_DEPTH`. `seed` tells one light from the next.
pub fn flicker(seed: u32, time: f64) -> f32 {
    let phase = (f64::from(seed) * SEED_SPREAD).fract() * TAU;
    let wave = FLICKER_WAVES
        .iter()
        .map(|turns| ((time * turns).fract() * TAU + phase).sin())
        .sum::<f64>()
        / FLICKER_WAVES.len() as f64;
    let dim = (1.0 + wave) / 2.0;
    1.0 - FLICKER_DEPTH * dim as f32
}

/// One pool of light on the screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightSource {
    pub center: Pos2,
    pub shape: u8,
    pub color: Option<u8>,
    /// The share of its light it throws now: below 1 while it flickers.
    pub strength: f32,
}

/// The light map of the last frame, and its texture.
#[derive(Default)]
pub struct LightMap {
    texture: Option<TextureHandle>,
    cells: Vec<[f32; 3]>,
}

impl LightMap {
    /// Adds each light to a map that starts at the light of the world, and
    /// lays the map over `rect`. `shape` gives the light shape of a number.
    #[allow(clippy::too_many_arguments)]
    pub fn draw<'a>(
        &mut self,
        painter: &Painter,
        rect: Rect,
        light: WorldLight,
        alternative: bool,
        sources: &[LightSource],
        zoom: f32,
        mut shape: impl FnMut(u8) -> Option<&'a LightShape>,
    ) {
        let width = (rect.width() / LIGHT_CELL).ceil().max(1.0) as usize;
        let height = (rect.height() / LIGHT_CELL).ceil().max(1.0) as usize;
        let base = if alternative { 0.0 } else { light.level };
        self.cells.clear();
        self.cells.resize(width * height, [base; 3]);
        if light.lights_on || alternative {
            for source in sources {
                if let Some(shape) = shape(source.shape) {
                    add_light(&mut self.cells, width, rect, source, shape, zoom);
                }
            }
        }
        let pixels = self
            .cells
            .iter()
            .map(|cell| overlay_pixel(*cell, alternative))
            .collect();
        let image = ColorImage {
            size: [width, height],
            pixels,
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
            Vec2::new(width as f32, height as f32) * LIGHT_CELL,
        );
        painter.image(texture.id(), full, uv, Color32::WHITE);
    }
}

/// Adds one light shape to the map, centered on its source.
fn add_light(
    cells: &mut [[f32; 3]],
    width: usize,
    rect: Rect,
    source: &LightSource,
    shape: &LightShape,
    zoom: f32,
) {
    let size = Vec2::new(shape.width as f32, shape.height as f32) * zoom;
    let area = Rect::from_center_size(source.center, size).intersect(rect);
    if area.width() <= 0.0 || area.height() <= 0.0 {
        return;
    }
    let first_column = ((area.left() - rect.left()) / LIGHT_CELL).floor() as usize;
    let last_column = ((area.right() - rect.left()) / LIGHT_CELL).ceil() as usize;
    let first_row = ((area.top() - rect.top()) / LIGHT_CELL).floor() as usize;
    let last_row = ((area.bottom() - rect.top()) / LIGHT_CELL).ceil() as usize;
    let origin = source.center - size / 2.0;
    for row in first_row..last_row {
        for column in first_column..last_column.min(width) {
            let middle = rect.min + Vec2::new(column as f32 + 0.5, row as f32 + 0.5) * LIGHT_CELL;
            let on_shape = (middle - origin) / zoom;
            if on_shape.x < 0.0 || on_shape.y < 0.0 {
                continue;
            }
            let (x, y) = (on_shape.x as usize, on_shape.y as usize);
            if x >= shape.width || y >= shape.height {
                continue;
            }
            let level = shape.levels[y * shape.width + x];
            if level == 0 {
                continue;
            }
            let Some(cell) = cells.get_mut(row * width + column) else {
                continue;
            };
            let added = light_rgb(source.color, level);
            for channel in 0..3 {
                cell[channel] += added[channel] * source.strength;
            }
        }
    }
}

/// The overlay pixel that turns the world under it into the light of one
/// cell of the map.
fn overlay_pixel(cell: [f32; 3], alternative: bool) -> Color32 {
    let [red, green, blue] = cell.map(|c| c.clamp(0.0, 1.0));
    let byte = |share: f32| (share.clamp(0.0, 1.0) * CHANNEL_MAX).round() as u8;
    if alternative {
        let glow = |c: f32| byte(c * ALTERNATIVE_GLOW);
        return Color32::from_rgba_premultiplied(glow(red), glow(green), glow(blue), 0);
    }
    let weakest = red.min(green).min(blue);
    let glow = |c: f32| byte((c - weakest) * GLOW_BASE);
    Color32::from_rgba_premultiplied(glow(red), glow(green), glow(blue), byte(1.0 - weakest))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: LightRules = LightRules {
        custom_level: None,
        alternative: false,
        dark_nights: false,
        colored: true,
        candle_flicker: false,
    };

    #[test]
    fn day_is_full_light_and_the_darkest_night_keeps_some_light() {
        let day = world_light(0, 0, &PLAIN);
        assert_eq!(day.level, 1.0);
        assert!(!day.lights_on);
        let night = world_light(u8::MAX, 0, &PLAIN);
        assert_eq!(night.level, 2.0 / LEVEL_STEPS);
        assert!(night.lights_on);
        let dark = LightRules {
            dark_nights: true,
            ..PLAIN
        };
        assert!(world_light(u8::MAX, 0, &dark).level < night.level);
    }

    #[test]
    fn a_personal_light_brightens_the_night_and_turns_the_lamps_off() {
        let lit = world_light(LIGHT_LEVEL_DARKEST, LIGHT_LEVEL_DARKEST, &PLAIN);
        assert!(lit.level > world_light(LIGHT_LEVEL_DARKEST, 0, &PLAIN).level);
        assert!(!lit.lights_on);
    }

    #[test]
    fn a_custom_level_replaces_the_shard_or_only_caps_it() {
        const CHOSEN: u8 = 10;
        let absolute = LightRules {
            custom_level: Some((CHOSEN, LightLevelRule::Absolute)),
            ..PLAIN
        };
        assert_eq!(
            world_light(0, 0, &absolute).level,
            (LEVEL_STEPS - f32::from(CHOSEN)) / LEVEL_STEPS
        );
        let minimum = LightRules {
            custom_level: Some((CHOSEN, LightLevelRule::Minimum)),
            ..PLAIN
        };
        assert_eq!(world_light(0, 0, &minimum).level, 1.0);
        assert_eq!(
            world_light(LIGHT_LEVEL_DARKEST, 0, &minimum),
            world_light(0, 0, &absolute)
        );
    }

    #[test]
    fn a_plain_light_is_grey_and_a_colored_one_follows_its_curves() {
        let full = light_rgb(None, LIGHT_LEVEL_MAX);
        assert!(full.iter().all(|c| (c - 248.0 / CHANNEL_MAX).abs() < 1e-6));
        const RED: u8 = 40;
        let red = light_rgb(Some(RED), LIGHT_LEVEL_MAX);
        assert_eq!(red, [1.0, 0.0, 0.0]);
        // The small curve gives no light at half the level.
        const LANTERN: u8 = 31;
        assert_eq!(light_rgb(Some(LANTERN), 10)[0], 0.0);
    }

    #[test]
    fn colored_lights_follow_the_setting_and_the_graphic() {
        const TORCH: u16 = 0x0A12;
        const BRAZIER: u16 = 0x0E31;
        assert_eq!(shown_light_color(&PLAIN, TORCH, None), Some(30));
        assert_eq!(shown_light_color(&PLAIN, BRAZIER, Some(5)), Some(40));
        assert_eq!(shown_light_color(&PLAIN, 0x0001, Some(5)), Some(5));
        let plain = LightRules {
            colored: false,
            ..PLAIN
        };
        assert_eq!(shown_light_color(&plain, TORCH, None), None);
    }

    #[test]
    fn a_light_brightens_the_cells_under_it_only() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::splat(LIGHT_CELL * 10.0));
        let width = 10;
        let mut cells = vec![[0.0; 3]; width * width];
        let shape = LightShape {
            width: 8,
            height: 8,
            levels: vec![LIGHT_LEVEL_MAX; 64],
        };
        let source = LightSource {
            center: rect.center(),
            shape: 0,
            color: None,
            strength: 1.0,
        };
        add_light(&mut cells, width, rect, &source, &shape, 1.0);
        let lit = |column: usize, row: usize| cells[row * width + column][0] > 0.0;
        assert!(lit(4, 4) && lit(5, 5));
        assert!(!lit(0, 0) && !lit(9, 9));
        let full = cells[4 * width + 4][0];
        let mut dim_cells = vec![[0.0; 3]; width * width];
        let dim = LightSource {
            strength: 0.5,
            ..source
        };
        add_light(&mut dim_cells, width, rect, &dim, &shape, 1.0);
        assert!((dim_cells[4 * width + 4][0] - full / 2.0).abs() < 1e-6);
    }

    #[test]
    fn a_flicker_stays_near_full_and_each_light_flickers_on_its_own() {
        const FRAME: f64 = 1.0 / 60.0;
        const FRAMES: u32 = 600;
        let strengths: Vec<f32> = (0..FRAMES)
            .map(|frame| flicker(1, f64::from(frame) * FRAME))
            .collect();
        assert!(strengths
            .iter()
            .all(|s| (1.0 - FLICKER_DEPTH..=1.0).contains(s)));
        let lowest = strengths.iter().copied().fold(1.0, f32::min);
        assert!(lowest < 1.0 - FLICKER_DEPTH / 2.0, "it dims");
        assert_ne!(flicker(1, 0.5), flicker(2, 0.5));
        assert_eq!(flicker(1, 0.5), flicker(1, 0.5));
    }

    #[test]
    fn the_overlay_darkens_by_the_weakest_channel() {
        let dark = overlay_pixel([0.25; 3], false);
        assert_eq!(dark.a(), 191);
        assert_eq!((dark.r(), dark.g(), dark.b()), (0, 0, 0));
        let day = overlay_pixel([1.0; 3], false);
        assert_eq!(day.a(), 0);
        let alternative = overlay_pixel([1.0, 0.0, 0.0], true);
        assert_eq!(alternative.a(), 0);
        assert!(alternative.r() > 0);
    }
}
