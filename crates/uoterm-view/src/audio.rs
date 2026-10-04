//! The rules of the sound of the window: which music plays, the footsteps
//! of the mobiles near, how loud a sound arrives from afar, the rain, and
//! which sound effects play when too many ask at once. The rules are the
//! reference client's; the window plays the sounds.

use crate::frame::WatchSound;
use crate::settings::{SoundKind, SoundOptions};
use std::ops::RangeInclusive;
use std::path::Path;
use uoterm_nav::MusicTrack;

/// A sound this many tiles away is silent. Nearer sounds are louder.
const HEARING_TILES: f32 = 18.0;
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
pub const HEARD: f32 = 1.0;
pub const UNHEARD: f32 = 0.0;

/// One step a mobile took this frame, for the footstep sound.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub tiles_away: f32,
    pub mounted: bool,
    pub running: bool,
}

/// The sound of one step. `count` is how many steps the window has played.
pub fn step_sound(step: Step, count: usize) -> u16 {
    match (step.mounted, step.running) {
        (true, true) => STEPS_MOUNT_RUN[count % STEPS_MOUNT_RUN.len()],
        (true, false) => STEPS_ON_FOOT[0],
        (false, _) => STEPS_ON_FOOT[count % STEPS_ON_FOOT.len()],
    }
}

/// How much of a sound arrives from `tiles_away`.
pub fn nearness(tiles_away: f32) -> f32 {
    (1.0 - tiles_away / HEARING_TILES).clamp(0.0, 1.0)
}

/// The volume of all sound: a window out of focus is silent, unless the
/// player wants to hear it in the background.
pub fn gain(focused: bool, play_in_background: bool) -> f32 {
    if focused || play_in_background {
        HEARD
    } else {
        UNHEARD
    }
}

/// The rain loop the weather asks for, when the player wants rain.
pub fn rain_sound(weather: Option<(u8, u8)>, options: &SoundOptions) -> Option<u16> {
    let (kind, drops) = weather.filter(|_| options.rain_sound)?;
    let rains = kind == WEATHER_RAIN || kind == WEATHER_FIERCE_STORM;
    let sound = if drops > HEAVY_RAIN_DROPS {
        HEAVY_RAIN_SOUND
    } else {
        LIGHT_RAIN_SOUND
    };
    (rains && drops > 0 && !options.filters_sound(sound)).then_some(sound)
}

/// How loud the rain plays.
pub fn rain_volume(options: &SoundOptions) -> f32 {
    options.volume(SoundKind::Effects) * RAIN_LOUDNESS
}

/// One sound effect to play: which, its kind, and how much of it arrives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectCue {
    pub sound: u16,
    pub kind: SoundKind,
    /// 1 at the character, and less with distance.
    pub nearness: f32,
}

impl EffectCue {
    /// How loud the sound is with these options, before the window focus.
    pub fn loudness(&self, options: &SoundOptions) -> f32 {
        options.volume(self.kind) * self.nearness
    }
}

/// Where a new sound goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Room {
    Free,
    /// In the place of the playing sound at this index.
    InPlaceOf(usize),
    /// Nowhere: the new sound does not play.
    Full,
}

/// Where a new sound of `loudness` goes, when sounds of `playing` loudness
/// play and at most `max` may. Of sounds equally quiet, the oldest goes.
pub fn room(playing: &[f32], max: usize, loudness: f32) -> Room {
    if playing.len() < max {
        return Room::Free;
    }
    let quietest = playing
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.total_cmp(b));
    match quietest {
        Some((index, &quiet)) if quiet <= loudness => Room::InPlaceOf(index),
        _ => Room::Full,
    }
}

/// The file of a track that plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicFile<'a> {
    Mp3(&'a Path),
    Midi(&'a Path),
}

/// The file of `track` to play. A MIDI file plays only with a SoundFont.
/// When `Config.txt` names the MIDI file, it plays before the MP3 file;
/// else the MP3 file plays first.
pub fn music_file(track: &MusicTrack, has_sound_font: bool) -> Option<MusicFile<'_>> {
    let midi = track
        .midi
        .as_deref()
        .filter(|_| has_sound_font)
        .map(MusicFile::Midi);
    let mp3 = track.mp3.as_deref().map(MusicFile::Mp3);
    if track.midi_first {
        midi.or(mp3)
    } else {
        mp3.or(midi)
    }
}

/// War mode plays one of these, by chance.
pub const COMBAT_MUSIC: RangeInclusive<u16> = 38..=40;
/// The music of the death screen.
pub const DEATH_MUSIC: u16 = 42;

/// What one frame says of the character, for the music.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Moment {
    /// The music the shard asked for. None for silence.
    pub region: Option<u16>,
    pub war: bool,
    pub dead: bool,
}

/// Follows the frames, and gives the music to play.
#[derive(Debug, Default)]
pub struct Score {
    last: Option<Moment>,
    /// The music of a fight or of a death, which plays over the region.
    over_region: Option<u16>,
}

impl Score {
    /// The music to play now. The first frame only starts the region
    /// music: a war or a death from before the window is old news.
    ///
    /// New region music ends the music over it. A death plays the death
    /// music, and war mode plays combat music; both need the combat music
    /// option, and war also needs music on. Peace, or life again, brings the
    /// region music back. `combat_music` picks the combat track.
    pub fn follow(
        &mut self,
        now: Moment,
        options: &SoundOptions,
        combat_music: impl FnOnce() -> u16,
    ) -> Option<u16> {
        if let Some(before) = self.last {
            if now.region != before.region {
                self.over_region = None;
            }
            if now.dead != before.dead {
                self.over_region = (now.dead && options.combat_music).then_some(DEATH_MUSIC);
            } else if !now.dead && now.war != before.war {
                let fights = now.war && options.combat_music && options.music_on;
                self.over_region = fights.then(combat_music);
            }
        }
        self.last = Some(now);
        self.over_region.or(now.region)
    }
}

/// The combat track for a number the caller picks by chance.
pub fn pick_combat_track(seed: u32) -> u16 {
    let first = *COMBAT_MUSIC.start();
    let count = u32::from(*COMBAT_MUSIC.end() - first) + 1;
    first + (seed % count) as u16
}

/// The cues the window has not played. The first frame plays none: those
/// sounds are from before the window opened.
pub fn new_cues(cues: &[WatchSound], last: &mut Option<u64>) -> Vec<WatchSound> {
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
    use std::path::PathBuf;

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

    #[test]
    fn a_sound_plays_free_below_the_limit() {
        assert_eq!(room(&[], 2, 0.1), Room::Free);
        assert_eq!(room(&[0.5], 2, 0.1), Room::Free);
    }

    #[test]
    fn at_the_limit_a_sound_takes_the_place_of_the_quietest_or_the_oldest() {
        assert_eq!(room(&[0.5, 0.2, 0.9], 3, 0.4), Room::InPlaceOf(1));
        assert_eq!(room(&[0.3, 0.3, 0.9], 3, 0.3), Room::InPlaceOf(0));
        assert_eq!(room(&[0.5, 0.2, 0.9], 3, 0.1), Room::Full);
    }

    #[test]
    fn with_no_room_at_all_nothing_plays() {
        assert_eq!(room(&[], 0, 1.0), Room::Full);
    }

    #[test]
    fn loudness_is_the_kind_volume_by_nearness() {
        let options = SoundOptions {
            master_volume: 1.0,
            sound_volume: 0.5,
            ..SoundOptions::default()
        };
        let cue = EffectCue {
            sound: 1,
            kind: SoundKind::Effects,
            nearness: 0.5,
        };
        assert!((cue.loudness(&options) - 0.25).abs() < f32::EPSILON);
    }

    fn track(mp3: bool, midi: bool, midi_first: bool) -> MusicTrack {
        MusicTrack {
            mp3: mp3.then(|| PathBuf::from("a.mp3")),
            midi: midi.then(|| PathBuf::from("a.mid")),
            midi_first,
            repeats: false,
        }
    }

    #[test]
    fn midi_plays_only_with_a_sound_font_and_mp3_is_the_fallback() {
        let mp3 = Some(MusicFile::Mp3(Path::new("a.mp3")));
        let midi = Some(MusicFile::Midi(Path::new("a.mid")));
        assert_eq!(music_file(&track(true, true, true), true), midi);
        assert_eq!(music_file(&track(true, true, true), false), mp3);
        assert_eq!(music_file(&track(true, true, false), true), mp3);
        assert_eq!(music_file(&track(false, true, false), true), midi);
        assert_eq!(music_file(&track(false, true, false), false), None);
        assert_eq!(music_file(&track(true, false, true), true), mp3);
    }

    #[test]
    fn every_seed_picks_a_combat_track() {
        let picked: Vec<u16> = (0..6).map(pick_combat_track).collect();
        assert!(picked.iter().all(|track| COMBAT_MUSIC.contains(track)));
        for track in COMBAT_MUSIC {
            assert!(picked.contains(&track), "track {track} can play");
        }
    }

    const REGION: u16 = 9;
    const OTHER_REGION: u16 = 12;
    const COMBAT: u16 = 39;

    fn moment(war: bool, dead: bool) -> Moment {
        Moment {
            region: Some(REGION),
            war,
            dead,
        }
    }

    fn follow(score: &mut Score, now: Moment, options: &SoundOptions) -> Option<u16> {
        score.follow(now, options, || COMBAT)
    }

    #[test]
    fn war_plays_combat_music_and_peace_brings_the_region_back() {
        let options = SoundOptions::default();
        let mut score = Score::default();
        assert_eq!(
            follow(&mut score, moment(false, false), &options),
            Some(REGION)
        );
        assert_eq!(
            follow(&mut score, moment(true, false), &options),
            Some(COMBAT)
        );
        assert_eq!(
            follow(&mut score, moment(true, false), &options),
            Some(COMBAT)
        );
        assert_eq!(
            follow(&mut score, moment(false, false), &options),
            Some(REGION)
        );
    }

    #[test]
    fn a_war_from_before_the_window_plays_no_combat_music() {
        let options = SoundOptions::default();
        let mut score = Score::default();
        assert_eq!(
            follow(&mut score, moment(true, true), &options),
            Some(REGION)
        );
        assert_eq!(
            follow(&mut score, moment(true, true), &options),
            Some(REGION)
        );
    }

    #[test]
    fn combat_music_needs_its_option_and_music_on() {
        for options in [
            SoundOptions {
                combat_music: false,
                ..SoundOptions::default()
            },
            SoundOptions {
                music_on: false,
                ..SoundOptions::default()
            },
        ] {
            let mut score = Score::default();
            follow(&mut score, moment(false, false), &options);
            assert_eq!(
                follow(&mut score, moment(true, false), &options),
                Some(REGION)
            );
        }
    }

    #[test]
    fn a_death_plays_the_death_music_until_life_or_new_region_music() {
        let options = SoundOptions::default();
        let mut score = Score::default();
        follow(&mut score, moment(true, false), &options);
        // The death screen ends war mode in the same frame.
        let died = follow(&mut score, moment(false, true), &options);
        assert_eq!(died, Some(DEATH_MUSIC));
        assert_eq!(
            follow(&mut score, moment(false, true), &options),
            Some(DEATH_MUSIC)
        );
        assert_eq!(
            follow(&mut score, moment(false, false), &options),
            Some(REGION)
        );
        follow(&mut score, moment(false, true), &options);
        let moved = Moment {
            region: Some(OTHER_REGION),
            ..moment(false, true)
        };
        assert_eq!(follow(&mut score, moved, &options), Some(OTHER_REGION));
    }

    #[test]
    fn the_death_music_needs_the_combat_music_option() {
        let options = SoundOptions {
            combat_music: false,
            ..SoundOptions::default()
        };
        let mut score = Score::default();
        follow(&mut score, moment(false, false), &options);
        assert_eq!(
            follow(&mut score, moment(false, true), &options),
            Some(REGION)
        );
    }

    #[test]
    fn new_region_music_ends_the_combat_music() {
        let options = SoundOptions::default();
        let mut score = Score::default();
        follow(&mut score, moment(false, false), &options);
        follow(&mut score, moment(true, false), &options);
        let moved = Moment {
            region: Some(OTHER_REGION),
            ..moment(true, false)
        };
        assert_eq!(follow(&mut score, moved, &options), Some(OTHER_REGION));
        let silence = Moment {
            region: None,
            ..moved
        };
        assert_eq!(follow(&mut score, silence, &options), None);
    }
}
