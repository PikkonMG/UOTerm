//! The Modern windows of the shard that hold pages, as both windows show
//! them: the old-style menu with a question and a list of answers, the
//! book two pages at a time, and the bulletin board. Where each first
//! stands, its words, and what its buttons do. How a book turns and what
//! goes to the shard is `model::pages`'.

use super::layout::{first_place, Spot};
use super::places::{FOOT_ROW, TITLE_ROW};
use super::theme::PANEL_PAD;
use crate::act::Act;
use crate::frame::{WatchBook, WatchPost};
use crate::geom::{Area, Rgba, Vector};
use crate::model::pages::{fitted, last_left, page_count, turned, BookDraft, PAGES_SHOWN};
use uoterm_protocol::BOOK_PAGE_LINE_MAX;

pub const MENU_ID: &str = "modern:old_menu";
pub const BOOK_ID: &str = "modern:book";
pub const BOARD_ID: &str = "modern:board";

const MENU_WIDTH: f32 = 340.0;
/// The old menu shows this many answers at a time.
pub const MENU_ROWS: usize = 8;
pub const MENU_ROW: f32 = 34.0;
/// The row of the cover of a book and of the head of a board.
pub const COVER_ROW: f32 = 32.0;
pub const BOOK_PAGE_WIDTH: f32 = 250.0;
/// The lines a page of a book shows.
pub const BOOK_LINES: usize = 10;
pub const BOOK_LINE: f32 = 20.0;
pub const BOOK_GUTTER: f32 = 28.0;
/// The words of a page stand this far in from its edge.
pub const PAGE_MARGIN: f32 = 8.0;
/// The paper and the ink of a sealed book and of a message of a board.
pub const PAPER: Rgba = Rgba::from_rgb(226, 214, 184);
pub const INK: Rgba = Rgba::from_rgb(46, 36, 24);

pub const BOARD_LIST_WIDTH: f32 = 300.0;
pub const BOARD_TEXT_WIDTH: f32 = 330.0;
/// The board shows this many messages at a time.
pub const BOARD_ROWS: usize = 9;
pub const BOARD_ROW: f32 = 36.0;
pub const BOARD_WRITE_ROWS: usize = 4;
/// An answer stands this much to the right of the message it answers.
pub const REPLY_INDENT: f32 = 14.0;

pub const WORDS_CANCEL: &str = "Cancel";
pub const WORDS_CLOSE: &str = "Close";
pub const WORDS_SAVE: &str = "Save";
const WORDS_FIRST: &str = "First";
const WORDS_BACK: &str = "Back";
const WORDS_NEXT: &str = "Next";
const WORDS_LAST: &str = "Last";
pub const WORDS_BY: &str = "by";
pub const HINT_TITLE: &str = "The title of the book";
pub const HINT_AUTHOR: &str = "The author";
pub const WORDS_POST: &str = "Post";
pub const WORDS_REPLY: &str = "Reply";
pub const WORDS_REMOVE: &str = "Remove";
const WORDS_PICK_ONE: &str = "Click a message to read it.";
const WORDS_LOADING: &str = "The message comes in a moment.";
pub const HINT_SUBJECT: &str = "Subject";
pub const HINT_TEXT: &str = "Your message";
/// The lines of a message join into its words with this.
const LINE_BREAK: &str = "\n";

/// Where the old menu with `entries` answers first stands in `window`.
pub fn menu_first_place(window: Area, entries: usize) -> Area {
    let rows = entries.clamp(1, MENU_ROWS);
    let height = PANEL_PAD * 2.0 + COVER_ROW + rows as f32 * MENU_ROW + FOOT_ROW;
    first_place(window, Spot::Middle(0), Vector::new(MENU_WIDTH, height))
}

/// The act of a click on the answer at `at` of the old menu, from zero:
/// the shard counts the answers from one.
pub fn menu_pick_act(at: usize) -> Act {
    Act::OldMenuPick(Some(u16::try_from(at + 1).unwrap_or(u16::MAX)))
}

/// The act of Cancel or the close mark of the old menu.
pub fn menu_cancel_act() -> Act {
    Act::OldMenuPick(None)
}

/// Where the book first stands in `window`: two pages and the gutter, the
/// cover row over them and the buttons under them.
pub fn book_first_place(window: Area) -> Area {
    let paper = BOOK_LINES as f32 * BOOK_LINE;
    let size = Vector::new(
        BOOK_PAGE_WIDTH * PAGES_SHOWN as f32 + BOOK_GUTTER + PANEL_PAD * 2.0,
        TITLE_ROW + COVER_ROW + paper + FOOT_ROW + PANEL_PAD * 2.0,
    );
    first_place(window, Spot::Middle(0), size)
}

/// The author of a sealed book, as its cover row says.
pub fn by_words(author: &str) -> String {
    format!("{WORDS_BY} {author}")
}

/// The room a line of a page has, between its margins.
pub fn page_line_room() -> f32 {
    BOOK_PAGE_WIDTH - PAGE_MARGIN * 2.0
}

/// The book that is open: the left page that shows, from zero, and what
/// the player wrote in it.
pub struct OpenBook {
    pub serial: u32,
    pub left: usize,
    pub draft: BookDraft<String>,
}

impl OpenBook {
    pub fn new(serial: u32) -> Self {
        Self {
            serial,
            left: 0,
            draft: BookDraft::new(String::new(), String::new(), String::new),
        }
    }

    /// Follows the book the shard has open: another book starts over, and
    /// the words the shard sent come in.
    pub fn follow<'a>(open: &'a mut Option<Self>, book: &WatchBook) -> &'a mut Self {
        if open
            .as_ref()
            .is_none_or(|shown| shown.serial != book.serial)
        {
            *open = Some(Self::new(book.serial));
        }
        let shown = open.get_or_insert_with(|| Self::new(book.serial));
        shown.draft.take_shard_words(book);
        shown
    }

    /// The turn buttons, and the left page each one turns to.
    pub fn turns(&self, book: &WatchBook) -> [(&'static str, usize); 4] {
        let count = page_count(book);
        [
            (WORDS_FIRST, 0),
            (WORDS_BACK, turned(self.left, false, count)),
            (WORDS_NEXT, turned(self.left, true, count)),
            (WORDS_LAST, last_left(count)),
        ]
    }

    /// The acts of a sealed book: it asks the shard for the pages in sight
    /// that did not come, while the human has control.
    pub fn asks(&mut self, book: &WatchBook, live: bool) -> Vec<Act> {
        if !live || book.writable {
            return Vec::new();
        }
        let numbers = self.left + 1..self.left + 1 + PAGES_SHOWN;
        self.draft.ask_missing(book, numbers)
    }

    /// Turns to the left page `to`. A book the player writes sends what
    /// changed as the pages turn.
    pub fn turn(&mut self, to: usize, writing: bool) -> Vec<Act> {
        if to == self.left {
            return Vec::new();
        }
        self.left = to;
        if writing {
            self.draft.written()
        } else {
            Vec::new()
        }
    }

    /// The player typed `words` on page `number` (from zero), with the
    /// caret at `caret` chars. A line too wide for the page breaks, and a
    /// page that is full takes no more. Gives the new caret when the page
    /// took the words. `fits` tells whether one line fits the page.
    pub fn write_page(
        &mut self,
        number: usize,
        words: &str,
        caret: usize,
        fits: impl Fn(&str) -> bool,
    ) -> Option<usize> {
        let page = self.draft.pages.get_mut(number)?;
        let (kept, new_caret) = fitted(words, caret, fits, BOOK_PAGE_LINE_MAX)?;
        page.field = kept;
        page.changed = true;
        Some(new_caret)
    }

    /// The player typed the title and the author of a book he writes.
    pub fn write_cover(&mut self, title: &str, author: &str) {
        title.clone_into(&mut self.draft.title);
        author.clone_into(&mut self.draft.author);
        self.draft.cover_changed = true;
    }
}

/// Where the bulletin board first stands in `window`: the list of
/// messages, and the message that is read with the fields to write one.
pub fn board_first_place(window: Area) -> Area {
    let size = Vector::new(
        BOARD_LIST_WIDTH + BOOK_GUTTER + BOARD_TEXT_WIDTH + PANEL_PAD * 2.0,
        COVER_ROW + BOARD_ROWS as f32 * BOARD_ROW + FOOT_ROW + PANEL_PAD * 2.0,
    );
    first_place(window, Spot::Middle(0), size)
}

/// The words a board shows of the message that is read: none picked, one
/// still on its way, or its lines.
pub fn board_text(reading: Option<&WatchPost>) -> String {
    match reading.map(|post| post.lines.as_ref()) {
        None => WORDS_PICK_ONE.to_string(),
        Some(None) => WORDS_LOADING.to_string(),
        Some(Some(lines)) => lines.join(LINE_BREAK),
    }
}

/// Who wrote a message and when, as its row says.
pub fn poster_words(post: &WatchPost) -> String {
    format!("{}  {}", post.poster, post.time)
}

/// A button of the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardPress {
    Post,
    Reply,
    Remove,
    Close,
}

/// The message the player writes on a board.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BoardDraft {
    pub subject: String,
    pub text: String,
}

impl BoardDraft {
    /// The act of a button of the board, with `reading` the message that
    /// is read. A post or a reply needs a subject, and empties the fields.
    pub fn press(&mut self, press: BoardPress, reading: Option<u32>) -> Option<Act> {
        let ready = !self.subject.trim().is_empty();
        match press {
            BoardPress::Post | BoardPress::Reply if ready => {
                let act = Act::BoardPost {
                    subject: self.subject.trim().to_string(),
                    text: std::mem::take(&mut self.text),
                    reply_to: reading.filter(|_| press == BoardPress::Reply),
                };
                self.subject.clear();
                Some(act)
            }
            BoardPress::Post | BoardPress::Reply => None,
            BoardPress::Remove => reading.map(Act::BoardRemove),
            BoardPress::Close => Some(Act::BoardClose),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOK: u32 = 0x4000_0B00;
    const POST: u32 = 52;

    fn book(writable: bool) -> WatchBook {
        WatchBook {
            serial: BOOK,
            title: "Tales".into(),
            author: "Ann".into(),
            page_count: 5,
            pages: vec![vec!["Once".into(), "upon".into()]],
            arrived: vec![true],
            writable,
        }
    }

    #[test]
    fn a_book_takes_the_words_of_the_shard_and_a_new_book_starts_over() {
        let mut open = None;
        let shown = OpenBook::follow(&mut open, &book(true));
        assert_eq!(shown.draft.pages.len(), 5);
        assert_eq!(shown.draft.pages[0].field, "Once\nupon");
        shown.left = 2;
        let other = WatchBook {
            serial: BOOK + 1,
            ..book(true)
        };
        assert_eq!(OpenBook::follow(&mut open, &other).left, 0);
    }

    #[test]
    fn a_written_page_goes_when_the_pages_turn_and_a_sealed_one_asks() {
        let mut open = None;
        let shown = OpenBook::follow(&mut open, &book(true));
        assert_eq!(shown.write_page(1, "Hi", 2, |_| true), Some(2));
        assert_eq!(
            shown.write_page(1, "a\nb\nc\nd\ne\nf\ng\nh\ni", 0, |_| true),
            None
        );
        let to = shown.turns(&book(true))[2].1;
        assert_eq!(to, 2);
        let acts = shown.turn(to, true);
        assert_eq!(
            acts,
            vec![Act::BookPage {
                page: 2,
                text: "Hi".into()
            }]
        );
        let mut sealed = None;
        let shown = OpenBook::follow(&mut sealed, &book(false));
        assert_eq!(shown.asks(&book(false), true), vec![Act::BookRead(2)]);
        assert!(shown.asks(&book(false), true).is_empty(), "each once");
        assert!(shown.asks(&book(false), false).is_empty());
    }

    #[test]
    fn a_post_needs_a_subject_and_a_reply_answers_the_message_read() {
        let mut draft = BoardDraft::default();
        assert_eq!(draft.press(BoardPress::Post, Some(POST)), None);
        draft.subject = " Ore ".into();
        draft.text = "I buy.".into();
        assert_eq!(
            draft.press(BoardPress::Reply, Some(POST)),
            Some(Act::BoardPost {
                subject: "Ore".into(),
                text: "I buy.".into(),
                reply_to: Some(POST)
            })
        );
        assert_eq!(draft, BoardDraft::default(), "the fields empty");
        assert_eq!(draft.press(BoardPress::Remove, None), None);
        assert_eq!(
            draft.press(BoardPress::Remove, Some(POST)),
            Some(Act::BoardRemove(POST))
        );
        assert_eq!(menu_pick_act(0), Act::OldMenuPick(Some(1)));
        assert_eq!(board_text(None), WORDS_PICK_ONE);
    }
}
