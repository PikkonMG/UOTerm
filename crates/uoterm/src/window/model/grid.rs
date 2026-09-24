//! The grid containers: which item stands in which slot, the slots the
//! player locked, the search, and the items he chose to move together.

use super::properties::plain_words;
use crate::view::{WatchContainer, WatchFrame};
use crate::window::control::{Act, DropTo, WHOLE_PILE};
use crate::window::settings::{GridLayout, GridSearch};
use std::collections::BTreeSet;

/// The key of a container in the kept layouts and gump places.
pub fn layout_key(container: u32) -> String {
    format!("{container:08X}")
}

/// The id of the gump place of a grid container.
pub fn place_id(container: u32) -> String {
    format!("grid:{}", layout_key(container))
}

/// The slot of each item: a locked item keeps its slot, and the others fill
/// the free slots in their order. The grid grows past `cells` when there
/// are more items than slots.
pub fn arrange(items: &[u32], layout: &GridLayout, cells: usize) -> Vec<Option<u32>> {
    let mut slots: Vec<Option<u32>> = vec![None; cells];
    let mut placed = BTreeSet::new();
    for (slot, serial) in &layout.locked {
        let Ok(slot) = slot.parse::<usize>() else {
            continue;
        };
        if items.contains(serial) && placed.insert(*serial) {
            if slot >= slots.len() {
                slots.resize(slot + 1, None);
            }
            slots[slot] = Some(*serial);
        }
    }
    let mut free = 0;
    for serial in items.iter().filter(|serial| !placed.contains(*serial)) {
        while free < slots.len() && slots[free].is_some() {
            free += 1;
        }
        if free == slots.len() {
            slots.push(None);
        }
        slots[free] = Some(*serial);
    }
    slots
}

/// Locks an item into a slot. Any other lock of the item or of the slot
/// goes.
pub fn lock(layout: &mut GridLayout, slot: usize, serial: u32) {
    layout.locked.retain(|_, locked| *locked != serial);
    layout.locked.insert(slot.to_string(), serial);
}

pub fn unlock(layout: &mut GridLayout, slot: usize) {
    layout.locked.remove(&slot.to_string());
}

pub fn is_locked(layout: &GridLayout, slot: usize) -> bool {
    layout.locked.contains_key(&slot.to_string())
}

/// How many columns and rows of cells of `cell` points fit a field.
pub fn fits(width: f32, height: f32, cell: f32) -> (usize, usize) {
    let count = |side: f32| ((side / cell).floor() as usize).max(1);
    (count(width), count(height))
}

/// How an item looks under the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Plain,
    /// It matches, and the search marks matches.
    Marked,
    /// It does not match, and the search marks matches.
    Dimmed,
    /// It does not match, and the search hides the rest.
    Hidden,
}

/// True when the name or a property line holds the words of the search.
pub fn matches(query: &str, name: &str, lines: &[String]) -> bool {
    let wanted = plain_words(query);
    wanted.is_empty()
        || plain_words(name).contains(&wanted)
        || lines.iter().any(|line| plain_words(line).contains(&wanted))
}

pub fn search_look(mode: GridSearch, query: &str, name: &str, lines: &[String]) -> Look {
    if plain_words(query).is_empty() {
        return Look::Plain;
    }
    match (matches(query, name, lines), mode) {
        (true, GridSearch::Highlight) => Look::Marked,
        (true, GridSearch::Filter) => Look::Plain,
        (false, GridSearch::Highlight) => Look::Dimmed,
        (false, GridSearch::Filter) => Look::Hidden,
    }
}

/// The items the player chose, to move them together.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    chosen: BTreeSet<u32>,
}

impl Selection {
    pub fn toggle(&mut self, serial: u32) {
        if !self.chosen.remove(&serial) {
            self.chosen.insert(serial);
        }
    }

    pub fn contains(&self, serial: u32) -> bool {
        self.chosen.contains(&serial)
    }

    pub fn is_empty(&self) -> bool {
        self.chosen.is_empty()
    }

    pub fn len(&self) -> usize {
        self.chosen.len()
    }

    pub fn clear(&mut self) {
        self.chosen.clear();
    }

    /// Forgets the items that are no longer in any open container.
    pub fn keep_present(&mut self, frame: &WatchFrame) {
        self.chosen.retain(|serial| {
            frame
                .containers
                .iter()
                .any(|container| container.items.iter().any(|item| item.serial == *serial))
        });
    }

    /// One move for each chosen item into `bag`, the whole pile each. The
    /// bag does not go into itself, and what the frame shows in it already
    /// stays.
    pub fn moves_into(&self, bag: u32, frame: &WatchFrame) -> Vec<Act> {
        let inside: Vec<u32> = open_bag(frame, bag)
            .map(|open| open.items.iter().map(|item| item.serial).collect())
            .unwrap_or_default();
        self.chosen
            .iter()
            .filter(|serial| **serial != bag && !inside.contains(serial))
            .map(|serial| Act::Move {
                item: *serial,
                amount: WHOLE_PILE,
                to: DropTo::Into(bag),
            })
            .collect()
    }
}

/// The open container that is this bag, for its preview.
pub fn open_bag(frame: &WatchFrame, bag: u32) -> Option<&WatchContainer> {
    frame
        .containers
        .iter()
        .find(|container| container.serial == bag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchPackItem;

    const BAG: u32 = 0x4000_0100;

    #[test]
    fn locked_items_keep_their_slots_and_the_rest_fill_the_gaps() {
        let mut layout = GridLayout::default();
        lock(&mut layout, 2, 30);
        assert_eq!(
            arrange(&[10, 20, 30], &layout, 4),
            vec![Some(10), Some(20), Some(30), None]
        );
        lock(&mut layout, 0, 30);
        assert_eq!(layout.locked.len(), 1, "an item has one lock");
        assert_eq!(
            arrange(&[10, 20, 30], &layout, 4),
            vec![Some(30), Some(10), Some(20), None]
        );
        lock(&mut layout, 6, 99);
        assert_eq!(
            arrange(&[10], &layout, 2),
            vec![Some(10), None],
            "a gone item frees its slot"
        );
        assert_eq!(
            arrange(&[10, 20, 30], &layout, 2).len(),
            3,
            "more items grow the grid"
        );
        unlock(&mut layout, 0);
        assert!(!is_locked(&layout, 0) && is_locked(&layout, 6));
    }

    #[test]
    fn a_field_holds_whole_cells_and_one_at_the_least() {
        assert_eq!(fits(230.0, 100.0, 46.0), (5, 2));
        assert_eq!(fits(10.0, 10.0, 46.0), (1, 1));
    }

    #[test]
    fn the_search_marks_or_hides_by_name_and_property() {
        let props = vec!["Fire Resist 10%".to_string()];
        assert_eq!(
            search_look(GridSearch::Highlight, "", "a ring", &props),
            Look::Plain
        );
        assert_eq!(
            search_look(GridSearch::Highlight, "fire", "a ring", &props),
            Look::Marked
        );
        assert_eq!(
            search_look(GridSearch::Highlight, "cold", "a ring", &props),
            Look::Dimmed
        );
        assert_eq!(
            search_look(GridSearch::Filter, "cold", "a ring", &props),
            Look::Hidden
        );
        assert_eq!(
            search_look(GridSearch::Filter, "RING", "a ring", &props),
            Look::Plain
        );
    }

    #[test]
    fn chosen_items_move_together_and_gone_ones_are_forgotten() {
        let mut chosen = Selection::default();
        chosen.toggle(7);
        chosen.toggle(8);
        chosen.toggle(BAG);
        chosen.toggle(8);
        assert_eq!(chosen.len(), 2);
        assert_eq!(
            chosen.moves_into(BAG, &WatchFrame::default()),
            vec![Act::Move {
                item: 7,
                amount: WHOLE_PILE,
                to: DropTo::Into(BAG)
            }]
        );
        let frame = WatchFrame {
            containers: vec![WatchContainer {
                serial: BAG,
                items: vec![WatchPackItem {
                    serial: 7,
                    ..WatchPackItem::default()
                }],
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        };
        assert!(
            chosen.moves_into(BAG, &frame).is_empty(),
            "7 is in the bag already"
        );
        chosen.keep_present(&frame);
        assert!(chosen.contains(7) && !chosen.contains(BAG));
        assert!(open_bag(&frame, BAG).is_some());
        assert_eq!(place_id(BAG), "grid:40000100");
    }
}
