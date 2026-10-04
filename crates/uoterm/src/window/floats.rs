//! The window draws the words and numbers that float over heads. What
//! floats and for how long is `uoterm_view::floats`.

pub use uoterm_view::floats::*;

use super::bridge;
use super::scene::Scene;
use super::settings::{Profile, UiStyle};
use super::theme::{self, number_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{Align2, Painter, Pos2, Rect, Vec2};

/// Draws each float over its mobile or thing. True while one still shows.
pub fn draw(
    floats: &mut Floats,
    painter: &Painter,
    rect: Rect,
    frame: &WatchFrame,
    scene: &mut Scene,
    time: f64,
    profile: &Profile,
) -> bool {
    floats.take_in(
        frame,
        time,
        profile,
        |words, look| scene.text_lines(words, look),
        |hue| bridge::rgba(scene.words_color(hue)),
    );
    let classic = profile.interface.ui_style == UiStyle::Classic;
    let fading = profile.general.text_fading;
    // The newest words of a mobile are nearest his head.
    let mut lines_over: Vec<(u32, f32)> = Vec::new();
    for float in floats.live().iter().rev() {
        let Some(head) = scene.head_of(rect, frame, float.serial) else {
            continue;
        };
        let age = time - float.born;
        let color = bridge::color(float.color);
        if float.number {
            let rise = DAMAGE_RISE_PER_SECOND * age as f32;
            let at = head - Vec2::new(0.0, rise);
            if classic {
                let words = scene.words(painter, &float.words, float.look);
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
                    theme::with_alpha(color, alpha(age, float.seconds, true)),
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
            let words = scene.words(painter, &float.words, float.look);
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
                theme::with_alpha(color, shown),
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
    !floats.live().is_empty()
}
