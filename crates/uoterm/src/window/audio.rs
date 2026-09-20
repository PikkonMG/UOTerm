//! The sound of the watch window: the music of the region, the sound effects
//! the shard asks for, and the footsteps of the mobiles near. Each kind has
//! its own volume, and one master volume is over them all.
//!
//! The sounds come from the client files. With no client files, or with no
//! sound device, the window is silent and the rest of it works the same.

use super::kept;
use crate::view::{WatchFrame, WatchSound};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::num::NonZero;
use std::path::Path;
use std::sync::Arc;
use uoterm_nav::{MusicList, SoundData, SOUND_SAMPLE_RATE};

const SETTINGS_FILE: &str = "watch-audio.toml";
const ONE_CHANNEL: NonZero<u16> = NonZero::new(1).unwrap();
const SAMPLE_RATE: NonZero<u32> = NonZero::new(SOUND_SAMPLE_RATE).unwrap();
const SAMPLE_FULL_SCALE: f32 = 32_768.0;
/// A sound this many tiles away is silent. Nearer sounds are louder.
const HEARING_TILES: f32 = 18.0;
/// The window keeps this many decoded sounds. When full, it starts again.
const SOUND_CACHE_CAP: usize = 128;
/// A person on foot makes these two sounds in turn, and a mount that runs
/// makes these two. A mount that walks makes the first foot sound only.
const STEPS_ON_FOOT: [u16; 2] = [0x012B, 0x012C];
const STEPS_MOUNT_RUN: [u16; 2] = [0x0129, 0x012A];
const DEFAULT_MASTER: f32 = 0.8;
const DEFAULT_MUSIC: f32 = 0.5;
const DEFAULT_EFFECTS: f32 = 0.8;
const DEFAULT_FOOTSTEPS: f32 = 0.4;

pub const NOTE_NO_FILES: &str = "No sound: the client files hold no sounds.";
pub const NOTE_NO_DEVICE: &str = "No sound: this computer gave no sound device.";

/// The kinds of sound that have a volume of their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Music,
    Effects,
    Footsteps,
}

/// What the operator set. Each volume is from 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub muted: bool,
    pub master: f32,
    pub music: f32,
    pub effects: f32,
    pub footsteps: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            muted: false,
            master: DEFAULT_MASTER,
            music: DEFAULT_MUSIC,
            effects: DEFAULT_EFFECTS,
            footsteps: DEFAULT_FOOTSTEPS,
        }
    }
}

impl Settings {
    /// How loud one kind plays: its own volume under the master volume.
    pub fn volume(&self, kind: Kind) -> f32 {
        if self.muted {
            return 0.0;
        }
        let own = match kind {
            Kind::Music => self.music,
            Kind::Effects => self.effects,
            Kind::Footsteps => self.footsteps,
        };
        (own * self.master).clamp(0.0, 1.0)
    }

    pub fn load() -> Self {
        kept::load(SETTINGS_FILE)
    }

    pub fn save(&self) {
        kept::save(SETTINGS_FILE, self);
    }
}

/// One step a mobile took this frame, for the footstep sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub tiles_away: f32,
    pub mounted: bool,
    pub running: bool,
}

/// The sound of one step. `count` is how many steps the window has played.
fn step_sound(step: Step, count: usize) -> u16 {
    match (step.mounted, step.running) {
        (true, true) => STEPS_MOUNT_RUN[count % STEPS_MOUNT_RUN.len()],
        (true, false) => STEPS_ON_FOOT[0],
        (false, _) => STEPS_ON_FOOT[count % STEPS_ON_FOOT.len()],
    }
}

/// How much of a sound arrives from `tiles_away`.
fn nearness(tiles_away: f32) -> f32 {
    (1.0 - tiles_away / HEARING_TILES).clamp(0.0, 1.0)
}

struct Device {
    sink: MixerDeviceSink,
    music: Player,
}

pub struct Audio {
    pub settings: Settings,
    /// Why there is no sound. Empty when there is sound.
    note: &'static str,
    device: Option<Device>,
    sounds: Option<SoundData>,
    music_list: Option<MusicList>,
    decoded: HashMap<u16, Arc<[f32]>>,
    /// The number of the last sound cue played. None before the first frame.
    last_cue: Option<u64>,
    playing_music: Option<u16>,
    steps_taken: usize,
}

impl Audio {
    pub fn new(uopath: Option<&Path>) -> Self {
        let sounds = uopath.and_then(|dir| SoundData::open(dir).ok());
        let device = sounds.as_ref().and_then(|_| {
            let mut sink = DeviceSinkBuilder::open_default_sink().ok()?;
            sink.log_on_drop(false);
            let music = Player::connect_new(sink.mixer());
            Some(Device { sink, music })
        });
        let note = match (&sounds, &device) {
            (None, _) => NOTE_NO_FILES,
            (Some(_), None) => NOTE_NO_DEVICE,
            (Some(_), Some(_)) => "",
        };
        Self {
            settings: Settings::load(),
            note,
            device,
            sounds,
            music_list: uopath.and_then(|dir| MusicList::open(dir).ok()),
            decoded: HashMap::new(),
            last_cue: None,
            playing_music: None,
            steps_taken: 0,
        }
    }

    pub fn note(&self) -> &str {
        self.note
    }

    /// Plays what is new in this frame.
    pub fn play(&mut self, frame: &WatchFrame, steps: &[Step]) {
        if self.device.is_none() {
            return;
        }
        self.follow_music(frame.music);
        for cue in new_cues(&frame.sounds, &mut self.last_cue) {
            let tiles_away = f32::from(cue.x.abs_diff(frame.x).max(cue.y.abs_diff(frame.y)));
            self.play_sound(cue.sound, Kind::Effects, tiles_away);
        }
        for step in steps {
            self.steps_taken += 1;
            let sound = step_sound(*step, self.steps_taken);
            self.play_sound(sound, Kind::Footsteps, step.tiles_away);
        }
    }

    /// Call this when the operator moved a volume, so the music follows.
    pub fn settings_changed(&self) {
        if let Some(device) = &self.device {
            device.music.set_volume(self.settings.volume(Kind::Music));
        }
    }

    fn follow_music(&mut self, wanted: Option<u16>) {
        if wanted == self.playing_music {
            return;
        }
        self.playing_music = wanted;
        let Some(device) = &self.device else {
            return;
        };
        device.music.stop();
        let track = wanted.and_then(|music| self.music_list.as_ref()?.track(music));
        let Some(track) = track else {
            return;
        };
        let Ok(file) = std::fs::File::open(&track.path) else {
            return;
        };
        device.music.set_volume(self.settings.volume(Kind::Music));
        if track.repeats {
            if let Ok(source) = Decoder::new_looped(std::io::BufReader::new(file)) {
                device.music.append(source);
            }
        } else if let Ok(source) = Decoder::new(std::io::BufReader::new(file)) {
            device.music.append(source);
        }
        device.music.play();
    }

    fn play_sound(&mut self, sound: u16, kind: Kind, tiles_away: f32) {
        let volume = self.settings.volume(kind) * nearness(tiles_away);
        if volume <= 0.0 {
            return;
        }
        let Some(samples) = self.samples(sound) else {
            return;
        };
        if let Some(device) = &self.device {
            let source = SamplesBuffer::new(ONE_CHANNEL, SAMPLE_RATE, samples.to_vec());
            device.sink.mixer().add(source.amplify(volume));
        }
    }

    fn samples(&mut self, sound: u16) -> Option<Arc<[f32]>> {
        if let Some(known) = self.decoded.get(&sound) {
            return Some(Arc::clone(known));
        }
        let samples: Arc<[f32]> = self
            .sounds
            .as_ref()?
            .samples(sound)?
            .into_iter()
            .map(|s| f32::from(s) / SAMPLE_FULL_SCALE)
            .collect();
        if self.decoded.len() >= SOUND_CACHE_CAP {
            self.decoded.clear();
        }
        self.decoded.insert(sound, Arc::clone(&samples));
        Some(samples)
    }
}

/// The cues the window has not played. The first frame plays none: those
/// sounds are from before the window opened.
fn new_cues(cues: &[WatchSound], last: &mut Option<u64>) -> Vec<WatchSound> {
    let newest = cues.iter().map(|c| c.seq).max();
    let fresh = match *last {
        None => Vec::new(),
        Some(played) => cues.iter().filter(|c| c.seq > played).copied().collect(),
    };
    *last = newest.or(*last).or(Some(0));
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cue(seq: u64) -> WatchSound {
        WatchSound {
            seq,
            sound: 0x023B,
            x: 0,
            y: 0,
        }
    }

    #[test]
    fn each_cue_plays_one_time_and_old_ones_do_not_play() {
        let mut last = None;
        assert!(new_cues(&[cue(1), cue(2)], &mut last).is_empty());
        assert_eq!(new_cues(&[cue(1), cue(2)], &mut last), vec![]);
        assert_eq!(
            new_cues(&[cue(2), cue(3), cue(4)], &mut last),
            vec![cue(3), cue(4)]
        );
        assert!(new_cues(&[], &mut last).is_empty());
        assert_eq!(new_cues(&[cue(5)], &mut last), vec![cue(5)]);
    }

    #[test]
    fn a_window_that_opens_in_silence_plays_the_first_sound() {
        let mut last = None;
        assert!(new_cues(&[], &mut last).is_empty());
        assert_eq!(new_cues(&[cue(1)], &mut last), vec![cue(1)]);
    }

    #[test]
    fn each_kind_is_under_the_master_and_mute_silences_all() {
        let mut settings = Settings {
            master: 0.5,
            music: 0.4,
            ..Settings::default()
        };
        assert!((settings.volume(Kind::Music) - 0.2).abs() < f32::EPSILON);
        settings.muted = true;
        assert_eq!(settings.volume(Kind::Effects), 0.0);
    }

    #[test]
    fn feet_take_turns_and_a_mount_that_walks_has_one_sound() {
        let step = |mounted, running| Step {
            tiles_away: 0.0,
            mounted,
            running,
        };
        assert_ne!(
            step_sound(step(false, false), 0),
            step_sound(step(false, false), 1)
        );
        assert_ne!(
            step_sound(step(true, true), 0),
            step_sound(step(true, true), 1)
        );
        assert_eq!(
            step_sound(step(true, false), 0),
            step_sound(step(true, false), 1)
        );
    }

    #[test]
    fn a_far_sound_is_quiet_and_a_sound_past_hearing_is_silent() {
        assert_eq!(nearness(0.0), 1.0);
        assert!(nearness(9.0) < 1.0 && nearness(9.0) > 0.0);
        assert_eq!(nearness(HEARING_TILES + 1.0), 0.0);
    }

    #[test]
    fn settings_come_back_from_the_file_and_a_bad_file_gives_the_defaults() {
        let dir = std::env::temp_dir().join(format!("uoterm-audio-{}", uuid::Uuid::new_v4()));
        let path = dir.join(SETTINGS_FILE);
        let settings = Settings {
            music: 0.1,
            muted: true,
            ..Settings::default()
        };
        kept::save_to(&path, &settings);
        assert_eq!(kept::load_from::<Settings>(&path), settings);
        std::fs::write(&path, "music = \"loud\"").unwrap();
        assert_eq!(kept::load_from::<Settings>(&path), Settings::default());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
