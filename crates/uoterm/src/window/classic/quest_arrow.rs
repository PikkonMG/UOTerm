//! The quest arrow of the classic client: the
//! gump picture of an arrow that points from the character toward the place
//! the shard marked, standing at that place, or at the edge of the game
//! view when the place is out of view. It blinks red once a second. A click
//! tells the shard with the left or the right button (0xBF 0x07).

use crate::view::WatchFrame;
use crate::window::control::{Act, Hand};
use crate::window::scene::Scene;
use eframe::egui::{self, Color32, Id, Order, Pos2, Rect, Sense, Vec2};

/// The arrows, one for each direction from north round to north-west,
/// start here; the arrow of a direction is the one after it.
const FIRST_ARROW: u16 = 0x1194;
const DIRECTIONS: u8 = 8;
const BLINK_HUE: u16 = 0x0021;
const NO_HUE: u16 = 0;
const BLINK_SECONDS: f64 = 1.0;
const BLINKS_PER_CYCLE: f64 = 2.0;
const AREA_ID: &str = "classic-quest-arrow";
const HALF: f32 = 2.0;

// The directions of the game, as the reference client numbers them.
const NORTH: u8 = 0;
const NORTH_EAST: u8 = 1;
const EAST: u8 = 2;
const SOUTH_EAST: u8 = 3;
const SOUTH: u8 = 4;
const SOUTH_WEST: u8 = 5;
const WEST: u8 = 6;
const NORTH_WEST: u8 = 7;
/// A step is more along one way than the other when it is this many times
/// longer, as the classic client reads the mouse.
const STRAIGHT_RATIO: (i32, i32) = (5, 2);

/// The direction from one tile to another, as the reference client
/// reads it: straight when a step is much longer one
/// way, else diagonal. The same tile gives north.
fn direction(from: (u16, u16), to: (u16, u16)) -> u8 {
    let dx = i32::from(to.0) - i32::from(from.0);
    let dy = i32::from(to.1) - i32::from(from.1);
    let (along, across) = STRAIGHT_RATIO;
    let (wide, tall) = (dx.abs(), dy.abs());
    let straight_x = dy == 0 || tall * along <= wide * across;
    let straight_y = dx == 0 || tall * across >= wide * along;
    match (dx.signum(), dy.signum()) {
        (0, 0) => NORTH,
        (-1, _) if straight_x => WEST,
        (1, _) if straight_x => EAST,
        (_, -1) if straight_y => NORTH,
        (_, 1) if straight_y => SOUTH,
        (-1, -1) => NORTH_WEST,
        (1, -1) => NORTH_EAST,
        (-1, 1) => SOUTH_WEST,
        _ => SOUTH_EAST,
    }
}

/// The gump picture of the arrow of a direction.
fn arrow_gump(direction: u8) -> u16 {
    FIRST_ARROW + u16::from((direction + 1) % DIRECTIONS)
}

/// How far the arrow's corner stands from the place, so its point is at
/// the place, as the reference client sets it for each direction.
fn corner_offset(direction: u8, size: Vec2) -> Vec2 {
    let (w, h) = (size.x, size.y);
    match direction {
        NORTH => Vec2::new(-w, 0.0),
        SOUTH => Vec2::new(0.0, -h),
        EAST => Vec2::new(-w, -h),
        NORTH_EAST => Vec2::new(-w, -h / HALF),
        SOUTH_WEST => Vec2::new(w / HALF, -h / HALF),
        NORTH_WEST => Vec2::new(-w / HALF, h / HALF),
        SOUTH_EAST => Vec2::new(-w / HALF, -h),
        _ => Vec2::ZERO,
    }
}

/// The arrow's corner held inside the game view.
fn held_in(corner: Pos2, size: Vec2, view: Rect) -> Pos2 {
    Pos2::new(
        corner
            .x
            .clamp(view.left(), (view.right() - size.x).max(view.left())),
        corner
            .y
            .clamp(view.top(), (view.bottom() - size.y).max(view.top())),
    )
}

/// Draws the arrow over the game view when the shard shows one, and sends
/// its clicks while the human has control.
pub fn draw(
    ctx: &egui::Context,
    view: Rect,
    frame: &WatchFrame,
    scene: &mut Scene,
    hand: &Hand,
    scale: f32,
) {
    let Some((x, y)) = frame.quest_arrow else {
        return;
    };
    let facing = direction((frame.x, frame.y), (x, y));
    let time = ctx.input(|i| i.time);
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(BLINK_SECONDS));
    let hue = if (time / BLINK_SECONDS) % BLINKS_PER_CYCLE < 1.0 {
        NO_HUE
    } else {
        BLINK_HUE
    };
    let Some((texture, sprite)) = scene.gump_picture(arrow_gump(facing), hue) else {
        return;
    };
    let size = Vec2::new(sprite.width, sprite.height) * scale;
    let place = scene.screen_of(view, [f32::from(x), f32::from(y), f32::from(frame.z)]);
    let corner = held_in(place + corner_offset(facing, size), size, view);
    let rect = Rect::from_min_size(corner, size);
    egui::Area::new(Id::new(AREA_ID))
        .order(Order::Foreground)
        .fixed_pos(corner)
        .constrain(false)
        .show(ctx, |ui| {
            ui.painter().image(texture, rect, sprite.uv, Color32::WHITE);
            let response = ui.interact(rect, Id::new((AREA_ID, "click")), Sense::click());
            let right = response.secondary_clicked();
            if (response.clicked() || right) && frame.human_control {
                hand.act(Act::QuestArrow { right });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_arrow_points_as_the_classic_client_reads_directions() {
        let me = (100, 100);
        assert_eq!(direction(me, (100, 90)), NORTH);
        assert_eq!(direction(me, (110, 100)), EAST);
        assert_eq!(direction(me, (100, 110)), SOUTH);
        assert_eq!(direction(me, (90, 100)), WEST);
        assert_eq!(direction(me, (110, 90)), NORTH_EAST);
        assert_eq!(direction(me, (110, 110)), SOUTH_EAST);
        assert_eq!(direction(me, (90, 110)), SOUTH_WEST);
        assert_eq!(direction(me, (90, 90)), NORTH_WEST);
        assert_eq!(direction(me, (110, 99)), EAST, "much more east than north");
        assert_eq!(
            direction(me, (101, 110)),
            SOUTH,
            "much more south than east"
        );
        assert_eq!(direction(me, me), NORTH);
    }

    #[test]
    fn each_direction_has_its_arrow_and_its_point() {
        assert_eq!(arrow_gump(NORTH), FIRST_ARROW + 1);
        assert_eq!(arrow_gump(NORTH_WEST), FIRST_ARROW);
        let size = Vec2::new(20.0, 10.0);
        assert_eq!(corner_offset(WEST, size), Vec2::ZERO);
        assert_eq!(corner_offset(EAST, size), Vec2::new(-20.0, -10.0));
        let view = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));
        assert_eq!(
            held_in(Pos2::new(-50.0, 95.0), size, view),
            Pos2::new(0.0, 90.0)
        );
    }
}
