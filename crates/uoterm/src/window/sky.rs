//! The window paints what lies over the map: the pictures of spells, the
//! dark of the night, rain and snow. Where each one is and how long it lasts
//! are `uoterm_view::sky`.

pub use uoterm_view::sky::*;

use super::bridge;
use super::scene::Scene;
use super::settings::VideoOptions;
use crate::view::WatchFrame;
use eframe::egui::{self, Color32, Painter, Rect, Stroke, Vec2};

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
    let mut lit = Vec::new();
    for live in sky.live() {
        let place = match effect_place(live, time, |serial| scene.place_of(frame, serial)) {
            EffectPlace::Bolt(struck) => {
                let struck = bridge::point(scene.screen_of(rect, struck));
                let bolt = lightning_bolt(bridge::area(rect), struck, live.born);
                painter.add(egui::Shape::line(
                    bolt.into_iter().map(bridge::pos2).collect(),
                    Stroke::new(LIGHTNING_WIDTH, Color32::WHITE),
                ));
                continue;
            }
            EffectPlace::Picture(place) => place,
        };
        let effect = &live.effect;
        lit.push((place, effect.graphic));
        let Some((texture, sprite)) = scene.item_picture(effect.graphic, effect.hue) else {
            continue;
        };
        let zoom = scene.zoom();
        let center = scene.screen_of(rect, place) - Vec2::new(0.0, BODY_LIFT * zoom);
        let area = Rect::from_center_size(center, Vec2::new(sprite.width, sprite.height) * zoom);
        painter.image(texture, area, bridge::rect(sprite.uv), Color32::WHITE);
    }
    let weather_shows = frame.weather.filter(|_| video.weather_effects);
    if let Some((kind, count)) = weather_shows {
        weather(painter, rect, kind, count, time);
    }
    scene.draw_lights(painter, rect, frame, &lit);
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
