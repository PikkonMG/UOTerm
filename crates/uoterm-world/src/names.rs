//! Display names for the things a server tells us about.
//!
//! A UO server sends mobiles and items with no display name. A client learns a
//! name by asking for the object's property list (`0xD6`) and reading the first
//! line of the reply. Until it asks, every person and every item in the world
//! model is nameless. This module tracks which serials still need that
//! question, and turns a reply into a name.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use uoterm_protocol::{ObjectProperty, Serial};

/// Tab is the field separator inside a property list argument string. A name
/// arrives as `"\ta wooden chair\t"` because the cliloc format has a prefix and
/// a suffix field around the name.
const ARGUMENT_SEPARATOR: char = '\t';
/// A name line in the prefix, name and suffix form has this many fields.
const AFFIXED_NAME_FIELDS: usize = 3;
/// Where the name stands in that form. The prefix before it is a fame
/// title such as "Lord".
const AFFIXED_NAME_AT: usize = 1;
/// Where the suffix stands in that form: a title such as "the banker".
const AFFIXED_SUFFIX_AT: usize = 2;

/// What we know, and what we still have to ask, about object names.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NameBook {
    /// Property list revision we already hold a name for.
    revisions: HashMap<Serial, u32>,
    /// Serials to ask about, oldest first.
    wanted: VecDeque<Serial>,
    /// Serials currently in `wanted`, for a cheap duplicate test.
    queued: HashSet<Serial>,
    /// Serials we have asked about and are still waiting on.
    asked: HashSet<Serial>,
}

impl NameBook {
    /// Ask for this serial's name when the next batch goes out. Asking twice
    /// for the same serial is free.
    pub fn want(&mut self, serial: Serial) {
        if self.queued.contains(&serial) || self.asked.contains(&serial) {
            return;
        }
        self.queued.insert(serial);
        self.wanted.push_back(serial);
    }

    /// A `0xDC` packet told us this object's property list revision. Ask again
    /// only when the revision differs from the one our name came from.
    pub fn note_revision(&mut self, serial: Serial, hash: u32) {
        if self.revisions.get(&serial) == Some(&hash) {
            return;
        }
        self.want(serial);
    }

    /// A `0xD6` reply arrived. Stop asking for this serial at this revision.
    pub fn accept(&mut self, serial: Serial, hash: u32) {
        self.revisions.insert(serial, hash);
        self.asked.remove(&serial);
        self.queued.remove(&serial);
        self.wanted.retain(|s| *s != serial);
    }

    /// Take up to `max` serials to put in one request packet.
    pub fn take_batch(&mut self, max: usize) -> Vec<Serial> {
        let mut batch = Vec::with_capacity(max.min(self.wanted.len()));
        while batch.len() < max {
            let Some(serial) = self.wanted.pop_front() else {
                break;
            };
            self.queued.remove(&serial);
            self.asked.insert(serial);
            batch.push(serial);
        }
        batch
    }

    /// Put every unanswered request back in the queue. A server may drop a
    /// request, and without this the object stays nameless forever.
    pub fn retry_unanswered(&mut self) {
        let stale: Vec<Serial> = self.asked.drain().collect();
        for serial in stale {
            self.want(serial);
        }
    }

    /// The object left our view. Drop everything we were tracking for it.
    pub fn forget(&mut self, serial: Serial) {
        self.revisions.remove(&serial);
        self.asked.remove(&serial);
        if self.queued.remove(&serial) {
            self.wanted.retain(|s| *s != serial);
        }
    }

    /// How many serials are waiting to be asked about.
    pub fn wanted_len(&self) -> usize {
        self.wanted.len()
    }

    /// How many requests are out with no reply yet.
    pub fn asked_len(&self) -> usize {
        self.asked.len()
    }
}

/// The trimmed fields of the first line of a property list, the line that
/// holds the name.
fn name_line_fields(properties: &[ObjectProperty]) -> Option<Vec<&str>> {
    let first = properties.first()?;
    Some(
        first
            .arguments
            .split(ARGUMENT_SEPARATOR)
            .map(str::trim)
            .collect(),
    )
}

/// The display name carried by a property list, if it has one.
///
/// The first line of the list is the name. Its arguments are tab separated
/// because the cliloc format wraps the name in a prefix and a suffix field;
/// a line in another form gives its first field that is not empty.
pub fn display_name(properties: &[ObjectProperty]) -> Option<String> {
    let fields = name_line_fields(properties)?;
    let name = if fields.len() == AFFIXED_NAME_FIELDS {
        fields.get(AFFIXED_NAME_AT).copied()
    } else {
        fields.iter().copied().find(|field| !field.is_empty())
    };
    name.filter(|name| !name.is_empty()).map(str::to_string)
}

/// The title after the name, such as "the banker", when the name line has
/// the prefix, name and suffix form and the suffix is not empty. A player
/// looks for a banker or a healer by this title.
pub fn display_title(properties: &[ObjectProperty]) -> Option<String> {
    let fields = name_line_fields(properties)?;
    if fields.len() != AFFIXED_NAME_FIELDS {
        return None;
    }
    fields
        .get(AFFIXED_SUFFIX_AT)
        .filter(|title| !title.is_empty())
        .map(|title| title.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLILOC_NAME_WITH_AFFIX: u32 = 1050045;
    const CLILOC_PLAIN_NAME: u32 = 1042971;
    const BATCH_MAX: usize = 15;

    fn property(cliloc: u32, arguments: &str) -> ObjectProperty {
        ObjectProperty {
            cliloc,
            arguments: arguments.into(),
        }
    }

    #[test]
    fn display_name_strips_the_prefix_and_suffix_fields() {
        let props = vec![property(CLILOC_NAME_WITH_AFFIX, "\ta wooden chair\t")];
        assert_eq!(display_name(&props).as_deref(), Some("a wooden chair"));
    }

    /// A fame title stands before the name and a job title after it; the
    /// name is the middle field, and the job title is the title.
    #[test]
    fn a_title_on_either_side_is_not_the_name() {
        let props = vec![property(CLILOC_NAME_WITH_AFFIX, "Lady\tKate\t the banker")];
        assert_eq!(display_name(&props).as_deref(), Some("Kate"));
        assert_eq!(display_title(&props).as_deref(), Some("the banker"));
        let untitled = vec![property(CLILOC_NAME_WITH_AFFIX, "\ta cow\t")];
        assert_eq!(display_title(&untitled), None);
        assert_eq!(display_title(&[property(CLILOC_PLAIN_NAME, "Pikkon")]), None);
    }

    #[test]
    fn display_name_reads_a_plain_argument() {
        let props = vec![property(CLILOC_PLAIN_NAME, "Pikkon")];
        assert_eq!(display_name(&props).as_deref(), Some("Pikkon"));
    }

    #[test]
    fn display_name_is_none_without_a_usable_first_line() {
        assert_eq!(display_name(&[]), None);
        assert_eq!(display_name(&[property(CLILOC_PLAIN_NAME, "\t\t")]), None);
    }

    #[test]
    fn a_serial_is_queued_once_and_batched_once() {
        let mut book = NameBook::default();
        book.want(Serial(1));
        book.want(Serial(1));
        book.want(Serial(2));
        assert_eq!(book.wanted_len(), 2);
        let batch = book.take_batch(BATCH_MAX);
        assert_eq!(batch, vec![Serial(1), Serial(2)]);
        assert_eq!(book.wanted_len(), 0);
        assert_eq!(book.asked_len(), 2);
        assert!(book.take_batch(BATCH_MAX).is_empty());
    }

    #[test]
    fn take_batch_stops_at_the_requested_size() {
        let mut book = NameBook::default();
        for serial in 1..=5u32 {
            book.want(Serial(serial));
        }
        assert_eq!(book.take_batch(2).len(), 2);
        assert_eq!(book.wanted_len(), 3);
    }

    #[test]
    fn a_reply_stops_further_asking_until_the_revision_changes() {
        const FIRST_REVISION: u32 = 0xAAAA;
        const SECOND_REVISION: u32 = 0xBBBB;
        let mut book = NameBook::default();
        book.want(Serial(9));
        book.take_batch(BATCH_MAX);
        book.accept(Serial(9), FIRST_REVISION);
        assert_eq!(book.asked_len(), 0);

        book.note_revision(Serial(9), FIRST_REVISION);
        assert_eq!(book.wanted_len(), 0);

        book.note_revision(Serial(9), SECOND_REVISION);
        assert_eq!(book.wanted_len(), 1);
    }

    #[test]
    fn unanswered_requests_are_asked_again() {
        let mut book = NameBook::default();
        book.want(Serial(4));
        book.take_batch(BATCH_MAX);
        assert_eq!(book.wanted_len(), 0);
        book.retry_unanswered();
        assert_eq!(book.wanted_len(), 1);
        assert_eq!(book.asked_len(), 0);
    }

    #[test]
    fn forgetting_an_object_clears_every_record_of_it() {
        const REVISION: u32 = 7;
        let mut book = NameBook::default();
        book.want(Serial(3));
        book.forget(Serial(3));
        assert_eq!(book.wanted_len(), 0);

        book.want(Serial(3));
        book.take_batch(BATCH_MAX);
        book.accept(Serial(3), REVISION);
        book.forget(Serial(3));
        book.note_revision(Serial(3), REVISION);
        assert_eq!(book.wanted_len(), 1);
    }
}
