//! The mesh the map is painted with, and the shapes laid on it: pictures
//! from the texture, their shadows, water that moves, and plain shapes
//! that read the white point of the texture.

use crate::art::Sprite;
use crate::geom::{Area, Point, Rgba, Vector};

/// A shadow is this dark, and its foot is this far over the foot of the
/// picture.
const SHADOW_ALPHA: f32 = 0.4;
const SHADOW_RISE: f32 = 10.0;
const SHADOW_SQUASH: f32 = 0.5;
/// A person on a chair leans his upper body this far, and is folded at
/// these shares of his height. His feet under the last one do not show.
const SIT_LEAN: f32 = 8.0;
const SIT_FOLDS: [f32; 4] = [0.0, 0.35, 0.60, 0.94];
/// Water that moves grows and shrinks by these shares.
const WATER_GROW: f32 = 1.1;
const WATER_SWAY: f32 = 0.1;
const WATER_SWAY_DOWN: f32 = 0.05;
/// The points round an ellipse.
const ELLIPSE_POINTS: usize = 24;
/// The indices of the two triangles of a quad, from its first corner.
const QUAD_TRIANGLES: [u32; 6] = [0, 1, 2, 0, 2, 3];

/// One corner of a triangle: where it is in points, the point of the
/// texture it reads, and its premultiplied color.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub rgba: [u8; 4],
}

/// Triangles in paint order: each three indices are one triangle, and a
/// later triangle covers an earlier one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty() && self.vertices.is_empty()
    }
}

/// A mesh being laid, with the white point of its texture.
pub(super) struct Canvas {
    pub mesh: Mesh,
    white: Point,
}

impl Canvas {
    pub fn new(white: Point) -> Self {
        Self {
            mesh: Mesh::default(),
            white,
        }
    }

    fn vertex(&mut self, pos: Point, uv: Point, color: Rgba) -> u32 {
        self.mesh.vertices.push(Vertex {
            pos: [pos.x, pos.y],
            uv: [uv.x, uv.y],
            rgba: color.to_array(),
        });
        self.mesh.vertices.len() as u32 - 1
    }

    pub fn quad(&mut self, points: [Point; 4], uvs: [Point; 4], color: Rgba) {
        self.quad_colors(points, uvs, [color; 4]);
    }

    /// A picture laid on four points, each with its own color.
    pub fn quad_colors(&mut self, points: [Point; 4], uvs: [Point; 4], colors: [Rgba; 4]) {
        let first = self.vertex(points[0], uvs[0], colors[0]);
        for i in 1..points.len() {
            self.vertex(points[i], uvs[i], colors[i]);
        }
        self.mesh
            .indices
            .extend(QUAD_TRIANGLES.map(|corner| first + corner));
    }

    /// A picture cut in a grid of cells. Each corner of a cell takes the
    /// share of `color` that `shown` gives for its place.
    pub fn sprite_grid(
        &mut self,
        sprite: Sprite,
        area: Area,
        color: Rgba,
        cell: f32,
        shown: impl Fn(Point) -> f32,
    ) {
        let columns = (area.width() / cell).ceil().max(1.0) as u32;
        let rows = (area.height() / cell).ceil().max(1.0) as u32;
        let first = self.mesh.vertices.len() as u32;
        let whole = sprite.uv;
        let (size, uv_size) = (area.size(), whole.size());
        for row in 0..=rows {
            for column in 0..=columns {
                let share = Vector::new(column as f32 / columns as f32, row as f32 / rows as f32);
                let pos = area.min + Vector::new(size.x * share.x, size.y * share.y);
                let uv = whole.min + Vector::new(uv_size.x * share.x, uv_size.y * share.y);
                self.vertex(pos, uv, color.with_alpha(shown(pos)));
            }
        }
        let stride = columns + 1;
        for row in 0..rows {
            for column in 0..columns {
                let top_left = first + row * stride + column;
                let bottom_left = top_left + stride;
                self.mesh.indices.extend([
                    top_left,
                    top_left + 1,
                    bottom_left + 1,
                    top_left,
                    bottom_left + 1,
                    bottom_left,
                ]);
            }
        }
    }

    /// The shadow of a picture, laid flat and slanted on the ground, as the
    /// classic client draws it.
    pub fn shadow(&mut self, sprite: Sprite, area: Area, zoom: f32, alpha: f32) {
        let width = area.width();
        let height = area.height() * SHADOW_SQUASH;
        let top = area.min.y + height - SHADOW_RISE * zoom;
        // The classic client slants it by its own height.
        let slant = height;
        let left = area.min.x;
        let points = [
            Point::new(left + slant, top),
            Point::new(left + slant + width, top),
            Point::new(left + width, top + height),
            Point::new(left, top + height),
        ];
        let color = Rgba::from_black_alpha((SHADOW_ALPHA * alpha * f32::from(u8::MAX)) as u8);
        self.quad(points, corners(sprite.uv), color);
    }

    /// Water that moves: the picture again over itself, grown and shrunk
    /// with the time.
    pub fn water(&mut self, sprite: Sprite, area: Area, color: Rgba, time: f64) {
        let time = time as f32;
        let grow = Vector::new(
            WATER_GROW + time.sin() * WATER_SWAY,
            WATER_GROW + time.cos() * WATER_SWAY_DOWN,
        );
        let size = area.size();
        let grown = Area::from_min_size(area.min, Vector::new(size.x * grow.x, size.y * grow.y));
        self.sprite(sprite, grown, color);
    }

    /// A round glow of `color` that fades to nothing at its edge.
    pub fn glow(&mut self, center: Point, radius: f32, color: Rgba) {
        let white = self.white;
        let middle = self.vertex(center, white, color);
        let first_edge = self.mesh.vertices.len() as u32;
        for point in ellipse(center, Vector::splat(radius)) {
            self.vertex(point, white, Rgba::TRANSPARENT);
        }
        let edges = ELLIPSE_POINTS as u32;
        for i in 0..edges {
            let next = (i + 1) % edges;
            self.mesh
                .indices
                .extend([middle, first_edge + i, first_edge + next]);
        }
    }

    /// A person on a chair, as the classic client draws him: his standing
    /// picture folded at the waist and the knees, the upper body leaning
    /// forward, and the feet left out.
    pub fn sitting(&mut self, sprite: Sprite, area: Area, color: Rgba, mirrored: bool, zoom: f32) {
        let lean = if mirrored { -SIT_LEAN } else { SIT_LEAN } * zoom;
        let leans = [lean, lean, 0.0, 0.0];
        let uv = sprite.uv;
        let at = |fold: usize| {
            let share = SIT_FOLDS[fold];
            let y = area.min.y + area.height() * share;
            let v = uv.min.y + uv.height() * share;
            (y, v, leans[fold])
        };
        for fold in 0..SIT_FOLDS.len() - 1 {
            let (top, top_v, top_lean) = at(fold);
            let (bottom, bottom_v, bottom_lean) = at(fold + 1);
            self.quad(
                [
                    Point::new(area.min.x + top_lean, top),
                    Point::new(area.max.x + top_lean, top),
                    Point::new(area.max.x + bottom_lean, bottom),
                    Point::new(area.min.x + bottom_lean, bottom),
                ],
                [
                    Point::new(uv.min.x, top_v),
                    Point::new(uv.max.x, top_v),
                    Point::new(uv.max.x, bottom_v),
                    Point::new(uv.min.x, bottom_v),
                ],
                color,
            );
        }
    }

    /// A filled shape with no picture. The points must go round a convex
    /// shape.
    pub fn fill(&mut self, points: &[Point], color: Rgba) {
        let white = self.white;
        let first = self.vertex(points[0], white, color);
        for point in &points[1..] {
            self.vertex(*point, white, color);
        }
        for i in 1..points.len() as u32 - 1 {
            self.mesh.indices.extend([first, first + i, first + i + 1]);
        }
    }

    pub fn ring(&mut self, center: Point, radius: Vector, width: f32, color: Rgba) {
        let outer = ellipse(center, radius);
        let inner = ellipse(center, radius - Vector::splat(width));
        for i in 0..ELLIPSE_POINTS {
            let next = (i + 1) % ELLIPSE_POINTS;
            self.fill(&[outer[i], outer[next], inner[next], inner[i]], color);
        }
    }

    pub fn sprite(&mut self, sprite: Sprite, area: Area, color: Rgba) {
        self.quad(corners(area), corners(sprite.uv), color);
    }
}

/// The points round an ellipse, clockwise on the screen from the right.
pub(super) fn ellipse(center: Point, radius: Vector) -> [Point; ELLIPSE_POINTS] {
    std::array::from_fn(|i| {
        let angle = i as f32 / ELLIPSE_POINTS as f32 * std::f32::consts::TAU;
        center + Vector::new(angle.cos() * radius.x, angle.sin() * radius.y)
    })
}

/// The four corners of an area, clockwise from the top left.
pub(super) fn corners(area: Area) -> [Point; 4] {
    [
        area.min,
        Point::new(area.max.x, area.min.y),
        area.max,
        Point::new(area.min.x, area.max.y),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Point = Point::new(0.5, 0.5);

    #[test]
    fn a_quad_is_two_triangles_from_its_first_corner() {
        let mut canvas = Canvas::new(WHITE);
        let area = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(2.0, 2.0));
        canvas.fill(&corners(area), Rgba::WHITE);
        canvas.fill(&corners(area), Rgba::WHITE);
        assert_eq!(canvas.mesh.indices, [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
        assert!(canvas
            .mesh
            .vertices
            .iter()
            .all(|v| v.uv == [WHITE.x, WHITE.y]));
    }

    #[test]
    fn a_glow_fades_from_its_middle_to_a_clear_edge() {
        let mut canvas = Canvas::new(WHITE);
        canvas.glow(Point::new(10.0, 10.0), 5.0, Rgba::WHITE);
        let vertices = &canvas.mesh.vertices;
        assert_eq!(vertices.len(), ELLIPSE_POINTS + 1);
        assert_eq!(vertices[0].rgba, Rgba::WHITE.to_array());
        assert!(vertices[1..].iter().all(|v| v.rgba == [0; 4]));
        assert_eq!(canvas.mesh.indices.len(), ELLIPSE_POINTS * 3);
    }

    #[test]
    fn a_grid_lays_a_cell_for_each_step_of_the_picture() {
        let mut canvas = Canvas::new(WHITE);
        let sprite = Sprite::whole(16.0, 8.0);
        let area = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(16.0, 8.0));
        canvas.sprite_grid(sprite, area, Rgba::WHITE, 8.0, |_| 1.0);
        assert_eq!(canvas.mesh.vertices.len(), 3 * 2);
        assert_eq!(canvas.mesh.indices.len(), 2 * 6);
        assert_eq!(canvas.mesh.vertices.last().unwrap().uv, [1.0, 1.0]);
    }
}
