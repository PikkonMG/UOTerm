//! The control bar at the middle of the top, the words of the last act
//! under it, the chat line, the question with Yes and No, the panel
//! launcher, and the message while no picture shows. The buttons are
//! `clicks::bar_buttons`; a press does what the bar of the Rust window
//! does.

use super::{Colored, FrameSpec, Framed, Place, PANEL_LAUNCHER, PANEL_QUESTION};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::act::Act;
use uoterm_view::actions::{GumpKind, GumpOp, WindowCommand};
use uoterm_view::clicks::{bar_buttons, bar_status, beside_bar, report_shows, ChatMode, Press};
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point};
use uoterm_view::model::asked::asked_commands;
use uoterm_view::model::journal;
use uoterm_view::ui::control_bar::{
    bar_size, place_numbers, GUARD_QUESTION_SIZE, REPORT_GAP, WORDS_FACES, WORDS_MAP_LABEL,
    WORDS_PIN, WORDS_TAKE_TO_TALK,
};
use uoterm_view::ui::deck::WORDS_BAR_FULL;
use uoterm_view::ui::hud::{
    WORDS_NO_PICTURE, WORDS_TRIES_AGAIN, WORDS_WAITING, WORDS_WAITING_LINE,
};
use uoterm_view::ui::launch::{launcher_size, LAUNCHER_ID, LAUNCHES, WORDS_LAUNCHER};
use uoterm_view::ui::layout::{first_place, Spot};
use uoterm_view::ui::question::{
    question_first_place, Asked, Question, QUESTION_ID, WORDS_NO, WORDS_QUESTION, WORDS_YES,
};
use uoterm_view::ui::theme::{css_color, ALARM, TEXT};

/// What the bar and its neighbors keep between frames.
#[derive(Default)]
pub(crate) struct BarState {
    /// The player folded the bar to its place row.
    pub folded: bool,
    pub launcher_open: bool,
    /// The question of the Modern style that waits for Yes or No.
    pub question: Option<Question>,
}

/// The control bar at the middle of the top.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ControlBarData {
    pub place: Place,
    pub location: LocationData,
    pub folded: bool,
    /// The words of the button that opens the launcher, and whether it
    /// shows.
    pub launcher_words: &'static str,
    pub launcher_shows: bool,
    /// What the human does now, over the buttons.
    pub status: Option<&'static str>,
    /// The words of each button, in order.
    pub buttons: Vec<&'static str>,
}

/// Where the character is: the numbers, the map and the way it faces.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocationData {
    pub numbers: String,
    pub map_words: &'static str,
    pub map: String,
    pub faces_words: &'static str,
    pub facing: String,
}

/// The words of the last act, under the bar.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ReportData {
    pub text: String,
    pub failed: bool,
    /// The middle of the top of the words.
    pub at: Point,
}

/// The chat line: its words, whether it takes the keys, its mode and the
/// words in it while it is empty.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ChatData {
    pub text: String,
    pub open: bool,
    pub hidden: bool,
    /// The human has control: the line takes words.
    pub live: bool,
    /// The words of the line while the agent has the character.
    pub idle_words: &'static str,
    pub mode_words: &'static str,
    pub hint: String,
    /// The words of the button that pins a command on the hotbar, when it
    /// shows.
    pub pin_words: Option<&'static str>,
    /// Where the line stands while the journal is shut; None while it
    /// stands in the last row of the journal.
    pub strip: Option<Place>,
}

/// The question that waits for Yes or No: the question of the guard in
/// the middle, or the question of the Modern style in its frame.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct QuestionData {
    pub words: String,
    /// The words are a warning.
    pub alarm: bool,
    pub yes: &'static str,
    pub no: &'static str,
    /// The place of the question of the guard.
    pub place: Place,
    /// The frame of a question of the Modern style.
    pub framed: Option<super::FrameData>,
}

/// The panel launcher.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LauncherData {
    pub buttons: Vec<LauncherButton>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LauncherButton {
    pub words: &'static str,
    pub shows: bool,
}

/// The message in the middle while no picture of the session shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WaitingData {
    pub heading: Colored,
    pub lines: Vec<String>,
}

/// A button of the bar by its words, so a press means the same button
/// after control changed: `{"press": words}`; `{"fold": true}` folds or
/// unfolds the bar; `{"launcher": true}` opens or shuts the launcher.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BarAction {
    Press(String),
    Fold(bool),
    Launcher(bool),
}

/// The chat line: `{"mode": true}` turns to the next mode; `{"pin": true}`
/// puts the command in the line on the hotbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ChatAction {
    Mode(bool),
    Pin(bool),
}

/// `{"answer": true}` answers the question.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum QuestionAction {
    Answer(bool),
}

/// `{"toggle": index}` shows or hides a panel of the launcher.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LauncherAction {
    Toggle(usize),
}

impl WebView {
    fn bar_area(&self, frame: &WatchFrame) -> Area {
        let status = bar_status(frame, self.hand.aiming()).is_some();
        first_place(
            self.panel_room(),
            Spot::ControlBar,
            bar_size(self.panels.bar.folded, status),
        )
    }

    pub(super) fn control_bar_data(&self, frame: &WatchFrame) -> ControlBarData {
        ControlBarData {
            place: Place::from(self.bar_area(frame)),
            location: LocationData {
                numbers: place_numbers(frame),
                map_words: WORDS_MAP_LABEL,
                map: frame.map.to_string(),
                faces_words: WORDS_FACES,
                facing: frame.facing.clone(),
            },
            folded: self.panels.bar.folded,
            launcher_words: WORDS_LAUNCHER,
            launcher_shows: self.panels.bar.launcher_open,
            status: if self.panels.bar.folded {
                None
            } else {
                bar_status(frame, self.hand.aiming())
            },
            buttons: if self.panels.bar.folded {
                Vec::new()
            } else {
                bar_buttons(frame)
                    .into_iter()
                    .map(|(words, _)| words)
                    .collect()
            },
        }
    }

    pub(super) fn bar_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<BarAction>(action) else {
            return;
        };
        match action {
            BarAction::Fold(_) => self.panels.bar.folded = !self.panels.bar.folded,
            BarAction::Launcher(_) => {
                self.panels.bar.launcher_open = !self.panels.bar.launcher_open;
            }
            BarAction::Press(words) => {
                let Some(frame) = self.frame.clone() else {
                    return;
                };
                let pressed = bar_buttons(&frame)
                    .into_iter()
                    .find(|(button, _)| *button == words);
                if let Some((_, press)) = pressed {
                    self.bar_press(&frame, press);
                }
            }
        }
    }

    /// Does what a button of the bar does, as the bar of the Rust window.
    fn bar_press(&mut self, frame: &WatchFrame, press: Press) {
        let toggle = |kind| WindowCommand::Gump(GumpOp::Toggle, kind);
        match press {
            Press::Act(act) => self.hand.act(act),
            Press::Sheet => self.panels.sheet.open = !self.panels.sheet.open,
            Press::Map => self.style_command(frame, toggle(GumpKind::WorldMap)),
            Press::Macros => self.style_command(frame, toggle(GumpKind::Macros)),
            Press::Chat => self.style_command(frame, toggle(GumpKind::Chat)),
            Press::Options => self.style_command(frame, toggle(GumpKind::Options)),
            Press::Profile => self.hand.act(Act::ProfileRead(frame.serial)),
            Press::Quit => self.style_command(frame, WindowCommand::QuitGame),
            Press::Bag(bag) => self.hand.act(Act::Use(bag)),
        }
    }

    /// The words of the last report under the bar, while they show.
    pub(super) fn report_data(&self, time: f64) -> Option<ReportData> {
        let (report, since) = self.hand.newest_report()?;
        if !report_shows(*since, time) {
            return None;
        }
        let frame = self.frame.as_ref()?;
        let (at, _) = beside_bar(self.bar_area(frame), false, REPORT_GAP);
        Some(ReportData {
            text: report.text.clone(),
            failed: report.failed,
            at,
        })
    }

    pub(super) fn chat_data(&self) -> ChatData {
        let speech = &self.profile.speech;
        let live = self.frame.as_ref().is_some_and(|frame| frame.human_control);
        let asked = self.frame.as_ref().and_then(asked_commands);
        let answering = asked.is_some();
        let hint = self.frame.as_ref().map_or_else(String::new, |frame| {
            self.chat_mode
                .hint(frame, answering, self.chat.is_open(speech), true)
                .to_string()
        });
        let can_pin = self.chat_mode == ChatMode::Command && !answering;
        ChatData {
            text: self.chat.text.clone(),
            open: self.chat.is_open(speech),
            hidden: self.chat.is_hidden(),
            live,
            idle_words: WORDS_TAKE_TO_TALK,
            mode_words: self.chat_mode.words(answering),
            hint,
            pin_words: can_pin.then_some(WORDS_PIN),
            strip: self.chat_strip(),
        }
    }

    pub(super) fn chat_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<ChatAction>(action) else {
            return;
        };
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let asked = asked_commands(&frame);
        match action {
            ChatAction::Mode(_) if asked.is_none() => self.chat_mode = self.chat_mode.next(),
            ChatAction::Pin(_) if self.chat_mode == ChatMode::Command && asked.is_none() => {
                let words = self.chat.text.trim().to_string();
                if words.is_empty() {
                    return;
                }
                if self.pin_command(&frame.name, &words) {
                    self.chat.text.clear();
                } else {
                    self.hand.fail(WORDS_BAR_FULL);
                }
            }
            _ => {}
        }
    }

    pub(super) fn question_spec(&self) -> Option<FrameSpec> {
        let question = self.panels.bar.question.as_ref()?;
        let default = question_first_place(self.panel_room(), &question.words);
        Some(FrameSpec::fixed(QUESTION_ID, WORDS_QUESTION, default).closable())
    }

    pub(super) fn question_data(&self) -> Option<QuestionData> {
        let room = self.panel_room();
        if let Some(words) = self.hand.question() {
            return Some(QuestionData {
                words: words.to_string(),
                alarm: true,
                yes: WORDS_YES,
                no: WORDS_NO,
                place: Place::from(Area::from_center_size(room.center(), GUARD_QUESTION_SIZE)),
                framed: None,
            });
        }
        let question = self.panels.bar.question.as_ref()?;
        let spec = self.question_spec()?;
        let frame = self.frame_data(PANEL_QUESTION, &spec);
        Some(QuestionData {
            words: question.words.clone(),
            alarm: false,
            yes: WORDS_YES,
            no: WORDS_NO,
            place: frame.area,
            framed: Some(frame),
        })
    }

    pub(super) fn question_action(&mut self, action: Value) {
        if let Ok(QuestionAction::Answer(yes)) = serde_json::from_value(action) {
            self.answer_asked(yes);
        }
    }

    /// Answers the question that waits: the guard's first, else the
    /// question of the Modern style, whose Yes does what it asked.
    pub(crate) fn answer_asked(&mut self, yes: bool) {
        if self.hand.question().is_some() {
            self.hand.answer_question(yes);
            return;
        }
        let asked = self
            .panels
            .bar
            .question
            .take()
            .map(|question| question.asked);
        match asked.filter(|_| yes) {
            Some(Asked::Quit) => self.hand.act(Act::Quit),
            Some(Asked::DeleteJournalTab(name)) => {
                journal::delete_tab(&mut self.profile.journal.tabs, &name);
                self.keep_profile();
            }
            None => {}
        }
    }

    /// Asks before the game quits, as the classic client does.
    pub(crate) fn ask_quit(&mut self) {
        self.panels.bar.question = Some(Question::quit());
    }

    pub(super) fn launcher_spec(&self) -> FrameSpec {
        let default = first_place(self.panel_room(), Spot::Launcher, launcher_size());
        FrameSpec::fixed(LAUNCHER_ID, WORDS_LAUNCHER, default).closable()
    }

    pub(super) fn launcher_data(&self) -> Option<Framed<LauncherData>> {
        if !self.panels.bar.launcher_open {
            return None;
        }
        let buttons = LAUNCHES
            .iter()
            .map(|(launch, words)| LauncherButton {
                words,
                shows: launch.shows(&self.profile),
            })
            .collect();
        Some(self.framed(
            PANEL_LAUNCHER,
            &self.launcher_spec(),
            LauncherData { buttons },
        ))
    }

    pub(super) fn launcher_action(&mut self, action: Value) {
        let Ok(LauncherAction::Toggle(at)) = serde_json::from_value(action) else {
            return;
        };
        if let Some((launch, _)) = LAUNCHES.get(at) {
            let shows = launch.shows(&self.profile);
            launch.set(&mut self.profile, !shows);
            self.keep_profile();
        }
    }

    pub(super) fn waiting_data(&self) -> WaitingData {
        match self.frame.as_ref() {
            Some(frame) if !frame.error.is_empty() => WaitingData {
                heading: Colored {
                    words: WORDS_NO_PICTURE.to_string(),
                    color: css_color(ALARM),
                },
                lines: vec![frame.error.clone(), WORDS_TRIES_AGAIN.to_string()],
            },
            _ => WaitingData {
                heading: Colored {
                    words: WORDS_WAITING.to_string(),
                    color: css_color(TEXT),
                },
                lines: vec![WORDS_WAITING_LINE.to_string()],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press, saved_profiles};
    use super::super::{PANEL_BAR, PANEL_CHAT, PANEL_LAUNCHER, PANEL_QUESTION};
    use crate::out::OutCall;
    use crate::tests::{fixture_watch_with_backpack, settled};
    use crate::WebView;
    use serde_json::{json, Value};
    use uoterm_view::act::Act;
    use uoterm_view::actions::{GumpKind, GumpOp, WindowCommand};
    use uoterm_view::clicks::{bar_buttons, Press};
    use uoterm_view::ui::launch::{Launch, LAUNCHES};
    use uoterm_view::ui::question::QUIT_WORDS;

    /// The words of the button whose press is `wanted`.
    fn button(view: &WebView, wanted: impl Fn(&Press) -> bool) -> &'static str {
        bar_buttons(view.frame_ref().unwrap())
            .into_iter()
            .find(|(_, press)| wanted(press))
            .map(|(words, _)| words)
            .unwrap()
    }

    /// A view of Mara whom the agent has.
    fn agent_has_her() -> WebView {
        let mut view = WebView::new("{}");
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["human_control"] = json!(false);
        view.frame(&watch.to_string(), 0.0);
        view
    }

    #[test]
    fn take_control_makes_the_same_act_as_the_bar_of_the_window() {
        let mut view = agent_has_her();
        let take = button(&view, |press| matches!(press, Press::Act(Act::Take)));
        let out = press(&mut view, PANEL_BAR, json!({ "press": take }));
        assert_eq!(out_acts(&out), vec![Act::Take.for_page()]);
        assert_eq!(take, "Take control");
    }

    #[test]
    fn war_stop_and_give_back_act_as_in_the_window() {
        let mut view = settled();
        for act in [Act::War(true), Act::Stop, Act::GiveBack] {
            let at = button(
                &view,
                |press| matches!(press, Press::Act(found) if *found == act),
            );
            let out = press(&mut view, PANEL_BAR, json!({ "press": at }));
            assert_eq!(out_acts(&out), vec![act.for_page()]);
        }
    }

    #[test]
    fn the_sheet_button_opens_the_sheet_and_quit_asks_first() {
        let mut view = settled();
        let sheet = button(&view, |press| matches!(press, Press::Sheet));
        press(&mut view, PANEL_BAR, json!({ "press": sheet }));
        assert!(view.panel_data(0.0).sheet.is_some());
        let quit = button(&view, |press| matches!(press, Press::Quit));
        press(&mut view, PANEL_BAR, json!({ "press": quit }));
        let question = view.panel_data(0.0).question.unwrap();
        assert_eq!(question.words, QUIT_WORDS);
        assert!(question.framed.is_some());
        let out = press(&mut view, PANEL_QUESTION, json!({ "answer": true }));
        assert_eq!(out_acts(&out), vec![Act::Quit.for_page()]);
        assert!(view.panel_data(0.0).question.is_none());
    }

    #[test]
    fn the_map_button_goes_to_the_page_as_the_window_command_of_its_window() {
        let mut view = settled();
        let map = button(&view, |press| matches!(press, Press::Map));
        let out = press(&mut view, PANEL_BAR, json!({ "press": map }));
        assert_eq!(
            out,
            vec![OutCall::Window {
                command: WindowCommand::Gump(GumpOp::Toggle, GumpKind::WorldMap)
            }]
        );
    }

    #[test]
    fn the_bar_folds_and_the_launcher_turns_a_panel() {
        let mut view = settled();
        press(&mut view, PANEL_BAR, json!({ "fold": true }));
        let bar = view.panel_data(0.0).bar.unwrap();
        assert!(bar.folded && bar.buttons.is_empty());
        press(&mut view, PANEL_BAR, json!({ "launcher": true }));
        let launcher = view.panel_data(0.0).launcher.unwrap();
        let radar = LAUNCHES
            .iter()
            .position(|(launch, _)| *launch == Launch::Radar)
            .unwrap();
        assert!(launcher.body.buttons[radar].shows);
        let out = press(&mut view, PANEL_LAUNCHER, json!({ "toggle": radar }));
        assert!(!Launch::Radar.shows(&saved_profiles(&out)[0]));
        assert!(view.panel_data(0.0).radar.is_none());
        press(&mut view, PANEL_LAUNCHER, json!({ "close": true }));
        assert!(view.panel_data(0.0).launcher.is_none());
    }

    #[test]
    fn a_command_pins_on_the_hotbar_from_the_chat_line() {
        let mut view = settled();
        for _ in 0..2 {
            press(&mut view, PANEL_CHAT, json!({ "mode": true }));
        }
        assert_eq!(view.panel_data(0.0).chat.mode_words, "Do");
        view.input_native(
            &json!({"kind": "ChatWords", "text": "useskill 'hiding'"}).to_string(),
            0.0,
        );
        let out = press(&mut view, PANEL_CHAT, json!({ "pin": true }));
        assert!(matches!(out.as_slice(), [OutCall::SaveKept { .. }]));
        let data = view.panel_data(0.0);
        assert!(data.chat.text.is_empty());
        assert_eq!(data.hotbar.unwrap().body.slots[0].tip, "useskill 'hiding'");
    }

    #[test]
    fn no_picture_shows_the_waiting_words() {
        let mut view = WebView::new("{}");
        let data = view.panel_data(0.0);
        assert!(data.waiting.is_some() && data.bar.is_none());
    }

    #[test]
    fn a_press_of_a_button_that_is_gone_does_nothing() {
        let war = "War";
        let mut view = agent_has_her();
        assert!(press(&mut view, PANEL_BAR, json!({ "press": war })).is_empty());
        let mut control = settled();
        let out = press(&mut control, PANEL_BAR, json!({ "press": war }));
        assert_eq!(out_acts(&out), vec![Act::War(true).for_page()]);
    }
}
