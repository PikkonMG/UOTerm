//! The colors of `radarcol.mul`: one color for each land tile and each
//! item, as a map of the world shows them from far above.

use std::path::Path;

use crate::art::channel;
use crate::mul::{read_file, MapError};

pub const RADARCOL_NAME: &str = "radarcol.mul";
/// The colors of the items come after the colors of the land.
const ITEM_BASE: usize = 0x4000;
const WORD: usize = 2;
const RED_SHIFT: u16 = 10;
const GREEN_SHIFT: u16 = 5;

pub struct RadarColors {
    colors: Vec<u16>,
}

impl RadarColors {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let path = uopath.as_ref().join(RADARCOL_NAME);
        if !path.exists() {
            return Err(MapError::Missing(RADARCOL_NAME));
        }
        Ok(Self::parse(&read_file(&path)?))
    }

    fn parse(data: &[u8]) -> Self {
        Self {
            colors: data
                .chunks_exact(WORD)
                .map(|word| u16::from_le_bytes([word[0], word[1]]))
                .collect(),
        }
    }

    fn rgb(&self, index: usize) -> Option<[u8; 3]> {
        let color = *self.colors.get(index)?;
        Some([
            channel(color >> RED_SHIFT),
            channel(color >> GREEN_SHIFT),
            channel(color),
        ])
    }

    pub fn land(&self, land_id: u16) -> Option<[u8; 3]> {
        self.rgb(usize::from(land_id))
    }

    pub fn item(&self, graphic: u16) -> Option<[u8; 3]> {
        self.rgb(ITEM_BASE + usize::from(graphic))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PURE_RED: u16 = 31 << RED_SHIFT;
    const PURE_BLUE: u16 = 31;
    const GRASS: u16 = 3;
    const WALL: u16 = 5;

    #[test]
    fn land_and_items_have_their_own_parts_of_the_file() {
        let mut data = vec![0u8; (ITEM_BASE + 16) * WORD];
        let grass_at = usize::from(GRASS) * WORD;
        data[grass_at..grass_at + WORD].copy_from_slice(&PURE_RED.to_le_bytes());
        let wall_at = (ITEM_BASE + usize::from(WALL)) * WORD;
        data[wall_at..wall_at + WORD].copy_from_slice(&PURE_BLUE.to_le_bytes());
        let colors = RadarColors::parse(&data);
        assert_eq!(colors.land(GRASS), Some([u8::MAX, 0, 0]));
        assert_eq!(colors.item(WALL), Some([0, 0, u8::MAX]));
        assert_eq!(colors.item(u16::MAX), None);
    }
}
