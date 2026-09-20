//! What the panels and the map share while the human plays: the item on
//! the mouse, the places that take a dropped item, and the box that asks
//! how much of a pile to move.

use super::control::{Act, DropTo, Hand};
use super::scene::Scene;
use super::theme::{self, number_font, text_font};
use crate::view::{WatchFrame, WatchPackItem};
use eframe::egui::{self, Align2, Color32, Id, Key, Pos2, Rect, Sense, Vec2};

const CARRY_SIDE: f32 = 52.0;
const CARRY_ALPHA: f32 = 0.85;
const SPLIT_SIZE: Vec2 = Vec2::new(260.0, 118.0);
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
    zones: Vec<(Rect, Zone)>,
    split: Option<Split>,
    /// The slot an item was dropped on this frame, for the hotbar to take.
    pub slotted: Option<(usize, WatchPackItem)>,
}

impl Desk {
    /// Call this first in each frame. The panels then name their zones again.
    pub fn begin(&mut self) {
        self.zones.clear();
        self.slotted = None;
    }

    pub fn zone(&mut self, area: Rect, zone: Zone) {
        self.zones.push((area, zone));
    }

    pub fn carries(&self) -> bool {
        self.carry.is_some()
    }

    /// A panel calls this when the human starts to drag one of its items.
    pub fn pick_up(&mut self, item: &WatchPackItem) {
        self.carry = Some(item.clone());
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
    /// no zone, where a drop does nothing.
    pub fn carry_and_land(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        scene: &mut Scene,
        hand: &Hand,
        on_panel: bool,
    ) {
        let Some(item) = self.carry.clone() else {
            return;
        };
        let (mouse, released, whole_pile) = ui.input(|i| {
            (
                i.pointer.interact_pos(),
                i.pointer.any_released(),
                !i.modifiers.shift,
            )
        });
        let Some(mouse) = mouse else {
            self.carry = None;
            return;
        };
        if !released {
            let painter = ui.ctx().layer_painter(egui::LayerId::new(
                egui::Order::Tooltip,
                Id::new("desk-carry"),
            ));
            let area = Rect::from_center_size(mouse, Vec2::splat(CARRY_SIDE));
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
        if item.amount > 1 && !whole_pile {
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

    /// The box that asks how much of a pile to move. Gives its place.
    pub fn split_box(&mut self, ui: &mut egui::Ui, rect: Rect, hand: &Hand) -> Option<Rect> {
        let split = self.split.as_mut()?;
        let panel = Rect::from_center_size(rect.center(), SPLIT_SIZE);
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            format!("{WORDS_SPLIT}  {}", split.item.name),
            text_font(theme::SIZE_BODY),
            theme::TEXT,
        );
        let slider_row = Rect::from_min_size(
            inner.left_top() + Vec2::new(0.0, SPLIT_ROW),
            Vec2::new(inner.width(), SPLIT_ROW),
        );
        ui.put(
            slider_row,
            egui::Slider::new(&mut split.amount, 1..=split.item.amount).text_color(theme::TEXT),
        );
        ui.painter().text(
            inner.right_top(),
            Align2::RIGHT_TOP,
            split.amount.to_string(),
            number_font(theme::SIZE_BODY),
            theme::GOAL,
        );
        let (_, go) = theme::button(
            ui,
            inner.left_bottom() - Vec2::new(0.0, SPLIT_ROW),
            WORDS_MOVE,
            theme::GOAL,
        );
        let backdrop = ui.interact(panel, Id::new("desk-split"), Sense::click());
        let cancel = ui.input(|i| i.key_pressed(Key::Escape))
            || (ui.input(|i| i.pointer.any_click()) && !backdrop.hovered());
        if go || ui.input(|i| i.key_pressed(Key::Enter)) {
            hand.act(Act::Move {
                item: split.item.serial,
                amount: split.amount,
                to: split.to,
            });
            self.split = None;
        } else if cancel {
            self.split = None;
        }
        Some(panel)
    }
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
