//! The window paints what lies over the map: the pictures of spells, the
//! dark of the night, rain and snow. Where each one is and how long it lasts
//! are `uoterm_view::sky`.

pub use uoterm_view::sky::*;

use super::bridge;
use super::scene::Scene;
use super::settings::VideoOptions;
use crate::view::WatchFrame;
use eframe::egui::{self, Color32, Painter, Rect, Stroke};

/// Draws the effects and the weather, then lays the light of the world
/// over them, as the Video page says. True while something still moves.
pub fn draw(
    sky: &mut Sky,
    painter: &Painter,
    rect: Rect,
    frame: &WatchFrame,
    scene: &mut Scene,
    time: f64,
    video: &VideoOptions,
) -> bool {
    sky.take_in(frame, time);
    let shown = shown_effects(sky, time, |serial| scene.place_of(frame, serial));
    for effect in &shown {
        match *effect {
            ShownEffect::Bolt { struck, born } => {
                let struck = bridge::point(scene.screen_of(rect, struck));
                let bolt = lightning_bolt(bridge::area(rect), struck, born);
                painter.add(egui::Shape::line(
                    bolt.into_iter().map(bridge::pos2).collect(),
                    Stroke::new(LIGHTNING_WIDTH, Color32::WHITE),
                ));
            }
            ShownEffect::Picture {
                place,
                graphic,
                hue,
            } => {
                let Some((texture, sprite)) = scene.item_picture(graphic, hue) else {
                    continue;
                };
                let foot = bridge::point(scene.screen_of(rect, place));
                let area = effect_area(foot, sprite.width, sprite.height, scene.zoom());
                painter.image(
                    texture,
                    bridge::rect(area),
                    bridge::rect(sprite.uv),
                    Color32::WHITE,
                );
            }
        }
    }
    let weather_shows = frame.weather.filter(|_| video.weather_effects);
    if let Some((kind, count)) = weather_shows {
        weather(painter, rect, kind, count, time);
    }
    scene.draw_lights(painter, rect, frame, &lit_effects(&shown));
    !sky.live().is_empty() || weather_shows.is_some()
}

fn weather(painter: &Painter, rect: Rect, kind: u8, count: u8, time: f64) {
    if let Some(tint) = storm_tint(kind) {
        painter.rect_filled(rect, 0.0, bridge::color(tint));
    }
    let color = bridge::color(drop_color(kind));
    for drop in weather_drops(bridge::area(rect), kind, count, time) {
        match drop {
            Drop::Flake(middle) => {
                painter.circle_filled(bridge::pos2(middle), SNOW_RADIUS, color);
            }
            Drop::Streak(tail, head) => {
                painter.line_segment(
                    [bridge::pos2(tail), bridge::pos2(head)],
                    Stroke::new(RAIN_WIDTH, color),
                );
            }
        }
    }
}
