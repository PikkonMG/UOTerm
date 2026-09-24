//! The book gump of the classic client, as the reference
//! client draws every book, old and new: the open book with the cover (the title
//! and the author) on the first left page, two pages at a time, and the
//! corners that turn them; a double click on a corner goes to the first or
//! the last pages. In a book the player may write in, the cover and the
//! pages are text boxes, and each changed one goes to the shard when the
//! pages turn or the book closes. The pages of a sealed book that the shard
//! has not sent are asked for when they come in sight. The session sends a
//! changed cover in the form the shard opened the book with, old (`0x93`)
//! or new (`0xD4`).

use super::canvas::Canvas;
use super::registry::{well_known, Closing, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::{WatchBook, WatchFrame};
use crate::window::control::Act;
use crate::window::model::pages::{
    self, fitted, last_left, turned, BookDraft, BookWords, PAGES_SHOWN,
};
use uoterm_protocol::BOOK_PAGE_LINE_MAX;

pub const BOOK: GumpKind = GumpKind {
    id: well_known::BOOK,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(BookGump::new(serial.unwrap_or_default())),
};

const BACKGROUND: u16 = 0x01FE;
const TURN_BACK: u16 = 0x01FF;
const TURN_ON: u16 = 0x0200;
const TURN_ON_X: i32 = 356;
const NO_HUE: u16 = 0;
// The pages: where the left and the right one start, and their size.
const LEFT_X: i32 = 38;
const RIGHT_X: i32 = 223;
const UPPER_MARGIN: i32 = 26;
const PAGE_WIDTH: i32 = 156;
const PAGE_HEIGHT: i32 = 166;
/// The lines a page of the classic client shows.
const PAGE_LINES: usize = 10;
// The cover.
const COVER_X: i32 = 40;
const TITLE_Y: i32 = 60;
const TITLE_HEIGHT: i32 = 50;
const BY_Y: i32 = 130;
const AUTHOR_Y: i32 = 160;
const AUTHOR_HEIGHT: i32 = 25;
const COVER_WIDTH: i32 = 155;
const COVER_TEXT_WIDTH: u32 = 150;
const TITLE_MAX_CHARS: usize = 47;
const AUTHOR_MAX_CHARS: usize = 29;
const WORDS_BY: &str = "by";
// The number under each page.
const NUMBER_OFFSET_X: i32 = 80;
const NUMBER_Y: i32 = 220;
// The words.
const FONT: u8 = 1;
const PAGE_HUE: u16 = 0x01B5;
const LABEL_HUE: u16 = 1;
const COVER_HUE: u16 = 0;
const NEW_LINE: &str = "\n";
/// The sound of a page that turns, and of a book that opens.
const PAGE_SOUND: u16 = 0x0055;

/// The words of the pages: a little taller lines, as a book has.
fn page_look() -> TextLook {
    let mut look = TextLook::unicode(FONT, PAGE_HUE);
    look.style.extra_height = true;
    look
}

/// Where the page at a place of the book starts: the cover and the even
/// pages on the left, the odd pages on the right.
fn side_x(place: usize) -> i32 {
    if place.is_multiple_of(PAGES_SHOWN) {
        LEFT_X
    } else {
        RIGHT_X
    }
}

pub struct BookGump {
    serial: u32,
    /// The place of the left page that shows. The cover is at place zero
    /// and page `n` at place `n`, so the first two pages to show are the
    /// cover and page one.
    left: usize,
    draft: BookDraft<TextField>,
    opened: bool,
    /// The book shown last is one the player writes in, so the keys go
    /// to its text boxes and not to the game.
    writable: bool,
}

impl BookGump {
    fn new(serial: u32) -> Self {
        Self {
            serial,
            left: 0,
            draft: BookDraft::new(
                TextField::default().with_max_chars(Some(TITLE_MAX_CHARS)),
                TextField::default().with_max_chars(Some(AUTHOR_MAX_CHARS)),
                page_field,
            ),
            opened: false,
            writable: false,
        }
    }

    /// Turns to the pages whose left one is at `to`. Gives the acts of the
    /// turn: a book the player writes in sends what changed; a sealed book
    /// asks for the pages that come in sight and did not come yet. None
    /// when the pages do not move.
    fn turn_to(&mut self, to: usize, book: &WatchBook) -> Option<Vec<Act>> {
        if to == self.left {
            return None;
        }
        self.left = to;
        if book.writable {
            return Some(self.draft.written());
        }
        Some(self.draft.ask_missing(book, to..to + PAGES_SHOWN))
    }

    /// The title, "by" and the author on the first left page.
    fn cover(&mut self, g: &mut Canvas<'_>, writable: bool) {
        let look = TextLook::unicode(FONT, COVER_HUE);
        let label = TextLook::unicode(FONT, LABEL_HUE);
        if writable {
            let draft = &mut self.draft;
            let fields = [
                ("title", TITLE_Y, TITLE_HEIGHT, &mut draft.title),
                ("author", AUTHOR_Y, AUTHOR_HEIGHT, &mut draft.author),
            ];
            let mut changed = false;
            for (key, y, height, field) in fields {
                changed |= g
                    .text_box(key, COVER_X, y, COVER_WIDTH, height, field, &look)
                    .changed;
            }
            draft.cover_changed |= changed;
        } else {
            let look = look.cropped(COVER_TEXT_WIDTH);
            g.label(COVER_X, TITLE_Y, self.draft.title.text(), &look);
            g.label(COVER_X, AUTHOR_Y, self.draft.author.text(), &look);
        }
        g.label(COVER_X, BY_Y, WORDS_BY, &label);
    }

    /// The page at a place, from one, with its number under it.
    fn page(&mut self, g: &mut Canvas<'_>, place: usize, writable: bool) {
        let Some(page) = self.draft.pages.get_mut(place - 1) else {
            return;
        };
        let x = side_x(place);
        let look = page_look();
        if writable {
            let before = page.field.clone();
            let outcome = g.text_box(
                ("page", place),
                x,
                UPPER_MARGIN,
                PAGE_WIDTH,
                PAGE_HEIGHT,
                &mut page.field,
                &look,
            );
            if outcome.changed {
                let fonts = &g.text.fonts;
                let fits = |line: &str| fonts.width(look.font, line) <= PAGE_WIDTH as u32;
                match fitted(
                    page.field.text(),
                    page.field.caret(),
                    fits,
                    BOOK_PAGE_LINE_MAX,
                ) {
                    Some((words, caret)) => {
                        if words != page.field.text() {
                            page.field.set_text(&words);
                            page.field.place_caret(caret, false);
                        }
                        page.changed = true;
                    }
                    // The page is full: the key does nothing.
                    None => page.field = before,
                }
            }
        } else {
            let line_height = g.text.fonts.line_height(&look) as i32;
            let cropped = look.cropped(PAGE_WIDTH as u32);
            for (row, line) in page
                .field
                .text()
                .split(NEW_LINE)
                .take(PAGE_LINES)
                .enumerate()
            {
                g.label(x, UPPER_MARGIN + row as i32 * line_height, line, &cropped);
            }
        }
        let number = TextLook::unicode(FONT, LABEL_HUE);
        g.label(x + NUMBER_OFFSET_X, NUMBER_Y, &place.to_string(), &number);
    }
}

/// The field of a page: its words run over several lines.
fn page_field() -> TextField {
    let mut field = TextField::default();
    field.multiline = true;
    field
}

impl BookWords for TextField {
    fn words(&self) -> &str {
        self.text()
    }

    fn set_words(&mut self, words: &str) {
        self.set_text(words);
    }
}

impl GumpBody for BookGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let frame = cx.frame;
        let Some(book) = frame.book.as_ref().filter(|b| b.serial == self.serial) else {
            return;
        };
        if !std::mem::replace(&mut self.opened, true) {
            cx.play_sound(PAGE_SOUND);
        }
        self.writable = book.writable;
        self.draft.take_shard_words(book);
        g.pic(0, 0, BACKGROUND, NO_HUE);
        // The cover takes a place before the pages.
        let places = pages::page_count(book) + 1;
        let mut to = None;
        if self.left > 0 {
            let corner = g.pic_button("back", 0, 0, TURN_BACK, NO_HUE);
            if corner.double_clicked() {
                to = Some(0);
            } else if corner.clicked() {
                to = Some(turned(self.left, false, places));
            }
        }
        if self.left < last_left(places) {
            let corner = g.pic_button("on", TURN_ON_X, 0, TURN_ON, NO_HUE);
            if corner.double_clicked() {
                to = Some(last_left(places));
            } else if corner.clicked() {
                to = Some(turned(self.left, true, places));
            }
        }
        for place in self.left..(self.left + PAGES_SHOWN).min(places) {
            if place == 0 {
                self.cover(g, book.writable);
            } else {
                self.page(g, place, book.writable);
            }
        }
        if let Some(acts) = to.and_then(|to| self.turn_to(to, book)) {
            cx.play_sound(PAGE_SOUND);
            for act in acts {
                cx.act(act);
            }
        }
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        for act in self.draft.closing() {
            cx.act(act);
        }
        Closing::Now
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame
            .book
            .as_ref()
            .is_some_and(|book| book.serial == self.serial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    const SERIAL: u32 = 0x4000_0B00;

    fn book(writable: bool) -> WatchBook {
        WatchBook {
            serial: SERIAL,
            title: "Tales".into(),
            author: "Ann".into(),
            page_count: 5,
            pages: vec![vec!["Once".into(), "upon".into()], Vec::new()],
            arrived: vec![true, false],
            writable,
        }
    }

    #[test]
    fn the_cover_and_even_pages_stand_left_and_odd_pages_right() {
        assert_eq!(side_x(0), LEFT_X);
        assert_eq!(side_x(1), RIGHT_X);
        assert_eq!(side_x(2), LEFT_X);
        assert_eq!(side_x(5), RIGHT_X);
    }

    #[test]
    fn a_sealed_book_asks_once_for_the_pages_that_come_in_sight() {
        let book = book(false);
        let mut gump = BookGump::new(SERIAL);
        gump.draft.take_shard_words(&book);
        assert_eq!(gump.draft.pages.len(), 5);
        assert_eq!(gump.draft.pages[0].field.text(), "Once\nupon");
        assert_eq!(gump.turn_to(0, &book), None, "the pages did not move");
        assert_eq!(
            gump.turn_to(2, &book),
            Some(vec![Act::BookRead(2), Act::BookRead(3)])
        );
        assert_eq!(
            gump.turn_to(4, &book),
            Some(vec![Act::BookRead(4), Act::BookRead(5)])
        );
        assert_eq!(gump.turn_to(2, &book), Some(Vec::new()), "asked once");
        assert_eq!(gump.draft.closing(), vec![Act::BookClose]);
    }

    #[test]
    fn a_written_book_sends_what_changed_when_the_pages_turn_or_it_closes() {
        let mut book = book(true);
        let mut gump = BookGump::new(SERIAL);
        gump.draft.take_shard_words(&book);
        gump.draft.pages[1].field.set_text("twice");
        gump.draft.pages[1].changed = true;
        gump.draft.title.set_text("Tales Two");
        gump.draft.cover_changed = true;
        assert_eq!(
            gump.turn_to(2, &book),
            Some(vec![
                Act::BookName {
                    title: "Tales Two".into(),
                    author: "Ann".into(),
                },
                Act::BookPage {
                    page: 2,
                    text: "twice".into(),
                },
            ])
        );
        assert_eq!(gump.turn_to(0, &book), Some(Vec::new()), "sent once");
        // The shard's old words come again before it takes the new ones:
        // the page keeps what the player wrote.
        book.pages[1] = vec!["old".into()];
        gump.draft.pages[1].changed = true;
        gump.draft.take_shard_words(&book);
        assert_eq!(gump.draft.pages[1].field.text(), "twice");
        assert_eq!(
            gump.draft.closing(),
            vec![
                Act::BookPage {
                    page: 2,
                    text: "twice".into(),
                },
                Act::BookClose,
            ]
        );
    }

    #[test]
    fn the_gump_lives_while_its_book_is_open() {
        let gump = BookGump::new(SERIAL);
        let mut frame = WatchFrame::default();
        assert!(!gump.alive(&frame));
        frame.book = Some(book(false));
        assert!(gump.alive(&frame));
        frame.book = Some(WatchBook {
            serial: SERIAL + 1,
            ..book(false)
        });
        assert!(!gump.alive(&frame), "another book");
    }

    #[test]
    fn both_kinds_of_book_draw_with_the_client_files() {
        for writable in [false, true] {
            let frame = WatchFrame {
                book: Some(book(writable)),
                ..WatchFrame::default()
            };
            let mut profile = Profile::default();
            let mut manager = GumpManager::default();
            let id = GumpId::of(well_known::BOOK, SERIAL);
            manager.open(id, &mut profile);
            if !draw_frames(&mut manager, &mut profile, &frame) {
                return;
            }
            assert!(manager.is_open(&id));
            draw_frames(&mut manager, &mut profile, &WatchFrame::default());
            assert!(!manager.is_open(&id), "the book closed");
        }
    }
}
