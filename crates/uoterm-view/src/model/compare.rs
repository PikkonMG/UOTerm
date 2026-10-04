//! The compare of an item with the one the character wears in its place:
//! each property with a number, and how much more or less the item has.

use super::properties::{self, Property};
use crate::frame::{WatchEquip, WatchFrame};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uoterm_nav::{TileData, TileFlagSet};

/// The layer of each wearable graphic, from the tiledata of the client.
/// With no client files no item has a layer, and nothing is compared. The
/// host reads them; the browser fetches them from the host.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemLayers {
    layers: BTreeMap<u16, u8>,
}

impl ItemLayers {
    /// The layers of the wearable items of a tiledata file: a wearable
    /// item keeps its layer in its quality.
    pub fn of_tiles(tiles: &TileData) -> Self {
        let layers = (0..tiles.item_count())
            .filter_map(|at| u16::try_from(at).ok())
            .filter_map(|graphic| {
                let tile = tiles.item(graphic)?;
                (tile.flags.contains(TileFlagSet::WEARABLE) && tile.quality != 0)
                    .then_some((graphic, tile.quality))
            })
            .collect();
        Self { layers }
    }

    /// The layer an item of this graphic is worn on. None for an item that
    /// is not worn.
    pub fn layer(&self, graphic: u16) -> Option<u8> {
        self.layers.get(&graphic).copied()
    }
}

/// What the character wears on a layer.
pub fn worn_on(frame: &WatchFrame, layer: u8) -> Option<&WatchEquip> {
    frame.look.equipment.iter().find(|item| item.layer == layer)
}

/// One property with a number on either item.
#[derive(Clone, Debug, PartialEq)]
pub struct Difference {
    pub words: String,
    pub item: f32,
    pub worn: f32,
}

impl Difference {
    pub fn change(&self) -> f32 {
        self.item - self.worn
    }

    /// The change in words, as "+5 fire resist".
    pub fn words_for_player(&self) -> String {
        format!("{:+} {}", self.change(), self.words)
    }
}

fn numbered(lines: &[String]) -> Vec<Property> {
    lines
        .iter()
        .map(|line| properties::parse(line))
        .filter(|property| property.value.is_some() && !property.words.is_empty())
        .collect()
}

/// Each numbered property that differs between the item and the worn one.
/// A property only one of them has counts as 0 on the other.
pub fn differences(item_lines: &[String], worn_lines: &[String]) -> Vec<Difference> {
    let item = numbered(item_lines);
    let worn = numbered(worn_lines);
    let value_in = |list: &[Property], words: &str| {
        list.iter()
            .find(|property| property.words == words)
            .and_then(|property| property.value)
            .unwrap_or(0.0)
    };
    let mut words: Vec<&str> = Vec::new();
    for property in item.iter().chain(&worn) {
        if !words.contains(&property.words.as_str()) {
            words.push(&property.words);
        }
    }
    words
        .into_iter()
        .map(|words| Difference {
            words: words.to_string(),
            item: value_in(&item, words),
            worn: value_in(&worn, words),
        })
        .filter(|difference| difference.change() != 0.0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_string()).collect()
    }

    #[test]
    fn each_changed_number_is_one_difference() {
        let item = lines(&["a ring", "Fire Resist 10%", "Luck 100", "Durability 5 / 5"]);
        let worn = lines(&["a ring", "Fire Resist 4%", "Luck 100", "Faster Casting 1"]);
        let found = differences(&item, &worn);
        let words: Vec<String> = found.iter().map(Difference::words_for_player).collect();
        assert_eq!(
            words,
            vec!["+6 fire resist", "+5 durability", "-1 faster casting"]
        );
    }

    #[test]
    fn with_no_client_files_no_item_has_a_layer() {
        assert_eq!(ItemLayers::default().layer(0x13FF), None);
        let frame = WatchFrame::default();
        assert!(worn_on(&frame, 1).is_none());
        assert_eq!(
            ItemLayers::of_tiles(&TileData::default()),
            ItemLayers::default()
        );
    }

    #[test]
    fn the_layers_survive_the_trip_to_the_browser() {
        const KATANA: u16 = 0x13FF;
        const ONE_HANDED: u8 = 1;
        let text = format!(r#"{{"layers":{{"{KATANA}":{ONE_HANDED}}}}}"#);
        let layers: ItemLayers = serde_json::from_str(&text).unwrap();
        assert_eq!(layers.layer(KATANA), Some(ONE_HANDED));
        assert_eq!(serde_json::to_string(&layers).unwrap(), text);
    }
}
