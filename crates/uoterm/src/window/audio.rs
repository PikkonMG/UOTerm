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

use super::settings::SoundOptions;
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
use uoterm_view::audio::{music_file, AudioOut, Mixer, MusicFile};

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

/// The music that plays: its number, and the SoundFont when the track has
/// a MIDI file. A new SoundFont for a track with no MIDI file plays on.
#[derive(Clone, Debug, PartialEq, Eq)]
struct MusicPlaying {
    number: u16,
    sound_font: Option<PathBuf>,
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
    mixer: Mixer,
    music: Option<MusicPlaying>,
    sound_fonts: SoundFonts,
    effects: Effects,
    /// The loop of the rain that plays. None when the files lack it.
    rain: Option<Player>,
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
            mixer: Mixer::default(),
            music: None,
            sound_fonts: SoundFonts::default(),
            effects: Effects::default(),
            rain: None,
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
        for voice in self.effects.take_ended() {
            self.mixer.ended(voice);
        }
        let combat_seed = rand::random;
        let mut outs = self.mixer.follow(frame, options, focused, combat_seed);
        outs.extend(self.mixer.steps(steps, options));
        self.carry_out(outs, options);
    }

    /// Plays a sound effect of the window itself, such as a container gump
    /// that opens, as loud as a sound at the character.
    pub fn play_effect(&mut self, sound: u16, options: &SoundOptions) {
        if self.device.is_some() {
            let outs = self.mixer.effect(sound, options);
            self.carry_out(outs, options);
        }
    }

    /// Call this when the player changed the sound options, so what plays
    /// follows at once.
    pub fn options_changed(&mut self, options: &SoundOptions) {
        if self.device.is_some() {
            let volumes = self.mixer.volumes(options);
            self.carry_out(vec![volumes], options);
        }
    }

    /// Does what the mixer asks of the sound device.
    fn carry_out(&mut self, outs: Vec<AudioOut>, options: &SoundOptions) {
        for out in outs {
            match out {
                AudioOut::Effect {
                    voice,
                    sound,
                    volume,
                    replace,
                } => {
                    if let Some(replaced) = replace {
                        self.effects.stop(replaced);
                    }
                    match (self.samples(sound), &self.device) {
                        (Some(samples), Some(device)) => {
                            self.effects
                                .start(device.sink.mixer(), voice, &samples, volume);
                        }
                        _ => self.mixer.ended(voice),
                    }
                }
                AudioOut::Music {
                    track,
                    volume,
                    sound_font,
                } => self.start_music(track, volume, sound_font, options),
                AudioOut::MusicStop => {
                    self.music = None;
                    self.music_note = "";
                    if let Some(device) = &self.device {
                        device.music.stop();
                    }
                }
                AudioOut::Rain { sound, volume } => self.start_rain(sound, volume),
                // The old loop stops when its player drops.
                AudioOut::RainStop => self.rain = None,
                AudioOut::Volumes {
                    voices,
                    music,
                    rain,
                } => {
                    self.effects.set_volumes(&voices);
                    if let Some(device) = &self.device {
                        device.music.set_volume(music);
                    }
                    if let Some(player) = &self.rain {
                        player.set_volume(rain);
                    }
                }
            }
        }
    }

    fn start_music(
        &mut self,
        number: u16,
        volume: f32,
        sound_font: Option<PathBuf>,
        options: &SoundOptions,
    ) {
        let track = self
            .music_list
            .as_ref()
            .and_then(|list| list.track(number))
            .cloned();
        let has_midi = track.as_ref().is_some_and(|track| track.midi.is_some());
        let playing = MusicPlaying {
            number,
            sound_font: sound_font.filter(|_| has_midi),
        };
        let Some(device) = &self.device else {
            return;
        };
        device.music.set_volume(volume);
        if self.music.as_ref() == Some(&playing) {
            return;
        }
        self.music_note = "";
        device.music.stop();
        let font = playing
            .sound_font
            .as_deref()
            .and_then(|path| self.sound_fonts.get(path));
        self.music = Some(playing);
        let Some(track) = track else {
            return;
        };
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

    fn start_rain(&mut self, sound: u16, volume: f32) {
        let samples = self.samples(sound);
        self.rain = samples.zip(self.device.as_ref()).map(|(samples, device)| {
            let player = Player::connect_new(device.sink.mixer());
            player.set_volume(volume);
            let source = SamplesBuffer::new(ONE_CHANNEL, SAMPLE_RATE, samples.to_vec());
            player.append(source.repeat_infinite());
            player
        });
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
