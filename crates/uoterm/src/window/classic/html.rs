//! The HTML of gumps, as the classic client reads it: a short list of tags
//! that set the font, the color, bold, italic, underline and the alignment
//! of words, and a few that break the line. The reader gives each char its
//! look; `text` lays the chars out in lines.
//!
//! The rules follow the reference client. A tag it does not know is dropped. `<body>`
//! drops the words before it. A `<b>`, `<i>`, `<u>` or `<p>` sets the line
//! to the left, as the classic client does. Links show, underlined, and are
//! never opened. The five common entities, such as `&nbsp;`, become their
//! chars.

use uoterm_nav::TextAlign;

/// A color in red, green, blue and alpha bytes.
pub type Rgba = [u8; 4];

const OPAQUE: u8 = u8::MAX;
const TAG_OPEN: char = '<';
const TAG_CLOSE: char = '>';
const END_MARK: char = '/';
const QUOTE: char = '"';
const HEX_MARK: char = '#';
const HEX_PREFIX: &str = "0x";
const HEX_RADIX: u32 = 16;
const NEW_LINE: char = '\n';
const ENTITIES: [(&str, char); 5] = [
    ("&nbsp;", ' '),
    ("&lt;", '<'),
    ("&gt;", '>'),
    ("&quot;", '"'),
    ("&amp;", '&'),
];
/// The Unicode fonts the size tags pick.
const FONT_BIG: u8 = 0;
const FONT_NORMAL: u8 = 1;
const FONT_SMALL: u8 = 2;
/// `<basefont size=big>` asks for this font; most clients have none, and
/// then the font stays as it was.
const FONT_SIZE_BIG_WORD: u8 = 4;
/// `<basefont size=N>`: these sizes keep the normal font, a smaller one
/// picks the small font, a larger one the big font.
const SIZE_NORMAL: [u8; 2] = [0, 4];
const SIZE_SMALL_BELOW: u8 = 4;
/// The color of a link, and of the words of a `<bq>`.
const LINK_COLOR: Rgba = [0x00, 0x00, 0xFF, OPAQUE];
const QUOTE_COLOR: Rgba = [0x00, 0x80, 0x00, OPAQUE];
const NAMED_COLORS: [(&str, Rgba); 19] = [
    ("red", [0xFF, 0x00, 0x00, OPAQUE]),
    ("cyan", [0x00, 0xFF, 0xFF, OPAQUE]),
    ("blue", [0x00, 0x00, 0xFF, OPAQUE]),
    ("darkblue", [0x00, 0x00, 0xA0, OPAQUE]),
    ("lightblue", [0xAD, 0xD8, 0xE6, OPAQUE]),
    ("purple", [0x80, 0x00, 0x80, OPAQUE]),
    ("yellow", [0xFF, 0xFF, 0x00, OPAQUE]),
    ("lime", [0x00, 0xFF, 0x00, OPAQUE]),
    ("magenta", [0xFF, 0x00, 0xFF, OPAQUE]),
    ("white", [0xFE, 0xFE, 0xFF, OPAQUE]),
    ("silver", [0xC0, 0xC0, 0xC0, OPAQUE]),
    ("grey", [0x80, 0x80, 0x80, OPAQUE]),
    ("gray", [0x80, 0x80, 0x80, OPAQUE]),
    ("black", [0x01, 0x01, 0x01, OPAQUE]),
    ("orange", [0xFF, 0xA5, 0x00, OPAQUE]),
    ("brown", [0xA5, 0x2A, 0x2A, OPAQUE]),
    ("maroon", [0x80, 0x00, 0x00, OPAQUE]),
    ("green", [0x00, 0x80, 0x00, OPAQUE]),
    ("olive", [0x80, 0x80, 0x00, OPAQUE]),
];

/// The tags the classic client knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tag {
    /// The words outside every tag.
    Base,
    Bold,
    Italic,
    Link,
    Underline,
    Paragraph,
    Big,
    Small,
    Body,
    BaseFont,
    Heading(u8),
    Break,
    Quote,
    Left,
    Center,
    Right,
    Div,
}

impl Tag {
    fn named(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        Some(match name.as_str() {
            "b" => Self::Bold,
            "i" => Self::Italic,
            "a" => Self::Link,
            "u" => Self::Underline,
            "p" => Self::Paragraph,
            "big" => Self::Big,
            "small" => Self::Small,
            "body" => Self::Body,
            "basefont" => Self::BaseFont,
            "br" => Self::Break,
            "bq" => Self::Quote,
            "left" => Self::Left,
            "center" => Self::Center,
            "right" => Self::Right,
            "div" => Self::Div,
            heading => {
                let level = heading.strip_prefix('h')?.parse().ok()?;
                (1..=6).contains(&level).then_some(Self::Heading(level))?
            }
        })
    }
}

/// How one run of chars looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharLook {
    /// The Unicode font.
    pub font: u8,
    pub color: Rgba,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// A line that starts in a `<p>` starts a little to the right.
    pub indent: bool,
    pub align: TextAlign,
}

/// One tag on the stack, with the look it asks for. `None` keeps what the
/// tags under it gave.
#[derive(Clone, Copy, Debug)]
struct Open {
    tag: Tag,
    font: Option<u8>,
    color: Option<Rgba>,
    bold: bool,
    italic: bool,
    underline: bool,
    indent: bool,
    align: TextAlign,
}

impl Open {
    fn of(tag: Tag) -> Self {
        let mut open = Self {
            tag,
            font: None,
            color: None,
            bold: false,
            italic: false,
            underline: false,
            indent: false,
            align: TextAlign::Left,
        };
        match tag {
            Tag::Bold => open.bold = true,
            Tag::Italic => open.italic = true,
            Tag::Underline => open.underline = true,
            Tag::Paragraph => open.indent = true,
            Tag::Big => open.font = Some(FONT_BIG),
            Tag::Small => open.font = Some(FONT_SMALL),
            Tag::Heading(1) => {
                open.bold = true;
                open.underline = true;
                open.font = Some(FONT_BIG);
            }
            Tag::Heading(2) => {
                open.bold = true;
                open.font = Some(FONT_BIG);
            }
            Tag::Heading(3) => open.font = Some(FONT_BIG),
            Tag::Heading(4) => {
                open.bold = true;
                open.font = Some(FONT_SMALL);
            }
            Tag::Heading(5) => {
                open.italic = true;
                open.font = Some(FONT_SMALL);
            }
            Tag::Heading(_) => open.font = Some(FONT_SMALL),
            Tag::Quote => open.color = Some(QUOTE_COLOR),
            Tag::Center => open.align = TextAlign::Center,
            Tag::Right => open.align = TextAlign::Right,
            _ => {}
        }
        open
    }
}

/// One char and its look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HtmlChar {
    pub ch: char,
    pub look: CharLook,
}

/// Words read from HTML.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HtmlText {
    pub chars: Vec<HtmlChar>,
    /// The color a `<body bgcolor>` asks for behind the words.
    pub background: Option<Rgba>,
}

/// A color as HTML writes it: `#RRGGBB`, a hex number, or a name. None for
/// words that are no color.
pub fn parse_color(words: &str) -> Option<Rgba> {
    if let Some(hex) = words.strip_prefix(HEX_MARK) {
        let hex = hex.strip_prefix(HEX_PREFIX).unwrap_or(hex);
        let value = u32::from_str_radix(hex, HEX_RADIX).ok()?;
        let [_, red, green, blue] = value.to_be_bytes();
        return Some([red, green, blue, OPAQUE]);
    }
    if words.starts_with(|c: char| c.is_ascii_digit()) {
        // A bare number is the color as the classic client keeps it:
        // blue, green, red and alpha from the high byte down.
        let value = u32::from_str_radix(words, HEX_RADIX).ok()?;
        let [blue, green, red, _] = value.to_be_bytes();
        return Some([red, green, blue, OPAQUE]);
    }
    NAMED_COLORS
        .iter()
        .find(|(name, _)| words.eq_ignore_ascii_case(name))
        .map(|(_, color)| *color)
}

/// The `name=value` pairs of a tag, with quotes taken off the values.
fn attributes(content: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut rest = content.trim_start();
    while !rest.is_empty() {
        let name_end = rest
            .find(|c: char| c.is_whitespace() || c == '=')
            .unwrap_or(rest.len());
        let name = rest[..name_end].to_ascii_lowercase();
        rest = rest[name_end..].trim_start();
        let mut value = String::new();
        if let Some(after) = rest.strip_prefix('=') {
            let after = after.trim_start();
            if let Some(quoted) = after.strip_prefix(QUOTE) {
                let end = quoted.find(QUOTE).unwrap_or(quoted.len());
                value = quoted[..end].to_string();
                rest = quoted.get(end + 1..).unwrap_or("");
            } else {
                let end = after
                    .find(|c: char| c.is_whitespace() || c == '=')
                    .unwrap_or(after.len());
                value = after[..end].to_string();
                rest = &after[end..];
            }
        }
        if name.is_empty() {
            break;
        }
        pairs.push((name, value));
        rest = rest.trim_start();
    }
    pairs
}

/// The font a `<basefont size=...>` asks for.
fn size_font(size: &str) -> u8 {
    match size.parse::<u8>() {
        Ok(number) if SIZE_NORMAL.contains(&number) => FONT_NORMAL,
        Ok(number) if number < SIZE_SMALL_BELOW => FONT_SMALL,
        Ok(_) => FONT_BIG,
        Err(_) if size.eq_ignore_ascii_case("big") => FONT_SIZE_BIG_WORD,
        Err(_) if size.eq_ignore_ascii_case("small") => FONT_BIG,
        Err(_) => FONT_NORMAL,
    }
}

/// The alignment an `align=` value asks for.
fn align_word(word: &str) -> Option<TextAlign> {
    match word.to_ascii_lowercase().as_str() {
        "left" => Some(TextAlign::Left),
        "center" => Some(TextAlign::Center),
        "right" => Some(TextAlign::Right),
        _ => None,
    }
}

/// Reads the attributes of a start tag into its look.
fn read_attributes(open: &mut Open, content: &str, text_color: &mut Option<Rgba>) -> Option<Rgba> {
    let mut background = None;
    for (name, value) in attributes(content) {
        match (open.tag, name.as_str()) {
            (Tag::Body, "text") => *text_color = parse_color(&value),
            (Tag::Body, "bgcolor") => background = parse_color(&value),
            (Tag::BaseFont, "color") => open.color = parse_color(&value),
            (Tag::BaseFont, "size") => open.font = Some(size_font(&value)),
            (Tag::Link, "href") => {
                open.underline = true;
                open.color = Some(LINK_COLOR);
            }
            (Tag::Paragraph | Tag::Div, "align") => {
                if let Some(align) = align_word(&value) {
                    open.align = align;
                }
            }
            _ => {}
        }
    }
    background
}

/// The look the stack of open tags gives, from the bottom up.
fn current_look(stack: &[Open], base: CharLook, has_font: &dyn Fn(u8) -> bool) -> CharLook {
    let mut look = base;
    let font_of = |open: &Open, look: &mut CharLook| {
        if let Some(font) = open.font.filter(|f| has_font(*f)) {
            look.font = font;
        }
    };
    for open in stack {
        match open.tag {
            Tag::Base | Tag::Body => {}
            Tag::Bold | Tag::Italic | Tag::Underline | Tag::Paragraph => {
                look.bold |= open.bold;
                look.italic |= open.italic;
                look.underline |= open.underline;
                look.indent |= open.indent;
                look.align = open.align;
            }
            Tag::Link => {
                look.underline |= open.underline;
                if let Some(color) = open.color {
                    look.color = color;
                }
            }
            Tag::Big | Tag::Small => font_of(open, &mut look),
            Tag::BaseFont => {
                font_of(open, &mut look);
                if let Some(color) = open.color {
                    look.color = color;
                }
            }
            Tag::Heading(_) => {
                look.bold |= open.bold;
                look.italic |= open.italic;
                look.underline |= open.underline;
                font_of(open, &mut look);
            }
            Tag::Quote => {
                if let Some(color) = open.color {
                    look.color = color;
                }
            }
            Tag::Left | Tag::Center | Tag::Right | Tag::Div => look.align = open.align,
            Tag::Break => {}
        }
    }
    look
}

/// The words with the five common entities turned into their chars.
fn decode_entities(words: &str) -> String {
    let mut out = String::with_capacity(words.len());
    let mut rest = words;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        match ENTITIES.iter().find(|(name, _)| {
            rest.get(..name.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(name))
        }) {
            Some((name, ch)) => {
                out.push(*ch);
                rest = &rest[name.len()..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Reads HTML words into chars with their looks. `base` is the look of the
/// words outside every tag; `has_font` says which Unicode fonts the client
/// has, so a tag that asks for a missing font changes nothing.
pub fn parse_html(html: &str, base: CharLook, has_font: &dyn Fn(u8) -> bool) -> HtmlText {
    let mut base = base;
    let mut out = HtmlText::default();
    let mut stack = vec![Open::of(Tag::Base)];
    let mut look = base;
    let mut rest = html;
    let push_words = |words: &str, look: CharLook, out: &mut HtmlText| {
        out.chars.extend(
            decode_entities(words)
                .chars()
                .filter(|ch| *ch != '\r')
                .map(|ch| HtmlChar { ch, look }),
        );
    };
    while let Some(at) = rest.find(TAG_OPEN) {
        push_words(&rest[..at], look, &mut out);
        let Some(close) = rest[at..].find(TAG_CLOSE) else {
            // A tag that never closes takes the rest of the words.
            return out;
        };
        let inside = rest[at + 1..at + close].trim_start();
        rest = &rest[at + close + 1..];
        let (is_end, inside) = match inside.strip_prefix(END_MARK) {
            Some(after) => (true, after.trim_start()),
            None => (false, inside),
        };
        let name_end = inside
            .find(|c: char| c.is_whitespace() || c == END_MARK)
            .unwrap_or(inside.len());
        let Some(tag) = Tag::named(&inside[..name_end]) else {
            continue;
        };
        // A mark right after the name, as in `<br/>`, closes the tag.
        let is_end = is_end || inside[name_end..].starts_with(END_MARK);
        let mut line_break = false;
        if is_end {
            if let Some(at) = stack
                .iter()
                .rposition(|open| open.tag == tag)
                .filter(|at| *at > 0)
            {
                stack.remove(at);
            }
        } else {
            let mut open = Open::of(tag);
            let mut text_color = None;
            let background = read_attributes(&mut open, &inside[name_end..], &mut text_color);
            if tag == Tag::Body {
                out.chars.clear();
                stack.truncate(1);
                if let Some(color) = text_color {
                    base.color = color;
                }
                if background.is_some() {
                    out.background = background;
                }
            } else {
                stack.push(open);
            }
        }
        look = current_look(&stack, base, has_font);
        match tag {
            Tag::Left | Tag::Center | Tag::Right | Tag::Paragraph => {
                let starts_later = !is_end && !out.chars.is_empty() && tag != Tag::Paragraph;
                line_break = is_end || starts_later;
            }
            Tag::Break | Tag::Quote => line_break = true,
            _ => {}
        }
        if line_break {
            out.chars.push(HtmlChar { ch: NEW_LINE, look });
        }
    }
    push_words(rest, look, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Rgba = [0xFF, 0xFF, 0xFF, OPAQUE];

    fn base() -> CharLook {
        CharLook {
            font: FONT_NORMAL,
            color: WHITE,
            bold: false,
            italic: false,
            underline: false,
            indent: false,
            align: TextAlign::Left,
        }
    }

    fn words(text: &HtmlText) -> String {
        text.chars.iter().map(|c| c.ch).collect()
    }

    fn every_font(_: u8) -> bool {
        true
    }

    #[test]
    fn plain_words_keep_the_base_look() {
        let text = parse_html("Hello there", base(), &every_font);
        assert_eq!(words(&text), "Hello there");
        assert!(text.chars.iter().all(|c| c.look == base()));
    }

    #[test]
    fn bold_italic_and_underline_nest_and_close() {
        let text = parse_html("<b>a<i>b</i></b><u>c</u>d", base(), &every_font);
        assert_eq!(words(&text), "abcd");
        let looks: Vec<(bool, bool, bool)> = text
            .chars
            .iter()
            .map(|c| (c.look.bold, c.look.italic, c.look.underline))
            .collect();
        assert_eq!(
            looks,
            vec![
                (true, false, false),
                (true, true, false),
                (false, false, true),
                (false, false, false)
            ]
        );
    }

    #[test]
    fn basefont_sets_the_color_and_a_size_picks_a_font() {
        let text = parse_html(
            "<BASEFONT COLOR=#FF0000 size=2>r</basefont><basefont color=\"lime\">g",
            base(),
            &every_font,
        );
        assert_eq!(text.chars[0].look.color, [0xFF, 0, 0, OPAQUE]);
        assert_eq!(text.chars[0].look.font, FONT_SMALL);
        assert_eq!(text.chars[1].look.color, [0, 0xFF, 0, OPAQUE]);
        assert_eq!(text.chars[1].look.font, FONT_NORMAL);
    }

    #[test]
    fn a_missing_font_keeps_the_font_it_had() {
        let text = parse_html("<big>x</big>", base(), &|font| font != FONT_BIG);
        assert_eq!(text.chars[0].look.font, FONT_NORMAL);
    }

    #[test]
    fn breaks_centers_and_paragraphs_make_lines() {
        let text = parse_html("a<br>b<center>c</center>d<p>e</p>", base(), &every_font);
        assert_eq!(words(&text), "a\nb\nc\nde\n");
        let center = text.chars.iter().find(|c| c.ch == 'c').unwrap();
        assert_eq!(center.look.align, TextAlign::Center);
        let para = text.chars.iter().find(|c| c.ch == 'e').unwrap();
        assert!(para.look.indent);
        assert_eq!(parse_html("<br/>x", base(), &every_font).chars[0].ch, '\n');
    }

    #[test]
    fn bold_inside_center_is_set_left_as_the_classic_client_does() {
        let text = parse_html("<center><b>Title</b></center>", base(), &every_font);
        assert_eq!(text.chars[0].look.align, TextAlign::Left);
        assert!(text.chars[0].look.bold);
    }

    #[test]
    fn links_show_underlined_in_the_link_color() {
        let text = parse_html("<a href=\"http://x\">site</a>", base(), &every_font);
        assert_eq!(words(&text), "site");
        assert!(text.chars.iter().all(|c| c.look.underline));
        assert_eq!(text.chars[0].look.color, LINK_COLOR);
    }

    #[test]
    fn body_drops_the_words_before_it_and_sets_colors() {
        let text = parse_html(
            "lost<body text=#00FF00 bgcolor=black>kept",
            base(),
            &every_font,
        );
        assert_eq!(words(&text), "kept");
        assert_eq!(text.chars[0].look.color, [0, 0xFF, 0, OPAQUE]);
        assert_eq!(text.background, Some([1, 1, 1, OPAQUE]));
    }

    #[test]
    fn unknown_tags_go_and_entities_become_chars() {
        let text = parse_html("<font x>a&nbsp;&lt;b&gt;&amp;</font>", base(), &every_font);
        assert_eq!(words(&text), "a <b>&");
        assert_eq!(words(&parse_html("cut <b", base(), &every_font)), "cut ");
    }

    #[test]
    fn headings_and_quotes_follow_the_classic_looks() {
        let text = parse_html("<h1>a</h1><bq>b</bq>", base(), &every_font);
        let heading = text.chars[0].look;
        assert!(heading.bold && heading.underline && heading.font == FONT_BIG);
        let quote = text.chars.iter().find(|c| c.ch == 'b').unwrap();
        assert_eq!(quote.look.color, QUOTE_COLOR);
    }

    #[test]
    fn colors_read_hex_numbers_and_names() {
        assert_eq!(parse_color("#0x102030"), Some([0x10, 0x20, 0x30, OPAQUE]));
        assert_eq!(parse_color("Orange"), Some([0xFF, 0xA5, 0x00, OPAQUE]));
        assert_eq!(parse_color("0000FFFF"), Some([0xFF, 0x00, 0x00, OPAQUE]));
        assert_eq!(parse_color("nothing"), None);
    }
}
