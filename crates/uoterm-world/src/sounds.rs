//! What the shard tells the client to play: sound effects at a place, and
//! the music of a region. A headless client makes no sound. It keeps the
//! last few cues, so a watch window can play them.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// How many sound cues the world keeps. A window asks several times each
/// second, so a short list is enough.
pub const SOUND_CUE_CAP: usize = 32;

/// One sound effect the shard asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundCue {
    /// Counts up from one. A window plays each cue with a number above the
    /// last one it played.
    pub seq: u64,
    pub sound: u16,
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sounds {
    cues: VecDeque<SoundCue>,
    last_seq: u64,
    /// The music the shard last asked for. None when it asked for silence,
    /// or never asked.
    music: Option<u16>,
}

impl Sounds {
    pub fn heard(&mut self, sound: u16, x: u16, y: u16) {
        self.last_seq += 1;
        self.cues.push_back(SoundCue {
            seq: self.last_seq,
            sound,
            x,
            y,
        });
        while self.cues.len() > SOUND_CUE_CAP {
            self.cues.pop_front();
        }
    }

    pub fn music_changed(&mut self, index: u16, stop: bool) {
        self.music = (!stop).then_some(index);
    }

    pub fn cues(&self) -> Vec<SoundCue> {
        self.cues.iter().copied().collect()
    }

    pub fn music(&self) -> Option<u16> {
        self.music
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SWORD_HIT: u16 = 0x023B;

    #[test]
    fn cues_count_up_and_old_ones_drop_off() {
        let mut sounds = Sounds::default();
        for _ in 0..SOUND_CUE_CAP + 3 {
            sounds.heard(SWORD_HIT, 10, 20);
        }
        let cues = sounds.cues();
        assert_eq!(cues.len(), SOUND_CUE_CAP);
        assert_eq!(cues.last().unwrap().seq, SOUND_CUE_CAP as u64 + 3);
        assert_eq!(cues[0].seq, 4);
    }

    #[test]
    fn a_stop_silences_the_music() {
        const BRITAIN: u16 = 9;
        let mut sounds = Sounds::default();
        assert_eq!(sounds.music(), None);
        sounds.music_changed(BRITAIN, false);
        assert_eq!(sounds.music(), Some(BRITAIN));
        sounds.music_changed(0x1FFF, true);
        assert_eq!(sounds.music(), None);
    }
}
