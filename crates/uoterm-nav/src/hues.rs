//! The color ramps of `hues.mul`. A hue replaces each color of a picture
//! with the ramp color that has the same brightness.

use std::path::Path;

use crate::mul::{read_file, MapError};

pub const HUES_NAME: &str = "hues.mul";
pub const HUE_RAMP_LEN: usize = 32;
/// The part of a hue number that names the ramp.
pub const HUE_ID_MASK: u16 = 0x3FFF;
/// A hue with this bit set colors only the grey pixels of a picture.
pub const HUE_PARTIAL_BIT: u16 = 0x8000;

const GROUP_HEADER_BYTES: usize = 4;
const HUES_PER_GROUP: usize = 8;
const WORD: usize = 2;
const RAMP_BYTES: usize = HUE_RAMP_LEN * WORD;
/// After the ramp come the first and last ramp index and a 20-byte name.
const HUE_BYTES: usize = RAMP_BYTES + WORD + WORD + 20;
const GROUP_BYTES: usize = GROUP_HEADER_BYTES + HUES_PER_GROUP * HUE_BYTES;
const RED_SHIFT: u16 = 10;
const GREEN_SHIFT: u16 = 5;
const CHANNEL_MASK: u16 = 0x1F;
const KEEP_FLAGS: u16 = 0x8000;

pub struct HueData {
    ramps: Vec<[u16; HUE_RAMP_LEN]>,
}

/// One ramp, ready to color a picture.
#[derive(Clone, Copy, Debug)]
pub struct HueRamp<'a> {
    colors: &'a [u16; HUE_RAMP_LEN],
    partial: bool,
}

impl HueRamp<'_> {
    /// The ramp color for one file color. The top bit of `color` is kept.
    pub fn apply(self, color: u16) -> u16 {
        let red = (color >> RED_SHIFT) & CHANNEL_MASK;
        let green = (color >> GREEN_SHIFT) & CHANNEL_MASK;
        let blue = color & CHANNEL_MASK;
        if self.partial && !(red == green && green == blue) {
            return color;
        }
        (color & KEEP_FLAGS) | (self.colors[usize::from(red)] & !KEEP_FLAGS)
    }
}

impl HueData {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let path = uopath.as_ref().join(HUES_NAME);
        if !path.exists() {
            return Err(MapError::Missing(HUES_NAME));
        }
        Ok(Self::parse(&read_file(&path)?))
    }

    fn parse(data: &[u8]) -> Self {
        let mut ramps = Vec::new();
        for group in data.chunks_exact(GROUP_BYTES) {
            for hue in group[GROUP_HEADER_BYTES..].chunks_exact(HUE_BYTES) {
                let mut ramp = [0u16; HUE_RAMP_LEN];
                for (slot, word) in ramp.iter_mut().zip(hue[..RAMP_BYTES].chunks_exact(WORD)) {
                    *slot = u16::from_le_bytes([word[0], word[1]]);
                }
                ramps.push(ramp);
            }
        }
        Self { ramps }
    }

    /// The ramp a hue number names. None for hue zero and for a number the
    /// file does not hold. `partial_item` is the tiledata word that the item
    /// takes color on its grey pixels only.
    pub fn ramp(&self, hue: u16, partial_item: bool) -> Option<HueRamp<'_>> {
        let id = usize::from(hue & HUE_ID_MASK);
        let colors = self.ramps.get(id.checked_sub(1)?)?;
        Some(HueRamp {
            colors,
            partial: partial_item || hue & HUE_PARTIAL_BIT != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREY_MID: u16 = (16 << RED_SHIFT) | (16 << GREEN_SHIFT) | 16;
    const PURE_RED: u16 = 31 << RED_SHIFT;
    const RAMP_MARK: u16 = 0x0123;
    const FIRST_HUE: u16 = 1;

    fn one_group() -> Vec<u8> {
        let mut data = vec![0u8; GROUP_BYTES];
        let at = GROUP_HEADER_BYTES + 16 * WORD;
        data[at..at + WORD].copy_from_slice(&RAMP_MARK.to_le_bytes());
        data
    }

    #[test]
    fn hue_zero_is_no_ramp() {
        let hues = HueData::parse(&one_group());
        assert!(hues.ramp(0, false).is_none());
        assert!(hues.ramp(FIRST_HUE, false).is_some());
        assert!(hues.ramp(HUES_PER_GROUP as u16 + 1, false).is_none());
    }

    #[test]
    fn ramp_follows_the_red_channel_and_keeps_the_top_bit() {
        let hues = HueData::parse(&one_group());
        let ramp = hues.ramp(FIRST_HUE, false).unwrap();
        assert_eq!(ramp.apply(GREY_MID | KEEP_FLAGS), RAMP_MARK | KEEP_FLAGS);
    }

    #[test]
    fn partial_hue_leaves_colored_pixels() {
        let hues = HueData::parse(&one_group());
        let ramp = hues.ramp(FIRST_HUE | HUE_PARTIAL_BIT, false).unwrap();
        assert_eq!(ramp.apply(PURE_RED), PURE_RED);
        assert_eq!(ramp.apply(GREY_MID), RAMP_MARK);
    }
}
