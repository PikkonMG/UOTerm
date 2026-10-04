//! The journal of the Modern style: the tabs of the Journal page, the
//! lines of each kept past what the session sends, the wheel to read back,
//! a search, a save to a text file, the filters, dark mode and its own
//! opacity. The chat line stands in its last row, and in a strip of its
//! own while the journal is shut. What a tab shows is
//! `uoterm_view::model::journal`.

use super::{Colored, FrameSpec, Framed, Place, PANEL_JOURNAL};
use crate::out::OutCall;
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::art::hue_color;
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point, Vector};
use uoterm_view::model::journal::{
    self, saved_file_name, saved_text, scrolled_back, visible, Entry, JournalLog, LocalTime,
};
use uoterm_view::model::places;
use uoterm_view::settings::{Choice, JournalKind};
use uoterm_view::ui::launch::JOURNAL_ID;
use uoterm_view::ui::layout::{first_place, Spot};
use uoterm_view::ui::lists::{
    back_words, journal_waiting_words, journal_wheel_turns, journal_words_color, saved_words,
    WordsColor, HINT_NEW_TAB, HINT_SEARCH, HINT_TAB, HINT_TAB_NAME, JOURNAL_CHAT_ROW,
    JOURNAL_FILTERS, JOURNAL_HEIGHT, JOURNAL_LEAST, JOURNAL_NOTE_SECONDS, JOURNAL_WIDTH,
    WORDS_DELETE_TAB, WORDS_JOURNAL, WORDS_NEW_TAB, WORDS_NO_LINES, WORDS_RENAME, WORDS_SAVE,
};
use uoterm_view::ui::question::Question;
use uoterm_view::ui::theme::{
    css_color, ALARM, DARK_GLASS, GLASS, PANEL_PAD, TEXT, TEXT_DIM, WAITING,
};

/// The page gets at most this many of the lines that may show; more than
/// this never fit a journal.
const PAGE_LINES: usize = 100;
const PERCENT: f32 = 100.0;

#[derive(Default)]
pub(crate) struct JournalState {
    log: JournalLog,
    tab: usize,
    /// How many lines the player read back from the newest.
    back: usize,
    search: String,
    /// Words about a save, whether it failed, and when they came.
    note: Option<(String, bool, f64)>,
    /// The clock of the computer, as the page gave it.
    pub local: LocalTime,
}

/// The journal.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JournalData {
    pub tabs: Vec<JournalTab>,
    /// The words of each kind of line a tab may show.
    pub kinds: &'static [&'static str],
    pub new_tab: &'static str,
    pub new_tab_hint: &'static str,
    pub tab_hint: &'static str,
    pub tab_name_hint: &'static str,
    pub rename: &'static str,
    pub delete_tab: &'static str,
    pub search: String,
    pub search_hint: &'static str,
    pub save: &'static str,
    /// The words about a save, while they show.
    pub note: Option<Colored>,
    pub filters: Vec<Filter>,
    /// The lines that may show, the newest last.
    pub lines: Vec<JournalLine>,
    pub no_lines: Option<&'static str>,
    /// How far the player read back, while he does.
    pub back: Option<String>,
    /// The glass of the journal, in dark mode or not, and its own opacity.
    pub glass: String,
    pub glass_opacity: f32,
}

/// One tab, and the kinds of lines it shows.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JournalTab {
    pub name: String,
    pub chosen: bool,
    pub kinds: Vec<bool>,
}

/// One filter under the search.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Filter {
    pub words: &'static str,
    pub shown: bool,
}

/// One line: its time, who said it, and its words in their color.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JournalLine {
    pub stamp: Option<String>,
    pub name: Option<String>,
    pub text: String,
    pub color: String,
}

/// A tab and the place of a kind among the kinds.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TabKind {
    pub tab: usize,
    pub kind: usize,
}

/// A tab and its new name.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct TabName {
    pub tab: usize,
    pub name: String,
}

/// What the player does in the journal: `{"tab": i}`, `{"filter": i}`,
/// `{"search": words}`, `{"save": true}`, `{"new_tab": name}`,
/// `{"rename": {tab, name}}`, `{"kind": {tab, kind}}`, `{"delete_tab": i}`
/// (which asks first) or `{"wheel": notches}` (up reads back).
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum JournalAction {
    Tab(usize),
    Filter(usize),
    Search(String),
    Save(bool),
    NewTab(String),
    Rename(TabName),
    Kind(TabKind),
    DeleteTab(usize),
    Wheel(f32),
}

impl WebView {
    /// Keeps the new lines of the frame. Call it once in each frame.
    pub(crate) fn follow_journal(&mut self, frame: &WatchFrame) {
        let max = usize::from(self.profile.journal.max_lines);
        let state = &mut self.panels.journal;
        let stamp = state.local.line_stamp();
        state.log.take(frame, &stamp, max);
    }

    /// Takes the clock of the computer the page reads.
    pub(crate) fn set_local_time(&mut self, local: LocalTime) {
        self.panels.journal.local = local;
    }

    pub(super) fn journal_spec(&self) -> FrameSpec {
        let size = Vector::new(JOURNAL_WIDTH, JOURNAL_HEIGHT);
        let default = first_place(self.panel_room(), Spot::Journal, size);
        FrameSpec::fixed(JOURNAL_ID, WORDS_JOURNAL, default)
            .sized(JOURNAL_LEAST)
            .closable()
            .foldable()
    }

    /// The place of the chat line while the journal is shut: a strip at
    /// the foot of where the journal stands.
    pub(super) fn chat_strip(&self) -> Option<Place> {
        if !places::is_shut(&self.profile, JOURNAL_ID) {
            return None;
        }
        let whole = self.panel_area(&self.journal_spec());
        let height = JOURNAL_CHAT_ROW + PANEL_PAD * 2.0;
        Some(Place::from(Area {
            min: Point::new(whole.min.x, whole.max.y - height),
            max: whole.max,
        }))
    }

    /// The lines the chosen tab shows now.
    fn shown_lines(&self) -> Vec<&Entry> {
        let state = &self.panels.journal;
        let journal = &self.profile.journal;
        journal
            .tabs
            .get(state.tab)
            .map(|tab| {
                state
                    .log
                    .shown(tab, journal, &self.profile.ignore, &state.search)
            })
            .unwrap_or_default()
    }

    pub(super) fn journal_data(
        &mut self,
        frame: &WatchFrame,
        time: f64,
    ) -> Option<Framed<JournalData>> {
        if places::is_shut(&self.profile, JOURNAL_ID) {
            return None;
        }
        let tabs_count = self.profile.journal.tabs.len();
        let state = &mut self.panels.journal;
        state.tab = state.tab.min(tabs_count.saturating_sub(1));
        if state
            .note
            .as_ref()
            .is_some_and(|(_, _, since)| time - since > JOURNAL_NOTE_SECONDS)
        {
            state.note = None;
        }
        let shown = self.shown_lines();
        let (range, back) = visible(shown.len(), self.panels.journal.back);
        let with_stamp = !self.profile.journal.hide_timestamps;
        let first = range.end.saturating_sub(PAGE_LINES).max(range.start);
        let lines = shown[first..range.end]
            .iter()
            .map(|entry| self.journal_line(entry, with_stamp))
            .collect();
        let no_lines = shown.is_empty().then_some(WORDS_NO_LINES);
        self.panels.journal.back = back;
        let state = &self.panels.journal;
        let journal = &self.profile.journal;
        let mut options = journal.clone();
        let filters = JOURNAL_FILTERS
            .iter()
            .map(|(words, option)| Filter {
                words,
                shown: *option(&mut options),
            })
            .collect();
        let tabs = journal
            .tabs
            .iter()
            .enumerate()
            .map(|(at, tab)| JournalTab {
                name: tab.name.clone(),
                chosen: at == state.tab,
                kinds: (0..JournalKind::LABELS.len())
                    .map(|kind| tab.kinds.contains(&JournalKind::from_index(kind)))
                    .collect(),
            })
            .collect();
        let glass = if journal.dark_mode { DARK_GLASS } else { GLASS };
        let body = JournalData {
            tabs,
            kinds: JournalKind::LABELS,
            new_tab: WORDS_NEW_TAB,
            new_tab_hint: HINT_NEW_TAB,
            tab_hint: HINT_TAB,
            tab_name_hint: HINT_TAB_NAME,
            rename: WORDS_RENAME,
            delete_tab: WORDS_DELETE_TAB,
            search: state.search.clone(),
            search_hint: HINT_SEARCH,
            save: WORDS_SAVE,
            note: state.note.as_ref().map(|(words, failed, _)| Colored {
                words: words.clone(),
                color: css_color(if *failed { ALARM } else { WAITING }),
            }),
            filters,
            lines,
            no_lines,
            back: (back > 0).then(|| back_words(back)),
            glass: css_color(glass),
            glass_opacity: f32::from(journal.opacity) / PERCENT,
        };
        let mut framed = self.framed(PANEL_JOURNAL, &self.journal_spec(), body);
        if frame.unanswered > 0 {
            framed.frame.aside = Some(Colored {
                words: journal_waiting_words(frame.unanswered),
                color: css_color(WAITING),
            });
        }
        Some(framed)
    }

    fn journal_line(&self, entry: &Entry, with_stamp: bool) -> JournalLine {
        let color = match journal_words_color(entry) {
            WordsColor::Dim => TEXT_DIM,
            WordsColor::Plain => TEXT,
            WordsColor::Hue(hue) => hue_color(&self.art, hue),
        };
        JournalLine {
            stamp: with_stamp.then(|| entry.stamp.clone()),
            name: (!entry.name.is_empty()).then(|| entry.name.clone()),
            text: entry.text.clone(),
            color: css_color(color),
        }
    }

    pub(super) fn journal_action(&mut self, action: Value) {
        let Ok(action) = serde_json::from_value::<JournalAction>(action) else {
            return;
        };
        let tabs = &mut self.profile.journal.tabs;
        match action {
            JournalAction::Tab(at) if at < tabs.len() => {
                self.panels.journal.tab = at;
                self.panels.journal.back = 0;
            }
            JournalAction::Filter(at) => {
                let Some((_, option)) = JOURNAL_FILTERS.get(at) else {
                    return;
                };
                let shown = option(&mut self.profile.journal);
                *shown = !*shown;
                self.keep_profile();
            }
            JournalAction::Search(words) => {
                self.panels.journal.search = words;
                self.panels.journal.back = 0;
            }
            JournalAction::Save(_) => self.save_journal(),
            JournalAction::NewTab(name) => {
                if let Some(tab) = journal::new_tab(&name) {
                    tabs.push(tab);
                    self.panels.journal.tab = tabs.len() - 1;
                    self.keep_profile();
                }
            }
            JournalAction::Rename(TabName { tab, name }) => {
                let named = journal::tab_name(&name);
                if let (Some(tab), Some(named)) = (tabs.get_mut(tab), named) {
                    tab.name = named;
                    self.keep_profile();
                }
            }
            JournalAction::Kind(TabKind { tab, kind }) => {
                if let Some(tab) = tabs
                    .get_mut(tab)
                    .filter(|_| kind < JournalKind::LABELS.len())
                {
                    journal::flip_kind(tab, JournalKind::from_index(kind));
                    self.keep_profile();
                }
            }
            JournalAction::DeleteTab(at) => {
                if let Some(tab) = tabs.get(at) {
                    self.panels.bar.question = Some(Question::delete_journal_tab(&tab.name));
                }
            }
            JournalAction::Wheel(notches) => {
                let state = &mut self.panels.journal;
                let turns = journal_wheel_turns(super::panel_notches(notches));
                state.back = scrolled_back(state.back, turns);
            }
            JournalAction::Tab(_) => {}
        }
    }

    /// Gives the page the lines the tab shows as a text file to keep, named
    /// for the character and the time.
    fn save_journal(&mut self) {
        let Some(name) = self.frame.as_ref().map(|frame| frame.name.clone()) else {
            return;
        };
        let with_stamp = !self.profile.journal.hide_timestamps;
        let text = saved_text(&self.shown_lines(), with_stamp);
        let file = saved_file_name(&name, &self.panels.journal.local.file_stamp());
        self.panels.journal.note = Some((saved_words(&file), false, self.hand.time()));
        self.hand.push(OutCall::Download { name: file, text });
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{press, saved_profiles};
    use super::*;
    use crate::tests::{fixture_watch_with_backpack, settled};
    use serde_json::json;

    /// A view whose frames brought one line said by Bob.
    fn view_with_a_line() -> WebView {
        let mut view = settled();
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["journal_lines"] = json!([{
            "seq": 1, "serial": 2, "name": "Bob", "kind": 0, "hue": 0, "text": "hail"
        }]);
        view.frame(&watch.to_string(), 0.1);
        let frame = view.frame_ref().unwrap().clone();
        view.follow_journal(&frame);
        view
    }

    #[test]
    fn the_journal_shows_the_lines_of_its_tab_and_saves_them_as_a_file() {
        let mut view = view_with_a_line();
        let data = view.panel_data(0.1).journal.unwrap().body;
        assert_eq!(data.lines.len(), 1, "{:?}", data.lines);
        assert_eq!(data.lines[0].name.as_deref(), Some("Bob"));
        assert_eq!(data.lines[0].text, "hail");
        let out = press(&mut view, PANEL_JOURNAL, json!({ "save": true }));
        let [OutCall::Download { name, text }] = out.as_slice() else {
            panic!("{out:?}");
        };
        assert!(name.starts_with("Mara-") && text.contains("Bob: hail"));
        assert!(view.panel_data(0.1).journal.unwrap().body.note.is_some());
    }

    #[test]
    fn a_new_tab_shows_speech_and_a_delete_asks_first() {
        let mut view = settled();
        let tabs = view.profile.journal.tabs.len();
        let out = press(&mut view, PANEL_JOURNAL, json!({ "new_tab": "Mine" }));
        assert_eq!(saved_profiles(&out)[0].journal.tabs.len(), tabs + 1);
        let data = view.panel_data(0.0).journal.unwrap().body;
        assert!(data.tabs[tabs].chosen);
        press(&mut view, PANEL_JOURNAL, json!({ "delete_tab": tabs }));
        assert_eq!(
            view.panel_data(0.0).question.unwrap().words,
            "Delete [Mine] tab?"
        );
        press(
            &mut view,
            crate::panels::PANEL_QUESTION,
            json!({ "answer": true }),
        );
        assert_eq!(view.profile.journal.tabs.len(), tabs);
    }

    #[test]
    fn a_filter_hides_the_shards_lines_and_the_wheel_reads_back() {
        let mut view = view_with_a_line();
        let before = view.profile.journal.show_system_lines;
        press(&mut view, PANEL_JOURNAL, json!({ "filter": 0 }));
        assert_ne!(view.profile.journal.show_system_lines, before);
        press(&mut view, PANEL_JOURNAL, json!({ "wheel": 1.0 }));
        assert!(
            view.panel_data(0.1).journal.unwrap().body.back.is_none(),
            "one line holds back"
        );
    }

    #[test]
    fn a_shut_journal_keeps_the_chat_line_in_a_strip() {
        let mut view = settled();
        assert!(view.panel_data(0.0).chat.strip.is_none());
        press(&mut view, PANEL_JOURNAL, json!({ "close": true }));
        assert!(view.panel_data(0.0).chat.strip.is_some());
    }
}
