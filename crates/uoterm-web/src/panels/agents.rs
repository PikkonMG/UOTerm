//! The agent windows of the Modern style: one for each agent the Agents
//! page offers, the list of them, and the ignore list. Each window reads
//! the agent's settings from the session and changes them with the agent
//! tools. Which windows show, their rows and what a press does are
//! `uoterm_view::ui::agents`' and `ui::lists`', as in the Rust window; the
//! acts go only while the human has control.

use super::{Colored, FrameSpec, Framed, PANEL_AGENTS, PANEL_IGNORE};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::frame::{WatchFrame, WatchPackItem};
use uoterm_view::model::agents::AgentsView;
use uoterm_view::model::places;
use uoterm_view::ui::agents::{
    agents_key, open_windows, AgentWindow, AGENTS_MAX_AGE, AGENT_WINDOW_MIN,
};
use uoterm_view::ui::lists::{agent_bag_items, AgentButton, AgentPress, AgentRow};
use uoterm_view::ui::theme::css_color;
use uoterm_world::tool_names::TOOL_AGENTS;

/// One window of the agents.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AgentWindowData {
    pub rows: Vec<AgentRowData>,
}

/// One row of an agent window, as `AgentRow` lays it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentRowData {
    Words {
        words: Colored,
    },
    /// Words on the left, and buttons of `width` on the right.
    Labeled {
        words: String,
        buttons: Vec<Colored>,
        width: f32,
    },
    Buttons {
        buttons: Vec<Colored>,
    },
    /// The items of the backpack and its open bags.
    Pictures {
        pictures: Vec<BagPicture>,
    },
    /// A field of words and its button.
    Field {
        hint: &'static str,
        button: &'static str,
        width: f32,
        words: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BagPicture {
    pub picture: Option<String>,
    pub name: String,
}

/// A place in a row: the row, and the button or the picture in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct RowPlace {
    row: usize,
    at: usize,
}

/// `{"press": {row, at}}` presses a button, `{"picture": {row, at}}` a
/// picture, `{"typing": words}` keeps the words of the field, and
/// `{"submit": row}` is Enter or the button of the field.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AgentAction {
    Press(RowPlace),
    Picture(RowPlace),
    Typing(String),
    Submit(usize),
}

/// The colored words of the buttons of a row.
fn buttons_data(buttons: &[AgentButton]) -> Vec<Colored> {
    buttons
        .iter()
        .map(|button| Colored {
            words: button.words.clone(),
            color: css_color(button.color),
        })
        .collect()
}

/// The window of a panel name: the list, the ignore list, or an agent's
/// window by its place id.
pub(super) fn agent_window(panel: &str) -> Option<AgentWindow> {
    match panel {
        PANEL_AGENTS => Some(AgentWindow::Chooser),
        PANEL_IGNORE => Some(AgentWindow::Ignore),
        _ => AgentWindow::of(panel).filter(|window| matches!(window, AgentWindow::Agent(_))),
    }
}

/// The name of a window in its `Panel` events.
fn panel_name(window: AgentWindow) -> String {
    match window {
        AgentWindow::Chooser => PANEL_AGENTS.to_string(),
        AgentWindow::Ignore => PANEL_IGNORE.to_string(),
        AgentWindow::Agent(_) => window.place_id(),
    }
}

impl WebView {
    /// The read of the agents, asked for while a window of them shows.
    fn agents_view(&mut self) -> Option<AgentsView> {
        self.hand
            .reads()
            .want(agents_key(), AGENTS_MAX_AGE)
            .map(AgentsView::new)
    }

    /// The rows of a window now, and the items of the bags they show.
    fn agent_rows(
        &mut self,
        window: AgentWindow,
        frame: &WatchFrame,
    ) -> (Vec<AgentRow>, Vec<WatchPackItem>) {
        let view = (window != AgentWindow::Ignore)
            .then(|| self.agents_view())
            .flatten();
        let failure = self.hand.reads().failure(&agents_key()).map(str::to_string);
        let bag = agent_bag_items(frame);
        let rows = self.panels.agents.rows(
            window,
            view.as_ref(),
            failure.as_deref(),
            frame,
            &self.profile,
            &bag,
        );
        (rows, bag)
    }

    pub(super) fn agent_spec(&self, panel: &str) -> Option<FrameSpec> {
        let window = agent_window(panel)?;
        let open = open_windows(&self.profile);
        let shown = open.iter().position(|open| *open == window)?;
        let before = open[..shown]
            .iter()
            .filter(|open| matches!(open, AgentWindow::Agent(_)))
            .count();
        let default = window.first_place(self.panel_room(), before);
        Some(
            FrameSpec::fixed(&window.place_id(), window.title(), default)
                .closable()
                .sized(AGENT_WINDOW_MIN),
        )
    }

    pub(super) fn agents_data(&mut self, frame: &WatchFrame) -> Vec<Framed<AgentWindowData>> {
        let mut windows = Vec::new();
        for window in open_windows(&self.profile) {
            let panel = panel_name(window);
            let Some(spec) = self.agent_spec(&panel) else {
                continue;
            };
            let (rows, bag) = self.agent_rows(window, frame);
            let typed = self.panels.agents.typed(window).to_string();
            let rows = rows
                .into_iter()
                .map(|row| match row {
                    AgentRow::Words { words, color } => AgentRowData::Words {
                        words: Colored {
                            words,
                            color: css_color(color),
                        },
                    },
                    AgentRow::Labeled {
                        words,
                        buttons,
                        width,
                        ..
                    } => AgentRowData::Labeled {
                        words,
                        buttons: buttons_data(&buttons),
                        width,
                    },
                    AgentRow::Buttons { buttons, .. } => AgentRowData::Buttons {
                        buttons: buttons_data(&buttons),
                    },
                    AgentRow::Pictures { .. } => AgentRowData::Pictures {
                        pictures: bag
                            .iter()
                            .map(|item| BagPicture {
                                picture: self.item_picture(item),
                                name: item.name.clone(),
                            })
                            .collect(),
                    },
                    AgentRow::Field {
                        hint,
                        button,
                        width,
                        ..
                    } => AgentRowData::Field {
                        hint,
                        button,
                        width,
                        words: typed.clone(),
                    },
                })
                .collect();
            windows.push(self.framed(&panel, &spec, AgentWindowData { rows }));
        }
        windows
    }

    pub(super) fn agent_action(&mut self, panel: &str, action: Value) {
        let (Some(window), Some(frame)) = (agent_window(panel), self.frame.clone()) else {
            return;
        };
        let Ok(action) = serde_json::from_value::<AgentAction>(action) else {
            return;
        };
        let (rows, _) = self.agent_rows(window, &frame);
        let presses: Vec<AgentPress> = match action {
            AgentAction::Typing(words) => {
                self.panels.agents.set_typed(window, &words);
                return;
            }
            AgentAction::Press(RowPlace { row, at }) => match rows.get(row) {
                Some(AgentRow::Labeled { buttons, .. } | AgentRow::Buttons { buttons, .. }) => {
                    buttons
                        .get(at)
                        .map(|button| button.press.clone())
                        .into_iter()
                        .collect()
                }
                _ => Vec::new(),
            },
            AgentAction::Picture(RowPlace { row, at }) => match rows.get(row) {
                Some(AgentRow::Pictures { presses, .. }) => {
                    presses.get(at).cloned().into_iter().collect()
                }
                _ => Vec::new(),
            },
            AgentAction::Submit(row) => match rows.get(row) {
                Some(AgentRow::Field { field_use, .. }) => {
                    self.panels.agents.submit(window, field_use)
                }
                _ => Vec::new(),
            },
        };
        let (acts, changed) = self.panels.agents.apply(presses, window, &mut self.profile);
        if changed {
            self.keep_profile();
        }
        if acts.is_empty() || !frame.human_control {
            return;
        }
        for act in acts {
            self.hand.act(act);
        }
        self.hand.reads().refresh(TOOL_AGENTS);
    }

    /// The close mark of an agent window.
    pub(super) fn close_agent_window(&mut self, panel: &str) {
        if let Some(window) = agent_window(panel) {
            places::set_open(&mut self.profile, &window.place_id(), false);
            self.keep_profile();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, saved_profiles, view_with};
    use super::*;
    use serde_json::json;
    use uoterm_view::model::agents::AgentPanel;
    use uoterm_view::ui::launch::{AGENTS_ID, IGNORE_ID};

    #[test]
    fn the_list_opens_an_agent_window_as_the_window_does() {
        let mut view = view_with("human_control", json!(true), true);
        view.profile.agents.loot = true;
        places::set_open(&mut view.profile, AGENTS_ID, true);
        let frame = view.frame.clone().unwrap();
        let (rows, _) = view.agent_rows(AgentWindow::Chooser, &frame);
        let (row, at) = rows
            .iter()
            .enumerate()
            .find_map(|(row, agent_row)| match agent_row {
                AgentRow::Labeled { buttons, .. } | AgentRow::Buttons { buttons, .. } => buttons
                    .iter()
                    .position(|button| {
                        button.press == AgentPress::Open(AgentPanel::Loot.place_id(), true)
                    })
                    .map(|at| (row, at)),
                _ => None,
            })
            .expect("the list opens the loot window");
        let out = press(
            &mut view,
            PANEL_AGENTS,
            json!({ "press": { "row": row, "at": at } }),
        );
        assert!(places::is_open(
            &saved_profiles(&out)[0],
            &AgentPanel::Loot.place_id()
        ));
        let shown = view.panel_data(0.0).agents;
        assert!(shown
            .iter()
            .any(|window| window.frame.panel == AgentPanel::Loot.place_id()));
    }

    #[test]
    fn the_ignore_list_takes_a_name_from_its_field_and_clears_it() {
        let mut view = view_with("human_control", json!(false), false);
        places::set_open(&mut view.profile, IGNORE_ID, true);
        press(&mut view, PANEL_IGNORE, json!({ "typing": " Bob " }));
        let field_row = view.panel_data(0.0).agents[0]
            .body
            .rows
            .iter()
            .position(|row| matches!(row, AgentRowData::Field { .. }))
            .unwrap();
        let out = press(&mut view, PANEL_IGNORE, json!({ "submit": field_row }));
        assert_eq!(
            saved_profiles(&out)[0].ignore.names,
            vec!["Bob".to_string()]
        );
        assert!(out_acts(&out).is_empty());
        let data = view.panel_data(0.0);
        let cleared = data.agents[0]
            .body
            .rows
            .iter()
            .any(|row| matches!(row, AgentRowData::Field { words, .. } if words.is_empty()));
        assert!(cleared, "a used field clears");
    }

    /// The place of the first button of the loot window that acts on the
    /// character, with the session's agents read as empty.
    fn loot_switch(view: &mut WebView) -> (usize, usize) {
        view.profile.agents.loot = true;
        places::set_open(&mut view.profile, &AgentPanel::Loot.place_id(), true);
        view.hand.reads().arrived(agents_key(), Ok(json!({})), 0.0);
        let frame = view.frame.clone().unwrap();
        let (rows, _) = view.agent_rows(AgentWindow::Agent(AgentPanel::Loot), &frame);
        rows.iter()
            .enumerate()
            .find_map(|(row, agent_row)| match agent_row {
                AgentRow::Labeled { buttons, .. } | AgentRow::Buttons { buttons, .. } => buttons
                    .iter()
                    .position(|button| matches!(button.press, AgentPress::Acts(_)))
                    .map(|at| (row, at)),
                _ => None,
            })
            .expect("the loot window switches its agent")
    }

    #[test]
    fn an_agent_switch_acts_only_with_control() {
        for control in [false, true] {
            let mut view = view_with("human_control", json!(control), control);
            let (row, at) = loot_switch(&mut view);
            let panel = AgentPanel::Loot.place_id();
            let out = press(
                &mut view,
                &panel,
                json!({ "press": { "row": row, "at": at } }),
            );
            assert_eq!(!out_acts(&out).is_empty(), control, "control {control}");
        }
    }

    #[test]
    fn a_window_closes_by_its_mark() {
        let mut view = view_with("human_control", json!(true), true);
        places::set_open(&mut view.profile, IGNORE_ID, true);
        press(&mut view, PANEL_IGNORE, json!({ "close": true }));
        assert!(view.panel_data(0.0).agents.is_empty());
    }
}
