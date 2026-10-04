//! The mouse pointers of the classic client, drawn from the art files over
//! the world in the Classic style: an arrow that points the way from the
//! character to the mouse, from the war set in war mode, and the cross
//! while the shard waits for a target. The pointer of the system hides
//! while the mouse is over the world.
//!
//! While the shard waits for a target, both styles draw what the reference client's
//! cursor adds when the options ask: an aura under the mouse in the hue of
//! what the target does, and how far the thing under the mouse is. The
//! rules are `uoterm_view::cursor`; here the window draws.

use super::bridge;
use super::scene::Scene;
use super::settings::Profile;
use super::theme::{self, text_font};
use crate::view::WatchFrame;
use eframe::egui::{
    self, epaint::Mesh, Align2, Color32, CursorIcon, Id, LayerId, Order, Rect, Vec2,
};
use uoterm_nav::cursor_hue;
use uoterm_view::cursor::{aura_hue, cursor_shape, distance_to};

const CURSOR_LAYER: &str = "uoterm-classic-cursor";
/// The aura under the mouse, as the reference client's cursor draws it: this wide, in
/// the hue of what the target does.
const AURA_RADIUS: f32 = 30.0;
const AURA_EDGE_POINTS: usize = 32;
/// The distance stands this far up and left of the mouse.
const RANGE_OFFSET: Vec2 = Vec2::new(-26.0, -21.0);

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
    let shape = cursor_shape(
        frame.target_cursor,
        bridge::point(character),
        bridge::point(mouse),
    );
    let hue = cursor_hue(frame.war, frame.map);
    let Some((texture, sprite)) = scene.cursor_picture(shape, frame.war, hue) else {
        return;
    };
    ctx.set_cursor_icon(CursorIcon::None);
    let painter = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new(CURSOR_LAYER)));
    let area = Rect::from_min_size(
        mouse - bridge::vec2(sprite.anchor),
        Vec2::new(sprite.width, sprite.height),
    );
    painter.image(texture, area, bridge::rect(sprite.uv), Color32::WHITE);
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
