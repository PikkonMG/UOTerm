//! Words in the fonts of the client, and the HTML of gumps. The fonts are
//! `crate::art::text`, and how words look is `uoterm_view::art`. Each drawn
//! block of words becomes one texture, kept in a cache by its words and its
//! look, so a label that does not change is drawn once.

pub use crate::art::text::UoFonts;
pub use uoterm_view::art::{TextLook, UoFont};

use super::html::{parse_html, CharLook, HtmlChar, Rgba};
use eframe::egui::{self, ColorImage, TextureHandle, TextureId, TextureOptions, Vec2};
use std::collections::HashMap;
use std::hash::Hash;
use uoterm_nav::{TextAlign, TextPicture, UnicodeStyle, UNICODE_PICTURE_PADDING};

/// How many drawn blocks of words the cache keeps. A busy screen shows a
/// few hundred.
const TEXT_CACHE_CAPACITY: usize = 1024;
const TEXTURE_NAME: &str = "uoterm-classic-text";
const TEXTURE_OPTIONS: TextureOptions = TextureOptions::NEAREST;
const RGBA_BYTES: usize = 4;
/// Every line of HTML words is this tall, as in the classic client.
const HTML_LINE_HEIGHT: u32 = 18;
/// How far a line in a `<p>` starts from the left.
const HTML_INDENT: u32 = 14;
/// The Unicode font that gump HTML is written in.
pub const HTML_FONT: u8 = 1;
const SPACE: char = ' ';
const NEW_LINE: char = '\n';

/// How a block of gump HTML is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HtmlLook {
    /// The width the lines wrap to.
    pub width: u32,
    /// The color of words no tag colors.
    pub color: Rgba,
}

/// The pixels one char takes across, as the Unicode font draws it.
fn html_advance(fonts: &UoFonts, ch: &HtmlChar) -> u32 {
    let width = fonts
        .unicode
        .width(ch.look.font, ch.ch.encode_utf8(&mut [0; 4]));
    width + u32::from(ch.look.bold && ch.ch != SPACE)
}

/// Draws gump HTML, wrapped to the width of `look`.
pub fn render_html(fonts: &UoFonts, html: &str, look: &HtmlLook) -> Option<TextPicture> {
    let base = CharLook {
        font: HTML_FONT,
        color: look.color,
        bold: false,
        italic: false,
        underline: false,
        indent: false,
        align: TextAlign::Left,
    };
    let text = parse_html(html, base, &|font| fonts.unicode.has_font(font));
    let lines = html_lines(&text.chars, look.width, |ch| html_advance(fonts, ch));
    if lines.is_empty() {
        return None;
    }
    let width = (look.width + UNICODE_PICTURE_PADDING) as usize;
    let height = (lines.len() as u32 * HTML_LINE_HEIGHT + UNICODE_PICTURE_PADDING) as usize;
    let mut picture = TextPicture {
        width,
        height,
        rgba: vec![0; width * height * RGBA_BYTES],
    };
    if let Some(color) = text.background {
        for pixel in picture.rgba.chunks_exact_mut(RGBA_BYTES) {
            pixel.copy_from_slice(&color);
        }
    }
    for (row, line) in lines.iter().enumerate() {
        let chars = &text.chars[line.start..line.end];
        let mut x = line.left(look.width);
        let top = row * HTML_LINE_HEIGHT as usize;
        for run in chars.chunk_by(|a, b| a.look == b.look) {
            let words: String = run.iter().map(|c| c.ch).collect();
            let look = run[0].look;
            let style = UnicodeStyle {
                bold: look.bold,
                italic: look.italic,
                underline: look.underline,
                ..UnicodeStyle::default()
            };
            // The run is drawn as wide as its chars step, so bold chars,
            // one pixel wider each, are not cut at its end.
            let run_width: u32 = run.iter().map(|c| html_advance(fonts, c)).sum();
            let drawn = fonts.unicode.render(
                look.font,
                &words,
                Some(run_width),
                TextAlign::Left,
                style,
                look.color,
            );
            if let Some(drawn) = drawn {
                blit(&mut picture, &drawn, x as usize, top);
            }
            x += run_width;
        }
    }
    Some(picture)
}

/// Lays one picture over another at a place. Clear pixels of the top one
/// leave the bottom one as it was.
fn blit(onto: &mut TextPicture, top: &TextPicture, left: usize, upper: usize) {
    for y in 0..top.height {
        let to_y = upper + y;
        if to_y >= onto.height {
            break;
        }
        for x in 0..top.width {
            let to_x = left + x;
            if to_x >= onto.width {
                break;
            }
            let from = (y * top.width + x) * RGBA_BYTES;
            let pixel = &top.rgba[from..from + RGBA_BYTES];
            if pixel[RGBA_BYTES - 1] != 0 {
                let to = (to_y * onto.width + to_x) * RGBA_BYTES;
                onto.rgba[to..to + RGBA_BYTES].copy_from_slice(pixel);
            }
        }
    }
}

/// One line of HTML words: the chars it holds and how wide they are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlLine {
    pub start: usize,
    pub end: usize,
    pub width: u32,
    pub align: TextAlign,
    pub indent: u32,
}

impl HtmlLine {
    /// Where the line starts in a block of `width`.
    pub fn left(&self, width: u32) -> u32 {
        match self.align {
            TextAlign::Left => self.indent,
            TextAlign::Center => width.saturating_sub(self.width) / 2,
            TextAlign::Right => width.saturating_sub(self.width),
        }
    }
}

/// Breaks HTML chars into lines no wider than `width`, as the classic
/// client does: at the last space that fits, or where a word wider than the
/// whole line overflows. The space a line breaks at, and each new line
/// char, belong to no line. A line in a `<p>` that sits on the left starts
/// a little to the right.
pub fn html_lines(
    chars: &[HtmlChar],
    width: u32,
    advance: impl Fn(&HtmlChar) -> u32,
) -> Vec<HtmlLine> {
    let mut lines = Vec::new();
    let mut at = 0;
    while at < chars.len() {
        let first = chars[at].look;
        let indent = if first.indent && first.align == TextAlign::Left {
            HTML_INDENT
        } else {
            0
        };
        let room = width.saturating_sub(indent);
        let start = at;
        let mut used = 0;
        let mut last_space = None;
        let mut next = None;
        while at < chars.len() {
            let ch = &chars[at];
            if ch.ch == NEW_LINE {
                next = Some(at + 1);
                break;
            }
            let step = advance(ch);
            if at > start && used + step > room {
                next = Some(match (ch.ch == SPACE, last_space) {
                    (true, _) => at + 1,
                    (false, Some(space)) => {
                        at = space;
                        space + 1
                    }
                    (false, None) => at,
                });
                break;
            }
            if ch.ch == SPACE {
                last_space = Some(at);
            }
            used += step;
            at += 1;
        }
        let end = at;
        let width = chars[start..end].iter().map(&advance).sum();
        lines.push(HtmlLine {
            start,
            end,
            width,
            align: first.align,
            indent,
        });
        match next {
            Some(after) => {
                at = after;
                if at == chars.len() && chars[at - 1].ch == NEW_LINE {
                    lines.push(HtmlLine {
                        start: at,
                        end: at,
                        width: 0,
                        align: first.align,
                        indent: 0,
                    });
                }
            }
            None => break,
        }
    }
    lines
}

/// A cache that forgets the entry used least lately when it is full.
pub struct LruCache<K, V> {
    entries: HashMap<K, (V, u64)>,
    capacity: usize,
    clock: u64,
}

impl<K: Hash + Eq + Clone, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
            clock: 0,
        }
    }

    /// The value of `key`. `make` gives it the first time.
    pub fn get_or_make(&mut self, key: &K, make: impl FnOnce() -> V) -> &V {
        self.clock += 1;
        if !self.entries.contains_key(key) {
            if self.entries.len() >= self.capacity {
                self.forget_oldest();
            }
            self.entries.insert(key.clone(), (make(), self.clock));
        }
        let entry = self.entries.get_mut(key).expect("the entry was just made");
        entry.1 = self.clock;
        &entry.0
    }

    fn forget_oldest(&mut self) {
        let oldest = self
            .entries
            .iter()
            .min_by_key(|(_, (_, used))| *used)
            .map(|(key, _)| key.clone());
        if let Some(key) = oldest {
            self.entries.remove(&key);
        }
    }
}

/// What a drawn block of words is kept under.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TextKey {
    Plain(String, TextLook),
    Html(String, HtmlLook),
}

/// A drawn block of words, ready to paint.
#[derive(Clone)]
pub struct TextTexture {
    texture: TextureHandle,
    /// The size in pixels.
    pub size: Vec2,
}

impl TextTexture {
    pub fn id(&self) -> TextureId {
        self.texture.id()
    }
}

/// The fonts and the cache of drawn words.
pub struct TextKit {
    pub fonts: UoFonts,
    /// The client keeps its gump art in the UOP package, whose gumps some
    /// classic layouts place a little differently.
    pub uop_gumps: bool,
    cache: LruCache<TextKey, Option<TextTexture>>,
}

impl TextKit {
    pub fn new(fonts: UoFonts, uop_gumps: bool) -> Self {
        Self {
            fonts,
            uop_gumps,
            cache: LruCache::new(TEXT_CACHE_CAPACITY),
        }
    }

    /// The texture of a block of words. None when there is nothing to draw.
    pub fn label(
        &mut self,
        ctx: &egui::Context,
        text: &str,
        look: &TextLook,
    ) -> Option<TextTexture> {
        let fonts = &self.fonts;
        self.cache
            .get_or_make(&TextKey::Plain(text.to_string(), *look), || {
                upload(ctx, fonts.render(text, look)?)
            })
            .clone()
    }

    /// The texture of a block of gump HTML.
    pub fn html(
        &mut self,
        ctx: &egui::Context,
        html: &str,
        look: &HtmlLook,
    ) -> Option<TextTexture> {
        let fonts = &self.fonts;
        self.cache
            .get_or_make(&TextKey::Html(html.to_string(), *look), || {
                upload(ctx, render_html(fonts, html, look)?)
            })
            .clone()
    }
}

fn upload(ctx: &egui::Context, picture: TextPicture) -> Option<TextTexture> {
    if picture.width == 0 || picture.height == 0 {
        return None;
    }
    let image = ColorImage::from_rgba_unmultiplied([picture.width, picture.height], &picture.rgba);
    Some(TextTexture {
        texture: ctx.load_texture(TEXTURE_NAME, image, TEXTURE_OPTIONS),
        size: Vec2::new(picture.width as f32, picture.height as f32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAR_WIDTH: u32 = 5;

    fn chars(words: &str, look: CharLook) -> Vec<HtmlChar> {
        words.chars().map(|ch| HtmlChar { ch, look }).collect()
    }

    fn plain() -> CharLook {
        CharLook {
            font: HTML_FONT,
            color: [0, 0, 0, u8::MAX],
            bold: false,
            italic: false,
            underline: false,
            indent: false,
            align: TextAlign::Left,
        }
    }

    fn texts(chars: &[HtmlChar], lines: &[HtmlLine]) -> Vec<String> {
        lines
            .iter()
            .map(|l| chars[l.start..l.end].iter().map(|c| c.ch).collect())
            .collect()
    }

    #[test]
    fn html_lines_break_at_spaces_and_new_lines() {
        let words = chars("aaa bbb ccc\ndd", plain());
        let lines = html_lines(&words, 7 * CHAR_WIDTH, |_| CHAR_WIDTH);
        assert_eq!(texts(&words, &lines), ["aaa bbb", "ccc", "dd"]);
        assert_eq!(lines[0].width, 7 * CHAR_WIDTH);
        let long = chars("abcdefg", plain());
        let lines = html_lines(&long, 3 * CHAR_WIDTH, |_| CHAR_WIDTH);
        assert_eq!(texts(&long, &lines), ["abc", "def", "g"]);
        let ending = chars("a\n", plain());
        assert_eq!(
            texts(&ending, &html_lines(&ending, 50, |_| CHAR_WIDTH)),
            ["a", ""]
        );
    }

    #[test]
    fn a_paragraph_line_on_the_left_is_indented_and_others_are_aligned() {
        let para = chars(
            "ab",
            CharLook {
                indent: true,
                ..plain()
            },
        );
        let lines = html_lines(&para, 100, |_| CHAR_WIDTH);
        assert_eq!(lines[0].left(100), HTML_INDENT);
        let centered = chars(
            "ab",
            CharLook {
                align: TextAlign::Center,
                ..plain()
            },
        );
        let lines = html_lines(&centered, 100, |_| CHAR_WIDTH);
        assert_eq!(lines[0].left(100), (100 - 2 * CHAR_WIDTH) / 2);
    }

    #[test]
    fn the_cache_forgets_the_entry_used_least_lately() {
        let mut cache = LruCache::new(2);
        let mut made = 0;
        let mut get = |cache: &mut LruCache<u32, u32>, key: u32| {
            *cache.get_or_make(&key, || {
                made += 1;
                key * 10
            })
        };
        assert_eq!(get(&mut cache, 1), 10);
        assert_eq!(get(&mut cache, 2), 20);
        assert_eq!(get(&mut cache, 1), 10);
        assert_eq!(get(&mut cache, 3), 30);
        // Key 2 was used least lately, so it went, and 1 stayed.
        assert_eq!(get(&mut cache, 1), 10);
        assert_eq!(get(&mut cache, 2), 20);
        assert_eq!(made, 4);
    }

    #[test]
    fn real_fonts_draw_html() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        let fonts = UoFonts::open(&dir).unwrap();
        let look = HtmlLook {
            width: 120,
            color: [0, 0, 0, u8::MAX],
        };
        let html = render_html(
            &fonts,
            "<center><b>Runebook</b></center><br>Charges: 5",
            &look,
        )
        .unwrap();
        assert_eq!(
            html.height,
            (3 * HTML_LINE_HEIGHT + UNICODE_PICTURE_PADDING) as usize
        );
    }
}
