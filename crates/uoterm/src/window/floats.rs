//! The window draws the words and numbers that float over heads. What
//! floats and for how long is `uoterm_view::floats`.

pub use uoterm_view::floats::*;

use super::bridge;
use super::scene::{Scene, Words};
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
    let live = floats.live();
    let heads: Vec<Option<Pos2>> = live
        .iter()
        .map(|float| scene.head_of(rect, frame, float.serial))
        .collect();
    // The Classic style draws the words in UO fonts, as tall as they are.
    let words: Vec<Option<Words>> = live
        .iter()
        .zip(&heads)
        .map(|(float, head)| {
            head.filter(|_| classic)
                .map(|_| scene.words(painter, &float.words, float.look))
        })
        .collect();
    let placed = lay_out(
        live,
        time,
        fading,
        |at| heads[at].map(bridge::point),
        |at| {
            words[at]
                .as_ref()
                .map_or(SPEECH_LINE, |words| words.size().y)
        },
    );
    for place in placed {
        let float = &live[place.index];
        let bottom = bridge::pos2(place.bottom);
        if let Some(words) = &words[place.index] {
            let size = words.size();
            let shown = if float.number { 1.0 } else { place.alpha };
            words.paint(painter, bottom - Vec2::new(size.x / 2.0, size.y), shown);
            continue;
        }
        let font = if float.number {
            number_font(DAMAGE_SIZE)
        } else {
            title_font(theme::SIZE_PLATE)
        };
        theme::shadowed_text(
            painter,
            bottom,
            Align2::CENTER_BOTTOM,
            &float.words,
            font,
            theme::with_alpha(bridge::color(float.color), place.alpha),
        );
    }
    !live.is_empty()
}
