//! The books of the spells tab: the words of each book button, the book
//! the tab shows, and the book of a school.

use crate::frame::{WatchFrame, WatchSpellbook};
use crate::model::spell_data::book_info;
use uoterm_assist::spells::School;

/// The words of a book button: the school, with a number when the
/// character has more books of it.
pub fn spell_book_words(books: &[&WatchSpellbook], at: usize) -> String {
    let info = |book: &WatchSpellbook| book_info(&book.school, book.graphic).name;
    let school = info(books[at]);
    let same = books.iter().filter(|book| info(book) == school).count();
    let title = book_info(&books[at].school, books[at].graphic).title;
    if same < 2 {
        return title.to_string();
    }
    let number = books[..at]
        .iter()
        .filter(|book| info(book) == school)
        .count()
        + 1;
    format!("{title} {number}")
}

/// The book the tab shows: the one the player chose, or else the first.
pub fn spell_chosen(frame: &WatchFrame, wanted: Option<u32>) -> Option<&WatchSpellbook> {
    frame
        .spellbooks
        .iter()
        .find(|book| Some(book.serial) == wanted)
        .or_else(|| frame.spellbooks.first())
}

/// The book of a school, by its serial. None when the character has none
/// yet.
pub fn choose_school(frame: &WatchFrame, school: School) -> Option<u32> {
    frame
        .spellbooks
        .iter()
        .find(|book| book_info(&book.school, book.graphic).school == school)
        .map(|book| book.serial)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(serial: u32, school: &str) -> WatchSpellbook {
        WatchSpellbook {
            serial,
            school: school.into(),
            ..WatchSpellbook::default()
        }
    }

    #[test]
    fn each_book_is_named_by_its_school_and_counted_when_there_are_more() {
        let books = [book(1, "magery"), book(2, "necromancy"), book(3, "magery")];
        let shown: Vec<&WatchSpellbook> = books.iter().collect();
        assert_eq!(spell_book_words(&shown, 0), "Magery 1");
        assert_eq!(spell_book_words(&shown, 1), "Necromancy");
        assert_eq!(spell_book_words(&shown, 2), "Magery 2");
    }

    #[test]
    fn the_chosen_book_or_the_first_shows_and_a_school_finds_its_book() {
        let frame = WatchFrame {
            spellbooks: vec![book(1, "magery"), book(2, "chivalry")],
            ..WatchFrame::default()
        };
        assert_eq!(spell_chosen(&frame, None).map(|b| b.serial), Some(1));
        assert_eq!(spell_chosen(&frame, Some(2)).map(|b| b.serial), Some(2));
        assert_eq!(choose_school(&frame, School::Chivalry), Some(2));
        assert_eq!(choose_school(&frame, School::Bushido), None);
        assert!(spell_chosen(&WatchFrame::default(), None).is_none());
    }
}
