//! The network statistics and the debug window of the Modern style: the
//! round trip to the shard in a color that goes from green to red as it
//! grows, with the bytes that came and went, and the frames each second
//! with the zoom and where the character is. A double click on either
//! shows more or less, as the classic gumps do. The words are
//! `uoterm_view::ui::info_bar`'s.

use super::super::boxes_ui::Tools;
use super::super::model::places;
use super::super::model::stats::{self, DebugFacts};
use super::super::settings::Profile;
use super::super::theme::{self, number_font};
use super::frame::{self, FrameEvent, PanelSpec};
use crate::view::WatchFrame;
use crate::window::bridge;
use eframe::egui::{self, Id, Rect, Sense};
use uoterm_view::ui::info_bar::{stats_first_place, stats_title, StatsLook, StatsPanel, HINT_MORE};
use uoterm_view::ui::launch::{DEBUG_ID, NET_STATS_ID};

/// The two panels, and whether each shows its other words.
#[derive(Default)]
pub struct StatsUi {
    look: StatsLook,
}

/// One panel of words. Gives its place, whether it was double-clicked and
/// whether it was closed.
fn words_panel(
    ui: &egui::Ui,
    rect: Rect,
    tools: &Tools<'_>,
    profile: &mut Profile,
    (id, panel): (&str, StatsPanel),
    (words, color): (String, egui::Color32),
) -> (Rect, bool, bool) {
    let galley = ui
        .painter()
        .layout_no_wrap(words, number_font(theme::SIZE_SMALL), color);
    let title = stats_title(panel);
    let spec = PanelSpec {
        id,
        title,
        default: bridge::rect(stats_first_place(
            bridge::area(rect),
            panel,
            bridge::vector(galley.size()),
        )),
        min_size: None,
        closable: true,
    };
    let placed = frame::place(rect, &spec, profile);
    let body = frame::draw(ui.painter(), placed, title);
    ui.painter().galley(body.min, galley, color);
    let response = ui.interact(body, Id::new(("stats-body", id)), Sense::click());
    if response.hovered() {
        super::super::tips::label(ui, HINT_MORE, "");
    }
    let closed = frame::controls(ui, placed, &spec, profile, tools) == Some(FrameEvent::Closed);
    (placed, response.double_clicked(), closed)
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
        let facts = DebugFacts {
            fps: stats::fps(ui.ctx().input(|i| i.stable_dt)),
            zoom: tools.scene.zoom(),
            selected: ui
                .ctx()
                .pointer_hover_pos()
                .and_then(|at| tools.scene.thing_at(at))
                .map(|pick| pick.serial),
        };
        for (id, panel) in [
            (NET_STATS_ID, StatsPanel::Network),
            (DEBUG_ID, StatsPanel::Debug),
        ] {
            if !places::is_open(profile, id) {
                continue;
            }
            let (words, color) = self.look.words(panel, frame, &facts);
            let (placed, flip, closed) = words_panel(
                ui,
                rect,
                tools,
                profile,
                (id, panel),
                (words, bridge::color(color)),
            );
            if flip {
                self.look.flip(panel);
            }
            covered.push(placed);
            if closed {
                places::set_open(profile, id, false);
                tools.keep_profile(profile);
            }
            if panel == StatsPanel::Debug {
                // The frames each second change all the time.
                ui.ctx().request_repaint();
            }
        }
        covered
    }
}
