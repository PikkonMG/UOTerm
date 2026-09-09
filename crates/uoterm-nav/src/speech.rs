//! The phrases a shard listens for, and the numbers it listens by.
//!
//! A shard does not read the words a character says. It reads the keyword
//! numbers the client sends beside them: say "i renounce my young player
//! status" with no number attached and the shard hears chatter, because the
//! matching is the client's job and always has been. `speech.mul` holds every
//! phrase and the number it stands for, and this module does that matching the
//! way the reference client does it, so `encode::keyword_speech` in
//! `uoterm-protocol` has numbers to send.
//!
//! The file is read once and never changes after that, so one instance serves
//! every character on a shard. Read it through the same cache the runtime
//! shares a [`crate::MulMap`] with, never once per character.

use std::path::Path;

use uoterm_protocol::ClientVersion;

use crate::mul::{first_existing, read_file, slice_at, MapError};

/// One record opens with the keyword number and the length of the phrase, both
/// big-endian `u16`. The phrase itself is UTF-8 and follows straight after.
pub const SPEECH_RECORD_HEADER: usize = 4;
/// A phrase writes this where any run of text is allowed.
pub const SPEECH_WILDCARD: char = '*';

/// Client directories do not agree on the case of this name.
pub(crate) const SPEECH_MUL_NAMES: [&str; 2] = ["speech.mul", "Speech.mul"];

/// A client older than this sends no keyword numbers at all, whatever a
/// character says. The reference client writes the letter of a lettered
/// version in the last field, so 3.0.5d is 3.0.5 with `d` for its patch, and
/// plain 3.0.5 sits below it.
pub const KEYWORD_SPEECH_MIN_VERSION: ClientVersion = ClientVersion {
    major: 3,
    minor: 0,
    revision: 5,
    patch: b'd' as u32,
};

/// Only spaces are trimmed off a spoken phrase, which is what the reference
/// client trims.
const TRIMMED: char = ' ';

/// One phrase the shard listens for.
struct Phrase {
    keyword: u16,
    /// The runs of text between the wildcards, folded for matching. Any one of
    /// them matching is enough, which is how the reference client reads a
    /// phrase with a wildcard inside it.
    parts: Vec<Vec<char>>,
    /// The phrase does not open with a wildcard, so a match has to sit at the
    /// start of what was said.
    anchored_start: bool,
    /// The phrase does not close with a wildcard, so a match has to sit at the
    /// end of what was said.
    anchored_end: bool,
}

impl Phrase {
    fn new(keyword: u16, pattern: &str) -> Self {
        Self {
            keyword,
            parts: pattern
                .split(SPEECH_WILDCARD)
                .filter(|part| !part.is_empty())
                .map(fold)
                .collect(),
            anchored_start: !pattern.starts_with(SPEECH_WILDCARD),
            anchored_end: !pattern.ends_with(SPEECH_WILDCARD),
        }
    }

    /// True when what was said matches this phrase. `said` is already folded.
    fn matches(&self, said: &[char]) -> bool {
        self.parts.iter().any(|part| {
            if part.len() > said.len() {
                return false;
            }
            if self.anchored_start && !said.starts_with(part) {
                return false;
            }
            if self.anchored_end && !said.ends_with(part) {
                return false;
            }
            said.windows(part.len())
                .enumerate()
                .any(|(at, window)| window == part && whole_word(said, at, part.len()))
        })
    }
}

/// True when the run of text at `at` is a word of its own and not the middle of
/// a longer one. A mark of punctuation beside it still leaves it a word, so
/// "bank!" is the same word as "bank".
fn whole_word(said: &[char], at: usize, len: usize) -> bool {
    let end = at + len;
    let before = at == 0 || !said[at - 1].is_alphabetic();
    let after = end == said.len() || !said[end].is_alphabetic();
    before && after
}

/// Folds text so that what was said and the phrase match whatever case either
/// was written in.
fn fold(text: &str) -> Vec<char> {
    text.chars()
        .map(|letter| letter.to_lowercase().next().unwrap_or(letter))
        .collect()
}

/// Every phrase the client files hold, and the keyword each one stands for.
pub struct SpeechData {
    phrases: Vec<Phrase>,
}

impl SpeechData {
    /// Reads `speech.mul` out of a client directory.
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let path = first_existing(uopath.as_ref(), &SPEECH_MUL_NAMES)
            .ok_or(MapError::Missing(SPEECH_MUL_NAMES[0]))?;
        Self::from_file(&path)
    }

    /// Reads one keyword file. The file must divide into whole records and
    /// hold at least one phrase, because a file with no phrases in it leaves a
    /// character unable to say anything a shard reacts to.
    pub fn from_file(path: &Path) -> Result<Self, MapError> {
        let data = read_file(path)?;
        let mut phrases = Vec::new();
        let mut at = 0usize;
        while at < data.len() {
            let head = slice_at(&data, at, SPEECH_RECORD_HEADER).ok_or(MapError::Truncated)?;
            let keyword = u16::from_be_bytes([head[0], head[1]]);
            let len = usize::from(u16::from_be_bytes([head[2], head[3]]));
            at += SPEECH_RECORD_HEADER;
            if len == 0 {
                continue;
            }
            let text = slice_at(&data, at, len).ok_or(MapError::Truncated)?;
            at += len;
            phrases.push(Phrase::new(keyword, &String::from_utf8_lossy(text)));
        }
        if phrases.is_empty() {
            return Err(MapError::Truncated);
        }
        Ok(Self { phrases })
    }

    /// The keyword numbers a client of that version sends beside those words,
    /// lowest first.
    ///
    /// A phrase that matches twice is counted twice, because the reference
    /// client counts it twice and the shard is told what a shard expects.
    /// Nothing at all comes back below [`KEYWORD_SPEECH_MIN_VERSION`], where
    /// the reference client sends the words with no numbers beside them.
    pub fn keywords(&self, version: ClientVersion, said: &str) -> Vec<u16> {
        if !version.at_least(KEYWORD_SPEECH_MIN_VERSION) {
            return Vec::new();
        }
        let said = fold(said.trim_matches(TRIMMED));
        let mut found: Vec<u16> = self
            .phrases
            .iter()
            .filter(|phrase| phrase.matches(&said))
            .map(|phrase| phrase.keyword)
            .collect();
        found.sort_unstable();
        found
    }

    /// How many phrases the client files hold.
    pub fn phrase_count(&self) -> usize {
        self.phrases.len()
    }
}
