//! What lies over the map for a moment or for an hour: the pictures of
//! spells, the dark of the night, rain and snow. Here is where each one is
//! and how long it lasts; the window paints them.

use crate::frame::{WatchCueKind, WatchEffect, WatchFrame};
use crate::geom::{Area, Point, Rgba, Vector};

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
pub const LIGHTNING_WIDTH: f32 = 3.0;
const LIGHTNING_JOINTS: usize = 7;
const LIGHTNING_SWAY: f32 = 26.0;
/// The bolt sways by its own scatter, salted by the tenth of a second it
/// struck in.
const LIGHTNING_SALT_PER_SECOND: f64 = 10.0;
/// An effect stands in the middle of the body, not at the feet.
pub const BODY_LIFT: f32 = 22.0;

/// The tint of a storm.
const NIGHT: Rgba = Rgba::from_rgb(8, 12, 40);

const WEATHER_SNOW: u8 = 2;
const WEATHER_STORM: u8 = 1;
/// The most drops the shard asks for. The window draws this share more,
/// because it is larger than the window of the game.
const DROPS_SCALE: f32 = 4.0;
const RAIN_SPEED: f32 = 900.0;
const RAIN_LENGTH: f32 = 16.0;
const RAIN_SLANT: f32 = 0.25;
/// The width of a streak of rain.
pub const RAIN_WIDTH: f32 = 1.0;
const SNOW_SPEED: f32 = 90.0;
pub const SNOW_RADIUS: f32 = 1.8;
const SNOW_SWAY: f32 = 14.0;
/// A flake sways on a wave of its own, started at a share of this many
/// radians.
const SNOW_SWAY_PHASE: f32 = 6.0;
const DROP_ALPHA: f32 = 0.55;
const RAIN: Rgba = Rgba::from_rgb(170, 190, 230);
const SNOW: Rgba = Rgba::from_rgb(240, 244, 250);
const STORM_TINT_ALPHA: f32 = 0.12;
/// The three uses of the scatter of one drop: where it is in its fall,
/// across, and in its sway.
const SALT_FALL: u32 = 1;
const SALT_ACROSS: u32 = 2;
const SALT_SWAY: u32 = 3;

/// One effect that shows now.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveEffect {
    pub source: u32,
    pub effect: WatchEffect,
    pub born: f64,
}

#[derive(Default)]
pub struct Sky {
    /// None until the first picture. What came before it does not show.
    last_cue: Option<u64>,
    live: Vec<LiveEffect>,
}

fn tiles(place: (u16, u16, i8)) -> [f32; 3] {
    [f32::from(place.0), f32::from(place.1), f32::from(place.2)]
}

/// How long an effect shows.
pub fn seconds_of(effect: &WatchEffect) -> f64 {
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
pub fn scatter(drop: usize, salt: u32) -> f32 {
    let mut mixed = (drop as u32) ^ salt.wrapping_mul(SCATTER_SALT_MIX);
    mixed ^= mixed >> SCATTER_SHIFT_WIDE;
    mixed = mixed.wrapping_mul(SCATTER_MIX_A);
    mixed ^= mixed >> SCATTER_SHIFT_NARROW;
    mixed = mixed.wrapping_mul(SCATTER_MIX_B);
    mixed ^= mixed >> SCATTER_SHIFT_WIDE;
    (mixed & SCATTER_MASK) as f32 / SCATTER_MASK as f32
}

impl Sky {
    /// Takes the new effects of a frame, and lets the old ones end.
    pub fn take_in(&mut self, frame: &WatchFrame, time: f64) {
        let newest = frame.cues.iter().map(|cue| cue.seq).max();
        if let Some(seen) = self.last_cue {
            for cue in frame.cues.iter().filter(|cue| cue.seq > seen) {
                if let WatchCueKind::Effect(effect) = cue.kind {
                    self.live.push(LiveEffect {
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

    /// The effects that show now, the oldest first.
    pub fn live(&self) -> &[LiveEffect] {
        &self.live
    }
}

/// Where an effect shows now, in tiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectPlace {
    /// A bolt from the top of the window down to the one it strikes.
    Bolt([f32; 3]),
    /// The picture of the effect.
    Picture([f32; 3]),
}

/// Where an effect shows at `time`. `place_of` gives where a mobile is
/// drawn now, so a spell flies to where its target is seen.
pub fn effect_place(
    live: &LiveEffect,
    time: f64,
    place_of: impl Fn(u32) -> Option<[f32; 3]>,
) -> EffectPlace {
    let effect = &live.effect;
    let from = place_of(live.source).unwrap_or_else(|| tiles(effect.from));
    if effect.kind == EFFECT_LIGHTNING {
        return EffectPlace::Bolt(from);
    }
    let age = ((time - live.born) / seconds_of(effect)) as f32;
    EffectPlace::Picture(match effect.kind {
        EFFECT_MOVING => {
            let to = place_of(effect.target).unwrap_or_else(|| tiles(effect.to));
            [0, 1, 2].map(|i| from[i] + (to[i] - from[i]) * age)
        }
        EFFECT_ON_MOBILE => from,
        _ => tiles(effect.from),
    })
}

/// The joints of a bolt from the top of `area` down to the point it
/// strikes. One bolt keeps its shape while it shows.
pub fn lightning_bolt(area: Area, struck: Point, born: f64) -> Vec<Point> {
    let salt = (born * LIGHTNING_SALT_PER_SECOND) as u32;
    (0..=LIGHTNING_JOINTS)
        .map(|joint| {
            let share = joint as f32 / LIGHTNING_JOINTS as f32;
            let sway = if joint == LIGHTNING_JOINTS {
                0.0
            } else {
                (scatter(joint, salt) - 0.5) * 2.0 * LIGHTNING_SWAY
            };
            Point::new(
                struck.x + sway,
                area.min.y + (struck.y - area.min.y) * share,
            )
        })
        .collect()
}

/// One drop of the weather.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drop {
    /// A flake of snow, by its middle.
    Flake(Point),
    /// A streak of rain, from its tail to its head.
    Streak(Point, Point),
}

/// The tint a kind of weather lays over the world. Only a storm has one.
pub fn storm_tint(kind: u8) -> Option<Rgba> {
    (kind == WEATHER_STORM).then(|| NIGHT.with_alpha(STORM_TINT_ALPHA))
}

/// The color of the drops of a kind of weather.
pub fn drop_color(kind: u8) -> Rgba {
    let color = if kind == WEATHER_SNOW { SNOW } else { RAIN };
    color.with_alpha(DROP_ALPHA)
}

/// The drops of `count` asked by the shard, over `area` at `time`.
pub fn weather_drops(area: Area, kind: u8, count: u8, time: f64) -> Vec<Drop> {
    let snow = kind == WEATHER_SNOW;
    let speed = if snow { SNOW_SPEED } else { RAIN_SPEED };
    let drops = (f32::from(count) * DROPS_SCALE) as usize;
    (0..drops)
        .map(|drop| {
            let fall = (scatter(drop, SALT_FALL) + time as f32 * speed / area.height()).fract();
            let x = area.min.x + scatter(drop, SALT_ACROSS) * area.width();
            let y = area.min.y + fall * area.height();
            if snow {
                let sway =
                    (time as f32 + scatter(drop, SALT_SWAY) * SNOW_SWAY_PHASE).sin() * SNOW_SWAY;
                Drop::Flake(Point::new(x + sway, y))
            } else {
                let tail = Vector::new(-RAIN_LENGTH * RAIN_SLANT, -RAIN_LENGTH);
                let head = Point::new(x + fall * area.height() * -RAIN_SLANT, y);
                Drop::Streak(head + tail, head)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::WatchCue;

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
            let gap = (scatter(drop, SALT_ACROSS) - scatter(drop, SALT_FALL)).rem_euclid(1.0);
            bands[((gap * BANDS as f32) as usize).min(BANDS - 1)] += 1;
        }
        assert!(
            bands.iter().all(|&n| n >= FEWEST_IN_A_BAND),
            "the gaps between across and down spread out: {bands:?}"
        );
    }

    #[test]
    fn a_spell_flies_to_where_its_target_is_seen() {
        let live = LiveEffect {
            source: 5,
            effect: fireball(10),
            born: 0.0,
        };
        let seen = |serial: u32| (serial == 0).then_some([200.0, 200.0, 0.0]);
        let half = seconds_of(&live.effect) / 2.0;
        assert_eq!(
            effect_place(&live, half, seen),
            EffectPlace::Picture([150.0, 150.0, 0.0])
        );
        let bolt = LiveEffect {
            effect: WatchEffect {
                kind: EFFECT_LIGHTNING,
                ..fireball(0)
            },
            ..live
        };
        assert_eq!(
            effect_place(&bolt, 0.0, |_| None),
            EffectPlace::Bolt([100.0, 100.0, 0.0])
        );
    }

    #[test]
    fn a_bolt_falls_from_the_top_to_the_struck_point() {
        let area = Area::from_min_size(Point::new(0.0, 10.0), Vector::new(400.0, 300.0));
        let struck = Point::new(200.0, 250.0);
        let bolt = lightning_bolt(area, struck, 1.5);
        assert_eq!(bolt.len(), LIGHTNING_JOINTS + 1);
        assert_eq!(bolt[0].y, area.min.y);
        assert_eq!(*bolt.last().unwrap(), struck);
        assert_eq!(
            bolt,
            lightning_bolt(area, struck, 1.5),
            "one bolt keeps its shape"
        );
    }

    #[test]
    fn the_weather_scales_its_drops_and_only_a_storm_tints() {
        let area = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(100.0, 100.0));
        let snow = weather_drops(area, WEATHER_SNOW, 10, 0.5);
        assert_eq!(snow.len(), 40);
        assert!(matches!(snow[0], Drop::Flake(_)));
        assert!(matches!(
            weather_drops(area, WEATHER_STORM, 1, 0.5)[0],
            Drop::Streak(..)
        ));
        assert!(storm_tint(WEATHER_STORM).is_some());
        assert!(storm_tint(WEATHER_SNOW).is_none());
    }
}
