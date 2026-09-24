//! The counter bar: how many of each chosen item the backpack holds, with
//! the bags in it that are open, and when an amount is low; and the cells
//! the player fills by dropping an item on them, and empties.

use crate::view::{WatchContainer, WatchFrame, WatchPackItem};
use crate::window::settings::{CounterItem, CounterOptions, NO_HUE};
use std::collections::HashMap;

const THOUSAND: u32 = 1000;
const MILLION: u32 = 1_000_000;

/// The open containers inside the backpack, the backpack first. A bag
/// whose contents the window never saw counts as empty.
pub fn backpack_containers(frame: &WatchFrame) -> Vec<&WatchContainer> {
    let Some(pack) = frame.backpack() else {
        return Vec::new();
    };
    let mut found: Vec<&WatchContainer> = Vec::new();
    let mut next = vec![pack];
    while let Some(serial) = next.pop() {
        if found.iter().any(|c| c.serial == serial) {
            continue;
        }
        if let Some(container) = frame.containers.iter().find(|c| c.serial == serial) {
            next.extend(container.items.iter().map(|item| item.serial));
            found.push(container);
        }
    }
    found
}

/// True when a pack item is one the counter counts.
fn counts(item: &CounterItem, pack: &WatchPackItem) -> bool {
    pack.graphic == item.graphic && (item.hue == NO_HUE || pack.hue == item.hue)
}

/// How many of the item the backpack holds.
pub fn count(containers: &[&WatchContainer], item: &CounterItem) -> u32 {
    containers
        .iter()
        .flat_map(|container| container.items.iter())
        .filter(|pack| counts(item, pack))
        .map(|pack| u32::from(pack.amount.max(1)))
        .sum()
}

/// The first pack item the counter counts, to use it.
pub fn first_counted<'a>(
    containers: &[&'a WatchContainer],
    item: &CounterItem,
) -> Option<&'a WatchPackItem> {
    containers
        .iter()
        .flat_map(|container| container.items.iter())
        .find(|pack| counts(item, pack))
}

/// An amount as the bar shows it: shortened to "1.2k" from the amount the
/// Counters page names, when it asks for that.
pub fn amount_words(amount: u32, options: &CounterOptions) -> String {
    if !options.abbreviate || amount < options.abbreviate_at {
        return amount.to_string();
    }
    let (unit, mark) = if amount >= MILLION {
        (MILLION, "m")
    } else {
        (THOUSAND, "k")
    };
    let whole = amount / unit;
    let tenth = amount % unit * 10 / unit;
    if tenth == 0 {
        format!("{whole}{mark}")
    } else {
        format!("{whole}.{tenth}{mark}")
    }
}

/// True when the bar marks the amount as low.
pub fn is_low(amount: u32, options: &CounterOptions) -> bool {
    options.highlight_when_low && amount < options.low_amount
}

/// How many cells the bar has.
pub fn cells(options: &CounterOptions) -> usize {
    usize::from(options.rows.max(1)) * usize::from(options.columns.max(1))
}

/// The item a slot of the bar uses, from one: the first of the backpack
/// the slot counts.
pub fn slot_item(frame: &WatchFrame, options: &CounterOptions, slot: u8) -> Option<u32> {
    let item = options.items.get(usize::from(slot).checked_sub(1)?)?;
    first_counted(&backpack_containers(frame), item).map(|pack| pack.serial)
}

/// The counter of an item dropped on the bar: its name, graphic and hue.
pub fn counted(item: &WatchPackItem) -> CounterItem {
    CounterItem {
        label: item.name.clone(),
        graphic: item.graphic,
        hue: item.hue,
    }
}

/// Puts a counter in a cell: in the place of the one there, or after the
/// last when the cell is empty.
pub fn put_in_cell(items: &mut Vec<CounterItem>, cell: usize, counter: CounterItem) {
    match items.get_mut(cell) {
        Some(old) => *old = counter,
        None => items.push(counter),
    }
}

/// Empties a cell. False when it held nothing.
pub fn clear_cell(items: &mut Vec<CounterItem>, cell: usize) -> bool {
    let held = cell < items.len();
    if held {
        items.remove(cell);
    }
    held
}

/// The last time the amount of a cell changed, and which way.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Change {
    pub at: f64,
    pub up: bool,
}

/// Follows the amount of each cell of the bar, to mark the ones that
/// changed.
#[derive(Default)]
pub struct Changes {
    last: HashMap<usize, (u32, Option<Change>)>,
}

impl Changes {
    /// Takes the amount of a cell now. Gives its last change; the first
    /// amount seen is no change.
    pub fn observe(&mut self, cell: usize, amount: u32, time: f64) -> Option<Change> {
        let change = match self.last.get(&cell) {
            Some((before, change)) if *before == amount => *change,
            Some((before, _)) => Some(Change {
                at: time,
                up: amount > *before,
            }),
            None => None,
        };
        self.last.insert(cell, (amount, change));
        change
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchEquip, WatchLook};
    use uoterm_protocol::types::LAYER_BACKPACK;

    const PACK: u32 = 0x4000_0100;
    const POUCH: u32 = 0x4000_0200;
    const BANDAGE: u16 = 0x0E21;

    fn item(serial: u32, graphic: u16, hue: u16, amount: u16) -> WatchPackItem {
        WatchPackItem {
            serial,
            graphic,
            hue,
            amount,
            ..WatchPackItem::default()
        }
    }

    fn frame() -> WatchFrame {
        WatchFrame {
            look: WatchLook {
                equipment: vec![WatchEquip {
                    serial: PACK,
                    layer: LAYER_BACKPACK,
                    ..WatchEquip::default()
                }],
                ..WatchLook::default()
            },
            containers: vec![
                WatchContainer {
                    serial: PACK,
                    items: vec![item(1, BANDAGE, 0, 30), item(POUCH, 0x0E79, 0, 1)],
                    ..WatchContainer::default()
                },
                WatchContainer {
                    serial: POUCH,
                    items: vec![item(2, BANDAGE, 0x0021, 12)],
                    ..WatchContainer::default()
                },
                WatchContainer {
                    serial: 0x4000_0900,
                    items: vec![item(3, BANDAGE, 0, 99)],
                    ..WatchContainer::default()
                },
            ],
            ..WatchFrame::default()
        }
    }

    #[test]
    fn the_backpack_and_its_open_bags_are_counted_and_nothing_else() {
        let frame = frame();
        let bags = backpack_containers(&frame);
        assert_eq!(bags.len(), 2);
        let every_hue = CounterItem {
            label: "Bandages".into(),
            graphic: BANDAGE,
            hue: NO_HUE,
        };
        assert_eq!(count(&bags, &every_hue), 42);
        let red = CounterItem {
            hue: 0x0021,
            ..every_hue
        };
        assert_eq!(count(&bags, &red), 12);
        assert_eq!(first_counted(&bags, &red).map(|pack| pack.serial), Some(2));
        assert!(backpack_containers(&WatchFrame::default()).is_empty());
    }

    #[test]
    fn a_dropped_item_fills_a_cell_and_a_cleared_cell_empties() {
        let mut items = Vec::new();
        let bandage = counted(&item(PACK, BANDAGE, 0, 5));
        put_in_cell(&mut items, 3, bandage.clone());
        assert_eq!(
            items,
            vec![bandage.clone()],
            "an empty cell takes the next place"
        );
        let other = counted(&item(POUCH, 0x0F7A, 0, 1));
        put_in_cell(&mut items, 0, other.clone());
        assert_eq!(items, vec![other]);
        assert!(clear_cell(&mut items, 0));
        assert!(!clear_cell(&mut items, 0));
        assert!(items.is_empty());
    }

    #[test]
    fn a_large_amount_is_shortened_and_a_small_one_is_low() {
        let mut options = CounterOptions {
            abbreviate: true,
            ..CounterOptions::default()
        };
        assert_eq!(amount_words(999, &options), "999");
        assert_eq!(amount_words(1000, &options), "1k");
        assert_eq!(amount_words(1250, &options), "1.2k");
        assert_eq!(amount_words(2_500_000, &options), "2.5m");
        options.abbreviate = false;
        assert_eq!(amount_words(1250, &options), "1250");
        assert!(!is_low(1, &options));
        options.highlight_when_low = true;
        assert!(is_low(4, &options) && !is_low(5, &options));
        assert_eq!(cells(&options), 1);
    }

    #[test]
    fn a_slot_uses_its_first_item_and_a_change_is_kept_with_its_way() {
        let options = CounterOptions {
            items: vec![CounterItem {
                label: "Bandages".into(),
                graphic: BANDAGE,
                hue: NO_HUE,
            }],
            ..CounterOptions::default()
        };
        assert_eq!(slot_item(&frame(), &options, 1), Some(1));
        assert_eq!(slot_item(&frame(), &options, 2), None);
        assert_eq!(slot_item(&frame(), &options, 0), None);
        let mut changes = Changes::default();
        assert_eq!(changes.observe(0, 30, 1.0), None);
        assert_eq!(changes.observe(0, 30, 2.0), None);
        assert_eq!(
            changes.observe(0, 29, 3.0),
            Some(Change { at: 3.0, up: false })
        );
        assert_eq!(
            changes.observe(0, 29, 4.0),
            Some(Change { at: 3.0, up: false })
        );
    }
}
