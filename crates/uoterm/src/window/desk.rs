//! The desk of the window: it draws the item on the mouse and the box that
//! asks how much of a pile to move. What a drop does is
//! `uoterm_view::desk`.

pub use uoterm_view::desk::*;

use super::boxes_ui::Tools;
use super::bridge;
use super::control::{Act, Hand};
use super::modern::frame::{self, PanelSpec};
use super::scene::Scene;
use super::settings::Profile;
use super::theme::{self, number_font};
use crate::view::{WatchFrame, WatchPackItem};
use eframe::egui::{self, Color32, Id, Key, Pos2, Rect, Sense, Vec2};
use uoterm_nav::TileFlagSet;
use uoterm_view::geom::Point;

const CARRY_SIDE: f32 = 52.0;
const CARRY_ALPHA: f32 = 0.85;

/// The map of the window under the mouse.
struct SceneUnder<'a> {
    scene: &'a mut Scene,
    rect: Rect,
    frame: &'a WatchFrame,
}

impl MapUnder for SceneUnder<'_> {
    fn thing_at(&self, at: Point) -> Option<u32> {
        self.scene
            .thing_at(bridge::pos2(at))
            .map(|thing| thing.serial)
    }

    fn tile_at(&mut self, at: Point) -> (u16, u16, i8) {
        self.scene.tile_at(self.rect, self.frame, bridge::pos2(at))
    }

    /// With no tile data the amount of the item tells.
    fn stacks(&self, graphic: u16) -> bool {
        self.scene
            .item_tile(graphic)
            .is_none_or(|tile| tile.flags.contains(TileFlagSet::STACKABLE))
    }
}

/// For a gump that lands a dropped item itself: the carried item and its
/// grab, when the mouse button came up in this frame.
pub fn land(desk: &mut Desk, ui: &egui::Ui) -> Option<(WatchPackItem, Vec2)> {
    let released = ui.input(|i| i.pointer.any_released());
    desk.land(released)
        .map(|(item, grab)| (item, bridge::vec2(grab)))
}

/// Draws the carried item at the mouse and lands it when the button comes
/// up. `on_panel` is true when the mouse is on a panel that is no zone,
/// where a drop does nothing. A pile asks how many to move by the "Hold
/// Shift to split stacks" option, `shift_to_split`.
#[allow(clippy::too_many_arguments)]
pub fn carry_and_land(
    desk: &mut Desk,
    ui: &egui::Ui,
    rect: Rect,
    frame: &WatchFrame,
    scene: &mut Scene,
    hand: &Hand,
    on_panel: bool,
    shift_to_split: bool,
) {
    let mouse = ui.input(|i| DeskMouse {
        at: i.pointer.interact_pos().map(bridge::point),
        released: i.pointer.any_released(),
        shift: i.modifiers.shift,
        on_panel,
    });
    let carried = desk.carried().cloned();
    let mut map = SceneUnder { scene, rect, frame };
    match desk.landing(mouse, shift_to_split, &mut map) {
        Landing::Nothing => {}
        Landing::Carried(at) => {
            if let Some(item) = carried {
                draw_carried(ui, bridge::pos2(at), &item, map.scene);
            }
        }
        Landing::AskAmount(split) => desk.ask_amount(split),
        Landing::Act(act) => hand.act(act),
    }
}

/// The picture of the carried item, with its middle at `at`.
fn draw_carried(ui: &egui::Ui, at: Pos2, item: &WatchPackItem, scene: &mut Scene) {
    let painter = ui.ctx().layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        Id::new("desk-carry"),
    ));
    let area = Rect::from_center_size(at, Vec2::splat(CARRY_SIDE));
    if let Some((texture, sprite)) = scene.item_picture(item.graphic, item.hue) {
        let fitted = theme::fit(area, sprite.width, sprite.height);
        let tint = theme::with_alpha(Color32::WHITE, CARRY_ALPHA);
        painter.image(texture, fitted, bridge::rect(sprite.uv), tint);
    }
    ui.ctx().request_repaint();
}

/// The box that asks how much of a pile to move, which the player moves
/// and locks. Gives its place.
pub fn split_box(
    ui: &mut egui::Ui,
    rect: Rect,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> Option<Rect> {
    let mut split = tools.desk.take_split()?;
    let title = split_title(&split.item.name);
    let spec = PanelSpec {
        id: SPLIT_ID,
        title: &title,
        default: bridge::rect(split_first_place(bridge::area(rect))),
        min_size: None,
        closable: true,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), panel, &title);
    let slider_row = Rect::from_min_size(body.left_top(), Vec2::new(body.width(), SPLIT_ROW));
    ui.put(
        slider_row,
        egui::Slider::new(&mut split.amount, 1..=split.item.amount).text_color(theme::TEXT),
    );
    ui.painter().text(
        slider_row.right_center(),
        egui::Align2::RIGHT_CENTER,
        split.amount.to_string(),
        number_font(theme::SIZE_BODY),
        theme::GOAL,
    );
    let (_, go) = theme::button(
        ui,
        Pos2::new(body.left(), slider_row.bottom()),
        WORDS_MOVE,
        theme::GOAL,
    );
    let backdrop = ui.interact(panel, Id::new("desk-split"), Sense::click());
    let closed = frame::controls(ui, panel, &spec, profile, tools).is_some();
    let cancel = closed
        || ui.input(|i| i.key_pressed(Key::Escape))
        || (ui.input(|i| i.pointer.any_click()) && !backdrop.hovered());
    if go || ui.input(|i| i.key_pressed(Key::Enter)) {
        tools.hand.act(Act::Move {
            item: split.item.serial,
            amount: split.amount,
            to: split.to,
        });
    } else if !cancel {
        tools.desk.ask_amount(split);
    }
    Some(panel)
}
