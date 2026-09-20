//! Words and numbers that float over heads for a moment: what a mobile
//! said, and how many hits he lost. The session sends each one with a
//! number that counts up, so each one floats once.

use super::scene::Scene;
use super::theme::{self, number_font, title_font};
use crate::view::{WatchCueKind, WatchFrame};
use eframe::egui::{Align2, Color32, Painter, Pos2, Rect, Vec2};

const SPEECH_SECONDS: f64 = 5.0;
const SPEECH_FADE_SECONDS: f64 = 1.0;
const SPEECH_LINE: f32 = 18.0;
const SPEECH_MAX_CHARS: usize = 60;
/// Room between the head and the words, for the name plate.
const SPEECH_LIFT: f32 = 34.0;
const DAMAGE_SECONDS: f64 = 1.2;
const DAMAGE_RISE: f32 = 42.0;
const DAMAGE_SIZE: f32 = 20.0;
/// The message type of a line the shard wrote, not a mobile.
const KIND_SYSTEM: u8 = 1;
/// The message type of a name that a single click shows.
const KIND_LABEL: u8 = 6;
const ELLIPSIS: &str = "...";

struct Float {
    serial: u32,
    words: String,
    color: Color32,
    born: f64,
    number: bool,
}

impl Float {
    fn seconds(&self) -> f64 {
        if self.number {
            DAMAGE_SECONDS
        } else {
            SPEECH_SECONDS
        }
    }
}

#[derive(Default)]
pub struct Floats {
    /// None until the first picture. What came before it does not float.
    last_speech: Option<u64>,
    last_cue: Option<u64>,
    live: Vec<Float>,
}

fn shortened(text: &str) -> String {
    if text.chars().count() <= SPEECH_MAX_CHARS {
        return text.to_string();
    }
    let cut: String = text.chars().take(SPEECH_MAX_CHARS).collect();
    format!("{cut}{ELLIPSIS}")
}

/// How much of a float shows, from 1 to 0 over its last moments.
fn alpha(age: f64, seconds: f64) -> f32 {
    ((seconds - age) / SPEECH_FADE_SECONDS).clamp(0.0, 1.0) as f32
}

impl Floats {
    fn take_in(&mut self, frame: &WatchFrame, scene: &Scene, time: f64) {
        let newest_speech = frame.speech.iter().map(|line| line.seq).max();
        if let Some(seen) = self.last_speech {
            for line in frame.speech.iter().filter(|line| line.seq > seen) {
                if line.serial == 0 || matches!(line.kind, KIND_SYSTEM | KIND_LABEL) {
                    continue;
                }
                self.live.push(Float {
                    serial: line.serial,
                    words: shortened(&line.text),
                    color: scene.words_color(line.hue),
                    born: time,
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
                self.live.push(Float {
                    serial: cue.serial,
                    words: amount.to_string(),
                    color: if cue.serial == frame.serial {
                        theme::ALARM
                    } else {
                        theme::WAITING
                    },
                    born: time,
                    number: true,
                });
            }
        }
        self.last_cue = newest_cue.or(self.last_cue).or(Some(0));
        self.live
            .retain(|float| time - float.born < float.seconds());
    }

    /// Draws each float over its mobile. True while one still shows.
    pub fn draw(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        scene: &Scene,
        time: f64,
    ) -> bool {
        self.take_in(frame, scene, time);
        // The newest words of a mobile are nearest his head.
        let mut lines_over: Vec<(u32, f32)> = Vec::new();
        for float in self.live.iter().rev() {
            let Some(head) = scene.head_of(rect, frame, float.serial) else {
                continue;
            };
            let age = time - float.born;
            let color = theme::with_alpha(float.color, alpha(age, float.seconds()));
            if float.number {
                let rise = DAMAGE_RISE * (age / DAMAGE_SECONDS) as f32;
                theme::shadowed_text(
                    painter,
                    head - Vec2::new(0.0, rise),
                    Align2::CENTER_BOTTOM,
                    &float.words,
                    number_font(DAMAGE_SIZE),
                    color,
                );
                continue;
            }
            let stacked = match lines_over.iter_mut().find(|(s, _)| *s == float.serial) {
                Some((_, height)) => {
                    *height += SPEECH_LINE;
                    *height
                }
                None => {
                    lines_over.push((float.serial, 0.0));
                    0.0
                }
            };
            theme::shadowed_text(
                painter,
                Pos2::new(head.x, head.y - SPEECH_LIFT - stacked),
                Align2::CENTER_BOTTOM,
                &float.words,
                title_font(theme::SIZE_PLATE),
                color,
            );
        }
        !self.live.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchCue, WatchSpeech};

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

    #[test]
    fn only_new_speech_of_a_mobile_floats() {
        let scene = Scene::new(None);
        let mut floats = Floats::default();
        let mut frame = WatchFrame {
            speech: vec![said(1, KIND_SAY, "old words")],
            ..WatchFrame::default()
        };
        floats.take_in(&frame, &scene, 0.0);
        assert!(
            floats.live.is_empty(),
            "words from before the window opened"
        );
        frame.speech.push(said(2, KIND_SAY, "hail"));
        frame.speech.push(said(3, KIND_SYSTEM, "You see: Ann"));
        floats.take_in(&frame, &scene, 1.0);
        floats.take_in(&frame, &scene, 1.1);
        assert_eq!(floats.live.len(), 1);
        assert_eq!(floats.live[0].words, "hail");
        floats.take_in(&frame, &scene, 1.0 + SPEECH_SECONDS);
        assert!(floats.live.is_empty());
    }

    #[test]
    fn a_hit_floats_as_a_number() {
        let scene = Scene::new(None);
        let mut floats = Floats::default();
        let mut frame = WatchFrame::default();
        floats.take_in(&frame, &scene, 0.0);
        frame.cues.push(WatchCue {
            seq: 1,
            serial: ANN,
            kind: WatchCueKind::Damage(12),
        });
        floats.take_in(&frame, &scene, 0.5);
        assert_eq!(floats.live[0].words, "12");
        assert!(floats.live[0].number);
    }

    #[test]
    fn long_words_are_cut_and_a_float_fades_at_its_end() {
        let long = "a".repeat(SPEECH_MAX_CHARS + 5);
        assert_eq!(
            shortened(&long).chars().count(),
            SPEECH_MAX_CHARS + ELLIPSIS.len()
        );
        assert_eq!(alpha(0.0, SPEECH_SECONDS), 1.0);
        assert_eq!(alpha(SPEECH_SECONDS, SPEECH_SECONDS), 0.0);
    }
}
