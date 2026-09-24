//! Words and numbers that float over heads for a moment: what a mobile
//! said, the name a click asked for, and how many hits he lost. The session
//! sends each one with a number that counts up, so each one floats once.
//!
//! The Classic style draws them in the UO fonts, as the reference client does: speech
//! in a Unicode font with a black border, for as long as the Speech page
//! says, and damage in the small ASCII font. The Modern style keeps the
//! window's own font.

use super::classic::text::TextLook;
use super::model::casting::overhead_spell;
use super::model::journal::SPEECH_SPELL;
use super::scene::Scene;
use super::settings::{FontOptions, GameFontKind, Profile, SpeechOptions};
use super::theme::{self, number_font, title_font};
use crate::view::{WatchCueKind, WatchFrame, WatchSpeech};
use eframe::egui::{Align2, Color32, Painter, Pos2, Rect, Vec2};
use std::collections::HashMap;
use uoterm_nav::TextAlign;
use uoterm_protocol::types::{SPEECH_ALLIANCE, SPEECH_GUILD, SPEECH_LABEL, SPEECH_SYSTEM};
use uoterm_world::{SPEECH_KIND_PARTY, SPEECH_KIND_PARTY_PRIVATE};

const SPEECH_LINE: f32 = 18.0;
const SPEECH_MAX_CHARS: usize = 60;
/// Room between the head and the words, for the name plate.
const SPEECH_LIFT: f32 = 34.0;
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
const DAMAGE_RISE_PER_SECOND: f32 = 40.0;
const DAMAGE_SIZE: f32 = 20.0;
/// The ASCII font of damage numbers, and their hues.
const DAMAGE_FONT: u8 = 3;
const OWN_DAMAGE_HUE: u16 = 0x0034;
const DAMAGE_HUE: u16 = 0x0021;
/// Damage per second counts the damage of this many seconds.
const DPS_SECONDS: f64 = 15.0;
const ELLIPSIS: &str = "...";

struct Float {
    serial: u32,
    words: String,
    hue: u16,
    color: Color32,
    born: f64,
    seconds: f64,
    number: bool,
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

fn shortened(text: &str) -> String {
    if text.chars().count() <= SPEECH_MAX_CHARS {
        return text.to_string();
    }
    let cut: String = text.chars().take(SPEECH_MAX_CHARS).collect();
    format!("{cut}{ELLIPSIS}")
}

/// How much of a float shows, from 1 to 0 over its last moments. With
/// fading off it shows whole to its end.
fn alpha(age: f64, seconds: f64, fading: bool) -> f32 {
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
fn floating_hue(line: &WatchSpeech, profile: &Profile, classic: bool) -> Option<u16> {
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
    fn take_in(&mut self, frame: &WatchFrame, scene: &mut Scene, profile: &Profile, time: f64) {
        let classic = profile.interface.ui_style == super::settings::UiStyle::Classic;
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
                let lines = scene.text_lines(&words, speech_look(&profile.fonts, hue));
                self.live.push(Float {
                    serial: line.serial,
                    color: scene.words_color(hue),
                    words,
                    hue,
                    born: time,
                    seconds: speech_seconds(&profile.speech, lines),
                    number: false,
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
                self.live.push(Float {
                    serial: cue.serial,
                    words,
                    hue: if own { OWN_DAMAGE_HUE } else { DAMAGE_HUE },
                    color: if own { theme::ALARM } else { theme::WAITING },
                    born: time,
                    seconds: DAMAGE_SECONDS,
                    number: true,
                });
            }
        }
        self.last_cue = newest_cue.or(self.last_cue).or(Some(0));
        self.live.retain(|float| time - float.born < float.seconds);
        for taken in self.damage.values_mut() {
            taken.retain(|(at, _)| time - at < DPS_SECONDS);
        }
        self.damage.retain(|_, taken| !taken.is_empty());
    }

    /// Draws each float over its mobile or thing. True while one still
    /// shows.
    pub fn draw(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
        time: f64,
        profile: &Profile,
    ) -> bool {
        self.take_in(frame, scene, profile, time);
        let classic = profile.interface.ui_style == super::settings::UiStyle::Classic;
        let fading = profile.general.text_fading;
        // The newest words of a mobile are nearest his head.
        let mut lines_over: Vec<(u32, f32)> = Vec::new();
        for float in self.live.iter().rev() {
            let Some(head) = scene.head_of(rect, frame, float.serial) else {
                continue;
            };
            let age = time - float.born;
            if float.number {
                let rise = DAMAGE_RISE_PER_SECOND * age as f32;
                let at = head - Vec2::new(0.0, rise);
                if classic {
                    let look = TextLook::ascii(DAMAGE_FONT, float.hue);
                    let words = scene.words(painter, &float.words, look);
                    words.paint(
                        painter,
                        at - Vec2::new(words.size().x / 2.0, words.size().y),
                        1.0,
                    );
                } else {
                    theme::shadowed_text(
                        painter,
                        at,
                        Align2::CENTER_BOTTOM,
                        &float.words,
                        number_font(DAMAGE_SIZE),
                        theme::with_alpha(float.color, alpha(age, float.seconds, true)),
                    );
                }
                continue;
            }
            let shown = alpha(age, float.seconds, fading);
            let stacked = lines_over
                .iter()
                .find(|(serial, _)| *serial == float.serial)
                .map_or(0.0, |(_, height)| *height);
            let bottom = Pos2::new(head.x, head.y - SPEECH_LIFT - stacked);
            let height = if classic {
                let look = speech_look(&profile.fonts, float.hue);
                let words = scene.words(painter, &float.words, look);
                let size = words.size();
                words.paint(painter, bottom - Vec2::new(size.x / 2.0, size.y), shown);
                size.y
            } else {
                theme::shadowed_text(
                    painter,
                    bottom,
                    Align2::CENTER_BOTTOM,
                    &float.words,
                    title_font(theme::SIZE_PLATE),
                    theme::with_alpha(float.color, shown),
                );
                SPEECH_LINE
            };
            match lines_over
                .iter_mut()
                .find(|(serial, _)| *serial == float.serial)
            {
                Some((_, stack)) => *stack += height,
                None => lines_over.push((float.serial, height)),
            }
        }
        !self.live.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::super::classic::text::UoFont;
    use super::*;
    use crate::view::WatchCue;

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
        profile.interface.ui_style = super::super::settings::UiStyle::Modern;
        profile
    }

    #[test]
    fn only_new_speech_of_a_mobile_floats() {
        let mut scene = Scene::new(None);
        let profile = modern();
        let mut floats = Floats::default();
        let mut frame = WatchFrame {
            speech: vec![said(1, KIND_SAY, "old words")],
            ..WatchFrame::default()
        };
        floats.take_in(&frame, &mut scene, &profile, 0.0);
        assert!(
            floats.live.is_empty(),
            "words from before the window opened"
        );
        frame.speech.push(said(2, KIND_SAY, "hail"));
        frame.speech.push(said(3, SPEECH_SYSTEM, "You see: Ann"));
        frame.speech.push(said(4, SPEECH_LABEL, "Ann"));
        floats.take_in(&frame, &mut scene, &profile, 1.0);
        floats.take_in(&frame, &mut scene, &profile, 1.1);
        assert_eq!(
            floats.live.len(),
            1,
            "labels float in the Classic style only"
        );
        assert_eq!(floats.live[0].words, "hail");
        let life = floats.live[0].seconds;
        floats.take_in(&frame, &mut scene, &profile, 1.0 + life);
        assert!(floats.live.is_empty());
    }

    #[test]
    fn a_hit_floats_as_a_number_with_the_damage_per_second() {
        let mut scene = Scene::new(None);
        let mut profile = modern();
        let mut floats = Floats::default();
        let mut frame = WatchFrame::default();
        floats.take_in(&frame, &mut scene, &profile, 0.0);
        frame.cues.push(WatchCue {
            seq: 1,
            serial: ANN,
            kind: WatchCueKind::Damage(15),
        });
        floats.take_in(&frame, &mut scene, &profile, 0.5);
        assert_eq!(floats.live[0].words, "15 (DPS: 1.0)");
        assert!(floats.live[0].number);
        profile.combat.dps_with_damage = false;
        frame.cues.push(WatchCue {
            seq: 2,
            serial: ANN,
            kind: WatchCueKind::Damage(12),
        });
        floats.take_in(&frame, &mut scene, &profile, 0.6);
        assert_eq!(floats.live[1].words, "12");
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
