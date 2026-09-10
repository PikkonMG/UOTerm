//! Splits one script line into words, quoted text and compare signs.

use crate::error::ParseError;

/// A compare sign between two values in a condition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compare {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

impl Compare {
    /// The sign as a script writes it.
    pub fn sign(self) -> &'static str {
        match self {
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessOrEqual => "<=",
            Self::Greater => ">",
            Self::GreaterOrEqual => ">=",
        }
    }

    /// True when `left sign right` holds.
    pub fn holds<T: PartialOrd>(self, left: T, right: T) -> bool {
        match self {
            Self::Equal => left == right,
            Self::NotEqual => left != right,
            Self::Less => left < right,
            Self::LessOrEqual => left <= right,
            Self::Greater => left > right,
            Self::GreaterOrEqual => left >= right,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// A word with no quotes: a command, a keyword, a number or a name.
    Word(String),
    /// Text in single or double quotes, the quotes taken off.
    Quoted(String),
    Compare(Compare),
}

/// A line that holds only a note.
pub const COMMENT: &str = "//";

const SINGLE_QUOTE: char = '\'';
const DOUBLE_QUOTE: char = '"';

/// The tokens of one line. A line that is empty or only a note gives none.
pub fn tokenize(line: &str, line_no: usize) -> Result<Vec<Token>, ParseError> {
    let text = line.trim();
    if text.is_empty() || text.starts_with(COMMENT) {
        return Ok(Vec::new());
    }
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();
    while let Some(&(start, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
            continue;
        }
        if c == SINGLE_QUOTE || c == DOUBLE_QUOTE {
            chars.next();
            let mut quoted = String::new();
            let mut closed = false;
            for (_, q) in chars.by_ref() {
                if q == c {
                    closed = true;
                    break;
                }
                quoted.push(q);
            }
            if !closed {
                return Err(ParseError::new(line_no, "a quote is not closed"));
            }
            tokens.push(Token::Quoted(quoted));
            continue;
        }
        if let Some((compare, len)) = compare_at(&text[start..]) {
            tokens.push(Token::Compare(compare));
            for _ in 0..len {
                chars.next();
            }
            continue;
        }
        let mut word = String::new();
        while let Some(&(at, w)) = chars.peek() {
            // A `!` that is not part of `!=` stays in the word: it is the
            // force mark of `target!`.
            if w.is_whitespace()
                || w == SINGLE_QUOTE
                || w == DOUBLE_QUOTE
                || compare_at(&text[at..]).is_some()
            {
                break;
            }
            word.push(w);
            chars.next();
        }
        tokens.push(Token::Word(word));
    }
    Ok(tokens)
}

/// The compare sign that opens `text`, and how many characters it takes.
fn compare_at(text: &str) -> Option<(Compare, usize)> {
    const TWO: [(&str, Compare); 4] = [
        ("==", Compare::Equal),
        ("!=", Compare::NotEqual),
        ("<=", Compare::LessOrEqual),
        (">=", Compare::GreaterOrEqual),
    ];
    for (sign, compare) in TWO {
        if text.starts_with(sign) {
            return Some((compare, sign.len()));
        }
    }
    match text.chars().next()? {
        '<' => Some((Compare::Less, 1)),
        '>' => Some((Compare::Greater, 1)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &str) -> Vec<Token> {
        tokenize(line, 1).expect("a line")
    }

    fn w(s: &str) -> Token {
        Token::Word(s.into())
    }

    #[test]
    fn words_quotes_and_numbers_are_split_on_spaces() {
        assert_eq!(
            words("usetype! 0xe21 'any' \"ground\" 2"),
            vec![
                w("usetype!"),
                w("0xe21"),
                Token::Quoted("any".into()),
                Token::Quoted("ground".into()),
                w("2")
            ]
        );
    }

    #[test]
    fn quoted_text_keeps_its_spaces_and_other_quote_marks() {
        assert_eq!(
            words("feed 'mount' \"japan's melon\""),
            vec![
                w("feed"),
                Token::Quoted("mount".into()),
                Token::Quoted("japan's melon".into())
            ]
        );
    }

    #[test]
    fn compare_signs_are_found_with_or_without_spaces() {
        assert_eq!(
            words("if hits<maxhits"),
            vec![
                w("if"),
                w("hits"),
                Token::Compare(Compare::Less),
                w("maxhits")
            ]
        );
        assert_eq!(
            words("if skill 'Magery' >= 99"),
            vec![
                w("if"),
                w("skill"),
                Token::Quoted("Magery".into()),
                Token::Compare(Compare::GreaterOrEqual),
                w("99")
            ]
        );
        assert_eq!(
            words("while hits != 0"),
            vec![
                w("while"),
                w("hits"),
                Token::Compare(Compare::NotEqual),
                w("0")
            ]
        );
    }

    #[test]
    fn the_force_mark_stays_on_its_command() {
        assert_eq!(
            words("target! 'enemy'"),
            vec![w("target!"), Token::Quoted("enemy".into())]
        );
    }

    #[test]
    fn notes_and_blank_lines_give_nothing() {
        assert!(words("   ").is_empty());
        assert!(words("// heal loop").is_empty());
    }

    #[test]
    fn an_open_quote_is_an_error_on_its_line() {
        let err = tokenize("msg 'hello", 7).unwrap_err();
        assert_eq!(err.line, 7);
    }

    #[test]
    fn a_compare_holds_or_not() {
        assert!(Compare::Less.holds(1, 2));
        assert!(!Compare::Equal.holds(1, 2));
        assert!(Compare::GreaterOrEqual.holds(2.0, 2.0));
    }
}
