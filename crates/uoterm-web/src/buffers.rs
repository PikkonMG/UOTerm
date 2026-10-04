//! What one frame draws, as flat arrays a WebGL renderer takes as they
//! are: the world under the light, the light map, and what lies over the
//! light. The names, the hit points and the words over heads are laid out
//! here and drawn by the page as text. The shapes of the overlays turn into
//! triangles here, as egui turns them in the Rust window.

use crate::web_art::Upload;
use serde::Serialize;
use uoterm_view::audio::Step;
use uoterm_view::geom::{Area, Point, Rgba, Vector};
use uoterm_view::lights::{LightCells, LIGHT_CELL};
use uoterm_view::scene::plates::PlacedPlate;
use uoterm_view::scene::{Mesh, Overlay, Vertex};
use wasm_bindgen::prelude::*;

/// The points round a whole circle.
const CIRCLE_SEGMENTS: usize = 32;
const FULL_TURN: f32 = std::f32::consts::TAU;
/// The corners of a quad, as two triangles.
const QUAD_TRIANGLES: [u32; 6] = [0, 1, 2, 0, 2, 3];
const XY: usize = 2;
const RGBA: usize = 4;

/// Words over a head, laid out: the page draws them as text with their
/// bottom middle at `x`, `y`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlacedWords {
    pub words: String,
    pub x: f32,
    pub y: f32,
    pub color: Rgba,
    pub alpha: f32,
    /// A number of hits lost, in the font of numbers.
    pub number: bool,
}

/// Triangles in paint order, built from shapes. Shapes with no picture read
/// the white point of the texture.
pub struct Shapes {
    mesh: Mesh,
    white: [f32; 2],
}

impl Shapes {
    pub fn new(white: Point) -> Self {
        Self {
            mesh: Mesh::default(),
            white: [white.x, white.y],
        }
    }

    pub fn into_mesh(self) -> Mesh {
        self.mesh
    }

    /// Adds triangles that are made already.
    pub fn append(&mut self, mesh: Mesh) {
        let start = self.mesh.vertices.len() as u32;
        self.mesh.vertices.extend(mesh.vertices);
        self.mesh
            .indices
            .extend(mesh.indices.into_iter().map(|index| start + index));
    }

    /// One quad with its corners in order round it.
    fn quad(&mut self, corners: [Point; 4], uvs: [[f32; 2]; 4], color: Rgba) {
        let start = self.mesh.vertices.len() as u32;
        for (corner, uv) in corners.into_iter().zip(uvs) {
            self.mesh.vertices.push(Vertex {
                pos: [corner.x, corner.y],
                uv,
                rgba: color.to_array(),
            });
        }
        self.mesh
            .indices
            .extend(QUAD_TRIANGLES.map(|index| start + index));
    }

    /// A picture of the texture over `area`.
    pub fn picture(&mut self, area: Area, uv: Area, color: Rgba) {
        let corners = [
            area.min,
            Point::new(area.max.x, area.min.y),
            area.max,
            Point::new(area.min.x, area.max.y),
        ];
        let uvs = [
            [uv.min.x, uv.min.y],
            [uv.max.x, uv.min.y],
            [uv.max.x, uv.max.y],
            [uv.min.x, uv.max.y],
        ];
        self.quad(corners, uvs, color);
    }

    /// A box of one color.
    pub fn fill(&mut self, area: Area, color: Rgba) {
        let white = self.white;
        let corners = [
            area.min,
            Point::new(area.max.x, area.min.y),
            area.max,
            Point::new(area.min.x, area.max.y),
        ];
        self.quad(corners, [white; 4], color);
    }

    /// A straight line `width` wide.
    pub fn segment(&mut self, from: Point, to: Point, width: f32, color: Rgba) {
        let along = to - from;
        let length = along.length();
        if length <= 0.0 {
            return;
        }
        let half = Vector::new(-along.y, along.x) / length * (width / 2.0);
        let white = self.white;
        self.quad(
            [from + half, to + half, to - half, from - half],
            [white; 4],
            color,
        );
    }

    /// A line through the points, back to the first when `closed`.
    pub fn line(&mut self, points: &[Point], closed: bool, width: f32, color: Rgba) {
        for pair in points.windows(2) {
            self.segment(pair[0], pair[1], width, color);
        }
        if let (true, Some(first), Some(last)) = (closed, points.first(), points.last()) {
            self.segment(*last, *first, width, color);
        }
    }

    /// Dashes `dash` long with `gap` between them, from `from` to `to`.
    pub fn dashed(&mut self, from: Point, to: Point, width: f32, color: Rgba, dash: f32, gap: f32) {
        let along = to - from;
        let length = along.length();
        if length <= 0.0 || dash <= 0.0 {
            return;
        }
        let step = along / length;
        let mut start = 0.0;
        while start < length {
            let end = (start + dash).min(length);
            self.segment(from + step * start, from + step * end, width, color);
            start = end + gap;
        }
    }

    /// A filled shape with no dent, from its points in order.
    pub fn convex(&mut self, points: &[Point], color: Rgba) {
        let start = self.mesh.vertices.len() as u32;
        for point in points {
            self.mesh.vertices.push(Vertex {
                pos: [point.x, point.y],
                uv: self.white,
                rgba: color.to_array(),
            });
        }
        for at in 1..points.len().saturating_sub(1) as u32 {
            self.mesh
                .indices
                .extend([start, start + at, start + at + 1]);
        }
    }

    /// The points round a circle.
    fn round(center: Point, radius: f32) -> Vec<Point> {
        (0..CIRCLE_SEGMENTS)
            .map(|at| {
                let turn = FULL_TURN * at as f32 / CIRCLE_SEGMENTS as f32;
                center + Vector::new(turn.cos(), turn.sin()) * radius
            })
            .collect()
    }

    pub fn disc(&mut self, center: Point, radius: f32, color: Rgba) {
        self.convex(&Self::round(center, radius), color);
    }

    pub fn circle(&mut self, center: Point, radius: f32, width: f32, color: Rgba) {
        self.line(&Self::round(center, radius), true, width, color);
    }

    /// One shape over the world.
    pub fn overlay(&mut self, overlay: Overlay) {
        match overlay {
            Overlay::ClosedLine {
                points,
                width,
                color,
            } => self.line(&points, true, width, color),
            Overlay::Segment {
                from,
                to,
                width,
                color,
            } => self.segment(from, to, width, color),
            Overlay::Dashed {
                from,
                to,
                width,
                color,
                dash,
                gap,
            } => self.dashed(from, to, width, color, dash, gap),
            Overlay::Disc {
                center,
                radius,
                color,
            } => self.disc(center, radius, color),
            Overlay::Circle {
                center,
                radius,
                width,
                color,
            } => self.circle(center, radius, width, color),
            Overlay::Polygon {
                points,
                fill,
                width,
                edge,
            } => {
                self.convex(&points, fill);
                self.line(&points, true, width, edge);
            }
            Overlay::Pictures(mesh) => self.append(mesh),
        }
    }
}

/// The arrays of one mesh.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshArrays {
    pub positions: Vec<f32>,
    pub uvs: Vec<f32>,
    pub colors: Vec<u8>,
    pub indices: Vec<u32>,
}

impl From<Mesh> for MeshArrays {
    fn from(mesh: Mesh) -> Self {
        let count = mesh.vertices.len();
        let mut arrays = Self {
            positions: Vec::with_capacity(count * XY),
            uvs: Vec::with_capacity(count * XY),
            colors: Vec::with_capacity(count * RGBA),
            indices: mesh.indices,
        };
        for vertex in mesh.vertices {
            arrays.positions.extend(vertex.pos);
            arrays.uvs.extend(vertex.uv);
            arrays.colors.extend(vertex.rgba);
        }
        arrays
    }
}

/// What one frame draws, in this order: the world, the light map over it,
/// the overlay over the light; then the page lays the plates and the words
/// over heads as text. Positions are in points from the top left of the
/// view; colors are premultiplied RGBA bytes.
#[wasm_bindgen]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawBuffers {
    pub(crate) world: MeshArrays,
    pub(crate) overlay: MeshArrays,
    pub(crate) uploads: Vec<Upload>,
    pub(crate) atlas_reset: bool,
    pub(crate) light: Option<LightCells>,
    pub(crate) plates: Vec<PlacedPlate>,
    pub(crate) floats: Vec<PlacedWords>,
    pub(crate) steps: Vec<Step>,
    pub(crate) moving: bool,
}

impl DrawBuffers {
    /// The light map as RGBA bytes, row by row.
    fn light_bytes(&self) -> Vec<u8> {
        self.light
            .as_ref()
            .map(|light| {
                light
                    .cells
                    .iter()
                    .flat_map(|cell| cell.to_array())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// A list for the page, as plain JavaScript values.
fn to_js<T: Serialize>(value: &T) -> JsValue {
    serde_wasm_bindgen::to_value(value).unwrap_or(JsValue::NULL)
}

#[wasm_bindgen]
impl DrawBuffers {
    /// The world: x, y of each vertex.
    pub fn positions(&self) -> js_sys::Float32Array {
        js_sys::Float32Array::from(self.world.positions.as_slice())
    }

    /// The world: u, v of each vertex in the texture.
    pub fn uvs(&self) -> js_sys::Float32Array {
        js_sys::Float32Array::from(self.world.uvs.as_slice())
    }

    /// The world: premultiplied r, g, b, a of each vertex.
    pub fn colors(&self) -> js_sys::Uint8Array {
        js_sys::Uint8Array::from(self.world.colors.as_slice())
    }

    /// The world: three vertices for each triangle, in paint order.
    pub fn indices(&self) -> js_sys::Uint32Array {
        js_sys::Uint32Array::from(self.world.indices.as_slice())
    }

    #[wasm_bindgen(js_name = overlayPositions)]
    pub fn overlay_positions(&self) -> js_sys::Float32Array {
        js_sys::Float32Array::from(self.overlay.positions.as_slice())
    }

    #[wasm_bindgen(js_name = overlayUvs)]
    pub fn overlay_uvs(&self) -> js_sys::Float32Array {
        js_sys::Float32Array::from(self.overlay.uvs.as_slice())
    }

    #[wasm_bindgen(js_name = overlayColors)]
    pub fn overlay_colors(&self) -> js_sys::Uint8Array {
        js_sys::Uint8Array::from(self.overlay.colors.as_slice())
    }

    #[wasm_bindgen(js_name = overlayIndices)]
    pub fn overlay_indices(&self) -> js_sys::Uint32Array {
        js_sys::Uint32Array::from(self.overlay.indices.as_slice())
    }

    /// The pictures placed in the texture this frame:
    /// `{key, x, y, width, height}[]`.
    pub fn uploads(&self) -> JsValue {
        to_js(&self.uploads)
    }

    /// True when the texture starts again this frame: the page clears it
    /// and lays the white square in its top left corner first.
    #[wasm_bindgen(js_name = atlasReset)]
    pub fn atlas_reset(&self) -> bool {
        self.atlas_reset
    }

    /// The cells of the light map across and down. Zero when the world
    /// shows in its own light.
    #[wasm_bindgen(js_name = lightWidth)]
    pub fn light_width(&self) -> usize {
        self.light.as_ref().map_or(0, |light| light.width)
    }

    #[wasm_bindgen(js_name = lightHeight)]
    pub fn light_height(&self) -> usize {
        self.light.as_ref().map_or(0, |light| light.height)
    }

    /// The light map as premultiplied RGBA bytes, row by row from the top
    /// left of the view. The page multiplies the world by it.
    #[wasm_bindgen(js_name = lightCells)]
    pub fn light_cells(&self) -> js_sys::Uint8Array {
        js_sys::Uint8Array::from(self.light_bytes().as_slice())
    }

    /// The side of one cell of the light map, in points.
    #[wasm_bindgen(js_name = lightCell)]
    pub fn light_cell(&self) -> f32 {
        LIGHT_CELL
    }

    /// The name plates of the Modern style, laid out: `PlacedPlate[]`.
    pub fn plates(&self) -> JsValue {
        to_js(&self.plates)
    }

    /// The words and numbers over heads, laid out: `PlacedWords[]`.
    pub fn floats(&self) -> JsValue {
        to_js(&self.floats)
    }

    /// The footsteps of this frame, for the sound: `Step[]`.
    pub fn steps(&self) -> JsValue {
        to_js(&self.steps)
    }

    /// Something still moves, so the next frame must come at once.
    pub fn moving(&self) -> bool {
        self.moving
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Point = Point::new(0.5, 0.5);

    #[test]
    fn a_line_is_a_quad_as_wide_as_the_line() {
        let mut shapes = Shapes::new(WHITE);
        shapes.segment(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            2.0,
            Rgba::WHITE,
        );
        let mesh = shapes.into_mesh();
        let ys: Vec<f32> = mesh.vertices.iter().map(|vertex| vertex.pos[1]).collect();
        assert_eq!(ys, [1.0, 1.0, -1.0, -1.0]);
        assert_eq!(mesh.indices, QUAD_TRIANGLES);
        assert!(mesh.vertices.iter().all(|vertex| vertex.uv == [0.5, 0.5]));
    }

    #[test]
    fn a_polygon_fills_as_a_fan_and_gets_its_edge() {
        let mut shapes = Shapes::new(WHITE);
        let square = vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(0.0, 1.0),
        ];
        shapes.overlay(Overlay::Polygon {
            points: square,
            fill: Rgba::WHITE,
            width: 1.0,
            edge: Rgba::WHITE,
        });
        let mesh = shapes.into_mesh();
        assert_eq!(&mesh.indices[..6], [0, 1, 2, 0, 2, 3]);
        assert_eq!(mesh.indices.len(), 6 + 4 * QUAD_TRIANGLES.len());
    }

    #[test]
    fn dashes_leave_gaps_and_a_mesh_appended_keeps_its_triangles() {
        let mut shapes = Shapes::new(WHITE);
        shapes.dashed(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            1.0,
            Rgba::WHITE,
            3.0,
            2.0,
        );
        let dashes = shapes.mesh.indices.len() / QUAD_TRIANGLES.len();
        assert_eq!(dashes, 2, "0-3 and 5-8");
        let picture = Mesh {
            vertices: vec![Vertex::default(); 3],
            indices: vec![0, 1, 2],
        };
        shapes.append(picture);
        let start = (dashes * 4) as u32;
        assert_eq!(
            shapes.mesh.indices[dashes * 6..],
            [start, start + 1, start + 2]
        );
    }

    #[test]
    fn a_mesh_turns_into_flat_arrays() {
        let mesh = Mesh {
            vertices: vec![Vertex {
                pos: [1.0, 2.0],
                uv: [0.25, 0.5],
                rgba: [1, 2, 3, 4],
            }],
            indices: vec![0, 0, 0],
        };
        let arrays = MeshArrays::from(mesh);
        assert_eq!(arrays.positions, [1.0, 2.0]);
        assert_eq!(arrays.uvs, [0.25, 0.5]);
        assert_eq!(arrays.colors, [1, 2, 3, 4]);
        assert_eq!(arrays.indices, [0, 0, 0]);
    }
}
