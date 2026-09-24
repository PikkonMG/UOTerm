//! The sounds and the music of the client files.
//!
//! A sound effect is a record of `soundLegacyMUL.uop`, or of `sound.mul` with
//! `soundidx.mul`: a header, then samples. `Sound.def` gives a sound with no
//! record of its own the record of a different sound.
//!
//! The music is MP3 files under `Music/Digital` in clients from 4.0.11c, and
//! MIDI files under `Music` in older clients. `Config.txt` names the file of
//! each number and says if it starts again at its end. A client with no
//! `Config.txt` uses the list the first clients had.

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
const MUSIC_DIR: &str = "Music";
/// Clients from 4.0.11c keep `Config.txt` with the MP3 files; older ones
/// keep it with the MIDI files.
const MUSIC_CONFIGS: [&str; 2] = ["Digital/Config.txt", "Config.txt"];
const MP3_EXTENSION: &str = "mp3";
const MIDI_EXTENSION: &str = "mid";
const MUSIC_LOOP_WORD: &str = "loop";
/// A row of `Config.txt` splits at these.
const MUSIC_ROW_DELIMITERS: [char; 3] = [' ', ',', '\t'];
/// A row is a number and a name, and maybe the loop word.
const MUSIC_ROW_MIN_PARTS: usize = 2;
const MUSIC_ROW_MAX_PARTS: usize = 3;
/// The music of a client with no `Config.txt`, by number: the name of the
/// file, and if it starts again at its end.
const FIRST_CLIENT_MUSIC: [(&str, bool); 67] = [
    ("oldult01", true),
    ("create1", false),
    ("dragflit", false),
    ("oldult02", true),
    ("oldult03", true),
    ("oldult04", true),
    ("oldult05", true),
    ("oldult06", true),
    ("stones2", true),
    ("britain1", true),
    ("britain2", true),
    ("bucsden", true),
    ("jhelom", false),
    ("lbcastle", false),
    ("linelle", false),
    ("magincia", true),
    ("minoc", true),
    ("ocllo", true),
    ("samlethe", false),
    ("serpents", true),
    ("skarabra", true),
    ("trinsic", true),
    ("vesper", true),
    ("wind", true),
    ("yew", true),
    ("cave01", false),
    ("dungeon9", false),
    ("forest_a", false),
    ("intown01", false),
    ("jungle_a", false),
    ("mountn_a", false),
    ("plains_a", false),
    ("sailing", false),
    ("swamp_a", false),
    ("tavern01", false),
    ("tavern02", false),
    ("tavern03", false),
    ("tavern04", false),
    ("combat1", false),
    ("combat2", false),
    ("combat3", false),
    ("approach", false),
    ("death", false),
    ("victory", false),
    ("btcastle", false),
    ("nujelm", true),
    ("dungeon2", false),
    ("cove", true),
    ("moonglow", true),
    ("zento", true),
    ("tokunodungeon", true),
    ("taiko", true),
    ("dreadhornarea", true),
    ("elfcity", true),
    ("grizzledungeon", true),
    ("melisandeslair", true),
    ("paroxysmuslair", true),
    ("gwennoconversation", true),
    ("goodendgame", true),
    ("goodvsevil", true),
    ("greatearthserpents", true),
    ("humanoids_u9", true),
    ("minocnegative", true),
    ("paws", true),
    ("selimsbar", true),
    ("serpentislecombat_u7", true),
    ("valoriaships", true),
];

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

/// One piece of music: its MP3 file, its MIDI file, and if it starts again
/// at its end. A client has one of the two files, or both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicTrack {
    pub mp3: Option<PathBuf>,
    pub midi: Option<PathBuf>,
    /// `Config.txt` names the MIDI file, so it plays before the MP3 file.
    pub midi_first: bool,
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

/// One row of `Config.txt`, as the reference client reads it: the number,
/// the name of the file, and the loop word. The name may have no
/// extension, and its case may differ from the file on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
struct MusicRow {
    number: u16,
    name: String,
    midi_first: bool,
    repeats: bool,
}

fn parse_music_row(line: &str) -> Option<MusicRow> {
    let parts: Vec<&str> = line
        .split(MUSIC_ROW_DELIMITERS)
        .filter(|part| !part.is_empty())
        .collect();
    if !(MUSIC_ROW_MIN_PARTS..=MUSIC_ROW_MAX_PARTS).contains(&parts.len()) {
        return None;
    }
    let (name, extension) = parts[1].rsplit_once('.').unwrap_or((parts[1], ""));
    if name.is_empty() {
        return None;
    }
    Some(MusicRow {
        number: parts[0].parse().ok()?,
        name: name.to_ascii_lowercase(),
        midi_first: extension.eq_ignore_ascii_case(MIDI_EXTENSION),
        repeats: parts
            .get(MUSIC_ROW_MIN_PARTS)
            .is_some_and(|word| word.eq_ignore_ascii_case(MUSIC_LOOP_WORD)),
    })
}

/// The rows of the list the first clients had, for a client with no
/// `Config.txt`.
fn first_client_rows() -> Vec<MusicRow> {
    FIRST_CLIENT_MUSIC
        .iter()
        .zip(0u16..)
        .map(|(&(name, repeats), number)| MusicRow {
            number,
            name: name.to_string(),
            midi_first: false,
            repeats,
        })
        .collect()
}

/// The files under `dir` and its folders, in path order.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut folders = vec![dir.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for path in entries.filter_map(|entry| Some(entry.ok()?.path())) {
            if path.is_dir() {
                folders.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Music files by the lowercase name with no extension: the MP3 files,
/// then the MIDI files.
type MusicFiles = (HashMap<String, PathBuf>, HashMap<String, PathBuf>);

/// The music files under `dir`. Of two files with one name, the first in
/// path order wins.
fn music_files(dir: &Path) -> MusicFiles {
    let (mut mp3, mut midi) = (HashMap::new(), HashMap::new());
    for path in files_under(dir) {
        let (Some(extension), Some(stem)) = (
            path.extension().and_then(|e| e.to_str()),
            path.file_stem().and_then(|s| s.to_str()),
        ) else {
            continue;
        };
        let files = if extension.eq_ignore_ascii_case(MP3_EXTENSION) {
            &mut mp3
        } else if extension.eq_ignore_ascii_case(MIDI_EXTENSION) {
            &mut midi
        } else {
            continue;
        };
        let name = stem.to_ascii_lowercase();
        files.entry(name).or_insert(path);
    }
    (mp3, midi)
}

impl MusicList {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let dir = uopath.as_ref().join(MUSIC_DIR);
        if !dir.is_dir() {
            return Err(MapError::Missing(MUSIC_DIR));
        }
        let rows = match first_existing(&dir, &MUSIC_CONFIGS) {
            Some(config) => {
                let config = read_file(&config)?;
                let config: String = config.iter().map(|&b| char::from(b)).collect();
                config.lines().filter_map(parse_music_row).collect()
            }
            None => first_client_rows(),
        };
        let (mp3, midi) = music_files(&dir);
        let tracks = rows
            .into_iter()
            .filter_map(|row| {
                let track = MusicTrack {
                    mp3: mp3.get(&row.name).cloned(),
                    midi: midi.get(&row.name).cloned(),
                    midi_first: row.midi_first,
                    repeats: row.repeats,
                };
                let has_file = track.mp3.is_some() || track.midi.is_some();
                has_file.then_some((row.number, track))
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
    }

    fn row(number: u16, name: &str, midi_first: bool, repeats: bool) -> Option<MusicRow> {
        Some(MusicRow {
            number,
            name: name.into(),
            midi_first,
            repeats,
        })
    }

    #[test]
    fn config_rows_give_the_name_the_kind_and_the_loop() {
        assert_eq!(
            parse_music_row("9 Britain1.mp3,loop"),
            row(9, "britain1", false, true)
        );
        assert_eq!(
            parse_music_row("12 Stones2"),
            row(12, "stones2", false, false)
        );
        assert_eq!(
            parse_music_row("0\toldult01.MID loop"),
            row(0, "oldult01", true, true)
        );
        assert_eq!(
            parse_music_row("64 SelimsBar.mp3                 "),
            row(64, "selimsbar", false, false)
        );
        assert_eq!(
            parse_music_row("42 deathtune,once"),
            row(42, "deathtune", false, false)
        );
        assert_eq!(parse_music_row("# words"), None);
        assert_eq!(parse_music_row("7"), None);
        assert_eq!(parse_music_row("7 a b c"), None);
        assert_eq!(parse_music_row(""), None);
    }

    #[test]
    fn a_client_with_no_config_has_the_first_list() {
        const DEATH: usize = 42;
        let rows = first_client_rows();
        assert_eq!(rows.len(), FIRST_CLIENT_MUSIC.len());
        assert_eq!(Some(rows[0].clone()), row(0, "oldult01", false, true));
        assert_eq!(Some(rows[DEATH].clone()), row(42, "death", false, false));
    }

    #[test]
    fn the_list_finds_mp3_and_midi_files_in_any_case_and_folder() {
        let uopath = crate::tests::scratch("music");
        let music = uopath.join(MUSIC_DIR);
        std::fs::create_dir_all(music.join("Digital")).unwrap();
        std::fs::write(
            music.join("Digital/Config.txt"),
            "1 Town.mp3,loop\n2 old.mid\n3 gone\n4 both\n",
        )
        .unwrap();
        for file in [
            "Digital/TOWN.MP3",
            "old.mid",
            "Digital/both.mp3",
            "both.mid",
        ] {
            std::fs::write(music.join(file), b"").unwrap();
        }
        let list = MusicList::open(&uopath).unwrap();
        let town = list.track(1).unwrap();
        assert!(town.repeats && town.midi.is_none());
        assert!(town.mp3.as_ref().unwrap().ends_with("Digital/TOWN.MP3"));
        let old = list.track(2).unwrap();
        assert!(old.midi_first && !old.repeats && old.mp3.is_none() && old.midi.is_some());
        assert!(list.track(3).is_none());
        let both = list.track(4).unwrap();
        assert!(both.mp3.is_some() && both.midi.is_some() && !both.midi_first);
        let _ = std::fs::remove_dir_all(&uopath);
    }

    #[test]
    fn an_old_client_with_no_config_plays_its_midi_by_the_first_list() {
        const STONES: u16 = 8;
        let uopath = crate::tests::scratch("old-music");
        std::fs::create_dir_all(uopath.join(MUSIC_DIR)).unwrap();
        std::fs::write(uopath.join(MUSIC_DIR).join("Stones2.mid"), b"").unwrap();
        let list = MusicList::open(&uopath).unwrap();
        let stones = list.track(STONES).unwrap();
        assert!(stones.repeats && stones.midi.is_some() && stones.mp3.is_none());
        assert!(list.track(STONES + 1).is_none());
        let _ = std::fs::remove_dir_all(&uopath);
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
        assert!(music
            .track(SOME_MUSIC)
            .unwrap()
            .mp3
            .as_ref()
            .unwrap()
            .exists());
    }
}
