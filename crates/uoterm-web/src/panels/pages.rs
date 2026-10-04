//! The windows of the shard that hold pages: the old-style menu with a
//! question and its answers, the book two pages at a time, and the
//! bulletin board. Their clicks work only while the human has control, as
//! in the Rust window; what each button does is `uoterm_view::ui::pages`
//! and `model::pages`.

use super::{FrameSpec, Framed, PANEL_BOARD, PANEL_BOOK, PANEL_OLD_MENU};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uoterm_view::frame::WatchFrame;
use uoterm_view::model::pages::{shown_depth, threaded, PAGES_SHOWN};
use uoterm_view::ui::pages::{
    board_first_place, board_text, book_first_place, by_words, menu_cancel_act, menu_first_place,
    menu_pick_act, page_line_room, poster_words, BoardDraft, BoardPress, OpenBook, BOARD_ID,
    BOOK_ID, BOOK_LINES, HINT_AUTHOR, HINT_SUBJECT, HINT_TEXT, HINT_TITLE, INK, MENU_ID, PAPER,
    REPLY_INDENT, WORDS_CANCEL, WORDS_CLOSE, WORDS_POST, WORDS_REMOVE, WORDS_REPLY, WORDS_SAVE,
};
use uoterm_view::ui::theme::css_color;

/// An answer of the old menu with no picture has graphic zero.
const NO_GRAPHIC: u16 = 0;

/// What the book and the board keep between frames.
#[derive(Default)]
pub(crate) struct PagesState {
    book: Option<OpenBook>,
    board: BoardDraft,
    /// How many times the player typed on a page, so the page draws the
    /// words the book kept again after each.
    edits: u64,
    /// The page last typed on and where its caret stands, from zero.
    caret: Option<(usize, usize)>,
}

/// The old-style menu: the question is its title.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OldMenuData {
    pub live: bool,
    pub entries: Vec<MenuEntry>,
    pub cancel: Option<&'static str>,
}

/// One answer of the old menu.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MenuEntry {
    pub picture: Option<String>,
    pub name: String,
}

/// The open book.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BookData {
    /// The player may write in it now.
    pub writing: bool,
    /// The author of a sealed book.
    pub by: Option<String>,
    pub title: String,
    pub author: String,
    pub title_hint: &'static str,
    pub author_hint: &'static str,
    /// The two pages that show.
    pub pages: Vec<PageData>,
    /// The lines a page holds.
    pub lines: usize,
    /// Counts the typing on the pages.
    pub edits: u64,
    pub turns: Vec<&'static str>,
    pub save: Option<&'static str>,
    pub close: Option<&'static str>,
    pub paper: String,
    pub ink: String,
}

/// One page that shows: its number and its words.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PageData {
    pub number: String,
    pub words: String,
    /// Where the caret stands after the player typed on it, in chars.
    pub caret: Option<usize>,
}

/// The bulletin board.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BoardData {
    pub live: bool,
    pub posts: Vec<PostRow>,
    /// The message that is read, or what shows in its place.
    pub text: String,
    pub subject: String,
    pub body: String,
    pub subject_hint: &'static str,
    pub text_hint: &'static str,
    pub post: &'static str,
    pub reply: Option<&'static str>,
    pub remove: Option<&'static str>,
    pub close: &'static str,
    pub paper: String,
    pub ink: String,
}

/// One message of the board, under the one it answers.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PostRow {
    pub serial: u32,
    pub subject: String,
    pub poster: String,
    /// How far right it stands.
    pub indent: f32,
    pub reading: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum MenuAction {
    Pick(usize),
    Cancel(bool),
}

/// The player typed on a page that shows: `side` 0 is the left page.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct PageWords {
    side: usize,
    words: String,
    caret: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
struct CoverWords {
    title: String,
    author: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BookAction {
    /// A turn button, by its place.
    Turn(usize),
    Page(PageWords),
    Cover(CoverWords),
    Save(bool),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BoardAction {
    Read(u32),
    Subject(String),
    Text(String),
    Post(bool),
    Reply(bool),
    Remove(bool),
}

/// A spec closable only while the human has control.
fn closable_while(spec: FrameSpec, live: bool) -> FrameSpec {
    if live {
        spec.closable()
    } else {
        spec
    }
}

impl WebView {
    /// The book in one frame: it follows the book the shard has open, and
    /// a sealed one asks for the pages in sight.
    pub(crate) fn follow_pages(&mut self, frame: &WatchFrame) {
        let Some(book) = frame.book.as_ref() else {
            self.panels.pages.book = None;
            return;
        };
        let open = OpenBook::follow(&mut self.panels.pages.book, book);
        for act in open.asks(book, frame.human_control) {
            self.hand.act(act);
        }
    }

    pub(super) fn menu_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        let menu = frame.old_menu.as_ref()?;
        let default = menu_first_place(self.panel_room(), menu.entries.len());
        let spec = FrameSpec::fixed(MENU_ID, &menu.question, default);
        Some(closable_while(spec, frame.human_control))
    }

    pub(super) fn menu_data(&mut self, frame: &WatchFrame) -> Option<Framed<OldMenuData>> {
        let menu = frame.old_menu.as_ref()?;
        let spec = self.menu_spec(frame)?;
        let live = frame.human_control;
        let entries = menu
            .entries
            .iter()
            .map(|entry| MenuEntry {
                picture: (entry.graphic != NO_GRAPHIC)
                    .then(|| self.item_picture(entry))
                    .flatten(),
                name: entry.name.clone(),
            })
            .collect();
        let body = OldMenuData {
            live,
            entries,
            cancel: live.then_some(WORDS_CANCEL),
        };
        Some(self.framed(PANEL_OLD_MENU, &spec, body))
    }

    pub(super) fn menu_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Some(menu) = frame.old_menu.as_ref() else {
            return;
        };
        match serde_json::from_value::<MenuAction>(action) {
            Ok(MenuAction::Pick(at)) if at < menu.entries.len() => {
                self.hand.act(menu_pick_act(at));
            }
            Ok(MenuAction::Cancel(_)) => self.hand.act(menu_cancel_act()),
            _ => {}
        }
    }

    pub(super) fn book_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        let book = frame.book.as_ref()?;
        let spec = FrameSpec::fixed(BOOK_ID, &book.title, book_first_place(self.panel_room()));
        Some(closable_while(spec, frame.human_control))
    }

    pub(super) fn book_data(&mut self, frame: &WatchFrame) -> Option<Framed<BookData>> {
        let book = frame.book.as_ref()?;
        let spec = self.book_spec(frame)?;
        let live = frame.human_control;
        let writing = live && book.writable;
        let state = &self.panels.pages;
        let open = state.book.as_ref()?;
        let pages = (open.left..open.left + PAGES_SHOWN)
            .filter_map(|number| {
                let page = open.draft.pages.get(number)?;
                Some(PageData {
                    number: (number + 1).to_string(),
                    words: page.field.clone(),
                    caret: state
                        .caret
                        .filter(|(typed, _)| *typed == number)
                        .map(|(_, caret)| caret),
                })
            })
            .collect();
        let body = BookData {
            writing,
            by: (!writing).then(|| by_words(&open.draft.author)),
            title: open.draft.title.clone(),
            author: open.draft.author.clone(),
            title_hint: HINT_TITLE,
            author_hint: HINT_AUTHOR,
            pages,
            lines: BOOK_LINES,
            edits: state.edits,
            turns: open.turns(book).iter().map(|(words, _)| *words).collect(),
            save: writing.then_some(WORDS_SAVE),
            close: live.then_some(WORDS_CLOSE),
            paper: css_color(PAPER),
            ink: css_color(INK),
        };
        Some(self.framed(PANEL_BOOK, &spec, body))
    }

    pub(super) fn book_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone() else {
            return;
        };
        let Some(book) = frame.book.as_ref() else {
            return;
        };
        let Ok(action) = serde_json::from_value::<BookAction>(action) else {
            return;
        };
        let writing = frame.human_control && book.writable;
        let fits_room = page_line_room();
        let measure = self.body_measure.as_ref();
        let state = &mut self.panels.pages;
        let Some(open) = state.book.as_mut() else {
            return;
        };
        let acts = match action {
            BookAction::Turn(at) => match open.turns(book).get(at) {
                Some((_, to)) => open.turn(*to, writing),
                None => Vec::new(),
            },
            BookAction::Page(typed) if writing => {
                let fits = |line: &str| measure.is_none_or(|measure| measure(line).x <= fits_room);
                let number = open.left + typed.side;
                state.edits += 1;
                if let Some(caret) = open.write_page(number, &typed.words, typed.caret, fits) {
                    state.caret = Some((number, caret));
                }
                Vec::new()
            }
            BookAction::Cover(cover) if writing => {
                open.write_cover(&cover.title, &cover.author);
                Vec::new()
            }
            BookAction::Save(_) if writing => open.draft.written(),
            _ => Vec::new(),
        };
        for act in acts {
            self.hand.act(act);
        }
    }

    /// The close mark or Close of the book: what changed goes, then the
    /// close.
    pub(super) fn close_book(&mut self) {
        if !self.frame.as_ref().is_some_and(|frame| frame.human_control) {
            return;
        }
        if let Some(open) = self.panels.pages.book.as_mut() {
            for act in open.draft.closing() {
                self.hand.act(act);
            }
        }
    }

    pub(super) fn board_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        let board = frame.board.as_ref()?;
        let spec = FrameSpec::fixed(BOARD_ID, &board.name, board_first_place(self.panel_room()));
        Some(closable_while(spec, frame.human_control))
    }

    pub(super) fn board_data(&self, frame: &WatchFrame) -> Option<Framed<BoardData>> {
        let board = frame.board.as_ref()?;
        let spec = self.board_spec(frame)?;
        let live = frame.human_control;
        let reading = board
            .reading
            .and_then(|serial| board.posts.iter().find(|post| post.serial == serial));
        let draft = &self.panels.pages.board;
        let posts = threaded(&board.posts)
            .into_iter()
            .map(|(post, depth)| PostRow {
                serial: post.serial,
                subject: post.subject.clone(),
                poster: poster_words(post),
                indent: REPLY_INDENT * shown_depth(depth) as f32,
                reading: board.reading == Some(post.serial),
            })
            .collect();
        let body = BoardData {
            live,
            posts,
            text: board_text(reading),
            subject: draft.subject.clone(),
            body: draft.text.clone(),
            subject_hint: HINT_SUBJECT,
            text_hint: HINT_TEXT,
            post: WORDS_POST,
            reply: reading.map(|_| WORDS_REPLY),
            remove: reading.map(|_| WORDS_REMOVE),
            close: WORDS_CLOSE,
            paper: css_color(PAPER),
            ink: css_color(INK),
        };
        Some(self.framed(PANEL_BOARD, &spec, body))
    }

    pub(super) fn board_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Some(board) = frame.board.as_ref() else {
            return;
        };
        let Ok(action) = serde_json::from_value::<BoardAction>(action) else {
            return;
        };
        let draft = &mut self.panels.pages.board;
        let press = match action {
            BoardAction::Read(serial) => {
                self.hand.act(uoterm_view::act::Act::BoardRead(serial));
                return;
            }
            BoardAction::Subject(words) => {
                draft.subject = words;
                return;
            }
            BoardAction::Text(words) => {
                draft.text = words;
                return;
            }
            BoardAction::Post(_) => BoardPress::Post,
            BoardAction::Reply(_) => BoardPress::Reply,
            BoardAction::Remove(_) => BoardPress::Remove,
        };
        if let Some(act) = draft.press(press, board.reading) {
            self.hand.act(act);
        }
    }

    /// The close mark or Close of the board.
    pub(super) fn close_board(&mut self) {
        if !self.frame.as_ref().is_some_and(|frame| frame.human_control) {
            return;
        }
        let reading = self
            .frame
            .as_ref()
            .and_then(|frame| frame.board.as_ref()?.reading);
        if let Some(act) = self.panels.pages.board.press(BoardPress::Close, reading) {
            self.hand.act(act);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press};
    use super::super::{PANEL_BOARD, PANEL_BOOK, PANEL_OLD_MENU};
    use crate::tests::{fixture_watch_with_backpack, settled};
    use crate::WebView;
    use serde_json::json;
    use uoterm_view::act::Act;
    use uoterm_view::geom::Vector;
    use uoterm_view::ui::pages::{menu_cancel_act, menu_pick_act, BoardDraft, BoardPress};

    const BOOK: u32 = 99;
    const POST: u32 = 52;

    fn view_with(key: &str, value: serde_json::Value, control: bool) -> WebView {
        let mut watch: serde_json::Value =
            serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch[key] = value;
        watch["human_control"] = json!(control);
        let mut view = settled();
        view.frame(&watch.to_string(), 0.1);
        view.tick_native(0.1, crate::tests::VIEW, None);
        view.take_out_native();
        view
    }

    fn menu(control: bool) -> WebView {
        view_with(
            "menu",
            json!({ "question": "What do you make?", "entries": [
                { "graphic": 3922, "hue": 0, "name": "dagger" }] }),
            control,
        )
    }

    fn book(writable: bool, control: bool) -> WebView {
        view_with(
            "book",
            json!({ "serial": BOOK, "title": "Tales", "author": "Ann", "page_count": 4,
                "pages": [["Once", "upon"], null, null, null], "writable": writable }),
            control,
        )
    }

    fn board(control: bool) -> WebView {
        view_with(
            "board",
            json!({ "serial": 50, "name": "town board", "reading": POST, "posts": [
                { "serial": 51, "parent": null, "poster": "Ann", "subject": "Ore", "time": "Day 1",
                  "lines": null },
                { "serial": POST, "parent": 51, "poster": "Bob", "subject": "Re: Ore",
                  "time": "Day 2", "lines": ["I buy."] }] }),
            control,
        )
    }

    #[test]
    fn a_click_on_an_answer_of_the_old_menu_picks_it_as_the_window() {
        let mut view = menu(true);
        let data = view.panel_data(0.0).old_menu.unwrap();
        assert_eq!(data.frame.title, "What do you make?");
        assert_eq!(data.body.entries[0].name, "dagger");
        let out = press(&mut view, PANEL_OLD_MENU, json!({"pick": 0}));
        assert_eq!(out_acts(&out), vec![menu_pick_act(0).for_page()]);
        let out = press(&mut view, PANEL_OLD_MENU, json!({"cancel": true}));
        assert_eq!(out_acts(&out), vec![menu_cancel_act().for_page()]);
        let out = press(&mut view, PANEL_OLD_MENU, json!({"close": true}));
        assert_eq!(out_acts(&out), vec![menu_cancel_act().for_page()]);
    }

    #[test]
    fn a_book_written_sends_its_pages_as_they_turn_and_as_it_closes() {
        let mut view = book(true, true);
        view.set_body_measure(Box::new(|text: &str| Vector::new(text.len() as f32, 10.0)));
        let data = view.panel_data(0.0).book.unwrap();
        assert!(data.body.writing);
        assert_eq!(data.body.pages[0].words, "Once\nupon");
        press(
            &mut view,
            PANEL_BOOK,
            json!({"page": {"side": 1, "words": "Hi", "caret": 2}}),
        );
        let out = press(&mut view, PANEL_BOOK, json!({"turn": 2}));
        let page = Act::BookPage {
            page: 2,
            text: "Hi".into(),
        };
        assert_eq!(out_acts(&out), vec![page.for_page()]);
        assert_eq!(view.panel_data(0.0).book.unwrap().body.pages[0].number, "3");
        press(
            &mut view,
            PANEL_BOOK,
            json!({"cover": {"title": "Tales", "author": "Mara"}}),
        );
        let out = press(&mut view, PANEL_BOOK, json!({"close": true}));
        let name = Act::BookName {
            title: "Tales".into(),
            author: "Mara".into(),
        };
        assert_eq!(
            out_acts(&out),
            vec![name.for_page(), Act::BookClose.for_page()]
        );
    }

    #[test]
    fn a_sealed_book_asks_for_the_pages_in_sight() {
        let mut view = settled();
        let mut watch: serde_json::Value =
            serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["book"] = json!({ "serial": BOOK, "title": "Tales", "author": "Ann",
            "page_count": 4, "pages": [["Once"], null, null, null], "writable": false });
        view.frame(&watch.to_string(), 0.1);
        view.tick_native(0.1, crate::tests::VIEW, None);
        assert_eq!(
            out_acts(&view.take_out_native()),
            vec![Act::BookRead(2).for_page()]
        );
        assert_eq!(
            view.panel_data(0.0).book.unwrap().body.by.unwrap(),
            "by Ann"
        );
    }

    #[test]
    fn a_post_and_a_reply_on_the_board_are_the_windows() {
        let mut view = board(true);
        let data = view.panel_data(0.0).board.unwrap();
        assert_eq!(
            data.body.posts[1].indent,
            uoterm_view::ui::pages::REPLY_INDENT
        );
        assert_eq!(data.body.text, "I buy.");
        let out = press(&mut view, PANEL_BOARD, json!({"read": 51}));
        assert_eq!(out_acts(&out), vec![Act::BoardRead(51).for_page()]);
        press(&mut view, PANEL_BOARD, json!({"subject": "Ore"}));
        press(&mut view, PANEL_BOARD, json!({"text": "Sold."}));
        let out = press(&mut view, PANEL_BOARD, json!({"reply": true}));
        let mut draft = BoardDraft {
            subject: "Ore".into(),
            text: "Sold.".into(),
        };
        let same = draft.press(BoardPress::Reply, Some(POST)).unwrap();
        assert_eq!(out_acts(&out), vec![same.for_page()]);
        let out = press(&mut view, PANEL_BOARD, json!({"remove": true}));
        assert_eq!(out_acts(&out), vec![Act::BoardRemove(POST).for_page()]);
        let out = press(&mut view, PANEL_BOARD, json!({"close": true}));
        assert_eq!(out_acts(&out), vec![Act::BoardClose.for_page()]);
    }

    #[test]
    fn no_page_window_acts_without_control() {
        let mut view = menu(false);
        assert!(out_acts(&press(&mut view, PANEL_OLD_MENU, json!({"pick": 0}))).is_empty());
        assert!(view.panel_data(0.0).old_menu.unwrap().body.cancel.is_none());
        let mut view = book(true, false);
        assert!(!view.panel_data(0.0).book.unwrap().body.writing);
        press(
            &mut view,
            PANEL_BOOK,
            json!({"page": {"side": 0, "words": "x", "caret": 1}}),
        );
        assert!(out_acts(&press(&mut view, PANEL_BOOK, json!({"close": true}))).is_empty());
        let mut view = board(false);
        assert!(out_acts(&press(&mut view, PANEL_BOARD, json!({"read": 51}))).is_empty());
    }
}
