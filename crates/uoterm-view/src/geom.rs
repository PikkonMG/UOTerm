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

    pub const fn translate(&self, offset: Vector) -> Self {
        Self {
            min: Point::new(self.min.x + offset.x, self.min.y + offset.y),
            max: Point::new(self.max.x + offset.x, self.max.y + offset.y),
        }
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

impl Rgba {
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
    /// premultiplied channel scales, so the colour stays valid.
    pub fn with_alpha(self, factor: f32) -> Self {
        Self(
            self.0
                .map(|channel| (f32::from(channel) * factor).round() as u8),
        )
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
    fn alpha_scales_every_premultiplied_channel() {
        let half = Rgba([200, 100, 50, 255]).with_alpha(0.5);
        assert_eq!(half, Rgba([100, 50, 25, 128]));
    }
}
