//! The newer animation packages, `AnimationFrame1.uop` and its sisters. A
//! body that `mobtypes.txt` marks for them has no pictures in the classic
//! files. One record holds one action of one body in all five directions.
//!
//! A record starts with a header, the count of frames and where the frame
//! table starts. Each row of the table has the action, the number of the
//! frame from one, and where its pixels are from the start of the row. The
//! frames of the first direction come first, then the second, and so on. A
//! frame that is missing from the table is an empty picture.
//!
//! `AnimationSequence.uop` says, for some bodies, that an action shows the
//! pictures of a different action.

use std::collections::HashMap;
use std::path::Path;

use crate::anim::{decode_frame, DIRECTIONS_STORED, PALETTE_BYTES};
use crate::art::ArtPixels;
use crate::mul::slice_at;
use crate::uop::{hash_filename, read_directory, Package};

const FRAME_FILES: std::ops::RangeInclusive<u32> = 1..=6;
const SEQUENCE_NAME: &str = "AnimationSequence.uop";
const HEADER_SKIP: usize = 32;
const WORD: usize = 2;
const DWORD: usize = 4;
/// A row of the frame table: the action, the frame number, eight bytes that
/// are not used here, and the offset of the pixels.
const ROW_BYTES: usize = 16;
const ROW_FRAME_AT: usize = WORD;
const ROW_PIXELS_AT: usize = 12;
/// No real action has more frames than this in all its directions.
const MAX_ROWS: usize = 1024;
/// A sequence record: the body, 48 bytes that are not used here, the count
/// of replaced actions. Then each replaced action: the old action, a frame
/// count, the new action, and 60 bytes that are not used here.
const SEQUENCE_SKIP: usize = 48;
const REPLACE_BYTES: usize = DWORD * 3 + 60;
/// Two counts that mark a record with no list of actions.
const NO_REPLACE_LIST: [u32; 2] = [48, 68];

fn frame_file_name(number: u32) -> String {
    format!("AnimationFrame{number}.uop")
}

fn record_name(body: u16, action: u32) -> String {
    format!("build/animationlegacyframe/{body:06}/{action:02}.bin")
}

fn dword_at(data: &[u8], at: usize) -> Option<u32> {
    slice_at(data, at, DWORD).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn word_at(data: &[u8], at: usize) -> Option<u16> {
    slice_at(data, at, WORD).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

pub(crate) struct UopAnims {
    /// Each package, with its records under the hash of their names.
    packages: Vec<(Package, HashMap<u64, u32>)>,
    /// For a body, the actions that show the pictures of another action.
    replaced: HashMap<u16, HashMap<u32, u32>>,
}

impl UopAnims {
    /// None when the client files hold none of the packages.
    pub(crate) fn open(dir: &Path) -> Option<Self> {
        let packages: Vec<_> = FRAME_FILES
            .filter_map(|number| {
                let path = dir.join(frame_file_name(number));
                let directory = read_directory(&path).ok()?;
                // The package reads a record by its place in a list.
                let (hashes, entries): (Vec<u64>, Vec<_>) = directory.into_iter().unzip();
                let places = hashes
                    .into_iter()
                    .enumerate()
                    .map(|(place, hash)| (hash, place as u32))
                    .collect();
                let entries = entries.into_iter().map(Some).collect();
                Some((Package::new(&path, entries).ok()?, places))
            })
            .collect();
        (!packages.is_empty()).then(|| Self {
            packages,
            replaced: read_sequences(&dir.join(SEQUENCE_NAME)),
        })
    }

    /// Each frame of one action of a body in one stored direction.
    pub(crate) fn frames(
        &self,
        body: u16,
        action: u32,
        direction: u32,
    ) -> Option<Vec<Option<(i32, i32, ArtPixels)>>> {
        let action = self
            .replaced
            .get(&body)
            .and_then(|actions| actions.get(&action))
            .copied()
            .unwrap_or(action);
        let hash = hash_filename(&record_name(body, action));
        let data = self
            .packages
            .iter()
            .find_map(|(package, places)| package.read(*places.get(&hash)?))?;
        decode_direction(&data, direction)
    }
}

/// The frame numbers of the table with their pixel places, in order. A
/// number the table jumps over gets no place.
fn frame_places(data: &[u8]) -> Option<Vec<Option<usize>>> {
    let count = (dword_at(data, HEADER_SKIP)? as usize).min(MAX_ROWS);
    let table = dword_at(data, HEADER_SKIP + DWORD)? as usize;
    let mut places = Vec::new();
    for row in 0..count {
        let at = table + row * ROW_BYTES;
        let number = usize::from(word_at(data, at + ROW_FRAME_AT)?);
        let pixels = at + dword_at(data, at + ROW_PIXELS_AT)? as usize;
        if number == 0 || number > MAX_ROWS {
            return None;
        }
        while places.len() + 1 < number {
            places.push(None);
        }
        if places.len() < number {
            places.push(Some(pixels));
        }
    }
    Some(places)
}

fn decode_direction(data: &[u8], direction: u32) -> Option<Vec<Option<(i32, i32, ArtPixels)>>> {
    let places = frame_places(data)?;
    let directions = DIRECTIONS_STORED as usize;
    let per_direction = (places.len() + directions / 2) / directions;
    if per_direction == 0 {
        return None;
    }
    let first = direction as usize * per_direction;
    let frames: Vec<_> = (first..first + per_direction)
        .map(|place| {
            let pixels = (*places.get(place)?)?;
            let palette = slice_at(data, pixels, PALETTE_BYTES)?;
            decode_frame(data, palette, pixels + PALETTE_BYTES)
        })
        .collect();
    frames.iter().any(Option::is_some).then_some(frames)
}

fn read_sequences(path: &Path) -> HashMap<u16, HashMap<u32, u32>> {
    let Ok(directory) = read_directory(path) else {
        return HashMap::new();
    };
    let entries: Vec<_> = directory.into_values().map(Some).collect();
    let count = entries.len() as u32;
    let Ok(package) = Package::new(path, entries) else {
        return HashMap::new();
    };
    (0..count)
        .filter_map(|place| parse_sequence(&package.read(place)?))
        .collect()
}

fn parse_sequence(data: &[u8]) -> Option<(u16, HashMap<u32, u32>)> {
    let body = u16::try_from(dword_at(data, 0)?).ok()?;
    let count = dword_at(data, DWORD + SEQUENCE_SKIP)?;
    let mut replaced = HashMap::new();
    if !NO_REPLACE_LIST.contains(&count) {
        let list = DWORD * 2 + SEQUENCE_SKIP;
        for row in 0..count as usize {
            let at = list + row * REPLACE_BYTES;
            let old = dword_at(data, at)?;
            let frame_count = dword_at(data, at + DWORD)?;
            let new = dword_at(data, at + DWORD * 2)?;
            // A row with frames of its own is a new action, not a replaced one.
            if frame_count == 0 {
                replaced.insert(old, new);
            }
        }
    }
    Some((body, replaced))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE_AT: u32 = 40;

    /// A record with the given frame numbers and no pixels.
    fn record(numbers: &[u16]) -> Vec<u8> {
        let mut data = vec![0u8; HEADER_SKIP];
        data.extend((numbers.len() as u32).to_le_bytes());
        data.extend(TABLE_AT.to_le_bytes());
        for number in numbers {
            let mut row = [0u8; ROW_BYTES];
            row[ROW_FRAME_AT..ROW_FRAME_AT + WORD].copy_from_slice(&number.to_le_bytes());
            row[ROW_PIXELS_AT..].copy_from_slice(&500u32.to_le_bytes());
            data.extend(row);
        }
        data
    }

    #[test]
    fn a_frame_the_table_jumps_over_is_an_empty_place() {
        let places = frame_places(&record(&[1, 2, 4])).unwrap();
        assert_eq!(places.len(), 4);
        assert!(places[0].is_some() && places[2].is_none() && places[3].is_some());
        assert_eq!(places[1], Some(TABLE_AT as usize + ROW_BYTES + 500));
        assert!(frame_places(&record(&[0])).is_none());
        assert!(frame_places(&[0; 8]).is_none());
    }

    #[test]
    fn a_sequence_record_replaces_only_actions_with_no_frames_of_their_own() {
        let mut data = Vec::new();
        data.extend(666u32.to_le_bytes());
        data.extend([0u8; SEQUENCE_SKIP]);
        data.extend(2u32.to_le_bytes());
        for (old, frame_count, new) in [(4u32, 0u32, 24u32), (5, 9, 25)] {
            data.extend(old.to_le_bytes());
            data.extend(frame_count.to_le_bytes());
            data.extend(new.to_le_bytes());
            data.extend([0u8; 60]);
        }
        let (body, replaced) = parse_sequence(&data).unwrap();
        assert_eq!(body, 666);
        assert_eq!(replaced.get(&4), Some(&24));
        assert_eq!(replaced.get(&5), None);
    }

    #[test]
    fn the_names_of_the_records_are_as_the_client_writes_them() {
        assert_eq!(
            record_name(666, 4),
            "build/animationlegacyframe/000666/04.bin"
        );
        assert_eq!(frame_file_name(3), "AnimationFrame3.uop");
    }
}
