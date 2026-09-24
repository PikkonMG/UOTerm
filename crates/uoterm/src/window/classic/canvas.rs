//! The controls of classic gumps, drawn from gump art with UO fonts: the
//! canvas one gump draws on in one frame. Each control is one call, in the
//! gump's own pixels from its top left corner; the canvas scales them and
//! fades them as the settings ask. The controls follow those of the
//! reference client: pictures, tiled pictures, framed boxes, buttons,
//! check boxes, radio buttons, sliders, scroll bars, scroll areas, scrolls
//! that grow, labels, HTML boxes, text boxes, static pictures, flat
//! buttons, drop-down lists, color boxes, checkered shades and dark
//! see-through boxes.
//!
//! Pictures and words drag the gump; buttons, boxes, sliders and fields
//! take their own clicks, as in the classic client.

use super::layout::{self, frame_part_ids, frame_parts, FRAME_PARTS};
use super::text::{HtmlLook, TextKit, TextLook, TextTexture};
use super::text_field::{FieldKey, FieldOutcome, TextField};
use crate::window::atlas::Sprite;
use crate::window::keys;
use crate::window::scene::Scene;
use eframe::egui::{
    self, Color32, CornerRadius, Event, EventFilter, Id, Key, Order, Pos2, Rect, Response, Sense,
    Stroke, TextureId, Vec2,
};
use std::hash::Hash;
use uoterm_world::GumpScroll;

// The scroll bar of the classic client.
const SCROLL_UP: u16 = 251;
const SCROLL_UP_PRESSED: u16 = 250;
const SCROLL_DOWN: u16 = 253;
const SCROLL_DOWN_PRESSED: u16 = 252;
const SCROLL_TOP: u16 = 257;
const SCROLL_MIDDLE: u16 = 256;
const SCROLL_BOTTOM: u16 = 255;
const SCROLL_KNOB: u16 = 254;
const SCROLL_FLAG: u16 = 0x0828;
/// How far one turn of the wheel scrolls.
const SCROLL_STEP: i32 = 50;
/// A held arrow scrolls one more pixel each time it has moved this often.
const SCROLL_SPEED_UP_STEPS: i32 = 8;
/// A scroll area keeps this much room at its right for its bar.
const SCROLL_AREA_BAR: i32 = 14;
// HTML words.
const HTML_BACKGROUND: u16 = 0x2486;
const HTML_BAR_ROOM: i32 = 16;
const HTML_BACKGROUND_ROOM: i32 = 8;
/// Words on a background with no color of their own lose this much more.
const HTML_BACKGROUND_EXTRA_ROOM: i32 = 9;
const HTML_BACKGROUND_PAD: i32 = 4;
/// The 15-bit white a shard sends for white HTML words.
const HTML_WHITE_15: u32 = 0x7FFF;
const HTML_WHITE: [u8; 4] = [0xFF, 0xFF, 0xFE, u8::MAX];
const HTML_NEAR_BLACK: [u8; 4] = [0x01, 0x01, 0x01, u8::MAX];
const HTML_WHITE_DEFAULT: [u8; 4] = [0xFF, 0xFF, 0xFF, u8::MAX];
const COLOR_CHANNEL_MAX: u32 = 31;
const RED_SHIFT: u32 = 10;
const GREEN_SHIFT: u32 = 5;
// Sliders.
const SLIDER_LEFT: u16 = 213;
const SLIDER_MIDDLE: u16 = 214;
const SLIDER_RIGHT: u16 = 215;
const SLIDER_KNOB: u16 = 216;
const SLIDER_BLUE_KNOB: u16 = 0x0845;
// Drop-down lists.
const COMBO_FRAME: u16 = 0x0BB8;
const COMBO_ARROW: u16 = 0x00FC;
const COMBO_HEIGHT: i32 = 25;
const COMBO_FONT: u8 = 9;
const COMBO_HUE: u16 = 0x0453;
const COMBO_TEXT_AT: (i32, i32) = (2, 5);
const COMBO_ARROW_AT: (i32, i32) = (18, 2);
const COMBO_ROW: i32 = 17;
const COMBO_LIST_MAX_HEIGHT: i32 = 200;
const COMBO_LIST_PAD: i32 = 4;
// Color boxes.
/// The frame of a color box of the Options gump.
pub const COLOR_BOX: u16 = 0x00D4;
const COLOR_BOX_INSET: i32 = 3;
/// A solid box in a hue shows the brightest step of its ramp.
const HUE_BRIGHT_STEP: usize = 31;
// Resizing.
const RESIZE_GRIP: u16 = 0x0837;
const RESIZE_GRIP_PRESSED: u16 = 0x0838;
const EXPANDER: u16 = 0x082E;
const EXPANDER_PRESSED: u16 = 0x082F;
const EXPANDABLE_MIN_HEIGHT: i32 = 274;
const EXPANDABLE_MAX_HEIGHT: i32 = 800;
const EXPANDER_GAP: i32 = 2;
const EXPANDABLE_PARTS: u16 = 4;
// Looks.
const HIGHLIGHT_ALPHA: f32 = 0.25;
const CHECKER_ALPHA: f32 = 0.5;
const SELECTION_COLOR: Color32 = Color32::from_rgba_premultiplied(16, 48, 80, 64);
const CARET: &str = "_";
const CHECKBOX_TEXT_GAP: i32 = 2;
const SLIDER_TEXT_GAP: i32 = 2;
const NO_HUE: u16 = 0;
const HALF: i32 = 2;

/// The three pictures of a button: at rest, pressed and under the mouse.
/// Zero is no picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonArt {
    pub normal: u16,
    pub pressed: u16,
    pub over: u16,
}

impl ButtonArt {
    pub const fn new(normal: u16, pressed: u16, over: u16) -> Self {
        Self {
            normal,
            pressed,
            over,
        }
    }
}

/// How an item picture of [`Canvas::item_button`] shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemLook {
    pub graphic: u16,
    pub hue: u16,
    /// The hue covers every pixel, as the mark of the item under the mouse.
    pub whole_hue: bool,
    /// The size of the picture against its own, as a container scales it.
    pub scale: f32,
    /// A pile shows a second picture this many pixels down and right.
    pub pile_offset: Option<i32>,
}

/// The look of a horizontal slider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderStyle {
    /// A sunken metal bar with a knob, as in the Options gump.
    Recessed,
    /// A blue knob with no bar, as in the color picker.
    BlueKnob,
}

/// How a block of HTML sits in its box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HtmlBox {
    /// A paper background under the words.
    pub background: bool,
    pub scroll: GumpScroll,
    /// A 15-bit color for words no tag colors. None follows the classic
    /// rules: dark on paper, light with a bar, dark with neither.
    pub color: Option<u32>,
}

/// What the gump manager tells the canvas of one gump.
pub struct CanvasInput {
    /// The id every control of the gump is made from.
    pub id: Id,
    /// Where the top left corner of the gump is, in window points.
    pub origin: Pos2,
    /// Window points for each pixel of the gump.
    pub scale: f32,
    /// How opaque the gump is, from 0 to 1.
    pub alpha: f32,
    /// Where the pointer is, when this gump is the top one under it.
    pub pointer: Option<Pos2>,
    /// The pointer clicked on a picture or words of the gump, not on a
    /// control, and did not drag it.
    pub body_click: Option<Pos2>,
    pub body_double_click: bool,
    /// The player right-clicked the gump, and its kind does not close by it.
    pub right_click: bool,
    /// The size the gump is drawn at, in its own pixels, when it has one:
    /// the size the player gave it, or the first size of its kind.
    pub size: Option<Vec2>,
    /// The map the character is on, for the colors of item pictures.
    pub map: u8,
}

/// What one gump drew in one frame, for the gump manager.
#[derive(Default)]
pub struct CanvasOutput {
    /// Everything drawn, in window points.
    pub drawn: Option<Rect>,
    /// Everything drawn, with what lies off the window, in window points.
    pub whole: Option<Rect>,
    /// The places that drag the gump.
    pub body: Vec<Rect>,
    /// The places the gump takes the pointer.
    pub hits: Vec<Rect>,
    /// The words of the tooltip of the control under the pointer.
    pub tip: Option<String>,
    /// A new size the player gave the gump.
    pub size_kept: Option<Vec2>,
    /// A new place the gump asked for, in window points.
    pub move_to: Option<Pos2>,
}

/// The canvas one gump draws on in one frame.
pub struct Canvas<'a> {
    ui: &'a mut egui::Ui,
    pub scene: &'a mut Scene,
    pub text: &'a mut TextKit,
    input: CanvasInput,
    /// How far the controls are moved, in gump pixels, inside a scroll
    /// area.
    offset: Vec2,
    clip: Rect,
    /// The boxes of the gump that cut what they hold, without the window.
    own_clip: Rect,
    alpha: f32,
    out: CanvasOutput,
    /// The last thing drawn, for a tooltip.
    last: Rect,
}

/// The 15-bit color of the game as red, green, blue and alpha bytes.
fn game_rgba(color: u32) -> [u8; 4] {
    let channel = |shift: u32| {
        (((color >> shift) & COLOR_CHANNEL_MAX) * u32::from(u8::MAX) / COLOR_CHANNEL_MAX) as u8
    };
    [
        channel(RED_SHIFT),
        channel(GREEN_SHIFT),
        channel(0),
        u8::MAX,
    ]
}

/// The color HTML words take, and how much narrower than the box they
/// wrap, by the classic rules.
fn html_color_and_room(look: &HtmlBox) -> ([u8; 4], i32) {
    let mut room = 0;
    if look.scroll != GumpScroll::None {
        room += HTML_BAR_ROOM;
    }
    if look.background {
        room += HTML_BACKGROUND_ROOM;
    }
    let color = match look.color {
        Some(HTML_WHITE_15) => HTML_WHITE,
        Some(color) => game_rgba(color),
        None if look.background => {
            room += HTML_BACKGROUND_EXTRA_ROOM;
            HTML_NEAR_BLACK
        }
        None if look.scroll == GumpScroll::None => HTML_NEAR_BLACK,
        None => HTML_WHITE_DEFAULT,
    };
    (color, room)
}

/// The key a text box takes from one event, when it takes one.
fn field_key(event: &Event) -> Option<FieldKey> {
    match event {
        Event::Text(words) | Event::Paste(words) => Some(FieldKey::Insert(words.clone())),
        Event::Copy => Some(FieldKey::Copy),
        Event::Cut => Some(FieldKey::Cut),
        Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } => {
            let select = modifiers.shift;
            let word = modifiers.command || modifiers.alt;
            Some(match key {
                Key::Backspace => FieldKey::Backspace,
                Key::Delete => FieldKey::Delete,
                Key::ArrowLeft => FieldKey::Left { select, word },
                Key::ArrowRight => FieldKey::Right { select, word },
                Key::ArrowUp => FieldKey::Up { select },
                Key::ArrowDown => FieldKey::Down { select },
                Key::Home => FieldKey::Home { select },
                Key::End => FieldKey::End { select },
                Key::Enter => FieldKey::Enter,
                Key::A if modifiers.command => FieldKey::SelectAll,
                _ => return None,
            })
        }
        _ => None,
    }
}

impl<'a> Canvas<'a> {
    pub fn new(
        ui: &'a mut egui::Ui,
        scene: &'a mut Scene,
        text: &'a mut TextKit,
        input: CanvasInput,
    ) -> Self {
        let clip = ui.clip_rect();
        Self {
            ui,
            scene,
            text,
            alpha: input.alpha,
            input,
            offset: Vec2::ZERO,
            clip,
            own_clip: Rect::EVERYTHING,
            out: CanvasOutput::default(),
            last: Rect::NOTHING,
        }
    }

    /// Hands what was drawn to the gump manager.
    pub fn finish(self) -> CanvasOutput {
        self.out
    }

    /// The egui context, for a gump that needs more than the controls.
    pub fn ctx(&self) -> &egui::Context {
        self.ui.ctx()
    }

    /// The egui ui of the gump layer, for input the controls do not take.
    pub fn ui(&self) -> &egui::Ui {
        self.ui
    }

    /// The window points of a place of the gump.
    pub fn at(&self, x: i32, y: i32) -> Pos2 {
        self.input.origin + (self.offset + Vec2::new(x as f32, y as f32)) * self.input.scale
    }

    /// The place of the gump under a window point, in gump pixels.
    pub fn to_gump(&self, at: Pos2) -> Vec2 {
        (at - self.input.origin) / self.input.scale - self.offset
    }

    /// The window area of a box of the gump.
    pub fn area(&self, x: i32, y: i32, size: Vec2) -> Rect {
        Rect::from_min_size(self.at(x, y), size * self.input.scale)
    }

    fn id(&self, key: impl Hash) -> Id {
        self.input.id.with(key)
    }

    fn tint(&self) -> Color32 {
        Color32::WHITE.gamma_multiply(self.alpha)
    }

    /// Draws the rest of the calls in `draw` at a share of their opacity,
    /// as a gump does under a `checkertrans`.
    pub fn faded<R>(&mut self, share: f32, draw: impl FnOnce(&mut Self) -> R) -> R {
        let before = self.alpha;
        self.alpha *= share;
        let result = draw(self);
        self.alpha = before;
        result
    }

    /// Draws the calls in `draw` cut to a box of the gump, as a classic
    /// control that clips what it holds.
    pub fn clipped<R>(
        &mut self,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        draw: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let before = (self.clip, self.own_clip);
        let area = self.area(x, y, Vec2::new(w as f32, h as f32));
        self.clip = self.clip.intersect(area);
        self.own_clip = self.own_clip.intersect(area);
        let result = draw(self);
        (self.clip, self.own_clip) = before;
        result
    }

    /// Draws the calls in `draw` at a share of the gump's size, as a
    /// container gump draws at the container scale. Their places and sizes
    /// stay in the gump's own pixels.
    pub fn scaled<R>(&mut self, share: f32, draw: impl FnOnce(&mut Self) -> R) -> R {
        let before = self.input.scale;
        self.input.scale *= share;
        let result = draw(self);
        self.input.scale = before;
        result
    }

    /// Notes one thing drawn: it takes the pointer, and a picture or words
    /// also drag the gump.
    fn piece(&mut self, rect: Rect, drags: bool) {
        let own = rect.intersect(self.own_clip);
        if own.is_positive() {
            self.out.whole = Some(self.out.whole.map_or(own, |all| all.union(own)));
        }
        let rect = rect.intersect(self.clip);
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            self.last = Rect::NOTHING;
            return;
        }
        self.out.drawn = Some(self.out.drawn.map_or(rect, |all| all.union(rect)));
        self.out.hits.push(rect);
        if drags {
            self.out.body.push(rect);
        }
        self.last = rect;
    }

    fn paint(&self, texture: TextureId, rect: Rect, uv: Rect) {
        self.ui
            .painter()
            .with_clip_rect(self.clip)
            .image(texture, rect, uv, self.tint());
    }

    /// The size of a gump picture, or None when the files lack it.
    pub fn gump_size(&mut self, gump: u16) -> Option<Vec2> {
        self.scene
            .gump_picture(gump, NO_HUE)
            .map(|(_, sprite)| Vec2::new(sprite.width, sprite.height))
    }

    fn picture_at(&mut self, gump: u16, hue: u16, rect: Rect) {
        if let Some((texture, sprite)) = self.scene.gump_picture(gump, hue) {
            self.paint(texture, rect, sprite.uv);
        }
    }

    /// A gump picture at its own size, in a hue (zero keeps its colors).
    /// Gives its size.
    pub fn pic(&mut self, x: i32, y: i32, gump: u16, hue: u16) -> Vec2 {
        let Some((texture, sprite)) = self.scene.gump_picture(gump, hue) else {
            return Vec2::ZERO;
        };
        let size = Vec2::new(sprite.width, sprite.height);
        let rect = self.area(x, y, size);
        self.paint(texture, rect, sprite.uv);
        self.piece(rect, true);
        size
    }

    fn tile_over(&mut self, texture: TextureId, sprite: Sprite, rect: Rect) {
        let size = Vec2::new(sprite.width, sprite.height) * self.input.scale;
        for tile in layout::tiles(rect, size) {
            let uv = Rect::from_min_size(sprite.uv.min, sprite.uv.size() * tile.shown);
            self.paint(texture, tile.area, uv);
        }
    }

    /// One gump picture laid side by side over a box.
    pub fn pic_tiled(&mut self, x: i32, y: i32, w: i32, h: i32, gump: u16, hue: u16) {
        let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
        if let Some((texture, sprite)) = self.scene.gump_picture(gump, hue) {
            self.tile_over(texture, sprite, rect);
        }
        self.piece(rect, true);
    }

    /// A band of the rows of a gump picture, `rows` rows from `top`, laid
    /// again and again down a box `height` high, as the parts of the shop
    /// gump are drawn from one picture.
    pub fn pic_band(&mut self, x: i32, y: i32, gump: u16, (top, rows): (i32, i32), height: i32) {
        let Some((texture, sprite)) = self.scene.gump_picture(gump, NO_HUE) else {
            return;
        };
        let share = |pixels: i32| pixels as f32 / sprite.height.max(1.0);
        let band = Sprite {
            uv: Rect::from_min_size(
                sprite.uv.min + Vec2::new(0.0, sprite.uv.height() * share(top)),
                Vec2::new(sprite.uv.width(), sprite.uv.height() * share(rows)),
            ),
            height: rows as f32,
            ..sprite
        };
        let rect = self.area(x, y, Vec2::new(sprite.width, height as f32));
        self.tile_over(texture, band, rect);
        self.piece(rect, true);
    }

    /// Keeps a size the player gave the gump, as a gump with its own grip
    /// does; `size` reads it back.
    pub fn keep_size(&mut self, size: Vec2) {
        self.out.size_kept = Some(size);
    }

    /// A gump picture laid side by side to a width, at its own height, as
    /// a bar of a health bar is.
    pub fn pic_width(&mut self, x: i32, y: i32, gump: u16, hue: u16, width: i32) {
        if let Some(size) = self.gump_size(gump) {
            self.pic_tiled(x, y, width, size.y as i32, gump, hue);
        }
    }

    /// A frame of nine gump pictures from `gump` on, stretched to a box.
    pub fn frame(&mut self, x: i32, y: i32, w: i32, h: i32, gump: u16) {
        let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
        let mut sizes = [None; FRAME_PARTS];
        for (size, part) in sizes.iter_mut().zip(frame_part_ids(gump)) {
            *size = self.gump_size(part).map(|s| s * self.input.scale);
        }
        for part in frame_parts(gump, &sizes, rect) {
            let Some((texture, sprite)) = self.scene.gump_picture(part.gump, NO_HUE) else {
                continue;
            };
            if part.tiled {
                self.tile_over(texture, sprite, part.area);
            } else {
                self.paint(texture, part.area, sprite.uv);
            }
        }
        self.piece(rect, true);
    }

    /// The picture of an item, in a hue, as a gump shows it. Gives its size.
    pub fn item(&mut self, x: i32, y: i32, graphic: u16, hue: u16) -> Vec2 {
        let Some((texture, sprite)) = self.scene.item_picture(self.input.map, graphic, hue) else {
            return Vec2::ZERO;
        };
        let size = Vec2::new(sprite.width, sprite.height);
        let rect = self.area(x, y, size);
        self.paint(texture, rect, sprite.uv);
        self.piece(rect, true);
        size
    }

    /// A picture the window made, such as a paperdoll figure, at its size.
    pub fn sprite(&mut self, x: i32, y: i32, texture: TextureId, sprite: Sprite) {
        let rect = self.area(x, y, Vec2::new(sprite.width, sprite.height));
        self.paint(texture, rect, sprite.uv);
        self.piece(rect, true);
    }

    /// A line one pixel thick between two places of the gump, as the
    /// course between the pins of a map.
    pub fn line(&mut self, from: (i32, i32), to: (i32, i32), color: Color32) {
        let stroke = Stroke::new(self.input.scale, color.gamma_multiply(self.alpha));
        self.ui
            .painter()
            .with_clip_rect(self.clip)
            .line_segment([self.at(from.0, from.1), self.at(to.0, to.1)], stroke);
    }

    /// A box of one color, such as a line or a dark backdrop.
    pub fn fill(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color32) {
        let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
        self.ui.painter().with_clip_rect(self.clip).rect_filled(
            rect,
            CornerRadius::ZERO,
            color.gamma_multiply(self.alpha),
        );
        self.piece(rect, true);
    }

    /// The one-pixel frame of a box, as the reference client draws it.
    pub fn outline(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color32) {
        let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
        self.ui.painter().with_clip_rect(self.clip).rect_stroke(
            rect,
            CornerRadius::ZERO,
            egui::Stroke::new(self.input.scale, color.gamma_multiply(self.alpha)),
            egui::StrokeKind::Inside,
        );
    }

    /// A dark see-through box in the darkest color of a hue (zero is
    /// black), as the classic client draws a see-through box.
    pub fn shade(&mut self, x: i32, y: i32, w: i32, h: i32, hue: u16, opacity: f32) {
        let [r, g, b] = self.text.fonts.hue_rgb(hue, 0).unwrap_or([0, 0, 0]);
        let color = Color32::from_rgba_unmultiplied(r, g, b, (opacity * f32::from(u8::MAX)) as u8);
        self.fill(x, y, w, h, color);
    }

    /// Half-dark glass over a part of a gump, as a shard's `checkertrans`.
    pub fn checker_trans(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let color = Color32::BLACK.gamma_multiply(CHECKER_ALPHA);
        let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
        self.ui.painter().with_clip_rect(self.clip).rect_filled(
            rect,
            CornerRadius::ZERO,
            color.gamma_multiply(self.alpha),
        );
    }

    fn words_texture(&mut self, words: &str, look: &TextLook) -> Option<TextTexture> {
        let ctx = self.ui.ctx().clone();
        self.text.label(&ctx, words, look)
    }

    /// Words in a UO font. Gives their size.
    pub fn label(&mut self, x: i32, y: i32, words: &str, look: &TextLook) -> Vec2 {
        let Some(texture) = self.words_texture(words, look) else {
            return Vec2::ZERO;
        };
        let rect = self.area(x, y, texture.size);
        self.paint(
            texture.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
        );
        self.piece(rect, true);
        texture.size
    }

    /// The size words take in a font, with no drawing.
    pub fn measure(&mut self, words: &str, look: &TextLook) -> Vec2 {
        self.words_texture(words, look)
            .map_or(Vec2::ZERO, |texture| texture.size)
    }

    /// The sentence of a text number of the client, or `fallback`.
    pub fn words(&self, number: u32, fallback: &str) -> String {
        self.text.fonts.words(number, fallback)
    }

    /// True when the pointer is on the last thing drawn, and this gump is
    /// the top one there.
    pub fn last_hovered(&self) -> bool {
        self.input
            .pointer
            .is_some_and(|pointer| self.last.contains(pointer))
    }

    /// Gives the last thing drawn a tooltip.
    pub fn tooltip(&mut self, words: &str) {
        if self.last_hovered() && !words.is_empty() {
            self.out.tip = Some(words.to_string());
        }
    }

    /// Gives a box of the gump a tooltip, as a classic HitBox has.
    pub fn tooltip_at(&mut self, x: i32, y: i32, w: i32, h: i32, words: &str) {
        if self.hovered(x, y, w, h) && !words.is_empty() {
            self.out.tip = Some(words.to_string());
        }
    }

    /// True when the pointer is on the gump and the gump is the top one
    /// there.
    pub fn pointer_on_gump(&self) -> bool {
        self.input.pointer.is_some()
    }

    /// True when the pointer is on this box of the gump and the gump is the
    /// top one there.
    pub fn hovered(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        let rect = self
            .area(x, y, Vec2::new(w as f32, h as f32))
            .intersect(self.clip);
        self.input
            .pointer
            .is_some_and(|pointer| rect.contains(pointer))
    }

    /// Where the pointer clicked the pictures or words of the gump, in gump
    /// pixels.
    pub fn body_click(&self) -> Option<Vec2> {
        self.input
            .body_click
            .map(|at| (at - self.input.origin) / self.input.scale - self.offset)
    }

    pub fn body_double_click(&self) -> bool {
        self.input.body_double_click
    }

    /// The player right-clicked a gump that does not close by it.
    pub fn right_click(&self) -> bool {
        self.input.right_click
    }

    /// Asks the manager to move the gump to a window place.
    pub fn move_to(&mut self, place: Pos2) {
        self.out.move_to = Some(place);
    }

    /// A box that takes clicks, with nothing drawn.
    pub fn click_area(&mut self, key: impl Hash, x: i32, y: i32, w: i32, h: i32) -> Response {
        let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
        let response = self
            .ui
            .interact(rect.intersect(self.clip), self.id(key), Sense::click());
        self.piece(rect, false);
        response
    }

    /// A box that takes clicks and shows the white glow of the classic
    /// HitBox under the pointer.
    pub fn hit_box(&mut self, key: impl Hash, x: i32, y: i32, w: i32, h: i32) -> Response {
        let response = self.click_area(key, x, y, w, h);
        if response.hovered() {
            self.ui.painter().with_clip_rect(self.clip).rect_filled(
                response.rect,
                CornerRadius::ZERO,
                Color32::WHITE.gamma_multiply(HIGHLIGHT_ALPHA * self.alpha),
            );
        }
        response
    }

    /// A gump picture that takes clicks and drags itself, as a spell icon
    /// of a book does: dragging it does not drag the gump.
    pub fn pic_button(&mut self, key: impl Hash, x: i32, y: i32, gump: u16, hue: u16) -> Response {
        let size = self.gump_size(gump).unwrap_or(Vec2::ZERO);
        let rect = self.area(x, y, size);
        self.picture_at(gump, hue, rect);
        self.drag_control(key, rect)
    }

    /// An item picture that takes clicks and drags itself, as an item in a
    /// container does: dragging it does not drag the gump. None when the
    /// files lack its picture.
    pub fn item_button(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        look: ItemLook,
    ) -> Option<Response> {
        let map = self.input.map;
        let (texture, sprite) = if look.whole_hue {
            self.scene
                .item_picture_whole_hue(map, look.graphic, look.hue)
        } else {
            self.scene.item_picture(map, look.graphic, look.hue)
        }?;
        let size = Vec2::new(sprite.width, sprite.height) * look.scale;
        let rect = self.area(x, y, size);
        self.paint(texture, rect, sprite.uv);
        let mut whole = rect;
        if let Some(offset) = look.pile_offset {
            let pile = self.area(x + offset, y + offset, size);
            self.paint(texture, pile, sprite.uv);
            whole = whole.union(pile);
        }
        Some(self.drag_control(key, whole))
    }

    /// A box that takes clicks and drags, with nothing drawn, as a row of
    /// the skills gump does: dragging it does not drag the gump.
    pub fn drag_area(&mut self, key: impl Hash, x: i32, y: i32, w: i32, h: i32) -> Response {
        let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
        self.drag_control(key, rect)
    }

    fn drag_control(&mut self, key: impl Hash, rect: Rect) -> Response {
        let response = self.ui.interact(
            rect.intersect(self.clip),
            self.id(key),
            Sense::click_and_drag(),
        );
        self.piece(rect, false);
        response
    }

    /// A button of gump pictures that takes clicks over a box of its own
    /// size, as a `buttontileart` does.
    pub fn button_sized(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        art: ButtonArt,
        size: Vec2,
    ) -> bool {
        self.art_button(key, x, y, art, Some(size)).clicked()
    }

    /// The size of an item picture, or none when the files lack it.
    pub fn item_size(&mut self, graphic: u16) -> Vec2 {
        self.scene
            .item_picture(self.input.map, graphic, NO_HUE)
            .map_or(Vec2::ZERO, |(_, sprite)| {
                Vec2::new(sprite.width, sprite.height)
            })
    }

    /// Puts the keys in the text box made with `key`.
    pub fn focus(&mut self, key: impl Hash) {
        let id = self.id(key);
        self.ui.memory_mut(|m| m.request_focus(id));
    }

    /// A button of gump pictures. True when it was clicked.
    pub fn button(&mut self, key: impl Hash, x: i32, y: i32, art: ButtonArt) -> bool {
        self.art_button(key, x, y, art, None).clicked()
    }

    fn art_button(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        art: ButtonArt,
        size: Option<Vec2>,
    ) -> Response {
        let size = size
            .or_else(|| self.gump_size(art.normal))
            .unwrap_or(Vec2::ZERO);
        let rect = self.area(x, y, size);
        let response = self
            .ui
            .interact(rect.intersect(self.clip), self.id(key), Sense::click());
        let down = response.is_pointer_button_down_on();
        let shown = if down && art.pressed != 0 {
            art.pressed
        } else if (down || response.hovered()) && art.over != 0 {
            art.over
        } else {
            art.normal
        };
        let own = self.gump_size(shown).unwrap_or(size);
        self.picture_at(shown, NO_HUE, self.area(x, y, own));
        self.piece(rect, false);
        response
    }

    /// A button with words in its middle, which take `hover_hue` under the
    /// pointer, as the top bar has.
    #[allow(clippy::too_many_arguments)]
    pub fn caption_button(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        art: ButtonArt,
        caption: &str,
        look: &TextLook,
        hover_hue: u16,
    ) -> bool {
        let response = self.art_button(key, x, y, art, None);
        let look = if response.hovered() {
            TextLook {
                hue: hover_hue,
                ..*look
            }
        } else {
            *look
        };
        let size = self.gump_size(art.normal).unwrap_or(Vec2::ZERO);
        let words = self.measure(caption, &look);
        let pressed = i32::from(response.is_pointer_button_down_on());
        let left = x + (size.x - words.x) as i32 / HALF;
        let top = y + pressed + (size.y - words.y) as i32 / HALF;
        self.label(left, top, caption, &look);
        response.clicked()
    }

    /// A button as a box of words that glows under the pointer and stays lit
    /// while selected, as the pages of the Options gump are.
    #[allow(clippy::too_many_arguments)]
    pub fn nice_button(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        caption: &str,
        look: &TextLook,
        selected: bool,
    ) -> bool {
        let response = self.hit_box(key, x, y, w, h);
        if selected && !response.hovered() {
            let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
            self.ui.painter().with_clip_rect(self.clip).rect_filled(
                rect,
                CornerRadius::ZERO,
                Color32::WHITE.gamma_multiply(HIGHLIGHT_ALPHA * self.alpha),
            );
        }
        let look = look.cropped(w as u32);
        let size = self.measure(caption, &look);
        self.label(x, y + (h - size.y as i32) / HALF, caption, &look);
        response.clicked()
    }

    /// A check box of two pictures, with words at its right. True when the
    /// player flipped it.
    pub fn checkbox(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        (off, on): (u16, u16),
        ticked: &mut bool,
        label: Option<(&str, &TextLook)>,
    ) -> bool {
        let clicked = self.choice_box(key, x, y, (off, on), *ticked, label);
        if clicked {
            *ticked = !*ticked;
        }
        clicked
    }

    /// A round choice of two pictures, with words at its right. True when
    /// the player picked it.
    pub fn radio(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        (off, on): (u16, u16),
        picked: bool,
        label: Option<(&str, &TextLook)>,
    ) -> bool {
        self.choice_box(key, x, y, (off, on), picked, label) && !picked
    }

    fn choice_box(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        (off, on): (u16, u16),
        ticked: bool,
        label: Option<(&str, &TextLook)>,
    ) -> bool {
        let shown = if ticked { on } else { off };
        let art = self.gump_size(shown).unwrap_or(Vec2::ZERO);
        let words = label.map_or(Vec2::ZERO, |(words, look)| self.measure(words, look));
        let size = Vec2::new(
            art.x
                + if label.is_some() {
                    CHECKBOX_TEXT_GAP as f32 + words.x
                } else {
                    0.0
                },
            art.y.max(words.y),
        );
        let rect = self.area(x, y, size);
        let response = self
            .ui
            .interact(rect.intersect(self.clip), self.id(key), Sense::click());
        self.picture_at(shown, NO_HUE, self.area(x, y, art));
        if let Some((words, look)) = label {
            self.label(x + art.x as i32 + CHECKBOX_TEXT_GAP, y, words, look);
        }
        self.piece(rect, false);
        response.clicked()
    }

    /// A horizontal slider from `min` to `max`. True when the player moved
    /// it.
    #[allow(clippy::too_many_arguments)]
    pub fn slider(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        width: i32,
        (min, max): (i32, i32),
        value: &mut i32,
        style: SliderStyle,
    ) -> bool {
        let knob = match style {
            SliderStyle::Recessed => SLIDER_KNOB,
            SliderStyle::BlueKnob => SLIDER_BLUE_KNOB,
        };
        let knob_size = self.gump_size(knob).unwrap_or(Vec2::ZERO);
        let travel = width - knob_size.x as i32;
        let rect = self.area(x, y, Vec2::new(width as f32, knob_size.y));
        let response = self.ui.interact(
            rect.intersect(self.clip),
            self.id(key),
            Sense::click_and_drag(),
        );
        let before = *value;
        if let Some(pointer) = response
            .interact_pointer_pos()
            .filter(|_| response.is_pointer_button_down_on() || response.clicked())
        {
            let along = ((pointer.x - rect.left()) / self.input.scale) as i32;
            *value = layout::knob_value(along, min, max, travel).clamp(min, max);
        }
        if response.hovered() {
            let wheel = self.ui.input(|i| i.raw_scroll_delta.y);
            if wheel != 0.0 {
                *value = (*value + wheel.signum() as i32).clamp(min, max);
            }
        }
        if style == SliderStyle::Recessed {
            let left = self.gump_size(SLIDER_LEFT).unwrap_or(Vec2::ZERO);
            let right = self.gump_size(SLIDER_RIGHT).unwrap_or(Vec2::ZERO);
            self.picture_at(SLIDER_LEFT, NO_HUE, self.area(x, y, left));
            let middle = width - left.x as i32 - right.x as i32;
            if let Some((texture, sprite)) = self.scene.gump_picture(SLIDER_MIDDLE, NO_HUE) {
                let area = self.area(
                    x + left.x as i32,
                    y,
                    Vec2::new(middle as f32, sprite.height),
                );
                self.tile_over(texture, sprite, area);
            }
            self.picture_at(
                SLIDER_RIGHT,
                NO_HUE,
                self.area(x + width - right.x as i32, y, right),
            );
        }
        let knob_x = layout::knob_offset(*value, min, max, travel);
        self.picture_at(knob, NO_HUE, self.area(x + knob_x, y, knob_size));
        self.piece(rect, false);
        *value != before
    }

    /// Words beside a slider, at its right and in its middle, as the
    /// classic slider shows its value.
    pub fn slider_words(&mut self, x: i32, y: i32, width: i32, words: &str, look: &TextLook) {
        let knob = self.gump_size(SLIDER_KNOB).unwrap_or(Vec2::ZERO);
        let size = self.measure(words, look);
        let top = y + (knob.y - size.y) as i32 / HALF;
        self.label(x + width + SLIDER_TEXT_GAP, top, words, look);
    }

    /// The classic scroll bar: arrows at the ends and a knob between them.
    /// `value` runs from 0 to `max`. Gives its width.
    pub fn scroll_bar(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        h: i32,
        value: &mut i32,
        max: i32,
    ) -> i32 {
        let id = self.id(key);
        let size = |canvas: &mut Self, gump| canvas.gump_size(gump).unwrap_or(Vec2::ZERO);
        let (up, down) = (size(self, SCROLL_UP), size(self, SCROLL_DOWN));
        let (top, bottom) = (size(self, SCROLL_TOP), size(self, SCROLL_BOTTOM));
        let knob = size(self, SCROLL_KNOB);
        let width = top.x as i32;
        let up_rect = self.area(x, y, up);
        let down_at = y + h - down.y as i32;
        let down_rect = self.area(x, down_at, down);
        let up_press =
            self.ui
                .interact(up_rect.intersect(self.clip), id.with("up"), Sense::click());
        let down_press = self.ui.interact(
            down_rect.intersect(self.clip),
            id.with("down"),
            Sense::click(),
        );
        let room = h - up.y as i32 - down.y as i32 - knob.y as i32;
        let lane = self.area(
            x,
            y + up.y as i32,
            Vec2::new(
                width as f32,
                (h - up.y as i32 - down.y as i32).max(0) as f32,
            ),
        );
        let lane_press = self.ui.interact(
            lane.intersect(self.clip),
            id.with("lane"),
            Sense::click_and_drag(),
        );
        let held = id.with("held");
        let steps: i32 = self.ui.data(|d| d.get_temp(held)).unwrap_or(0);
        let arrow = if up_press.is_pointer_button_down_on() {
            -1
        } else if down_press.is_pointer_button_down_on() {
            1
        } else {
            0
        };
        if arrow != 0 {
            *value += arrow * (1 + steps / SCROLL_SPEED_UP_STEPS);
            self.ui.data_mut(|d| d.insert_temp(held, steps + 1));
            self.ui.ctx().request_repaint();
        } else {
            self.ui.data_mut(|d| d.insert_temp(held, 0));
        }
        if let Some(pointer) = lane_press
            .interact_pointer_pos()
            .filter(|_| lane_press.is_pointer_button_down_on())
        {
            let along = ((pointer.y - lane.top()) / self.input.scale) as i32 - knob.y as i32 / HALF;
            *value = layout::scroll_value(along, 0, max, room);
        }
        *value = (*value).clamp(0, max.max(0));
        // The lane: its top and bottom ends when there is room, and the
        // middle laid down between them.
        let middle = h - up.y as i32 - down.y as i32 - top.y as i32 - bottom.y as i32;
        if middle > 0 {
            self.picture_at(SCROLL_TOP, NO_HUE, self.area(x, y + up.y as i32, top));
            self.pic_tiled_quiet(
                x,
                y + up.y as i32 + top.y as i32,
                width,
                middle,
                SCROLL_MIDDLE,
            );
            let bottom_at = y + h - down.y as i32 - bottom.y as i32;
            self.picture_at(SCROLL_BOTTOM, NO_HUE, self.area(x, bottom_at, bottom));
        } else {
            let lane_height = h - up.y as i32 - down.y as i32;
            self.pic_tiled_quiet(x, y + up.y as i32, width, lane_height, SCROLL_MIDDLE);
        }
        let up_art = if up_press.is_pointer_button_down_on() {
            SCROLL_UP_PRESSED
        } else {
            SCROLL_UP
        };
        let down_art = if down_press.is_pointer_button_down_on() {
            SCROLL_DOWN_PRESSED
        } else {
            SCROLL_DOWN
        };
        self.picture_at(up_art, NO_HUE, up_rect);
        self.picture_at(down_art, NO_HUE, down_rect);
        if max > 0 && room > 0 {
            let knob_y = y + up.y as i32 + layout::scroll_knob(*value, 0, max, room);
            let knob_x = x + (width - knob.x as i32) / HALF;
            self.picture_at(SCROLL_KNOB, NO_HUE, self.area(knob_x, knob_y, knob));
        }
        let whole = self.area(x, y, Vec2::new(width as f32, h as f32));
        self.piece(whole, false);
        width
    }

    /// The small flag that slides down the right edge of some HTML boxes
    /// and of the journal. `value` runs from 0 to `max`.
    pub fn scroll_flag(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        h: i32,
        value: &mut i32,
        max: i32,
    ) {
        let flag = self.gump_size(SCROLL_FLAG).unwrap_or(Vec2::ZERO);
        let room = h - flag.y as i32;
        let lane = self.area(x, y, Vec2::new(flag.x, h as f32));
        let press = self.ui.interact(
            lane.intersect(self.clip),
            self.id(key),
            Sense::click_and_drag(),
        );
        if let Some(pointer) = press
            .interact_pointer_pos()
            .filter(|_| press.is_pointer_button_down_on())
        {
            let along = ((pointer.y - lane.top()) / self.input.scale) as i32 - flag.y as i32 / HALF;
            *value = layout::scroll_value(along, 0, max, room);
        }
        if max > 0 {
            let at = y + layout::scroll_knob(*value, 0, max, room);
            self.picture_at(SCROLL_FLAG, NO_HUE, self.area(x, at, flag));
        }
        self.piece(lane, false);
    }

    fn pic_tiled_quiet(&mut self, x: i32, y: i32, w: i32, h: i32, gump: u16) {
        if let Some((texture, sprite)) = self.scene.gump_picture(gump, NO_HUE) {
            let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
            self.tile_over(texture, sprite, rect);
        }
    }

    /// Scrolls a value by the wheel while the pointer is on this gump box.
    fn wheel(&self, x: i32, y: i32, w: i32, h: i32, value: &mut i32, max: i32) {
        if !self.hovered(x, y, w, h) {
            return;
        }
        let wheel = self.ui.input(|i| i.raw_scroll_delta.y);
        if wheel != 0.0 {
            *value = (*value - wheel.signum() as i32 * SCROLL_STEP).clamp(0, max.max(0));
        }
    }

    /// Turns of the wheel while the pointer is on this box of the gump, up
    /// counted as more. Zero when it is elsewhere.
    pub fn wheel_turns(&self, x: i32, y: i32, w: i32, h: i32) -> i32 {
        if !self.hovered(x, y, w, h) {
            return 0;
        }
        let wheel = self.ui.input(|i| i.raw_scroll_delta.y);
        if wheel == 0.0 {
            0
        } else {
            wheel.signum() as i32
        }
    }

    /// Scrolls the scroll area made with `key` by `delta` pixels, as the
    /// arrows of a gump that scroll it do.
    pub fn scroll_by(&mut self, key: impl Hash, delta: i32) {
        let value_key = self.id(key).with("value");
        self.ui.data_mut(|d| {
            let value: i32 = d.get_temp(value_key).unwrap_or(0);
            d.insert_temp(value_key, (value + delta).max(0));
        });
    }

    /// A box whose content scrolls under a scroll bar at its right. `draw`
    /// draws the content from the top left of the box and gives its height.
    pub fn scroll_area(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        draw: impl FnOnce(&mut Self) -> i32,
    ) {
        let id = self.id(key);
        let (content_key, value_key) = (id.with("content"), id.with("value"));
        let content: i32 = self.ui.data(|d| d.get_temp(content_key)).unwrap_or(0);
        let mut value: i32 = self.ui.data(|d| d.get_temp(value_key)).unwrap_or(0);
        let max = (content - h).max(0);
        self.wheel(x, y, w, h, &mut value, max);
        if max > 0 {
            self.scroll_bar(
                id.with("bar"),
                x + w - SCROLL_AREA_BAR,
                y,
                h,
                &mut value,
                max,
            );
        }
        value = value.clamp(0, max);
        let inner = self
            .area(x, y, Vec2::new((w - SCROLL_AREA_BAR) as f32, h as f32))
            .intersect(self.clip);
        let (before_offset, before_clip) = (self.offset, self.clip);
        self.offset += Vec2::new(x as f32, (y - value) as f32);
        self.clip = inner;
        let height = draw(self);
        self.offset = before_offset;
        self.clip = before_clip;
        self.ui.data_mut(|d| {
            d.insert_temp(content_key, height);
            d.insert_temp(value_key, value);
        });
    }

    /// Gump HTML in a box, with its background and its scroll bar, by the
    /// classic rules.
    #[allow(clippy::too_many_arguments)]
    pub fn html(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        html: &str,
        look: &HtmlBox,
    ) {
        let (color, room) = html_color_and_room(look);
        let has_bar = look.scroll != GumpScroll::None;
        let bar_room = if has_bar { HTML_BAR_ROOM } else { 0 };
        if look.background {
            self.frame(x, y, w - bar_room, h, HTML_BACKGROUND);
        }
        let width = (w - room).max(1) as u32;
        let ctx = self.ui.ctx().clone();
        let texture = self.text.html(&ctx, html, &HtmlLook { width, color });
        let text_height = texture.as_ref().map_or(0, |t| t.size.y as i32);
        let pad = if look.background {
            HTML_BACKGROUND_PAD
        } else {
            0
        };
        let max = (text_height - h
            + if look.background {
                HTML_BACKGROUND_ROOM
            } else {
                0
            })
        .max(0);
        let id = self.id(key);
        let value_key = id.with("value");
        let mut value: i32 = self.ui.data(|d| d.get_temp(value_key)).unwrap_or(0);
        if has_bar {
            self.wheel(x, y, w, h, &mut value, max);
            let bar_x = x + w - SCROLL_AREA_BAR;
            match look.scroll {
                GumpScroll::Flag => self.scroll_flag(id.with("flag"), bar_x, y, h, &mut value, max),
                _ => {
                    self.scroll_bar(id.with("bar"), bar_x, y, h, &mut value, max);
                }
            }
        }
        value = value.clamp(0, max);
        self.ui.data_mut(|d| d.insert_temp(value_key, value));
        let area = self.area(x, y, Vec2::new(w as f32, h as f32));
        if let Some(texture) = texture {
            let rect = self.area(x + pad, y + pad - value, texture.size);
            let before = self.clip;
            self.clip = self.clip.intersect(area);
            self.paint(
                texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            );
            self.clip = before;
        }
        self.piece(area, true);
    }

    /// A text box: words the player types, with a caret and a selection.
    /// A click puts the keys in it. Gives what the keys did.
    #[allow(clippy::too_many_arguments)]
    pub fn text_box(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        field: &mut TextField,
        look: &TextLook,
    ) -> FieldOutcome {
        let id = self.id(key);
        self.text_box_as(id, x, y, w, h, field, look)
    }

    /// A text box with an id of its own rather than one of this gump, such
    /// as the chat line, which the keys know by its id.
    #[allow(clippy::too_many_arguments)]
    pub fn text_box_as(
        &mut self,
        id: Id,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        field: &mut TextField,
        look: &TextLook,
    ) -> FieldOutcome {
        keys::mark_word_field(self.ui.ctx(), id);
        let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
        let response = self
            .ui
            .interact(rect.intersect(self.clip), id, Sense::click());
        let line_height = self.text.fonts.line_height(look).max(1) as i32;
        let shown = field.shown();
        let lines: Vec<&str> = shown.split('\n').collect();
        let scroll_key = id.with("scroll");
        let scroll_x: i32 = self.ui.data(|d| d.get_temp(scroll_key)).unwrap_or(0);
        if response.clicked() {
            self.ui.memory_mut(|m| m.request_focus(id));
            if let Some(pointer) = response.interact_pointer_pos() {
                let local = (pointer - rect.min) / self.input.scale;
                let row =
                    ((local.y as i32) / line_height).clamp(0, lines.len() as i32 - 1) as usize;
                let before: usize = lines[..row].iter().map(|l| l.chars().count() + 1).sum();
                let column = self.column_at(lines[row], local.x as i32 + scroll_x, look);
                let select = self.ui.input(|i| i.modifiers.shift);
                field.place_caret(before + column, select);
            }
        }
        if response.clicked_elsewhere() {
            self.ui.memory_mut(|m| m.surrender_focus(id));
        }
        let focused = self.ui.memory(|m| m.has_focus(id));
        let mut outcome = FieldOutcome::default();
        if focused {
            let filter = EventFilter {
                tab: false,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: false,
            };
            self.ui.memory_mut(|m| m.set_focus_lock_filter(id, filter));
            let events = self.ui.input(|i| i.events.clone());
            for event in &events {
                let Some(key) = field_key(event) else {
                    continue;
                };
                let done = field.apply(key);
                outcome.changed |= done.changed;
                outcome.submitted |= done.submitted;
                if let Some(copied) = done.copied {
                    self.ui.ctx().copy_text(copied);
                }
            }
        }
        // Where the caret is: its line and how far along it.
        let shown = field.shown();
        let lines: Vec<&str> = shown.split('\n').collect();
        let (caret_row, caret_column) = row_and_column(&shown, field.caret());
        let caret_x = self.prefix_width(lines[caret_row], caret_column, look);
        let caret_width = self.measure(CARET, look).x as i32;
        let scroll_x = if caret_x - scroll_x > w - caret_width {
            caret_x - (w - caret_width)
        } else {
            scroll_x.min(caret_x)
        }
        .max(0);
        self.ui.data_mut(|d| d.insert_temp(scroll_key, scroll_x));
        let before = self.clip;
        self.clip = self.clip.intersect(rect);
        if let Some(selected) = field.selection() {
            self.draw_selection(x - scroll_x, y, &shown, selected, line_height, look);
        }
        for (row, line) in lines.iter().enumerate() {
            let top = y + row as i32 * line_height;
            if let Some(texture) = self.words_texture(line, look) {
                let area = self.area(x - scroll_x, top, texture.size);
                self.paint(
                    texture.id(),
                    area,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                );
            }
        }
        if focused {
            let top = y + caret_row as i32 * line_height;
            if let Some(texture) = self.words_texture(CARET, look) {
                let area = self.area(x - scroll_x + caret_x, top, texture.size);
                self.paint(
                    texture.id(),
                    area,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                );
            }
        }
        self.clip = before;
        self.piece(rect, false);
        outcome
    }

    /// The width of the first `chars` chars of a line.
    fn prefix_width(&self, line: &str, chars: usize, look: &TextLook) -> i32 {
        let prefix: String = line.chars().take(chars).collect();
        self.text.fonts.width(look.font, &prefix) as i32
    }

    /// The char a click `x` pixels along a line lands before.
    fn column_at(&self, line: &str, x: i32, look: &TextLook) -> usize {
        let count = line.chars().count();
        (0..=count)
            .min_by_key(|chars| (self.prefix_width(line, *chars, look) - x).abs())
            .unwrap_or(count)
    }

    fn draw_selection(
        &mut self,
        x: i32,
        y: i32,
        shown: &str,
        selected: std::ops::Range<usize>,
        line_height: i32,
        look: &TextLook,
    ) {
        let mut start = 0;
        for (row, line) in shown.split('\n').enumerate() {
            let count = line.chars().count();
            let (from, to) = (
                selected.start.clamp(start, start + count) - start,
                selected.end.clamp(start, start + count) - start,
            );
            if from < to {
                let left = self.prefix_width(line, from, look);
                let right = self.prefix_width(line, to, look);
                let area = self.area(
                    x + left,
                    y + row as i32 * line_height,
                    Vec2::new((right - left) as f32, line_height as f32),
                );
                self.ui.painter().with_clip_rect(self.clip).rect_filled(
                    area,
                    CornerRadius::ZERO,
                    SELECTION_COLOR,
                );
            }
            start += count + 1;
        }
    }

    /// A drop-down list of words in a stone box. True when the player
    /// picked another entry.
    pub fn combobox(
        &mut self,
        key: impl Hash,
        x: i32,
        y: i32,
        w: i32,
        items: &[&str],
        index: &mut usize,
    ) -> bool {
        let id = self.id(key);
        self.frame(x, y, w, COMBO_HEIGHT, COMBO_FRAME);
        let look = TextLook::ascii(COMBO_FONT, COMBO_HUE).cropped((w - COMBO_ARROW_AT.0) as u32);
        let shown = items.get(*index).copied().unwrap_or_default();
        self.label(x + COMBO_TEXT_AT.0, y + COMBO_TEXT_AT.1, shown, &look);
        self.pic(
            x + w - COMBO_ARROW_AT.0,
            y + COMBO_ARROW_AT.1,
            COMBO_ARROW,
            NO_HUE,
        );
        let rect = self.area(x, y, Vec2::new(w as f32, COMBO_HEIGHT as f32));
        let response = self
            .ui
            .interact(rect.intersect(self.clip), id, Sense::click());
        self.piece(rect, false);
        let open_key = id.with("open");
        let mut open: bool = self.ui.data(|d| d.get_temp(open_key)).unwrap_or(false);
        if response.clicked() {
            open = !open;
        }
        let mut changed = false;
        if open {
            // The click that opens the list is not a pick in it.
            match self.combo_list(id, rect, w, items, &look) {
                _ if response.clicked() => {}
                ListPick::Picked(at) => {
                    changed = at != *index;
                    *index = at;
                    open = false;
                }
                ListPick::Closed => open = false,
                ListPick::Open => {}
            }
        }
        self.ui.data_mut(|d| d.insert_temp(open_key, open));
        changed
    }

    /// Draws the open list of a drop-down in a layer over every gump.
    fn combo_list(
        &mut self,
        id: Id,
        under: Rect,
        w: i32,
        items: &[&str],
        look: &TextLook,
    ) -> ListPick {
        let ctx = self.ui.ctx().clone();
        let rows = items.len() as i32;
        let height = (rows * COMBO_ROW + COMBO_LIST_PAD * HALF).min(COMBO_LIST_MAX_HEIGHT);
        let scroll_key = id.with("list-scroll");
        let mut scroll: i32 = ctx.data(|d| d.get_temp(scroll_key)).unwrap_or(0);
        let max_scroll = (rows * COMBO_ROW + COMBO_LIST_PAD * HALF - height).max(0);
        let input = CanvasInput {
            id: id.with("list"),
            origin: under.left_bottom(),
            scale: self.input.scale,
            alpha: self.input.alpha,
            pointer: ctx.pointer_hover_pos(),
            body_click: None,
            body_double_click: false,
            right_click: false,
            size: None,
            map: self.input.map,
        };
        let (scene, text) = (&mut *self.scene, &mut *self.text);
        let mut pick = ListPick::Open;
        egui::Area::new(id.with("list-area"))
            .order(Order::Foreground)
            .fixed_pos(under.left_bottom())
            .constrain(false)
            .show(&ctx, |ui| {
                let mut list = Canvas::new(ui, scene, text, input);
                list.frame(0, 0, w, height, COMBO_FRAME);
                list.wheel(0, 0, w, height, &mut scroll, max_scroll);
                let area = list.area(0, 0, Vec2::new(w as f32, height as f32));
                list.clip = list
                    .clip
                    .intersect(area.shrink(COMBO_LIST_PAD as f32 * list.input.scale));
                for (at, words) in items.iter().enumerate() {
                    let top = COMBO_LIST_PAD + at as i32 * COMBO_ROW - scroll;
                    if list
                        .hit_box(
                            ("row", at),
                            COMBO_LIST_PAD,
                            top,
                            w - COMBO_LIST_PAD * HALF,
                            COMBO_ROW,
                        )
                        .clicked()
                    {
                        pick = ListPick::Picked(at);
                    }
                    list.label(COMBO_LIST_PAD + COMBO_TEXT_AT.0, top, words, look);
                }
                let outside = ui.input(|i| i.pointer.any_click())
                    && !ui.rect_contains_pointer(area)
                    && !ui.rect_contains_pointer(under);
                if outside && pick == ListPick::Open {
                    pick = ListPick::Closed;
                }
            });
        ctx.data_mut(|d| d.insert_temp(scroll_key, scroll));
        pick
    }

    /// A small box in the color of a hue, framed as in the Options gump.
    /// True when it was clicked.
    pub fn color_box(&mut self, key: impl Hash, x: i32, y: i32, hue: u16) -> bool {
        let size = self.gump_size(COLOR_BOX).unwrap_or(Vec2::ZERO);
        let rect = self.area(x, y, size);
        let response = self
            .ui
            .interact(rect.intersect(self.clip), self.id(key), Sense::click());
        self.picture_at(COLOR_BOX, NO_HUE, rect);
        self.hue_box(
            x + COLOR_BOX_INSET,
            y + COLOR_BOX_INSET,
            size.x as i32 - COLOR_BOX_INSET * HALF,
            size.y as i32 - COLOR_BOX_INSET * HALF,
            hue,
        );
        self.piece(rect, false);
        response.clicked()
    }

    /// A solid box in the color a hue gives white.
    pub fn hue_box(&mut self, x: i32, y: i32, w: i32, h: i32, hue: u16) {
        let [r, g, b] = self
            .text
            .fonts
            .hue_rgb(hue, HUE_BRIGHT_STEP)
            .unwrap_or([u8::MAX; 3]);
        let rect = self.area(x, y, Vec2::new(w as f32, h as f32));
        self.ui.painter().with_clip_rect(self.clip).rect_filled(
            rect,
            CornerRadius::ZERO,
            Color32::from_rgb(r, g, b).gamma_multiply(self.alpha),
        );
    }

    /// The size the gump is drawn at, when it has one; see
    /// [`CanvasInput::size`].
    pub fn size(&self) -> Option<Vec2> {
        self.input.size
    }

    /// The grip at the bottom right corner of a gump the player resizes by
    /// dragging, as the reference client's resizable gumps have. The gump manager draws it
    /// for a kind with `resizable` rules.
    pub fn resize_grip(&mut self, min: Vec2) {
        let Some(size) = self.input.size else {
            return;
        };
        let grip = self.gump_size(RESIZE_GRIP).unwrap_or(Vec2::ZERO);
        let at = size - grip;
        let rect = self.area(at.x as i32, at.y as i32, grip);
        let response = self.ui.interact(
            rect.intersect(self.clip),
            self.id("resize-grip"),
            Sense::drag(),
        );
        let art = if response.is_pointer_button_down_on() {
            RESIZE_GRIP_PRESSED
        } else {
            RESIZE_GRIP
        };
        self.picture_at(art, NO_HUE, rect);
        self.piece(rect, false);
        if response.dragged() {
            let grown = (size + response.drag_delta() / self.input.scale).max(min);
            self.out.size_kept = Some(grown);
        }
    }

    /// A scroll of paper that the player makes longer or shorter with the
    /// gripper at its foot, as the journal and the skills gump are: its top,
    /// its middle laid down, its foot and the gripper, from `gump` on. Gives
    /// the width and the height.
    pub fn expandable_scroll(&mut self, x: i32, y: i32, gump: u16, default_height: i32) -> Vec2 {
        self.hued_expandable_scroll(x, y, gump, default_height, NO_HUE)
    }

    /// An expandable scroll whose paper takes a hue, as the dark journal
    /// does. The gripper keeps its own colors.
    pub fn hued_expandable_scroll(
        &mut self,
        x: i32,
        y: i32,
        gump: u16,
        default_height: i32,
        hue: u16,
    ) -> Vec2 {
        let parts: Vec<Vec2> = (0..EXPANDABLE_PARTS)
            .map(|part| self.gump_size(gump + part).unwrap_or(Vec2::ZERO))
            .collect();
        let (top, side, middle, foot) = (parts[0], parts[1], parts[2], parts[3]);
        let width = parts.iter().map(|p| p.x).fold(0.0, f32::max) as i32;
        let saved = self.input.size.map(|s| s.y as i32);
        let mut height = saved
            .unwrap_or(default_height)
            .clamp(EXPANDABLE_MIN_HEIGHT, EXPANDABLE_MAX_HEIGHT);
        let expander = self.gump_size(EXPANDER).unwrap_or(Vec2::ZERO);
        let expander_at = (
            x + (width - expander.x as i32) / HALF,
            y + height - expander.y as i32 - EXPANDER_GAP,
        );
        let rect = self.area(expander_at.0, expander_at.1, expander);
        let response = self.ui.interact(
            rect.intersect(self.clip),
            self.id("expander"),
            Sense::drag(),
        );
        if response.dragged() {
            height = (height + (response.drag_delta().y / self.input.scale) as i32)
                .clamp(EXPANDABLE_MIN_HEIGHT, EXPANDABLE_MAX_HEIGHT);
            self.out.size_kept = Some(Vec2::new(width as f32, height as f32));
        }
        let middle_x = x + (width - side.x as i32) / HALF;
        let middle_y = y + top.y as i32;
        let middle_height = height - top.y as i32 - foot.y as i32 - expander.y as i32;
        self.pic(x, y, gump, hue);
        self.pic_tiled(
            middle_x,
            middle_y,
            side.x as i32,
            middle_height,
            gump + 1,
            hue,
        );
        self.pic_tiled(
            middle_x,
            middle_y,
            middle.x as i32,
            middle_height,
            gump + 2,
            hue,
        );
        let off = top.x as i32 - foot.x as i32;
        self.pic(
            x + off / HALF + off / (HALF * HALF),
            y + height - foot.y as i32 - expander.y as i32,
            gump + EXPANDABLE_PARTS - 1,
            hue,
        );
        let art = if response.is_pointer_button_down_on() {
            EXPANDER_PRESSED
        } else {
            EXPANDER
        };
        self.picture_at(art, NO_HUE, rect);
        self.piece(rect, false);
        Vec2::new(width as f32, height as f32)
    }
}

/// What the open list of a drop-down did this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ListPick {
    Open,
    Picked(usize),
    Closed,
}

/// The line and the column of a char place in words of many lines.
fn row_and_column(words: &str, place: usize) -> (usize, usize) {
    let before: Vec<char> = words.chars().take(place).collect();
    let row = before.iter().filter(|ch| **ch == '\n').count();
    let column = before.iter().rev().take_while(|ch| **ch != '\n').count();
    (row, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_colors_and_room_follow_the_classic_rules() {
        let plain = HtmlBox {
            background: false,
            scroll: GumpScroll::None,
            color: None,
        };
        assert_eq!(html_color_and_room(&plain), (HTML_NEAR_BLACK, 0));
        let barred = HtmlBox {
            scroll: GumpScroll::Bar,
            ..plain
        };
        assert_eq!(
            html_color_and_room(&barred),
            (HTML_WHITE_DEFAULT, HTML_BAR_ROOM)
        );
        let paper = HtmlBox {
            background: true,
            ..plain
        };
        assert_eq!(
            html_color_and_room(&paper),
            (
                HTML_NEAR_BLACK,
                HTML_BACKGROUND_ROOM + HTML_BACKGROUND_EXTRA_ROOM
            )
        );
        let colored = HtmlBox {
            color: Some(0x7C00),
            ..paper
        };
        assert_eq!(
            html_color_and_room(&colored),
            ([u8::MAX, 0, 0, u8::MAX], HTML_BACKGROUND_ROOM)
        );
        let white = HtmlBox {
            color: Some(HTML_WHITE_15),
            ..plain
        };
        assert_eq!(html_color_and_room(&white).0, HTML_WHITE);
    }

    #[test]
    fn keys_become_field_keys() {
        let key = |key: Key, modifiers: egui::Modifiers| Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        assert_eq!(
            field_key(&key(Key::ArrowLeft, egui::Modifiers::SHIFT)),
            Some(FieldKey::Left {
                select: true,
                word: false
            })
        );
        assert_eq!(
            field_key(&key(Key::A, egui::Modifiers::COMMAND)),
            Some(FieldKey::SelectAll)
        );
        assert_eq!(field_key(&key(Key::A, egui::Modifiers::NONE)), None);
        assert_eq!(
            field_key(&Event::Text("x".into())),
            Some(FieldKey::Insert("x".into()))
        );
    }

    #[test]
    fn a_place_in_many_lines_has_a_row_and_a_column() {
        assert_eq!(row_and_column("ab\ncd", 4), (1, 1));
        assert_eq!(row_and_column("ab", 2), (0, 2));
        assert_eq!(row_and_column("", 0), (0, 0));
    }
}
