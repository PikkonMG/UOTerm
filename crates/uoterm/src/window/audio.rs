//! The sound of the window: the music of the region, the music of war and
//! of death, the sound effects the shard asks for, the footsteps of the
//! mobiles near, and the rain. The volumes are the Sound page of the
//! profile, and the rules are `uoterm_view::audio`.
//!
//! The sounds come from the client files. With no client files, or with no
//! sound device, the window is silent and the rest of it works the same.

mod effects;
mod midi;

pub use uoterm_view::audio::Step;

use super::settings::{SoundKind, SoundOptions};
use crate::view::WatchFrame;
use effects::{Effects, ONE_CHANNEL, SAMPLE_RATE};
use midi::{MidiSource, SoundFonts};
use rodio::buffer::SamplesBuffer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uoterm_nav::{MusicList, SoundData};
use uoterm_view::audio::{
    gain, music_file, nearness, new_cues, pick_combat_track, rain_sound, rain_volume, step_sound,
    EffectCue, Moment, MusicFile, Score, HEARD, UNHEARD,
};

const SAMPLE_FULL_SCALE: f32 = 32_768.0;
/// The window keeps this many decoded sounds. When full, it starts again.
const SOUND_CACHE_CAP: usize = 128;
pub const NOTE_NO_FILES: &str = "No sound: the client files hold no sounds.";
pub const NOTE_NO_DEVICE: &str = "No sound: this computer gave no sound device.";
pub const NOTE_NEEDS_SOUND_FONT: &str =
    "No music: this client has MIDI music only. Set a MIDI SoundFont file.";
pub const NOTE_BAD_SOUND_FONT: &str =
    "No music: the MIDI SoundFont file or the MIDI file could not be read.";

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
        let combat_music = || pick_combat_track(rand::random());
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
