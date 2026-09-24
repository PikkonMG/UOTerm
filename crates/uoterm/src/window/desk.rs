//! What the panels and the map share while the human plays: the item on
//! the mouse, the places that take a dropped item, and the box that asks
//! how much of a pile to move, by the "Hold Shift to split stacks" option.

use super::boxes_ui::Tools;
use super::control::{Act, DropTo, Hand};
use super::model::clicks::asks_amount;
use super::modern::frame::{self, PanelSpec};
use super::modern::layout::{self, Spot};
use super::scene::Scene;
use super::settings::Profile;
use super::theme::{self, number_font};
use crate::view::{WatchFrame, WatchPackItem};
use eframe::egui::{self, Color32, Id, Key, Pos2, Rect, Sense, Vec2};
use uoterm_nav::TileFlagSet;

const CARRY_SIDE: f32 = 52.0;
const CARRY_ALPHA: f32 = 0.85;
const SPLIT_ID: &str = "modern:split";
const SPLIT_WIDTH: f32 = 260.0;
const SPLIT_ROW: f32 = 28.0;
const WORDS_SPLIT: &str = "How many?";
const WORDS_MOVE: &str = "Move";

/// What a dropped item lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    /// A container, a pile to join, or a mobile to give to.
    Into(u32),
    /// The body of the character: the item is put on.
    Wear,
    /// A slot of the hotbar. The item stays where it is.
    Slot(usize),
}

/// A pile that waits for its amount.
struct Split {
    item: WatchPackItem,
    to: DropTo,
    amount: u16,
}

#[derive(Default)]
pub struct Desk {
    carry: Option<WatchPackItem>,
    /// Where the middle of the carried picture is from the mouse, in window
    /// points.
    grab: Vec2,
    /// The item was picked up in this frame, so a mouse button that comes up
    /// in it, as the click of an Okay button, does not drop it.
    picked_now: bool,
    zones: Vec<(Rect, Zone)>,
    split: Option<Split>,
    /// The slot an item was dropped on this frame, for the hotbar to take.
    pub slotted: Option<(usize, WatchPackItem)>,
    /// The container whose gump is under the mouse in this frame, and in
    /// the last one, whose item the other gumps mark.
    hovered_container: Option<u32>,
    marked_container: Option<u32>,
}

impl Desk {
    /// Call this first in each frame. The panels then name their zones again.
    pub fn begin(&mut self) {
        self.zones.clear();
        self.slotted = None;
        self.marked_container = self.hovered_container.take();
        self.picked_now = false;
    }

    /// A container gump is under the mouse: the other gumps mark its item.
    pub fn hover_container(&mut self, container: u32) {
        self.hovered_container = Some(container);
    }

    /// The container whose gump was under the mouse in the last frame.
    pub fn marked_container(&self) -> Option<u32> {
        self.marked_container
    }

    pub fn zone(&mut self, area: Rect, zone: Zone) {
        self.zones.push((area, zone));
    }

    pub fn carries(&self) -> bool {
        self.carry.is_some()
    }

    /// A panel calls this when the human starts to drag one of its items.
    pub fn pick_up(&mut self, item: &WatchPackItem) {
        self.pick_up_at(item, Vec2::ZERO);
    }

    /// Picks up an item that stays `grab` away from the mouse, as a gump
    /// with relative drag and drop holds it.
    pub fn pick_up_at(&mut self, item: &WatchPackItem, grab: Vec2) {
        self.carry = Some(item.clone());
        self.grab = grab;
        self.picked_now = true;
    }

    /// The item on the mouse.
    pub fn carried(&self) -> Option<&WatchPackItem> {
        self.carry.as_ref()
    }

    /// For a gump that lands a dropped item itself, as a container puts it
    /// at the place of the mouse: the carried item and its grab, when the
    /// mouse button came up in this frame. The desk then carries nothing.
    pub fn land(&mut self, ui: &egui::Ui) -> Option<(WatchPackItem, Vec2)> {
        if self.picked_now || !ui.input(|i| i.pointer.any_released()) {
            return None;
        }
        self.carry.take().map(|item| (item, self.grab))
    }

    /// The last zone named is on top, as it was drawn last.
    fn zone_at(&self, point: Pos2) -> Option<Zone> {
        self.zones
            .iter()
            .rev()
            .find(|(area, _)| area.contains(point))
            .map(|(_, zone)| *zone)
    }

    /// Draws the carried item at the mouse and lands it when the button
    /// comes up. `on_panel` is true when the mouse is on a panel that is
    /// no zone, where a drop does nothing. A pile asks how many to move by
    /// the "Hold Shift to split stacks" option, `shift_to_split`.
    #[allow(clippy::too_many_arguments)]
    pub fn carry_and_land(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
        hand: &Hand,
        on_panel: bool,
        shift_to_split: bool,
    ) {
        let Some(item) = self.carry.clone() else {
            return;
        };
        let (mouse, released, shift) = ui.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.any_released(),
                i.modifiers.shift,
            )
        });
        let released = released && !self.picked_now;
        let Some(mouse) = mouse else {
            self.carry = None;
            return;
        };
        if !released {
            let painter = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                Id::new("desk-carry"),
            ));
            let area = Rect::from_center_size(mouse + self.grab, Vec2::splat(CARRY_SIDE));
            if let Some((texture, sprite)) = scene.item_picture(frame.map, item.graphic, item.hue) {
                let fitted = theme::fit(area, sprite.width, sprite.height);
                let tint = theme::with_alpha(Color32::WHITE, CARRY_ALPHA);
                painter.image(texture, fitted, sprite.uv, tint);
            }
            ui.ctx().request_repaint();
            return;
        }
        self.carry = None;
        let zone = self.zone_at(mouse);
        if let Some(Zone::Slot(slot)) = zone {
            self.slotted = Some((slot, item));
            return;
        }
        let to = match zone {
            Some(Zone::Wear) => {
                hand.act(Act::Wear(item.serial));
                return;
            }
            Some(Zone::Into(dest)) if dest != item.serial => DropTo::Into(dest),
            Some(_) => return,
            None if on_panel => return,
            None => match scene.thing_at(mouse) {
                Some(thing) if thing.serial != item.serial => DropTo::Into(thing.serial),
                Some(_) => return,
                None => {
                    let (x, y, z) = scene.tile_at(rect, frame, mouse);
                    DropTo::Ground { x, y, z }
                }
            },
        };
        if asks_amount(
            item.amount,
            stacks(scene, item.graphic),
            shift_to_split,
            shift,
        ) {
            self.split = Some(Split {
                amount: item.amount,
                item,
                to,
            });
            return;
        }
        hand.act(Act::Move {
            item: item.serial,
            amount: item.amount.max(1),
            to,
        });
    }
}

/// True when items of this graphic stack into one pile. With no tile data
/// the amount of the item tells.
fn stacks(scene: &Scene, graphic: u16) -> bool {
    scene
        .item_tile(graphic)
        .is_none_or(|tile| tile.flags.contains(TileFlagSet::STACKABLE))
}

/// The box that asks how much of a pile to move, which the player moves
/// and locks. Gives its place.
pub fn split_box(
    ui: &mut egui::Ui,
    rect: Rect,
    tools: &mut Tools<'_>,
    profile: &mut Profile,
) -> Option<Rect> {
    let mut split = tools.desk.split.take()?;
    let title = format!("{WORDS_SPLIT}  {}", split.item.name);
    let spec = PanelSpec {
        id: SPLIT_ID,
        title: &title,
        default: layout::first_place(
            rect,
            Spot::Middle(0),
            Vec2::new(
                SPLIT_WIDTH,
                frame::TITLE_ROW + SPLIT_ROW * 2.0 + theme::PANEL_PAD * 2.0,
            ),
        ),
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
        tools.desk.split = Some(split);
    }
    Some(panel)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BAG: u32 = 0x4000_0002;

    #[test]
    fn the_zone_drawn_last_is_on_top() {
        let mut desk = Desk::default();
        let area = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        desk.zone(area, Zone::Into(BAG));
        desk.zone(area.shrink(20.0), Zone::Wear);
        assert_eq!(desk.zone_at(Pos2::new(50.0, 50.0)), Some(Zone::Wear));
        assert_eq!(desk.zone_at(Pos2::new(5.0, 5.0)), Some(Zone::Into(BAG)));
        assert_eq!(desk.zone_at(Pos2::new(500.0, 5.0)), None);
        desk.begin();
        assert_eq!(desk.zone_at(Pos2::new(50.0, 50.0)), None);
    }
}
