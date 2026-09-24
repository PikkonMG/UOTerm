//! The network statistics and the debug window of the Modern style: the
//! round trip to the shard in a color that goes from green to red as it
//! grows, with the bytes that came and went, and the frames each second
//! with the zoom and where the character is. A double click on either
//! shows more or less, as the classic gumps do. The words are
//! `model::stats`'.

use super::super::boxes_ui::Tools;
use super::super::model::places;
use super::super::model::stats::{self, DebugFacts, PingLevel};
use super::super::settings::Profile;
use super::super::theme::{self, number_font};
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use crate::view::WatchFrame;
use eframe::egui::{self, Color32, Id, Rect, Sense, Vec2};

pub const NET_STATS_ID: &str = "modern:net_stats";
pub const DEBUG_ID: &str = "modern:debug";
const WORDS_NET_STATS: &str = "Network";
const WORDS_DEBUG: &str = "Debug";
const HINT_MORE: &str = "Double-click: more or less.";
/// The panels first stand at the top of the middle, one under the other.
const NET_STATS_SPOT: Spot = Spot::MiddleTop(0);
const DEBUG_SPOT: Spot = Spot::MiddleTop(1);

/// The color of the round trip: green, yellow, orange or red.
fn ping_color(level: PingLevel) -> Color32 {
    match level {
        PingLevel::Quick => theme::HITS_POISONED,
        PingLevel::Fair => theme::STAM,
        PingLevel::Slow => theme::WAITING,
        PingLevel::Lagging => theme::ALARM,
    }
}

/// The two panels, and whether each shows its short words.
#[derive(Default)]
pub struct StatsUi {
    net_folded: bool,
    debug_full: bool,
}

/// One panel of words. Gives its place, whether it was double-clicked and
/// whether it was closed.
fn words_panel(
    ui: &egui::Ui,
    rect: Rect,
    tools: &Tools<'_>,
    profile: &mut Profile,
    (id, title, spot): (&str, &str, Spot),
    (words, color): (&str, Color32),
) -> (Rect, bool, bool) {
    let galley =
        ui.painter()
            .layout_no_wrap(words.to_string(), number_font(theme::SIZE_SMALL), color);
    let size = frame::with_title_room(
        galley.size() + Vec2::new(0.0, frame::TITLE_ROW) + Vec2::splat(theme::PANEL_PAD * 2.0),
    );
    let spec = PanelSpec {
        id,
        title,
        default: layout::first_place(rect, spot, size),
        min_size: None,
        closable: true,
    };
    let panel = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), panel, title);
    ui.painter().galley(body.min, galley, color);
    let response = ui.interact(body, Id::new(("stats-body", id)), Sense::click());
    if response.hovered() {
        super::super::tips::label(ui, HINT_MORE, "");
    }
    let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
    (panel, response.double_clicked(), closed)
}

impl StatsUi {
    /// Draws the panels that are open. Gives their places.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Vec<Rect> {
        let mut covered = Vec::new();
        if places::is_open(profile, NET_STATS_ID) {
            let words = stats::net_words(frame, self.net_folded);
            let color = ping_color(stats::ping_level(stats::ping(frame)));
            let (panel, flip, closed) = words_panel(
                ui,
                rect,
                tools,
                profile,
                (NET_STATS_ID, WORDS_NET_STATS, NET_STATS_SPOT),
                (&words, color),
            );
            self.net_folded ^= flip;
            covered.push(panel);
            close_on(closed, NET_STATS_ID, tools, profile);
        }
        if places::is_open(profile, DEBUG_ID) {
            let facts = DebugFacts {
                fps: stats::fps(ui.ctx().input(|i| i.stable_dt)),
                zoom: tools.scene.zoom(),
                selected: ui
                    .ctx()
                    .pointer_hover_pos()
                    .and_then(|at| tools.scene.thing_at(at))
                    .map(|pick| pick.serial),
            };
            let words = stats::debug_words(frame, &facts, self.debug_full);
            let (panel, flip, closed) = words_panel(
                ui,
                rect,
                tools,
                profile,
                (DEBUG_ID, WORDS_DEBUG, DEBUG_SPOT),
                (&words, theme::TEXT),
            );
            self.debug_full ^= flip;
            covered.push(panel);
            close_on(closed, DEBUG_ID, tools, profile);
            // The frames each second change all the time.
            ui.ctx().request_repaint();
        }
        covered
    }
}

fn close_on(closed: bool, id: &str, tools: &Tools<'_>, profile: &mut Profile) {
    if closed {
        places::set_open(profile, id, false);
        tools.keep_profile(profile);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_round_trip_goes_from_green_to_red() {
        assert_eq!(ping_color(PingLevel::Quick), theme::HITS_POISONED);
        assert_eq!(ping_color(PingLevel::Lagging), theme::ALARM);
        assert_ne!(ping_color(PingLevel::Fair), ping_color(PingLevel::Slow));
    }
}
