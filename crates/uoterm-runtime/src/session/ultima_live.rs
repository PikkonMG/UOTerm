//! UltimaLive: shards that change the map while the game runs. The shard
//! sends the new land and statics of whole blocks, and asks now and then for
//! checksums of the blocks around the character to learn which blocks the
//! client still holds in an old form.
//!
//! The map files are shared by every session on the same client directory,
//! and another session may play on a shard that never changed its map. So a
//! session whose shard changes a map reads that map from a copy of its own
//! from then on, with every change the shard made laid over the files.

use super::*;
use uoterm_protocol::encode::LIVE_HASH_COUNT;
use uoterm_protocol::{LiveMapDefinition, LiveMapEvent};

/// A hash query covers this many blocks each way from its middle block.
const HASH_REACH: i64 = 2;
/// The side of the square of blocks a hash query covers.
const HASH_SIDE: i64 = 2 * HASH_REACH + 1;
/// A wrap size is in tiles, and a block is eight tiles wide.
const TILES_PER_BLOCK: u16 = 8;
/// `watch` sends the changed blocks this many blocks each way from the
/// character, which covers what a window draws.
const WATCH_BLOCK_REACH: u32 = 4;

/// What the shard changed in one block: its land, its statics, or both.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct BlockEdit {
    land: Option<Vec<u8>>,
    statics: Option<Vec<u8>>,
    /// The count of changes when this block last changed, so a window that
    /// laid it over its map already knows it has.
    changed: u64,
}

#[derive(Debug, Default)]
pub(super) struct UltimaLive {
    /// The name the shard runs UltimaLive under. Nothing is answered or
    /// changed before the shard says it.
    shard: Option<String>,
    /// The size and the wrap size of each map, as the shard defined them.
    definitions: HashMap<u8, LiveMapDefinition>,
    /// Every block the shard changed, by map and block number.
    edits: HashMap<u8, HashMap<u32, BlockEdit>>,
    /// The checksums worked out already. A change drops the one of its block.
    hashes: HashMap<(u8, u32), u16>,
    /// Counts the changes, so a window knows when to draw the map again.
    revision: u64,
}

impl UltimaLive {
    /// True when the shard changed this map, so the session reads its own
    /// copy of it.
    pub(super) fn has_edits(&self, map: u8) -> bool {
        self.edits
            .get(&map)
            .is_some_and(|blocks| !blocks.is_empty())
    }

    /// Opens a map for this session alone, with every change the shard made
    /// to it laid over the files.
    pub(super) fn open_map(
        &self,
        uopath: &Path,
        map: u8,
        variant: MapVariant,
    ) -> std::result::Result<Arc<MulMap>, MapError> {
        let files = MulMap::open_variant(uopath, map, variant)?;
        for (block, edit) in self.edits.get(&map).into_iter().flatten() {
            apply_edit(&files, *block, edit);
        }
        Ok(Arc::new(files))
    }
}

/// Lays one changed block over a map. A block the map does not have is
/// logged and left out: it cannot be drawn or walked.
fn apply_edit(map: &MulMap, block: u32, edit: &BlockEdit) {
    let block = u64::from(block);
    let land = edit
        .land
        .as_ref()
        .map_or(Ok(()), |land| map.set_live_land(block, land));
    let statics = edit
        .statics
        .as_ref()
        .map_or(Ok(()), |records| map.set_live_statics(block, records));
    if let Err(error) = land.and(statics) {
        tracing::warn!(%error, block, "an UltimaLive block was left out");
    }
}

/// Takes one UltimaLive packet.
pub(super) fn on_packet(inner: &mut Inner, msg: &Inbound) {
    match msg {
        Inbound::UltimaLive(LiveMapEvent::Login { shard }) => login(inner, shard),
        Inbound::UltimaLive(LiveMapEvent::MapDefinitions(maps)) => {
            for definition in maps {
                inner
                    .ultima_live
                    .definitions
                    .insert(definition.map, *definition);
            }
        }
        Inbound::UltimaLive(LiveMapEvent::HashQuery { block, map }) => {
            answer_hashes(inner, *block, *map);
        }
        Inbound::UltimaLive(LiveMapEvent::Statics {
            block,
            map,
            records,
        }) => change_block(inner, *map, *block, |edit| {
            edit.statics = Some(records.clone());
        }),
        Inbound::LiveTerrain { block, map, land } => change_block(inner, *map, *block, |edit| {
            edit.land = Some(land.clone());
        }),
        _ => {}
    }
}

/// The shard runs UltimaLive under a name. The reference client takes a name
/// only when it is no path, and a name it refuses turns UltimaLive off.
fn login(inner: &mut Inner, shard: &str) {
    let name = shard.trim();
    let usable = !name.is_empty() && !name.contains(['/', '\\']);
    inner.ultima_live.shard = usable.then(|| name.to_string());
    tracing::info!(shard = name, usable, "UltimaLive login");
}

/// True when the shard runs UltimaLive and speaks of the map underfoot. The
/// reference client leaves every other map alone.
fn speaks_of_this_map(inner: &Inner, map: u8) -> bool {
    inner.ultima_live.shard.is_some() && map == inner.map_index()
}

/// Takes a changed block. The first change of a map moves this session to a
/// copy of the map of its own; later ones are laid over that copy.
fn change_block(inner: &mut Inner, map: u8, block: u32, change: impl FnOnce(&mut BlockEdit)) {
    if !speaks_of_this_map(inner, map) {
        return;
    }
    let on_the_map = inner.maps.get(&map).is_none_or(|files| {
        u64::from(block) < u64::from(files.blocks_wide()) * u64::from(files.blocks_high())
    });
    if !on_the_map {
        return;
    }
    let first = !inner.ultima_live.has_edits(map);
    let live = &mut inner.ultima_live;
    let edit = live.edits.entry(map).or_default().entry(block).or_default();
    change(edit);
    live.revision += 1;
    edit.changed = live.revision;
    let edit = edit.clone();
    live.hashes.remove(&(map, block));
    if first {
        inner.maps.remove(&map);
        inner.map_variants.remove(&map);
        inner.ensure_facet();
    } else if let Some(files) = inner.maps.get(&map) {
        apply_edit(files, block, &edit);
    }
}

/// The block numbers a hash query asks about, column by column around
/// `block`, as the reference client walks them. A map wraps at its wrap size
/// while the middle block lies inside it, and at its full size past it.
fn hash_blocks(
    block: u32,
    wide: u32,
    high: u32,
    wrap: Option<(u32, u32)>,
) -> [Option<u32>; LIVE_HASH_COUNT] {
    let (bx, by) = (i64::from(block / high), i64::from(block % high));
    let (wrap_wide, wrap_high) = wrap.unwrap_or((0, 0));
    let across = if bx < i64::from(wrap_wide) {
        wrap_wide
    } else {
        wide
    };
    let down = if by < i64::from(wrap_high) {
        wrap_high
    } else {
        high
    };
    let count = u64::from(wide) * u64::from(high);
    let mut out = [None; LIVE_HASH_COUNT];
    for dx in -HASH_REACH..=HASH_REACH {
        let x = (bx + dx).rem_euclid(i64::from(across));
        for dy in -HASH_REACH..=HASH_REACH {
            let y = (by + dy).rem_euclid(i64::from(down));
            let number = x * i64::from(high) + y;
            let slot = ((dx + HASH_REACH) * HASH_SIDE + dy + HASH_REACH) as usize;
            out[slot] = (u64::try_from(number).is_ok_and(|n| n < count)).then_some(number as u32);
        }
    }
    out
}

/// Answers a hash query with the checksum of each block around `block`. A
/// block past the map answers zero, as the reference client answers it.
fn answer_hashes(inner: &mut Inner, block: u32, map: u8) {
    if !speaks_of_this_map(inner, map) {
        return;
    }
    inner.ensure_facet();
    let Some(files) = inner.maps.get(&map).cloned() else {
        return;
    };
    let (wide, high) = (
        u32::from(files.blocks_wide()),
        u32::from(files.blocks_high()),
    );
    if block >= wide.saturating_mul(high) {
        return;
    }
    let wrap = inner.ultima_live.definitions.get(&map).map(|definition| {
        let tiles = |wrap: u16, size: u16, files: u32| {
            let files =
                u16::try_from(files.saturating_mul(u32::from(TILES_PER_BLOCK))).unwrap_or(u16::MAX);
            u32::from(wrap.min(size).min(files) / TILES_PER_BLOCK)
        };
        (
            tiles(definition.wrap_width, definition.width, wide),
            tiles(definition.wrap_height, definition.height, high),
        )
    });
    let mut hashes = [0u16; LIVE_HASH_COUNT];
    for (hash, number) in hashes.iter_mut().zip(hash_blocks(block, wide, high, wrap)) {
        let Some(number) = number else {
            continue;
        };
        *hash = match inner.ultima_live.hashes.get(&(map, number)) {
            Some(known) => *known,
            None => {
                let sum = files.block_crc(u64::from(number)).unwrap_or_else(|error| {
                    tracing::warn!(%error, block = number, "an UltimaLive checksum failed");
                    0
                });
                inner.ultima_live.hashes.insert((map, number), sum);
                sum
            }
        };
    }
    inner
        .outbound
        .push_back(encode::ultima_live_hashes(block, map, &hashes));
}

/// Bytes as two lowercase hex digits each, the compact form `watch` sends
/// the changed blocks in.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The blocks the shard changed near the character on the map underfoot,
/// for a window to lay over its own map files, and the count of changes so
/// it knows when they moved.
pub(super) fn watch_value(inner: &Inner) -> Value {
    let map = inner.map_index();
    let here = inner.world.read().self_state.location;
    let (bx, by) = (
        u32::from(here.x / TILES_PER_BLOCK),
        u32::from(here.y / TILES_PER_BLOCK),
    );
    let high = inner
        .maps
        .get(&map)
        .map(|files| u32::from(files.blocks_high()));
    let blocks: Vec<Value> = match (inner.ultima_live.edits.get(&map), high) {
        (Some(edits), Some(high)) => edits
            .iter()
            .filter(|(block, _)| {
                let (x, y) = (*block / high, *block % high);
                x.abs_diff(bx) <= WATCH_BLOCK_REACH && y.abs_diff(by) <= WATCH_BLOCK_REACH
            })
            .map(|(block, edit)| {
                json!({
                    "block": block,
                    "changed": edit.changed,
                    "land": edit.land.as_deref().map(hex),
                    "statics": edit.statics.as_deref().map(hex),
                })
            })
            .collect(),
        _ => Vec::new(),
    };
    json!({
        "shard": inner.ultima_live.shard,
        "map": map,
        "revision": inner.ultima_live.revision,
        "blocks": blocks,
    })
}

#[cfg(test)]
mod tests {
    use super::super::relay_tests::test_session;
    use super::*;
    use uoterm_nav::client_data_dir_from_env;
    use uoterm_protocol::LIVE_LAND_BYTES;

    const SHARD: &str = "LiveShard";
    const FELUCCA: u8 = 0;
    /// A tile of Britain on the default map, and its block.
    const BRITAIN: Point3 = Point3 {
        x: 1496,
        y: 1628,
        z: 10,
    };

    fn logged_in(inner: &mut Inner) {
        on_packet(
            inner,
            &Inbound::UltimaLive(LiveMapEvent::Login {
                shard: SHARD.into(),
            }),
        );
    }

    #[test]
    fn a_hash_query_covers_the_five_by_five_blocks_column_by_column() {
        const WIDE: u32 = 10;
        const HIGH: u32 = 8;
        let middle = 4 * HIGH + 3;
        let blocks = hash_blocks(middle, WIDE, HIGH, None);
        assert_eq!(blocks[0], Some(2 * HIGH + 1));
        assert_eq!(blocks[12], Some(middle));
        assert_eq!(blocks[1], Some(2 * HIGH + 2), "the next slot goes down");
        assert_eq!(blocks[5], Some(3 * HIGH + 1), "then the next column");
        // At the corner the square wraps to the far sides of the map.
        let corner = hash_blocks(0, WIDE, HIGH, None);
        assert_eq!(corner[0], Some((WIDE - 2) * HIGH + (HIGH - 2)));
        // Inside the wrap area it wraps at the wrap size.
        let wrapped = hash_blocks(0, WIDE, HIGH, Some((6, 8)));
        assert_eq!(wrapped[0], Some(4 * HIGH + 6));
    }

    #[test]
    fn nothing_changes_before_the_shard_says_it_runs_ultima_live() {
        let mut inner = test_session();
        let land = Inbound::LiveTerrain {
            block: 1,
            map: FELUCCA,
            land: vec![0; LIVE_LAND_BYTES],
        };
        on_packet(&mut inner, &land);
        on_packet(
            &mut inner,
            &Inbound::UltimaLive(LiveMapEvent::HashQuery {
                block: 1,
                map: FELUCCA,
            }),
        );
        assert!(!inner.ultima_live.has_edits(FELUCCA));
        assert!(inner.outbound.is_empty());
        logged_in(&mut inner);
        on_packet(
            &mut inner,
            &Inbound::LiveTerrain {
                block: 1,
                map: FELUCCA + 1,
                land: vec![0; LIVE_LAND_BYTES],
            },
        );
        assert!(
            !inner.ultima_live.has_edits(FELUCCA + 1),
            "another map is left alone"
        );
        on_packet(&mut inner, &land);
        assert!(inner.ultima_live.has_edits(FELUCCA));
        assert_eq!(inner.ultima_live.revision, 1);
        on_packet(
            &mut inner,
            &Inbound::UltimaLive(LiveMapEvent::Login {
                shard: "../escape".into(),
            }),
        );
        assert_eq!(inner.ultima_live.shard, None, "a path is no shard name");
    }

    /// With the real client files: a changed block is walked and answered in
    /// its new form by this session, and the map every other session shares
    /// keeps the files.
    #[test]
    fn a_changed_block_is_walked_and_hashed_by_this_session_alone() {
        const RAISED: i8 = 50;
        let Some(dir) = client_data_dir_from_env() else {
            eprintln!("skipped: UOTERM_TEST_UOPATH is not set");
            return;
        };
        let mut inner = test_session();
        inner.uopath = Some(dir);
        inner.world.write().self_state.location = BRITAIN;
        inner.ensure_facet();
        let shared = inner.maps.get(&FELUCCA).cloned().expect("the facet opens");
        let high = u32::from(shared.blocks_high());
        let block =
            u32::from(BRITAIN.x / TILES_PER_BLOCK) * high + u32::from(BRITAIN.y / TILES_PER_BLOCK);
        let pristine = shared.block_crc(u64::from(block)).unwrap();
        logged_in(&mut inner);
        let mut land = Vec::new();
        for _ in 0..LIVE_LAND_BYTES / 3 {
            land.extend_from_slice(&3u16.to_le_bytes());
            land.push(RAISED as u8);
        }
        on_packet(
            &mut inner,
            &Inbound::LiveTerrain {
                block,
                map: FELUCCA,
                land,
            },
        );
        let own = inner.maps.get(&FELUCCA).cloned().unwrap();
        assert!(
            !Arc::ptr_eq(&own, &shared),
            "the session reads its own copy"
        );
        assert_eq!(own.column(BRITAIN.x, BRITAIN.y).land.north_west, RAISED);
        assert_ne!(shared.column(BRITAIN.x, BRITAIN.y).land.north_west, RAISED);
        on_packet(
            &mut inner,
            &Inbound::UltimaLive(LiveMapEvent::HashQuery {
                block,
                map: FELUCCA,
            }),
        );
        let answer = inner.outbound.pop_back().expect("the query is answered");
        let middle = 15 + 2 * (LIVE_HASH_COUNT / 2);
        let hash = u16::from_be_bytes([answer[middle], answer[middle + 1]]);
        assert_eq!(hash, own.block_crc(u64::from(block)).unwrap());
        assert_ne!(hash, pristine);
        let shown = watch_value(&inner);
        assert_eq!(shown["blocks"][0]["block"], block);
        assert_eq!(shown["blocks"][0]["changed"], 1);
        assert_eq!(
            shown["blocks"][0]["land"].as_str().unwrap().len(),
            2 * LIVE_LAND_BYTES
        );
        // A change of map form opens the copy again with every change on it.
        inner.map_variants.insert(
            FELUCCA,
            MapVariant {
                land_patches: 1,
                ..MapVariant::default()
            },
        );
        inner.ensure_facet();
        let reopened = inner.maps.get(&FELUCCA).cloned().unwrap();
        assert_eq!(
            reopened.column(BRITAIN.x, BRITAIN.y).land.north_west,
            RAISED
        );
    }
}
