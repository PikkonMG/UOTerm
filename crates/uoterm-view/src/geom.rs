//! Plain geometry and colour types. They mirror what the window draws with,
//! so the shared rules do not depend on egui.

use serde::{Deserialize, Serialize};
use std::ops::{Add, Div, Mul, Sub};

/// A position in screen points.
#[derive(Copy, Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Point) -> f32 {
        (self - other).length()
    }
}

/// A distance or an offset in screen points.
#[derive(Copy, Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
}

impl Vector {
    pub const ZERO: Vector = Vector::new(0.0, 0.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn splat(both: f32) -> Self {
        Self::new(both, both)
    }

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }
}

impl Add<Vector> for Point {
    type Output = Point;

    fn add(self, offset: Vector) -> Point {
        Point::new(self.x + offset.x, self.y + offset.y)
    }
}

impl Sub for Point {
    type Output = Vector;

    fn sub(self, other: Point) -> Vector {
        Vector::new(self.x - other.x, self.y - other.y)
    }
}

impl Add for Vector {
    type Output = Vector;

    fn add(self, other: Vector) -> Vector {
        Vector::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Vector {
    type Output = Vector;

    fn sub(self, other: Vector) -> Vector {
        Vector::new(self.x - other.x, self.y - other.y)
    }
}

impl Sub<Vector> for Point {
    type Output = Point;

    fn sub(self, offset: Vector) -> Point {
        Point::new(self.x - offset.x, self.y - offset.y)
    }
}

impl Mul<f32> for Vector {
    type Output = Vector;

    fn mul(self, factor: f32) -> Vector {
        Vector::new(self.x * factor, self.y * factor)
    }
}

impl Div<f32> for Vector {
    type Output = Vector;

    fn div(self, divisor: f32) -> Vector {
        Vector::new(self.x / divisor, self.y / divisor)
    }
}

/// A rectangle with both corners inside it.
#[derive(Copy, Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Area {
    pub min: Point,
    pub max: Point,
}

impl Area {
    pub const fn from_min_size(min: Point, size: Vector) -> Self {
        Self {
            min,
            max: Point::new(min.x + size.x, min.y + size.y),
        }
    }

    /// The smallest area that holds every point.
    pub fn from_points(points: &[Point]) -> Self {
        let nothing = Self {
            min: Point::new(f32::INFINITY, f32::INFINITY),
            max: Point::new(-f32::INFINITY, -f32::INFINITY),
        };
        points.iter().fold(nothing, |area, point| Self {
            min: Point::new(area.min.x.min(point.x), area.min.y.min(point.y)),
            max: Point::new(area.max.x.max(point.x), area.max.y.max(point.y)),
        })
    }

    /// The smallest area that holds both points, whichever corners they are.
    pub fn from_two_points(a: Point, b: Point) -> Self {
        Self {
            min: Point::new(a.x.min(b.x), a.y.min(b.y)),
            max: Point::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    pub const fn from_center_size(center: Point, size: Vector) -> Self {
        let half = Vector::new(size.x * 0.5, size.y * 0.5);
        Self {
            min: Point::new(center.x - half.x, center.y - half.y),
            max: Point::new(center.x + half.x, center.y + half.y),
        }
    }

    pub const fn contains(&self, point: Point) -> bool {
        self.min.x <= point.x
            && point.x <= self.max.x
            && self.min.y <= point.y
            && point.y <= self.max.y
    }

    pub const fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    pub const fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    pub const fn size(&self) -> Vector {
        Vector::new(self.width(), self.height())
    }

    pub const fn center(&self) -> Point {
        Point::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    /// The middle of the top edge.
    pub const fn center_top(&self) -> Point {
        Point::new(self.center().x, self.min.y)
    }

    /// The middle of the bottom edge.
    pub const fn center_bottom(&self) -> Point {
        Point::new(self.center().x, self.max.y)
    }

    /// True when the areas meet, edges included.
    pub const fn intersects(&self, other: Area) -> bool {
        self.min.x <= other.max.x
            && other.min.x <= self.max.x
            && self.min.y <= other.max.y
            && other.min.y <= self.max.y
    }

    /// How far the point is from the area: zero inside it. An area with a
    /// negative size is infinitely far.
    pub fn distance_to(&self, point: Point) -> f32 {
        if self.min.x > self.max.x || self.min.y > self.max.y {
            return f32::INFINITY;
        }
        let outside = |low: f32, high: f32, at: f32| {
            if low > at {
                low - at
            } else if at > high {
                at - high
            } else {
                0.0
            }
        };
        let dx = outside(self.min.x, self.max.x, point.x);
        let dy = outside(self.min.y, self.max.y, point.y);
        (dx * dx + dy * dy).sqrt()
    }

    /// The area moved by `offset`, its size kept as egui keeps it.
    pub const fn translate(&self, offset: Vector) -> Self {
        Self::from_min_size(
            Point::new(self.min.x + offset.x, self.min.y + offset.y),
            self.size(),
        )
    }

    /// The part both areas cover. Areas that do not meet give an area with
    /// a negative size.
    pub fn intersect(&self, other: Area) -> Self {
        Self {
            min: Point::new(self.min.x.max(other.min.x), self.min.y.max(other.min.y)),
            max: Point::new(self.max.x.min(other.max.x), self.max.y.min(other.max.y)),
        }
    }

    /// The area grown by `amount` on every side. A negative amount shrinks it.
    pub const fn expand(&self, amount: f32) -> Self {
        Self {
            min: Point::new(self.min.x - amount, self.min.y - amount),
            max: Point::new(self.max.x + amount, self.max.y + amount),
        }
    }
}

/// A premultiplied colour, red, green, blue and alpha, as egui keeps it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Rgba(pub [u8; 4]);

/// The sRGB curve, as egui works it out, so a colour made here is the
/// colour the window makes.
const SRGB_LINEAR_LIMIT: u8 = 10;
const SRGB_LINEAR_SLOPE: f32 = 3294.6;
const SRGB_OFFSET: f32 = 14.025;
const SRGB_SCALE: f32 = 269.025;
const SRGB_GAMMA: f32 = 2.4;
const SRGB_LINEAR_TOP: f32 = 0.0031308;
const CHANNEL_MAX: f32 = 255.0;
/// A channel rounds to the nearest whole value.
const HALF: f32 = 0.5;

/// A channel of an sRGB colour, as light from 0 to 1.
fn linear_of(channel: u8) -> f32 {
    if channel <= SRGB_LINEAR_LIMIT {
        f32::from(channel) / SRGB_LINEAR_SLOPE
    } else {
        ((f32::from(channel) + SRGB_OFFSET) / SRGB_SCALE).powf(SRGB_GAMMA)
    }
}

/// Light from 0 to 1 as a channel of an sRGB colour.
fn channel_of(linear: f32) -> u8 {
    if linear <= 0.0 {
        0
    } else if linear <= SRGB_LINEAR_TOP {
        (SRGB_LINEAR_SLOPE * linear + HALF) as u8
    } else if linear <= 1.0 {
        (SRGB_SCALE * linear.powf(1.0 / SRGB_GAMMA) - SRGB_OFFSET + HALF) as u8
    } else {
        u8::MAX
    }
}

impl Rgba {
    pub const WHITE: Rgba = Rgba::from_rgb(u8::MAX, u8::MAX, u8::MAX);
    pub const TRANSPARENT: Rgba = Rgba([0; 4]);

    /// Black that covers this much of what is under it.
    pub const fn from_black_alpha(alpha: u8) -> Self {
        Self([0, 0, 0, alpha])
    }

    /// An opaque colour.
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, u8::MAX])
    }

    pub const fn from_rgba_premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    pub const fn to_array(self) -> [u8; 4] {
        self.0
    }

    /// The colour faded by `factor` (0.0 clear, 1.0 unchanged). Every
    /// premultiplied channel scales, so the colour stays valid. It rounds
    /// as egui's `gamma_multiply` does.
    pub fn with_alpha(self, factor: f32) -> Self {
        Self(
            self.0
                .map(|channel| (f32::from(channel) * factor + HALF) as u8),
        )
    }

    /// The same colour with nothing of what is under it showing through.
    pub fn to_opaque(self) -> Self {
        let [red, green, blue, alpha] = self.0;
        let alpha = f32::from(alpha) / CHANNEL_MAX;
        let unmultiplied = |channel: u8| {
            let light = linear_of(channel);
            channel_of(if alpha == 0.0 { light } else { light / alpha })
        };
        Self::from_rgb(unmultiplied(red), unmultiplied(green), unmultiplied(blue))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_area_holds_its_far_corner() {
        let area = Area::from_min_size(Point::new(1.0, 2.0), Vector::new(3.0, 4.0));
        assert!(area.contains(Point::new(4.0, 6.0)));
        assert!(!area.contains(Point::new(4.1, 6.0)));
    }

    #[test]
    fn two_points_make_an_area_whichever_corners_they_are() {
        let area = Area::from_two_points(Point::new(5.0, 1.0), Point::new(2.0, 4.0));
        assert_eq!(area.min, Point::new(2.0, 1.0));
        assert_eq!(area.max, Point::new(5.0, 4.0));
    }

    #[test]
    fn an_area_moves_by_a_vector() {
        let area = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(2.0, 2.0));
        let moved = area.translate(Vector::new(5.0, -1.0));
        assert_eq!(moved.min, Point::new(5.0, -1.0));
        assert_eq!(moved.size(), Vector::new(2.0, 2.0));
    }

    #[test]
    fn an_area_meets_another_and_measures_a_point() {
        let area = Area::from_points(&[Point::new(4.0, 1.0), Point::new(0.0, 3.0)]);
        assert_eq!(
            area,
            Area::from_two_points(Point::new(0.0, 1.0), Point::new(4.0, 3.0))
        );
        assert_eq!(area.center_top(), Point::new(2.0, 1.0));
        assert_eq!(area.center_bottom(), Point::new(2.0, 3.0));
        assert!(area.intersects(area.translate(Vector::new(4.0, 0.0))));
        assert!(!area.intersects(area.translate(Vector::new(4.5, 0.0))));
        assert_eq!(area.distance_to(Point::new(7.0, 7.0)), 5.0);
        assert_eq!(area.distance_to(area.center()), 0.0);
    }

    #[test]
    fn an_opaque_colour_keeps_its_own_light() {
        assert_eq!(Rgba::WHITE.to_opaque(), Rgba::WHITE);
        // Half of white over nothing is a grey that covers all under it.
        let half = Rgba::WHITE.with_alpha(0.5);
        let lifted = half.to_opaque();
        assert!(lifted.0[0] > half.0[0] && lifted.0[0] < u8::MAX);
        assert_eq!(lifted.0[3], u8::MAX);
        assert_eq!(Rgba::TRANSPARENT.to_opaque(), Rgba::from_rgb(0, 0, 0));
        assert_eq!(
            Rgba::from_rgb(10, 20, 30).to_opaque(),
            Rgba::from_rgb(10, 20, 30)
        );
    }

    #[test]
    fn alpha_scales_every_premultiplied_channel() {
        let half = Rgba([200, 100, 50, 255]).with_alpha(0.5);
        assert_eq!(half, Rgba([100, 50, 25, 128]));
    }
}
