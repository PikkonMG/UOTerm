//! The sound effects that play now, one player for each voice of
//! `uoterm_view::audio::Mixer`. The mixer decides which voice starts, which
//! one it replaces and how loud each plays; the players only play.

use rodio::buffer::SamplesBuffer;
use rodio::mixer::Mixer;
use rodio::Player;
use std::collections::HashMap;
use std::num::NonZero;
use uoterm_nav::SOUND_SAMPLE_RATE;
use uoterm_view::audio::VoiceVolume;

pub const ONE_CHANNEL: NonZero<u16> = NonZero::new(1).unwrap();
pub const SAMPLE_RATE: NonZero<u32> = NonZero::new(SOUND_SAMPLE_RATE).unwrap();

/// The players of the voices that play.
#[derive(Default)]
pub struct Effects {
    playing: HashMap<u64, Player>,
}

impl Effects {
    /// Plays `samples` as the voice `voice`, `volume` loud.
    pub fn start(&mut self, mixer: &Mixer, voice: u64, samples: &[f32], volume: f32) {
        let player = Player::connect_new(mixer);
        player.set_volume(volume);
        player.append(SamplesBuffer::new(
            ONE_CHANNEL,
            SAMPLE_RATE,
            samples.to_vec(),
        ));
        self.playing.insert(voice, player);
    }

    /// Stops the voice `voice`. Its player stops when it drops.
    pub fn stop(&mut self, voice: u64) {
        self.playing.remove(&voice);
    }

    /// Takes out the voices that came to their end, and gives them.
    pub fn take_ended(&mut self) -> Vec<u64> {
        let ended: Vec<u64> = self
            .playing
            .iter()
            .filter(|(_, player)| player.empty())
            .map(|(voice, _)| *voice)
            .collect();
        for voice in &ended {
            self.playing.remove(voice);
        }
        ended
    }

    /// Makes each voice as loud as `volumes` say.
    pub fn set_volumes(&self, volumes: &[VoiceVolume]) {
        for told in volumes {
            if let Some(player) = self.playing.get(&told.voice) {
                player.set_volume(told.volume);
            }
        }
    }
}
