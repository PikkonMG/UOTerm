//! The spell cast indicator: the character said the words of a spell, so a
//! cast began. It shows how long the cast takes, and who stands in range.
//! The words tell of every cast, by the player, a hotkey or the agent.
//! Also the hue of a spell by what it does, and the words of power as they
//! float over a caster by the Combat page's overhead spell options.

use super::journal::NewLines;
use super::spell_data::spell_by_words;
use crate::view::{WatchFrame, WatchMobile};
use crate::window::settings::CombatOptions;
use uoterm_assist::spells::{School, Spell, SpellFlag};

/// The marks of the overhead spell format, as the reference client reads them.
const FORMAT_POWER: &str = "{power}";
const FORMAT_SPELL: &str = "{spell}";

/// Cast times count in ticks of a quarter second.
const SECONDS_PER_TICK: f32 = 0.25;
/// A Magery spell takes this many ticks more than its circle, the first
/// circle as 0 (ModernUO `MagerySpell.CastDelayBase`).
const MAGERY_BASE_TICKS: f32 = 3.0;
/// Arch Cure takes one tick less than its circle.
const ARCH_CURE: u16 = 25;
/// A Mysticism spell takes this long, and a tick more for each circle
/// (ModernUO `MysticSpell.CastDelayBase`).
const MYSTIC_BASE_SECONDS: f32 = 0.5;
const FIRST_MYSTIC: u16 = 678;
const MYSTIC_SPELLS_PER_CIRCLE: u16 = 2;
/// A mastery with no delay of its own takes this long
/// (ServUO `SkillMasterySpell.CastDelayBase`).
const MASTERY_SECONDS: f32 = 2.25;
/// The masteries that are never cast: the passive ones and the moves.
const MASTERIES_NOT_CAST: [u16; 11] = [715, 716, 718, 726, 727, 729, 733, 739, 741, 742, 745];
/// The spells that have a delay of their own, in seconds, as the newest
/// era of ModernUO gives it (ServUO for the ones ModernUO lacks). The
/// Bushido and Ninjitsu moves are not here: they are not cast.
const OWN_SECONDS: [(u16, f32); 56] = [
    // Necromancy.
    (101, 1.5),
    (102, 1.5),
    (103, 1.5),
    (104, 0.75),
    (105, 0.75),
    (106, 2.0),
    (107, 2.0),
    (108, 1.5),
    (109, 1.0),
    (110, 1.75),
    (111, 2.0),
    (112, 2.0),
    (113, 2.0),
    (114, 2.0),
    (115, 1.25),
    (116, 2.0),
    (117, 2.0),
    // Chivalry.
    (201, 1.0),
    (202, 1.5),
    (203, 0.5),
    (204, 0.25),
    (205, 1.0),
    (206, 0.5),
    (207, 1.75),
    (208, 1.5),
    (209, 1.75),
    (210, 1.5),
    // Bushido.
    (402, 0.25),
    (403, 0.25),
    (404, 0.25),
    // Ninjitsu.
    (503, 1.0),
    (507, 1.0),
    (508, 1.5),
    // Spellweaving.
    (601, 0.5),
    (602, 3.0),
    (603, 1.0),
    (604, 1.0),
    (605, 1.5),
    (606, 1.5),
    (607, 1.5),
    (608, 2.0),
    (609, 2.5),
    (610, 2.5),
    (611, 3.0),
    (612, 3.0),
    (613, 3.5),
    (614, 3.5),
    (615, 4.0),
    (616, 3.0),
    // Masteries.
    (709, 2.0),
    (711, 3.0),
    (720, 2.5),
    (734, 1.0),
    (735, 1.0),
    (736, 1.0),
    (738, 1.0),
];

/// Faster casting counts up to this, and up to the lower cap for Magery,
/// Necromancy, and Chivalry of a caster with enough Magery.
const FASTER_CASTING_CAP: i16 = 4;
const FASTER_CASTING_MAGE_CAP: i16 = 2;
/// Chivalry takes the lower cap from this Magery on, in tenths.
const PALADIN_MAGE_TENTHS: u16 = 700;
/// The Protection spell takes this much faster casting away.
const PROTECTION_LOSS: i16 = 2;
/// Since Stygian Abyss each cast takes one tick more.
const STYGIAN_ABYSS_LOSS: i16 = 1;
/// No cast is faster than this, nor a Chivalry cast faster than its own.
const MINIMUM_SECONDS: f32 = 0.25;
const CHIVALRY_MINIMUM_SECONDS: f32 = 0.5;
const SKILL_MAGERY: u16 = 25;
/// The buff icons of the Protection spell and of Essence of Wind.
const ICON_PROTECTION: u16 = 1029;
const ICON_ESSENCE_OF_WIND: u16 = 1023;
const ARGUMENT_SEPARATOR: char = '\t';

/// A cast with no known time shows for this long, unless its cursor comes.
pub const CAST_UNKNOWN_SECONDS: f32 = 3.0;
/// The words a shard writes when a cast fails.
const FIZZLE_WORDS: &str = "fizzles";

/// What the character brings to a cast.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Caster {
    pub faster_casting: i16,
    /// His Magery in tenths of a point.
    pub magery: u16,
    /// The Protection spell is on him.
    pub protected: bool,
    /// The faster casting Essence of Wind takes from him.
    pub wind_loss: i16,
}

impl Caster {
    /// The caster as the frame shows the character.
    pub fn of(frame: &WatchFrame) -> Self {
        let buff = |icon: u16| frame.buff_icons.iter().find(|buff| buff.icon == icon);
        Self {
            faster_casting: frame.status.faster_casting,
            magery: frame
                .skills
                .iter()
                .find(|skill| skill.id == SKILL_MAGERY)
                .map_or(0, |skill| skill.value),
            protected: buff(ICON_PROTECTION).is_some(),
            wind_loss: buff(ICON_ESSENCE_OF_WIND)
                .and_then(|buff| {
                    buff.arguments
                        .split(ARGUMENT_SEPARATOR)
                        .find(|word| !word.is_empty())?
                        .trim()
                        .parse()
                        .ok()
                })
                .unwrap_or(0),
        }
    }
}

/// How long a spell takes with no faster casting at all, when it is cast.
fn base_seconds(spell: &Spell) -> Option<f32> {
    match spell.school {
        School::Magery => {
            let circle = f32::from(spell.circle()?.saturating_sub(1));
            let quicker = if spell.id == ARCH_CURE { 1.0 } else { 0.0 };
            Some((MAGERY_BASE_TICKS + circle - quicker) * SECONDS_PER_TICK)
        }
        School::Mysticism => {
            let circle = spell.id.checked_sub(FIRST_MYSTIC)? / MYSTIC_SPELLS_PER_CIRCLE;
            Some(MYSTIC_BASE_SECONDS + f32::from(circle) * SECONDS_PER_TICK)
        }
        School::Mastery if MASTERIES_NOT_CAST.contains(&spell.id) => None,
        school => OWN_SECONDS
            .iter()
            .find(|(id, _)| *id == spell.id)
            .map(|(_, seconds)| *seconds)
            .or((school == School::Mastery).then_some(MASTERY_SECONDS)),
    }
}

/// How long a spell takes to cast for this caster, when it is cast. The
/// rules are those of ModernUO `Spell.GetCastDelay` for the newest era.
pub fn cast_seconds(spell: &Spell, caster: &Caster) -> Option<f32> {
    let base = base_seconds(spell)?;
    let cap = match spell.school {
        School::Magery | School::Necromancy => FASTER_CASTING_MAGE_CAP,
        School::Chivalry if caster.magery >= PALADIN_MAGE_TENTHS => FASTER_CASTING_MAGE_CAP,
        _ => FASTER_CASTING_CAP,
    };
    let protection = if caster.protected { PROTECTION_LOSS } else { 0 };
    let faster =
        caster.faster_casting.min(cap) - protection - caster.wind_loss - STYGIAN_ABYSS_LOSS;
    // Faster casting does not speed Bushido.
    let scale = if spell.school == School::Bushido {
        0.0
    } else {
        1.0
    };
    let minimum = if spell.school == School::Chivalry {
        CHIVALRY_MINIMUM_SECONDS
    } else {
        MINIMUM_SECONDS
    };
    Some((base - scale * f32::from(faster) * SECONDS_PER_TICK).max(minimum))
}

/// A cast that runs now.
#[derive(Clone, Debug, PartialEq)]
pub struct Cast {
    pub spell: u16,
    pub name: String,
    pub flag: SpellFlag,
    pub started: f64,
    /// None when the time of the spell is not known.
    pub seconds: Option<f32>,
    /// The spell's cursor came: the cast is done and waits for a target.
    pub aiming: bool,
}

impl Cast {
    /// The share of the cast done, from 0 to 1.
    pub fn share_done(&self, time: f64) -> f32 {
        let seconds = self.seconds.unwrap_or(CAST_UNKNOWN_SECONDS);
        ((time - self.started) as f32 / seconds).clamp(0.0, 1.0)
    }
}

#[derive(Default)]
pub struct CastWatch {
    cast: Option<Cast>,
    new_lines: NewLines,
}

/// The hue of a spell by what it does, as the Combat page sets it.
pub fn spell_hue(flag: SpellFlag, combat: &CombatOptions) -> u16 {
    match flag {
        SpellFlag::Beneficial => combat.benefic_spell_hue,
        SpellFlag::Harmful => combat.harmful_spell_hue,
        SpellFlag::Neutral => combat.neutral_spell_hue,
    }
}

/// The words of power a caster says as they float over him, and their
/// hue, as the reference client shows them: with the overhead spell format on, the
/// format with `{power}` and `{spell}` put in; with the overhead spell hue
/// on, the hue of what the spell does. Words of no spell stay as they are.
pub fn overhead_spell(text: &str, hue: u16, combat: &CombatOptions) -> (String, u16) {
    let Some(spell) = spell_by_words(text) else {
        return (text.to_string(), hue);
    };
    let words = if combat.spell_format_on && !combat.spell_format.trim().is_empty() {
        combat
            .spell_format
            .replace(FORMAT_POWER, &spell.words)
            .replace(FORMAT_SPELL, &spell.name)
            .trim()
            .to_string()
    } else {
        text.to_string()
    };
    let hue = if combat.spell_hue_on {
        spell_hue(spell.flag, combat)
    } else {
        hue
    };
    (words, hue)
}

impl CastWatch {
    /// Reads the new journal lines and the cursor.
    pub fn observe(&mut self, frame: &WatchFrame, time: f64) {
        for line in self.new_lines.take(&frame.speech) {
            if line.serial == frame.serial {
                if let Some(spell) = spell_by_words(&line.text) {
                    self.cast = Some(Cast {
                        spell: spell.id,
                        name: spell.name.clone(),
                        flag: spell.flag,
                        started: time,
                        seconds: cast_seconds(spell, &Caster::of(frame)),
                        aiming: false,
                    });
                }
            } else if line.text.to_lowercase().contains(FIZZLE_WORDS) {
                self.cast = None;
            }
        }
        if let Some(cast) = self.cast.as_mut() {
            cast.aiming |= frame.target_cursor;
            if cast.share_done(time) >= 1.0 && !frame.target_cursor {
                self.cast = None;
            }
        }
    }

    pub fn cast(&self) -> Option<&Cast> {
        self.cast.as_ref()
    }
}

/// The mobiles a spell reaches from the character: within `tiles`.
pub fn in_range(frame: &WatchFrame, tiles: u8) -> Vec<&WatchMobile> {
    frame
        .mobiles
        .iter()
        .filter(|mobile| mobile.dist <= u16::from(tiles))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchSpeech;
    use uoterm_assist::spells::SpellBook;

    const ME: u32 = 0x55;

    #[test]
    fn spell_words_float_in_the_format_and_hue_of_the_combat_page() {
        const SERVER_HUE: u16 = 0x0033;
        let mut combat = CombatOptions::default();
        assert_eq!(
            overhead_spell("In Vas Mani", SERVER_HUE, &combat),
            ("In Vas Mani".to_string(), SERVER_HUE),
            "both options are off"
        );
        combat.spell_format_on = true;
        combat.spell_hue_on = true;
        assert_eq!(
            overhead_spell("in vas mani", SERVER_HUE, &combat),
            (
                "In Vas Mani [Greater Heal]".to_string(),
                combat.benefic_spell_hue
            )
        );
        let (_, harmful) = overhead_spell("Corp Por", SERVER_HUE, &combat);
        assert_eq!(harmful, combat.harmful_spell_hue);
        assert_eq!(
            overhead_spell("Vendor buy", SERVER_HUE, &combat),
            ("Vendor buy".to_string(), SERVER_HUE),
            "no spell says these words"
        );
    }

    fn said(seq: u64, serial: u32, text: &str) -> WatchSpeech {
        WatchSpeech {
            seq,
            serial,
            text: text.to_string(),
            ..WatchSpeech::default()
        }
    }

    fn seconds(name: &str, caster: Caster) -> Option<f32> {
        let book = SpellBook::standard();
        cast_seconds(book.by_name(name).unwrap(), &caster)
    }

    fn with_faster_casting(faster_casting: i16) -> Caster {
        Caster {
            faster_casting,
            ..Caster::default()
        }
    }

    #[test]
    fn a_magery_cast_takes_longer_for_a_higher_circle() {
        let slow = Caster::default();
        assert_eq!(seconds("Clumsy", slow), Some(1.0));
        assert_eq!(seconds("Arch Cure", slow), Some(1.5));
        assert_eq!(seconds("Flame Strike", slow), Some(2.5));
        assert_eq!(seconds("Earthquake", slow), Some(2.75));
    }

    #[test]
    fn faster_casting_counts_up_to_the_cap_of_the_school() {
        let fast = with_faster_casting(4);
        assert_eq!(seconds("Clumsy", with_faster_casting(2)), Some(0.5));
        assert_eq!(seconds("Clumsy", fast), Some(0.5), "Magery counts 2");
        assert_eq!(
            seconds("Pain Spike", fast),
            Some(0.75),
            "so does Necromancy"
        );
        assert_eq!(seconds("Close Wounds", fast), Some(0.75));
        let paladin_mage = Caster {
            magery: PALADIN_MAGE_TENTHS,
            ..fast
        };
        assert_eq!(seconds("Close Wounds", paladin_mage), Some(1.25));
        assert_eq!(seconds("Consecrate Weapon", fast), Some(0.5), "the floor");
        assert_eq!(
            seconds("Confidence", fast),
            Some(0.25),
            "Bushido is not sped"
        );
        assert_eq!(seconds("Word of Death", fast), Some(2.75));
        let protected = Caster {
            protected: true,
            ..with_faster_casting(2)
        };
        assert_eq!(seconds("Clumsy", protected), Some(1.0));
    }

    #[test]
    fn every_school_has_its_times_and_moves_have_none() {
        let slow = Caster::default();
        assert_eq!(seconds("Nether Bolt", slow), Some(0.75));
        assert_eq!(seconds("Rising Colossus", slow), Some(2.5));
        assert_eq!(seconds("Wither", slow), Some(1.5));
        assert_eq!(seconds("Animal Form", slow), Some(1.25));
        assert_eq!(seconds("Wildfire", slow), Some(2.75));
        assert_eq!(seconds("Inspire", slow), Some(2.5));
        assert_eq!(seconds("Nether Blast", slow), Some(2.25));
        assert_eq!(seconds("Focus Attack", slow), None);
        assert_eq!(seconds("Lightning Strike", slow), None);
        assert_eq!(seconds("Potency", slow), None);
        let book = SpellBook::standard();
        let casts = book
            .iter()
            .filter(|spell| cast_seconds(spell, &slow).is_some());
        assert_eq!(
            casts.count(),
            book.iter().count() - 8 - MASTERIES_NOT_CAST.len()
        );
    }

    #[test]
    fn the_caster_reads_his_faster_casting_magery_and_spells() {
        use crate::view::{WatchBuff, WatchSkill};
        let mut frame = WatchFrame {
            skills: vec![WatchSkill {
                id: SKILL_MAGERY,
                value: 800,
                ..WatchSkill::default()
            }],
            buff_icons: vec![
                WatchBuff {
                    icon: ICON_PROTECTION,
                    ..WatchBuff::default()
                },
                WatchBuff {
                    icon: ICON_ESSENCE_OF_WIND,
                    arguments: "\t3\t15".into(),
                    ..WatchBuff::default()
                },
            ],
            ..WatchFrame::default()
        };
        frame.status.faster_casting = 3;
        assert_eq!(
            Caster::of(&frame),
            Caster {
                faster_casting: 3,
                magery: 800,
                protected: true,
                wind_loss: 3,
            }
        );
    }

    #[test]
    fn the_words_of_the_character_start_a_cast_and_it_ends_in_time() {
        let mut watch = CastWatch::default();
        let mut frame = WatchFrame {
            serial: ME,
            speech: vec![said(1, ME, "In Mani")],
            ..WatchFrame::default()
        };
        watch.observe(&frame, 0.0);
        assert!(watch.cast().is_none(), "an old line starts nothing");
        frame.speech.push(said(2, 9, "In Vas Mani"));
        frame.speech.push(said(3, ME, "in vas mani"));
        watch.observe(&frame, 1.0);
        let cast = watch.cast().unwrap();
        assert_eq!(cast.name, "Greater Heal");
        assert!((cast.share_done(1.875) - 0.5).abs() < 0.001);
        frame.target_cursor = true;
        watch.observe(&frame, 3.0);
        assert!(
            watch.cast().unwrap().aiming,
            "the cursor waits for a target"
        );
        frame.target_cursor = false;
        watch.observe(&frame, 3.1);
        assert!(watch.cast().is_none());
    }

    #[test]
    fn a_fizzle_ends_the_cast_and_range_counts_tiles() {
        let mut watch = CastWatch::default();
        let mut frame = WatchFrame {
            serial: ME,
            ..WatchFrame::default()
        };
        watch.observe(&frame, 0.0);
        frame.speech = vec![said(1, ME, "Uus Jux"), said(2, 0, "The spell fizzles.")];
        watch.observe(&frame, 0.1);
        assert!(watch.cast().is_none());
        frame.mobiles = vec![
            WatchMobile {
                dist: 3,
                ..WatchMobile::default()
            },
            WatchMobile {
                dist: 20,
                ..WatchMobile::default()
            },
        ];
        assert_eq!(in_range(&frame, 12).len(), 1);
    }
}
