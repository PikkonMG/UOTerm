//! The Unicode fonts of `unifont.mul`, `unifont1.mul` and on: the fonts of
//! speech, of the journal and of the newer gumps.
//!
//! A file starts with one 32-bit place for each of the 65536 chars; zero is
//! a char the font does not hold. At that place a glyph is its x offset, its
//! y offset, its width and its height, one signed byte each, then its rows
//! of one bit for each pixel, the high bit first.
//!
//! The drawing follows the classic client: bold widens each stroke by one
//! pixel, italic slants the rows, a border rings the ink in black (the
//! overhead words), and underline draws a line under the letters.

use std::path::Path;

use crate::hues::HueData;
use crate::mul::{read_file, slice_at, MapError};
use crate::text::{
    crop, widest_line, wrap, CharMetric, TextAlign, TextLine, TextPicture, ELLIPSIS,
};

pub const UNIFONT_NAME: &str = "unifont.mul";
/// The client looks for this many Unicode fonts.
pub const UNIFONT_MAX: usize = 20;
/// A space is no glyph. It moves the pen this far.
pub const UNICODE_SPACE_WIDTH: u32 = 8;
/// A line with no glyph on it takes this height.
pub const UNICODE_EMPTY_LINE_HEIGHT: u32 = 14;
/// The hue that writes in plain white.
pub const UNICODE_WHITE_HUE: u16 = 0xFFFF;
/// The picture of a block is this much wider and taller than its lines.
pub const UNICODE_PICTURE_PADDING: u32 = 4;
/// The step of a hue ramp that Unicode words take.
pub const UNICODE_HUE_STEP: usize = 30;

/// The font that stands in for a missing glyph, and the font that stands in
/// for a missing `unifont1.mul`.
const FALLBACK_FONT: u8 = 0;
const ALIASED_FONT: u8 = 1;
const LOOKUP_ENTRY: usize = 4;
const LOOKUP_CHARS: u32 = 0x1_0000;
const GLYPH_HEADER: usize = 4;
const BITS_PER_BYTE: usize = 8;
const HIGH_BIT: u8 = 0x80;
/// A line of spaces only is this tall.
const SPACE_LINE_HEIGHT: u32 = 5;
/// The extra height style adds this to every line.
const EXTRA_LINE_HEIGHT: u32 = 4;
/// Italic moves a row one pixel right for this many rows above the bottom.
const ITALIC_SLANT: f32 = 3.3;
/// A centered line is centered in the width less this.
const CENTER_MARGIN: i32 = 8;
/// A right-aligned line ends this far from the right edge.
const RIGHT_MARGIN: i32 = 10;
const RGBA_BYTES: usize = 4;
const WHITE: [u8; RGBA_BYTES] = [u8::MAX, u8::MAX, u8::MAX, u8::MAX];
/// The ink of hue zero, and of the border. It is not quite black, so a clear
/// pixel is never mistaken for it.
const NEAR_BLACK: [u8; RGBA_BYTES] = [1, 1, 1, u8::MAX];
/// Ink this dark on every channel gets no border.
const DARK_CHANNEL_MAX: u8 = 8;
const CLEAR: u32 = 0;
const UNDERLINE_MEASURE: char = 'a';
const SPACE: char = ' ';

/// The file name of one Unicode font.
pub fn unifont_name(font: usize) -> String {
    match font {
        0 => UNIFONT_NAME.to_string(),
        _ => format!("unifont{font}.mul"),
    }
}

/// How words are drawn. The default is plain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct UnicodeStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// A black ring round the ink, as the words over heads have.
    pub border: bool,
    /// Four more pixels of height for every line.
    pub extra_height: bool,
}

impl UnicodeStyle {
    fn extra_height(self) -> u32 {
        if self.extra_height {
            EXTRA_LINE_HEIGHT
        } else {
            0
        }
    }
}

/// One glyph of a Unicode font, read in place from the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnicodeGlyph<'a> {
    pub offset_x: i8,
    pub offset_y: i8,
    pub width: u8,
    pub height: u8,
    rows: &'a [u8],
}

impl UnicodeGlyph<'_> {
    fn row_bytes(&self) -> usize {
        (usize::from(self.width) - 1) / BITS_PER_BYTE + 1
    }

    /// True when the pixel at `x`, `y` of the glyph is ink.
    pub fn is_ink(&self, x: usize, y: usize) -> bool {
        if x >= usize::from(self.width) || y >= usize::from(self.height) {
            return false;
        }
        let byte = self.rows[y * self.row_bytes() + x / BITS_PER_BYTE];
        byte & (HIGH_BIT >> (x % BITS_PER_BYTE)) != 0
    }

    /// How far the glyph moves the pen.
    pub fn advance(&self) -> u32 {
        (i32::from(self.offset_x) + i32::from(self.width) + 1).max(0) as u32
    }
}

/// Every Unicode font of the client.
#[derive(Clone, Debug, Default)]
pub struct UnicodeFonts {
    files: Vec<Option<Vec<u8>>>,
}

impl UnicodeFonts {
    /// Reads every Unicode font file that is there. `unifont.mul` must be.
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let dir = uopath.as_ref();
        let mut files = Vec::with_capacity(UNIFONT_MAX);
        for font in 0..UNIFONT_MAX {
            let path = dir.join(unifont_name(font));
            files.push(if path.exists() {
                Some(read_file(&path)?)
            } else {
                None
            });
        }
        if files[usize::from(FALLBACK_FONT)].is_none() {
            return Err(MapError::Missing(UNIFONT_NAME));
        }
        Ok(Self { files })
    }

    fn file(&self, font: u8) -> Option<&[u8]> {
        let own = |font: u8| self.files.get(usize::from(font))?.as_deref();
        match own(font) {
            None if font == ALIASED_FONT => own(FALLBACK_FONT),
            found => found,
        }
    }

    /// True when the client has this font. A missing `unifont1.mul` is
    /// `unifont.mul` again.
    pub fn has_font(&self, font: u8) -> bool {
        self.file(font).is_some()
    }

    fn own_glyph(&self, font: u8, ch: char) -> Option<UnicodeGlyph<'_>> {
        let data = self.file(font)?;
        let code = u32::from(ch);
        if code >= LOOKUP_CHARS {
            return None;
        }
        let place = slice_at(data, code as usize * LOOKUP_ENTRY, LOOKUP_ENTRY)?;
        let at = i32::from_le_bytes([place[0], place[1], place[2], place[3]]);
        let at = usize::try_from(at).ok().filter(|at| *at > 0)?;
        let head = slice_at(data, at, GLYPH_HEADER)?;
        let [offset_x, offset_y, width, height] =
            [head[0], head[1], head[2], head[3]].map(|b| b as i8);
        let width = u8::try_from(width).ok().filter(|w| *w > 0)?;
        let height = u8::try_from(height).ok().filter(|h| *h > 0)?;
        let row_bytes = (usize::from(width) - 1) / BITS_PER_BYTE + 1;
        let rows = slice_at(data, at + GLYPH_HEADER, row_bytes * usize::from(height))?;
        Some(UnicodeGlyph {
            offset_x,
            offset_y,
            width,
            height,
            rows,
        })
    }

    /// The glyph a font draws for a char. A char the font does not hold is
    /// taken from `unifont.mul`. None for a font the client does not have.
    pub fn glyph(&self, font: u8, ch: char) -> Option<UnicodeGlyph<'_>> {
        if !self.has_font(font) {
            return None;
        }
        self.own_glyph(font, ch)
            .or_else(|| self.own_glyph(FALLBACK_FONT, ch))
    }

    fn metric(&self, font: u8, ch: char, style: UnicodeStyle) -> Option<CharMetric> {
        if ch == SPACE {
            return self.has_font(font).then_some(CharMetric {
                advance: UNICODE_SPACE_WIDTH,
                height: SPACE_LINE_HEIGHT + style.extra_height(),
            });
        }
        self.glyph(font, ch).map(|g| CharMetric {
            advance: g.advance(),
            height: (i32::from(g.offset_y) + i32::from(g.height)).max(0) as u32
                + style.extra_height(),
        })
    }

    /// The width of `text` in pixels, as its widest line.
    pub fn width(&self, font: u8, text: &str) -> u32 {
        widest_line(text, |ch| self.metric(font, ch, UnicodeStyle::default()))
    }

    /// The lines of `text` in a font, broken to fit `max_width` when one is
    /// given, with the height each line takes.
    pub fn layout(
        &self,
        font: u8,
        text: &str,
        max_width: Option<u32>,
        style: UnicodeStyle,
    ) -> Vec<TextLine> {
        if !self.has_font(font) {
            return Vec::new();
        }
        let empty = UNICODE_EMPTY_LINE_HEIGHT + style.extra_height();
        wrap(text, max_width, empty, |ch| self.metric(font, ch, style))
    }

    /// The height of the block `text` makes in a font.
    pub fn height(&self, font: u8, text: &str, max_width: Option<u32>, style: UnicodeStyle) -> u32 {
        self.layout(font, text, max_width, style)
            .iter()
            .map(|l| l.height)
            .sum()
    }

    /// `text` whole when it fits in `max_width` on one line, and cut short
    /// with three dots when it does not.
    pub fn crop(&self, font: u8, text: &str, max_width: u32) -> String {
        let dots = self
            .glyph(font, '.')
            .map_or(0, |g| (u32::from(g.width) + 1) * ELLIPSIS.len() as u32);
        crop(text, max_width, dots, |ch| {
            self.metric(font, ch, UnicodeStyle::default())
        })
    }

    /// The words in one RGBA color. The picture is as wide as `max_width`,
    /// or as the widest line when none is given, plus a margin on the right
    /// and the bottom. None when there is nothing to draw.
    pub fn render(
        &self,
        font: u8,
        text: &str,
        max_width: Option<u32>,
        align: TextAlign,
        style: UnicodeStyle,
        color: [u8; RGBA_BYTES],
    ) -> Option<TextPicture> {
        let inner = max_width.unwrap_or_else(|| self.width(font, text));
        if inner == 0 {
            return None;
        }
        let lines = self.layout(font, text, Some(inner), style);
        let lines_height: u32 = lines.iter().map(|l| l.height).sum();
        if lines_height == 0 {
            return None;
        }
        let mut canvas = Canvas::new(
            (inner + UNICODE_PICTURE_PADDING) as i32,
            (lines_height + UNICODE_PICTURE_PADDING) as i32,
        );
        let pen = Pen {
            color: u32::from_le_bytes(color),
            dark: color[..3].iter().all(|c| *c <= DARK_CHANNEL_MAX),
            style,
            underline_drop: self
                .glyph(font, UNDERLINE_MEASURE)
                .map_or(0, |g| i32::from(g.offset_y) + i32::from(g.height)),
        };
        let mut top = 0i32;
        for line in &lines {
            let left = line_start(canvas.width, line.width as i32, align);
            self.draw_line(&mut canvas, &pen, font, &line.text, left, top);
            top += line.height as i32;
        }
        Some(canvas.into_picture())
    }

    /// The words in a hue, as the client colors Unicode words.
    #[allow(clippy::too_many_arguments)]
    pub fn render_hued(
        &self,
        font: u8,
        text: &str,
        max_width: Option<u32>,
        align: TextAlign,
        style: UnicodeStyle,
        hue: u16,
        hues: &HueData,
    ) -> Option<TextPicture> {
        let color = unicode_text_color(hue, hues);
        self.render(font, text, max_width, align, style, color)
    }

    fn draw_line(&self, canvas: &mut Canvas, pen: &Pen, font: u8, text: &str, left: i32, top: i32) {
        let bold = i32::from(pen.style.bold);
        let mut x = left;
        for ch in text.chars() {
            let start = x;
            let (offset_x, width) = if ch == SPACE {
                x += UNICODE_SPACE_WIDTH as i32;
                (0, UNICODE_SPACE_WIDTH as i32)
            } else {
                let Some(glyph) = self.glyph(font, ch) else {
                    continue;
                };
                let offset_x = i32::from(glyph.offset_x) + 1;
                let place = GlyphPlace {
                    left: x + offset_x,
                    top: top + i32::from(glyph.offset_y),
                    width: i32::from(glyph.width),
                    height: i32::from(glyph.height),
                    italic: pen.style.italic,
                };
                canvas.ink(&glyph, &place, bold, pen.color);
                if pen.style.bold {
                    canvas.embolden(&place, pen.color);
                }
                if pen.style.border && !pen.dark {
                    canvas.ring(&place);
                }
                x += place.width + offset_x + bold;
                (offset_x, place.width)
            };
            if pen.style.underline {
                let min_x = if start + offset_x > 0 { -1 } else { 0 };
                let max_x = width + i32::from(x + offset_x + width < canvas.width);
                let y = top + pen.underline_drop;
                if y >= canvas.height {
                    break;
                }
                for cx in min_x..max_x {
                    let under_x = cx + start + offset_x + bold;
                    if under_x >= canvas.width {
                        break;
                    }
                    canvas.set(under_x, y.max(0), pen.color);
                }
            }
        }
    }
}

/// The color words take in a hue. Hue 0xFFFF is white; hue zero and a hue
/// the file does not hold are near black.
pub fn unicode_text_color(hue: u16, hues: &HueData) -> [u8; RGBA_BYTES] {
    if hue == UNICODE_WHITE_HUE {
        return WHITE;
    }
    hues.step_rgb(hue, UNICODE_HUE_STEP)
        .map_or(NEAR_BLACK, |[r, g, b]| [r, g, b, u8::MAX])
}

/// Where the pen starts on a line of `line_width` in a picture of `width`.
fn line_start(width: i32, line_width: i32, align: TextAlign) -> i32 {
    match align {
        TextAlign::Left => 0,
        TextAlign::Center => ((width - CENTER_MARGIN) / 2 - line_width / 2).max(0),
        TextAlign::Right => (width - RIGHT_MARGIN - line_width).max(0),
    }
}

/// What every glyph of one block is drawn with.
struct Pen {
    color: u32,
    /// True for ink too dark to take a border.
    dark: bool,
    style: UnicodeStyle,
    /// How far below the top of a line the underline runs.
    underline_drop: i32,
}

/// Where one glyph lies on the picture.
struct GlyphPlace {
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    italic: bool,
}

impl GlyphPlace {
    /// How far italic moves one row of the glyph to the right.
    fn slant(&self, row: i32) -> i32 {
        if self.italic && (0..self.height).contains(&row) {
            ((self.height - row) as f32 / ITALIC_SLANT) as i32
        } else {
            0
        }
    }

    /// The picture row of a glyph row. Rows above the picture draw on its
    /// top row.
    fn row(&self, row: i32) -> i32 {
        (self.top + row).max(0)
    }
}

/// The picture being drawn, one packed RGBA color for each pixel.
struct Canvas {
    width: i32,
    height: i32,
    pixels: Vec<u32>,
}

impl Canvas {
    fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            pixels: vec![CLEAR; (width * height) as usize],
        }
    }

    fn get(&self, x: i32, y: i32) -> u32 {
        if (0..self.width).contains(&x) && (0..self.height).contains(&y) {
            self.pixels[(y * self.width + x) as usize]
        } else {
            CLEAR
        }
    }

    fn set(&mut self, x: i32, y: i32, color: u32) {
        if (0..self.width).contains(&x) && (0..self.height).contains(&y) {
            self.pixels[(y * self.width + x) as usize] = color;
        }
    }

    /// Draws the ink of a glyph. Bold draws it one pixel to the right.
    fn ink(&mut self, glyph: &UnicodeGlyph<'_>, place: &GlyphPlace, bold: i32, color: u32) {
        for row in 0..place.height {
            let y = place.row(row);
            if y >= self.height {
                break;
            }
            let left = place.left + place.slant(row) + bold;
            for column in 0..place.width {
                let x = left + column;
                if x >= self.width {
                    break;
                }
                if glyph.is_ink(column as usize, row as usize) {
                    self.set(x, y, color);
                }
            }
        }
    }

    /// Bold: each clear pixel left of the ink is filled, which makes every
    /// stroke two pixels wide. A pixel filled outside the glyph box stays a
    /// dark edge.
    fn embolden(&mut self, place: &GlyphPlace, color: u32) {
        let near_black = u32::from_le_bytes(NEAR_BLACK);
        let mark = if near_black == color {
            near_black + 1
        } else {
            near_black
        };
        let min_x = if place.left > 0 { -1 } else { 0 };
        let max_x = place.width + i32::from(place.left + place.width < self.width);
        for row in 0..place.height {
            let y = place.row(row);
            if y >= self.height {
                break;
            }
            for column in min_x..max_x {
                let x = column + place.left + place.slant(row);
                if x >= self.width {
                    break;
                }
                if self.get(x, y) != CLEAR {
                    continue;
                }
                let reach = if column < place.width && x + 1 < self.width {
                    2
                } else {
                    1
                };
                let inked = (0..reach).any(|step| {
                    let next = self.get(x + step, y);
                    next != CLEAR && next != mark
                });
                if inked {
                    self.set(x, y, mark);
                }
            }
        }
        for row in 0..place.height {
            let y = place.row(row);
            if y >= self.height {
                break;
            }
            for column in 0..place.width {
                let x = column + place.left + place.slant(row);
                if x >= self.width {
                    break;
                }
                if self.get(x, y) == mark {
                    self.set(x, y, color);
                }
            }
        }
    }

    /// Border: each clear pixel next to ink turns near black.
    fn ring(&mut self, place: &GlyphPlace) {
        let black = u32::from_le_bytes(NEAR_BLACK);
        let min_x = if place.left > 0 { -1 } else { 0 };
        let min_y = if place.top > 0 { -1 } else { 0 };
        let max_x = place.width + i32::from(place.left + place.width < self.width);
        let max_y = place.height + i32::from(place.top + place.height < self.height);
        for row in min_y..max_y {
            let y = place.row(row);
            if y >= self.height {
                break;
            }
            for column in min_x..max_x {
                let x = column + place.left + place.slant(row);
                if x >= self.width {
                    break;
                }
                if self.get(x, y) != CLEAR {
                    continue;
                }
                let from_x = if column > 0 { -1 } else { 0 };
                let from_y = if row > 0 { -1 } else { 0 };
                let to_x = if column < place.width - 1 && x + 1 < self.width {
                    2
                } else {
                    1
                };
                let to_y = if row < place.height - 1 { 2 } else { 1 };
                let touches_ink = (from_x..to_x).any(|dx| {
                    (from_y..to_y).any(|dy| {
                        let near = self.get(x + dx, y + dy);
                        near != CLEAR && near != black
                    })
                });
                if touches_ink {
                    self.set(x, y, black);
                }
            }
        }
    }

    fn into_picture(self) -> TextPicture {
        TextPicture {
            width: self.width as usize,
            height: self.height as usize,
            rgba: self.pixels.iter().flat_map(|p| p.to_le_bytes()).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INK: [u8; RGBA_BYTES] = [200, 100, 50, u8::MAX];
    /// A glyph two wide and two high, all ink, with no offsets.
    const BLOCK: char = 'A';
    /// A glyph only the fallback font holds.
    const FALLBACK_ONLY: char = 'Z';
    /// A glyph one pixel wide with its ink one row down.
    const DOT: char = '.';

    fn glyph_bytes(offset_y: i8, width: i8, height: i8, rows: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8, offset_y as u8, width as u8, height as u8];
        out.extend_from_slice(rows);
        out
    }

    fn font_file(glyphs: &[(char, Vec<u8>)]) -> Vec<u8> {
        let mut data = vec![0u8; LOOKUP_CHARS as usize * LOOKUP_ENTRY];
        for (ch, bytes) in glyphs {
            let at = data.len() as i32;
            let place = u32::from(*ch) as usize * LOOKUP_ENTRY;
            data[place..place + LOOKUP_ENTRY].copy_from_slice(&at.to_le_bytes());
            data.extend(bytes);
        }
        data
    }

    fn fonts() -> UnicodeFonts {
        let block = glyph_bytes(0, 2, 2, &[0xC0, 0xC0]);
        let dot = glyph_bytes(1, 1, 1, &[0x80]);
        let base = font_file(&[
            (BLOCK, block.clone()),
            (DOT, dot),
            (FALLBACK_ONLY, block.clone()),
            (UNDERLINE_MEASURE, block.clone()),
        ]);
        let third = font_file(&[(BLOCK, block)]);
        UnicodeFonts {
            files: vec![Some(base), None, None, Some(third)],
        }
    }

    fn pixel(picture: &TextPicture, x: usize, y: usize) -> [u8; RGBA_BYTES] {
        let at = (y * picture.width + x) * RGBA_BYTES;
        picture.rgba[at..at + RGBA_BYTES].try_into().unwrap()
    }

    #[test]
    fn glyphs_read_their_offsets_and_bits() {
        let fonts = fonts();
        let dot = fonts.glyph(0, DOT).unwrap();
        assert_eq!((dot.offset_y, dot.width, dot.height), (1, 1, 1));
        assert!(dot.is_ink(0, 0));
        assert!(!dot.is_ink(1, 0));
        assert_eq!(fonts.glyph(0, BLOCK).unwrap().advance(), 3);
        assert!(fonts.glyph(0, 'q').is_none());
    }

    #[test]
    fn a_missing_glyph_comes_from_the_first_font_and_font_one_is_font_zero() {
        let fonts = fonts();
        assert!(fonts.glyph(3, FALLBACK_ONLY).is_some());
        assert!(fonts.has_font(1));
        assert!(fonts.glyph(1, DOT).is_some());
        assert!(!fonts.has_font(2));
        assert!(fonts.glyph(2, BLOCK).is_none());
        assert!(fonts
            .layout(2, "A", None, UnicodeStyle::default())
            .is_empty());
    }

    #[test]
    fn a_damaged_glyph_is_none() {
        let mut data = font_file(&[(BLOCK, glyph_bytes(0, 16, 4, &[0xFF]))]);
        let fonts = UnicodeFonts {
            files: vec![Some(data.clone())],
        };
        assert!(fonts.glyph(0, BLOCK).is_none());
        data.truncate(LOOKUP_ENTRY);
        let fonts = UnicodeFonts {
            files: vec![Some(data)],
        };
        assert!(fonts.glyph(0, BLOCK).is_none());
    }

    #[test]
    fn spaces_are_eight_wide_and_lines_are_as_tall_as_their_glyphs() {
        let fonts = fonts();
        assert_eq!(fonts.width(0, "A A"), 3 + UNICODE_SPACE_WIDTH + 3);
        let plain = UnicodeStyle::default();
        assert_eq!(fonts.height(0, "A", None, plain), 2);
        assert_eq!(fonts.height(0, " ", None, plain), SPACE_LINE_HEIGHT);
        let tall = UnicodeStyle {
            extra_height: true,
            ..plain
        };
        assert_eq!(
            fonts.height(0, "A\n", None, tall),
            2 + 2 * EXTRA_LINE_HEIGHT + 14
        );
        assert_eq!(fonts.layout(0, "AA AA", Some(8), plain).len(), 2);
    }

    #[test]
    fn plain_words_draw_their_ink_one_pixel_in() {
        let picture = fonts()
            .render(0, "A", None, TextAlign::Left, UnicodeStyle::default(), INK)
            .unwrap();
        assert_eq!(picture.width, 3 + UNICODE_PICTURE_PADDING as usize);
        assert_eq!(picture.height, 2 + UNICODE_PICTURE_PADDING as usize);
        assert_eq!(pixel(&picture, 0, 0), [0; RGBA_BYTES]);
        assert_eq!(pixel(&picture, 1, 0), INK);
        assert_eq!(pixel(&picture, 2, 1), INK);
        assert_eq!(pixel(&picture, 3, 0), [0; RGBA_BYTES]);
    }

    #[test]
    fn a_border_rings_the_ink_in_black() {
        let style = UnicodeStyle {
            border: true,
            ..UnicodeStyle::default()
        };
        let picture = fonts()
            .render(0, "A", None, TextAlign::Left, style, INK)
            .unwrap();
        assert_eq!(pixel(&picture, 0, 0), NEAR_BLACK);
        assert_eq!(pixel(&picture, 3, 1), NEAR_BLACK);
        assert_eq!(pixel(&picture, 1, 2), NEAR_BLACK);
        assert_eq!(pixel(&picture, 1, 1), INK);
        let dark = fonts()
            .render(0, "A", None, TextAlign::Left, style, NEAR_BLACK)
            .unwrap();
        assert_eq!(pixel(&dark, 0, 0), [0; RGBA_BYTES]);
    }

    #[test]
    fn bold_widens_the_strokes_and_underline_runs_under_the_letters() {
        let bold = UnicodeStyle {
            bold: true,
            ..UnicodeStyle::default()
        };
        let picture = fonts()
            .render(0, "A", Some(8), TextAlign::Left, bold, INK)
            .unwrap();
        assert_eq!(pixel(&picture, 1, 0), INK);
        assert_eq!(pixel(&picture, 2, 0), INK);
        assert_eq!(pixel(&picture, 3, 0), INK);
        let under = UnicodeStyle {
            underline: true,
            ..UnicodeStyle::default()
        };
        let picture = fonts()
            .render(0, "A", Some(8), TextAlign::Left, under, INK)
            .unwrap();
        assert_eq!(pixel(&picture, 1, 2), INK);
        assert_eq!(pixel(&picture, 0, 2), INK);
    }

    #[test]
    fn italic_slants_the_top_rows_right() {
        let italic = UnicodeStyle {
            italic: true,
            ..UnicodeStyle::default()
        };
        let tall = glyph_bytes(0, 1, 8, &[0x80; 8]);
        let fonts = UnicodeFonts {
            files: vec![Some(font_file(&[(BLOCK, tall)]))],
        };
        let picture = fonts
            .render(0, "A", Some(8), TextAlign::Left, italic, INK)
            .unwrap();
        assert_eq!(pixel(&picture, 3, 0), INK);
        assert_eq!(pixel(&picture, 1, 7), INK);
    }

    #[test]
    fn lines_align_left_center_and_right() {
        assert_eq!(line_start(40, 10, TextAlign::Left), 0);
        assert_eq!(line_start(40, 10, TextAlign::Center), 11);
        assert_eq!(line_start(40, 10, TextAlign::Right), 20);
        assert_eq!(line_start(12, 10, TextAlign::Right), 0);
    }

    #[test]
    fn a_label_is_cut_with_dots() {
        let fonts = fonts();
        assert_eq!(fonts.crop(0, "AAA", 9), "AAA");
        assert_eq!(fonts.crop(0, "AAAAAA", 12), "AA...");
    }

    #[test]
    fn the_real_fonts_draw_speech_with_a_border_when_client_files_are_here() {
        const SPEECH_FONT: u8 = 1;
        const SPEECH_HUE: u16 = 0x0034;
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        let fonts = UnicodeFonts::open(&dir).unwrap();
        let hues = HueData::open(&dir).unwrap();
        let style = UnicodeStyle {
            border: true,
            ..UnicodeStyle::default()
        };
        for font in 0..=12u8 {
            assert!(fonts.has_font(font), "font {font}");
            assert!(fonts.width(font, "Hail, friend!") > 0, "font {font}");
        }
        let text = "Hail, friend! Well met in Britain.";
        let picture = fonts
            .render_hued(
                SPEECH_FONT,
                text,
                Some(120),
                TextAlign::Center,
                style,
                SPEECH_HUE,
                &hues,
            )
            .unwrap();
        assert_eq!(picture.width, 120 + UNICODE_PICTURE_PADDING as usize);
        let pixels: Vec<&[u8]> = picture.rgba.chunks_exact(RGBA_BYTES).collect();
        assert!(pixels.iter().any(|p| *p == NEAR_BLACK));
        assert!(pixels.iter().any(|p| p[3] == u8::MAX && *p != NEAR_BLACK));
        assert!(fonts.layout(SPEECH_FONT, text, Some(120), style).len() > 1);
        assert_eq!(unicode_text_color(UNICODE_WHITE_HUE, &hues), WHITE);
        assert_eq!(unicode_text_color(0, &hues), NEAR_BLACK);
    }
}
