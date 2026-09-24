//! The corpses round the character, for the nearby-loot window and the
//! grid loot, and when a corpse opens by itself by the corpse options of
//! the General page. The grid loot of both styles: which items it offers,
//! how many of each pile its slider takes, and how far the corpse may be.

use crate::view::WatchFrame;
use crate::window::settings::{CorpseOpenRule, GeneralOptions, GridLoot};
use std::collections::{HashMap, HashSet};
use uoterm_protocol::types::{tile_distance, LAYER_BEARD, LAYER_FACE, LAYER_HAIR};

/// The graphic of every corpse; its amount is the body it was.
pub const CORPSE_GRAPHIC: u16 = 0x2006;
/// How far the nearby-loot window looks, in tiles. A shard lets a corpse
/// be looted from two tiles; the window lists a little farther, so the
/// player sees what to walk to.
pub const NEARBY_LOOT_TILES: u16 = 8;

/// The grid loot closes when the corpse on the ground is farther than this.
pub const GRID_LOOT_TILES: u32 = 3;
/// A slider takes at least one item of a pile.
const LEAST_TAKEN: i32 = 1;

/// True when an item of a corpse may be looted: not hair, a beard or a
/// face, by the layer its tile data names.
pub fn lootable(graphic: u16, layer: u8) -> bool {
    graphic != 0 && !matches!(layer, LAYER_HAIR | LAYER_BEARD | LAYER_FACE)
}

/// True while the grid loot of a corpse stays open: the corpse is open, and
/// on the ground within reach, or not on the ground.
pub fn grid_loot_alive(frame: &WatchFrame, corpse: u32) -> bool {
    let open = frame.containers.iter().any(|c| c.serial == corpse);
    let near = frame
        .items
        .iter()
        .find(|item| item.serial == corpse)
        .is_none_or(|item| tile_distance((item.x, item.y), (frame.x, frame.y)) <= GRID_LOOT_TILES);
    open && near
}

/// How many of each pile the slider of its cell takes.
#[derive(Default)]
pub struct LootAmounts {
    amounts: HashMap<u32, i32>,
}

impl LootAmounts {
    /// The amount of a pile of at most `most`, for its slider: all of it
    /// at first, kept from one to `most`.
    pub fn slot(&mut self, serial: u32, most: i32) -> &mut i32 {
        let amount = self.amounts.entry(serial).or_insert(most);
        *amount = (*amount).clamp(LEAST_TAKEN, most.max(LEAST_TAKEN));
        amount
    }

    /// How many of an item a click grabs.
    pub fn taken(&self, serial: u32) -> u16 {
        let amount = self.amounts.get(&serial).copied().unwrap_or(LEAST_TAKEN);
        u16::try_from(amount).unwrap_or(u16::MAX)
    }

    /// Forgets the piles that are gone.
    pub fn keep_only(&mut self, present: impl Fn(u32) -> bool) {
        self.amounts.retain(|serial, _| present(*serial));
    }
}

/// The amount at a share of a slider, from one at its left to `most` at
/// its right.
pub fn amount_at(share: f32, most: i32) -> i32 {
    let span = (most - LEAST_TAKEN).max(0) as f32;
    LEAST_TAKEN + (share.clamp(0.0, 1.0) * span).round() as i32
}

/// The share of a slider an amount stands at.
pub fn share_of(amount: i32, most: i32) -> f32 {
    let span = (most - LEAST_TAKEN).max(0);
    if span == 0 {
        1.0
    } else {
        (amount - LEAST_TAKEN) as f32 / span as f32
    }
}

/// One corpse near the character.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NearbyCorpse {
    pub serial: u32,
    pub name: String,
    pub distance: u32,
    /// The corpse is open, so its items are known.
    pub open: bool,
    /// How many items it holds, when it is open.
    pub items: usize,
}

pub fn is_corpse(frame: &WatchFrame, serial: u32) -> bool {
    frame
        .items
        .iter()
        .any(|item| item.serial == serial && item.graphic == CORPSE_GRAPHIC)
}

/// The corpses within `range` tiles, the nearest first.
pub fn nearby_corpses(frame: &WatchFrame, range: u16) -> Vec<NearbyCorpse> {
    let mut corpses: Vec<NearbyCorpse> = frame
        .items
        .iter()
        .filter(|item| item.graphic == CORPSE_GRAPHIC)
        .map(|item| {
            let open = frame.containers.iter().find(|c| c.serial == item.serial);
            NearbyCorpse {
                serial: item.serial,
                name: item.name.clone(),
                distance: tile_distance((item.x, item.y), (frame.x, frame.y)),
                open: open.is_some(),
                items: open.map_or(0, |c| c.items.len()),
            }
        })
        .filter(|corpse| corpse.distance <= u32::from(range))
        .collect();
    corpses.sort_by_key(|corpse| (corpse.distance, corpse.serial));
    corpses
}

/// How an open corpse shows: as a grid, as the plain container, or both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CorpseLook {
    pub grid: bool,
    pub plain: bool,
}

pub fn corpse_look(grid_loot: GridLoot) -> CorpseLook {
    match grid_loot {
        GridLoot::Off => CorpseLook {
            grid: false,
            plain: true,
        },
        GridLoot::GridOnly => CorpseLook {
            grid: true,
            plain: false,
        },
        GridLoot::Both => CorpseLook {
            grid: true,
            plain: true,
        },
    }
}

/// The corpses to open now by the corpse options: each one in range that
/// was not opened before, when the rule lets the character open one.
pub fn corpses_to_open(
    options: &GeneralOptions,
    frame: &WatchFrame,
    opened: &HashSet<u32>,
) -> Vec<u32> {
    if !options.auto_open_corpses {
        return Vec::new();
    }
    let blocked = match options.corpse_open_rule {
        CorpseOpenRule::Always => false,
        CorpseOpenRule::UnlessTargeting => frame.target_cursor,
        CorpseOpenRule::UnlessHidden => frame.hidden,
        CorpseOpenRule::UnlessTargetingOrHidden => frame.target_cursor || frame.hidden,
    };
    if blocked || frame.dead {
        return Vec::new();
    }
    nearby_corpses(frame, u16::from(options.corpse_open_range))
        .into_iter()
        .filter(|corpse| !corpse.open && !opened.contains(&corpse.serial))
        .map(|corpse| corpse.serial)
        .collect()
}

/// True when an open corpse is shown, by the option that skips the empty.
pub fn shows_corpse(options: &GeneralOptions, items: usize) -> bool {
    !(options.skip_empty_corpses && items == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchContainer, WatchItem};

    fn corpse(serial: u32, x: u16) -> WatchItem {
        WatchItem {
            serial,
            name: "a corpse".into(),
            graphic: CORPSE_GRAPHIC,
            x,
            ..WatchItem::default()
        }
    }

    fn frame() -> WatchFrame {
        WatchFrame {
            items: vec![
                corpse(1, 3),
                corpse(2, 1),
                corpse(3, 40),
                WatchItem {
                    serial: 4,
                    ..WatchItem::default()
                },
            ],
            containers: vec![WatchContainer {
                serial: 2,
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        }
    }

    #[test]
    fn the_nearest_corpses_come_first_and_the_far_ones_not() {
        let near = nearby_corpses(&frame(), NEARBY_LOOT_TILES);
        let serials: Vec<u32> = near.iter().map(|c| c.serial).collect();
        assert_eq!(serials, vec![2, 1]);
        assert!(near[0].open && !near[1].open);
        assert!(is_corpse(&frame(), 3) && !is_corpse(&frame(), 4));
    }

    #[test]
    fn corpses_open_by_the_rule_and_the_range() {
        let mut options = GeneralOptions::default();
        let none = HashSet::new();
        assert!(corpses_to_open(&options, &frame(), &none).is_empty(), "off");
        options.auto_open_corpses = true;
        options.corpse_open_range = 5;
        assert_eq!(corpses_to_open(&options, &frame(), &none), vec![1]);
        assert!(corpses_to_open(&options, &frame(), &HashSet::from([1])).is_empty());
        let hidden = WatchFrame {
            hidden: true,
            ..frame()
        };
        assert!(corpses_to_open(&options, &hidden, &none).is_empty());
        options.corpse_open_rule = CorpseOpenRule::Always;
        assert_eq!(corpses_to_open(&options, &hidden, &none), vec![1]);
    }

    #[test]
    fn the_grid_loot_offers_no_hair_and_closes_out_of_reach() {
        assert!(lootable(0x1517, 0));
        assert!(!lootable(0x203B, LAYER_HAIR));
        assert!(!lootable(0, 0));
        let mut frame = frame();
        assert!(grid_loot_alive(&frame, 2), "open and near");
        assert!(!grid_loot_alive(&frame, 1), "not open");
        frame.x = 10;
        assert!(!grid_loot_alive(&frame, 2), "too far");
        frame.items.clear();
        assert!(grid_loot_alive(&frame, 2), "not on the ground");
    }

    #[test]
    fn a_pile_slider_takes_all_at_first_and_stays_in_the_pile() {
        let mut amounts = LootAmounts::default();
        assert_eq!(*amounts.slot(9, 30), 30);
        *amounts.slot(9, 30) = 50;
        assert_eq!(*amounts.slot(9, 30), 30, "no more than the pile");
        *amounts.slot(9, 30) = 12;
        assert_eq!(amounts.taken(9), 12);
        assert_eq!(amounts.taken(10), 1, "a single item");
        amounts.keep_only(|serial| serial != 9);
        assert_eq!(amounts.taken(9), 1, "a pile that is gone is forgotten");
        assert_eq!(amount_at(0.0, 30), 1);
        assert_eq!(amount_at(1.0, 30), 30);
        assert_eq!(amount_at(0.5, 3), 2);
        assert_eq!(share_of(30, 30), 1.0);
        assert_eq!(share_of(1, 1), 1.0);
        assert_eq!(amount_at(share_of(12, 30), 30), 12);
    }

    #[test]
    fn the_grid_loot_option_picks_the_look_and_empty_corpses_can_hide() {
        assert_eq!(
            corpse_look(GridLoot::GridOnly),
            CorpseLook {
                grid: true,
                plain: false
            }
        );
        assert!(corpse_look(GridLoot::Off).plain);
        let mut options = GeneralOptions::default();
        assert!(shows_corpse(&options, 0));
        options.skip_empty_corpses = true;
        assert!(!shows_corpse(&options, 0) && shows_corpse(&options, 1));
    }
}
