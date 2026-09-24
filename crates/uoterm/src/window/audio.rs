//! The sound of the window: the music of the region, the music of war and
//! of death, the sound effects the shard asks for, the footsteps of the
//! mobiles near, and the rain. The volumes and the rules are the Sound page
//! of the profile.
//!
//! The sounds come from the client files. With no client files, or with no
//! sound device, the window is silent and the rest of it works the same.

mod effects;
mod midi;
mod score;

use super::settings::{SoundKind, SoundOptions};
use crate::view::{WatchFrame, WatchSound};
use effects::{EffectCue, Effects, ONE_CHANNEL, SAMPLE_RATE};
use midi::{music_file, MidiSource, MusicFile, SoundFonts};
use rand::Rng;
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use score::{Moment, Score, COMBAT_MUSIC};
use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uoterm_nav::{MusicList, SoundData};

const SAMPLE_FULL_SCALE: f32 = 32_768.0;
/// A sound this many tiles away is silent. Nearer sounds are louder.
const HEARING_TILES: f32 = 18.0;
/// The window keeps this many decoded sounds. When full, it starts again.
const SOUND_CACHE_CAP: usize = 128;
/// A person on foot makes these two sounds in turn, and a mount that runs
/// makes these two. A mount that walks makes the first foot sound only.
const STEPS_ON_FOOT: [u16; 2] = [0x012B, 0x012C];
const STEPS_MOUNT_RUN: [u16; 2] = [0x0129, 0x012A];
/// The weather kinds with rain: rain, and a fierce storm.
const WEATHER_RAIN: u8 = 0;
const WEATHER_FIERCE_STORM: u8 = 1;
/// Rain of more drops than this is heavy.
const HEAVY_RAIN_DROPS: u8 = 30;
/// The water loops of the client files that sound as rain.
const HEAVY_RAIN_SOUND: u16 = 0x0010;
const LIGHT_RAIN_SOUND: u16 = 0x0011;
/// The rain plays this much as loud as the sound effects.
const RAIN_LOUDNESS: f32 = 0.2;
/// The volume of all sound while the window may be heard, and while not.
const HEARD: f32 = 1.0;
const UNHEARD: f32 = 0.0;

pub const NOTE_NO_FILES: &str = "No sound: the client files hold no sounds.";
pub const NOTE_NO_DEVICE: &str = "No sound: this computer gave no sound device.";
pub const NOTE_NEEDS_SOUND_FONT: &str =
    "No music: this client has MIDI music only. Set a MIDI SoundFont file.";
pub const NOTE_BAD_SOUND_FONT: &str =
    "No music: the MIDI SoundFont file or the MIDI file could not be read.";

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

/// The volume of all sound: a window out of focus is silent, unless the
/// player wants to hear it in the background.
fn gain(focused: bool, play_in_background: bool) -> f32 {
    if focused || play_in_background {
        HEARD
    } else {
        UNHEARD
    }
}

/// The rain loop the weather asks for, when the player wants rain.
fn rain_sound(weather: Option<(u8, u8)>, options: &SoundOptions) -> Option<u16> {
    let (kind, drops) = weather.filter(|_| options.rain_sound)?;
    let rains = kind == WEATHER_RAIN || kind == WEATHER_FIERCE_STORM;
    let sound = if drops > HEAVY_RAIN_DROPS {
        HEAVY_RAIN_SOUND
    } else {
        LIGHT_RAIN_SOUND
    };
    (rains && drops > 0 && !options.filters_sound(sound)).then_some(sound)
}

fn rain_volume(options: &SoundOptions) -> f32 {
    options.volume(SoundKind::Effects) * RAIN_LOUDNESS
}

struct Device {
    sink: MixerDeviceSink,
    music: Player,
}

/// The music asked for last: its number, its volume, and the SoundFont
/// setting when the track has a MIDI file. The same ask plays on.
#[derive(Clone, Debug, PartialEq, Eq)]
struct MusicAsked {
    number: Option<u16>,
    kind: SoundKind,
    sound_font: Option<PathBuf>,
}

/// The rain that plays, and its loop. No loop when the files lack it.
struct Rain {
    sound: u16,
    player: Option<Player>,
}

pub struct Audio {
    /// Why there is no sound. Empty when there is sound.
    note: &'static str,
    /// Why the music asked for does not play. Empty when it plays.
    music_note: &'static str,
    device: Option<Device>,
    sounds: Option<SoundData>,
    music_list: Option<MusicList>,
    decoded: HashMap<u16, Arc<[f32]>>,
    /// The number of the last sound cue played. None before the first frame.
    last_cue: Option<u64>,
    music: Option<MusicAsked>,
    score: Score,
    sound_fonts: SoundFonts,
    effects: Effects,
    rain: Option<Rain>,
    steps_taken: usize,
    /// The volume of all sound, by the window focus.
    gain: f32,
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
            note,
            music_note: "",
            device,
            sounds,
            music_list: uopath.and_then(|dir| MusicList::open(dir).ok()),
            decoded: HashMap::new(),
            last_cue: None,
            music: None,
            score: Score::default(),
            sound_fonts: SoundFonts::default(),
            effects: Effects::default(),
            rain: None,
            steps_taken: 0,
            gain: HEARD,
        }
    }

    /// Why there is no sound, or no music. Empty when all plays.
    pub fn note(&self) -> &str {
        if self.note.is_empty() {
            self.music_note
        } else {
            self.note
        }
    }

    /// Plays what is new in this frame, as loud as `options` says.
    /// `focused` is true while the window has the keyboard.
    pub fn play(
        &mut self,
        frame: &WatchFrame,
        steps: &[Step],
        options: &SoundOptions,
        focused: bool,
    ) {
        if self.device.is_none() {
            return;
        }
        self.follow_focus(focused, options);
        let moment = Moment {
            region: frame.music,
            war: frame.war,
            dead: frame.dead,
        };
        let combat_music = || rand::thread_rng().gen_range(COMBAT_MUSIC);
        let music = self.score.follow(moment, options, combat_music);
        self.follow_music(music, SoundKind::Music, options);
        self.follow_rain(frame.weather, options);
        for cue in new_cues(&frame.sounds, &mut self.last_cue) {
            let tiles_away =
                uoterm_protocol::types::tile_distance((cue.x, cue.y), (frame.x, frame.y)) as f32;
            self.play_sound(cue.sound, SoundKind::Effects, tiles_away, options);
        }
        for step in steps {
            self.steps_taken += 1;
            let sound = step_sound(*step, self.steps_taken);
            self.play_sound(sound, SoundKind::Footsteps, step.tiles_away, options);
        }
    }

    /// Plays a sound effect of the window itself, such as a container gump
    /// that opens, as loud as a sound at the character.
    pub fn play_effect(&mut self, sound: u16, options: &SoundOptions) {
        if self.device.is_some() {
            self.play_sound(sound, SoundKind::Effects, 0.0, options);
        }
    }

    /// Call this when the player changed the sound options, so what plays
    /// follows at once.
    pub fn options_changed(&self, options: &SoundOptions) {
        let Some(device) = &self.device else {
            return;
        };
        if let Some(music) = &self.music {
            device
                .music
                .set_volume(options.volume(music.kind) * self.gain);
        }
        self.effects.set_volumes(options, self.gain);
        if let Some(player) = self.rain.as_ref().and_then(|rain| rain.player.as_ref()) {
            player.set_volume(rain_volume(options) * self.gain);
        }
    }

    fn follow_focus(&mut self, focused: bool, options: &SoundOptions) {
        let gain = gain(focused, options.play_in_background);
        if gain != self.gain {
            self.gain = gain;
            self.options_changed(options);
        }
    }

    fn follow_music(&mut self, number: Option<u16>, kind: SoundKind, options: &SoundOptions) {
        let number = number.filter(|music| !options.filters_music(*music));
        let track = number
            .and_then(|music| self.music_list.as_ref()?.track(music))
            .cloned();
        let has_midi = track.as_ref().is_some_and(|track| track.midi.is_some());
        let asked = MusicAsked {
            number,
            kind,
            sound_font: options.midi_sound_font.clone().filter(|_| has_midi),
        };
        if self.music.as_ref() == Some(&asked) {
            return;
        }
        self.music = Some(asked);
        self.music_note = "";
        let Some(device) = &self.device else {
            return;
        };
        device.music.stop();
        let Some(track) = track else {
            return;
        };
        device.music.set_volume(options.volume(kind) * self.gain);
        let font = options
            .midi_sound_font
            .as_deref()
            .filter(|_| has_midi)
            .and_then(|path| self.sound_fonts.get(path));
        match music_file(&track, font.is_some()) {
            Some(MusicFile::Mp3(path)) => {
                let Ok(file) = std::fs::File::open(path) else {
                    return;
                };
                let reader = BufReader::new(file);
                if track.repeats {
                    if let Ok(source) = Decoder::new_looped(reader) {
                        device.music.append(source);
                    }
                } else if let Ok(source) = Decoder::new(reader) {
                    device.music.append(source);
                }
            }
            Some(MusicFile::Midi(path)) => {
                match font.and_then(|font| MidiSource::open(path, &font, track.repeats)) {
                    Some(source) => device.music.append(source),
                    None => self.music_note = NOTE_BAD_SOUND_FONT,
                }
            }
            None if options.midi_sound_font.is_some() => self.music_note = NOTE_BAD_SOUND_FONT,
            None => self.music_note = NOTE_NEEDS_SOUND_FONT,
        }
        device.music.play();
    }

    fn follow_rain(&mut self, weather: Option<(u8, u8)>, options: &SoundOptions) {
        let wanted = rain_sound(weather, options);
        if self.rain.as_ref().map(|rain| rain.sound) == wanted {
            return;
        }
        // The old loop stops when its player drops.
        self.rain = None;
        let Some(sound) = wanted else {
            return;
        };
        let samples = self.samples(sound);
        let player = samples.zip(self.device.as_ref()).map(|(samples, device)| {
            let player = Player::connect_new(device.sink.mixer());
            player.set_volume(rain_volume(options) * self.gain);
            let source = SamplesBuffer::new(ONE_CHANNEL, SAMPLE_RATE, samples.to_vec());
            player.append(source.repeat_infinite());
            player
        });
        self.rain = Some(Rain { sound, player });
    }

    fn play_sound(&mut self, sound: u16, kind: SoundKind, tiles_away: f32, options: &SoundOptions) {
        let cue = EffectCue {
            sound,
            kind,
            nearness: nearness(tiles_away),
        };
        if self.gain <= UNHEARD || cue.loudness(options) <= 0.0 || options.filters_sound(sound) {
            return;
        }
        let Some(samples) = self.samples(sound) else {
            return;
        };
        if let Some(device) = &self.device {
            self.effects
                .start(device.sink.mixer(), cue, &samples, options, self.gain);
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
    fn a_window_out_of_focus_is_silent_unless_set_to_play_in_the_background() {
        assert_eq!(gain(true, false), HEARD);
        assert_eq!(gain(true, true), HEARD);
        assert_eq!(gain(false, true), HEARD);
        assert_eq!(gain(false, false), UNHEARD);
    }

    #[test]
    fn rain_and_fierce_storms_loop_the_rain_by_its_drops() {
        const SNOW: u8 = 2;
        let options = SoundOptions::default();
        let heavy = Some((WEATHER_RAIN, HEAVY_RAIN_DROPS + 1));
        assert_eq!(rain_sound(heavy, &options), Some(HEAVY_RAIN_SOUND));
        let light = Some((WEATHER_FIERCE_STORM, HEAVY_RAIN_DROPS));
        assert_eq!(rain_sound(light, &options), Some(LIGHT_RAIN_SOUND));
        assert_eq!(rain_sound(Some((WEATHER_RAIN, 0)), &options), None);
        assert_eq!(rain_sound(Some((SNOW, HEAVY_RAIN_DROPS)), &options), None);
        assert_eq!(rain_sound(None, &options), None);
        let no_rain = SoundOptions {
            rain_sound: false,
            ..SoundOptions::default()
        };
        assert_eq!(rain_sound(heavy, &no_rain), None);
        let filtered = SoundOptions {
            sound_filter: vec![HEAVY_RAIN_SOUND],
            ..SoundOptions::default()
        };
        assert_eq!(rain_sound(heavy, &filtered), None);
    }
}
