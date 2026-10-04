//! The marks over the world: the way to the walk goal, the ring of the
//! character, the range of a spell, the arrow of a quest, and a building
//! that waits for its place. Each is a list of plain shapes; the window
//! strokes the lines with its own smooth edges.

use super::canvas::{ellipse, Canvas, Mesh};
use super::{
    faded, Projection, SceneState, HALF_TILE, PAWN_RING_RX, PAWN_RING_RY, PAWN_RING_WIDTH,
};
use crate::art::{is_drawn, ItemPaint, WorldArt};
use crate::frame::WatchFrame;
use crate::geom::{Area, Point, Rgba, Vector};
use crate::ui::theme;

/// A tile's edge lies this far from its middle, in tiles.
const HALF_TILE_STEP: f32 = 0.5;
/// The width of the range circle's line.
const RANGE_LINE_WIDTH: f32 = 2.0;
/// The second ring of the character, drawn over everything so that a wall or
/// a neighbor never hides where he is.
const FOCUS_RING_GROW: f32 = 1.5;
const PATH_DASH: f32 = 9.0;
const PATH_GAP: f32 = 7.0;
const PATH_WIDTH: f32 = 2.0;
const BEACON_HEIGHT: f32 = 46.0;
const BEACON_RING: f32 = 0.5;
const EDGE_INSET: f32 = 28.0;
const EDGE_MARK_RADIUS: f32 = 6.0;
/// The room the quest arrow keeps from the edge of the window.
const ARROW_EDGE: f32 = 28.0;
const ARROW_LENGTH: f32 = 22.0;
const ARROW_WIDTH: f32 = 7.0;
const ARROW_EDGE_WIDTH: f32 = 1.5;
/// How much of a building that waits for its place shows.
const PLACING_ALPHA: f32 = 0.55;

/// One shape over the world, in screen points.
#[derive(Clone, Debug, PartialEq)]
pub enum Overlay {
    /// A line round the points and back to the first.
    ClosedLine {
        points: Vec<Point>,
        width: f32,
        color: Rgba,
    },
    Segment {
        from: Point,
        to: Point,
        width: f32,
        color: Rgba,
    },
    /// A line of dashes `dash` long with `gap` between them.
    Dashed {
        from: Point,
        to: Point,
        width: f32,
        color: Rgba,
        dash: f32,
        gap: f32,
    },
    /// A filled circle.
    Disc {
        center: Point,
        radius: f32,
        color: Rgba,
    },
    /// The line of a circle.
    Circle {
        center: Point,
        radius: f32,
        width: f32,
        color: Rgba,
    },
    /// A filled convex shape with an edge.
    Polygon {
        points: Vec<Point>,
        fill: Rgba,
        width: f32,
        edge: Rgba,
    },
    /// Pictures from the texture of the map.
    Pictures(Mesh),
}

/// A dashed line on the ground to the walk goal, and a beacon on it. When
/// the goal is outside the window, a mark on the edge points to it.
pub fn walk_goal(projection: &Projection, frame: &WatchFrame) -> Vec<Overlay> {
    let (Some(x), Some(y)) = (frame.dest_x, frame.dest_y) else {
        return Vec::new();
    };
    let camera = projection.camera;
    let from = projection.project(camera);
    let goal = projection.project([f32::from(x), f32::from(y), camera[2]]);
    let inside = projection.view.expand(-EDGE_INSET);
    let end = if inside.contains(goal) {
        goal
    } else {
        clamp_to(inside, from, goal)
    };
    let (width, color) = (PATH_WIDTH, theme::GOAL);
    let path = Overlay::Dashed {
        from,
        to: end,
        width,
        color,
        dash: PATH_DASH,
        gap: PATH_GAP,
    };
    if !inside.contains(goal) {
        let mark = Overlay::Disc {
            center: end,
            radius: EDGE_MARK_RADIUS,
            color,
        };
        return vec![path, mark];
    }
    let half = HALF_TILE * projection.zoom * BEACON_RING;
    let diamond = vec![
        goal + Vector::new(0.0, -half),
        goal + Vector::new(half, 0.0),
        goal + Vector::new(0.0, half),
        goal + Vector::new(-half, 0.0),
    ];
    vec![
        path,
        Overlay::ClosedLine {
            points: diamond,
            width,
            color,
        },
        Overlay::Segment {
            from: goal,
            to: goal - Vector::new(0.0, BEACON_HEIGHT * projection.zoom),
            width,
            color,
        },
    ]
}

/// The ring round the feet of the character, over everything.
pub fn focus_ring(foot: Point, zoom: f32) -> Overlay {
    let radius = Vector::new(PAWN_RING_RX, PAWN_RING_RY) * FOCUS_RING_GROW * zoom;
    Overlay::ClosedLine {
        points: ellipse(foot, radius).to_vec(),
        width: PAWN_RING_WIDTH * zoom,
        color: theme::SELF_FIGURE,
    }
}

/// The corners of the ground within `tiles` of the character, the top one
/// first. UO counts range as the larger of the two distances, so the tiles
/// in range make a square, which the map shows as a diamond.
pub fn range_corners(projection: &Projection, tiles: u8) -> [Point; 4] {
    let reach = f32::from(tiles) + HALF_TILE_STEP;
    let [x, y, z] = projection.camera;
    [
        (-reach, -reach),
        (reach, -reach),
        (reach, reach),
        (-reach, reach),
    ]
    .map(|(dx, dy)| projection.project([x + dx, y + dy, z]))
}

/// The outline of the ground within `tiles` of the character, as the
/// Combat page's range circle shows how far a spell reaches.
pub fn range_diamond(projection: &Projection, tiles: u8, color: Rgba) -> Overlay {
    Overlay::ClosedLine {
        points: range_corners(projection, tiles).to_vec(),
        width: RANGE_LINE_WIDTH * projection.zoom,
        color,
    }
}

/// Where the quest arrow stands, and the way it points. A place in view
/// keeps its own spot; one out of view is held at the edge of the window,
/// as a compass needle is.
pub fn arrow_at(view: Area, at: Point) -> (Point, Vector) {
    let room = view.expand(-ARROW_EDGE);
    let point = Point::new(
        at.x.clamp(room.min.x, room.max.x),
        at.y.clamp(room.min.y, room.max.y),
    );
    let away = at - view.center();
    let away = if away.length() > f32::EPSILON {
        away / away.length()
    } else {
        Vector::new(0.0, -1.0)
    };
    (point, away)
}

/// The arrow the shard points at a place, and the box the player clicks.
pub fn quest_arrow(projection: &Projection, frame: &WatchFrame) -> Option<(Overlay, Area)> {
    let (x, y) = frame.quest_arrow?;
    let at = projection.project([f32::from(x), f32::from(y), projection.camera[2]]);
    let (point, away) = arrow_at(projection.view, at);
    let tip = point + away * ARROW_LENGTH / 2.0;
    let back = point - away * ARROW_LENGTH / 2.0;
    let side = Vector::new(-away.y, away.x) * ARROW_WIDTH;
    let corners = vec![tip, back + side, back - side];
    let area = Area::from_points(&corners);
    let arrow = Overlay::Polygon {
        points: corners,
        fill: theme::GOAL,
        width: ARROW_EDGE_WIDTH,
        edge: theme::TEXT,
    };
    Some((arrow, area))
}

/// The point where the line from `from` to `to` leaves `area`.
pub fn clamp_to(area: Area, from: Point, to: Point) -> Point {
    let delta = to - from;
    let reach = |delta: f32, low: f32, high: f32| {
        if delta > 0.0 {
            high / delta
        } else if delta < 0.0 {
            low / delta
        } else {
            f32::INFINITY
        }
    };
    let t = reach(delta.x, area.min.x - from.x, area.max.x - from.x)
        .min(reach(delta.y, area.min.y - from.y, area.max.y - from.y))
        .clamp(0.0, 1.0);
    from + delta * t
}

impl SceneState {
    /// The outline of a building where the mouse points, while the shard
    /// waits for its place, and a ring on the tile it goes on. Its pieces
    /// show as a pale shape, so the human sees what the building covers
    /// before he puts it there. None while nothing waits for a place.
    pub fn placing_preview(
        &self,
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        mouse: Point,
    ) -> Option<Vec<Overlay>> {
        let placing = frame.placing?;
        let (x, y, z) = self.tile_at(art, view, frame, mouse);
        let pieces: Vec<(i16, i16, i16, u16)> = art
            .multi_pieces(placing.multi_id)
            .ready()
            .unwrap_or_default()
            .iter()
            .map(|piece| (piece.dx, piece.dy, piece.dz, piece.graphic))
            .collect();
        let projection = self.projection(view);
        let mut canvas = Canvas::new(art.white_uv());
        for (dx, dy, dz, graphic) in pieces {
            if !is_drawn(graphic) {
                continue;
            }
            let at = [
                f32::from(x) + f32::from(dx),
                f32::from(y) + f32::from(dy),
                f32::from(z) + f32::from(dz),
            ];
            let paint = ItemPaint {
                hue: placing.hue,
                ..ItemPaint::default()
            };
            let Some(sprite) = self.item_sprite(art, graphic, paint, true).ready() else {
                continue;
            };
            let foot = projection.project(at);
            let area = Area::from_min_size(
                foot - sprite.anchor * self.zoom,
                Vector::new(sprite.width, sprite.height) * self.zoom,
            );
            canvas.sprite(sprite, area, faded(Rgba::WHITE, PLACING_ALPHA));
        }
        let mut overlays = Vec::with_capacity(2);
        if !canvas.mesh.is_empty() {
            overlays.push(Overlay::Pictures(canvas.mesh));
        }
        // The tile the building goes on, so the human sees the exact spot.
        overlays.push(Overlay::Circle {
            center: projection.project([f32::from(x), f32::from(y), f32::from(z)]),
            radius: PAWN_RING_RX * self.zoom,
            width: PAWN_RING_WIDTH * self.zoom,
            color: theme::GOAL,
        });
        Some(overlays)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Area = Area {
        min: Point { x: 0.0, y: 0.0 },
        max: Point { x: 100.0, y: 100.0 },
    };
    const MIDDLE: Point = Point::new(50.0, 50.0);

    fn projection_at(camera: [f32; 3]) -> Projection {
        Projection {
            view: WINDOW,
            camera,
            zoom: 1.0,
            pixels_per_point: 1.0,
            peek: Vector::ZERO,
        }
    }

    #[test]
    fn the_quest_arrow_is_held_at_the_edge_when_its_place_is_out_of_view() {
        let view = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(1280.0, 800.0));
        // A place far to the right is held at the right edge.
        let (point, away) = arrow_at(view, Point::new(5000.0, 400.0));
        assert_eq!(point.x, view.max.x - ARROW_EDGE);
        assert!(away.x > 0.9, "it points to the right");
        // A place in view keeps its own spot.
        let inside = Point::new(700.0, 500.0);
        assert_eq!(arrow_at(view, inside).0, inside);
        // A place under the character points up, not nowhere.
        let (_, away) = arrow_at(view, view.center());
        assert_eq!(away, Vector::new(0.0, -1.0));
    }

    #[test]
    fn a_goal_outside_the_window_is_marked_on_the_edge() {
        let far_right = Point::new(250.0, 50.0);
        assert_eq!(clamp_to(WINDOW, MIDDLE, far_right), Point::new(100.0, 50.0));
        let far_up_left = Point::new(-50.0, -150.0);
        assert_eq!(clamp_to(WINDOW, MIDDLE, far_up_left), Point::new(25.0, 0.0));
    }

    #[test]
    fn the_range_circle_bounds_the_tiles_in_range() {
        const TILES: u8 = 3;
        let projection = projection_at([100.0, 100.0, 5.0]);
        let [top, right, bottom, left] = range_corners(&projection, TILES);
        let reach = (f32::from(TILES) + HALF_TILE_STEP) * HALF_TILE * 2.0;
        assert_eq!(top, MIDDLE - Vector::new(0.0, reach));
        assert_eq!(bottom, MIDDLE + Vector::new(0.0, reach));
        assert_eq!(right, MIDDLE + Vector::new(reach, 0.0));
        assert_eq!(left, MIDDLE - Vector::new(reach, 0.0));
    }

    #[test]
    fn a_walk_goal_in_view_has_a_path_a_diamond_and_a_beacon() {
        let projection = projection_at([100.0, 100.0, 0.0]);
        let near = WatchFrame {
            dest_x: Some(101),
            dest_y: Some(100),
            ..WatchFrame::default()
        };
        let shapes = walk_goal(&projection, &near);
        assert!(matches!(shapes[0], Overlay::Dashed { .. }));
        assert!(matches!(shapes[1], Overlay::ClosedLine { .. }));
        assert!(matches!(shapes[2], Overlay::Segment { .. }));
        let far = WatchFrame {
            dest_x: Some(300),
            ..near
        };
        let shapes = walk_goal(&projection, &far);
        assert!(matches!(shapes[1], Overlay::Disc { .. }));
        assert!(walk_goal(&projection, &WatchFrame::default()).is_empty());
    }
}
