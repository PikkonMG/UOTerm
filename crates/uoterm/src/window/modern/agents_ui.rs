//! The agent windows: one for each agent the Agents page offers, and one
//! that lists them. Each window reads the agent's settings from the session
//! and changes them with the agent tools; the session keeps them. The
//! ignore list is the profile's own, for speech the journal hides. The
//! rows of each window are `uoterm_view::ui::lists`'; this file draws them
//! and does what a press on one says.

use super::super::boxes_ui::Tools;
use super::super::control::Act;
use super::super::model::agents::{AgentPanel, AgentsView};
use super::super::model::places;
use super::super::model::reads::ReadKey;
use super::super::settings::Profile;
use super::frame::{self, FrameEvent, PanelSpec};
use super::layout::{self, Spot};
use super::rows::{Picked, Rows};
use crate::view::{WatchFrame, WatchPackItem};
use crate::window::bridge;
use eframe::egui::{self, Color32, Rect, Vec2};
use serde_json::json;
use std::collections::HashMap;
use uoterm_runtime::tools::TOOL_AGENTS;
use uoterm_view::ui::launch::{AGENTS_ID, IGNORE_ID};
use uoterm_view::ui::lists::{
    agent_bag_items, agent_chooser_rows, agent_list_rows, agent_settings_rows, field_presses,
    ignore_rows, AgentButton, AgentPress, AgentRow, WORDS_IGNORE,
};
use uoterm_view::ui::theme as shared_theme;

/// The agents are read this often while a window shows, in seconds.
const AGENTS_MAX_AGE: f64 = 2.0;
const WINDOW_SIZE: Vec2 = Vec2::new(320.0, 420.0);
const MIN_SIZE: Vec2 = Vec2::new(260.0, 160.0);
/// The list opens at the top of the right column and the ignore list a
/// step from it; each agent window opens in the middle, a step from the
/// one before.
const AGENTS_SPOT: Spot = Spot::RightColumn(0);
const IGNORE_SPOT: Spot = Spot::RightColumn(1);

const WORDS_AGENTS: &str = "Agents";
const WORDS_NOT_READ: &str = "The session has not answered yet.";

#[derive(Default)]
pub struct AgentsUi {
    scroll: HashMap<String, f32>,
    new_list: HashMap<AgentPanel, String>,
    /// The job list each window shows.
    chosen_list: HashMap<AgentPanel, String>,
    ignore_name: String,
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

/// Draws the rows of a window in order. `pictures` are those of
/// `agent_bag_items`, and `typed` the words of the field of the window.
/// Gives what the player pressed.
fn draw_rows(
    rows: &mut Rows<'_>,
    agent_rows: &[AgentRow],
    pictures: &[Picked],
    typed: &mut String,
) -> Vec<AgentPress> {
    let mut presses = Vec::new();
    for row in agent_rows {
        match row {
            AgentRow::Words { words, color } => rows.words(words, bridge::color(*color)),
            AgentRow::Labeled {
                part,
                at,
                words,
                buttons,
                width,
            } => {
                let picked = rows.labeled(part, *at, words, &labels(buttons), *width);
                presses.extend(pressed(buttons, picked));
            }
            AgentRow::Buttons { part, buttons } => {
                let picked = rows.buttons(part, &labels(buttons));
                presses.extend(pressed(buttons, picked));
            }
            AgentRow::Pictures {
                part,
                presses: by_picture,
            } => {
                let picked = rows.pictures(part, pictures);
                presses.extend(picked.and_then(|at| by_picture.get(at)).cloned());
            }
            AgentRow::Field {
                part,
                hint,
                button,
                width,
                field_use,
            } => {
                if rows.field(part, typed, hint, button, *width) {
                    let made = field_presses(field_use, typed);
                    if !made.is_empty() {
                        typed.clear();
                    }
                    presses.extend(made);
                }
            }
        }
    }
    presses
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
        let chooser = places::is_open(profile, AGENTS_ID);
        let windows: Vec<AgentPanel> = AgentPanel::ALL
            .into_iter()
            .filter(|panel| {
                panel.offered(&profile.agents) && places::is_open(profile, &panel.place_id())
            })
            .collect();
        let ignore = places::is_open(profile, IGNORE_ID);
        let mut covered = Vec::new();
        if ignore {
            covered.push(self.ignore_window(ui, rect, frame, tools, profile));
        }
        if !chooser && windows.is_empty() {
            return covered;
        }
        let view = tools
            .readings
            .want(ReadKey::new(TOOL_AGENTS, &json!({})), AGENTS_MAX_AGE)
            .map(AgentsView::new);
        if chooser {
            covered.push(self.chooser(ui, rect, frame, tools, profile, view.as_ref()));
        }
        for (at, panel) in windows.into_iter().enumerate() {
            covered.push(self.window(ui, rect, frame, tools, profile, panel, at, view.as_ref()));
        }
        covered
    }

    fn scroll_of(&self, id: &str) -> f32 {
        self.scroll.get(id).copied().unwrap_or_default()
    }

    /// Does what the presses of one window say: the acts go to the
    /// character while the human has control, and the reads of the agents
    /// come again soon.
    fn apply(
        &mut self,
        presses: Vec<AgentPress>,
        agent: Option<AgentPanel>,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) {
        let mut acts: Vec<Act> = Vec::new();
        let mut changed = false;
        for press in presses {
            match press {
                AgentPress::Acts(more) => acts.extend(more),
                AgentPress::Open(id, open) => {
                    places::set_open(profile, &id, open);
                    changed = true;
                }
                AgentPress::ChooseList(name) => {
                    if let Some(agent) = agent {
                        self.chosen_list.insert(agent, name);
                    }
                }
                AgentPress::Unignore(at) => {
                    if at < profile.ignore.names.len() {
                        profile.ignore.names.remove(at);
                        changed = true;
                    }
                }
                AgentPress::Ignore(name) => changed |= profile.ignore.add(&name),
            }
        }
        if changed {
            tools.keep_profile(profile);
        }
        if acts.is_empty() || !frame.human_control {
            return;
        }
        for act in acts {
            tools.hand.act(act);
        }
        tools.readings.refresh(TOOL_AGENTS);
    }

    fn close_on(event: Option<FrameEvent>, id: &str, tools: &mut Tools<'_>, profile: &mut Profile) {
        if event == Some(FrameEvent::Closed) {
            places::set_open(profile, id, false);
            tools.keep_profile(profile);
        }
    }

    /// The list of the agent windows, with the job that runs.
    fn chooser(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        view: Option<&AgentsView>,
    ) -> Rect {
        let spec = PanelSpec {
            id: AGENTS_ID,
            title: WORDS_AGENTS,
            default: layout::first_place(rect, AGENTS_SPOT, WINDOW_SIZE),
            min_size: Some(MIN_SIZE),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_AGENTS);
        let agent_rows = agent_chooser_rows(view, profile);
        let (presses, scroll) = {
            let mut rows = Rows::new(ui, body, AGENTS_ID, self.scroll_of(AGENTS_ID));
            let presses = draw_rows(&mut rows, &agent_rows, &[], &mut String::new());
            (presses, rows.finish())
        };
        self.scroll.insert(AGENTS_ID.to_string(), scroll);
        self.apply(presses, None, frame, tools, profile);
        let event = frame::controls(ui, panel, &spec, profile, tools);
        Self::close_on(event, AGENTS_ID, tools, profile);
        panel
    }

    /// One agent's window.
    #[allow(clippy::too_many_arguments)]
    fn window(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        agent: AgentPanel,
        at: usize,
        view: Option<&AgentsView>,
    ) -> Rect {
        let id = agent.place_id();
        let spec = PanelSpec {
            id: &id,
            title: agent.title(),
            default: layout::first_place(rect, Spot::Middle(at), WINDOW_SIZE),
            min_size: Some(MIN_SIZE),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, agent.title());
        let (pictures, bag) = bag_items(frame, tools);
        let agent_rows = match view {
            None => {
                let failure = tools
                    .readings
                    .failure(&ReadKey::new(TOOL_AGENTS, &json!({})));
                vec![AgentRow::Words {
                    words: failure.unwrap_or(WORDS_NOT_READ).to_string(),
                    color: shared_theme::TEXT_FAINT,
                }]
            }
            Some(view) => {
                let chosen = self.chosen_list.get(&agent).map(String::as_str);
                let mut agent_rows = agent_settings_rows(agent, view, frame, &bag);
                agent_rows.extend(agent_list_rows(agent, view, frame, chosen, &bag));
                agent_rows
            }
        };
        let typed = self.new_list.entry(agent).or_default();
        let (presses, scroll) = {
            let mut rows = Rows::new(
                ui,
                body,
                &id,
                self.scroll.get(&id).copied().unwrap_or_default(),
            );
            let presses = draw_rows(&mut rows, &agent_rows, &pictures, typed);
            (presses, rows.finish())
        };
        self.scroll.insert(id.clone(), scroll);
        self.apply(presses, Some(agent), frame, tools, profile);
        let event = frame::controls(ui, panel, &spec, profile, tools);
        Self::close_on(event, &id, tools, profile);
        panel
    }

    /// The ignore list of the profile: names whose speech the journal hides.
    fn ignore_window(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let spec = PanelSpec {
            id: IGNORE_ID,
            title: WORDS_IGNORE,
            default: layout::first_place(rect, IGNORE_SPOT, WINDOW_SIZE),
            min_size: Some(MIN_SIZE),
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, WORDS_IGNORE);
        let agent_rows = ignore_rows(&profile.ignore.names, frame);
        let (presses, scroll) = {
            let mut rows = Rows::new(ui, body, IGNORE_ID, self.scroll_of(IGNORE_ID));
            let presses = draw_rows(&mut rows, &agent_rows, &[], &mut self.ignore_name);
            (presses, rows.finish())
        };
        self.scroll.insert(IGNORE_ID.to_string(), scroll);
        self.apply(presses, None, frame, tools, profile);
        let event = frame::controls(ui, panel, &spec, profile, tools);
        Self::close_on(event, IGNORE_ID, tools, profile);
        panel
    }
}
