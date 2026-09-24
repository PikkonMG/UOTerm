//! The status of the character, apart from how a window draws it: the
//! words of each value, the facts of the status sheet in their sections,
//! and the stat locks the player turns, as the Classic status gump and the
//! Modern character tab both show them; and the journal lines that tell of
//! a stat that changed.

use super::skills::LOCK_WORDS;
use crate::view::WatchFrame;
use crate::window::settings::GeneralOptions;

/// The stats as the stat lock command names them: strength, dexterity,
/// intelligence.
pub const STAT_WORDS: [&str; 3] = ["str", "dex", "int"];
/// The stats as a sheet names them.
pub const STAT_NAMES: [&str; 3] = ["Strength", "Dexterity", "Intelligence"];
const LOCK_COUNT: u8 = LOCK_WORDS.len() as u8;

/// "current/most".
pub fn of(current: impl std::fmt::Display, most: impl std::fmt::Display) -> String {
    format!("{current}/{most}")
}

/// The damage of the weapon in hand: "least-most".
pub fn damage_words(frame: &WatchFrame) -> String {
    format!("{}-{}", frame.status.damage_min, frame.status.damage_max)
}

/// The three stats of the character, in the order of `STAT_WORDS`.
pub fn stats(frame: &WatchFrame) -> [u16; 3] {
    [
        frame.stats.strength,
        frame.stats.dexterity,
        frame.stats.intelligence,
    ]
}

/// The stats as the reference client's journal line names them.
const STAT_WORDS_LOWER: [&str; 3] = ["strength", "dexterity", "intelligence"];

/// Tells of the stats that changed since the last frame, as the classic
/// client writes it in the journal when the General page's "Inform when
/// stats change" is on.
#[derive(Default)]
pub struct StatChanges {
    last: Option<[u16; 3]>,
}

impl StatChanges {
    pub fn observe(&mut self, frame: &WatchFrame, options: &GeneralOptions) -> Vec<String> {
        let now = stats(frame);
        // A status with no stats says nothing of them.
        if now == [0; 3] {
            return Vec::new();
        }
        let Some(before) = self.last.replace(now) else {
            return Vec::new();
        };
        if !options.stat_change_messages {
            return Vec::new();
        }
        (0..now.len())
            .filter(|at| now[*at] != before[*at])
            .map(|at| {
                let change = i32::from(now[at]) - i32::from(before[at]);
                format!(
                    "Your {} has changed by {change}.  It is now {}",
                    STAT_WORDS_LOWER[at], now[at]
                )
            })
            .collect()
    }
}

/// One section of the status sheet: its title and its facts, each with
/// its words and its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    pub title: &'static str,
    pub facts: Vec<(&'static str, String)>,
}

/// Every fact of the status, in the sections of the sheet: the ones of
/// the classic status gump, the tithing points, and the combat and casting
/// properties of the newer shards.
pub fn sections(frame: &WatchFrame) -> Vec<Section> {
    let s = &frame.status;
    vec![
        Section {
            title: "Vitals",
            facts: vec![
                ("Hit Points", of(frame.hits, frame.hits_max)),
                ("Stamina", of(frame.stam, frame.stam_max)),
                ("Mana", of(frame.mana, frame.mana_max)),
                ("Maximum Stats", s.stat_cap.to_string()),
            ],
        },
        Section {
            title: "Belongings",
            facts: vec![
                ("Weight", of(frame.weight, frame.weight_max)),
                ("Gold", frame.gold.to_string()),
                ("Followers", of(s.followers, s.followers_max)),
                ("Luck", s.luck.to_string()),
                ("Tithing Points", s.tithing.to_string()),
            ],
        },
        Section {
            title: "Resistances",
            facts: vec![
                ("Physical", of(s.physical_resist, s.max_physical_resist)),
                ("Fire", of(s.fire_resist, s.max_fire_resist)),
                ("Cold", of(s.cold_resist, s.max_cold_resist)),
                ("Poison", of(s.poison_resist, s.max_poison_resist)),
                ("Energy", of(s.energy_resist, s.max_energy_resist)),
            ],
        },
        Section {
            title: "Combat",
            facts: vec![
                ("Damage", damage_words(frame)),
                ("Hit Chance Increase", s.hit_chance_increase.to_string()),
                (
                    "Defense Chance Increase",
                    of(s.defense_chance_increase, s.max_defense_chance_increase),
                ),
                ("Swing Speed Increase", s.swing_speed_increase.to_string()),
                ("Weapon Damage Increase", s.damage_increase.to_string()),
            ],
        },
        Section {
            title: "Casting",
            facts: vec![
                ("Faster Casting", s.faster_casting.to_string()),
                ("Faster Cast Recovery", s.faster_cast_recovery.to_string()),
                ("Lower Mana Cost", s.lower_mana_cost.to_string()),
                ("Lower Reagent Cost", s.lower_reagent_cost.to_string()),
                ("Spell Damage Increase", s.spell_damage_increase.to_string()),
            ],
        },
    ]
}

/// The lock after `lock`: up, down, locked, and up again.
pub fn next_lock(lock: u8) -> u8 {
    (lock % LOCK_COUNT + 1) % LOCK_COUNT
}

/// The script line that sets the lock of a stat, by its place in
/// `STAT_WORDS`.
pub fn stat_lock_command(stat: usize, lock: u8) -> String {
    format!(
        "setstatlock '{}' '{}'",
        STAT_WORDS[stat],
        LOCK_WORDS[usize::from(lock % LOCK_COUNT)]
    )
}

/// The stat locks the player set, until the shard tells them back.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatLocks {
    asked: [Option<u8>; 3],
}

impl StatLocks {
    /// The lock a stat shows: the one the player just asked for, or else
    /// the one the shard told. An ask the shard answered is forgotten.
    pub fn shown(&mut self, frame: &WatchFrame, stat: usize) -> u8 {
        let told = frame.stat_locks[stat];
        if self.asked[stat] == Some(told) {
            self.asked[stat] = None;
        }
        self.asked[stat].unwrap_or(told) % LOCK_COUNT
    }

    /// A click on the lock of a stat: it asks for the next lock. Gives the
    /// script line that asks the shard.
    pub fn turn(&mut self, frame: &WatchFrame, stat: usize) -> String {
        let next = next_lock(self.shown(frame, stat));
        self.asked[stat] = Some(next);
        stat_lock_command(stat, next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changed_stat_is_told_as_the_classic_client_tells_it() {
        let mut options = GeneralOptions {
            stat_change_messages: false,
            ..GeneralOptions::default()
        };
        let mut changes = StatChanges::default();
        let mut frame = WatchFrame::default();
        assert!(changes.observe(&frame, &options).is_empty(), "no stats yet");
        frame.stats.strength = 50;
        frame.stats.dexterity = 40;
        assert!(
            changes.observe(&frame, &options).is_empty(),
            "the first look"
        );
        frame.stats.strength = 51;
        frame.stats.dexterity = 39;
        assert!(
            changes.observe(&frame, &options).is_empty(),
            "the option is off"
        );
        options.stat_change_messages = true;
        frame.stats.strength = 52;
        assert_eq!(
            changes.observe(&frame, &options),
            ["Your strength has changed by 1.  It is now 52"]
        );
        frame.stats.dexterity = 38;
        assert_eq!(
            changes.observe(&frame, &options),
            ["Your dexterity has changed by -1.  It is now 38"]
        );
    }

    #[test]
    fn a_lock_turns_up_down_locked_and_waits_for_the_shard() {
        let mut frame = WatchFrame::default();
        let mut locks = StatLocks::default();
        assert_eq!(locks.shown(&frame, 0), 0);
        assert_eq!(locks.turn(&frame, 0), "setstatlock 'str' 'down'");
        assert_eq!(
            locks.shown(&frame, 0),
            1,
            "the ask shows before the shard answers"
        );
        assert_eq!(locks.turn(&frame, 0), "setstatlock 'str' 'locked'");
        frame.stat_locks[0] = 2;
        assert_eq!(locks.shown(&frame, 0), 2);
        frame.stat_locks[0] = 0;
        assert_eq!(locks.shown(&frame, 0), 0, "the answered ask is forgotten");
        assert_eq!(stat_lock_command(2, 0), "setstatlock 'int' 'up'");
        assert_eq!(next_lock(2), 0);
    }

    #[test]
    fn the_sections_hold_every_fact_of_the_status() {
        let mut frame = WatchFrame {
            hits: 50,
            hits_max: 60,
            gold: 1234,
            ..WatchFrame::default()
        };
        frame.status.tithing = 300;
        frame.status.fire_resist = 40;
        frame.status.max_fire_resist = 70;
        frame.status.damage_min = 11;
        frame.status.damage_max = 13;
        let all = sections(&frame);
        let fact = |words: &str| {
            all.iter()
                .flat_map(|section| section.facts.iter())
                .find(|(name, _)| *name == words)
                .map(|(_, value)| value.clone())
                .unwrap()
        };
        assert_eq!(fact("Hit Points"), "50/60");
        assert_eq!(fact("Gold"), "1234");
        assert_eq!(fact("Tithing Points"), "300");
        assert_eq!(fact("Fire"), "40/70");
        assert_eq!(fact("Damage"), "11-13");
        assert_eq!(fact("Faster Casting"), "0");
        assert_eq!(stats(&frame), [0, 0, 0]);
    }
}
