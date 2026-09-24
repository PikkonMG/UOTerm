//! The map of the world round the character, seen from far above and
//! turned as the play field is turned. A click on it walks the character
//! to that place, while the human has control. The human moves it by its
//! title and sizes it by its corner, and it opens again where he left it.

use super::control::{Act, Hand};
use super::kept;
use super::scene::Scene;
use super::theme::{self, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{
    self, epaint::Vertex, Align2, Color32, ColorImage, Id, Mesh, Pos2, Rect, Sense, Shape,
    TextureHandle, TextureOptions, Vec2,
};
use serde::{Deserialize, Serialize};

/// How many tiles one side of the picture covers.
const SPAN: usize = 256;
/// The picture is made again when the character is this far from its middle.
const REDRAW_TILES: u16 = 48;
/// The smallest the map field is drawn, whatever the size of the window.
const PANEL_SIDE: f32 = 470.0;
/// On a large screen the map grows to this share of the shorter side of the
/// window, so a person who plays at a high resolution can still read it.
const PANEL_SHARE: f32 = 0.7;
const TITLE_ROW: f32 = 30.0;
/// At this zoom the whole picture fits the field. Above it the map comes
/// closer and shows fewer tiles.
const ZOOM_MIN: f32 = 1.0;
const ZOOM_MAX: f32 = 8.0;
/// How much one notch of the wheel changes the zoom.
const ZOOM_PER_NOTCH: f32 = 1.15;
/// The wheel gives its step in points. This many points are one notch.
const WHEEL_NOTCH: f32 = 50.0;
const DOT_RADIUS: f32 = 2.5;
const SELF_RADIUS: f32 = 4.0;
const UNKNOWN: Color32 = Color32::from_rgb(10, 12, 18);

/// The smallest the human can make the map field.
const FIELD_SIDE_SMALLEST: f32 = 200.0;
/// The corner that sizes the map, and the marks drawn on it.
const GRIP_SIDE: f32 = 18.0;
const GRIP_MARKS: [f32; 3] = [4.0, 8.0, 12.0];
const GRIP_MARK_WIDTH: f32 = 1.5;
const MAP_FILE: &str = "watch-map.toml";

const WORDS_TITLE: &str = "Map";
const HINT_MOVE: &str = "Drag: move the map. Double-click: put it back.";
const HINT_SIZE: &str = "Drag: make the map larger or smaller.";
const WORDS_NO_FILES: &str = "The map needs the client files.";
const HINT_WALK: &str = "Click: walk there. Wheel: zoom.";
const HINT_ZOOM: &str = "Wheel: zoom.";

struct Picture {
    map: u8,
    /// The tile in the middle of the picture.
    middle: (u16, u16),
    texture: TextureHandle,
}

/// Where the human put the map, and how wide he made its field.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Placed {
    left: f32,
    top: f32,
    side: f32,
}

/// What the window keeps of the map between runs. With nothing kept, the
/// map opens in the middle of the window.
#[derive(Debug, Default, Serialize, Deserialize)]
struct KeptMap {
    placed: Option<Placed>,
}

pub struct MapUi {
    open: bool,
    picture: Option<Picture>,
    zoom: f32,
    placed: Option<Placed>,
}

/// The zoom after `notches` of the wheel, held inside the two bounds.
fn zoomed(zoom: f32, notches: f32) -> f32 {
    (zoom * ZOOM_PER_NOTCH.powf(notches)).clamp(ZOOM_MIN, ZOOM_MAX)
}

/// How wide one side of the map field is in a window of this size.
fn field_side(rect: Rect) -> f32 {
    let room = rect.height().min(rect.width()) * PANEL_SHARE - TITLE_ROW - theme::PANEL_PAD * 2.0;
    room.max(PANEL_SIDE)
}

/// The size of the panel round a field of this side.
fn panel_size(side: f32) -> Vec2 {
    Vec2::new(side, side + TITLE_ROW) + Vec2::splat(theme::PANEL_PAD * 2.0)
}

/// The widest field whose panel fits in the window.
fn side_room(rect: Rect) -> f32 {
    let pads = theme::PANEL_PAD * 2.0;
    (rect.width() - pads)
        .min(rect.height() - pads - TITLE_ROW)
        .max(FIELD_SIDE_SMALLEST)
}

/// The place of the panel: where the human put it, kept inside the window
/// however the window changed since, or else the middle of the window.
fn panel_rect(rect: Rect, placed: Option<Placed>) -> Rect {
    let Some(placed) = placed else {
        return Rect::from_center_size(rect.center(), panel_size(field_side(rect)));
    };
    let size = panel_size(placed.side.clamp(FIELD_SIDE_SMALLEST, side_room(rect)));
    let left = placed
        .left
        .clamp(rect.left(), (rect.right() - size.x).max(rect.left()));
    let top = placed
        .top
        .clamp(rect.top(), (rect.bottom() - size.y).max(rect.top()));
    Rect::from_min_size(Pos2::new(left, top), size)
}

/// The side of the field inside a panel.
fn side_of(panel: Rect) -> f32 {
    panel.width() - theme::PANEL_PAD * 2.0
}

/// Where a tile is on the turned map, as a step from the character. One
/// tile east goes right and down, one tile south goes left and down.
fn turned(tiles: Vec2, unit: f32) -> Vec2 {
    Vec2::new(tiles.x - tiles.y, tiles.x + tiles.y) * unit
}

/// The tiles from the character for a step on the turned map.
fn unturned(on_screen: Vec2, unit: f32) -> Vec2 {
    let (a, b) = (on_screen.x / unit, on_screen.y / unit);
    Vec2::new((a + b) / 2.0, (b - a) / 2.0)
}

impl MapUi {
    pub fn starting(open: bool) -> Self {
        let kept: KeptMap = kept::load(MAP_FILE);
        Self {
            open,
            picture: None,
            zoom: ZOOM_MIN,
            placed: kept.placed,
        }
    }

    fn keep_place(&self) {
        kept::save(
            MAP_FILE,
            &KeptMap {
                placed: self.placed,
            },
        );
    }

    /// The title moves the panel and the corner sizes it. A double-click on
    /// the title puts it back in the middle. The place is kept when a drag
    /// ends.
    fn move_and_size(&mut self, ui: &egui::Ui, panel: Rect) {
        let title = Rect::from_min_size(
            panel.min,
            Vec2::new(panel.width(), TITLE_ROW + theme::PANEL_PAD),
        );
        let grip = Rect::from_min_max(panel.max - Vec2::splat(GRIP_SIDE), panel.max);
        let painter = ui.painter();
        for mark in GRIP_MARKS {
            painter.line_segment(
                [
                    Pos2::new(panel.right() - mark, panel.bottom()),
                    Pos2::new(panel.right(), panel.bottom() - mark),
                ],
                egui::Stroke::new(GRIP_MARK_WIDTH, theme::TEXT_FAINT),
            );
        }
        let mover = ui.interact(title, Id::new("world-map-move"), Sense::click_and_drag());
        let sizer = ui.interact(grip, Id::new("world-map-size"), Sense::drag());
        let placed = Placed {
            left: panel.left(),
            top: panel.top(),
            side: side_of(panel),
        };
        if mover.double_clicked() {
            self.placed = None;
            self.keep_place();
            return;
        }
        if mover.dragged() {
            let moved = mover.drag_delta();
            self.placed = Some(Placed {
                left: placed.left + moved.x,
                top: placed.top + moved.y,
                ..placed
            });
        }
        if sizer.dragged() {
            let pulled = sizer.drag_delta();
            self.placed = Some(Placed {
                side: (placed.side + (pulled.x + pulled.y) / 2.0).max(FIELD_SIDE_SMALLEST),
                ..placed
            });
        }
        if mover.drag_stopped() || sizer.drag_stopped() {
            self.keep_place();
        }
        if sizer.hovered() || sizer.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            super::tips::label(ui, HINT_SIZE, "");
        } else if mover.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if mover.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            super::tips::label(ui, HINT_MOVE, "");
        }
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    fn picture_of(&mut self, ui: &egui::Ui, frame: &WatchFrame, scene: &mut Scene) -> bool {
        let fresh = self.picture.as_ref().is_some_and(|picture| {
            picture.map == frame.map
                && picture.middle.0.abs_diff(frame.x) < REDRAW_TILES
                && picture.middle.1.abs_diff(frame.y) < REDRAW_TILES
        });
        if fresh {
            return true;
        }
        let half = (SPAN / 2) as i32;
        let mut image = ColorImage::new([SPAN, SPAN], UNKNOWN);
        let mut any = false;
        for row in 0..SPAN {
            for column in 0..SPAN {
                let x = i32::from(frame.x) + column as i32 - half;
                let y = i32::from(frame.y) + row as i32 - half;
                let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else {
                    continue;
                };
                if let Some([r, g, b]) = scene.radar_rgb(frame.map, x, y) {
                    image.pixels[row * SPAN + column] = Color32::from_rgb(r, g, b);
                    any = true;
                }
            }
        }
        if !any {
            self.picture = None;
            return false;
        }
        self.picture = Some(Picture {
            map: frame.map,
            middle: (frame.x, frame.y),
            texture: ui
                .ctx()
                .load_texture("world-map", image, TextureOptions::NEAREST),
        });
        true
    }

    /// Draws the map when it is open. Gives the place it covers.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
        hand: &Hand,
    ) -> Option<Rect> {
        if !self.open {
            return None;
        }
        let panel = panel_rect(rect, self.placed);
        self.body(ui, panel, frame, scene, hand);
        // After the field, so the corner and the title take their own clicks.
        self.move_and_size(ui, panel);
        Some(panel)
    }

    fn body(
        &mut self,
        ui: &egui::Ui,
        panel: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
        hand: &Hand,
    ) {
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            WORDS_TITLE,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let field = Rect::from_min_max(inner.left_top() + Vec2::new(0.0, TITLE_ROW), inner.max);
        if !self.picture_of(ui, frame, scene) {
            ui.painter().text(
                field.center(),
                Align2::CENTER_CENTER,
                WORDS_NO_FILES,
                text_font(theme::SIZE_BODY),
                theme::TEXT_FAINT,
            );
            return;
        }
        let Some(picture) = &self.picture else {
            return;
        };
        let response = ui.interact(field, Id::new("world-map"), Sense::click());
        if response.hovered() {
            let notches = ui.input(|input| input.raw_scroll_delta.y) / WHEEL_NOTCH;
            if notches != 0.0 {
                self.zoom = zoomed(self.zoom, notches);
            }
        }
        // The turned picture is a diamond as wide as the field, and the zoom
        // spreads it wider than that.
        let unit = field.width() / (SPAN as f32 * 2.0) * self.zoom;
        let center = field.center();
        let from_character = Vec2::new(
            f32::from(picture.middle.0) - f32::from(frame.x),
            f32::from(picture.middle.1) - f32::from(frame.y),
        );
        let half = SPAN as f32 / 2.0;
        let corners = [(-half, -half), (half, -half), (half, half), (-half, half)];
        let uvs = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let mut mesh = Mesh::with_texture(picture.texture.id());
        for ((dx, dy), (u, v)) in corners.into_iter().zip(uvs) {
            mesh.vertices.push(Vertex {
                pos: center + turned(from_character + Vec2::new(dx, dy), unit),
                uv: Pos2::new(u, v),
                color: Color32::WHITE,
            });
        }
        mesh.indices.extend([0, 1, 2, 0, 2, 3]);
        let painter = ui.painter().with_clip_rect(field);
        painter.add(Shape::mesh(mesh));
        for mobile in &frame.mobiles {
            let step = Vec2::new(
                f32::from(mobile.x) - f32::from(frame.x),
                f32::from(mobile.y) - f32::from(frame.y),
            );
            painter.circle_filled(
                center + turned(step, unit),
                DOT_RADIUS,
                theme::notoriety_color(mobile.notoriety),
            );
        }
        if let (Some(x), Some(y)) = (frame.dest_x, frame.dest_y) {
            let step = Vec2::new(
                f32::from(x) - f32::from(frame.x),
                f32::from(y) - f32::from(frame.y),
            );
            painter.circle_stroke(
                center + turned(step, unit),
                SELF_RADIUS,
                egui::Stroke::new(1.5, theme::GOAL),
            );
        }
        // The marks the shard put on the map, each with its name.
        for mark in frame.waypoints.iter().filter(|mark| mark.map == frame.map) {
            let step = Vec2::new(
                f32::from(mark.x) - f32::from(frame.x),
                f32::from(mark.y) - f32::from(frame.y),
            );
            let at = center + turned(step, unit);
            painter.circle_filled(at, DOT_RADIUS, theme::WAITING);
            painter.text(
                at + Vec2::new(DOT_RADIUS * 2.0, 0.0),
                Align2::LEFT_CENTER,
                &mark.name,
                text_font(theme::SIZE_SMALL),
                theme::WAITING,
            );
        }
        painter.circle_filled(center, SELF_RADIUS, theme::SELF_FIGURE);
        if !frame.human_control {
            if response.hovered() {
                super::tips::label(ui, HINT_ZOOM, "");
            }
            return;
        }
        if let Some(mouse) = response.hover_pos() {
            super::tips::label(ui, HINT_WALK, "");
            if response.clicked() {
                let tiles = unturned(mouse - center, unit);
                let x = (f32::from(frame.x) + tiles.x).round().max(0.0) as u16;
                let y = (f32::from(frame.y) + tiles.y).round().max(0.0) as u16;
                hand.act(Act::WalkTo { x, y });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wheel_zooms_between_the_two_bounds() {
        assert!(zoomed(ZOOM_MIN, 1.0) > ZOOM_MIN, "a notch up comes closer");
        assert_eq!(zoomed(ZOOM_MIN, -5.0), ZOOM_MIN, "never below the fit");
        assert_eq!(zoomed(ZOOM_MAX, 20.0), ZOOM_MAX, "never above the bound");
        let twice = zoomed(zoomed(ZOOM_MIN, 1.0), -1.0);
        assert!((twice - ZOOM_MIN).abs() < 0.001, "up then down comes back");
    }

    #[test]
    fn a_large_window_gets_a_large_map() {
        let small = Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 600.0));
        assert_eq!(field_side(small), PANEL_SIDE);
        let large = Rect::from_min_size(Pos2::ZERO, Vec2::new(2560.0, 1440.0));
        assert!(field_side(large) > PANEL_SIDE, "a big screen shows more");
        assert!(field_side(large) < large.height(), "it stays in the window");
    }

    /// The map opens in the middle until the human puts it somewhere. A
    /// place or a size that no longer fits the window is held inside it.
    #[test]
    fn a_placed_map_stays_inside_the_window() {
        const SIDE: f32 = 300.0;
        let window = Rect::from_min_size(Pos2::ZERO, Vec2::new(1600.0, 900.0));
        let middle = panel_rect(window, None);
        assert!((middle.center() - window.center()).length() < 0.01);

        let placed = Placed {
            left: 40.0,
            top: 60.0,
            side: SIDE,
        };
        let panel = panel_rect(window, Some(placed));
        assert_eq!(panel.min, Pos2::new(placed.left, placed.top));
        assert!((side_of(panel) - SIDE).abs() < 0.01, "the size is kept");

        let far = panel_rect(
            window,
            Some(Placed {
                left: 5_000.0,
                top: -300.0,
                ..placed
            }),
        );
        assert!(window.contains_rect(far), "dragged off, it stays in view");

        let huge = panel_rect(
            window,
            Some(Placed {
                side: 10_000.0,
                ..placed
            }),
        );
        assert!(window.contains_rect(huge), "pulled too large, it fits");
        let tiny = panel_rect(
            window,
            Some(Placed {
                side: 1.0,
                ..placed
            }),
        );
        assert!((side_of(tiny) - FIELD_SIDE_SMALLEST).abs() < 0.01);
    }

    #[test]
    fn a_click_on_the_turned_map_finds_its_tile_again() {
        const UNIT: f32 = 0.9;
        let east = turned(Vec2::new(1.0, 0.0), UNIT);
        assert!(east.x > 0.0 && east.y > 0.0, "east goes right and down");
        let south = turned(Vec2::new(0.0, 1.0), UNIT);
        assert!(south.x < 0.0 && south.y > 0.0, "south goes left and down");
        let tiles = Vec2::new(37.0, -12.0);
        let back = unturned(turned(tiles, UNIT), UNIT);
        assert!((back - tiles).length() < 0.001);
    }
}
