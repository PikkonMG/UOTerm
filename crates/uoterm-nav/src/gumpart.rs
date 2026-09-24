//! The pictures of gumps: the stone and paper backgrounds of a dialog, its
//! buttons, its check boxes. They come from the gump art package of the
//! client, or from `gumpart.mul` with `gumpidx.mul` on a client without the
//! package. A record is one offset for each row, then the rows as runs of
//! one color. The package puts the width and the height before the rows; the
//! MUL index keeps them in its extra number.

use std::path::Path;

use crate::art::{ArtPixels, PIXEL_DRAWN};
use crate::mul::{first_existing, idx_entries, idx_sizes, read_file, slice_at, MapError};
use crate::uop::{load_gump_entries, Package};

pub const GUMP_UOP_NAME: &str = "gumpartLegacyMUL.uop";
/// The MUL files, by the names the clients spell them with.
pub const GUMP_MUL_NAMES: [&str; 2] = ["gumpart.mul", "Gumpart.mul"];
pub const GUMP_IDX_NAMES: [&str; 2] = ["gumpidx.mul", "Gumpidx.mul"];
/// No real gump is larger than this. A larger number is a damaged record.
pub const GUMP_MAX_SIDE: usize = 2048;

const DWORD: usize = 4;
const WORD: usize = 2;
/// The package puts the width and the height before the rows.
const SIZE_BYTES: usize = DWORD * 2;
const RUN_BYTES: usize = WORD * 2;

pub struct GumpArt {
    package: Package,
    /// The size of each gump the MUL index names. None for the package,
    /// which keeps the size in each record.
    mul_sizes: Option<Vec<Option<(usize, usize)>>>,
}

impl GumpArt {
    /// Opens the gump art package, or the MUL files when the client has no
    /// package.
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let dir = uopath.as_ref();
        let uop = dir.join(GUMP_UOP_NAME);
        if uop.exists() {
            return Ok(Self {
                package: Package::new(&uop, load_gump_entries(&uop)?)?,
                mul_sizes: None,
            });
        }
        let mul = first_existing(dir, &GUMP_MUL_NAMES).ok_or(MapError::Missing(GUMP_UOP_NAME))?;
        let idx =
            first_existing(dir, &GUMP_IDX_NAMES).ok_or(MapError::Missing(GUMP_IDX_NAMES[0]))?;
        let idx = read_file(&idx)?;
        let entries = idx_entries(&idx);
        let sizes = entries
            .iter()
            .zip(idx_sizes(&idx))
            .map(|(entry, size)| entry.and(checked_size(size)))
            .collect();
        Ok(Self {
            package: Package::new(&mul, entries)?,
            mul_sizes: Some(sizes),
        })
    }

    /// The picture of one gump id, or None when the files hold none.
    pub fn gump(&self, gump_id: u16) -> Option<ArtPixels> {
        let data = self.package.read(u32::from(gump_id))?;
        match &self.mul_sizes {
            Some(sizes) => {
                let (width, height) = (*sizes.get(usize::from(gump_id))?)?;
                decode_rows(&data, width, height)
            }
            None => decode_gump(&data),
        }
    }

    /// The width and the height of one gump, or None when the files hold
    /// none. The MUL files answer from the index alone; the package answers
    /// from the head of the record, with no rows decoded.
    pub fn gump_size(&self, gump_id: u16) -> Option<(usize, usize)> {
        match &self.mul_sizes {
            Some(sizes) => *sizes.get(usize::from(gump_id))?,
            None => package_size(&self.package.read(u32::from(gump_id))?),
        }
    }
}

/// A size no real gump has is None.
fn checked_size((width, height): (usize, usize)) -> Option<(usize, usize)> {
    let fits = |side: usize| (1..=GUMP_MAX_SIDE).contains(&side);
    (fits(width) && fits(height)).then_some((width, height))
}

fn package_size(data: &[u8]) -> Option<(usize, usize)> {
    checked_size((dword_at(data, 0)?, dword_at(data, DWORD)?))
}

fn dword_at(data: &[u8], at: usize) -> Option<usize> {
    slice_at(data, at, DWORD).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

fn word_at(data: &[u8], at: usize) -> Option<u16> {
    slice_at(data, at, WORD).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

/// A record of the package: the size, then the rows.
fn decode_gump(data: &[u8]) -> Option<ArtPixels> {
    let (width, height) = package_size(data)?;
    decode_rows(data.get(SIZE_BYTES..)?, width, height)
}

/// The offsets of the rows count in 4-byte steps from the first offset. A
/// row ends where the next one starts. A run of color zero is not drawn.
fn decode_rows(rows: &[u8], width: usize, height: usize) -> Option<ArtPixels> {
    let mut picture = ArtPixels::clear(width, height);
    for row in 0..height {
        let start = dword_at(rows, row * DWORD)? * DWORD;
        let end = match row + 1 {
            next if next < height => dword_at(rows, next * DWORD)? * DWORD,
            _ => rows.len(),
        };
        let mut x = 0;
        let mut at = start;
        while at + RUN_BYTES <= end && x < width {
            let color = word_at(rows, at)?;
            let run = usize::from(word_at(rows, at + WORD)?).min(width - x);
            if color != 0 {
                let from = row * width + x;
                picture.colors[from..from + run].fill(color | PIXEL_DRAWN);
            }
            x += run;
            at += RUN_BYTES;
        }
    }
    Some(picture)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mul::{IDX_EMPTY, IDX_WIDTH_SHIFT};
    use crate::tests::scratch;
    use std::fs;

    const RED: u16 = 0x7C00;

    /// The rows of a gump: one offset for each row, then the runs.
    fn rows(rows: &[&[(u16, u16)]]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut offset = rows.len() as u32;
        for row in rows {
            data.extend(offset.to_le_bytes());
            offset += row.len() as u32;
        }
        for (color, run) in rows.iter().flat_map(|row| row.iter()) {
            data.extend(color.to_le_bytes());
            data.extend(run.to_le_bytes());
        }
        data
    }

    fn record(width: u32, height: u32, runs: &[&[(u16, u16)]]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(width.to_le_bytes());
        data.extend(height.to_le_bytes());
        data.extend(rows(runs));
        data
    }

    #[test]
    fn a_gump_is_rows_of_runs_and_color_zero_is_not_drawn() {
        let data = record(3, 2, &[&[(RED, 3)], &[(0, 1), (RED, 2)]]);
        let picture = decode_gump(&data).unwrap();
        assert_eq!((picture.width, picture.height), (3, 2));
        assert_eq!(picture.colors[0], RED | PIXEL_DRAWN);
        assert_eq!(picture.colors[3], 0);
        assert_eq!(picture.colors[5], RED | PIXEL_DRAWN);
        assert_eq!(package_size(&data), Some((3, 2)));
    }

    #[test]
    fn a_damaged_record_gives_no_picture() {
        assert!(decode_gump(&[]).is_none());
        assert!(decode_gump(&record(0, 2, &[])).is_none());
        let huge = record(GUMP_MAX_SIDE as u32 + 1, 1, &[&[(RED, 1)]]);
        assert!(decode_gump(&huge).is_none());
        let mut cut = record(3, 2, &[&[(RED, 3)], &[(RED, 3)]]);
        cut.truncate(SIZE_BYTES + 2);
        assert!(decode_gump(&cut).is_none());
    }

    #[test]
    fn the_mul_files_serve_a_client_without_the_package() {
        const GUMP_ID: u16 = 2;
        let dir = scratch("gumpmul");
        let data = rows(&[&[(RED, 2), (0, 1)], &[(0, 1), (RED, 2)]]);
        let mut idx = Vec::new();
        for id in 0..=GUMP_ID + 1 {
            let (offset, len, extra) = if id == GUMP_ID {
                (0, data.len() as u32, (3u32 << IDX_WIDTH_SHIFT) | 2)
            } else {
                (IDX_EMPTY, 0, 0u32)
            };
            idx.extend(offset.to_le_bytes());
            idx.extend(len.to_le_bytes());
            idx.extend(extra.to_le_bytes());
        }
        fs::write(dir.join(GUMP_MUL_NAMES[0]), &data).unwrap();
        fs::write(dir.join(GUMP_IDX_NAMES[0]), &idx).unwrap();
        let art = GumpArt::open(&dir).unwrap();
        assert_eq!(art.gump_size(GUMP_ID), Some((3, 2)));
        let picture = art.gump(GUMP_ID).unwrap();
        assert_eq!((picture.width, picture.height), (3, 2));
        assert_eq!(picture.colors[0], RED | PIXEL_DRAWN);
        assert_eq!(picture.colors[2], 0);
        assert_eq!(picture.colors[5], RED | PIXEL_DRAWN);
        assert!(art.gump(GUMP_ID + 1).is_none());
        assert!(art.gump_size(GUMP_ID + 1).is_none());
        assert!(art.gump(u16::MAX).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_with_no_gump_files_is_an_error() {
        let dir = scratch("gumpmul");
        assert!(matches!(GumpArt::open(&dir), Err(MapError::Missing(_))));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_real_package_holds_the_stone_background_when_client_files_are_here() {
        const STONE_CORNER: u16 = 9200;
        let Some(uopath) = crate::mul::client_data_dir_from_env() else {
            return;
        };
        let art = GumpArt::open(uopath).unwrap();
        let corner = art.gump(STONE_CORNER).unwrap();
        assert!(corner.width > 0 && corner.colors.iter().any(|c| c & PIXEL_DRAWN != 0));
        assert_eq!(
            art.gump_size(STONE_CORNER),
            Some((corner.width, corner.height))
        );
    }
}
