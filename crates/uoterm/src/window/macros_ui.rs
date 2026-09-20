//! The macro editor. A macro is a script of the scripts folder. The human
//! picks one from the list, changes its lines, runs it, saves it, or puts
//! it on the hotbar. He can also record what he does as a new macro.
//!
//! With a TypeSafe key, one more field takes plain words, such as "heal
//! myself with a bandage". Jev picks the hotkey that does it, and the script
//! lines of that hotkey go to the end of the macro. Jev writes no script: it
//! only picks from the hotkeys the session has.

use super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::control::{Act, Answer, Ask};
use super::deck_ui::DeckUi;
use super::theme::{self, number_font, text_font, title_font};
use crate::view::WatchFrame;
use eframe::egui::{self, Align2, CornerRadius, Id, Key, Pos2, Rect, Sense, Vec2};

const PANEL_SIZE: Vec2 = Vec2::new(820.0, 520.0);
const LIST_WIDTH: f32 = 220.0;
const LIST_ROW: f32 = 28.0;
const TITLE_ROW: f32 = 34.0;
const FIELD_ROW: f32 = 30.0;
const GAP: f32 = 10.0;
/// How often the editor asks for the state of the running script.
const STATUS_EVERY: f64 = 1.0;
const NOTE_SECONDS: f64 = 6.0;
/// The room of the Add button at the right of the field for plain words.
const ADD_WIDTH: f32 = 84.0;

const WORDS_TITLE: &str = "Macros";
const WORDS_NEW: &str = "New";
const WORDS_SAVE: &str = "Save";
const WORDS_RUN: &str = "Run";
const WORDS_LOOP: &str = "Loop";
const WORDS_STOP: &str = "Stop";
const WORDS_RECORD: &str = "Record";
const WORDS_STOP_RECORDING: &str = "Stop recording";
const WORDS_PIN: &str = "Pin";
const WORDS_ADD: &str = "Add line";
const WORDS_CLOSE: &str = "Close";
const HINT_NAME: &str = "Macro name";
const HINT_LINES: &str = "One command on each line. See docs/SCRIPTS.md.";
const HINT_WISH: &str = "Say the next step in plain words, for example: heal myself with a bandage";
const HINT_WISH_OFF: &str = "Plain words need a TypeSafe key. Set TYPESAFE_API_KEY.";
const NOTE_NEEDS_NAME: &str = "Give the macro a name first.";
const NOTE_BAR_FULL: &str = "The hotbar is full. Right-click a slot to clear it.";
const NOTE_PINNED: &str = "The macro is on the hotbar.";
const NOTE_ASKING: &str = "Jev looks for the hotkey...";
/// The script command that runs a macro by its name.
const COMMAND_PLAY: &str = "playmacro";

pub struct MacrosUi {
    open: bool,
    names: Vec<String>,
    first_name: usize,
    name: String,
    lines: String,
    wish: String,
    recording: bool,
    status: String,
    last_status_ask: f64,
    /// The list of macros was asked for since the editor opened.
    list_asked: bool,
    /// Words for the human about the last thing the editor did, and when.
    note: Option<(String, bool, f64)>,
}

impl MacrosUi {
    /// The editor as the window starts. An open one asks for the list of
    /// macros with its first picture.
    pub fn starting(open: bool) -> Self {
        Self {
            open,
            names: Vec::new(),
            first_name: 0,
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
}

/// The macro with the new lines at its end, on lines of their own.
fn with_lines(macro_lines: &str, new_lines: &str) -> String {
    let kept = macro_lines.trim_end();
    if kept.is_empty() {
        format!("{new_lines}\n")
    } else {
        format!("{kept}\n{new_lines}\n")
    }
}

/// The script line that runs a macro by name. A name cannot hold a quote,
/// because the session refuses such a name when the macro is saved.
fn play_line(name: &str) -> String {
    format!("{COMMAND_PLAY} '{name}'")
}

impl MacrosUi {
    /// Opens or closes the editor. When it opens it asks for the list again.
    pub fn toggle(&mut self) {
        self.open = !self.open;
        self.list_asked = false;
    }

    fn say(&mut self, words: &str, failed: bool, time: f64) {
        self.note = Some((words.to_string(), failed, time));
    }

    fn take_answers(&mut self, answers: Vec<Answer>, time: f64) {
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
            }
        }
    }

    /// Draws the editor when it is open. Gives the place it covers.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        deck: &mut DeckUi,
    ) -> Option<Rect> {
        let time = tools.time;
        self.take_answers(tools.hand.new_answers(), time);
        if !self.open {
            return None;
        }
        if !self.list_asked {
            self.list_asked = true;
            tools.hand.ask(Ask::Scripts);
        }
        if time - self.last_status_ask >= STATUS_EVERY {
            self.last_status_ask = time;
            tools.hand.ask(Ask::ScriptStatus);
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(STATUS_EVERY));
        }
        let panel = Rect::from_center_size(rect.center(), PANEL_SIZE);
        // The editor is large and lies on other panels. A solid back keeps
        // their words from showing through its glass.
        ui.painter()
            .rect_filled(panel, CornerRadius::same(theme::PANEL_RADIUS), theme::VOID);
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            WORDS_TITLE,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        ui.painter().text(
            inner.right_top() + Vec2::new(0.0, theme::ROW_GAP),
            Align2::RIGHT_TOP,
            &self.status,
            number_font(theme::SIZE_SMALL),
            theme::TEXT_DIM,
        );
        let live = frame.human_control;
        let body = Rect::from_min_max(inner.left_top() + Vec2::new(0.0, TITLE_ROW), inner.max);
        self.list(ui, panel, body, tools);
        let right = Rect::from_min_max(
            Pos2::new(body.left() + LIST_WIDTH + GAP, body.top()),
            body.max,
        );
        let name_row = Rect::from_min_size(right.min, Vec2::new(right.width(), FIELD_ROW));
        let buttons_top = right.bottom() - FIELD_ROW;
        let wish_row = Rect::from_min_size(
            Pos2::new(right.left(), buttons_top - GAP - FIELD_ROW),
            Vec2::new(right.width(), FIELD_ROW),
        );
        let lines_box = Rect::from_min_max(
            Pos2::new(right.left(), name_row.bottom() + GAP),
            Pos2::new(right.right(), wish_row.top() - GAP),
        );
        let orders_on = tools.hand.orders_on;
        let (_, add) = theme::button(
            ui,
            Pos2::new(wish_row.right() - ADD_WIDTH, wish_row.top()),
            WORDS_ADD,
            if orders_on {
                theme::GOAL
            } else {
                theme::TEXT_FAINT
            },
        );
        let wish_field = Rect::from_min_max(
            wish_row.min,
            Pos2::new(wish_row.right() - ADD_WIDTH - GAP, wish_row.bottom()),
        );
        for field in [name_row, lines_box, wish_field] {
            ui.painter()
                .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        }
        let margin = egui::Margin::symmetric(8, 6);
        ui.put(
            name_row,
            egui::TextEdit::singleline(&mut self.name)
                .frame(false)
                .margin(margin)
                .hint_text(HINT_NAME)
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        ui.put(
            lines_box,
            egui::TextEdit::multiline(&mut self.lines)
                .frame(false)
                .margin(margin)
                .hint_text(HINT_LINES)
                .font(number_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let wish = ui.put(
            wish_field,
            egui::TextEdit::singleline(&mut self.wish)
                .frame(false)
                .margin(margin)
                .hint_text(if orders_on { HINT_WISH } else { HINT_WISH_OFF })
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let wished = add || (wish.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
        if wished && orders_on && !self.wish.trim().is_empty() {
            tools.hand.ask(Ask::LinesFor(self.wish.trim().to_string()));
            self.say(NOTE_ASKING, false, time);
        }
        self.buttons(
            ui,
            Pos2::new(right.left(), buttons_top),
            frame,
            tools,
            deck,
            live,
        );
        self.show_note(ui, panel, time);
        Some(panel)
    }

    fn list(&mut self, ui: &egui::Ui, panel: Rect, body: Rect, tools: &Tools<'_>) {
        let rows = (body.height() / LIST_ROW) as usize;
        let last_first = self.names.len().saturating_sub(rows);
        self.first_name = scrolled(ui, panel, self.first_name, last_first);
        let shown = self.names.iter().skip(self.first_name).take(rows);
        let mut picked = None;
        for (i, name) in shown.enumerate() {
            let row = Rect::from_min_size(
                body.left_top() + Vec2::new(0.0, i as f32 * LIST_ROW),
                Vec2::new(LIST_WIDTH, LIST_ROW - theme::ROW_GAP / 2.0),
            );
            let response = ui.interact(row, Id::new(("macro-name", name)), Sense::click());
            let fill = if *name == self.name || response.hovered() {
                theme::BUTTON_HOVER
            } else {
                theme::BUTTON
            };
            let painter = ui.painter().with_clip_rect(row);
            painter.rect_filled(row, CornerRadius::same(CELL_RADIUS), fill);
            painter.text(
                row.left_center() + Vec2::new(theme::ROW_GAP, 0.0),
                Align2::LEFT_CENTER,
                name,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            if response.clicked() {
                picked = Some(name.clone());
            }
        }
        if let Some(name) = picked {
            tools.hand.ask(Ask::ScriptText(name));
        }
    }

    fn buttons(
        &mut self,
        ui: &egui::Ui,
        left_top: Pos2,
        frame: &WatchFrame,
        tools: &Tools<'_>,
        deck: &mut DeckUi,
        live: bool,
    ) {
        let record_words = if self.recording {
            WORDS_STOP_RECORDING
        } else {
            WORDS_RECORD
        };
        let acting = [
            WORDS_SAVE,
            WORDS_RUN,
            WORDS_LOOP,
            WORDS_STOP,
            record_words,
            WORDS_PIN,
        ];
        let mut at = left_top;
        let mut pressed = None;
        for words in [WORDS_NEW].into_iter().chain(acting).chain([WORDS_CLOSE]) {
            let needs_control = acting.contains(&words);
            let color = match (needs_control && !live, words) {
                (true, _) => theme::TEXT_FAINT,
                (false, WORDS_RUN | WORDS_LOOP | WORDS_SAVE) => theme::GOAL,
                (false, WORDS_STOP | WORDS_STOP_RECORDING) => theme::ALARM,
                (false, _) => theme::TEXT,
            };
            let (area, clicked) = theme::button(ui, at, words, color);
            at.x = area.right() + theme::ROW_GAP;
            if clicked && (live || !needs_control) {
                pressed = Some(words);
            }
        }
        let name = self.name.trim().to_string();
        let time = tools.time;
        match pressed {
            Some(WORDS_NEW) => {
                self.name.clear();
                self.lines.clear();
            }
            Some(WORDS_CLOSE) => self.open = false,
            Some(WORDS_RUN | WORDS_LOOP) => tools.hand.act(Act::ScriptRun {
                text: self.lines.clone(),
                looping: pressed == Some(WORDS_LOOP),
            }),
            Some(WORDS_STOP) => tools.hand.act(Act::ScriptStop),
            Some(_) if name.is_empty() => self.say(NOTE_NEEDS_NAME, true, time),
            Some(WORDS_SAVE) => {
                tools.hand.act(Act::ScriptSave {
                    name,
                    text: self.lines.clone(),
                });
                tools.hand.ask(Ask::Scripts);
            }
            Some(WORDS_RECORD) => {
                self.recording = true;
                tools.hand.act(Act::RecordStart(name));
            }
            Some(WORDS_STOP_RECORDING) => {
                self.recording = false;
                tools.hand.act(Act::RecordStop);
                tools.hand.ask(Ask::Scripts);
                tools.hand.ask(Ask::ScriptText(name));
            }
            Some(WORDS_PIN) => {
                let pinned = deck.pin_command(&frame.name, &play_line(&name));
                let words = if pinned { NOTE_PINNED } else { NOTE_BAR_FULL };
                self.say(words, !pinned, time);
            }
            _ => {}
        }
    }

    fn show_note(&mut self, ui: &egui::Ui, panel: Rect, time: f64) {
        let Some((words, failed, since)) = &self.note else {
            return;
        };
        if time - since > NOTE_SECONDS {
            self.note = None;
            return;
        }
        let color = if *failed {
            theme::ALARM
        } else {
            theme::WAITING
        };
        theme::shadowed_text(
            ui.painter(),
            Pos2::new(panel.center().x, panel.bottom() + theme::ROW_GAP),
            Align2::CENTER_TOP,
            words,
            text_font(theme::SIZE_BODY),
            color,
        );
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
        let mut editor = MacrosUi::starting(false);
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
        assert!(matches!(&editor.note, Some((words, true, _)) if words.contains("No hotkey")));
    }

    #[test]
    fn a_pinned_macro_is_one_script_line() {
        assert_eq!(play_line("heal self"), "playmacro 'heal self'");
    }
}
