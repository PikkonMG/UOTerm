//! The sound effects that play now. The Sound page limits how many play at
//! once: a new sound takes the place of the quietest one, and a sound
//! quieter than all of them does not play. As in the reference client, a
//! sound does not start again while it still plays. Where a new sound goes
//! is `uoterm_view::audio::room`.

use crate::window::settings::SoundOptions;
use rodio::buffer::SamplesBuffer;
use rodio::mixer::Mixer;
use rodio::Player;
use std::num::NonZero;
use uoterm_nav::SOUND_SAMPLE_RATE;
use uoterm_view::audio::{room, EffectCue, Room};

pub const ONE_CHANNEL: NonZero<u16> = NonZero::new(1).unwrap();
pub const SAMPLE_RATE: NonZero<u32> = NonZero::new(SOUND_SAMPLE_RATE).unwrap();

struct Playing {
    cue: EffectCue,
    player: Player,
}

/// The sound effects that play, the oldest first.
#[derive(Default)]
pub struct Effects {
    playing: Vec<Playing>,
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
