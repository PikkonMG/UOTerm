//! The words of a text box and how keys change them, with no drawing: the
//! caret, the selection, a most number of chars, fields for numbers only,
//! password fields and fields of many lines. It works as the text box of
//! the reference client does.

use std::ops::Range;

const PASSWORD_MARK: char = '*';
const NEW_LINE: char = '\n';

/// One key or edit the player made in a text box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldKey {
    /// Typed chars, or chars pasted from the clipboard.
    Insert(String),
    Backspace,
    Delete,
    /// The arrows. `word` jumps a whole word; `select` keeps the start of
    /// the selection where it was.
    Left {
        select: bool,
        word: bool,
    },
    Right {
        select: bool,
        word: bool,
    },
    /// Up and down go to the line above or below, in a field of many
    /// lines.
    Up {
        select: bool,
    },
    Down {
        select: bool,
    },
    Home {
        select: bool,
    },
    End {
        select: bool,
    },
    SelectAll,
    Copy,
    Cut,
    Enter,
}

/// What one key did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FieldOutcome {
    pub changed: bool,
    /// Enter in a field of one line.
    pub submitted: bool,
    /// Words for the clipboard, from a copy or a cut.
    pub copied: Option<String>,
}

/// The words of a text box.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextField {
    text: String,
    /// The caret and the other end of the selection, in chars.
    caret: usize,
    anchor: usize,
    pub max_chars: Option<usize>,
    /// Only digits go in.
    pub numeric: bool,
    /// The words show as stars.
    pub password: bool,
    /// Enter makes a new line instead of sending.
    pub multiline: bool,
}

impl TextField {
    /// A field with these words and the caret at their end.
    pub fn new(text: &str) -> Self {
        let mut field = Self::default();
        field.set_text(text);
        field
    }

    pub fn with_max_chars(mut self, max_chars: Option<usize>) -> Self {
        self.max_chars = max_chars;
        self.set_text(&self.text.clone());
        self
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// Puts new words in, cut to the most chars, with the caret at the end.
    pub fn set_text(&mut self, text: &str) {
        self.text = self.allowed(text, usize::MAX);
        self.caret = self.len();
        self.anchor = self.caret;
    }

    /// The words as the box shows them.
    pub fn shown(&self) -> String {
        if self.password {
            std::iter::repeat_n(PASSWORD_MARK, self.len()).collect()
        } else {
            self.text.clone()
        }
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Puts the caret before the char at `index`, as a click does.
    pub fn place_caret(&mut self, index: usize, select: bool) {
        self.caret = index.min(self.len());
        if !select {
            self.anchor = self.caret;
        }
    }

    /// The chars that are selected, when some are.
    pub fn selection(&self) -> Option<Range<usize>> {
        let range = self.caret.min(self.anchor)..self.caret.max(self.anchor);
        (!range.is_empty()).then_some(range)
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    fn byte_at(&self, index: usize) -> usize {
        self.text
            .char_indices()
            .nth(index)
            .map_or(self.text.len(), |(at, _)| at)
    }

    fn selected_text(&self) -> Option<String> {
        let range = self.selection()?;
        Some(self.text[self.byte_at(range.start)..self.byte_at(range.end)].to_string())
    }

    /// The part of `words` this field takes, when `room` chars are left.
    fn allowed(&self, words: &str, room: usize) -> String {
        let room = self.max_chars.map_or(room, |max| room.min(max));
        words
            .chars()
            .filter(|ch| !ch.is_control() || (self.multiline && *ch == NEW_LINE))
            .filter(|ch| !self.numeric || ch.is_ascii_digit())
            .take(room)
            .collect()
    }

    /// Takes the selected chars out. True when there were some.
    fn remove_selection(&mut self) -> bool {
        let Some(range) = self.selection() else {
            return false;
        };
        let (start, end) = (self.byte_at(range.start), self.byte_at(range.end));
        self.text.replace_range(start..end, "");
        self.caret = range.start;
        self.anchor = range.start;
        true
    }

    /// Where a jump of one word from `from` lands.
    fn word_end(&self, from: usize, forward: bool) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut at = from;
        if forward {
            while at < chars.len() && chars[at].is_whitespace() {
                at += 1;
            }
            while at < chars.len() && !chars[at].is_whitespace() {
                at += 1;
            }
        } else {
            while at > 0 && chars[at - 1].is_whitespace() {
                at -= 1;
            }
            while at > 0 && !chars[at - 1].is_whitespace() {
                at -= 1;
            }
        }
        at
    }

    /// Where the caret lands one line up or down, at the same column or at
    /// the end of a shorter line.
    fn line_step(&self, down: bool) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let line_start = |at: usize| {
            chars[..at]
                .iter()
                .rposition(|ch| *ch == NEW_LINE)
                .map_or(0, |at| at + 1)
        };
        let line_end = |at: usize| {
            chars[at..]
                .iter()
                .position(|ch| *ch == NEW_LINE)
                .map_or(chars.len(), |len| at + len)
        };
        let start = line_start(self.caret);
        let column = self.caret - start;
        if down {
            let end = line_end(self.caret);
            if end == chars.len() {
                return self.caret;
            }
            let next = end + 1;
            (next + column).min(line_end(next))
        } else {
            if start == 0 {
                return self.caret;
            }
            let above = line_start(start - 1);
            (above + column).min(start - 1)
        }
    }

    fn move_caret(&mut self, to: usize, select: bool) {
        self.caret = to.min(self.len());
        if !select {
            self.anchor = self.caret;
        }
    }

    /// Applies one key.
    pub fn apply(&mut self, key: FieldKey) -> FieldOutcome {
        let mut outcome = FieldOutcome::default();
        match key {
            FieldKey::Insert(words) => {
                let removed = self.remove_selection();
                let room = self
                    .max_chars
                    .map_or(usize::MAX, |max| max - self.len().min(max));
                let words = self.allowed(&words, room);
                let at = self.byte_at(self.caret);
                self.text.insert_str(at, &words);
                self.move_caret(self.caret + words.chars().count(), false);
                outcome.changed = removed || !words.is_empty();
            }
            FieldKey::Backspace => {
                outcome.changed = self.remove_selection();
                if !outcome.changed && self.caret > 0 {
                    let (start, end) = (self.byte_at(self.caret - 1), self.byte_at(self.caret));
                    self.text.replace_range(start..end, "");
                    self.move_caret(self.caret - 1, false);
                    outcome.changed = true;
                }
            }
            FieldKey::Delete => {
                outcome.changed = self.remove_selection();
                if !outcome.changed && self.caret < self.len() {
                    let (start, end) = (self.byte_at(self.caret), self.byte_at(self.caret + 1));
                    self.text.replace_range(start..end, "");
                    outcome.changed = true;
                }
            }
            FieldKey::Left { select, word } => {
                let to = match (self.selection(), select, word) {
                    (Some(range), false, false) => range.start,
                    (_, _, true) => self.word_end(self.caret, false),
                    _ => self.caret.saturating_sub(1),
                };
                self.move_caret(to, select);
            }
            FieldKey::Right { select, word } => {
                let to = match (self.selection(), select, word) {
                    (Some(range), false, false) => range.end,
                    (_, _, true) => self.word_end(self.caret, true),
                    _ => self.caret + 1,
                };
                self.move_caret(to, select);
            }
            FieldKey::Up { select } => self.move_caret(self.line_step(false), select),
            FieldKey::Down { select } => self.move_caret(self.line_step(true), select),
            FieldKey::Home { select } => self.move_caret(0, select),
            FieldKey::End { select } => self.move_caret(self.len(), select),
            FieldKey::SelectAll => {
                self.anchor = 0;
                self.caret = self.len();
            }
            FieldKey::Copy => outcome.copied = self.selected_text().filter(|_| !self.password),
            FieldKey::Cut => {
                if !self.password {
                    outcome.copied = self.selected_text();
                    outcome.changed = self.remove_selection();
                }
            }
            FieldKey::Enter if self.multiline => {
                return self.apply(FieldKey::Insert(NEW_LINE.to_string()));
            }
            FieldKey::Enter => outcome.submitted = true,
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(field: &mut TextField, words: &str) -> FieldOutcome {
        field.apply(FieldKey::Insert(words.to_string()))
    }

    #[test]
    fn typing_goes_in_at_the_caret_and_backspace_takes_the_char_before() {
        let mut field = TextField::new("held");
        field.apply(FieldKey::Left {
            select: false,
            word: false,
        });
        assert!(typed(&mut field, "o").changed);
        assert_eq!(field.text(), "helod");
        field.apply(FieldKey::Backspace);
        field.apply(FieldKey::Delete);
        assert_eq!(field.text(), "hel");
        assert_eq!(field.caret(), 3);
        field.apply(FieldKey::Home { select: false });
        assert!(!field.apply(FieldKey::Backspace).changed);
    }

    #[test]
    fn a_selection_is_replaced_copied_and_cut() {
        let mut field = TextField::new("hello world");
        field.apply(FieldKey::Left {
            select: true,
            word: true,
        });
        assert_eq!(field.selection(), Some(6..11));
        let copied = field.apply(FieldKey::Copy).copied;
        assert_eq!(copied.as_deref(), Some("world"));
        typed(&mut field, "there");
        assert_eq!(field.text(), "hello there");
        field.apply(FieldKey::SelectAll);
        let cut = field.apply(FieldKey::Cut);
        assert_eq!(cut.copied.as_deref(), Some("hello there"));
        assert_eq!(field.text(), "");
    }

    #[test]
    fn the_most_chars_numbers_only_and_passwords_hold() {
        let mut field = TextField::new("").with_max_chars(Some(4));
        typed(&mut field, "abcdef");
        assert_eq!(field.text(), "abcd");
        let mut number = TextField {
            numeric: true,
            ..TextField::default()
        };
        typed(&mut number, "1a2b3");
        assert_eq!(number.text(), "123");
        let mut secret = TextField {
            password: true,
            ..TextField::new("pass")
        };
        assert_eq!(secret.shown(), "****");
        secret.apply(FieldKey::SelectAll);
        assert_eq!(secret.apply(FieldKey::Copy).copied, None);
        assert_eq!(
            TextField::new("toolong").with_max_chars(Some(3)).text(),
            "too"
        );
    }

    #[test]
    fn enter_sends_one_line_and_breaks_many_lines() {
        let mut line = TextField::new("go");
        assert!(line.apply(FieldKey::Enter).submitted);
        assert_eq!(line.text(), "go");
        typed(&mut line, "a\nb");
        assert_eq!(line.text(), "goab");
        let mut lines = TextField {
            multiline: true,
            ..TextField::new("a")
        };
        assert!(!lines.apply(FieldKey::Enter).submitted);
        typed(&mut lines, "bcd");
        assert_eq!(lines.text(), "a\nbcd");
        lines.apply(FieldKey::Up { select: false });
        assert_eq!(lines.caret(), 1);
        lines.apply(FieldKey::Home { select: false });
        lines.apply(FieldKey::Down { select: false });
        assert_eq!(lines.caret(), 2);
    }

    #[test]
    fn arrows_collapse_a_selection_and_words_jump() {
        let mut field = TextField::new("one two three");
        field.apply(FieldKey::Home { select: false });
        field.apply(FieldKey::Right {
            select: false,
            word: true,
        });
        assert_eq!(field.caret(), 3);
        field.apply(FieldKey::End { select: true });
        field.apply(FieldKey::Left {
            select: false,
            word: false,
        });
        assert_eq!(field.caret(), 3);
        assert_eq!(field.selection(), None);
        field.place_caret(99, false);
        assert_eq!(field.caret(), 13);
    }
}
