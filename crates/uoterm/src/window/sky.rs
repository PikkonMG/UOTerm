//! What lies over the map for a moment or for an hour: the pictures of
//! spells, the dark of the night, rain and snow.

use super::scene::Scene;
use super::settings::VideoOptions;
use super::theme;
use crate::view::{WatchCueKind, WatchEffect, WatchFrame};
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, Vec2};

const EFFECT_MOVING: u8 = 0;
const EFFECT_LIGHTNING: u8 = 1;
const EFFECT_ON_MOBILE: u8 = 3;
/// How long a flying effect takes for one tile, and the least it takes.
const FLY_SECONDS_PER_TILE: f64 = 0.06;
const FLY_SECONDS_MIN: f64 = 0.25;
/// One tick of the duration of an effect that stays.
const DURATION_TICK_SECONDS: f64 = 0.05;
const STAY_SECONDS_MIN: f64 = 0.6;
const LIGHTNING_SECONDS: f64 = 0.45;
const LIGHTNING_WIDTH: f32 = 3.0;
const LIGHTNING_JOINTS: usize = 7;
const LIGHTNING_SWAY: f32 = 26.0;
/// An effect stands in the middle of the body, not at the feet.
const BODY_LIFT: f32 = 22.0;

/// The tint of a storm.
const NIGHT: Color32 = Color32::from_rgb(8, 12, 40);

const WEATHER_SNOW: u8 = 2;
const WEATHER_STORM: u8 = 1;
/// The most drops the shard asks for. The window draws this share more,
/// because it is larger than the window of the game.
const DROPS_SCALE: f32 = 4.0;
const RAIN_SPEED: f32 = 900.0;
const RAIN_LENGTH: f32 = 16.0;
const RAIN_SLANT: f32 = 0.25;
const SNOW_SPEED: f32 = 90.0;
const SNOW_RADIUS: f32 = 1.8;
const SNOW_SWAY: f32 = 14.0;
const DROP_ALPHA: f32 = 0.55;
const RAIN: Color32 = Color32::from_rgb(170, 190, 230);
const SNOW: Color32 = Color32::from_rgb(240, 244, 250);
const STORM_TINT_ALPHA: f32 = 0.12;

struct Live {
    source: u32,
    effect: WatchEffect,
    born: f64,
}

#[derive(Default)]
pub struct Sky {
    /// None until the first picture. What came before it does not show.
    last_cue: Option<u64>,
    live: Vec<Live>,
}

fn tiles(place: (u16, u16, i8)) -> [f32; 3] {
    [f32::from(place.0), f32::from(place.1), f32::from(place.2)]
}

/// How long an effect shows.
fn seconds_of(effect: &WatchEffect) -> f64 {
    match effect.kind {
        EFFECT_MOVING => {
            let far = effect
                .from
                .0
                .abs_diff(effect.to.0)
                .max(effect.from.1.abs_diff(effect.to.1));
            (f64::from(far) * FLY_SECONDS_PER_TILE).max(FLY_SECONDS_MIN)
        }
        EFFECT_LIGHTNING => LIGHTNING_SECONDS,
        _ => (f64::from(effect.duration) * DURATION_TICK_SECONDS).max(STAY_SECONDS_MIN),
    }
}

/// The constants of a 32-bit hash finalizer: every bit of the input moves
/// about half the bits of the output.
const SCATTER_SALT_MIX: u32 = 0x9E37_79B9;
const SCATTER_MIX_A: u32 = 0x85EB_CA6B;
const SCATTER_MIX_B: u32 = 0xC2B2_AE35;
const SCATTER_SHIFT_WIDE: u32 = 16;
const SCATTER_SHIFT_NARROW: u32 = 13;
const SCATTER_MASK: u32 = 0xFFFF;

/// A number from 0 to 1 that is the same each time for one drop and one
/// use, so a drop keeps its column while it falls. The uses of one drop
/// have no link to each other, so the drops scatter over the whole window
/// and do not stand in a line.
fn scatter(drop: usize, salt: u32) -> f32 {
    let mut mixed = (drop as u32) ^ salt.wrapping_mul(SCATTER_SALT_MIX);
    mixed ^= mixed >> SCATTER_SHIFT_WIDE;
    mixed = mixed.wrapping_mul(SCATTER_MIX_A);
    mixed ^= mixed >> SCATTER_SHIFT_NARROW;
    mixed = mixed.wrapping_mul(SCATTER_MIX_B);
    mixed ^= mixed >> SCATTER_SHIFT_WIDE;
    (mixed & SCATTER_MASK) as f32 / SCATTER_MASK as f32
}

impl Sky {
    fn take_in(&mut self, frame: &WatchFrame, time: f64) {
        let newest = frame.cues.iter().map(|cue| cue.seq).max();
        if let Some(seen) = self.last_cue {
            for cue in frame.cues.iter().filter(|cue| cue.seq > seen) {
                if let WatchCueKind::Effect(effect) = cue.kind {
                    self.live.push(Live {
                        source: cue.serial,
                        effect,
                        born: time,
                    });
                }
            }
        }
        self.last_cue = newest.or(self.last_cue).or(Some(0));
        self.live
            .retain(|live| time - live.born < seconds_of(&live.effect));
    }

    /// Draws the effects and the weather, then lays the light of the world
    /// over them, as the Video page says. True while something still moves.
    pub fn draw(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
        time: f64,
        video: &VideoOptions,
    ) -> bool {
        self.take_in(frame, time);
        let mut lit = Vec::new();
        for live in &self.live {
            let effect = &live.effect;
            let from = scene
                .place_of(frame, live.source)
                .unwrap_or_else(|| tiles(effect.from));
            let age = ((time - live.born) / seconds_of(effect)) as f32;
            if effect.kind == EFFECT_LIGHTNING {
                lightning(painter, rect, scene.screen_of(rect, from), live.born);
                continue;
            }
            let place = match effect.kind {
                EFFECT_MOVING => {
                    let to = scene
                        .place_of(frame, effect.target)
                        .unwrap_or_else(|| tiles(effect.to));
                    [0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * age)
                }
                EFFECT_ON_MOBILE => from,
                _ => tiles(effect.from),
            };
            lit.push((place, effect.graphic));
            let Some((texture, sprite)) = scene.item_picture(frame.map, effect.graphic, effect.hue)
            else {
                continue;
            };
            let zoom = scene.zoom();
            let center = scene.screen_of(rect, place) - Vec2::new(0.0, BODY_LIFT * zoom);
            let area =
                Rect::from_center_size(center, Vec2::new(sprite.width, sprite.height) * zoom);
            painter.image(texture, area, sprite.uv, Color32::WHITE);
        }
        let weather_shows = frame.weather.filter(|_| video.weather_effects);
        if let Some((kind, count)) = weather_shows {
            weather(painter, rect, kind, count, time);
        }
        scene.draw_lights(painter, rect, frame, &lit);
        !self.live.is_empty() || weather_shows.is_some()
    }
}

/// A bolt from the top of the window down to the one it strikes.
fn lightning(painter: &Painter, rect: Rect, struck: Pos2, born: f64) {
    let salt = (born * 10.0) as u32;
    let points: Vec<Pos2> = (0..=LIGHTNING_JOINTS)
        .map(|joint| {
            let share = joint as f32 / LIGHTNING_JOINTS as f32;
            let sway = if joint == LIGHTNING_JOINTS {
                0.0
            } else {
                (scatter(joint, salt) - 0.5) * 2.0 * LIGHTNING_SWAY
            };
            Pos2::new(
                struck.x + sway,
                rect.top() + (struck.y - rect.top()) * share,
            )
        })
        .collect();
    painter.add(egui::Shape::line(
        points,
        Stroke::new(LIGHTNING_WIDTH, Color32::WHITE),
    ));
}

fn weather(painter: &Painter, rect: Rect, kind: u8, count: u8, time: f64) {
    if kind == WEATHER_STORM {
        painter.rect_filled(rect, 0.0, theme::with_alpha(NIGHT, STORM_TINT_ALPHA));
    }
    let snow = kind == WEATHER_SNOW;
    let (speed, color) = if snow {
        (SNOW_SPEED, SNOW)
    } else {
        (RAIN_SPEED, RAIN)
    };
    let color = theme::with_alpha(color, DROP_ALPHA);
    let drops = (f32::from(count) * DROPS_SCALE) as usize;
    for drop in 0..drops {
        let fall = (scatter(drop, 1) + time as f32 * speed / rect.height()).fract();
        let x = rect.left() + scatter(drop, 2) * rect.width();
        let y = rect.top() + fall * rect.height();
        if snow {
            let sway = (time as f32 + scatter(drop, 3) * 6.0).sin() * SNOW_SWAY;
            painter.circle_filled(Pos2::new(x + sway, y), SNOW_RADIUS, color);
        } else {
            let tail = Vec2::new(-RAIN_LENGTH * RAIN_SLANT, -RAIN_LENGTH);
            let head = Pos2::new(x + fall * rect.height() * -RAIN_SLANT, y);
            painter.line_segment([head + tail, head], Stroke::new(1.0, color));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchCue;

    fn fireball(far: u16) -> WatchEffect {
        WatchEffect {
            kind: EFFECT_MOVING,
            target: 0,
            graphic: 0x36D4,
            hue: 0,
            from: (100, 100, 0),
            to: (100 + far, 100, 0),
            duration: 0,
        }
    }

    #[test]
    fn a_far_fireball_flies_longer_and_a_near_one_still_shows() {
        assert_eq!(seconds_of(&fireball(1)), FLY_SECONDS_MIN);
        assert!(seconds_of(&fireball(12)) > FLY_SECONDS_MIN);
        let sparkle = WatchEffect {
            kind: EFFECT_ON_MOBILE,
            duration: 40,
            ..fireball(0)
        };
        assert_eq!(seconds_of(&sparkle), 2.0);
    }

    #[test]
    fn only_a_new_effect_shows_and_it_ends() {
        let mut sky = Sky::default();
        let mut frame = WatchFrame::default();
        sky.take_in(&frame, 0.0);
        frame.cues.push(WatchCue {
            seq: 1,
            serial: 5,
            kind: WatchCueKind::Effect(fireball(1)),
        });
        sky.take_in(&frame, 1.0);
        sky.take_in(&frame, 1.1);
        assert_eq!(sky.live.len(), 1);
        sky.take_in(&frame, 1.0 + FLY_SECONDS_MIN * 2.0);
        assert!(sky.live.is_empty());
    }

    #[test]
    fn a_drop_keeps_its_column() {
        assert_eq!(scatter(7, 2), scatter(7, 2));
        assert_ne!(scatter(7, 2), scatter(8, 2));
        assert!((0.0..=1.0).contains(&scatter(12_345, 9)));
    }

    /// The place across and the place down of a drop have no link, so the
    /// rain fills the window and does not fall as one slanted line.
    #[test]
    fn the_drops_do_not_stand_in_a_line() {
        const DROPS: usize = 1_000;
        const BANDS: usize = 10;
        const FEWEST_IN_A_BAND: usize = DROPS / BANDS / 2;
        let mut bands = [0usize; BANDS];
        for drop in 0..DROPS {
            let gap = (scatter(drop, 2) - scatter(drop, 1)).rem_euclid(1.0);
            bands[((gap * BANDS as f32) as usize).min(BANDS - 1)] += 1;
        }
        assert!(
            bands.iter().all(|&n| n >= FEWEST_IN_A_BAND),
            "the gaps between across and down spread out: {bands:?}"
        );
    }
}
