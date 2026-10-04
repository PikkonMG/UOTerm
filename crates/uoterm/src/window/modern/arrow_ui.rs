//! The clicks on the quest arrow of the Modern style, for egui. What a click
//! does is `uoterm_view::ui::quest_arrow`, which the browser shares.

use super::super::bridge;
use super::super::control::Hand;
use crate::view::WatchFrame;
use eframe::egui::{self, Id, Rect, Sense};
use uoterm_view::ui::quest_arrow::{arrow_act, arrow_click_area, HINT_ARROW};

const ARROW_CLICK_ID: &str = "modern-quest-arrow";

/// Takes the clicks on the arrow whose box is `arrow`. Gives the place it
/// takes clicks in, so the map does not take them.
pub fn click(ui: &egui::Ui, arrow: Rect, frame: &WatchFrame, hand: &Hand) -> Rect {
    let area = bridge::rect(arrow_click_area(bridge::area(arrow)));
    let response = ui.interact(area, Id::new(ARROW_CLICK_ID), Sense::click());
    if !frame.human_control {
        return area;
    }
    if response.hovered() {
        super::super::tips::label(ui, HINT_ARROW, "");
    }
    let right = response.secondary_clicked();
    if response.clicked() || right {
        if let Some(act) = arrow_act(frame, right) {
            hand.act(act);
        }
    }
    area
}
