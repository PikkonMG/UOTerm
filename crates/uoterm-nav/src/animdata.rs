//! The picture cycles of `animdata.mul`: a fire that burns, water that
//! moves, a spell that sparkles. Each item with the animation word in
//! tiledata has one record. The record lists, for each step of the cycle,
//! how far the picture of that step is from the item's own picture.

use std::path::Path;

use crate::mul::{read_file, MapError};

pub const ANIMDATA_NAME: &str = "animdata.mul";
/// The tiledata word that an item has a picture cycle.
pub const TILE_ANIMATED: u32 = 0x0100_0000;

const BLOCK_HEADER_BYTES: usize = 4;
const RECORDS_PER_BLOCK: usize = 8;
const OFFSETS_PER_RECORD: usize = 64;
/// After the offsets come an unused byte, the count of steps, the time of
/// one step and the wait before the cycle starts.
const RECORD_BYTES: usize = OFFSETS_PER_RECORD + 4;
const COUNT_AT: usize = OFFSETS_PER_RECORD + 1;
const INTERVAL_AT: usize = OFFSETS_PER_RECORD + 2;
const BLOCK_BYTES: usize = BLOCK_HEADER_BYTES + RECORDS_PER_BLOCK * RECORD_BYTES;
/// One unit of the step time of a record, in milliseconds.
const INTERVAL_UNIT_MS: u64 = 50;

struct Cycle {
    offsets: Vec<i8>,
    step_ms: u64,
}

pub struct ArtCycles {
    /// One place for each item graphic. None for an item with no cycle.
    cycles: Vec<Option<Cycle>>,
}

impl ArtCycles {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let path = uopath.as_ref().join(ANIMDATA_NAME);
        if !path.exists() {
            return Err(MapError::Missing(ANIMDATA_NAME));
        }
        Ok(Self::parse(&read_file(&path)?))
    }

    fn parse(data: &[u8]) -> Self {
        let mut cycles = Vec::new();
        for block in data.chunks_exact(BLOCK_BYTES) {
            for record in block[BLOCK_HEADER_BYTES..].chunks_exact(RECORD_BYTES) {
                let count = usize::from(record[COUNT_AT]).min(OFFSETS_PER_RECORD);
                cycles.push((count > 0).then(|| Cycle {
                    offsets: record[..count].iter().map(|b| *b as i8).collect(),
                    step_ms: u64::from(record[INTERVAL_AT].max(1)) * INTERVAL_UNIT_MS,
                }));
            }
        }
        Self { cycles }
    }

    /// The picture an animated item shows at `time_ms`. An item with no
    /// cycle shows its own picture.
    pub fn graphic_at(&self, graphic: u16, time_ms: u64) -> u16 {
        let Some(Some(cycle)) = self.cycles.get(usize::from(graphic)) else {
            return graphic;
        };
        let step = (time_ms / cycle.step_ms) as usize % cycle.offsets.len();
        graphic.wrapping_add_signed(i16::from(cycle.offsets[step]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRE: u16 = 3;
    const STEP_UNITS: u8 = 2;

    fn one_block() -> Vec<u8> {
        let mut data = vec![0u8; BLOCK_BYTES];
        let at = BLOCK_HEADER_BYTES + usize::from(FIRE) * RECORD_BYTES;
        data[at..at + 3].copy_from_slice(&[0, 1, 0xFF]);
        data[at + COUNT_AT] = 3;
        data[at + INTERVAL_AT] = STEP_UNITS;
        data
    }

    #[test]
    fn an_animated_item_goes_through_its_cycle_and_starts_again() {
        let cycles = ArtCycles::parse(&one_block());
        let step = u64::from(STEP_UNITS) * INTERVAL_UNIT_MS;
        assert_eq!(cycles.graphic_at(FIRE, 0), FIRE);
        assert_eq!(cycles.graphic_at(FIRE, step), FIRE + 1);
        assert_eq!(cycles.graphic_at(FIRE, step * 2), FIRE - 1);
        assert_eq!(cycles.graphic_at(FIRE, step * 3), FIRE);
    }

    #[test]
    fn an_item_with_no_cycle_keeps_its_picture() {
        let cycles = ArtCycles::parse(&one_block());
        assert_eq!(cycles.graphic_at(FIRE + 1, 500), FIRE + 1);
        assert_eq!(cycles.graphic_at(u16::MAX, 500), u16::MAX);
    }
}
