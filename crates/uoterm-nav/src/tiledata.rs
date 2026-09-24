//! Every field of `tiledata.mul`: for each land tile its flags, texture and
//! name, and for each item graphic its flags, weight, quality, amount,
//! animation, hue, light, height and name.
//!
//! The file is a table of land records in groups of 32, then a table of item
//! records in groups of 32. Each group starts with a header no reader needs.
//! High Seas files hold 64-bit flags, older files 32-bit flags.

use std::path::Path;

use crate::mul::{
    read_file, slice_at, tiledata_is_hs, MapError, GROUP_HEADER, LAND_COUNT, LAND_GROUP,
    LAND_NAME_AFTER_FLAGS, LAND_RECORD_HS, LAND_RECORD_OLD, STATIC_ANIM_BYTES_AFTER_FLAGS,
    STATIC_GROUP, STATIC_HEIGHT_BYTES_AFTER_FLAGS, STATIC_RECORD_HS, STATIC_RECORD_OLD,
    TILEDATA_FLAGS_HS, TILEDATA_FLAGS_OLD, TILEDATA_NAME, TILE_NAME_LEN,
};
use crate::tiles::TileFlagSet;

/// Where each field of an item record starts, counted from the end of its
/// flags.
const ITEM_WEIGHT_AFTER_FLAGS: usize = 0;
const ITEM_QUALITY_AFTER_FLAGS: usize = 1;
const ITEM_QUANTITY_AFTER_FLAGS: usize = 2;
const ITEM_HUE_AFTER_FLAGS: usize = 8;
const ITEM_LIGHT_AFTER_FLAGS: usize = 10;
const TEXTURE_AFTER_FLAGS: usize = 0;
const WORD: usize = 2;
const DWORD: usize = 4;
const QWORD: usize = 8;

/// One land tile of the tiledata file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LandTile {
    pub flags: TileFlagSet,
    /// The picture of `texmaps.mul` the land takes on a slope. Zero is none.
    pub texture_id: u16,
    pub name: String,
}

/// One item graphic of the tiledata file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemTile {
    pub flags: TileFlagSet,
    pub weight: u8,
    /// The layer a wearable item takes, and the light shape a light source
    /// on the map gives.
    pub quality: u8,
    pub quantity: u32,
    pub anim_id: u16,
    pub hue: u16,
    /// The light shape the item gives while a mobile holds it.
    pub light_index: u16,
    pub height: u8,
    pub name: String,
}

/// The whole tiledata file.
#[derive(Clone, Debug, Default)]
pub struct TileData {
    land: Vec<LandTile>,
    items: Vec<ItemTile>,
}

/// The shape of the records of one file layout.
struct Layout {
    flags: usize,
    land_record: usize,
    item_record: usize,
}

impl Layout {
    fn of(len: usize) -> Self {
        if tiledata_is_hs(len) {
            Self {
                flags: TILEDATA_FLAGS_HS,
                land_record: LAND_RECORD_HS,
                item_record: STATIC_RECORD_HS,
            }
        } else {
            Self {
                flags: TILEDATA_FLAGS_OLD,
                land_record: LAND_RECORD_OLD,
                item_record: STATIC_RECORD_OLD,
            }
        }
    }

    fn flags(&self, record: &[u8]) -> TileFlagSet {
        TileFlagSet(match self.flags {
            QWORD => u64_at(record, 0),
            _ => u64::from(u32_at(record, 0)),
        })
    }
}

fn u16_at(data: &[u8], at: usize) -> u16 {
    slice_at(data, at, WORD).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
}

fn u32_at(data: &[u8], at: usize) -> u32 {
    slice_at(data, at, DWORD).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

fn u64_at(data: &[u8], at: usize) -> u64 {
    slice_at(data, at, QWORD).map_or(0, |b| {
        u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
    })
}

/// The NUL-padded latin1 name that starts at `at` in a record.
fn name_at(record: &[u8], at: usize) -> String {
    let raw = record.get(at..).unwrap_or_default();
    let raw = &raw[..raw.len().min(TILE_NAME_LEN)];
    let len = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    raw[..len].iter().map(|&b| char::from(b)).collect()
}

/// The records of one table from `at`, in groups of `group` records that
/// each start with a header. The table ends after `limit` records or where
/// the file ends. Returns the records and where the table ends.
fn table(
    data: &[u8],
    mut at: usize,
    record: usize,
    group: usize,
    limit: usize,
) -> (Vec<&[u8]>, usize) {
    let mut records = Vec::new();
    while records.len() < limit {
        let header = if records.len() % group == 0 {
            GROUP_HEADER
        } else {
            0
        };
        let Some(bytes) = slice_at(data, at + header, record) else {
            break;
        };
        records.push(bytes);
        at += header + record;
    }
    (records, at)
}

impl TileData {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let path = uopath.as_ref().join(TILEDATA_NAME);
        if !path.exists() {
            return Err(MapError::Missing(TILEDATA_NAME));
        }
        Self::load(&path)
    }

    pub fn load(path: &Path) -> Result<Self, MapError> {
        Ok(Self::parse(&read_file(path)?))
    }

    /// Reads as much of the file as is whole. A cut file gives fewer records.
    pub(crate) fn parse(data: &[u8]) -> Self {
        let layout = Layout::of(data.len());
        let (land, end) = table(data, 0, layout.land_record, LAND_GROUP, LAND_COUNT);
        let land = land
            .into_iter()
            .map(|record| LandTile {
                flags: layout.flags(record),
                texture_id: u16_at(record, layout.flags + TEXTURE_AFTER_FLAGS),
                name: name_at(record, layout.flags + LAND_NAME_AFTER_FLAGS),
            })
            .collect();
        let (items, _) = table(data, end, layout.item_record, STATIC_GROUP, usize::MAX);
        let items = items
            .into_iter()
            .map(|record| {
                let field = |after: usize| layout.flags + after;
                ItemTile {
                    flags: layout.flags(record),
                    weight: record[field(ITEM_WEIGHT_AFTER_FLAGS)],
                    quality: record[field(ITEM_QUALITY_AFTER_FLAGS)],
                    quantity: u32_at(record, field(ITEM_QUANTITY_AFTER_FLAGS)),
                    anim_id: u16_at(record, field(STATIC_ANIM_BYTES_AFTER_FLAGS)),
                    hue: u16_at(record, field(ITEM_HUE_AFTER_FLAGS)),
                    light_index: u16_at(record, field(ITEM_LIGHT_AFTER_FLAGS)),
                    height: record[field(STATIC_HEIGHT_BYTES_AFTER_FLAGS)],
                    name: name_at(record, layout.item_record - TILE_NAME_LEN),
                }
            })
            .collect();
        Self { land, items }
    }

    /// The record of one land tile, or None past the end of the file.
    pub fn land(&self, land_id: u16) -> Option<&LandTile> {
        self.land.get(usize::from(land_id))
    }

    /// The record of one item graphic, or None past the end of the file.
    pub fn item(&self, graphic: u16) -> Option<&ItemTile> {
        self.items.get(usize::from(graphic))
    }

    pub fn land_count(&self) -> usize {
        self.land.len()
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_ITEM_GROUPS: usize = 1;
    const LAND_TEXTURE: u16 = 0x0123;
    const LAND_FLAGS: u64 = 0x0000_0080;
    const ITEM_ID: usize = 3;
    const ITEM_FLAGS: u64 = 0x0001_0080_0000;
    const ITEM_WEIGHT: u8 = 7;
    const ITEM_QUALITY: u8 = 29;
    const ITEM_QUANTITY: u32 = 0x0102_0304;
    const ITEM_ANIM: u16 = 0x0456;
    const ITEM_HUE: u16 = 0x0789;
    const ITEM_LIGHT: u16 = 0x000A;
    const ITEM_HEIGHT: u8 = 12;

    fn put_name(record: &mut [u8], name: &str) {
        let at = record.len() - TILE_NAME_LEN;
        record[at..at + name.len()].copy_from_slice(name.as_bytes());
    }

    /// A whole file of one layout: every land tile, then one group of items.
    fn file(hs: bool) -> Vec<u8> {
        let layout = if hs {
            Layout {
                flags: TILEDATA_FLAGS_HS,
                land_record: LAND_RECORD_HS,
                item_record: STATIC_RECORD_HS,
            }
        } else {
            Layout {
                flags: TILEDATA_FLAGS_OLD,
                land_record: LAND_RECORD_OLD,
                item_record: STATIC_RECORD_OLD,
            }
        };
        let flags = |value: u64| -> Vec<u8> { value.to_le_bytes()[..layout.flags].to_vec() };
        let mut data = Vec::new();
        for id in 0..LAND_COUNT {
            if id % LAND_GROUP == 0 {
                data.extend([0u8; GROUP_HEADER]);
            }
            let mut record = vec![0u8; layout.land_record];
            if id == 1 {
                record[..layout.flags].copy_from_slice(&flags(LAND_FLAGS));
                record[layout.flags..layout.flags + WORD]
                    .copy_from_slice(&LAND_TEXTURE.to_le_bytes());
                put_name(&mut record, "water");
            }
            data.extend(record);
        }
        for id in 0..STATIC_GROUP * TEST_ITEM_GROUPS {
            if id % STATIC_GROUP == 0 {
                data.extend([0u8; GROUP_HEADER]);
            }
            let mut record = vec![0u8; layout.item_record];
            if id == ITEM_ID {
                let f = layout.flags;
                let wide = if hs {
                    ITEM_FLAGS
                } else {
                    ITEM_FLAGS & u64::from(u32::MAX)
                };
                record[..f].copy_from_slice(&flags(wide));
                record[f] = ITEM_WEIGHT;
                record[f + 1] = ITEM_QUALITY;
                record[f + 2..f + 6].copy_from_slice(&ITEM_QUANTITY.to_le_bytes());
                record[f + 6..f + 8].copy_from_slice(&ITEM_ANIM.to_le_bytes());
                record[f + 8..f + 10].copy_from_slice(&ITEM_HUE.to_le_bytes());
                record[f + 10..f + 12].copy_from_slice(&ITEM_LIGHT.to_le_bytes());
                record[f + 12] = ITEM_HEIGHT;
                put_name(&mut record, "lantern");
            }
            data.extend(record);
        }
        data
    }

    fn check(hs: bool) {
        let tiles = TileData::parse(&file(hs));
        assert_eq!(tiles.land_count(), LAND_COUNT);
        assert_eq!(tiles.item_count(), STATIC_GROUP * TEST_ITEM_GROUPS);
        let water = tiles.land(1).unwrap();
        assert!(water.flags.contains(TileFlagSet::WET));
        assert_eq!(water.texture_id, LAND_TEXTURE);
        assert_eq!(water.name, "water");
        let lantern = tiles.item(ITEM_ID as u16).unwrap();
        assert!(lantern.flags.contains(TileFlagSet::LIGHT_SOURCE));
        assert_eq!(lantern.flags.contains(TileFlagSet::ALPHA_BLEND), hs);
        assert_eq!(lantern.weight, ITEM_WEIGHT);
        assert_eq!(lantern.quality, ITEM_QUALITY);
        assert_eq!(lantern.quantity, ITEM_QUANTITY);
        assert_eq!(lantern.anim_id, ITEM_ANIM);
        assert_eq!(lantern.hue, ITEM_HUE);
        assert_eq!(lantern.light_index, ITEM_LIGHT);
        assert_eq!(lantern.height, ITEM_HEIGHT);
        assert_eq!(lantern.name, "lantern");
        assert!(tiles
            .item((STATIC_GROUP * TEST_ITEM_GROUPS) as u16)
            .is_none());
    }

    #[test]
    fn every_field_reads_in_the_high_seas_layout() {
        check(true);
    }

    #[test]
    fn every_field_reads_in_the_old_layout() {
        check(false);
    }

    #[test]
    fn a_cut_file_gives_fewer_records_and_no_panic() {
        let data = file(true);
        let tiles = TileData::parse(&data[..data.len() / 2]);
        assert!(tiles.land_count() < LAND_COUNT);
        assert_eq!(tiles.item_count(), 0);
        assert_eq!(TileData::parse(&[]).land_count(), 0);
    }

    #[test]
    fn the_real_file_knows_water_and_a_lantern_when_client_files_are_here() {
        const LAND_WATER: u16 = 0x00A8;
        const ITEM_LANTERN: u16 = 0x0A22;
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        let tiles = TileData::open(dir).unwrap();
        assert_eq!(tiles.land_count(), LAND_COUNT);
        assert!(tiles
            .land(LAND_WATER)
            .unwrap()
            .flags
            .contains(TileFlagSet::WET));
        let lantern = tiles.item(ITEM_LANTERN).unwrap();
        assert!(lantern.flags.contains(TileFlagSet::LIGHT_SOURCE));
        assert!(lantern.name.contains("lantern"));
    }
}
