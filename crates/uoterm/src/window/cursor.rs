//! The mouse pointers of the classic client, drawn from the art files over
//! the world in the Classic style: an arrow that points the way from the
//! character to the mouse, from the war set in war mode, and the cross
//! while the shard waits for a target. The pointer of the system hides
//! while the mouse is over the world.
//!
//! While the shard waits for a target, both styles draw what the reference client's
//! cursor adds when the options ask: an aura under the mouse in the hue of
//! what the target does, and how far the thing under the mouse is.

use super::scene::Scene;
use super::settings::Profile;
use super::steer::way_of;
use super::theme::{self, text_font};
use crate::view::WatchFrame;
use eframe::egui::{
    self, epaint::Mesh, Align2, Color32, CursorIcon, Id, LayerId, Order, Pos2, Rect, Vec2,
};
use uoterm_nav::{cursor_hue, CursorShape};
use uoterm_protocol::types::{tile_distance, Direction};

const CURSOR_LAYER: &str = "uoterm-classic-cursor";
/// The aura under the mouse, as the reference client's cursor draws it: this wide, in
/// the hue of what the target does.
const AURA_RADIUS: f32 = 30.0;
const AURA_EDGE_POINTS: usize = 32;
const HUE_NEUTRAL: u16 = 0x03B2;
const HUE_HARMFUL: u16 = 0x0023;
const HUE_BENEFICIAL: u16 = 0x005A;
/// What the target does, as the shard's target cursor says.
const TARGET_HARMFUL: u8 = 1;
const TARGET_BENEFICIAL: u8 = 2;
/// The distance stands this far up and left of the mouse.
const RANGE_OFFSET: Vec2 = Vec2::new(-26.0, -21.0);

/// The pointer for the mouse at `mouse` while the character stands at
/// `character` on the screen.
pub fn cursor_shape(target: bool, character: Pos2, mouse: Pos2) -> CursorShape {
    if target {
        return CursorShape::Target;
    }
    Direction::from_name(way_of(mouse - character)).map_or(CursorShape::Normal, CursorShape::Walk)
}

/// Draws the pointer of the classic client over the world, and hides the
/// pointer of the system there. Nothing changes over a panel, or when the
/// client files hold no pointers.
pub fn draw(ctx: &egui::Context, scene: &mut Scene, frame: &WatchFrame, rect: Rect) {
    let Some(mouse) = ctx
        .pointer_hover_pos()
        .filter(|at| scene.is_on_world(rect, *at))
    else {
        return;
    };
    let Some(place) = scene.place_of(frame, frame.serial) else {
        return;
    };
    let character = scene.screen_of(rect, place);
    let shape = cursor_shape(frame.target_cursor, character, mouse);
    let hue = cursor_hue(frame.war, frame.map);
    let Some((texture, sprite)) = scene.cursor_picture(shape, frame.war, hue) else {
        return;
    };
    ctx.set_cursor_icon(CursorIcon::None);
    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new(CURSOR_LAYER)));
    let area = Rect::from_min_size(
        mouse - sprite.anchor,
        Vec2::new(sprite.width, sprite.height),
    );
    painter.image(texture, area, sprite.uv, Color32::WHITE);
}

/// The hue of the aura under the mouse for what the target does.
pub fn aura_hue(target_flags: u8) -> u16 {
    match target_flags {
        TARGET_HARMFUL => HUE_HARMFUL,
        TARGET_BENEFICIAL => HUE_BENEFICIAL,
        _ => HUE_NEUTRAL,
    }
}

/// How far a mobile or an item in view is from the character, in tiles.
pub fn distance_to(frame: &WatchFrame, serial: u32) -> Option<u32> {
    let mobile = frame.mobiles.iter().find(|mobile| mobile.serial == serial);
    let place = mobile.map(|mobile| (mobile.x, mobile.y)).or_else(|| {
        frame
            .items
            .iter()
            .find(|item| item.serial == serial)
            .map(|item| (item.x, item.y))
    })?;
    Some(tile_distance(place, (frame.x, frame.y)))
}

/// While the shard waits for a target, draws the aura under the mouse and
/// the distance of the thing under it, as the Video and General pages ask.
pub fn draw_targeting(
    ctx: &egui::Context,
    scene: &Scene,
    frame: &WatchFrame,
    rect: Rect,
    profile: &Profile,
) {
    if !frame.target_cursor {
        return;
    }
    let Some(mouse) = ctx
        .pointer_hover_pos()
        .filter(|at| scene.is_on_world(rect, *at))
    else {
        return;
    };
    let painter = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new(CURSOR_LAYER)));
    if profile.video.aura_on_mouse {
        let color = scene.words_color(aura_hue(frame.target_flags));
        let mut mesh = Mesh::default();
        mesh.colored_vertex(mouse, color);
        for point in 0..AURA_EDGE_POINTS {
            let turn = std::f32::consts::TAU * point as f32 / AURA_EDGE_POINTS as f32;
            let edge = mouse + Vec2::angled(turn) * AURA_RADIUS;
            mesh.colored_vertex(edge, Color32::TRANSPARENT);
            let next = (point + 1) % AURA_EDGE_POINTS;
            mesh.add_triangle(0, point as u32 + 1, next as u32 + 1);
        }
        painter.add(mesh);
    }
    if profile.general.target_range_indicator {
        let distance = scene
            .thing_at(mouse)
            .and_then(|thing| distance_to(frame, thing.serial));
        if let Some(distance) = distance {
            theme::shadowed_text(
                &painter,
                mouse + RANGE_OFFSET,
                Align2::LEFT_TOP,
                &distance.to_string(),
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchItem, WatchMobile};

    #[test]
    fn the_aura_takes_the_hue_of_what_the_target_does() {
        assert_eq!(aura_hue(0), HUE_NEUTRAL);
        assert_eq!(aura_hue(TARGET_HARMFUL), HUE_HARMFUL);
        assert_eq!(aura_hue(TARGET_BENEFICIAL), HUE_BENEFICIAL);
    }

    #[test]
    fn the_distance_is_to_a_mobile_or_an_item_in_view() {
        let frame = WatchFrame {
            x: 100,
            y: 100,
            mobiles: vec![WatchMobile {
                serial: 5,
                x: 103,
                y: 98,
                ..WatchMobile::default()
            }],
            items: vec![WatchItem {
                serial: 9,
                x: 100,
                y: 107,
                ..WatchItem::default()
            }],
            ..WatchFrame::default()
        };
        assert_eq!(distance_to(&frame, 5), Some(3));
        assert_eq!(distance_to(&frame, 9), Some(7));
        assert_eq!(distance_to(&frame, 1), None);
    }

    const CHARACTER: Pos2 = Pos2::new(400.0, 300.0);

    #[test]
    fn the_arrow_points_from_the_character_to_the_mouse() {
        // The map is turned by an eighth: up the screen is north-west.
        let up = CHARACTER - Vec2::new(0.0, 100.0);
        assert_eq!(
            cursor_shape(false, CHARACTER, up),
            CursorShape::Walk(Direction::Northwest)
        );
        let right = CHARACTER + Vec2::new(100.0, 0.0);
        assert_eq!(
            cursor_shape(false, CHARACTER, right),
            CursorShape::Walk(Direction::Northeast)
        );
        let down_right = CHARACTER + Vec2::new(100.0, 100.0);
        assert_eq!(
            cursor_shape(false, CHARACTER, down_right),
            CursorShape::Walk(Direction::East)
        );
    }

    #[test]
    fn a_target_request_shows_the_cross() {
        assert_eq!(
            cursor_shape(true, CHARACTER, CHARACTER),
            CursorShape::Target
        );
    }
}
