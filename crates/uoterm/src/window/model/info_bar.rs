//! The values of the info bar and of the window title: each item of the
//! Info Bar page as words, with the hue that tells when it runs low, as
//! the reference client colors them.

use crate::view::WatchFrame;
use crate::window::look::notoriety_hue;
use crate::window::settings::{CombatOptions, InfoBarData, TitleStats};

/// The hue of a value that is fine.
pub const HUE_FINE: u16 = 0x0481;
/// The hues of a value from bad to fair.
const HUE_LOW: u16 = 0x0021;
const HUE_HALF: u16 = 0x0030;
const HUE_FAIR: u16 = 0x0035;
const QUARTER: f32 = 0.25;
const HALF: f32 = 0.5;
const THREE_QUARTERS: f32 = 0.75;
const FULL: f32 = 1.0;
const PERCENT: f32 = 100.0;

/// One value as the bar shows it.
#[derive(Clone, Debug, PartialEq)]
pub struct InfoValue {
    pub words: String,
    /// The UO hue of the words.
    pub hue: u16,
    /// How full a bar of the value is, for the values that have a most.
    pub fill: Option<f32>,
}

fn share(now: u16, max: u16) -> Option<f32> {
    (max > 0).then(|| (f32::from(now) / f32::from(max)).clamp(0.0, FULL))
}

/// The hue of a value that is bad when it is low.
fn hue_when_low(share: Option<f32>) -> u16 {
    match share {
        Some(s) if s <= QUARTER => HUE_LOW,
        Some(s) if s <= HALF => HUE_HALF,
        Some(s) if s <= THREE_QUARTERS => HUE_FAIR,
        _ => HUE_FINE,
    }
}

/// The hue of a value that is bad when it is high: the weight.
fn hue_when_high(share: Option<f32>) -> u16 {
    match share {
        Some(s) if s >= FULL => HUE_LOW,
        Some(s) if s >= THREE_QUARTERS => HUE_HALF,
        Some(s) if s >= HALF => HUE_FAIR,
        _ => HUE_FINE,
    }
}

/// The hue of the character's name: his notoriety, in the colors of the
/// Combat page, or the fine hue for none.
fn name_hue(notoriety: u8, combat: &CombatOptions) -> u16 {
    Some(notoriety_hue(combat, notoriety))
        .filter(|hue| *hue != 0)
        .unwrap_or(HUE_FINE)
}

fn pair(now: u16, max: u16, hue_of: fn(Option<f32>) -> u16) -> InfoValue {
    let fill = share(now, max);
    InfoValue {
        words: format!("{now}/{max}"),
        hue: hue_of(fill),
        fill,
    }
}

fn plain(words: String) -> InfoValue {
    InfoValue {
        words,
        hue: HUE_FINE,
        fill: None,
    }
}

/// One item of the info bar, for the character of the frame.
pub fn value_of(data: InfoBarData, frame: &WatchFrame, combat: &CombatOptions) -> InfoValue {
    let status = &frame.status;
    match data {
        InfoBarData::HitPoints => pair(frame.hits, frame.hits_max, hue_when_low),
        InfoBarData::Mana => pair(frame.mana, frame.mana_max, hue_when_low),
        InfoBarData::Stamina => pair(frame.stam, frame.stam_max, hue_when_low),
        InfoBarData::Weight => pair(frame.weight, frame.weight_max, hue_when_high),
        InfoBarData::Followers => plain(format!("{}/{}", status.followers, status.followers_max)),
        InfoBarData::Gold => plain(frame.gold.to_string()),
        InfoBarData::Damage => plain(format!("{}-{}", status.damage_min, status.damage_max)),
        InfoBarData::Armor => plain(status.physical_resist.to_string()),
        InfoBarData::Luck => plain(status.luck.to_string()),
        InfoBarData::FireResist => plain(status.fire_resist.to_string()),
        InfoBarData::ColdResist => plain(status.cold_resist.to_string()),
        InfoBarData::PoisonResist => plain(status.poison_resist.to_string()),
        InfoBarData::EnergyResist => plain(status.energy_resist.to_string()),
        InfoBarData::LowerReagentCost => plain(status.lower_reagent_cost.to_string()),
        InfoBarData::SpellDamage => plain(status.spell_damage_increase.to_string()),
        InfoBarData::FasterCasting => plain(status.faster_casting.to_string()),
        InfoBarData::FasterCastRecovery => plain(status.faster_cast_recovery.to_string()),
        InfoBarData::HitChance => plain(status.hit_chance_increase.to_string()),
        InfoBarData::DefenseChance => plain(status.defense_chance_increase.to_string()),
        InfoBarData::LowerManaCost => plain(status.lower_mana_cost.to_string()),
        InfoBarData::DamageIncrease => plain(status.damage_increase.to_string()),
        InfoBarData::SwingSpeed => plain(status.swing_speed_increase.to_string()),
        InfoBarData::StatsCap => plain(status.stat_cap.to_string()),
        InfoBarData::Name => InfoValue {
            words: frame.name.clone(),
            hue: name_hue(frame.notoriety, combat),
            fill: None,
        },
        InfoBarData::TithingPoints => plain(status.tithing.to_string()),
    }
}

/// The title of the window with the vitals of the character, as the
/// Interface page asks: numbers, or percents.
pub fn title_words(base: &str, frame: &WatchFrame, mode: TitleStats) -> String {
    let vital = |letter: &str, now: u16, max: u16| match mode {
        TitleStats::Numbers => format!("{letter} {now}/{max}"),
        TitleStats::Percent => {
            let percent = share(now, max).map_or(0.0, |s| s * PERCENT);
            format!("{letter} {percent:.0}%")
        }
    };
    format!(
        "{base} - {} [{} {} {}]",
        frame.name,
        vital("H", frame.hits, frame.hits_max),
        vital("M", frame.mana, frame.mana_max),
        vital("S", frame.stam, frame.stam_max)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> WatchFrame {
        WatchFrame {
            name: "Mara".into(),
            notoriety: uoterm_protocol::types::NOTO_MURDERER,
            hits: 20,
            hits_max: 100,
            mana: 60,
            mana_max: 100,
            stam: 100,
            stam_max: 100,
            weight: 80,
            weight_max: 100,
            ..WatchFrame::default()
        }
    }

    #[test]
    fn a_low_vital_and_a_heavy_pack_take_warning_hues() {
        let combat = CombatOptions::default();
        let frame = frame();
        let hits = value_of(InfoBarData::HitPoints, &frame, &combat);
        assert_eq!((hits.words.as_str(), hits.hue), ("20/100", HUE_LOW));
        assert_eq!(value_of(InfoBarData::Mana, &frame, &combat).hue, HUE_FAIR);
        assert_eq!(
            value_of(InfoBarData::Stamina, &frame, &combat).hue,
            HUE_FINE
        );
        assert_eq!(value_of(InfoBarData::Weight, &frame, &combat).hue, HUE_HALF);
        let name = value_of(InfoBarData::Name, &frame, &combat);
        assert_eq!(
            (name.words.as_str(), name.hue),
            ("Mara", combat.murderer_hue)
        );
        let nobody = WatchFrame::default();
        assert_eq!(value_of(InfoBarData::Name, &nobody, &combat).hue, HUE_FINE);
        assert_eq!(value_of(InfoBarData::Damage, &frame, &combat).words, "0-0");
        assert_eq!(
            value_of(InfoBarData::HitPoints, &WatchFrame::default(), &combat).fill,
            None
        );
    }

    #[test]
    fn the_title_shows_numbers_or_percents() {
        let frame = frame();
        assert_eq!(
            title_words("UOTerm", &frame, TitleStats::Numbers),
            "UOTerm - Mara [H 20/100 M 60/100 S 100/100]"
        );
        assert_eq!(
            title_words("UOTerm", &frame, TitleStats::Percent),
            "UOTerm - Mara [H 20% M 60% S 100%]"
        );
    }
}
