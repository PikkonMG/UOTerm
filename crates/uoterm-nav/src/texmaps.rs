//! The land textures of `texmaps.mul` with `texidx.mul`: the square
//! pictures the client stretches over land that is not flat. A land tile
//! names its texture in tiledata. A texture is 64 or 128 pixels square, one
//! 15-bit color for each pixel, and every pixel is drawn.
//!
//! `TexTerr.def` points some textures at others, as the classic client
//! reads it.

use std::path::Path;

use crate::art::{ArtPixels, PIXEL_DRAWN};
use crate::mul::{idx_entries, read_file, MapError};
use crate::tiledata::TileData;
use crate::uop::Package;

pub const TEXMAPS_NAME: &str = "texmaps.mul";
pub const TEXIDX_NAME: &str = "texidx.mul";
pub const TEXTERR_DEF_NAME: &str = "TexTerr.def";
/// The two sides a texture comes in.
pub const TEXTURE_SMALL_SIDE: usize = 64;
pub const TEXTURE_LARGE_SIDE: usize = 128;

const WORD: usize = 2;
const COMMENT: char = '#';
const GROUP_OPEN: char = '{';
const GROUP_CLOSE: char = '}';
const GROUP_SEPARATOR: char = ',';

pub struct TexmapData {
    package: Package,
}

/// The rows of `TexTerr.def`, `texture {stands_in, ...} extra`, in file
/// order, for a file of `count` textures. The last number of the group that
/// names a texture of the file is the one drawn.
fn texterr_aliases(text: &str, count: usize) -> Vec<(usize, usize)> {
    text.lines()
        .filter_map(|line| {
            let line = line.split(COMMENT).next()?;
            let (texture, rest) = line.split_once(GROUP_OPEN)?;
            let (group, _) = rest.split_once(GROUP_CLOSE)?;
            let texture = texture.trim().parse().ok().filter(|t| *t < count)?;
            let group: Vec<usize> = group
                .split(|c: char| c == GROUP_SEPARATOR || c.is_whitespace())
                .filter_map(|n| n.parse().ok())
                .collect();
            let stands_in = group.into_iter().rev().find(|n| *n < count)?;
            Some((texture, stands_in))
        })
        .collect()
}

/// The side of a texture record of `len` bytes, or None for a length no
/// texture has.
fn texture_side(len: usize) -> Option<usize> {
    [TEXTURE_SMALL_SIDE, TEXTURE_LARGE_SIDE]
        .into_iter()
        .find(|side| side * side * WORD == len)
}

fn decode_texture(data: &[u8]) -> Option<ArtPixels> {
    let side = texture_side(data.len())?;
    Some(ArtPixels {
        width: side,
        height: side,
        colors: data
            .chunks_exact(WORD)
            .map(|w| u16::from_le_bytes([w[0], w[1]]) | PIXEL_DRAWN)
            .collect(),
    })
}

impl TexmapData {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let dir = uopath.as_ref();
        let (mul, idx) = (dir.join(TEXMAPS_NAME), dir.join(TEXIDX_NAME));
        if !mul.exists() {
            return Err(MapError::Missing(TEXMAPS_NAME));
        }
        if !idx.exists() {
            return Err(MapError::Missing(TEXIDX_NAME));
        }
        let mut entries = idx_entries(&read_file(&idx)?);
        let def = dir.join(TEXTERR_DEF_NAME);
        if def.exists() {
            let text: String = read_file(&def)?.iter().map(|&b| char::from(b)).collect();
            for (texture, stands_in) in texterr_aliases(&text, entries.len()) {
                entries[texture] = entries[stands_in];
            }
        }
        Ok(Self {
            package: Package::new(&mul, entries)?,
        })
    }

    /// One texture by its number, or None when the files hold none.
    pub fn texture(&self, texture_id: u16) -> Option<ArtPixels> {
        decode_texture(&self.package.read(u32::from(texture_id))?)
    }

    /// The texture a land tile takes on a slope, by the texture tiledata
    /// names for it. None when tiledata names none.
    pub fn land_texture(&self, tiles: &TileData, land_id: u16) -> Option<ArtPixels> {
        let texture_id = tiles
            .land(land_id)
            .map(|land| land.texture_id)
            .filter(|id| *id != 0)?;
        self.texture(texture_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLOR: u16 = 0x1234;
    const RGBA_BYTES: usize = 4;

    #[test]
    fn a_texture_is_a_square_of_drawn_colors() {
        let data: Vec<u8> =
            std::iter::repeat_n(COLOR.to_le_bytes(), TEXTURE_SMALL_SIDE * TEXTURE_SMALL_SIDE)
                .flatten()
                .collect();
        let texture = decode_texture(&data).unwrap();
        assert_eq!(
            (texture.width, texture.height),
            (TEXTURE_SMALL_SIDE, TEXTURE_SMALL_SIDE)
        );
        assert!(texture.colors.iter().all(|c| *c == COLOR | PIXEL_DRAWN));
        let large = vec![0u8; TEXTURE_LARGE_SIDE * TEXTURE_LARGE_SIDE * WORD];
        assert_eq!(decode_texture(&large).unwrap().width, TEXTURE_LARGE_SIDE);
    }

    #[test]
    fn a_record_of_another_length_is_no_texture() {
        assert!(decode_texture(&[]).is_none());
        assert!(decode_texture(&[0u8; 100]).is_none());
    }

    #[test]
    fn the_def_file_points_textures_at_others() {
        const COUNT: usize = 3000;
        let def = "# comment\n2500  {3} 1645\n2501 {4, 5, 9999} 0\nbad line\n9000 {3} 0\n";
        assert_eq!(texterr_aliases(def, COUNT), vec![(2500, 3), (2501, 5)]);
    }

    #[test]
    fn the_real_files_give_the_texture_of_grass_when_client_files_are_here() {
        const LAND_GRASS: u16 = 3;
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        let textures = TexmapData::open(&dir).unwrap();
        let tiles = TileData::open(&dir).unwrap();
        let grass = textures.land_texture(&tiles, LAND_GRASS).unwrap();
        assert!([TEXTURE_SMALL_SIDE, TEXTURE_LARGE_SIDE].contains(&grass.width));
        assert_eq!(grass.width, grass.height);
        assert_eq!(
            grass.rgba(None).len(),
            grass.width * grass.height * RGBA_BYTES
        );
    }
}
