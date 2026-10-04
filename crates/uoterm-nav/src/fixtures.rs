//! Small client files written by hand, for the tests of this crate and of
//! the crates that use it. Each test writes them into its own folder.

use std::fs;
use std::path::Path;

use crate::art::{ART_IDX_NAME, ART_MUL_NAME, ITEM_ART_BASE, PIXEL_DRAWN};
use crate::fonts::{glyph_index, ASCII_GLYPH_COUNT, FONTS_NAME};
use crate::hues::HUES_NAME;
use crate::mul::{
    block_index, BLOCK_BYTES, BLOCK_HEADER, CELL_BYTES, GROUP_HEADER, IDX_EMPTY, IDX_RECORD,
    LAND_COUNT, LAND_GROUP, LAND_NAME_AFTER_FLAGS, LAND_RECORD_OLD, STAIDX_RECORD,
    STATIC_HEIGHT_BYTES_AFTER_FLAGS, STATIC_RECORD, STATIC_RECORD_OLD, TILEDATA_FLAGS_OLD,
    TILEDATA_NAME, TILE_NAME_LEN,
};
use crate::tiles::{TILE_IMPASSABLE, TILE_SURFACE};
use crate::unifont::UNIFONT_NAME;

/// The mini map is this many blocks wide and this many high.
pub const MINI_MAP_BLOCKS: u16 = 2;
/// The land id of the block east of the first, and of the block south of it.
pub const LAND_ID_BLOCK_1_0: u16 = 2;
pub const LAND_ID_BLOCK_0_1: u16 = 1;
/// One land name per land id, so a test can prove a name comes from the
/// record of that id and not from a neighbour.
pub const FIXTURE_LAND_NAMES: [&str; 4] = ["dirt", "grass", "cave floor", "forest"];
/// The mini map holds one static: an impassable wall in the first block.
pub const FIXTURE_WALL_GRAPHIC: u16 = 0;
pub const FIXTURE_WALL_NAME: &str = "stone wall";
pub const FIXTURE_WALL_HEIGHT: u8 = 20;
pub const FIXTURE_WALL_Z: i8 = 0;
pub const FIXTURE_WALL_CX: u16 = 4;
pub const FIXTURE_WALL_CY: u16 = 0;
/// A second item of the tiledata, so a test can prove a height and a flag
/// come from the record of the graphic it asked about.
pub const FIXTURE_FLOOR_GRAPHIC: u16 = 1;
pub const FIXTURE_FLOOR_NAME: &str = "wooden floor";
pub const FIXTURE_FLOOR_HEIGHT: u8 = 5;
/// The height of every glyph of [`ascii_font_bytes`], and the color of
/// its top row.
pub const ASCII_FIXTURE_HEIGHT: u8 = 2;
pub const ASCII_FIXTURE_INK: u16 = 0x7FFF;
/// The width of every glyph of the font of [`write_small_fonts`].
pub const SMALL_FONT_WIDTH: u8 = 3;
/// The one drawn pixel of item 1 of [`write_two_items`], as an
/// [`crate::ArtPixels`] holds it.
pub const TWO_ITEMS_RED: u16 = RED | PIXEL_DRAWN;

const MAP_NAME: &str = "map0.mul";
const STATICS_NAME: &str = "statics0.mul";
const STAIDX_NAME: &str = "staidx0.mul";
/// The bytes of a tiledata record that follow the flags field.
const LAND_REST_LEN: usize = LAND_RECORD_OLD - TILEDATA_FLAGS_OLD;
const STATIC_REST_LEN: usize = STATIC_RECORD_OLD - TILEDATA_FLAGS_OLD;
const STATIC_NAME_IN_REST: usize = STATIC_REST_LEN - TILE_NAME_LEN;
const RED: u16 = 0x7C00;
/// The header of a written text database: a version field and a language
/// field, neither of which any reader uses.
const CLILOC_VERSION: u32 = 2;
const CLILOC_LANGUAGE: u16 = 1;
/// Every record of the client files carries this flag byte.
const CLILOC_FLAG: u8 = 0;

/// Every block of the mini map gets its own land id, so a test can prove
/// which block a coordinate landed in.
pub fn fixture_land_id(bx: u16, by: u16) -> u8 {
    (bx * MINI_MAP_BLOCKS + by) as u8
}

/// The bytes of the `map0.mul` of the mini map: flat land at height zero.
pub fn mini_map() -> Vec<u8> {
    let blocks = usize::from(MINI_MAP_BLOCKS * MINI_MAP_BLOCKS);
    let mut map = vec![0u8; BLOCK_BYTES * blocks];
    for by in 0..MINI_MAP_BLOCKS {
        for bx in 0..MINI_MAP_BLOCKS {
            let block = block_index(MINI_MAP_BLOCKS, bx, by) as usize;
            let base = block * BLOCK_BYTES + BLOCK_HEADER;
            for cell in map[base..base + BLOCK_BYTES - BLOCK_HEADER].chunks_exact_mut(CELL_BYTES) {
                cell[0] = fixture_land_id(bx, by);
            }
        }
    }
    map
}

/// Writes a client of two by two blocks: the land of [`mini_map`], the
/// wall in the first block, and the tiledata of [`mini_tiledata`].
pub fn write_mini_client(dir: &Path) {
    fs::write(dir.join(MAP_NAME), mini_map()).unwrap();
    let mut statics = vec![0u8; STATIC_RECORD];
    statics[0..2].copy_from_slice(&FIXTURE_WALL_GRAPHIC.to_le_bytes());
    statics[2] = FIXTURE_WALL_CX as u8;
    statics[3] = FIXTURE_WALL_CY as u8;
    statics[4] = FIXTURE_WALL_Z as u8;
    fs::write(dir.join(STATICS_NAME), statics).unwrap();
    let blocks = usize::from(MINI_MAP_BLOCKS * MINI_MAP_BLOCKS);
    let mut staidx = vec![0u8; STAIDX_RECORD * blocks];
    staidx[4..8].copy_from_slice(&(STATIC_RECORD as u32).to_le_bytes());
    for record in staidx.chunks_exact_mut(STAIDX_RECORD).skip(1) {
        record[0..4].copy_from_slice(&IDX_EMPTY.to_le_bytes());
        record[4..8].copy_from_slice(&IDX_EMPTY.to_le_bytes());
    }
    fs::write(dir.join(STAIDX_NAME), staidx).unwrap();
    fs::write(dir.join(TILEDATA_NAME), mini_tiledata()).unwrap();
}

/// Writes a NUL-padded latin1 name the way a tiledata record stores it.
fn put_tile_name(rest: &mut [u8], offset: usize, name: &str) {
    let bytes = name.as_bytes();
    assert!(bytes.len() <= TILE_NAME_LEN, "fixture name is too long");
    rest[offset..offset + bytes.len()].copy_from_slice(bytes);
}

/// An old-layout tiledata: every land tile, named by
/// [`FIXTURE_LAND_NAMES`], then one group of items with the wall and the
/// floor.
pub fn mini_tiledata() -> Vec<u8> {
    let land_groups = LAND_COUNT / LAND_GROUP;
    let mut data = Vec::with_capacity(
        land_groups * (GROUP_HEADER + LAND_GROUP * LAND_RECORD_OLD)
            + GROUP_HEADER
            + LAND_GROUP * STATIC_RECORD_OLD,
    );
    for group in 0..land_groups {
        data.extend_from_slice(&0u32.to_le_bytes());
        for slot in 0..LAND_GROUP {
            data.extend_from_slice(&0u32.to_le_bytes());
            let mut rest = [0u8; LAND_REST_LEN];
            if let Some(&name) = FIXTURE_LAND_NAMES.get(group * LAND_GROUP + slot) {
                put_tile_name(&mut rest, LAND_NAME_AFTER_FLAGS, name);
            }
            data.extend_from_slice(&rest);
        }
    }
    data.extend_from_slice(&0u32.to_le_bytes());
    for i in 0..LAND_GROUP {
        let (flags, height, name) = match i as u16 {
            FIXTURE_WALL_GRAPHIC => (TILE_IMPASSABLE, FIXTURE_WALL_HEIGHT, FIXTURE_WALL_NAME),
            FIXTURE_FLOOR_GRAPHIC => (TILE_SURFACE, FIXTURE_FLOOR_HEIGHT, FIXTURE_FLOOR_NAME),
            _ => (0, 0, ""),
        };
        data.extend_from_slice(&flags.to_le_bytes());
        let mut rest = [0u8; STATIC_REST_LEN];
        rest[STATIC_HEIGHT_BYTES_AFTER_FLAGS] = height;
        put_tile_name(&mut rest, STATIC_NAME_IN_REST, name);
        data.extend_from_slice(&rest);
    }
    data
}

fn words(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

/// Writes an `art.mul` and an `artidx.mul` that hold two items: item 0,
/// one red pixel, and item 1, a clear pixel then a red one on a row of
/// two.
pub fn write_two_items(dir: &Path) {
    let mut item_one = words(&[0, 0, 2, 1]);
    item_one.extend(words(&[0]));
    item_one.extend(words(&[1, 1, RED, 0, 0]));
    let item_zero = words(&[0, 0, 1, 1, 0, 0, 1, RED, 0, 0]);
    let mut mul = item_zero.clone();
    mul.extend(&item_one);
    let entries = ITEM_ART_BASE as usize + 2;
    let mut idx = Vec::with_capacity(entries * IDX_RECORD);
    for index in 0..entries {
        let (offset, len) = match index.checked_sub(ITEM_ART_BASE as usize) {
            Some(0) => (0, item_zero.len()),
            Some(_) => (item_zero.len(), item_one.len()),
            None => (IDX_EMPTY as usize, 0),
        };
        idx.extend((offset as u32).to_le_bytes());
        idx.extend((len as u32).to_le_bytes());
        idx.extend(0u32.to_le_bytes());
    }
    fs::write(dir.join(ART_MUL_NAME), mul).unwrap();
    fs::write(dir.join(ART_IDX_NAME), idx).unwrap();
}

/// Writes a plain text database the way the client files lay one out, and
/// gives back the bytes so a test can measure them.
pub fn write_cliloc(path: &Path, messages: &[(u32, &str)]) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&CLILOC_VERSION.to_le_bytes());
    data.extend_from_slice(&CLILOC_LANGUAGE.to_le_bytes());
    for (number, text) in messages {
        data.extend_from_slice(&number.to_le_bytes());
        data.push(CLILOC_FLAG);
        data.extend_from_slice(&(text.len() as u16).to_le_bytes());
        data.extend_from_slice(text.as_bytes());
    }
    fs::write(path, &data).unwrap();
    data
}

/// One ASCII font where every glyph is `width` wide and
/// [`ASCII_FIXTURE_HEIGHT`] high with a top row of ink, and the glyph of
/// `!` is empty.
pub fn ascii_font_bytes(width: u8) -> Vec<u8> {
    let mut data = vec![0u8];
    for index in 0..ASCII_GLYPH_COUNT {
        let empty = index == glyph_index('!');
        data.extend([width, ASCII_FIXTURE_HEIGHT, 0]);
        for row in 0..ASCII_FIXTURE_HEIGHT {
            for _ in 0..width {
                let color = if row == 0 && !empty {
                    ASCII_FIXTURE_INK
                } else {
                    0
                };
                data.extend(color.to_le_bytes());
            }
        }
    }
    data
}

/// Writes the font files a client must have: one ASCII font of
/// [`ascii_font_bytes`] with glyphs [`SMALL_FONT_WIDTH`] wide, and an
/// empty Unicode font and hue file.
pub fn write_small_fonts(dir: &Path) {
    fs::write(dir.join(FONTS_NAME), ascii_font_bytes(SMALL_FONT_WIDTH)).unwrap();
    fs::write(dir.join(UNIFONT_NAME), []).unwrap();
    fs::write(dir.join(HUES_NAME), []).unwrap();
}
