//! What the two kinds of UO font share: breaking words into lines that fit a
//! width, cutting a label short with dots, and the picture of the words.
//!
//! The rules follow the classic client. A line breaks at the last space that
//! fits, and a word wider than the whole width breaks where it overflows.
//! The space a line breaks at is dropped. A new line char always ends a line.

/// Where each line of a block of words sits across the width.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// One line of words after wrapping.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextLine {
    pub text: String,
    /// The width of the line in pixels.
    pub width: u32,
    /// The height the line takes in the block, in pixels.
    pub height: u32,
}

/// Words drawn in RGBA bytes, row order. A clear pixel is all zero.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextPicture {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// How far one char moves the pen and how tall it stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CharMetric {
    pub advance: u32,
    pub height: u32,
}

/// What a label that is cut short ends with.
pub(crate) const ELLIPSIS: &str = "...";
const NEW_LINE: char = '\n';
const CARRIAGE_RETURN: char = '\r';
const SPACE: char = ' ';

#[derive(Default)]
struct LineBuilder {
    chars: Vec<(char, CharMetric)>,
    /// The place in `chars` of the last space of the line.
    last_space: Option<usize>,
}

impl LineBuilder {
    fn width(&self) -> u32 {
        self.chars.iter().map(|(_, m)| m.advance).sum()
    }

    fn finish(self, empty_height: u32) -> TextLine {
        let height = self.chars.iter().map(|(_, m)| m.height).max();
        TextLine {
            text: self.chars.iter().map(|(c, _)| *c).collect(),
            width: self.width(),
            height: height.filter(|h| *h > 0).unwrap_or(empty_height),
        }
    }
}

/// Breaks `text` into lines no wider than `max_width`, or into its own lines
/// only when no width is given. `measure` gives None for a char the font
/// skips. A line with nothing on it takes `empty_height`.
pub(crate) fn wrap(
    text: &str,
    max_width: Option<u32>,
    empty_height: u32,
    measure: impl Fn(char) -> Option<CharMetric>,
) -> Vec<TextLine> {
    let chars: Vec<char> = text.chars().filter(|c| *c != CARRIAGE_RETURN).collect();
    let mut lines = Vec::new();
    let mut line = LineBuilder::default();
    let mut at = 0;
    while at < chars.len() {
        let ch = chars[at];
        if ch == NEW_LINE {
            lines.push(std::mem::take(&mut line).finish(empty_height));
            at += 1;
            continue;
        }
        let Some(metric) = measure(ch) else {
            at += 1;
            continue;
        };
        let overflows = max_width
            .is_some_and(|max| !line.chars.is_empty() && line.width() + metric.advance > max);
        if overflows {
            if ch == SPACE {
                // The space the line ends at is dropped.
                at += 1;
            } else if let Some(space) = line.last_space {
                // The word after the last space starts the next line.
                at -= line.chars.len() - space - 1;
                line.chars.truncate(space);
            }
            lines.push(std::mem::take(&mut line).finish(empty_height));
            continue;
        }
        if ch == SPACE {
            line.last_space = Some(line.chars.len());
        }
        line.chars.push((ch, metric));
        at += 1;
    }
    if !line.chars.is_empty() || chars.last() == Some(&NEW_LINE) {
        lines.push(line.finish(empty_height));
    }
    lines
}

/// The widest line of `text`, with no wrapping.
pub(crate) fn widest_line(text: &str, measure: impl Fn(char) -> Option<CharMetric>) -> u32 {
    text.split(NEW_LINE)
        .map(|line| {
            line.chars()
                .filter_map(&measure)
                .map(|m| m.advance)
                .sum::<u32>()
        })
        .max()
        .unwrap_or(0)
}

/// `text` as it fits in `max_width` on one line: whole when it fits, and cut
/// short with dots when it does not. `ellipsis_width` is the room the dots
/// take. A char the font skips is left out.
pub(crate) fn crop(
    text: &str,
    max_width: u32,
    ellipsis_width: u32,
    measure: impl Fn(char) -> Option<CharMetric>,
) -> String {
    if widest_line(text, &measure) <= max_width && !text.contains(NEW_LINE) {
        return text.to_string();
    }
    let room = max_width.saturating_sub(ellipsis_width);
    let mut used = 0;
    let mut out = String::new();
    for ch in text.chars().take_while(|c| *c != NEW_LINE) {
        let Some(metric) = measure(ch) else {
            continue;
        };
        used += metric.advance;
        if used > room {
            break;
        }
        out.push(ch);
    }
    out.push_str(ELLIPSIS);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAR_WIDTH: u32 = 5;
    const CHAR_HEIGHT: u32 = 10;
    const EMPTY_HEIGHT: u32 = 14;

    /// Every char is five wide, and a `~` is one the font skips.
    fn measure(ch: char) -> Option<CharMetric> {
        (ch != '~').then_some(CharMetric {
            advance: CHAR_WIDTH,
            height: CHAR_HEIGHT,
        })
    }

    fn texts(lines: &[TextLine]) -> Vec<&str> {
        lines.iter().map(|l| l.text.as_str()).collect()
    }

    #[test]
    fn lines_break_at_the_last_space_that_fits() {
        let lines = wrap("aaa bbb ccc", Some(7 * CHAR_WIDTH), EMPTY_HEIGHT, measure);
        assert_eq!(texts(&lines), ["aaa bbb", "ccc"]);
        assert_eq!(lines[0].width, 7 * CHAR_WIDTH);
        assert_eq!(lines[1].height, CHAR_HEIGHT);
    }

    #[test]
    fn a_word_wider_than_the_width_breaks_where_it_overflows() {
        let lines = wrap("abcdefg", Some(3 * CHAR_WIDTH), EMPTY_HEIGHT, measure);
        assert_eq!(texts(&lines), ["abc", "def", "g"]);
    }

    #[test]
    fn a_space_that_overflows_ends_the_line_and_is_dropped() {
        let lines = wrap("abc def", Some(3 * CHAR_WIDTH), EMPTY_HEIGHT, measure);
        assert_eq!(texts(&lines), ["abc", "def"]);
    }

    #[test]
    fn new_lines_end_lines_and_an_empty_line_takes_the_empty_height() {
        let lines = wrap("ab\n\ncd\n", None, EMPTY_HEIGHT, measure);
        assert_eq!(texts(&lines), ["ab", "", "cd", ""]);
        assert_eq!(lines[1].height, EMPTY_HEIGHT);
        assert_eq!(lines[1].width, 0);
        assert!(wrap("", None, EMPTY_HEIGHT, measure).is_empty());
    }

    #[test]
    fn skipped_chars_and_carriage_returns_take_no_room() {
        let lines = wrap("a~b\r", None, EMPTY_HEIGHT, measure);
        assert_eq!(texts(&lines), ["ab"]);
        assert_eq!(widest_line("a~b\nabcd", measure), 4 * CHAR_WIDTH);
    }

    #[test]
    fn a_label_that_does_not_fit_is_cut_short_with_dots() {
        let dots = 3 * CHAR_WIDTH;
        assert_eq!(crop("abcd", 4 * CHAR_WIDTH, dots, measure), "abcd");
        assert_eq!(crop("abcdefgh", 6 * CHAR_WIDTH, dots, measure), "abc...");
    }
}
