//! The map files of each facet, opened the first time they are asked for,
//! with the blocks an UltimaLive shard changed laid over them. The window
//! keeps one set for its session; the web server keeps one for each
//! session that sent a live map, so the changes of one session never show
//! in the map of another.

use std::collections::HashMap;
use std::path::Path;
use uoterm_nav::MulMap;
use uoterm_view::art::MapBlockAt;
use uoterm_view::frame::{WatchLiveBlock, WatchLiveMap};

/// The most UltimaLive blocks laid over one set of map files. A live map
/// of a picture holds the blocks round the character, far fewer. Past
/// this the files open anew, with only the blocks of the last live map.
pub const LIVE_BLOCKS_KEPT: usize = 4096;

#[derive(Default)]
pub struct FacetMaps {
    /// None marks a map the client files do not hold.
    maps: HashMap<u8, Option<MulMap>>,
    /// The UltimaLive blocks laid over the maps, each with the count of
    /// changes it was laid at.
    laid: HashMap<(u8, u32), u64>,
}

/// True when `block` of facet `map` is not laid at its count of changes.
fn is_fresh(laid: &HashMap<(u8, u32), u64>, map: u8, block: &WatchLiveBlock) -> bool {
    laid.get(&(map, block.block)) != Some(&block.changed)
}

/// The map files of a facet, opened the first time they are asked for.
fn open_facet<'a>(
    maps: &'a mut HashMap<u8, Option<MulMap>>,
    uopath: &Path,
    map_index: u8,
) -> Option<&'a MulMap> {
    maps.entry(map_index)
        .or_insert_with(|| MulMap::open(uopath, map_index).ok())
        .as_ref()
}

impl FacetMaps {
    /// The map files of a facet. None when the client files do not hold
    /// them.
    pub fn facet(&mut self, uopath: &Path, map_index: u8) -> Option<&MulMap> {
        open_facet(&mut self.maps, uopath, map_index)
    }

    /// The map files of a facet as `live` holds them when it laid blocks
    /// over that facet, else as these hold them.
    pub fn facet_under<'a>(
        &'a mut self,
        live: Option<&'a FacetMaps>,
        uopath: &Path,
        map_index: u8,
    ) -> Option<&'a MulMap> {
        let changed = live
            .filter(|live| live.has_live(map_index))
            .and_then(|live| live.maps.get(&map_index))
            .and_then(Option::as_ref);
        match changed {
            Some(map) => Some(map),
            None => self.facet(uopath, map_index),
        }
    }

    /// True when blocks of a facet are laid over its files.
    pub fn has_live(&self, map_index: u8) -> bool {
        self.laid.keys().any(|(map, _)| *map == map_index)
    }

    /// Lays the map blocks an UltimaLive shard changed over the map files.
    /// A block is laid again only when it changed again. Gives the blocks
    /// laid now. Past [`LIVE_BLOCKS_KEPT`] the files open anew first.
    pub fn take_live_map(&mut self, uopath: &Path, live: &WatchLiveMap) -> Vec<MapBlockAt> {
        let fresh_count = live
            .blocks
            .iter()
            .filter(|block| is_fresh(&self.laid, live.map, block))
            .count();
        if fresh_count == 0 {
            return Vec::new();
        }
        if self.laid.len() + fresh_count > LIVE_BLOCKS_KEPT {
            *self = Self::default();
        }
        let fresh: Vec<_> = live
            .blocks
            .iter()
            .filter(|block| is_fresh(&self.laid, live.map, block))
            .take(LIVE_BLOCKS_KEPT)
            .collect();
        let Some(map) = open_facet(&mut self.maps, uopath, live.map) else {
            return Vec::new();
        };
        let high = u32::from(map.blocks_high());
        let mut laid = Vec::with_capacity(fresh.len());
        for block in fresh {
            let number = u64::from(block.block);
            let land = block
                .land
                .as_ref()
                .map_or(Ok(()), |land| map.set_live_land(number, land));
            let statics = block
                .statics
                .as_ref()
                .map_or(Ok(()), |records| map.set_live_statics(number, records));
            self.laid.insert((live.map, block.block), block.changed);
            if let Err(error) = land.and(statics) {
                tracing::warn!(%error, block = number, "an UltimaLive block was left out");
                continue;
            }
            let (bx, by) = (block.block / high, block.block % high);
            if let (Ok(bx), Ok(by)) = (u16::try_from(bx), u16::try_from(by)) {
                laid.push(MapBlockAt {
                    map: live.map,
                    bx,
                    by,
                });
            }
        }
        laid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_nav::fixtures::write_mini_client;

    /// The fixture map is two blocks high.
    const SECOND_BLOCK: u32 = 1;
    const BLOCK_CELLS: usize = 64;
    const CELL_BYTES: usize = 3;

    fn live_map(blocks: impl IntoIterator<Item = u32>) -> WatchLiveMap {
        WatchLiveMap {
            map: 0,
            revision: 1,
            blocks: blocks
                .into_iter()
                .map(|block| WatchLiveBlock {
                    block,
                    changed: 1,
                    land: Some(vec![0; BLOCK_CELLS * CELL_BYTES]),
                    statics: None,
                })
                .collect(),
        }
    }

    #[test]
    fn the_laid_blocks_never_pass_the_limit() {
        let dir = std::env::temp_dir().join(format!("uoterm-facets-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        write_mini_client(&dir);
        let mut maps = FacetMaps::default();
        let first = maps.take_live_map(&dir, &live_map([SECOND_BLOCK]));
        assert_eq!(first.len(), 1);
        assert!(maps.has_live(0) && !maps.has_live(1));
        let many = LIVE_BLOCKS_KEPT as u32;
        maps.take_live_map(&dir, &live_map(SECOND_BLOCK + 1..=many + 1));
        assert!(maps.laid.len() <= LIVE_BLOCKS_KEPT, "{}", maps.laid.len());
        let again = maps.take_live_map(&dir, &live_map([SECOND_BLOCK]));
        assert_eq!(again.len(), 1, "the files opened anew, so it is laid again");
        assert!(maps.laid.len() <= LIVE_BLOCKS_KEPT);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
