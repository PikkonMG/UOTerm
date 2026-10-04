//! The agent windows of the Modern style, as both windows run them: one
//! for each agent the Agents page offers, one that lists them, and the
//! ignore list. Each window reads the agent's settings from the session
//! and changes them with the agent tools; the session keeps them. The
//! ignore list is the profile's own, for speech the journal hides. The
//! rows of each window are `ui::lists`'; here is which windows show, the
//! rows of each, and what a press on one does.

use super::launch::{AGENTS_ID, IGNORE_ID};
use super::layout::{first_place, Spot};
use super::lists::{
    agent_chooser_rows, agent_list_rows, agent_settings_rows, field_presses, ignore_rows,
    AgentPress, AgentRow, FieldUse, WORDS_IGNORE,
};
use super::theme::TEXT_FAINT;
use crate::act::Act;
use crate::frame::{WatchFrame, WatchPackItem};
use crate::geom::{Area, Vector};
use crate::model::agents::{AgentPanel, AgentsView};
use crate::model::places;
use crate::model::reads::ReadKey;
use crate::settings::Profile;
use serde_json::json;
use std::collections::HashMap;
use uoterm_world::tool_names::TOOL_AGENTS;

/// The agents are read this often while a window shows, in seconds.
pub const AGENTS_MAX_AGE: f64 = 2.0;
pub const AGENT_WINDOW_SIZE: Vector = Vector::new(320.0, 420.0);
pub const AGENT_WINDOW_MIN: Vector = Vector::new(260.0, 160.0);
/// The list opens at the top of the right column and the ignore list a
/// step from it; each agent window opens in the middle, a step from the
/// one before.
const AGENTS_SPOT: Spot = Spot::RightColumn(0);
const IGNORE_SPOT: Spot = Spot::RightColumn(1);
pub const WORDS_AGENTS: &str = "Agents";
const WORDS_NOT_READ: &str = "The session has not answered yet.";

/// One window of the agents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AgentWindow {
    /// The list of the agent windows.
    Chooser,
    Agent(AgentPanel),
    Ignore,
}

impl AgentWindow {
    /// The id of the window's place and of its open mark.
    pub fn place_id(self) -> String {
        match self {
            Self::Chooser => AGENTS_ID.to_string(),
            Self::Agent(agent) => agent.place_id(),
            Self::Ignore => IGNORE_ID.to_string(),
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Chooser => WORDS_AGENTS,
            Self::Agent(agent) => agent.title(),
            Self::Ignore => WORDS_IGNORE,
        }
    }

    /// The window of a place id.
    pub fn of(id: &str) -> Option<Self> {
        match id {
            AGENTS_ID => Some(Self::Chooser),
            IGNORE_ID => Some(Self::Ignore),
            _ => AgentPanel::ALL
                .into_iter()
                .find(|agent| agent.place_id() == id)
                .map(Self::Agent),
        }
    }

    /// Where the window first stands in `window`: `at` counts the agent
    /// windows before it.
    pub fn first_place(self, window: Area, at: usize) -> Area {
        let spot = match self {
            Self::Chooser => AGENTS_SPOT,
            Self::Agent(_) => Spot::Middle(at),
            Self::Ignore => IGNORE_SPOT,
        };
        first_place(window, spot, AGENT_WINDOW_SIZE)
    }
}

/// The read of the agents of the session.
pub fn agents_key() -> ReadKey {
    ReadKey::new(TOOL_AGENTS, &json!({}))
}

/// The windows that show, in the order they draw: the ignore list, the
/// list of the agent windows, then each agent window the page offers.
pub fn open_windows(profile: &Profile) -> Vec<AgentWindow> {
    let mut open = Vec::new();
    for window in [AgentWindow::Ignore, AgentWindow::Chooser] {
        if places::is_open(profile, &window.place_id()) {
            open.push(window);
        }
    }
    open.extend(
        AgentPanel::ALL
            .into_iter()
            .filter(|agent| {
                agent.offered(&profile.agents) && places::is_open(profile, &agent.place_id())
            })
            .map(AgentWindow::Agent),
    );
    open
}

/// What the agent windows keep apart from the profile: the words typed in
/// each field, and the job list each window shows.
#[derive(Default)]
pub struct AgentsState {
    typed: HashMap<AgentWindow, String>,
    chosen_list: HashMap<AgentPanel, String>,
}

impl AgentsState {
    /// The words typed in the field of a window.
    pub fn typed(&self, window: AgentWindow) -> &str {
        self.typed.get(&window).map_or("", String::as_str)
    }

    pub fn set_typed(&mut self, window: AgentWindow, words: &str) {
        self.typed.insert(window, words.to_string());
    }

    /// The rows of a window. `view` is the read of the agents, and
    /// `failure` its words when it failed; `bag` the items of
    /// `agent_bag_items`.
    pub fn rows(
        &self,
        window: AgentWindow,
        view: Option<&AgentsView>,
        failure: Option<&str>,
        frame: &WatchFrame,
        profile: &Profile,
        bag: &[WatchPackItem],
    ) -> Vec<AgentRow> {
        match (window, view) {
            (AgentWindow::Ignore, _) => ignore_rows(&profile.ignore.names, frame),
            (AgentWindow::Chooser, view) => agent_chooser_rows(view, profile),
            (AgentWindow::Agent(_), None) => vec![AgentRow::Words {
                words: failure.unwrap_or(WORDS_NOT_READ).to_string(),
                color: TEXT_FAINT,
            }],
            (AgentWindow::Agent(agent), Some(view)) => {
                let chosen = self.chosen_list.get(&agent).map(String::as_str);
                let mut rows = agent_settings_rows(agent, view, frame, bag);
                rows.extend(agent_list_rows(agent, view, frame, chosen, bag));
                rows
            }
        }
    }

    /// The presses of the field of a window, once its button or Enter was
    /// pressed. The field is cleared when they do something.
    pub fn submit(&mut self, window: AgentWindow, field_use: &FieldUse) -> Vec<AgentPress> {
        let made = field_presses(field_use, self.typed(window));
        if !made.is_empty() {
            self.typed.remove(&window);
        }
        made
    }

    /// Does what the presses of one window say to the profile and to this
    /// state. Gives the acts for the character, which go only while the
    /// human has control, and true when the profile changed.
    pub fn apply(
        &mut self,
        presses: Vec<AgentPress>,
        window: AgentWindow,
        profile: &mut Profile,
    ) -> (Vec<Act>, bool) {
        let mut acts = Vec::new();
        let mut changed = false;
        for press in presses {
            match press {
                AgentPress::Acts(more) => acts.extend(more),
                AgentPress::Open(id, open) => {
                    places::set_open(profile, &id, open);
                    changed = true;
                }
                AgentPress::ChooseList(name) => {
                    if let AgentWindow::Agent(agent) = window {
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
        (acts, changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presses_open_windows_and_change_the_ignore_list() {
        let mut state = AgentsState::default();
        let mut profile = Profile::default();
        let loot = AgentPanel::Loot.place_id();
        let (acts, changed) = state.apply(
            vec![AgentPress::Open(loot.clone(), true)],
            AgentWindow::Chooser,
            &mut profile,
        );
        assert!(acts.is_empty() && changed);
        assert!(places::is_open(&profile, &loot));
        state.set_typed(AgentWindow::Ignore, " Bob ");
        let made = state.submit(AgentWindow::Ignore, &FieldUse::IgnoreName);
        assert_eq!(state.typed(AgentWindow::Ignore), "", "a used field clears");
        let (_, changed) = state.apply(made, AgentWindow::Ignore, &mut profile);
        assert!(changed && profile.ignore.names == ["Bob"]);
        assert_eq!(
            AgentWindow::of(&loot),
            Some(AgentWindow::Agent(AgentPanel::Loot))
        );
        assert_eq!(AgentWindow::of(IGNORE_ID), Some(AgentWindow::Ignore));
    }
}
