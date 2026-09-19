//! The sounds and the music of the client files.
//!
//! A sound effect is a record of `soundLegacyMUL.uop`, or of `sound.mul` with
//! `soundidx.mul`: a header, then samples. `Sound.def` gives a sound with no
//! record of its own the record of a different sound. The music is MP3 files
//! under `Music/Digital`, named by `Config.txt`.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::mul::{capped_len, first_existing, idx_entries, is_uop_path, read_file, MapError};
use crate::uop::{decompress, load_sound_entries, UopIndex};

pub const SOUND_UOP_NAME: &str = "soundLegacyMUL.uop";
pub const SOUND_MUL_NAME: &str = "sound.mul";
pub const SOUND_IDX_NAME: &str = "soundidx.mul";
const SOUND_DEF_NAME: &str = "Sound.def";
const MUSIC_DIR: &str = "Music/Digital";
const MUSIC_CONFIG: &str = "Config.txt";
const MUSIC_EXTENSION: &str = "mp3";
const MUSIC_LOOP_WORD: &str = "loop";

/// Each sound is one channel of signed 16-bit samples at this rate.
pub const SOUND_SAMPLE_RATE: u32 = 22_050;
/// A record starts with the name of the sound and some numbers no reader
/// needs. The samples come after them.
const SOUND_HEADER_BYTES: usize = 40;
const SAMPLE_BYTES: usize = 2;

pub struct SoundData {
    file: Mutex<File>,
    entries: Vec<Option<UopIndex>>,
    /// A sound that plays as a different sound.
    plays_as: HashMap<u16, u16>,
}

/// One piece of music: the file, and if it starts again at its end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicTrack {
    pub path: PathBuf,
    pub repeats: bool,
}

/// The rows of `Sound.def`: `sound {plays_as} 0`.
fn parse_sound_def(text: &str) -> HashMap<u16, u16> {
    text.lines()
        .filter_map(|line| {
            let line = line.split('#').next()?.trim();
            let (sound, rest) = line.split_once('{')?;
            let (plays_as, _) = rest.split_once('}')?;
            let first = plays_as.split(',').next()?;
            Some((sound.trim().parse().ok()?, first.trim().parse().ok()?))
        })
        .collect()
}

fn samples_of(record: &[u8]) -> Option<Vec<i16>> {
    let samples: Vec<i16> = record
        .get(SOUND_HEADER_BYTES..)?
        .chunks_exact(SAMPLE_BYTES)
        .map(|s| i16::from_le_bytes([s[0], s[1]]))
        .collect();
    (!samples.is_empty()).then_some(samples)
}

impl SoundData {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let dir = uopath.as_ref();
        let path = first_existing(dir, &[SOUND_UOP_NAME, SOUND_MUL_NAME])
            .ok_or(MapError::Missing("sound"))?;
        let entries = if is_uop_path(&path) {
            load_sound_entries(&path)?
        } else {
            let idx = dir.join(SOUND_IDX_NAME);
            if !idx.exists() {
                return Err(MapError::Missing(SOUND_IDX_NAME));
            }
            idx_entries(&read_file(&idx)?)
        };
        let def = read_file(&dir.join(SOUND_DEF_NAME)).unwrap_or_default();
        let def: String = def.iter().map(|&b| char::from(b)).collect();
        Ok(Self {
            file: Mutex::new(File::open(&path)?),
            entries,
            plays_as: parse_sound_def(&def),
        })
    }

    /// The samples of one sound, or None when the files hold none.
    pub fn samples(&self, sound: u16) -> Option<Vec<i16>> {
        self.read(sound).or_else(|| {
            let other = *self.plays_as.get(&sound)?;
            self.read(other)
        })
    }

    fn read(&self, sound: u16) -> Option<Vec<i16>> {
        let entry = self.entries.get(usize::from(sound))?.as_ref()?;
        let mut file = self.file.lock().ok()?;
        let file_len = file.metadata().ok()?.len();
        let mut raw = vec![0u8; capped_len(file_len, entry.offset, entry.compressed_len)];
        file.seek(SeekFrom::Start(entry.offset)).ok()?;
        file.read_exact(&mut raw).ok()?;
        samples_of(&decompress(&raw, entry.compression, entry.decompressed_len).ok()?)
    }
}

/// The music the client files hold, under the number the shard sends.
pub struct MusicList {
    tracks: HashMap<u16, MusicTrack>,
}

/// One row of `Config.txt`: `number name[,loop]`. The name may have no
/// extension, and its case may differ from the file on disk.
fn parse_music_row(line: &str) -> Option<(u16, String, bool)> {
    let (number, rest) = line.trim().split_once(char::is_whitespace)?;
    let mut parts = rest.trim().split(',');
    let name = parts.next()?.trim();
    let name = name
        .rsplit_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_ascii_lowercase();
    let repeats = parts.any(|word| word.trim().eq_ignore_ascii_case(MUSIC_LOOP_WORD));
    (!name.is_empty()).then_some((number.parse().ok()?, name, repeats))
}

impl MusicList {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let dir = uopath.as_ref().join(MUSIC_DIR);
        let config = read_file(&dir.join(MUSIC_CONFIG))?;
        let config: String = config.iter().map(|&b| char::from(b)).collect();
        let on_disk: HashMap<String, PathBuf> = std::fs::read_dir(&dir)?
            .filter_map(|entry| Some(entry.ok()?.path()))
            .filter(|path| {
                path.extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case(MUSIC_EXTENSION))
            })
            .filter_map(|path| {
                let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
                Some((stem, path))
            })
            .collect();
        let tracks = config
            .lines()
            .filter_map(parse_music_row)
            .filter_map(|(number, name, repeats)| {
                let path = on_disk.get(&name)?.clone();
                Some((number, MusicTrack { path, repeats }))
            })
            .collect();
        Ok(Self { tracks })
    }

    pub fn track(&self, music: u16) -> Option<&MusicTrack> {
        self.tracks.get(&music)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_come_after_the_header() {
        let mut record = vec![0u8; SOUND_HEADER_BYTES];
        record.extend(1000i16.to_le_bytes());
        record.extend((-2i16).to_le_bytes());
        assert_eq!(samples_of(&record), Some(vec![1000, -2]));
        assert_eq!(samples_of(&record[..SOUND_HEADER_BYTES]), None);
        assert_eq!(samples_of(&[0u8; 8]), None);
    }

    #[test]
    fn tables_read_rows() {
        let def = parse_sound_def("# c\n654 {487} 0\n655 {263, 264} 0\n");
        assert_eq!(def[&654], 487);
        assert_eq!(def[&655], 263);
        assert_eq!(
            parse_music_row("9 Britain1.mp3,loop"),
            Some((9, "britain1".into(), true))
        );
        assert_eq!(
            parse_music_row("12 Stones2"),
            Some((12, "stones2".into(), false))
        );
        assert_eq!(parse_music_row("# words"), None);
    }

    #[test]
    fn real_files_give_a_sound_and_a_piece_of_music() {
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        const SWORD_HIT: u16 = 0x023B;
        const SOME_MUSIC: u16 = 9;
        let sounds = SoundData::open(&dir).unwrap();
        assert!(sounds.samples(SWORD_HIT).unwrap().len() > 1000);
        let music = MusicList::open(&dir).unwrap();
        assert!(music.track(SOME_MUSIC).unwrap().path.exists());
    }
}
