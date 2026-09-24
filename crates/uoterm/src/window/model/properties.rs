//! The properties of items, as the shard writes them in the tooltip: a
//! name, and often a number, as "Fire Resist 10%" or "Durability 40 / 50".
//! On a shard with no property lists the lines are the words of a click.

use super::reads::Readings;
use serde_json::{json, Value};
use uoterm_runtime::tools::TOOL_PROPERTIES;

/// How long the properties of an item are trusted before they are read
/// again. An item can be worn down or renamed.
pub const PROPERTIES_MAX_AGE: f64 = 20.0;
const DURABILITY_WORDS: &str = "durability";
const DURABILITY_SPLIT: char = '/';
const LINES_KEY: &str = "lines";

/// One line of properties: its words, in lower case with no number, and
/// its first number.
#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub words: String,
    pub value: Option<f32>,
}

/// Words made plain for a match: letters and digits in lower case, with
/// single spaces for all else.
pub fn plain_words(words: &str) -> String {
    words
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Reads one line into its words and its first number.
pub fn parse(line: &str) -> Property {
    let mut words = String::new();
    let mut number = String::new();
    let mut value = None;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        let starts_number = c.is_ascii_digit()
            || (c == '-' && chars.peek().is_some_and(char::is_ascii_digit) && value.is_none());
        if value.is_none() && starts_number {
            number.push(c);
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() || next == '.' {
                    number.push(next);
                    chars.next();
                } else {
                    break;
                }
            }
            value = number.parse::<f32>().ok();
            continue;
        }
        if value.is_none() || !c.is_ascii_digit() {
            words.push(c);
        }
    }
    Property {
        words: plain_words(&words),
        value,
    }
}

/// The durability of an item: what it has and the most it can have.
pub fn durability(lines: &[String]) -> Option<(u16, u16)> {
    let line = lines
        .iter()
        .find(|line| line.to_lowercase().contains(DURABILITY_WORDS))?;
    let (now, max) = line.split_once(DURABILITY_SPLIT)?;
    let numbers = |part: &str| -> Vec<u16> {
        part.split(|c: char| !c.is_ascii_digit())
            .filter_map(|piece| piece.parse().ok())
            .collect()
    };
    Some((*numbers(now).last()?, *numbers(max).first()?))
}

/// The property lines of an item, from the session. Empty until the first
/// answer comes.
pub fn lines_of(readings: &mut Readings, serial: u32) -> Vec<String> {
    readings
        .want(
            TOOL_PROPERTIES,
            json!({ "serial": serial }),
            PROPERTIES_MAX_AGE,
        )
        .and_then(|answer| answer.get(LINES_KEY))
        .and_then(Value::as_array)
        .map(|lines| {
            lines
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_string()).collect()
    }

    #[test]
    fn a_line_gives_its_words_and_its_first_number() {
        assert_eq!(
            parse("Fire Resist 10%"),
            Property {
                words: "fire resist".into(),
                value: Some(10.0)
            }
        );
        assert_eq!(parse("Weight: 5 Stones").words, "weight stones");
        assert_eq!(parse("Weight: 5 Stones").value, Some(5.0));
        assert_eq!(parse("Damage 11 - 13").value, Some(11.0));
        assert_eq!(parse("Hit Chance Increase -5%").value, Some(-5.0));
        assert_eq!(parse("Undead Slayer").value, None);
        assert_eq!(parse("  Legendary   Artifact ").words, "legendary artifact");
        assert_eq!(parse("Durability 40 / 50").words, "durability");
    }

    #[test]
    fn durability_reads_what_is_left_and_the_most() {
        assert_eq!(durability(&lines(&["Durability 40 / 50"])), Some((40, 50)));
        assert_eq!(durability(&lines(&["durability: 3/255"])), Some((3, 255)));
        assert_eq!(durability(&lines(&["a shirt"])), None);
    }
}
