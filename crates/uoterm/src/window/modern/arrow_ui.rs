//! The clicks on the quest arrow of the Modern style: a click with the left
//! or the right button tells the shard (0xBF 0x07), as the classic arrow
//! does, while the human has control.

use super::super::control::{Act, Hand};
use crate::view::WatchFrame;
use eframe::egui::{self, Id, Rect, Sense};

const ARROW_CLICK_ID: &str = "modern-quest-arrow";
/// The arrow takes clicks this far round its point too, so it is easy to
/// hit.
const CLICK_ROOM: f32 = 6.0;
const HINT_ARROW: &str = "The shard points here. Click: tell the shard.";

/// Takes the clicks on the arrow whose box is `arrow`. Gives the place it
/// takes clicks in, so the map does not take them.
pub fn click(ui: &egui::Ui, arrow: Rect, frame: &WatchFrame, hand: &Hand) -> Rect {
    let area = arrow.expand(CLICK_ROOM);
    let response = ui.interact(area, Id::new(ARROW_CLICK_ID), Sense::click());
    if !frame.human_control {
        return area;
    }
    if response.hovered() {
        super::super::tips::label(ui, HINT_ARROW, "");
    }
    let right = response.secondary_clicked();
    if response.clicked() || right {
        hand.act(Act::QuestArrow { right });
    }
    area
}
