//! The agent windows: one for each agent the Agents page offers, one that
//! lists them, and the ignore list. Which windows show, the rows of each
//! and what a press on one does are `uoterm_view::ui::agents`' and
//! `uoterm_view::ui::lists`'; this file draws them.

use super::super::boxes_ui::Tools;
use super::super::model::agents::AgentsView;
use super::super::model::places;
use super::super::settings::Profile;
use super::frame::{self, FrameEvent, PanelSpec};
use super::rows::{Picked, Rows};
use crate::view::{WatchFrame, WatchPackItem};
use crate::window::bridge;
use eframe::egui::{self, Color32, Rect};
use std::collections::HashMap;
use uoterm_runtime::tools::TOOL_AGENTS;
use uoterm_view::ui::agents::{
    agents_key, open_windows, AgentWindow, AgentsState, AGENTS_MAX_AGE, AGENT_WINDOW_MIN,
};
use uoterm_view::ui::lists::{agent_bag_items, AgentButton, AgentPress, AgentRow};

#[derive(Default)]
pub struct AgentsUi {
    scroll: HashMap<String, f32>,
    state: AgentsState,
}

/// The items of the backpack and its open bags, one of each graphic and
/// hue, with their pictures.
fn bag_items(frame: &WatchFrame, tools: &mut Tools<'_>) -> (Vec<Picked>, Vec<WatchPackItem>) {
    let seen = agent_bag_items(frame);
    let pictures = seen
        .iter()
        .map(|item| {
            (
                tools.scene.item_picture(item.graphic, item.hue),
                item.name.clone(),
            )
        })
        .collect();
    (pictures, seen)
}

/// The words and colors of the buttons of a row.
fn labels(buttons: &[AgentButton]) -> Vec<(&str, Color32)> {
    buttons
        .iter()
        .map(|button| (button.words.as_str(), bridge::color(button.color)))
        .collect()
}

/// The press of the button at `at`, when one was pressed.
fn pressed(buttons: &[AgentButton], at: Option<usize>) -> Option<AgentPress> {
    at.and_then(|at| buttons.get(at))
        .map(|button| button.press.clone())
}

impl AgentsUi {
    /// Draws the agent windows that are open. Gives the places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Vec<Rect> {
        let windows = open_windows(profile);
        let reads_agents = windows.iter().any(|window| *window != AgentWindow::Ignore);
        let view = reads_agents
            .then(|| {
                tools
                    .readings
                    .want(agents_key(), AGENTS_MAX_AGE)
                    .map(AgentsView::new)
            })
            .flatten();
        let mut agent_at = 0;
        let mut covered = Vec::new();
        for window in windows {
            let at = agent_at;
            if matches!(window, AgentWindow::Agent(_)) {
                agent_at += 1;
            }
            covered.push(self.window(ui, rect, frame, tools, profile, window, at, view.as_ref()));
        }
        covered
    }

    /// One window: the list of the agents, an agent's window, or the
    /// ignore list. `at` counts the agent windows before it.
    #[allow(clippy::too_many_arguments)]
    fn window(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        window: AgentWindow,
        at: usize,
        view: Option<&AgentsView>,
    ) -> Rect {
        let id = window.place_id();
        let spec = PanelSpec {
            id: &id,
            title: window.title(),
            default: bridge::rect(window.first_place(bridge::area(rect), at)),
            min_size: Some(bridge::vec2(AGENT_WINDOW_MIN)),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, window.title());
        let (pictures, bag) = bag_items(frame, tools);
        let failure = tools.readings.failure(&agents_key()).map(str::to_string);
        let agent_rows = self
            .state
            .rows(window, view, failure.as_deref(), frame, profile, &bag);
        let mut typed = self.state.typed(window).to_string();
        let scroll = self.scroll.get(&id).copied().unwrap_or_default();
        let (presses, scroll) = {
            let mut rows = Rows::new(ui, body, &id, scroll);
            let mut presses = Vec::new();
            for row in &agent_rows {
                presses.extend(self.row(&mut rows, row, &pictures, &mut typed, window));
            }
            (presses, rows.finish())
        };
        self.scroll.insert(id.clone(), scroll);
        if typed != self.state.typed(window) {
            self.state.set_typed(window, &typed);
        }
        let (acts, changed) = self.state.apply(presses, window, profile);
        if changed {
            tools.keep_profile(profile);
        }
        if !acts.is_empty() && frame.human_control {
            for act in acts {
                tools.hand.act(act);
            }
            tools.readings.refresh(TOOL_AGENTS);
        }
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            places::set_open(profile, &id, false);
            tools.keep_profile(profile);
        }
        panel
    }

    /// Draws one row. `pictures` are those of `agent_bag_items`, and
    /// `typed` the words of the field of the window. Gives what the player
    /// pressed.
    fn row(
        &mut self,
        rows: &mut Rows<'_>,
        row: &AgentRow,
        pictures: &[Picked],
        typed: &mut String,
        window: AgentWindow,
    ) -> Vec<AgentPress> {
        match row {
            AgentRow::Words { words, color } => {
                rows.words(words, bridge::color(*color));
                Vec::new()
            }
            AgentRow::Labeled {
                part,
                at,
                words,
                buttons,
                width,
            } => {
                let picked = rows.labeled(part, *at, words, &labels(buttons), *width);
                pressed(buttons, picked).into_iter().collect()
            }
            AgentRow::Buttons { part, buttons } => {
                let picked = rows.buttons(part, &labels(buttons));
                pressed(buttons, picked).into_iter().collect()
            }
            AgentRow::Pictures { part, presses } => {
                let picked = rows.pictures(part, pictures);
                picked
                    .and_then(|at| presses.get(at))
                    .cloned()
                    .into_iter()
                    .collect()
            }
            AgentRow::Field {
                part,
                hint,
                button,
                width,
                field_use,
            } => {
                if !rows.field(part, typed, hint, button, *width) {
                    return Vec::new();
                }
                self.state.set_typed(window, typed);
                let made = self.state.submit(window, field_use);
                *typed = self.state.typed(window).to_string();
                made
            }
        }
    }
}
