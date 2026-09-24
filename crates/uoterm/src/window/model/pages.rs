//! The rules of the books and the bulletin boards the shard opens, apart
//! from how they draw: how a book turns two pages at a time, which pages
//! of a sealed book to ask the shard for, what the player wrote in a book
//! and when it goes to the shard, how the messages of a board stand under
//! the ones they answer, and how typed words fit the lines of a page.

use crate::view::{WatchBook, WatchPost};
use crate::window::control::Act;
use std::collections::HashSet;

/// A book shows two pages at a time, as a book that lies open.
pub const PAGES_SHOWN: usize = 2;
/// An answer stands one step deeper than its message, down to this depth,
/// so a long thread still fits.
pub const MAX_SHOWN_DEPTH: usize = 4;
const NEW_LINE: char = '\n';
/// The lines of a page join into its words with this.
const LINE_BREAK: &str = "\n";
const SPACE: char = ' ';

/// The pages of a book: the count on its cover, or more when the shard
/// sent more.
pub fn page_count(book: &WatchBook) -> usize {
    usize::from(book.page_count).max(book.pages.len())
}

/// The last left page of a book of `count` pages, from zero.
pub fn last_left(count: usize) -> usize {
    count.saturating_sub(1) / PAGES_SHOWN * PAGES_SHOWN
}

/// The left page after a turn, kept inside the book.
pub fn turned(left_page: usize, forward: bool, count: usize) -> usize {
    if forward {
        (left_page + PAGES_SHOWN).min(last_left(count))
    } else {
        left_page.saturating_sub(PAGES_SHOWN)
    }
}

/// The pages among `numbers` (from one) that the book has and the shard
/// has not sent yet.
pub fn missing_pages(book: &WatchBook, numbers: impl Iterator<Item = usize>) -> Vec<u16> {
    let count = page_count(book);
    numbers
        .filter(|number| (1..=count).contains(number))
        .filter(|number| !book.arrived.get(number - 1).copied().unwrap_or(false))
        .filter_map(|number| u16::try_from(number).ok())
        .collect()
}

/// The words of one field of a book: plain words in a Modern panel, a text
/// field in a classic gump.
pub trait BookWords {
    fn words(&self) -> &str;
    fn set_words(&mut self, words: &str);
}

impl BookWords for String {
    fn words(&self) -> &str {
        self
    }

    fn set_words(&mut self, words: &str) {
        words.clone_into(self);
    }
}

/// Puts words in a field when they differ, so a caret stays otherwise.
fn set_if_other<T: BookWords>(field: &mut T, words: &str) {
    if field.words() != words {
        field.set_words(words);
    }
}

/// One page of a book as a window holds it.
pub struct DraftPage<T> {
    pub field: T,
    /// The lines of the shard the field showed last.
    seen: Option<Vec<String>>,
    /// The player changed it since it was last sent.
    pub changed: bool,
}

/// What a window holds of the open book: the cover and the pages with the
/// player's words over the shard's, what he changed, and the pages asked
/// of the shard.
pub struct BookDraft<T> {
    pub title: T,
    pub author: T,
    /// The title and the author of the shard the fields showed last.
    seen_cover: Option<(String, String)>,
    pub cover_changed: bool,
    pub pages: Vec<DraftPage<T>>,
    /// The pages asked of the shard, so each is asked once.
    asked: HashSet<u16>,
    new_page: fn() -> T,
}

impl<T: BookWords> BookDraft<T> {
    /// A draft with these cover fields; `new_page` makes the field of a
    /// page.
    pub fn new(title: T, author: T, new_page: fn() -> T) -> Self {
        Self {
            title,
            author,
            seen_cover: None,
            cover_changed: false,
            pages: Vec::new(),
            asked: HashSet::new(),
            new_page,
        }
    }

    /// Takes the words the shard sent since the last frame. A cover or a
    /// page the player changed keeps his words.
    pub fn take_shard_words(&mut self, book: &WatchBook) {
        let cover = (book.title.clone(), book.author.clone());
        if self.seen_cover.as_ref() != Some(&cover) {
            if !self.cover_changed {
                set_if_other(&mut self.title, &cover.0);
                set_if_other(&mut self.author, &cover.1);
            }
            self.seen_cover = Some(cover);
        }
        let new_page = self.new_page;
        self.pages.resize_with(page_count(book), || DraftPage {
            field: new_page(),
            seen: None,
            changed: false,
        });
        for (page, lines) in self.pages.iter_mut().zip(&book.pages) {
            if page.seen.as_ref() != Some(lines) {
                if !page.changed {
                    set_if_other(&mut page.field, &lines.join(LINE_BREAK));
                }
                page.seen = Some(lines.clone());
            }
        }
    }

    /// The acts that send the changed cover and pages, as the reference
    /// client sends them when the pages turn. Each change goes once.
    pub fn written(&mut self) -> Vec<Act> {
        let mut acts = Vec::new();
        if std::mem::take(&mut self.cover_changed) {
            acts.push(Act::BookName {
                title: self.title.words().to_string(),
                author: self.author.words().to_string(),
            });
        }
        for (index, page) in self.pages.iter_mut().enumerate() {
            if std::mem::take(&mut page.changed) {
                acts.push(Act::BookPage {
                    page: u16::try_from(index + 1).unwrap_or(u16::MAX),
                    text: page.field.words().to_string(),
                });
            }
        }
        acts
    }

    /// The acts that ask the shard for the pages among `numbers` (from one)
    /// that came in sight and did not come yet, each once.
    pub fn ask_missing(
        &mut self,
        book: &WatchBook,
        numbers: impl Iterator<Item = usize>,
    ) -> Vec<Act> {
        missing_pages(book, numbers)
            .into_iter()
            .filter(|page| self.asked.insert(*page))
            .map(Act::BookRead)
            .collect()
    }

    /// The acts of the player closing the book: what changed, then the
    /// close.
    pub fn closing(&mut self) -> Vec<Act> {
        let mut acts = self.written();
        acts.push(Act::BookClose);
        acts
    }
}

/// The messages of a board with each answer under its message, and how
/// deep each one stands. A message whose parent is gone stands at the left.
pub fn threaded(posts: &[WatchPost]) -> Vec<(&WatchPost, usize)> {
    fn add<'a>(
        posts: &'a [WatchPost],
        parent: Option<u32>,
        depth: usize,
        out: &mut Vec<(&'a WatchPost, usize)>,
    ) {
        for post in posts.iter().filter(|post| post.parent == parent) {
            out.push((post, depth));
            add(posts, Some(post.serial), depth + 1, out);
        }
    }
    let known = |serial: u32| posts.iter().any(|post| post.serial == serial);
    let mut out = Vec::new();
    add(posts, None, 0, &mut out);
    for orphan in posts
        .iter()
        .filter(|post| post.parent.is_some_and(|parent| !known(parent)))
    {
        out.push((orphan, 0));
        add(posts, Some(orphan.serial), 1, &mut out);
    }
    out
}

/// How deep a message of a thread shows.
pub fn shown_depth(depth: usize) -> usize {
    depth.min(MAX_SHOWN_DEPTH)
}

/// Typed words as the lines of a page hold them: a line too wide for the
/// page breaks at its last space, or before the char that does not fit
/// when it has no space, and the caret (in chars) moves with its char.
/// `fits` tells whether one line fits the width. None when the words need
/// more than `max_lines` lines.
pub fn fitted(
    text: &str,
    caret: usize,
    fits: impl Fn(&str) -> bool,
    max_lines: usize,
) -> Option<(String, usize)> {
    let mut out: Vec<char> = Vec::with_capacity(text.len());
    let mut line_start = 0;
    let mut new_caret = caret;
    for (at, ch) in text.chars().enumerate() {
        out.push(ch);
        if ch == NEW_LINE {
            line_start = out.len();
            continue;
        }
        let line: String = out[line_start..].iter().collect();
        if fits(&line) {
            continue;
        }
        let last = out.len() - 1;
        match out[line_start..last].iter().rposition(|c| *c == SPACE) {
            Some(space) => {
                out[line_start + space] = NEW_LINE;
                line_start += space + 1;
            }
            None if last > line_start => {
                out.insert(last, NEW_LINE);
                if at < caret {
                    new_caret += 1;
                }
                line_start = last + 1;
            }
            // One char wider than the page stays on its own line.
            None => {}
        }
    }
    let lines = out.iter().filter(|c| **c == NEW_LINE).count() + 1;
    (lines <= max_lines).then(|| (out.into_iter().collect(), new_caret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_book_turns_two_pages_and_stops_at_its_ends() {
        assert_eq!(turned(0, true, 5), 2);
        assert_eq!(turned(2, true, 5), 4);
        assert_eq!(turned(4, true, 5), 4);
        assert_eq!(turned(4, false, 5), 2);
        assert_eq!(turned(0, false, 5), 0);
        assert_eq!(turned(0, true, 2), 0);
        assert_eq!(turned(0, true, 0), 0);
        assert_eq!(last_left(6), 4);
        assert_eq!(last_left(7), 6);
    }

    #[test]
    fn only_pages_the_book_has_and_the_shard_did_not_send_are_missing() {
        let book = WatchBook {
            page_count: 4,
            pages: vec![vec!["Once".into()], Vec::new(), Vec::new()],
            arrived: vec![true, false, true],
            ..WatchBook::default()
        };
        assert_eq!(page_count(&book), 4);
        assert_eq!(missing_pages(&book, 0..6), vec![2, 4]);
        assert!(missing_pages(&book, 1..2).is_empty());
    }

    fn book(writable: bool) -> WatchBook {
        WatchBook {
            title: "Tales".into(),
            author: "Ann".into(),
            page_count: 5,
            pages: vec![vec!["Once".into(), "upon".into()], Vec::new()],
            arrived: vec![true, false],
            writable,
            ..WatchBook::default()
        }
    }

    fn draft() -> BookDraft<String> {
        BookDraft::new(String::new(), String::new(), String::new)
    }

    #[test]
    fn a_sealed_book_asks_once_for_the_pages_that_come_in_sight() {
        let book = book(false);
        let mut draft = draft();
        draft.take_shard_words(&book);
        assert_eq!(draft.pages.len(), 5);
        assert_eq!(draft.pages[0].field, "Once\nupon");
        assert_eq!(
            draft.ask_missing(&book, 2..4),
            vec![Act::BookRead(2), Act::BookRead(3)]
        );
        assert!(draft.ask_missing(&book, 2..4).is_empty(), "asked once");
        assert_eq!(draft.closing(), vec![Act::BookClose]);
    }

    #[test]
    fn a_written_book_sends_what_changed_once_and_keeps_the_players_words() {
        let mut book = book(true);
        let mut draft = draft();
        draft.take_shard_words(&book);
        assert_eq!(
            (draft.title.as_str(), draft.author.as_str()),
            ("Tales", "Ann")
        );
        draft.pages[1].field = "twice".into();
        draft.pages[1].changed = true;
        draft.author = "Bo".into();
        draft.cover_changed = true;
        assert_eq!(
            draft.written(),
            vec![
                Act::BookName {
                    title: "Tales".into(),
                    author: "Bo".into(),
                },
                Act::BookPage {
                    page: 2,
                    text: "twice".into(),
                },
            ]
        );
        assert!(draft.written().is_empty(), "sent once");
        book.pages[1] = vec!["old".into()];
        draft.pages[1].changed = true;
        draft.take_shard_words(&book);
        assert_eq!(draft.pages[1].field, "twice");
    }

    #[test]
    fn an_answer_stands_under_its_message_and_an_orphan_at_the_left() {
        let post = |serial: u32, parent: Option<u32>| WatchPost {
            serial,
            parent,
            ..WatchPost::default()
        };
        let posts = vec![
            post(1, None),
            post(2, None),
            post(3, Some(1)),
            post(4, Some(3)),
            post(5, Some(99)),
        ];
        let order: Vec<(u32, usize)> = threaded(&posts)
            .into_iter()
            .map(|(post, depth)| (post.serial, depth))
            .collect();
        assert_eq!(order, vec![(1, 0), (3, 1), (4, 2), (2, 0), (5, 0)]);
        assert_eq!(shown_depth(2), 2);
        assert_eq!(shown_depth(9), MAX_SHOWN_DEPTH);
    }

    #[test]
    fn typed_words_break_at_a_space_or_before_the_char_that_does_not_fit() {
        const WIDTH: usize = 5;
        let fits = |line: &str| line.chars().count() <= WIDTH;
        assert_eq!(fitted("ab cd", 5, fits, 3), Some(("ab cd".into(), 5)));
        assert_eq!(fitted("ab cdef", 7, fits, 3), Some(("ab\ncdef".into(), 7)));
        assert_eq!(
            fitted("abcdefg", 7, fits, 3),
            Some(("abcde\nfg".into(), 8)),
            "a break with no space adds a char before the caret"
        );
        assert_eq!(
            fitted("abcdefg", 2, fits, 3).map(|(_, caret)| caret),
            Some(2)
        );
        assert_eq!(fitted("a\nb\nc", 0, fits, 3), Some(("a\nb\nc".into(), 0)));
        assert_eq!(fitted("a\nb\nc\nd", 0, fits, 3), None, "too many lines");
        assert_eq!(fitted("", 0, fits, 1), Some((String::new(), 0)));
    }
}
