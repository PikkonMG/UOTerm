//! The macro editor of the Modern style. A macro is a script of the
//! scripts folder: the editor lists them, reads one, saves it, runs it,
//! records a new one, and pins one on the hotbar; plain words go to Jev,
//! who gives the script lines of the hotkey they mean. The script tools go
//! on the live link and the plain words to `/jev/lines`, as the asks of
//! the Rust window go; every rule is `uoterm_view::ui::macros`'.

use super::sheet::Choice;
use super::{Colored, NoteData, Place};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::{Ask, Asker};
use uoterm_view::geom::Area;
use uoterm_view::ui::macros::{
    add_line_color, wish_hint, MacroButton, MacroEditorPanel, HINT_LINES, HINT_NAME, MACROS_SIZE,
    WORDS_ADD_LINE, WORDS_MACROS,
};
use uoterm_view::ui::theme::css_color;

/// The page asks Jev and shows his answer or the server's words when he
/// cannot answer: it does not know beforehand whether a key is set.
const ORDERS_ON: bool = true;

/// The macro editor.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MacrosData {
    /// Where the editor stands: the middle of the room, over the panels.
    pub place: Place,
    pub title: &'static str,
    /// The state of the running or last script.
    pub status: String,
    /// The macros of the folder; the one being edited is chosen.
    pub names: Vec<Choice>,
    pub name: String,
    pub lines: String,
    pub wish: String,
    pub name_hint: &'static str,
    pub lines_hint: &'static str,
    pub wish_hint: &'static str,
    pub add_line: Colored,
    pub buttons: Vec<MacroButtonData>,
    pub note: Option<NoteData>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MacroButtonData {
    pub words: &'static str,
    pub color: String,
    pub enabled: bool,
}

/// `{"pick": i}` reads a macro of the list; `{"name": words}`,
/// `{"lines": words}` and `{"wish": words}` keep the fields;
/// `{"add_line": true}` asks Jev; `{"button": i}` presses a button.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MacrosAction {
    Pick(usize),
    Name(String),
    Lines(String),
    Wish(String),
    AddLine(bool),
    Button(usize),
}

impl WebView {
    /// The editor in one frame: it takes the answers that came, and asks
    /// for its list and the state of the script while it is open.
    pub(crate) fn follow_macros(&mut self, time: f64) {
        let answers = self.hand.new_answers(Asker::Macros);
        let editor = &mut self.panels.macros;
        editor.take_answers(answers, time);
        for ask in editor.asks_due(time) {
            self.hand.ask(Asker::Macros, ask);
        }
    }

    pub(crate) fn macros_open(&self) -> bool {
        self.panels.macros.open
    }

    pub(crate) fn toggle_macros(&mut self) {
        self.panels.macros.toggle();
    }

    pub(super) fn macros_data(&self, live: bool, time: f64) -> Option<MacrosData> {
        let editor: &MacroEditorPanel = &self.panels.macros;
        if !editor.open {
            return None;
        }
        let place = Area::from_center_size(self.panel_room().center(), MACROS_SIZE);
        Some(MacrosData {
            place: Place::from(place),
            title: WORDS_MACROS,
            status: editor.status.clone(),
            names: editor
                .names
                .iter()
                .map(|name| Choice {
                    words: name.clone(),
                    chosen: *name == editor.name,
                })
                .collect(),
            name: editor.name.clone(),
            lines: editor.lines.clone(),
            wish: editor.wish.clone(),
            name_hint: HINT_NAME,
            lines_hint: HINT_LINES,
            wish_hint: wish_hint(ORDERS_ON),
            add_line: Colored {
                words: WORDS_ADD_LINE.to_string(),
                color: css_color(add_line_color(ORDERS_ON)),
            },
            buttons: editor
                .buttons(live)
                .into_iter()
                .map(|shown| MacroButtonData {
                    words: shown.words,
                    color: css_color(shown.color),
                    enabled: shown.enabled,
                })
                .collect(),
            note: editor.note(time).map(|(words, failed)| NoteData {
                words: words.to_string(),
                failed,
            }),
        })
    }

    pub(super) fn macros_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let Ok(action) = serde_json::from_value::<MacrosAction>(action) else {
            return;
        };
        let time = self.hand.time();
        let editor = &mut self.panels.macros;
        match action {
            MacrosAction::Pick(at) => {
                if let Some(name) = editor.names.get(at).cloned() {
                    self.hand.ask(Asker::Macros, Ask::ScriptText(name));
                }
            }
            MacrosAction::Name(words) => editor.name = words,
            MacrosAction::Lines(words) => editor.lines = words,
            MacrosAction::Wish(words) => editor.wish = words,
            MacrosAction::AddLine(_) => {
                if let Some(ask) = editor.wish_ask(ORDERS_ON, time) {
                    self.hand.ask(Asker::Macros, ask);
                }
            }
            MacrosAction::Button(at) => {
                let Some(button) = MacroButton::ALL.get(at).copied() else {
                    return;
                };
                let press = editor.press(button, frame.human_control, time);
                for act in press.acts {
                    self.hand.act(act);
                }
                for ask in press.asks {
                    self.hand.ask(Asker::Macros, ask);
                }
                if let Some(line) = press.pin {
                    let pinned = self.pin_command(&frame.name, &line);
                    self.panels.macros.pinned(pinned, time);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, view_with};
    use super::super::PANEL_MACROS;
    use super::*;
    use crate::out::OutCall;
    use serde_json::json;
    use uoterm_view::act::Act;
    use uoterm_view::actions::resolve::play_line;
    use uoterm_world::tool_names::TOOL_SCRIPT_READ;

    const RUN: usize = 2;
    const RECORD: usize = 5;
    const PIN: usize = 6;

    fn editor(control: bool) -> WebView {
        let mut view = view_with("human_control", json!(control), control);
        view.toggle_macros();
        view
    }

    #[test]
    fn an_open_editor_reads_its_list_and_a_pick_reads_the_macro_by_the_script_tools() {
        let mut view = editor(true);
        view.tick_native(0.2, crate::tests::VIEW, None);
        let out = view.take_out_native();
        let reads: Vec<&str> = out
            .iter()
            .filter_map(|call| match call {
                OutCall::Read { tool, .. } => Some(tool.as_str()),
                _ => None,
            })
            .collect();
        assert!(reads.len() >= 2, "the list and the status: {reads:?}");
        view.panels.macros.names = vec!["heal".into()];
        let out = press(&mut view, PANEL_MACROS, json!({ "pick": 0 }));
        assert!(out.iter().any(|call| matches!(
            call,
            OutCall::Read { tool, args, .. } if tool == TOOL_SCRIPT_READ && args["name"] == "heal"
        )));
    }

    #[test]
    fn run_and_record_make_the_acts_of_the_window_and_pin_puts_the_macro_on_the_bar() {
        let mut view = editor(true);
        press(&mut view, PANEL_MACROS, json!({ "lines": "say hi\n" }));
        let acts = out_acts(&press(&mut view, PANEL_MACROS, json!({ "button": RUN })));
        let run = Act::ScriptRun {
            text: "say hi\n".into(),
            looping: false,
        };
        assert_eq!(acts, vec![run.for_page()]);
        press(&mut view, PANEL_MACROS, json!({ "name": "greet" }));
        let acts = out_acts(&press(&mut view, PANEL_MACROS, json!({ "button": RECORD })));
        assert_eq!(acts, vec![Act::RecordStart("greet".into()).for_page()]);
        press(&mut view, PANEL_MACROS, json!({ "button": PIN }));
        let frame = view.frame.clone().unwrap();
        let slots = view.hotbar_data(&frame).body.slots;
        assert!(slots.iter().any(|slot| slot.tip == play_line("greet")));
    }

    #[test]
    fn the_editor_acts_not_without_control_and_asks_jev_for_plain_words() {
        let mut view = editor(false);
        assert!(out_acts(&press(&mut view, PANEL_MACROS, json!({ "button": RUN }))).is_empty());
        press(&mut view, PANEL_MACROS, json!({ "wish": "heal me" }));
        let out = press(&mut view, PANEL_MACROS, json!({ "add_line": true }));
        assert!(out.iter().any(|call| matches!(
            call,
            OutCall::Jev { route, body, .. } if route == crate::out::JEV_LINES && body["wish"] == "heal me"
        )));
    }
}
