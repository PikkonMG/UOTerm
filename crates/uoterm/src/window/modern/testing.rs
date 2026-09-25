//! Draws Modern panels in a window with no screen and no client files, for
//! the tests of what they show and keep. The acts they send reach no
//! session. A canvas paints what a frame drew into a picture, for a test
//! that saves the picture to look at.

use super::super::boxes_ui::Tools;
use super::super::classic::testing::idle_hand;
use super::super::desk::Desk;
use super::super::link::Link;
use super::super::model::compare::ItemLayers;
use super::super::model::reads::Readings;
use super::super::ring_ui::RingUi;
use super::super::scene::Scene;
use super::super::settings::{Profile, ProfileHome};
use super::super::tips::Tips;
use eframe::egui::epaint::{ClippedPrimitive, ImageData, Primitive, Vertex};
use eframe::egui::{
    self, Color32, ColorImage, Pos2, RawInput, Rect, TextureFilter, TextureId, Vec2,
};
use std::collections::HashMap;

/// An address no session answers.
const NO_API: &str = "http://127.0.0.1:1";
pub const SCREEN: Vec2 = Vec2::new(1280.0, 800.0);

/// Draws a panel once for each list of input events, as the frames of a
/// player who moves the mouse, clicks and types. `draw` gets the screen
/// and the tools of the window.
pub fn draw_frames(
    profile: &mut Profile,
    frames: &[Vec<egui::Event>],
    mut draw: impl FnMut(&mut egui::Ui, Rect, &mut Tools<'_>, &mut Profile),
) {
    let ctx = egui::Context::default();
    // The panels write their titles in the faces of the theme.
    super::super::theme::install(&ctx);
    let mut scene = Scene::new(None);
    let hand = idle_hand();
    let mut desk = Desk::default();
    let mut tips = Tips::default();
    let mut ring = RingUi::default();
    // The panels keep the profile as they change it: in a folder of the
    // test's own, never in the player's profiles.
    let home_dir = std::env::temp_dir().join(format!("uoterm-panels-{}", uuid::Uuid::new_v4()));
    let home = ProfileHome::in_dir(&home_dir);
    let mut readings = Readings::start(Link::Http {
        api: NO_API.into(),
        session: String::new(),
    });
    let layers = ItemLayers::default();
    let screen = Rect::from_min_size(Pos2::ZERO, SCREEN);
    for (at, events) in frames.iter().enumerate() {
        desk.begin();
        let input = RawInput {
            events: events.clone(),
            screen_rect: Some(screen),
            ..RawInput::default()
        };
        let _ = ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut tools = Tools {
                    scene: &mut scene,
                    hand: &hand,
                    desk: &mut desk,
                    tips: &mut tips,
                    ring: &mut ring,
                    time: at as f64,
                    profile_home: &home,
                    readings: &mut readings,
                    layers: &layers,
                };
                draw(ui, screen, &mut tools, profile);
            });
        });
    }
    // The folder is there only when a panel kept the profile.
    if home_dir.exists() {
        std::fs::remove_dir_all(&home_dir).expect("the test's own profile folder");
    }
}

/// The events of typing words.
pub fn typing(words: &str) -> Vec<egui::Event> {
    vec![egui::Event::Text(words.into())]
}

/// The frames of one click of the left button at a place: the mouse comes,
/// the button goes down, and it comes up.
pub fn click(at: Pos2) -> Vec<Vec<egui::Event>> {
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    vec![
        vec![egui::Event::PointerMoved(at)],
        vec![button(true)],
        vec![button(false)],
    ]
}

/// The share of a channel at its full value.
const FULL: f32 = 255.0;
/// A pixel is sampled at its middle.
const PIXEL_MIDDLE: f32 = 0.5;

/// The textures of the frames so far, which the canvas paints with.
#[derive(Default)]
pub struct Canvas {
    textures: HashMap<TextureId, (ColorImage, TextureFilter)>,
}

impl Canvas {
    /// Takes in the textures a frame made or changed.
    pub fn take(&mut self, delta: &egui::TexturesDelta) {
        for (id, change) in &delta.set {
            let image = match &change.image {
                ImageData::Color(image) => (**image).clone(),
                ImageData::Font(font) => ColorImage {
                    size: font.size,
                    pixels: font.srgba_pixels(None).collect(),
                },
            };
            let filter = change.options.magnification;
            match change.pos {
                None => {
                    self.textures.insert(*id, (image, filter));
                }
                Some([left, top]) => {
                    let Some((whole, _)) = self.textures.get_mut(id) else {
                        continue;
                    };
                    let [width, height] = image.size;
                    for row in 0..height {
                        let to = (top + row) * whole.size[0] + left;
                        let from = row * width;
                        whole.pixels[to..to + width]
                            .copy_from_slice(&image.pixels[from..from + width]);
                    }
                }
            }
        }
        for id in &delta.free {
            self.textures.remove(id);
        }
    }

    /// Paints the shapes of a frame, as a screen of `size` points shows
    /// them at one pixel for each point.
    pub fn paint(&self, ctx: &egui::Context, output: egui::FullOutput, size: Vec2) -> ColorImage {
        let mut picture = ColorImage::new([size.x as usize, size.y as usize], Color32::BLACK);
        for ClippedPrimitive {
            clip_rect,
            primitive,
        } in ctx.tessellate(output.shapes, output.pixels_per_point)
        {
            let Primitive::Mesh(mesh) = primitive else {
                continue;
            };
            let texture = self.textures.get(&mesh.texture_id);
            for corners in mesh.indices.chunks_exact(3) {
                let corners =
                    [corners[0], corners[1], corners[2]].map(|at| mesh.vertices[at as usize]);
                fill_triangle(&mut picture, clip_rect, &corners, texture);
            }
        }
        picture
    }
}

/// The color of a texture at a place, from 0 to 1 on each side.
fn sample(texture: &(ColorImage, TextureFilter), uv: Pos2) -> [f32; 4] {
    let (image, filter) = texture;
    let [width, height] = image.size;
    let pixel = |x: i64, y: i64| {
        let x = x.clamp(0, width as i64 - 1) as usize;
        let y = y.clamp(0, height as i64 - 1) as usize;
        image.pixels[y * width + x].to_array().map(f32::from)
    };
    let (x, y) = (uv.x * width as f32, uv.y * height as f32);
    if *filter == TextureFilter::Nearest {
        return pixel(x.floor() as i64, y.floor() as i64);
    }
    let (x, y) = (x - PIXEL_MIDDLE, y - PIXEL_MIDDLE);
    let (left, top) = (x.floor(), y.floor());
    let (dx, dy) = (x - left, y - top);
    let (left, top) = (left as i64, top as i64);
    let corners = [
        (pixel(left, top), (1.0 - dx) * (1.0 - dy)),
        (pixel(left + 1, top), dx * (1.0 - dy)),
        (pixel(left, top + 1), (1.0 - dx) * dy),
        (pixel(left + 1, top + 1), dx * dy),
    ];
    let mut color = [0.0; 4];
    for (texel, share) in corners {
        for (channel, value) in color.iter_mut().zip(texel) {
            *channel += value * share;
        }
    }
    color
}

/// Fills one triangle of a mesh, blended over the picture as egui blends:
/// in gamma space with premultiplied alpha.
fn fill_triangle(
    picture: &mut ColorImage,
    clip: Rect,
    corners: &[Vertex; 3],
    texture: Option<&(ColorImage, TextureFilter)>,
) {
    let [a, b, c] = corners.map(|corner| corner.pos);
    let area = (b - a).x * (c - a).y - (b - a).y * (c - a).x;
    if area.abs() < f32::EPSILON {
        return;
    }
    let [width, height] = picture.size;
    let bounds = Rect::from_points(&[a, b, c])
        .intersect(clip)
        .intersect(Rect::from_min_size(
            Pos2::ZERO,
            Vec2::new(width as f32, height as f32),
        ));
    if !bounds.is_positive() {
        return;
    }
    for y in bounds.top().floor() as usize..(bounds.bottom().ceil() as usize).min(height) {
        for x in bounds.left().floor() as usize..(bounds.right().ceil() as usize).min(width) {
            let at = Pos2::new(x as f32 + PIXEL_MIDDLE, y as f32 + PIXEL_MIDDLE);
            let edge = |from: Pos2, to: Pos2| {
                (to - from).x * (at - from).y - (to - from).y * (at - from).x
            };
            let shares = [edge(b, c) / area, edge(c, a) / area, edge(a, b) / area];
            if shares.iter().any(|share| *share < 0.0) || !clip.contains(at) {
                continue;
            }
            let mut color = [0.0; 4];
            let mut uv = Pos2::ZERO;
            for (corner, share) in corners.iter().zip(shares) {
                for (channel, value) in color.iter_mut().zip(corner.color.to_array()) {
                    *channel += f32::from(value) * share;
                }
                uv += corner.uv.to_vec2() * share;
            }
            if let Some(texture) = texture {
                let texel = sample(texture, uv);
                for (channel, value) in color.iter_mut().zip(texel) {
                    *channel *= value / FULL;
                }
            }
            let under = &mut picture.pixels[y * width + x];
            let keep = 1.0 - color[3] / FULL;
            let blended: Vec<u8> = color
                .iter()
                .zip(under.to_array())
                .map(|(top, below)| (top + f32::from(below) * keep).round().clamp(0.0, FULL) as u8)
                .collect();
            *under =
                Color32::from_rgba_premultiplied(blended[0], blended[1], blended[2], blended[3]);
        }
    }
}
