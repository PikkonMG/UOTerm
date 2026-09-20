//! The pictures of the land and of the items, from `artLegacyMUL.uop` or from
//! `art.mul` with `artidx.mul`.
//!
//! A picture is kept as the client files hold it: one 15-bit color for each
//! pixel. A hue changes those colors before they become RGBA, so the hue
//! step works on the file colors and not on the screen colors.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use crate::hues::HueRamp;
use crate::mul::{
    capped_len, first_existing, idx_entries, is_uop_path, read_file, slice_at, MapError,
};
use crate::uop::{decompress, load_art_entries, UopIndex};

pub const ART_UOP_NAME: &str = "artLegacyMUL.uop";
pub const ART_MUL_NAME: &str = "art.mul";
pub const ART_IDX_NAME: &str = "artidx.mul";
/// The side of a land picture. The land is a diamond inside this square.
pub const LAND_ART_SIDE: usize = 44;
/// Item pictures follow the land pictures in the art files.
pub const ITEM_ART_BASE: u32 = 0x4000;
/// No real item picture is wider or taller than this. A larger number is a
/// damaged record.
pub const ITEM_ART_MAX_SIDE: usize = 1024;
/// This bit marks a pixel that is drawn. A clear pixel has the value zero.
pub const PIXEL_DRAWN: u16 = 0x8000;

const HALF_LAND_ROWS: usize = LAND_ART_SIDE / 2;
const WORD: usize = 2;
const ITEM_HEADER_BYTES: usize = 8;
const ITEM_WIDTH_AT: usize = 4;
const ITEM_HEIGHT_AT: usize = 6;
const COLOR_CHANNEL_MAX: u32 = 31;
const BYTE_MAX: u32 = 255;
const RED_SHIFT: u16 = 10;
const GREEN_SHIFT: u16 = 5;
const CHANNEL_MASK: u16 = 0x1F;
const RGBA_BYTES: usize = 4;

/// One picture in the colors of the client files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtPixels {
    pub width: usize,
    pub height: usize,
    /// Row order. A pixel with [`PIXEL_DRAWN`] clear is not drawn.
    pub colors: Vec<u16>,
}

impl ArtPixels {
    fn clear(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            colors: vec![0; width * height],
        }
    }

    /// The picture as RGBA bytes, in the colors of `ramp` when one is given.
    pub fn rgba(&self, ramp: Option<HueRamp<'_>>) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.colors.len() * RGBA_BYTES);
        for &color in &self.colors {
            if color & PIXEL_DRAWN == 0 {
                out.extend_from_slice(&[0; RGBA_BYTES]);
                continue;
            }
            let color = ramp.map_or(color, |r| r.apply(color));
            out.extend_from_slice(&[
                channel(color >> RED_SHIFT),
                channel(color >> GREEN_SHIFT),
                channel(color),
                u8::MAX,
            ]);
        }
        out
    }
}

/// One 5-bit color channel as a byte.
pub(crate) fn channel(bits: u16) -> u8 {
    let value = u32::from(bits & CHANNEL_MASK);
    ((value * BYTE_MAX + COLOR_CHANNEL_MAX / 2) / COLOR_CHANNEL_MAX) as u8
}

pub struct ArtData {
    file: Mutex<File>,
    entries: Vec<Option<UopIndex>>,
}

impl ArtData {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let dir = uopath.as_ref();
        let path =
            first_existing(dir, &[ART_UOP_NAME, ART_MUL_NAME]).ok_or(MapError::Missing("art"))?;
        let entries = if is_uop_path(&path) {
            load_art_entries(&path)?
        } else {
            let idx = dir.join(ART_IDX_NAME);
            if !idx.exists() {
                return Err(MapError::Missing(ART_IDX_NAME));
            }
            idx_entries(&read_file(&idx)?)
        };
        Ok(Self {
            file: Mutex::new(File::open(&path)?),
            entries,
        })
    }

    /// The picture of one land tile, or None when the files hold none.
    pub fn land(&self, land_id: u16) -> Option<ArtPixels> {
        decode_land(&self.read(u32::from(land_id))?)
    }

    /// The picture of one item graphic, or None when the files hold none.
    pub fn item(&self, graphic: u16) -> Option<ArtPixels> {
        decode_item(&self.read(ITEM_ART_BASE + u32::from(graphic))?)
    }

    fn read(&self, index: u32) -> Option<Vec<u8>> {
        let entry = self.entries.get(index as usize)?.as_ref()?;
        let mut file = self.file.lock().ok()?;
        let file_len = file.metadata().ok()?.len();
        let mut raw = vec![0u8; capped_len(file_len, entry.offset, entry.compressed_len)];
        file.seek(SeekFrom::Start(entry.offset)).ok()?;
        file.read_exact(&mut raw).ok()?;
        decompress(&raw, entry.compression, entry.decompressed_len).ok()
    }
}

fn word_at(data: &[u8], at: usize) -> Option<u16> {
    slice_at(data, at, WORD).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

/// A land picture has no header. Its rows grow by two pixels down to the
/// middle of the diamond and then shrink by two.
fn decode_land(data: &[u8]) -> Option<ArtPixels> {
    let mut art = ArtPixels::clear(LAND_ART_SIDE, LAND_ART_SIDE);
    let mut at = 0usize;
    for row in 0..LAND_ART_SIDE {
        let inset = if row < HALF_LAND_ROWS {
            HALF_LAND_ROWS - 1 - row
        } else {
            row - HALF_LAND_ROWS
        };
        for x in inset..LAND_ART_SIDE - inset {
            art.colors[row * LAND_ART_SIDE + x] = word_at(data, at)? | PIXEL_DRAWN;
            at += WORD;
        }
    }
    Some(art)
}

/// An item picture is a header, one start offset for each row, then runs.
/// Each run is a gap, a length, and that many colors. A run with no gap and
/// no length ends the row.
fn decode_item(data: &[u8]) -> Option<ArtPixels> {
    let width = usize::from(word_at(data, ITEM_WIDTH_AT)?);
    let height = usize::from(word_at(data, ITEM_HEIGHT_AT)?);
    if width == 0 || height == 0 || width > ITEM_ART_MAX_SIDE || height > ITEM_ART_MAX_SIDE {
        return None;
    }
    let runs_start = ITEM_HEADER_BYTES + height * WORD;
    let mut art = ArtPixels::clear(width, height);
    for row in 0..height {
        let row_start = usize::from(word_at(data, ITEM_HEADER_BYTES + row * WORD)?);
        let mut at = runs_start + row_start * WORD;
        let mut x = 0usize;
        loop {
            let gap = usize::from(word_at(data, at)?);
            let run = usize::from(word_at(data, at + WORD)?);
            at += WORD * 2;
            if gap == 0 && run == 0 {
                break;
            }
            x += gap;
            if x + run > width {
                return None;
            }
            for _ in 0..run {
                let color = word_at(data, at)?;
                at += WORD;
                if color != 0 {
                    art.colors[row * width + x] = color | PIXEL_DRAWN;
                }
                x += 1;
            }
        }
    }
    Some(art)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: u16 = 0x7FFF;
    const RED: u16 = 0x7C00;
    const LAND_PIXELS: usize = 1012;

    fn words(values: &[u16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn land_is_a_diamond() {
        let art = decode_land(&words(&[WHITE; LAND_PIXELS])).unwrap();
        let drawn = art.colors.iter().filter(|c| **c & PIXEL_DRAWN != 0).count();
        assert_eq!(drawn, LAND_PIXELS);
        assert_eq!(art.colors[0], 0);
        assert_ne!(art.colors[HALF_LAND_ROWS - 1], 0);
        assert_ne!(art.colors[HALF_LAND_ROWS * LAND_ART_SIDE], 0);
    }

    #[test]
    fn short_land_is_none() {
        assert!(decode_land(&words(&[WHITE; LAND_PIXELS - 1])).is_none());
    }

    #[test]
    fn item_runs_fill_rows() {
        // 3 wide, 2 high. Row 0: gap 1, one red pixel. Row 1: two white pixels.
        let mut data = words(&[0, 0, 3, 2]);
        data.extend(words(&[0, 5]));
        data.extend(words(&[1, 1, RED, 0, 0]));
        data.extend(words(&[0, 2, WHITE, WHITE, 0, 0]));
        let art = decode_item(&data).unwrap();
        assert_eq!((art.width, art.height), (3, 2));
        assert_eq!(
            art.colors,
            vec![
                0,
                RED | PIXEL_DRAWN,
                0,
                WHITE | PIXEL_DRAWN,
                WHITE | PIXEL_DRAWN,
                0
            ]
        );
        let rgba = art.rgba(None);
        assert_eq!(&rgba[4..8], &[u8::MAX, 0, 0, u8::MAX]);
        assert_eq!(&rgba[0..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn item_run_past_the_edge_is_none() {
        let mut data = words(&[0, 0, 2, 1]);
        data.extend(words(&[0]));
        data.extend(words(&[1, 2, WHITE, WHITE, 0, 0]));
        assert!(decode_item(&data).is_none());
    }

    #[test]
    fn real_files_give_grass_and_a_tree() {
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        const LAND_GRASS: u16 = 3;
        const ITEM_TREE: u16 = 0x0CDA;
        let art = ArtData::open(dir).unwrap();
        assert_eq!(art.land(LAND_GRASS).unwrap().width, LAND_ART_SIDE);
        assert!(art.item(ITEM_TREE).unwrap().height > LAND_ART_SIDE);
    }
}
