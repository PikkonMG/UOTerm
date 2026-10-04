//! Words and numbers that float over heads for a moment: what a mobile
//! said, the name a click asked for, and how many hits he lost. The session
//! sends each one with a number that counts up, so each one floats once.
//!
//! The Classic style draws them in the UO fonts, as the reference client does: speech
//! in a Unicode font with a black border, for as long as the Speech page
//! says, and damage in the small ASCII font. The Modern style keeps the
//! window's own font. Here is what floats and for how long; the window
//! draws it.

use crate::art::TextLook;
use crate::frame::{WatchCueKind, WatchFrame, WatchSpeech};
use crate::geom::{Point, Rgba, Vector};
use crate::model::casting::overhead_spell;
use crate::model::journal::SPEECH_SPELL;
use crate::settings::{FontOptions, GameFontKind, Profile, SpeechOptions, UiStyle};
use crate::ui::theme;
use std::collections::HashMap;
use uoterm_nav::TextAlign;
use uoterm_protocol::types::{SPEECH_ALLIANCE, SPEECH_GUILD, SPEECH_LABEL, SPEECH_SYSTEM};
use uoterm_world::{SPEECH_KIND_PARTY, SPEECH_KIND_PARTY_PRIVATE};

/// The height of one line of words in the window's own font.
pub const SPEECH_LINE: f32 = 18.0;
const SPEECH_MAX_CHARS: usize = 60;
/// Room between the head and the words, for the name plate.
pub const SPEECH_LIFT: f32 = 34.0;
/// Words over a head break into lines this wide.
const SPEECH_WRAP: u32 = 200;
/// Words fade out over their last second, when the General page says so.
const FADE_SECONDS: f64 = 1.0;
/// With the scaled delay, each line stays this long at a delay of 100.
const SCALED_LINE_SECONDS: f64 = 4.0;
const SCALED_DELAY_MIN: u16 = 10;
const DELAY_PER_CENT: f64 = 100.0;
/// With the plain delay, words stay this long for each step of the delay.
const PLAIN_DELAY_SECONDS: f64 = 0.04;
/// Damage floats this long, and rises this many points each second.
const DAMAGE_SECONDS: f64 = 1.5;
pub const DAMAGE_RISE_PER_SECOND: f32 = 40.0;
/// The size of damage numbers in the window's own font.
pub const DAMAGE_SIZE: f32 = crate::ui::theme::SIZE_DAMAGE;
/// The ASCII font of damage numbers, and their hues.
const DAMAGE_FONT: u8 = 3;
const OWN_DAMAGE_HUE: u16 = 0x0034;
const DAMAGE_HUE: u16 = 0x0021;
/// Damage per second counts the damage of this many seconds.
const DPS_SECONDS: f64 = 15.0;
const ELLIPSIS: &str = "...";

/// Words or a number over one mobile or thing.
#[derive(Clone, Debug, PartialEq)]
pub struct Float {
    pub serial: u32,
    pub words: String,
    /// How the Classic style draws the words in a UO font.
    pub look: TextLook,
    /// The color the Modern style draws the words in.
    pub color: Rgba,
    pub born: f64,
    pub seconds: f64,
    /// A number of hits lost, which rises as it floats.
    pub number: bool,
    /// Its time follows the lines of its words. False while the font has
    /// not measured them: a client that asks for the measure keeps one
    /// line's time until it comes.
    pub measured: bool,
}

#[derive(Default)]
pub struct Floats {
    /// None until the first picture. What came before it does not float.
    last_speech: Option<u64>,
    last_cue: Option<u64>,
    live: Vec<Float>,
    /// The damage each mobile took of late, and when.
    damage: HashMap<u32, Vec<(f64, u16)>>,
}

/// Words cut short with dots when they are too long to float.
pub fn shortened(text: &str) -> String {
    if text.chars().count() <= SPEECH_MAX_CHARS {
        return text.to_string();
    }
    let cut: String = text.chars().take(SPEECH_MAX_CHARS).collect();
    format!("{cut}{ELLIPSIS}")
}

/// How much of a float shows, from 1 to 0 over its last moments. With
/// fading off it shows whole to its end.
pub fn alpha(age: f64, seconds: f64, fading: bool) -> f32 {
    if !fading {
        return 1.0;
    }
    ((seconds - age) / FADE_SECONDS).clamp(0.0, 1.0) as f32
}

/// How long words of `lines` lines stay over a head, as the Speech page
/// says: longer for more lines when the delay scales, the same for all
/// when it does not.
pub fn speech_seconds(speech: &SpeechOptions, lines: usize) -> f64 {
    if speech.scale_speech_delay {
        let delay = speech.speech_delay.max(SCALED_DELAY_MIN);
        SCALED_LINE_SECONDS * lines as f64 * f64::from(delay) / DELAY_PER_CENT
    } else {
        f64::from(speech.speech_delay) * PLAIN_DELAY_SECONDS
    }
}

/// How words over heads look in a hue, from the Fonts page: a Unicode font
/// with a black border, or an ASCII font, broken into lines that fit.
pub fn speech_look(fonts: &FontOptions, hue: u16) -> TextLook {
    let look = if fonts.override_game_font && fonts.game_font_kind == GameFontKind::Ascii {
        TextLook::ascii(fonts.speech_font, hue)
    } else {
        TextLook::unicode(fonts.speech_font, hue).bordered()
    };
    look.wrap(SPEECH_WRAP).aligned(TextAlign::Center)
}

/// Whether a line floats over a head, and in which hue, as the Speech page
/// and the ignore list say. None for a line that does not float.
pub fn floating_hue(line: &WatchSpeech, profile: &Profile, classic: bool) -> Option<u16> {
    let speech = &profile.speech;
    if line.serial == 0 || line.kind == SPEECH_SYSTEM {
        return None;
    }
    if profile.ignore.names.iter().any(|name| name == &line.name) {
        return None;
    }
    let own_or = |hue: u16| if line.hue == 0 { hue } else { line.hue };
    match line.kind {
        SPEECH_LABEL => classic.then_some(line.hue),
        SPEECH_KIND_PARTY | SPEECH_KIND_PARTY_PRIVATE => speech
            .overhead_party_messages
            .then_some(own_or(speech.party_hue)),
        SPEECH_GUILD => (!speech.hide_guild_chat).then_some(own_or(speech.guild_hue)),
        SPEECH_ALLIANCE => (!speech.hide_alliance_chat).then_some(own_or(speech.alliance_hue)),
        _ => Some(line.hue),
    }
}

impl Floats {
    /// Takes the new speech and damage of a frame, and lets the old floats
    /// end. `lines` breaks words in a UO font as the window draws them,
    /// and `color` gives the color of words in a hue.
    pub fn take_in(
        &mut self,
        frame: &WatchFrame,
        time: f64,
        profile: &Profile,
        lines: impl Fn(&str, &TextLook) -> Vec<String>,
        color: impl Fn(u16) -> Rgba,
    ) {
        let classic = profile.interface.ui_style == UiStyle::Classic;
        let newest_speech = frame.speech.iter().map(|line| line.seq).max();
        if let Some(seen) = self.last_speech {
            for line in frame.speech.iter().filter(|line| line.seq > seen) {
                let Some(hue) = floating_hue(line, profile, classic) else {
                    continue;
                };
                let (text, hue) = if line.kind == SPEECH_SPELL {
                    overhead_spell(&line.text, hue, &profile.combat)
                } else {
                    (line.text.clone(), hue)
                };
                let words = shortened(&text);
                let look = speech_look(&profile.fonts, hue);
                let count = lines(&words, &look).len();
                self.live.push(Float {
                    serial: line.serial,
                    color: color(hue),
                    words,
                    look,
                    born: time,
                    seconds: speech_seconds(&profile.speech, count.max(1)),
                    number: false,
                    measured: count > 0,
                });
            }
        }
        self.last_speech = newest_speech.or(self.last_speech).or(Some(0));
        let newest_cue = frame.cues.iter().map(|cue| cue.seq).max();
        if let Some(seen) = self.last_cue {
            for cue in frame.cues.iter().filter(|cue| cue.seq > seen) {
                let WatchCueKind::Damage(amount) = cue.kind else {
                    continue;
                };
                let taken = self.damage.entry(cue.serial).or_default();
                taken.push((time, amount));
                let mut words = amount.to_string();
                if profile.combat.dps_with_damage {
                    let total: u32 = taken.iter().map(|(_, hits)| u32::from(*hits)).sum();
                    words = format!("{words} (DPS: {:.1})", f64::from(total) / DPS_SECONDS);
                }
                let own = cue.serial == frame.serial;
                let hue = if own { OWN_DAMAGE_HUE } else { DAMAGE_HUE };
                self.live.push(Float {
                    serial: cue.serial,
                    words,
                    look: TextLook::ascii(DAMAGE_FONT, hue),
                    color: if own { theme::ALARM } else { theme::WAITING },
                    born: time,
                    seconds: DAMAGE_SECONDS,
                    number: true,
                    measured: true,
                });
            }
        }
        self.last_cue = newest_cue.or(self.last_cue).or(Some(0));
        for float in self.live.iter_mut().filter(|float| !float.measured) {
            let count = lines(&float.words, &float.look).len();
            if count > 0 {
                float.seconds = speech_seconds(&profile.speech, count);
                float.measured = true;
            }
        }
        self.live.retain(|float| time - float.born < float.seconds);
        for taken in self.damage.values_mut() {
            taken.retain(|(at, _)| time - at < DPS_SECONDS);
        }
        self.damage.retain(|_, taken| !taken.is_empty());
    }

    /// The floats that show now, the oldest first.
    pub fn live(&self) -> &[Float] {
        &self.live
    }
}

/// Where one float shows this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacedFloat {
    /// Its place in [`Floats::live`].
    pub index: usize,
    /// The middle of the bottom of the words.
    pub bottom: Point,
    /// How much of it shows. The Classic style draws a number whole.
    pub alpha: f32,
}

/// Lays the floats over the heads, the newest first: a number rises from
/// the head as it ages, and words stand over the name plate, the newest
/// nearest the head. `head_of` gives where the head of the float at an
/// index is drawn; a float with none does not show. `height_of` gives the
/// height of the words of the float at an index.
pub fn lay_out(
    live: &[Float],
    time: f64,
    fading: bool,
    head_of: impl Fn(usize) -> Option<Point>,
    height_of: impl Fn(usize) -> f32,
) -> Vec<PlacedFloat> {
    let mut lines_over: Vec<(u32, f32)> = Vec::new();
    let mut placed = Vec::with_capacity(live.len());
    for (index, float) in live.iter().enumerate().rev() {
        let Some(head) = head_of(index) else {
            continue;
        };
        let age = time - float.born;
        if float.number {
            let rise = DAMAGE_RISE_PER_SECOND * age as f32;
            placed.push(PlacedFloat {
                index,
                bottom: head - Vector::new(0.0, rise),
                alpha: alpha(age, float.seconds, true),
            });
            continue;
        }
        let stacked = lines_over
            .iter()
            .find(|(serial, _)| *serial == float.serial)
            .map_or(0.0, |(_, height)| *height);
        placed.push(PlacedFloat {
            index,
            bottom: Point::new(head.x, head.y - SPEECH_LIFT - stacked),
            alpha: alpha(age, float.seconds, fading),
        });
        let height = height_of(index);
        match lines_over
            .iter_mut()
            .find(|(serial, _)| *serial == float.serial)
        {
            Some((_, stack)) => *stack += height,
            None => lines_over.push((float.serial, height)),
        }
    }
    placed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art::UoFont;
    use crate::frame::WatchCue;

    /// Every float is one line, in white.
    fn take_in(floats: &mut Floats, frame: &WatchFrame, profile: &Profile, time: f64) {
        floats.take_in(
            frame,
            time,
            profile,
            |words, _| vec![words.to_string()],
            |_| theme::TEXT,
        );
    }

    const ANN: u32 = 5;
    const KIND_SAY: u8 = 0;

    fn said(seq: u64, kind: u8, text: &str) -> WatchSpeech {
        WatchSpeech {
            seq,
            serial: ANN,
            kind,
            text: text.into(),
            ..WatchSpeech::default()
        }
    }

    fn modern() -> Profile {
        let mut profile = Profile::default();
        profile.interface.ui_style = UiStyle::Modern;
        profile
    }

    #[test]
    fn only_new_speech_of_a_mobile_floats() {
        let profile = modern();
        let mut floats = Floats::default();
        let mut frame = WatchFrame {
            speech: vec![said(1, KIND_SAY, "old words")],
            ..WatchFrame::default()
        };
        take_in(&mut floats, &frame, &profile, 0.0);
        assert!(
            floats.live.is_empty(),
            "words from before the window opened"
        );
        frame.speech.push(said(2, KIND_SAY, "hail"));
        frame.speech.push(said(3, SPEECH_SYSTEM, "You see: Ann"));
        frame.speech.push(said(4, SPEECH_LABEL, "Ann"));
        take_in(&mut floats, &frame, &profile, 1.0);
        take_in(&mut floats, &frame, &profile, 1.1);
        assert_eq!(
            floats.live.len(),
            1,
            "labels float in the Classic style only"
        );
        assert_eq!(floats.live[0].words, "hail");
        assert_eq!(floats.live[0].look, speech_look(&profile.fonts, 0));
        let life = floats.live[0].seconds;
        take_in(&mut floats, &frame, &profile, 1.0 + life);
        assert!(floats.live.is_empty());
    }

    #[test]
    fn a_hit_floats_as_a_number_with_the_damage_per_second() {
        let mut profile = modern();
        let mut floats = Floats::default();
        let mut frame = WatchFrame::default();
        take_in(&mut floats, &frame, &profile, 0.0);
        frame.cues.push(WatchCue {
            seq: 1,
            serial: ANN,
            kind: WatchCueKind::Damage(15),
        });
        take_in(&mut floats, &frame, &profile, 0.5);
        assert_eq!(floats.live[0].words, "15 (DPS: 1.0)");
        assert!(floats.live[0].number);
        profile.combat.dps_with_damage = false;
        frame.cues.push(WatchCue {
            seq: 2,
            serial: ANN,
            kind: WatchCueKind::Damage(12),
        });
        take_in(&mut floats, &frame, &profile, 0.6);
        assert_eq!(floats.live[1].words, "12");
    }

    #[test]
    fn newer_words_stand_nearest_the_head_and_a_number_rises() {
        const HEAD: Point = Point::new(50.0, 100.0);
        const LINE: f32 = 10.0;
        let float = |words: &str, born: f64, number: bool| Float {
            serial: 7,
            words: words.into(),
            look: TextLook::ascii(DAMAGE_FONT, 0),
            color: Rgba::WHITE,
            born,
            seconds: 5.0,
            number,
            measured: true,
        };
        let live = [
            float("old", 0.0, false),
            float("new", 1.0, false),
            float("12", 1.0, true),
            Float {
                serial: 8,
                ..float("gone", 1.0, false)
            },
        ];
        let heads = |at: usize| (live[at].serial == 7).then_some(HEAD);
        let placed = lay_out(&live, 2.0, false, heads, |_| LINE);
        let bottom_of = |at: usize| placed.iter().find(|p| p.index == at).map(|p| p.bottom);
        assert_eq!(bottom_of(1), Some(Point::new(50.0, 100.0 - SPEECH_LIFT)));
        assert_eq!(
            bottom_of(0),
            Some(Point::new(50.0, 100.0 - SPEECH_LIFT - LINE))
        );
        assert_eq!(
            bottom_of(2),
            Some(Point::new(50.0, 100.0 - DAMAGE_RISE_PER_SECOND))
        );
        assert_eq!(bottom_of(3), None, "no head drawn, no float");
        assert_eq!(placed[0].index, 2, "the newest first");
    }

    #[test]
    fn words_measured_late_stay_as_long_as_their_lines() {
        let mut profile = modern();
        profile.speech.scale_speech_delay = true;
        let mut floats = Floats::default();
        let mut frame = WatchFrame::default();
        let lines = |count: usize| move |_: &str, _: &TextLook| vec![String::new(); count];
        floats.take_in(&frame, 0.0, &profile, lines(0), |_| theme::TEXT);
        frame.speech.push(said(1, KIND_SAY, "a long line"));
        floats.take_in(&frame, 1.0, &profile, lines(0), |_| theme::TEXT);
        let one_line = speech_seconds(&profile.speech, 1);
        assert_eq!(floats.live()[0].seconds, one_line, "not measured yet");
        floats.take_in(&frame, 1.1, &profile, lines(3), |_| theme::TEXT);
        assert_eq!(floats.live()[0].seconds, speech_seconds(&profile.speech, 3));
        assert_eq!(floats.live()[0].born, 1.0, "it keeps its start");
    }

    #[test]
    fn the_speech_page_sets_how_long_words_stay() {
        const DELAY: u16 = 100;
        let mut speech = SpeechOptions {
            speech_delay: DELAY,
            ..SpeechOptions::default()
        };
        assert_eq!(speech_seconds(&speech, 1), SCALED_LINE_SECONDS);
        assert_eq!(speech_seconds(&speech, 2), SCALED_LINE_SECONDS * 2.0);
        speech.scale_speech_delay = false;
        assert_eq!(speech_seconds(&speech, 3), 4.0);
    }

    #[test]
    fn party_guild_and_ignored_lines_follow_the_speech_page() {
        let mut profile = Profile::default();
        let party = WatchSpeech {
            kind: SPEECH_KIND_PARTY,
            ..said(1, KIND_SAY, "heal me")
        };
        assert_eq!(floating_hue(&party, &profile, true), None);
        profile.speech.overhead_party_messages = true;
        assert_eq!(
            floating_hue(&party, &profile, true),
            Some(profile.speech.party_hue)
        );
        let guild = WatchSpeech {
            kind: SPEECH_GUILD,
            hue: 0x0035,
            ..said(2, KIND_SAY, "hi")
        };
        assert_eq!(floating_hue(&guild, &profile, true), Some(0x0035));
        profile.speech.hide_guild_chat = true;
        assert_eq!(floating_hue(&guild, &profile, true), None);
        let rude = WatchSpeech {
            name: "Troll".into(),
            ..said(3, KIND_SAY, "boo")
        };
        profile.ignore.names.push("Troll".into());
        assert_eq!(floating_hue(&rude, &profile, true), None);
    }

    #[test]
    fn long_words_are_cut_and_a_float_fades_at_its_end_only_when_fading_is_on() {
        let long = "a".repeat(SPEECH_MAX_CHARS + 5);
        assert_eq!(
            shortened(&long).chars().count(),
            SPEECH_MAX_CHARS + ELLIPSIS.len()
        );
        assert_eq!(alpha(0.0, 5.0, true), 1.0);
        assert_eq!(alpha(5.0, 5.0, true), 0.0);
        assert_eq!(alpha(5.0, 5.0, false), 1.0);
    }

    #[test]
    fn the_fonts_page_picks_the_speech_font() {
        const HUE: u16 = 0x0035;
        let mut fonts = FontOptions::default();
        let look = speech_look(&fonts, HUE);
        assert_eq!(look.font, UoFont::Unicode(fonts.speech_font));
        assert!(look.style.border);
        assert_eq!(look.width, Some(SPEECH_WRAP));
        fonts.override_game_font = true;
        fonts.game_font_kind = GameFontKind::Ascii;
        assert_eq!(
            speech_look(&fonts, HUE).font,
            UoFont::Ascii(fonts.speech_font)
        );
    }
}
