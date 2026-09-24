//! Which music plays: the music of the region the shard names, and for a
//! time over the region music, the music of a fight or of a death. The rules
//! are the reference client's.

use crate::window::settings::SoundOptions;
use std::ops::RangeInclusive;

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

#[cfg(test)]
mod tests {
    use super::*;

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
