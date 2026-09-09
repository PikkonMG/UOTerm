//! The client text database: the sentences a shard sends as numbers.
//!
//! A shard says most of what it has to say by number. A refusal, a system
//! message, the caption of a window and every line of an item's property list
//! arrive as a message number and, where the sentence has blanks in it, a run
//! of tab separated arguments to fill them. A journal line reading
//! `System: #1001018` is the server stating plainly why it refused an action,
//! in a form nobody can read. The sentence behind that number is in the client
//! files, and this module reads it.
//!
//! The file is `Cliloc.enu`. Modern clients ship it compressed and mark the
//! compressed form with one byte of its header; the older plain form is read
//! the same way once that wrapper is off.
//!
//! The whole database is read once and never changes after that, so one
//! instance serves every character on a shard. Read it through the same cache
//! the runtime shares a [`crate::MulMap`] with, never once per character.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::Path;

use crate::mul::{first_existing, read_file, slice_at, MapError};

/// A `u32` and a `u16` no reader uses open the plain database.
pub const CLILOC_HEADER: usize = 6;
/// One record: the message number `u32`, one flag byte, the length of the text
/// `u16`, then that many bytes of UTF-8.
pub const CLILOC_RECORD_HEADER: usize = 7;
/// This byte of the file header marks the compressed form.
pub const CLILOC_COMPRESSED_MARK_AT: usize = 3;
/// What that byte holds when the file is compressed.
pub const CLILOC_COMPRESSED_MARK: u8 = 0x8E;

/// Client directories do not agree on the case of this name. The client asks
/// the shard in English, so English is the language a shard answers in.
pub(crate) const CLILOC_ENU_NAMES: [&str; 2] = ["Cliloc.enu", "cliloc.enu"];

/// Tab separates one argument of a message from the next.
const ARGUMENT_SEPARATOR: char = '\t';
/// A blank in a sentence stands between two of these.
const SLOT_MARK: char = '~';
/// A blank is numbered, then named. This separates the two.
const SLOT_NAME_SEPARATOR: char = '_';
/// An argument that opens with this is itself a message number.
const NESTED_NUMBER_MARK: char = '#';

/// How many byte values a table of them holds.
const SYMBOL_COUNT: usize = 256;
/// Every byte value is counted with a `u32`, and those counts open the body.
const COUNT_BYTES: usize = 4;
/// The counts of all byte values together.
const COUNT_TABLE_BYTES: usize = SYMBOL_COUNT * COUNT_BYTES;
/// A `u32` no reader uses opens the compressed file.
const COMPRESSED_HEADER: usize = 4;

/// What the text between two [`SLOT_MARK`]s asks for.
enum Slot<'a> {
    /// The blank the argument at this place fills, counted from zero.
    Field(usize),
    /// A blank numbered zero, which no argument answers.
    Empty,
    /// No number at all, so it is not a blank.
    Text(&'a str),
}

/// Every message the client files describe.
pub struct ClilocData {
    entries: HashMap<u32, String>,
}

impl ClilocData {
    /// Reads `Cliloc.enu` out of a client directory.
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let path = first_existing(uopath.as_ref(), &CLILOC_ENU_NAMES)
            .ok_or(MapError::Missing(CLILOC_ENU_NAMES[0]))?;
        Self::from_file(&path)
    }

    /// Reads one text database, compressed or plain.
    pub fn from_file(path: &Path) -> Result<Self, MapError> {
        let raw = read_file(path)?;
        let plain = if is_compressed(&raw) {
            decompress(&raw)?
        } else {
            raw
        };
        Ok(Self {
            entries: read_entries(&plain)?,
        })
    }

    /// Builds a database from the messages already in memory.
    pub fn from_entries(entries: HashMap<u32, String>) -> Self {
        Self { entries }
    }

    /// The sentence for one message number, as the client files write it.
    /// Blanks are left as they stand; [`Self::render`] fills them.
    pub fn text(&self, number: u32) -> Option<&str> {
        self.entries.get(&number).map(String::as_str)
    }

    /// The sentence for one message number with its blanks filled in.
    ///
    /// `arguments` is the tab separated run the shard sends beside the number.
    /// A blank no argument reaches is filled with nothing, which is what the
    /// reference client does, so a sentence always reads as a sentence.
    ///
    /// An argument of more than one character may name a message of its own:
    /// with a `#` in front of it always, and as a bare number only when the
    /// shard sent more than one argument. An argument that names a message the
    /// files do not hold is left as the shard wrote it, so its number survives
    /// in the sentence instead of leaving a gap.
    pub fn render(&self, number: u32, arguments: &str) -> Option<String> {
        let pattern = self.text(number)?;
        let fields = split_arguments(arguments);
        let mut out = String::with_capacity(pattern.len());
        let mut rest = pattern;
        while let Some((before, slot, after)) = next_slot(rest) {
            out.push_str(before);
            match read_slot(slot) {
                Slot::Field(field) => {
                    let text = fields.get(field).copied().unwrap_or_default();
                    out.push_str(self.field_text(text, fields.len()));
                }
                Slot::Empty => {}
                Slot::Text(text) => {
                    out.push(SLOT_MARK);
                    out.push_str(text);
                    out.push(SLOT_MARK);
                }
            }
            rest = after;
        }
        out.push_str(rest);
        Some(out)
    }

    /// Turns a journal line of the form `#number` or `#number arguments` into
    /// the English sentence, or leaves the line as it arrived when the number
    /// is not in the files.
    pub fn render_line(&self, text: &str) -> String {
        let Some(rest) = text.strip_prefix(NESTED_NUMBER_MARK) else {
            return text.to_string();
        };
        let (number, arguments) = match rest.split_once(' ') {
            Some((number, arguments)) => (number, arguments),
            None => (rest, ""),
        };
        match message_number(number).and_then(|n| self.render(n, arguments)) {
            Some(sentence) => sentence,
            None => text.to_string(),
        }
    }

    /// How many messages the client files describe.
    pub fn message_count(&self) -> usize {
        self.entries.len()
    }

    /// One argument as the reference client reads it. `count` is how many
    /// arguments the shard sent, because a bare number stands for a message
    /// only when it is one of several.
    fn field_text<'a>(&'a self, field: &'a str, count: usize) -> &'a str {
        if field.chars().nth(1).is_none() {
            return field;
        }
        if let Some(number) = field.strip_prefix(NESTED_NUMBER_MARK) {
            return message_number(number)
                .and_then(|n| self.text(n))
                .unwrap_or(field);
        }
        if count > 1 {
            if let Some(text) = message_number(field).and_then(|n| self.text(n)) {
                if !text.is_empty() {
                    return text;
                }
            }
        }
        field
    }
}

fn message_number(text: &str) -> Option<u32> {
    text.parse().ok()
}

/// The text before the next blank, the blank itself, and what follows it.
fn next_slot(text: &str) -> Option<(&str, &str, &str)> {
    let open = text.find(SLOT_MARK)?;
    let body = &text[open + SLOT_MARK.len_utf8()..];
    let close = body.find(SLOT_MARK)?;
    Some((
        &text[..open],
        &body[..close],
        &body[close + SLOT_MARK.len_utf8()..],
    ))
}

/// Reads the number a blank is written with. The number counts from one, and
/// the name after it is for a person reading the file.
fn read_slot(slot: &str) -> Slot<'_> {
    let head = slot
        .split_once(SLOT_NAME_SEPARATOR)
        .map_or(slot, |(number, _name)| number);
    let digits = head
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(head.len());
    let Ok(number) = head[..digits].parse::<usize>() else {
        return Slot::Text(slot);
    };
    match number.checked_sub(1) {
        Some(field) => Slot::Field(field),
        None => Slot::Empty,
    }
}

/// The arguments of one message, in the order the blanks count them.
///
/// Tabs in front of the first argument belong to the format and not to an
/// argument, so they are dropped. A run of nothing but tabs is one empty
/// argument, and so is no text at all.
fn split_arguments(arguments: &str) -> Vec<&str> {
    let start = arguments
        .find(|c: char| c != ARGUMENT_SEPARATOR)
        .unwrap_or(arguments.len());
    arguments[start..].split(ARGUMENT_SEPARATOR).collect()
}

fn is_compressed(raw: &[u8]) -> bool {
    raw.get(CLILOC_COMPRESSED_MARK_AT) == Some(&CLILOC_COMPRESSED_MARK)
}

/// Reads every record of the plain database. The file must divide into whole
/// records and hold at least one, because a database with nothing in it turns
/// every message a shard sends back into the number it came as.
fn read_entries(plain: &[u8]) -> Result<HashMap<u32, String>, MapError> {
    let mut entries = HashMap::new();
    let mut at = CLILOC_HEADER;
    while at < plain.len() {
        let head = slice_at(plain, at, CLILOC_RECORD_HEADER).ok_or(MapError::Truncated)?;
        let number = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
        // head[4] is a flag byte the reference client reads and never uses.
        let len = usize::from(u16::from_le_bytes([head[5], head[6]]));
        at += CLILOC_RECORD_HEADER;
        let text = slice_at(plain, at, len).ok_or(MapError::Truncated)?;
        at += len;
        entries.insert(number, String::from_utf8_lossy(text).into_owned());
    }
    if entries.is_empty() {
        return Err(MapError::Truncated);
    }
    Ok(entries)
}

/// Unwraps the compressed database.
///
/// The file holds the last column of a Burrows-Wheeler transform, and each of
/// its bytes is written as the place that byte holds in a list that is then
/// moved to the front. The first stage undoes that list, the second walks the
/// transform back into the text.
fn decompress(raw: &[u8]) -> Result<Vec<u8>, MapError> {
    rebuild(&undo_move_to_front(raw)?)
}

/// Turns each stored place back into the byte it named.
///
/// The first place stands right after the header, and the last byte of the
/// file names a place nothing is written for, exactly as the reference client
/// leaves it.
fn undo_move_to_front(raw: &[u8]) -> Result<Vec<u8>, MapError> {
    let places = raw
        .get(COMPRESSED_HEADER..raw.len().saturating_sub(1))
        .ok_or(MapError::Truncated)?;
    let mut table: [u8; SYMBOL_COUNT] = std::array::from_fn(|value| value as u8);
    let mut out = Vec::with_capacity(places.len());
    for &place in places {
        let at = usize::from(place);
        let value = table[at];
        table.copy_within(..at, 1);
        table[0] = value;
        out.push(value);
    }
    Ok(out)
}

/// Walks the transform back into the text it was made from.
///
/// The body opens with the count of every byte value. Those counts put the
/// byte values in the order the sorted column holds them, which gives each
/// value a run of its own and a place to read the next byte from. The marks
/// that follow the counts say how far along the order each next byte sits.
fn rebuild(body: &[u8]) -> Result<Vec<u8>, MapError> {
    let table = body.get(..COUNT_TABLE_BYTES).ok_or(MapError::Truncated)?;
    let mut counts = [0usize; SYMBOL_COUNT];
    for (value, field) in table.chunks_exact(COUNT_BYTES).enumerate() {
        counts[value] = u32::from_le_bytes([field[0], field[1], field[2], field[3]]) as usize;
    }
    let total = counts
        .iter()
        .try_fold(0usize, |sum, count| sum.checked_add(*count))
        .ok_or(MapError::Truncated)?;
    let marks = body.get(COUNT_TABLE_BYTES..).ok_or(MapError::Truncated)?;
    if marks.len() < total {
        return Err(MapError::Truncated);
    }

    // The value the file holds most of opens the order, and values held the
    // same number of times keep the order their byte values have.
    let mut present: Vec<u8> = (0..SYMBOL_COUNT)
        .filter(|value| counts[*value] != 0)
        .map(|value| value as u8)
        .collect();
    present.sort_by_key(|value| Reverse(counts[usize::from(*value)]));

    let mut next = [0usize; SYMBOL_COUNT];
    let mut end = [0usize; SYMBOL_COUNT];
    let mut order: [u8; SYMBOL_COUNT] = std::array::from_fn(|value| value as u8);
    let mut run = 0usize;
    for &value in &present {
        let at = usize::from(value);
        // The mark that opens a run says where its value belongs in the order,
        // so it seeds the order instead of being walked over.
        order[usize::from(marks[run])] = value;
        next[at] = run + 1;
        run += counts[at];
        end[at] = run;
    }

    let mut out = Vec::with_capacity(total);
    let mut left = present.len();
    let mut value = order[0];
    for _ in 0..total {
        out.push(value);
        let at = usize::from(value);
        if next[at] >= end[at] {
            // That value is spent. Drop it out of the order and go on with
            // whatever the order holds next.
            if left > 0 {
                left -= 1;
                order.copy_within(1..=left, 0);
                value = order[0];
            }
        } else {
            let mark = usize::from(marks[next[at]]);
            next[at] += 1;
            if mark != 0 {
                order.copy_within(1..=mark, 0);
                order[mark] = value;
                value = order[0];
            }
        }
    }
    Ok(out)
}
