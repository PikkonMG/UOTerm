//! The sounds one client plays, as things for its sound device to do: the
//! music of the region and of war and death, the sound effects the shard
//! asks for, the footsteps, the rain, and how loud each plays now. The Rust
//! window plays them with its sound device and the browser with Web Audio;
//! both follow the rules here.

use super::{
    gain, nearness, new_cues, pick_combat_track, rain_sound, rain_volume, room, step_sound,
    EffectCue, Moment, Room, Score, Step, HEARD, UNHEARD,
};
use crate::frame::WatchFrame;
use crate::settings::{SoundKind, SoundOptions};
use serde::Serialize;
use std::path::PathBuf;
use uoterm_protocol::types::tile_distance;

/// The kinds whose volume the playing sounds follow.
const KINDS: [SoundKind; 3] = [SoundKind::Music, SoundKind::Effects, SoundKind::Footsteps];

/// One thing for the sound device to do.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum AudioOut {
    /// Play `sound` one time as the voice `voice`, `volume` loud. The voice
    /// `replace` stops first: the new sound takes its place.
    Effect {
        voice: u64,
        sound: u16,
        volume: f32,
        replace: Option<u64>,
    },
    /// Play the music track `track`, `volume` loud, in place of the music
    /// before. `sound_font` is the MIDI SoundFont the options name.
    Music {
        track: u16,
        volume: f32,
        sound_font: Option<PathBuf>,
    },
    /// Stop the music.
    MusicStop,
    /// Loop the rain `sound`, `volume` loud, in place of the rain before.
    Rain { sound: u16, volume: f32 },
    /// Stop the rain.
    RainStop,
    /// How loud each thing that plays is now: the options or the focus of
    /// the window changed.
    Volumes {
        voices: Vec<VoiceVolume>,
        music: f32,
        rain: f32,
    },
}

/// How loud one voice of the sound effects is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct VoiceVolume {
    pub voice: u64,
    pub volume: f32,
}

/// The music asked for last, with the SoundFont that plays a MIDI file.
#[derive(Clone, Debug, Default, PartialEq)]
struct MusicAsked {
    track: Option<u16>,
    sound_font: Option<PathBuf>,
}

/// What the volumes of the playing sounds follow: the focus gain and the
/// volume of each kind.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Loudness {
    gain: f32,
    kinds: [f32; KINDS.len()],
}

/// One sound effect that plays.
#[derive(Clone, Copy, Debug)]
struct Voice {
    id: u64,
    cue: EffectCue,
}

/// Follows the frames and gives what the sound device must do. The device
/// tells it when a voice ended, so the limit of sounds at once holds.
#[derive(Debug)]
pub struct Mixer {
    /// The number of the last sound cue played. None before the first frame.
    last_cue: Option<u64>,
    score: Score,
    steps_taken: usize,
    /// The sound effects that play, the oldest first.
    voices: Vec<Voice>,
    next_voice: u64,
    music: MusicAsked,
    rain: Option<u16>,
    /// The volume of all sound, by the window focus.
    gain: f32,
    /// What the volumes followed when they were last told.
    told: Option<Loudness>,
}

impl Default for Mixer {
    fn default() -> Self {
        Self {
            last_cue: None,
            score: Score::default(),
            steps_taken: 0,
            voices: Vec::new(),
            next_voice: 0,
            music: MusicAsked::default(),
            rain: None,
            gain: HEARD,
            told: None,
        }
    }
}

impl Mixer {
    /// What is new in this frame: the music, the rain and the sound cues
    /// of the shard. `focused` is true while the window has the keyboard;
    /// `combat_seed` gives a number by chance for the combat track.
    pub fn follow(
        &mut self,
        frame: &WatchFrame,
        options: &SoundOptions,
        focused: bool,
        combat_seed: impl FnOnce() -> u32,
    ) -> Vec<AudioOut> {
        let mut out = self.hear(focused, options);
        let moment = Moment {
            region: frame.music,
            war: frame.war,
            dead: frame.dead,
        };
        let music = self
            .score
            .follow(moment, options, || pick_combat_track(combat_seed()));
        self.follow_music(music, options, &mut out);
        self.follow_rain(frame.weather, options, &mut out);
        for cue in new_cues(&frame.sounds, &mut self.last_cue) {
            let tiles_away = tile_distance((cue.x, cue.y), (frame.x, frame.y)) as f32;
            self.start(cue.sound, SoundKind::Effects, tiles_away, options, &mut out);
        }
        out
    }

    /// The footsteps of the mobiles that stepped this frame.
    pub fn steps(&mut self, steps: &[Step], options: &SoundOptions) -> Vec<AudioOut> {
        let mut out = Vec::new();
        for step in steps {
            self.steps_taken += 1;
            let sound = step_sound(*step, self.steps_taken);
            self.start(
                sound,
                SoundKind::Footsteps,
                step.tiles_away,
                options,
                &mut out,
            );
        }
        out
    }

    /// A sound effect of the window itself, such as a container gump that
    /// opens, as loud as a sound at the character.
    pub fn effect(&mut self, sound: u16, options: &SoundOptions) -> Vec<AudioOut> {
        let mut out = Vec::new();
        self.start(sound, SoundKind::Effects, 0.0, options, &mut out);
        out
    }

    /// The voice `voice` came to its end, or could not play.
    pub fn ended(&mut self, voice: u64) {
        self.voices.retain(|playing| playing.id != voice);
    }

    /// Follows the focus of the window and the volumes of the options: the
    /// new volumes when either changed since they were last told.
    pub fn hear(&mut self, focused: bool, options: &SoundOptions) -> Vec<AudioOut> {
        self.gain = gain(focused, options.play_in_background);
        let loudness = Loudness {
            gain: self.gain,
            kinds: KINDS.map(|kind| options.volume(kind)),
        };
        let before = self.told.replace(loudness);
        if before.is_some_and(|before| before != loudness) {
            vec![self.volumes(options)]
        } else {
            Vec::new()
        }
    }

    /// How loud each thing that plays is now.
    pub fn volumes(&self, options: &SoundOptions) -> AudioOut {
        AudioOut::Volumes {
            voices: self
                .voices
                .iter()
                .map(|playing| VoiceVolume {
                    voice: playing.id,
                    volume: playing.cue.loudness(options) * self.gain,
                })
                .collect(),
            music: options.volume(SoundKind::Music) * self.gain,
            rain: rain_volume(options) * self.gain,
        }
    }

    fn follow_music(
        &mut self,
        track: Option<u16>,
        options: &SoundOptions,
        out: &mut Vec<AudioOut>,
    ) {
        let asked = MusicAsked {
            track: track.filter(|music| !options.filters_music(*music)),
            sound_font: options.midi_sound_font.clone(),
        };
        if asked == self.music {
            return;
        }
        let was_silent = self.music.track.is_none();
        self.music = asked;
        match self.music.track {
            Some(track) => out.push(AudioOut::Music {
                track,
                volume: options.volume(SoundKind::Music) * self.gain,
                sound_font: self.music.sound_font.clone(),
            }),
            None if !was_silent => out.push(AudioOut::MusicStop),
            None => {}
        }
    }

    fn follow_rain(
        &mut self,
        weather: Option<(u8, u8)>,
        options: &SoundOptions,
        out: &mut Vec<AudioOut>,
    ) {
        let wanted = rain_sound(weather, options);
        if wanted == self.rain {
            return;
        }
        self.rain = wanted;
        out.push(match wanted {
            Some(sound) => AudioOut::Rain {
                sound,
                volume: rain_volume(options) * self.gain,
            },
            None => AudioOut::RainStop,
        });
    }

    /// Starts `sound` when it may be heard and there is room for it. As in
    /// the reference client, a sound does not start again while it plays.
    fn start(
        &mut self,
        sound: u16,
        kind: SoundKind,
        tiles_away: f32,
        options: &SoundOptions,
        out: &mut Vec<AudioOut>,
    ) {
        let cue = EffectCue {
            sound,
            kind,
            nearness: nearness(tiles_away),
        };
        let loudness = cue.loudness(options);
        if self.gain <= UNHEARD
            || loudness <= 0.0
            || options.filters_sound(sound)
            || self.voices.iter().any(|playing| playing.cue.sound == sound)
        {
            return;
        }
        let playing: Vec<f32> = self
            .voices
            .iter()
            .map(|playing| playing.cue.loudness(options))
            .collect();
        let replace = match room(&playing, usize::from(options.max_sounds_at_once), loudness) {
            Room::Full => return,
            Room::InPlaceOf(index) => Some(self.voices.remove(index).id),
            Room::Free => None,
        };
        let voice = self.next_voice;
        self.next_voice += 1;
        self.voices.push(Voice { id: voice, cue });
        out.push(AudioOut::Effect {
            voice,
            sound,
            volume: loudness * self.gain,
            replace,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{WatchFrame, WatchSound};

    const SEED: u32 = 1;
    const SOUND: u16 = 0x2E;
    const OTHER_SOUND: u16 = 0x57;
    const REGION: u16 = 9;

    fn frame_with(cues: &[(u64, u16)]) -> WatchFrame {
        WatchFrame {
            sounds: cues
                .iter()
                .map(|&(seq, sound)| WatchSound {
                    seq,
                    sound,
                    x: 0,
                    y: 0,
                })
                .collect(),
            ..WatchFrame::default()
        }
    }

    fn effects(out: &[AudioOut]) -> Vec<u16> {
        out.iter()
            .filter_map(|out| match out {
                AudioOut::Effect { sound, .. } => Some(*sound),
                _ => None,
            })
            .collect()
    }

    fn follow(mixer: &mut Mixer, frame: &WatchFrame, options: &SoundOptions) -> Vec<AudioOut> {
        mixer.follow(frame, options, true, || SEED)
    }

    #[test]
    fn a_new_cue_starts_one_voice_and_the_first_frame_starts_none() {
        let options = SoundOptions::default();
        let mut mixer = Mixer::default();
        assert!(effects(&follow(&mut mixer, &frame_with(&[(1, SOUND)]), &options)).is_empty());
        let out = follow(
            &mut mixer,
            &frame_with(&[(1, SOUND), (2, OTHER_SOUND)]),
            &options,
        );
        assert_eq!(effects(&out), vec![OTHER_SOUND]);
        let volume = options.volume(SoundKind::Effects);
        assert!(out.contains(&AudioOut::Effect {
            voice: 0,
            sound: OTHER_SOUND,
            volume,
            replace: None,
        }));
    }

    #[test]
    fn a_sound_does_not_start_again_while_it_plays() {
        let options = SoundOptions::default();
        let mut mixer = Mixer::default();
        assert_eq!(effects(&mixer.effect(SOUND, &options)), vec![SOUND]);
        assert!(effects(&mixer.effect(SOUND, &options)).is_empty());
        mixer.ended(0);
        assert_eq!(effects(&mixer.effect(SOUND, &options)), vec![SOUND]);
    }

    #[test]
    fn at_the_limit_a_new_sound_takes_the_place_of_the_quietest() {
        let options = SoundOptions {
            max_sounds_at_once: 1,
            ..SoundOptions::default()
        };
        let mut mixer = Mixer::default();
        let far = Step {
            tiles_away: 9.0,
            mounted: false,
            running: false,
        };
        let first = mixer.steps(&[far], &options);
        let Some(AudioOut::Effect { voice, .. }) = first.first() else {
            panic!("a step plays: {first:?}");
        };
        let out = mixer.effect(OTHER_SOUND, &options);
        assert!(matches!(
            out.as_slice(),
            [AudioOut::Effect { replace: Some(replaced), .. }] if replaced == voice
        ));
    }

    #[test]
    fn filtered_and_silent_sounds_start_no_voice() {
        let filtered = SoundOptions {
            sound_filter: vec![SOUND],
            ..SoundOptions::default()
        };
        let mut mixer = Mixer::default();
        assert!(mixer.effect(SOUND, &filtered).is_empty());
        let muted = SoundOptions {
            muted: true,
            ..SoundOptions::default()
        };
        assert!(mixer.effect(SOUND, &muted).is_empty());
        let unheard = SoundOptions::default();
        mixer.hear(false, &unheard);
        assert!(mixer.effect(SOUND, &unheard).is_empty());
    }

    #[test]
    fn steps_take_turns_and_grow_quiet_with_distance() {
        let options = SoundOptions::default();
        let mut mixer = Mixer::default();
        let step = |tiles_away| Step {
            tiles_away,
            mounted: false,
            running: false,
        };
        let near = mixer.steps(&[step(0.0)], &options);
        mixer.ended(0);
        let far = mixer.steps(&[step(9.0)], &options);
        let (
            [AudioOut::Effect {
                sound: one,
                volume: loud,
                ..
            }],
            [AudioOut::Effect {
                sound: two,
                volume: quiet,
                ..
            }],
        ) = (near.as_slice(), far.as_slice())
        else {
            panic!("each step plays: {near:?} {far:?}");
        };
        assert_ne!(one, two);
        assert!(quiet < loud);
    }

    #[test]
    fn region_music_starts_once_and_silence_stops_it() {
        let options = SoundOptions::default();
        let mut mixer = Mixer::default();
        let mut frame = WatchFrame {
            music: Some(REGION),
            ..WatchFrame::default()
        };
        let music = AudioOut::Music {
            track: REGION,
            volume: options.volume(SoundKind::Music),
            sound_font: None,
        };
        assert_eq!(follow(&mut mixer, &frame, &options), vec![music]);
        assert!(follow(&mut mixer, &frame, &options).is_empty());
        frame.music = None;
        assert_eq!(
            follow(&mut mixer, &frame, &options),
            vec![AudioOut::MusicStop]
        );
    }

    #[test]
    fn a_filtered_track_plays_nothing() {
        let options = SoundOptions {
            music_filter: vec![REGION],
            ..SoundOptions::default()
        };
        let mut mixer = Mixer::default();
        let frame = WatchFrame {
            music: Some(REGION),
            ..WatchFrame::default()
        };
        assert!(follow(&mut mixer, &frame, &options).is_empty());
    }

    #[test]
    fn a_new_sound_font_asks_for_the_track_again() {
        let mut options = SoundOptions::default();
        let mut mixer = Mixer::default();
        let frame = WatchFrame {
            music: Some(REGION),
            ..WatchFrame::default()
        };
        follow(&mut mixer, &frame, &options);
        options.midi_sound_font = Some("font.sf2".into());
        let out = follow(&mut mixer, &frame, &options);
        assert!(matches!(
            out.as_slice(),
            [AudioOut::Music {
                track: REGION,
                sound_font: Some(_),
                ..
            }]
        ));
    }

    #[test]
    fn rain_loops_while_it_rains_and_stops_after() {
        let options = SoundOptions::default();
        let mut mixer = Mixer::default();
        let mut frame = WatchFrame {
            weather: Some((0, 1)),
            ..WatchFrame::default()
        };
        let out = follow(&mut mixer, &frame, &options);
        assert!(matches!(out.as_slice(), [AudioOut::Rain { .. }]));
        assert!(follow(&mut mixer, &frame, &options).is_empty());
        frame.weather = None;
        assert_eq!(
            follow(&mut mixer, &frame, &options),
            vec![AudioOut::RainStop]
        );
    }

    #[test]
    fn losing_the_focus_silences_what_plays_unless_it_plays_in_the_background() {
        let options = SoundOptions::default();
        let mut mixer = Mixer::default();
        mixer.effect(SOUND, &options);
        assert!(mixer.hear(true, &options).is_empty());
        let out = mixer.hear(false, &options);
        let [AudioOut::Volumes {
            voices,
            music,
            rain,
        }] = out.as_slice()
        else {
            panic!("the volumes change: {out:?}");
        };
        assert_eq!(
            voices,
            &vec![VoiceVolume {
                voice: 0,
                volume: UNHEARD
            }]
        );
        assert_eq!((*music, *rain), (UNHEARD, UNHEARD));
        assert!(mixer.hear(false, &options).is_empty());
        let background = SoundOptions {
            play_in_background: true,
            ..SoundOptions::default()
        };
        assert_eq!(mixer.hear(false, &background).len(), 1);
    }

    #[test]
    fn a_new_volume_of_the_options_tells_the_volumes_again() {
        let mut options = SoundOptions::default();
        let mut mixer = Mixer::default();
        mixer.hear(true, &options);
        options.music_volume /= 2.0;
        let out = mixer.hear(true, &options);
        assert_eq!(out, vec![mixer.volumes(&options)]);
    }
}
