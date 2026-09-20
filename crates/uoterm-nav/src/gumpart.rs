//! The pictures of gumps: the stone and paper backgrounds of a dialog, its
//! buttons, its check boxes. They come from the gump art package of the
//! client. A record is the width and the height, then one offset for each
//! row, then the rows as runs of one color.

use std::path::Path;

use crate::art::{ArtPixels, PIXEL_DRAWN};
use crate::mul::{slice_at, MapError};
use crate::uop::{load_gump_entries, Package};

pub const GUMP_UOP_NAME: &str = "gumpartLegacyMUL.uop";
/// No real gump is larger than this. A larger number is a damaged record.
pub const GUMP_MAX_SIDE: usize = 2048;

const DWORD: usize = 4;
const WORD: usize = 2;
/// The package puts the width and the height before the rows.
const SIZE_BYTES: usize = DWORD * 2;
const RUN_BYTES: usize = WORD * 2;

pub struct GumpArt {
    package: Package,
}

impl GumpArt {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let path = uopath.as_ref().join(GUMP_UOP_NAME);
        if !path.exists() {
            return Err(MapError::Missing(GUMP_UOP_NAME));
        }
        Ok(Self {
            package: Package::new(&path, load_gump_entries(&path)?)?,
        })
    }

    /// The picture of one gump id, or None when the files hold none.
    pub fn gump(&self, gump_id: u16) -> Option<ArtPixels> {
        decode_gump(&self.package.read(u32::from(gump_id))?)
    }
}

fn dword_at(data: &[u8], at: usize) -> Option<usize> {
    slice_at(data, at, DWORD).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
}

fn word_at(data: &[u8], at: usize) -> Option<u16> {
    slice_at(data, at, WORD).map(|b| u16::from_le_bytes([b[0], b[1]]))
}

/// The offsets of the rows count in 4-byte steps from the first offset. A
/// row ends where the next one starts. A run of color zero is not drawn.
fn decode_gump(data: &[u8]) -> Option<ArtPixels> {
    let width = dword_at(data, 0)?;
    let height = dword_at(data, DWORD)?;
    if width == 0 || height == 0 || width > GUMP_MAX_SIDE || height > GUMP_MAX_SIDE {
        return None;
    }
    let rows = slice_at(data, SIZE_BYTES, data.len().checked_sub(SIZE_BYTES)?)?;
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

    const RED: u16 = 0x7C00;

    fn record(width: u32, height: u32, rows: &[&[(u16, u16)]]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend(width.to_le_bytes());
        data.extend(height.to_le_bytes());
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

    #[test]
    fn a_gump_is_rows_of_runs_and_color_zero_is_not_drawn() {
        let data = record(3, 2, &[&[(RED, 3)], &[(0, 1), (RED, 2)]]);
        let picture = decode_gump(&data).unwrap();
        assert_eq!((picture.width, picture.height), (3, 2));
        assert_eq!(picture.colors[0], RED | PIXEL_DRAWN);
        assert_eq!(picture.colors[3], 0);
        assert_eq!(picture.colors[5], RED | PIXEL_DRAWN);
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
    fn the_real_package_holds_the_stone_background_when_client_files_are_here() {
        const STONE_CORNER: u16 = 9200;
        let Some(uopath) = crate::mul::client_data_dir_from_env() else {
            return;
        };
        let art = GumpArt::open(uopath).unwrap();
        let corner = art.gump(STONE_CORNER).unwrap();
        assert!(corner.width > 0 && corner.colors.iter().any(|c| c & PIXEL_DRAWN != 0));
    }
}
