//! The ASCII fonts of `fonts.mul`: the old bitmap fonts of names over heads,
//! of the old gumps and of the journal before the Unicode fonts.
//!
//! The file is one font after the other. A font is a header byte, then 224
//! glyphs for the chars 32 to 255. A glyph is its width, its height, a byte
//! no reader needs, and one 15-bit color for each pixel. Color zero is clear.

use std::path::Path;

use crate::art::{ArtPixels, PIXEL_DRAWN};
use crate::hues::{HueData, HueRamp};
use crate::mul::{read_file, slice_at, MapError};
use crate::text::{
    crop, widest_line, wrap, CharMetric, TextAlign, TextLine, TextPicture, ELLIPSIS,
};

pub const FONTS_NAME: &str = "fonts.mul";
/// The glyphs of one font, for the chars 32 to 255.
pub const ASCII_GLYPH_COUNT: usize = 224;
/// The first char a font draws. A lower char draws as the first glyph.
pub const ASCII_FIRST_CHAR: u32 = 32;
/// A line with no glyph on it takes this height.
pub const ASCII_EMPTY_LINE_HEIGHT: u32 = 14;
/// The picture of a block is this much wider than its lines may be.
pub const TEXT_PICTURE_PADDING: u32 = 4;

const FONT_HEADER: usize = 1;
const GLYPH_HEADER: usize = 3;
const WORD: usize = 2;
const LAST_CHAR: u32 = ASCII_FIRST_CHAR + ASCII_GLYPH_COUNT as u32 - 1;
/// Fonts 5 and 8 take a hue on every pixel; the others on grey pixels only.
const FULL_HUE_FONTS: [u8; 2] = [5, 8];
/// Font 6 sets its lines closer by this many pixels.
const CLOSE_LINES_FONT: u8 = 6;
const CLOSE_LINES_BY: u32 = 7;
/// Font 4 sets its lines further apart.
const WIDE_LINES_FONT: u8 = 4;
const WIDE_LINES_BY: u32 = 2;
const WIDE_EMPTY_LINES_BY: u32 = 6;
/// A right-aligned line ends this far from the right edge.
const RIGHT_MARGIN: i32 = 10;
/// The fonts below this number drop small letters and marks by a table of
/// their own. The others drop them by one step.
const TABLED_FONTS: usize = 10;
const SMALL_LETTER_DROP: [i32; TABLED_FONTS] = [2, 0, 2, 2, 0, 0, 2, 2, 0, 0];
const MARK_DROP: [i32; TABLED_FONTS] = [1, 0, 1, 1, -1, 0, 1, 1, 0, 0];
const UNTABLED_DROP: i32 = 2;
const CEDILLA: u32 = 0xB8;
const CEDILLA_DROP: i32 = 1;
const DIAERESIS: u32 = 0xA8;
const CAPITALS: std::ops::RangeInclusive<u32> = 0x41..=0x5A;
const ACCENTED_CAPITALS: std::ops::RangeInclusive<u32> = 0xC0..=0xDF;
const SMALL_LETTERS: std::ops::RangeInclusive<u32> = 0x61..=0x7A;

/// One glyph of an ASCII font.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AsciiGlyph {
    pub width: u8,
    pub height: u8,
    /// Row order. Zero is clear.
    pub colors: Vec<u16>,
}

/// Every ASCII font of the client.
#[derive(Clone, Debug, Default)]
pub struct AsciiFonts {
    fonts: Vec<Vec<AsciiGlyph>>,
}

/// The place of a char in a font. A char the fonts do not hold draws as the
/// first glyph, as a control char does.
fn glyph_index(ch: char) -> usize {
    let code = u32::from(ch);
    if (ASCII_FIRST_CHAR..=LAST_CHAR).contains(&code) {
        (code - ASCII_FIRST_CHAR) as usize
    } else {
        0
    }
}

/// How far below the top of its line a glyph is drawn. Capitals sit on the
/// top; small letters and marks drop by a step each font sets.
fn glyph_drop(font: u8, ch: char) -> i32 {
    let code = u32::from(ch);
    if code == CEDILLA {
        return CEDILLA_DROP;
    }
    if CAPITALS.contains(&code) || ACCENTED_CAPITALS.contains(&code) || code == DIAERESIS {
        return 0;
    }
    let font = usize::from(font);
    if font >= TABLED_FONTS {
        return UNTABLED_DROP;
    }
    if SMALL_LETTERS.contains(&code) {
        SMALL_LETTER_DROP[font]
    } else {
        MARK_DROP[font]
    }
}

/// The glyphs of one font from `at`, and where the font ends. None when the
/// file ends inside the font.
fn parse_font(data: &[u8], mut at: usize) -> Option<(Vec<AsciiGlyph>, usize)> {
    slice_at(data, at, FONT_HEADER)?;
    at += FONT_HEADER;
    let mut glyphs = Vec::with_capacity(ASCII_GLYPH_COUNT);
    for _ in 0..ASCII_GLYPH_COUNT {
        let header = slice_at(data, at, GLYPH_HEADER)?;
        let (width, height) = (header[0], header[1]);
        at += GLYPH_HEADER;
        let bytes = usize::from(width) * usize::from(height) * WORD;
        let colors = slice_at(data, at, bytes)?
            .chunks_exact(WORD)
            .map(|w| u16::from_le_bytes([w[0], w[1]]))
            .collect();
        at += bytes;
        glyphs.push(AsciiGlyph {
            width,
            height,
            colors,
        });
    }
    Some((glyphs, at))
}

impl AsciiFonts {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let path = uopath.as_ref().join(FONTS_NAME);
        if !path.exists() {
            return Err(MapError::Missing(FONTS_NAME));
        }
        Ok(Self::parse(&read_file(&path)?))
    }

    /// Every whole font of the file. A font the file cuts short is dropped.
    pub(crate) fn parse(data: &[u8]) -> Self {
        let mut fonts = Vec::new();
        let mut at = 0;
        while let Some((glyphs, end)) = parse_font(data, at) {
            fonts.push(glyphs);
            at = end;
        }
        Self { fonts }
    }

    /// How many fonts the file holds.
    pub fn count(&self) -> usize {
        self.fonts.len()
    }

    /// The glyph a font draws for a char, or None for a font the file does
    /// not hold.
    pub fn glyph(&self, font: u8, ch: char) -> Option<&AsciiGlyph> {
        self.fonts.get(usize::from(font))?.get(glyph_index(ch))
    }

    fn metric(&self, font: u8, ch: char) -> Option<CharMetric> {
        self.glyph(font, ch).map(|g| CharMetric {
            advance: u32::from(g.width),
            height: u32::from(g.height),
        })
    }

    /// The width of `text` in pixels, as its widest line.
    pub fn width(&self, font: u8, text: &str) -> u32 {
        widest_line(text, |ch| self.metric(font, ch))
    }

    /// The lines of `text` in a font, broken to fit `max_width` when one is
    /// given, with the height each line takes.
    pub fn layout(&self, font: u8, text: &str, max_width: Option<u32>) -> Vec<TextLine> {
        if usize::from(font) >= self.count() {
            return Vec::new();
        }
        let mut lines = wrap(text, max_width, ASCII_EMPTY_LINE_HEIGHT, |ch| {
            self.metric(font, ch)
        });
        if font == WIDE_LINES_FONT {
            for line in &mut lines {
                line.height += if line.text.is_empty() {
                    WIDE_EMPTY_LINES_BY
                } else {
                    WIDE_LINES_BY
                };
            }
        }
        lines
    }

    /// The height of the block `text` makes in a font.
    pub fn height(&self, font: u8, text: &str, max_width: Option<u32>) -> u32 {
        self.layout(font, text, max_width)
            .iter()
            .map(|l| l.height)
            .sum()
    }

    /// `text` whole when it fits in `max_width` on one line, and cut short
    /// with three dots when it does not.
    pub fn crop(&self, font: u8, text: &str, max_width: u32) -> String {
        let dot = self.metric(font, '.').map_or(0, |m| m.advance);
        let dots = dot * ELLIPSIS.len() as u32;
        crop(text, max_width, dots, |ch| self.metric(font, ch))
    }

    /// The words in the colors of the font. The picture is as wide as
    /// `max_width`, or as the widest line when none is given, plus a margin.
    /// None when there is nothing to draw.
    pub fn render(
        &self,
        font: u8,
        text: &str,
        max_width: Option<u32>,
        align: TextAlign,
    ) -> Option<ArtPixels> {
        let lines = self.layout(font, text, max_width);
        let inner = max_width.unwrap_or_else(|| lines.iter().map(|l| l.width).max().unwrap_or(0));
        let height: u32 = lines.iter().map(|l| l.height).sum();
        if inner == 0 || height == 0 {
            return None;
        }
        let width = inner + TEXT_PICTURE_PADDING;
        let mut picture = ArtPixels::clear(width as usize, height as usize);
        let line_gap = if font == CLOSE_LINES_FONT {
            CLOSE_LINES_BY
        } else {
            0
        };
        let mut top = 0i32;
        for line in &lines {
            let mut pen = line_start(width as i32, line.width as i32, align);
            for ch in line.text.chars() {
                let Some(glyph) = self.glyph(font, ch) else {
                    continue;
                };
                draw_glyph(&mut picture, glyph, pen, top + glyph_drop(font, ch));
                pen += i32::from(glyph.width);
            }
            top += line.height.saturating_sub(line_gap) as i32;
        }
        Some(picture)
    }

    /// The words in RGBA, in a hue. Fonts 5 and 8 take the hue on every
    /// pixel, the others on their grey pixels only. Hue zero keeps the
    /// colors of the font.
    pub fn render_rgba(
        &self,
        font: u8,
        text: &str,
        max_width: Option<u32>,
        align: TextAlign,
        hue: u16,
        hues: &HueData,
    ) -> Option<TextPicture> {
        let picture = self.render(font, text, max_width, align)?;
        Some(TextPicture {
            width: picture.width,
            height: picture.height,
            rgba: picture.rgba(ascii_hue_ramp(font, hue, hues)),
        })
    }
}

/// The ramp that colors a font in a hue.
pub fn ascii_hue_ramp(font: u8, hue: u16, hues: &HueData) -> Option<HueRamp<'_>> {
    hues.ramp(hue, !FULL_HUE_FONTS.contains(&font))
}

/// Puts the ink of one glyph on the picture, with its top left corner at
/// `left`, `top`. Ink outside the picture is left out.
fn draw_glyph(picture: &mut ArtPixels, glyph: &AsciiGlyph, left: i32, top: i32) {
    let width = usize::from(glyph.width);
    for (row, colors) in glyph.colors.chunks_exact(width.max(1)).enumerate() {
        let y = top + row as i32;
        if y < 0 {
            continue;
        }
        if y as usize >= picture.height {
            break;
        }
        for (column, &color) in colors.iter().enumerate() {
            let x = left + column as i32;
            if x < 0 || color == 0 {
                continue;
            }
            if x as usize >= picture.width {
                break;
            }
            picture.colors[y as usize * picture.width + x as usize] = color | PIXEL_DRAWN;
        }
    }
}

/// Where the pen starts on a line of `line_width` in a picture of `width`.
/// A right-aligned line too wide for the picture is not drawn.
fn line_start(width: i32, line_width: i32, align: TextAlign) -> i32 {
    match align {
        TextAlign::Left => 0,
        TextAlign::Center => ((width - line_width) / 2).max(0),
        TextAlign::Right => match width - RIGHT_MARGIN - line_width {
            start if start < 0 => width,
            start => start,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INK: u16 = 0x7FFF;
    const RGBA_BYTES: usize = 4;
    const ALPHA: usize = 3;

    /// One font where every glyph is `width` wide and two high with a top
    /// row of ink, and the glyph of `!` is empty.
    fn font_bytes(width: u8) -> Vec<u8> {
        const HEIGHT: u8 = 2;
        let mut data = vec![0u8];
        for index in 0..ASCII_GLYPH_COUNT {
            let empty = index == glyph_index('!');
            data.extend([width, HEIGHT, 0]);
            for row in 0..HEIGHT {
                for _ in 0..width {
                    let color = if row == 0 && !empty { INK } else { 0 };
                    data.extend(color.to_le_bytes());
                }
            }
        }
        data
    }

    fn two_fonts() -> AsciiFonts {
        let mut data = font_bytes(3);
        data.extend(font_bytes(4));
        AsciiFonts::parse(&data)
    }

    #[test]
    fn every_whole_font_is_read_and_a_cut_one_is_dropped() {
        let mut data = font_bytes(3);
        data.extend(font_bytes(4));
        data.extend(&font_bytes(5)[..10]);
        let fonts = AsciiFonts::parse(&data);
        assert_eq!(fonts.count(), 2);
        assert_eq!(fonts.glyph(1, 'A').unwrap().width, 4);
        assert!(fonts.glyph(2, 'A').is_none());
        assert_eq!(AsciiFonts::parse(&[]).count(), 0);
    }

    #[test]
    fn control_and_wide_chars_draw_as_the_first_glyph() {
        assert_eq!(glyph_index('\t'), 0);
        assert_eq!(glyph_index('\u{2603}'), 0);
        assert_eq!(glyph_index('A'), 0x41 - ASCII_FIRST_CHAR as usize);
    }

    #[test]
    fn width_and_height_come_from_the_glyphs() {
        let fonts = two_fonts();
        assert_eq!(fonts.width(0, "abcd"), 12);
        assert_eq!(fonts.width(1, "ab\nabc"), 12);
        assert_eq!(fonts.height(0, "ab cd", Some(6)), 4);
    }

    #[test]
    fn small_letters_drop_by_the_table_of_the_font() {
        assert_eq!(glyph_drop(0, 'a'), 2);
        assert_eq!(glyph_drop(0, 'A'), 0);
        assert_eq!(glyph_drop(4, '!'), -1);
        assert_eq!(glyph_drop(12, 'a'), UNTABLED_DROP);
        assert_eq!(glyph_drop(1, '\u{B8}'), CEDILLA_DROP);
    }

    #[test]
    fn a_picture_is_padded_and_aligned() {
        let fonts = two_fonts();
        let left = fonts.render(0, "A", Some(9), TextAlign::Left).unwrap();
        assert_eq!(left.width, 9 + TEXT_PICTURE_PADDING as usize);
        assert_eq!(left.height, 2);
        assert_eq!(left.colors[0], INK | PIXEL_DRAWN);
        let center = fonts.render(0, "A", Some(9), TextAlign::Center).unwrap();
        assert_eq!(center.colors[0], 0);
        assert_eq!(center.colors[5], INK | PIXEL_DRAWN);
        let right = fonts.render(0, "A", Some(20), TextAlign::Right).unwrap();
        assert_eq!(right.colors[24 - 10 - 3], INK | PIXEL_DRAWN);
        assert!(fonts.render(0, "", None, TextAlign::Left).is_none());
        assert!(fonts.render(9, "A", None, TextAlign::Left).is_none());
    }

    #[test]
    fn a_label_is_cut_with_dots() {
        let fonts = two_fonts();
        assert_eq!(fonts.crop(0, "abcdef", 30), "abcdef");
        assert_eq!(fonts.crop(0, "abcdefgh", 18), "abc...");
    }

    #[test]
    fn the_real_fonts_draw_a_name_when_client_files_are_here() {
        const TEST_FONT: u8 = 3;
        const FIRST_HUE: u16 = 1;
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        let fonts = AsciiFonts::open(&dir).unwrap();
        assert!(fonts.count() >= 10, "fonts 0 to 9 are in every client");
        let hues = HueData::open(&dir).unwrap();
        let picture = fonts
            .render_rgba(
                TEST_FONT,
                "Lord British",
                None,
                TextAlign::Left,
                FIRST_HUE,
                &hues,
            )
            .unwrap();
        assert_eq!(
            picture.width as u32,
            fonts.width(TEST_FONT, "Lord British") + TEXT_PICTURE_PADDING
        );
        assert!(picture
            .rgba
            .chunks_exact(RGBA_BYTES)
            .any(|p| p[ALPHA] == u8::MAX));
        let lines = fonts.layout(TEST_FONT, "Lord British of Britain", Some(60));
        assert!(lines.len() > 1 && lines.iter().all(|l| l.width <= 60));
    }
}
