//! Words in the fonts of the client: the ASCII fonts of `fonts.mul` and the
//! Unicode fonts of `unifont*.mul`, in a hue or a color, drawn as RGBA.

use std::path::Path;
use uoterm_nav::{AsciiFonts, ClilocData, HueData, TextBlock, TextPicture, UnicodeFonts};
use uoterm_view::art::{TextLook, UoFont};

/// Words as tall as the tallest line of a font.
const LINE_MEASURE: &str = "Ay";

/// The fonts, the hues and the text numbers of the client.
pub struct UoFonts {
    pub ascii: AsciiFonts,
    pub unicode: UnicodeFonts,
    pub hues: HueData,
    /// None when the client files hold no text numbers.
    cliloc: Option<ClilocData>,
}

impl UoFonts {
    pub fn open(uopath: &Path) -> Result<Self, String> {
        Ok(Self {
            ascii: AsciiFonts::open(uopath).map_err(|e| e.to_string())?,
            unicode: UnicodeFonts::open(uopath).map_err(|e| e.to_string())?,
            hues: HueData::open(uopath).map_err(|e| e.to_string())?,
            cliloc: ClilocData::open(uopath).ok(),
        })
    }

    /// The sentence of a text number of the client, or `fallback` when the
    /// files do not hold it.
    pub fn words(&self, number: u32, fallback: &str) -> String {
        self.cliloc
            .as_ref()
            .and_then(|cliloc| cliloc.text(number))
            .unwrap_or(fallback)
            .to_string()
    }

    /// The width of one line of words in a font, in pixels.
    pub fn width(&self, font: UoFont, text: &str) -> u32 {
        match font {
            UoFont::Ascii(font) => self.ascii.width(font, text),
            UoFont::Unicode(font) => self.unicode.width(font, text),
        }
    }

    /// The height of one line of words in a font, in pixels.
    pub fn line_height(&self, look: &TextLook) -> u32 {
        match look.font {
            UoFont::Ascii(font) => self.ascii.height(font, LINE_MEASURE, None),
            UoFont::Unicode(font) => self.unicode.height(font, LINE_MEASURE, None, look.style),
        }
    }

    /// The color of one step of a hue ramp, as a solid box in that hue
    /// shows it.
    pub fn hue_rgb(&self, hue: u16, step: usize) -> Option<[u8; 3]> {
        self.hues.step_rgb(hue, step)
    }

    /// The lines a block of words breaks into, as [`UoFonts::render`]
    /// draws them.
    pub fn lines(&self, text: &str, look: &TextLook) -> Vec<String> {
        let shown = self.shown(text, look);
        let laid = match look.font {
            UoFont::Ascii(font) => self.ascii.layout(font, &shown, look.width),
            UoFont::Unicode(font) => self.unicode.layout(font, &shown, look.width, look.style),
        };
        laid.into_iter().map(|line| line.text).collect()
    }

    /// The lines of a block of words and the size of their picture before
    /// the margin, as [`UoFonts::render`] makes it.
    pub fn measure(&self, text: &str, look: &TextLook) -> TextBlock {
        let shown = self.shown(text, look);
        match look.font {
            UoFont::Ascii(font) => self.ascii.measure(font, &shown, look.width),
            UoFont::Unicode(font) => self.unicode.measure(font, &shown, look.width, look.style),
        }
    }

    /// Draws a block of words.
    pub fn render(&self, text: &str, look: &TextLook) -> Option<TextPicture> {
        let shown = self.shown(text, look);
        match look.font {
            UoFont::Ascii(font) => self
                .ascii
                .render_rgba(font, &shown, look.width, look.align, look.hue, &self.hues),
            UoFont::Unicode(font) => self.unicode.render_hued(
                font, &shown, look.width, look.align, look.style, look.hue, &self.hues,
            ),
        }
    }

    /// The words a look shows: cut short with dots when it crops.
    fn shown(&self, text: &str, look: &TextLook) -> String {
        match look.width.filter(|_| look.crop) {
            Some(width) => match look.font {
                UoFont::Ascii(font) => self.ascii.crop(font, text, width),
                UoFont::Unicode(font) => self.unicode.crop(font, text, width),
            },
            None => text.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_nav::UNICODE_PICTURE_PADDING;

    #[test]
    fn real_fonts_draw_labels_and_break_lines() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        let fonts = UoFonts::open(&dir).unwrap();
        let label = fonts
            .render("Strength", &TextLook::ascii(1, 0x0386))
            .unwrap();
        assert!(label.width > 0 && label.height > 0);
        let cut = fonts
            .render(
                "A very long name",
                &TextLook::unicode(1, 0xFFFF).cropped(40),
            )
            .unwrap();
        assert!(cut.width <= 40 + UNICODE_PICTURE_PADDING as usize);
        let narrow = TextLook::unicode(1, 0xFFFF).wrap(60);
        assert!(fonts.lines("Hail and well met, traveller", &narrow).len() > 1);
        assert_eq!(fonts.lines("Hail", &narrow), ["Hail"]);
    }
}
