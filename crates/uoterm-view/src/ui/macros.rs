//! The macro editor of the Modern style, as both windows run it. A macro
//! is a script of the scripts folder. The human picks one from the list,
//! changes its lines, runs it, saves it, or puts it on the hotbar. He can
//! also record what he does as a new macro.
//!
//! With a TypeSafe key, one more field takes plain words, such as "heal
//! myself with a bandage". Jev picks the hotkey that does it, and the
//! script lines of that hotkey go to the end of the macro. Jev writes no
//! script: it only picks from the hotkeys the session has.

use super::theme::{ALARM, GOAL, TEXT, TEXT_FAINT, WAITING};
use crate::act::{Act, Answer, Ask};
use crate::actions::resolve::play_line;
use crate::asks::HINT_WISH_OFF;
use crate::geom::{Rgba, Vector};

pub const MACROS_SIZE: Vector = Vector::new(820.0, 520.0);
pub const MACROS_LIST_WIDTH: f32 = 220.0;
pub const MACROS_LIST_ROW: f32 = 28.0;
pub const MACROS_TITLE_ROW: f32 = 34.0;
pub const MACROS_FIELD_ROW: f32 = 30.0;
pub const MACROS_GAP: f32 = 10.0;
/// The room of the Add button at the right of the field for plain words.
pub const MACROS_ADD_WIDTH: f32 = 84.0;
/// How often the editor asks for the state of the running script.
pub const STATUS_EVERY: f64 = 1.0;
const NOTE_SECONDS: f64 = 6.0;

pub const WORDS_MACROS: &str = "Macros";
const WORDS_NEW: &str = "New";
const WORDS_SAVE: &str = "Save";
const WORDS_RUN: &str = "Run";
const WORDS_LOOP: &str = "Loop";
const WORDS_STOP: &str = "Stop";
const WORDS_RECORD: &str = "Record";
const WORDS_STOP_RECORDING: &str = "Stop recording";
const WORDS_PIN: &str = "Pin";
pub const WORDS_ADD_LINE: &str = "Add line";
const WORDS_CLOSE: &str = "Close";
pub const HINT_NAME: &str = "Macro name";
pub const HINT_LINES: &str = "One command on each line. See docs/SCRIPTS.md.";
const HINT_WISH: &str = "Say the next step in plain words, for example: heal myself with a bandage";
const NOTE_NEEDS_NAME: &str = "Give the macro a name first.";
const NOTE_BAR_FULL: &str = "The hotbar is full. Right-click a slot to clear it.";
const NOTE_PINNED: &str = "The macro is on the hotbar.";
const NOTE_ASKING: &str = "Jev looks for the hotkey...";

/// A button of the editor, left to right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacroButton {
    New,
    Save,
    Run,
    Loop,
    Stop,
    /// Record, or Stop recording while it records.
    Record,
    Pin,
    Close,
}

impl MacroButton {
    pub const ALL: [Self; 8] = [
        Self::New,
        Self::Save,
        Self::Run,
        Self::Loop,
        Self::Stop,
        Self::Record,
        Self::Pin,
        Self::Close,
    ];

    /// The buttons that act on the character need control.
    fn needs_control(self) -> bool {
        !matches!(self, Self::New | Self::Close)
    }
}

/// One button as it shows: its words, its color, and whether a press does
/// anything now.
#[derive(Clone, Debug, PartialEq)]
pub struct ShownButton {
    pub button: MacroButton,
    pub words: &'static str,
    pub color: Rgba,
    pub enabled: bool,
}

/// What a press asks of the window: acts for the character, asks of the
/// session, and the script line to pin on the hotbar.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MacroPress {
    pub acts: Vec<Act>,
    pub asks: Vec<Ask>,
    pub pin: Option<String>,
}

/// The editor: the macros of the folder, the one being edited, the plain
/// words for Jev, and the words of the last thing it did.
#[derive(Clone, Debug, PartialEq)]
pub struct MacroEditorPanel {
    pub open: bool,
    pub names: Vec<String>,
    pub name: String,
    pub lines: String,
    pub wish: String,
    recording: bool,
    pub status: String,
    last_status_ask: f64,
    /// The list of macros was asked for since the editor opened.
    list_asked: bool,
    /// Words for the human about the last thing the editor did, whether
    /// they tell of a failure, and when.
    note: Option<(String, bool, f64)>,
}

/// The macro with the new lines at its end, on lines of their own.
pub fn with_lines(macro_lines: &str, new_lines: &str) -> String {
    let kept = macro_lines.trim_end();
    if kept.is_empty() {
        format!("{new_lines}\n")
    } else {
        format!("{kept}\n{new_lines}\n")
    }
}

/// The color of the note of the editor.
pub fn note_color(failed: bool) -> Rgba {
    if failed {
        ALARM
    } else {
        WAITING
    }
}

/// The hint of the field for plain words, by whether Jev can answer.
pub fn wish_hint(orders_on: bool) -> &'static str {
    if orders_on {
        HINT_WISH
    } else {
        HINT_WISH_OFF
    }
}

/// The color of Add line, by whether Jev can answer.
pub fn add_line_color(orders_on: bool) -> Rgba {
    if orders_on {
        GOAL
    } else {
        TEXT_FAINT
    }
}

impl Default for MacroEditorPanel {
    fn default() -> Self {
        Self::starting(false)
    }
}

impl MacroEditorPanel {
    /// The editor as the window starts. An open one asks for the list of
    /// macros with its first frame.
    pub fn starting(open: bool) -> Self {
        Self {
            open,
            names: Vec::new(),
            name: String::new(),
            lines: String::new(),
            wish: String::new(),
            recording: false,
            status: String::new(),
            last_status_ask: f64::NEG_INFINITY,
            list_asked: false,
            note: None,
        }
    }

    /// Opens or closes the editor. When it opens it asks for the list
    /// again.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.list_asked = false;
    }

    fn say(&mut self, words: &str, failed: bool, time: f64) {
        self.note = Some((words.to_string(), failed, time));
    }

    /// The words of the last thing the editor did, and whether they tell
    /// of a failure, while they show.
    pub fn note(&self, time: f64) -> Option<(&str, bool)> {
        self.note
            .as_ref()
            .filter(|(_, _, since)| time - since <= NOTE_SECONDS)
            .map(|(words, failed, _)| (words.as_str(), *failed))
    }

    /// Takes the answers of the session and of Jev.
    pub fn take_answers(&mut self, answers: Vec<Answer>, time: f64) {
        for answer in answers {
            match answer {
                Answer::Scripts(names) => self.names = names,
                Answer::ScriptText { name, text } => {
                    self.name = name;
                    self.lines = text;
                }
                Answer::ScriptStatus(status) => self.status = status,
                Answer::Lines(Ok(lines)) => {
                    self.lines = with_lines(&self.lines, &lines);
                    self.wish.clear();
                    self.note = None;
                }
                Answer::Lines(Err(words)) => self.say(&words, true, time),
                // The editor asks for no place and no pick.
                Answer::Place(_) | Answer::Picked(_) => {}
            }
        }
    }

    /// The asks of an open editor in one frame: the list of macros once it
    /// opened, and the state of the running script now and then.
    pub fn asks_due(&mut self, time: f64) -> Vec<Ask> {
        let mut asks = Vec::new();
        if !self.open {
            return asks;
        }
        if !self.list_asked {
            self.list_asked = true;
            asks.push(Ask::Scripts);
        }
        if time - self.last_status_ask >= STATUS_EVERY {
            self.last_status_ask = time;
            asks.push(Ask::ScriptStatus);
        }
        asks
    }

    /// The buttons as they show, with or without control.
    pub fn buttons(&self, live: bool) -> Vec<ShownButton> {
        MacroButton::ALL
            .into_iter()
            .map(|button| {
                let words = match button {
                    MacroButton::New => WORDS_NEW,
                    MacroButton::Save => WORDS_SAVE,
                    MacroButton::Run => WORDS_RUN,
                    MacroButton::Loop => WORDS_LOOP,
                    MacroButton::Stop => WORDS_STOP,
                    MacroButton::Record if self.recording => WORDS_STOP_RECORDING,
                    MacroButton::Record => WORDS_RECORD,
                    MacroButton::Pin => WORDS_PIN,
                    MacroButton::Close => WORDS_CLOSE,
                };
                let enabled = live || !button.needs_control();
                let color = match (enabled, button) {
                    (false, _) => TEXT_FAINT,
                    (true, MacroButton::Run | MacroButton::Loop | MacroButton::Save) => GOAL,
                    (true, MacroButton::Stop) => ALARM,
                    (true, MacroButton::Record) if self.recording => ALARM,
                    (true, _) => TEXT,
                };
                ShownButton {
                    button,
                    words,
                    color,
                    enabled,
                }
            })
            .collect()
    }

    /// Does what a press of `button` does. A button that acts on the
    /// character does nothing without control; one that needs a name asks
    /// for it first.
    pub fn press(&mut self, button: MacroButton, live: bool, time: f64) -> MacroPress {
        let mut out = MacroPress::default();
        if button.needs_control() && !live {
            return out;
        }
        let name = self.name.trim().to_string();
        match button {
            MacroButton::New => {
                self.name.clear();
                self.lines.clear();
            }
            MacroButton::Close => self.open = false,
            MacroButton::Run | MacroButton::Loop => out.acts.push(Act::ScriptRun {
                text: self.lines.clone(),
                looping: button == MacroButton::Loop,
            }),
            MacroButton::Stop => out.acts.push(Act::ScriptStop),
            _ if name.is_empty() => self.say(NOTE_NEEDS_NAME, true, time),
            MacroButton::Save => {
                out.acts.push(Act::ScriptSave {
                    name,
                    text: self.lines.clone(),
                });
                out.asks.push(Ask::Scripts);
            }
            MacroButton::Record if !self.recording => {
                self.recording = true;
                out.acts.push(Act::RecordStart(name));
            }
            MacroButton::Record => {
                self.recording = false;
                out.acts.push(Act::RecordStop);
                out.asks.push(Ask::Scripts);
                out.asks.push(Ask::ScriptText(name));
            }
            MacroButton::Pin => out.pin = Some(play_line(&name)),
        }
        out
    }

    /// Tells whether the macro went on the hotbar.
    pub fn pinned(&mut self, pinned: bool, time: f64) {
        let words = if pinned { NOTE_PINNED } else { NOTE_BAR_FULL };
        self.say(words, !pinned, time);
    }

    /// The ask for the plain words, when Jev can answer and there are
    /// words.
    pub fn wish_ask(&mut self, orders_on: bool, time: f64) -> Option<Ask> {
        let wish = self.wish.trim().to_string();
        if !orders_on || wish.is_empty() {
            return None;
        }
        self.say(NOTE_ASKING, false, time);
        Some(Ask::LinesFor(wish))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_lines_go_to_the_end_of_the_macro_on_lines_of_their_own() {
        assert_eq!(with_lines("", "bandageself"), "bandageself\n");
        assert_eq!(
            with_lines("msg 'hi'\n\n", "cast 'Heal'\nwaitfortarget 3000"),
            "msg 'hi'\ncast 'Heal'\nwaitfortarget 3000\n"
        );
    }

    #[test]
    fn answers_fill_the_editor_and_a_fault_of_jev_shows_as_a_note() {
        let mut editor = MacroEditorPanel::starting(false);
        editor.take_answers(
            vec![
                Answer::Scripts(vec!["heal".into(), "mine".into()]),
                Answer::ScriptText {
                    name: "heal".into(),
                    text: "bandageself\n".into(),
                },
                Answer::ScriptStatus("running".into()),
            ],
            0.0,
        );
        assert_eq!((editor.names.len(), editor.name.as_str()), (2, "heal"));
        assert_eq!(editor.status, "running");
        editor.wish = "drink a cure potion".into();
        editor.take_answers(vec![Answer::Lines(Ok("potion 'cure'".into()))], 1.0);
        assert_eq!(editor.lines, "bandageself\npotion 'cure'\n");
        assert!(editor.wish.is_empty());
        editor.take_answers(vec![Answer::Lines(Err("No hotkey does that.".into()))], 2.0);
        assert!(matches!(editor.note(2.0), Some((words, true)) if words.contains("No hotkey")));
    }

    #[test]
    fn presses_act_only_with_control_and_record_starts_then_stops() {
        let mut editor = MacroEditorPanel::starting(true);
        assert_eq!(
            editor.press(MacroButton::Run, false, 0.0),
            MacroPress::default()
        );
        let save = editor.press(MacroButton::Save, true, 0.0);
        assert!(save.acts.is_empty(), "a macro needs a name");
        assert!(editor.note(0.0).is_some_and(|(_, failed)| failed));
        editor.name = "heal".into();
        assert_eq!(
            editor.press(MacroButton::Record, true, 0.0).acts,
            vec![Act::RecordStart("heal".into())]
        );
        assert_eq!(editor.buttons(true)[5].words, WORDS_STOP_RECORDING);
        let stop = editor.press(MacroButton::Record, true, 0.0);
        assert_eq!(stop.acts, vec![Act::RecordStop]);
        assert_eq!(
            stop.asks,
            vec![Ask::Scripts, Ask::ScriptText("heal".into())]
        );
        assert_eq!(
            editor.press(MacroButton::Pin, true, 0.0).pin,
            Some(play_line("heal"))
        );
        assert!(!editor.buttons(false)[1].enabled);
        editor.press(MacroButton::Close, false, 0.0);
        assert!(!editor.open);
    }

    #[test]
    fn an_open_editor_asks_its_list_once_and_its_status_each_second() {
        let mut editor = MacroEditorPanel::starting(true);
        assert_eq!(editor.asks_due(0.0), vec![Ask::Scripts, Ask::ScriptStatus]);
        assert!(editor.asks_due(0.5).is_empty());
        assert_eq!(editor.asks_due(1.0), vec![Ask::ScriptStatus]);
        editor.toggle();
        assert!(editor.asks_due(5.0).is_empty());
    }
}
