//! The sound effects that play now. The Sound page limits how many play at
//! once: a new sound takes the place of the quietest one, and a sound
//! quieter than all of them does not play. As in the reference client, a
//! sound does not start again while it still plays.

use crate::window::settings::{SoundKind, SoundOptions};
use rodio::buffer::SamplesBuffer;
use rodio::mixer::Mixer;
use rodio::Player;
use std::num::NonZero;
use uoterm_nav::SOUND_SAMPLE_RATE;

pub const ONE_CHANNEL: NonZero<u16> = NonZero::new(1).unwrap();
pub const SAMPLE_RATE: NonZero<u32> = NonZero::new(SOUND_SAMPLE_RATE).unwrap();

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

struct Playing {
    cue: EffectCue,
    player: Player,
}

/// The sound effects that play, the oldest first.
#[derive(Default)]
pub struct Effects {
    playing: Vec<Playing>,
}

/// Where a new sound goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Room {
    Free,
    /// In the place of the playing sound at this index.
    InPlaceOf(usize),
    /// Nowhere: the new sound does not play.
    Full,
}

/// Where a new sound of `loudness` goes, when sounds of `playing` loudness
/// play and at most `max` may. Of sounds equally quiet, the oldest goes.
fn room(playing: &[f32], max: usize, loudness: f32) -> Room {
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

impl Effects {
    /// Plays `samples` for `cue`, if there is room. `gain` is 0 when the
    /// window may not be heard.
    pub fn start(
        &mut self,
        mixer: &Mixer,
        cue: EffectCue,
        samples: &[f32],
        options: &SoundOptions,
        gain: f32,
    ) {
        self.playing.retain(|playing| !playing.player.empty());
        if self
            .playing
            .iter()
            .any(|playing| playing.cue.sound == cue.sound)
        {
            return;
        }
        let loudness: Vec<f32> = self
            .playing
            .iter()
            .map(|playing| playing.cue.loudness(options))
            .collect();
        let max = usize::from(options.max_sounds_at_once);
        match room(&loudness, max, cue.loudness(options)) {
            Room::Full => return,
            // The player stops when it drops.
            Room::InPlaceOf(index) => drop(self.playing.remove(index)),
            Room::Free => {}
        }
        let player = Player::connect_new(mixer);
        player.set_volume(cue.loudness(options) * gain);
        player.append(SamplesBuffer::new(
            ONE_CHANNEL,
            SAMPLE_RATE,
            samples.to_vec(),
        ));
        self.playing.push(Playing { cue, player });
    }

    /// Makes each playing sound as loud as `options` and `gain` say now.
    pub fn set_volumes(&self, options: &SoundOptions, gain: f32) {
        for playing in &self.playing {
            playing
                .player
                .set_volume(playing.cue.loudness(options) * gain);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
