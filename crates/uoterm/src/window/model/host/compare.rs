//! The layers of the wearable items, read from the tiledata of the client
//! files for the compare of `uoterm_view::model::compare`.

use std::path::Path;
use uoterm_nav::TileData;
use uoterm_view::model::compare::ItemLayers;

/// Reads the tiledata of the client files. With none, no item has a
/// layer, and nothing is compared.
pub fn load_item_layers(uopath: Option<&Path>) -> ItemLayers {
    uopath
        .and_then(|path| TileData::open(path).ok())
        .map(|tiles| ItemLayers::of_tiles(&tiles))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_no_client_files_no_item_has_a_layer() {
        const KATANA: u16 = 0x13FF;
        assert_eq!(load_item_layers(None).layer(KATANA), None);
    }

    #[test]
    fn a_katana_is_worn_in_the_hand_in_the_real_files() {
        let Ok(path) = std::env::var(uoterm_nav::ENV_TEST_UOPATH) else {
            return;
        };
        const KATANA: u16 = 0x13FF;
        const ONE_HANDED: u8 = 1;
        let layers = load_item_layers(Some(Path::new(&path)));
        assert_eq!(layers.layer(KATANA), Some(ONE_HANDED));
    }
}
