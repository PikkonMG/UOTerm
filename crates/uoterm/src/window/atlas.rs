//! One large texture that holds every picture the map has drawn, so the
//! whole map is one mesh and one draw call.

use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, TextureHandle, TextureOptions, Vec2};
use std::collections::HashMap;
use uoterm_nav::CursorShape;

const ATLAS_SIDE: usize = 4096;
/// Clear pixels between two pictures, so one never bleeds into the next when
/// the map is zoomed.
const GUTTER: usize = 1;
const WHITE_SIDE: usize = 4;
const TEXTURE_NAME: &str = "uoterm-watch-art";
const OPTIONS: TextureOptions = TextureOptions::NEAREST;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArtKey {
    Land {
        land_id: u16,
        hue: u16,
    },
    /// A land texture that is stretched over a slope.
    Texture {
        texture_id: u16,
        hue: u16,
    },
    Item {
        graphic: u16,
        hue: u16,
        /// The hue covers every pixel, not the grey ones only.
        whole_hue: bool,
        /// A black border marks a cave wall.
        border: bool,
    },
    /// Words in a UO font, under the hash of the words and how they look.
    Text(u64),
    /// A mouse pointer of the classic client.
    Cursor {
        shape: CursorShape,
        war: bool,
        hue: u16,
    },
    /// A picture of a gump: a background, a button, a check box.
    Gump {
        gump: u16,
        hue: u16,
        /// The hue covers the grey pixels only, as a body or a worn item
        /// on a paperdoll takes it.
        partial: bool,
    },
    /// One mobile as he looks now, under the hash of his look.
    Figure(u64),
}

/// One picture on its way into the atlas.
pub struct Picture {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
    /// From the top left of the picture to the point that goes on the tile.
    pub anchor: Vec2,
}

/// Where one picture lies in the atlas, and its size in pixels.
#[derive(Clone, Copy, Debug)]
pub struct Sprite {
    pub uv: Rect,
    pub width: f32,
    pub height: f32,
    pub anchor: Vec2,
}

pub struct Atlas {
    texture: TextureHandle,
    /// None marks a picture the client files do not hold.
    sprites: HashMap<ArtKey, Option<Sprite>>,
    shelf_x: usize,
    shelf_y: usize,
    shelf_height: usize,
}

impl Atlas {
    pub fn new(ctx: &egui::Context) -> Self {
        let blank = ColorImage::new([ATLAS_SIDE, ATLAS_SIDE], Color32::TRANSPARENT);
        let mut atlas = Self {
            texture: ctx.load_texture(TEXTURE_NAME, blank, OPTIONS),
            sprites: HashMap::new(),
            shelf_x: 0,
            shelf_y: 0,
            shelf_height: 0,
        };
        atlas.put_white();
        atlas
    }

    pub fn texture_id(&self) -> egui::TextureId {
        self.texture.id()
    }

    /// A point of the atlas that is plain white. A shape with no picture
    /// takes its color from its vertices alone when it reads this point.
    pub fn white_uv(&self) -> Pos2 {
        let middle = (WHITE_SIDE / 2) as f32 / ATLAS_SIDE as f32;
        Pos2::new(middle, middle)
    }

    /// The sprite for `key`. `load` makes the RGBA picture the first time.
    pub fn sprite(
        &mut self,
        key: ArtKey,
        load: impl FnOnce() -> Option<Picture>,
    ) -> Option<Sprite> {
        if let Some(known) = self.sprites.get(&key) {
            return *known;
        }
        let sprite = load().and_then(|picture| self.place(&picture));
        self.sprites.insert(key, sprite);
        sprite
    }

    fn put_white(&mut self) {
        self.place(&Picture {
            width: WHITE_SIDE,
            height: WHITE_SIDE,
            rgba: vec![u8::MAX; WHITE_SIDE * WHITE_SIDE * 4],
            anchor: Vec2::ZERO,
        });
    }

    fn place(&mut self, picture: &Picture) -> Option<Sprite> {
        let (width, height) = (picture.width, picture.height);
        if width + GUTTER > ATLAS_SIDE || height + GUTTER > ATLAS_SIDE {
            return None;
        }
        if self.shelf_x + width + GUTTER > ATLAS_SIDE {
            self.shelf_y += self.shelf_height + GUTTER;
            self.shelf_x = 0;
            self.shelf_height = 0;
        }
        if self.shelf_y + height + GUTTER > ATLAS_SIDE {
            self.start_again();
        }
        let (x, y) = (self.shelf_x, self.shelf_y);
        let image = ColorImage::from_rgba_unmultiplied([width, height], &picture.rgba);
        self.texture.set_partial([x, y], image, OPTIONS);
        self.shelf_x += width + GUTTER;
        self.shelf_height = self.shelf_height.max(height);
        let side = ATLAS_SIDE as f32;
        Some(Sprite {
            uv: Rect::from_min_max(
                Pos2::new(x as f32 / side, y as f32 / side),
                Pos2::new((x + width) as f32 / side, (y + height) as f32 / side),
            ),
            width: width as f32,
            height: height as f32,
            anchor: picture.anchor,
        })
    }

    /// The atlas is full. Forget every picture and fill it again from the
    /// pictures the next frames ask for.
    fn start_again(&mut self) {
        let blank = ColorImage::new([ATLAS_SIDE, ATLAS_SIDE], Color32::TRANSPARENT);
        self.texture.set(blank, OPTIONS);
        self.sprites.clear();
        self.shelf_x = 0;
        self.shelf_y = 0;
        self.shelf_height = 0;
        self.put_white();
    }
}
