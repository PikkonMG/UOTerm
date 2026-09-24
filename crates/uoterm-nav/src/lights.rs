//! The light shapes of `light.mul` with `lightidx.mul`: the pools of light a
//! torch, a lamp or a window throws at night. The index keeps the size of
//! each shape. A shape is one signed byte of light for each pixel, from -31
//! to 31; a dark pixel is stored with its bits turned over.
//!
//! Which shape an item throws follows the classic client: a light on the
//! map takes the quality byte of its tiledata record, an item on the ground
//! takes the light byte of the packet that shows it, and an item a mobile
//! holds takes the light index of its tiledata record.

use std::path::Path;

use crate::mul::{idx_entries, idx_sizes, read_file, MapError};
use crate::tiledata::ItemTile;
use crate::tiles::TileFlagSet;
use crate::uop::Package;

pub const LIGHT_NAME: &str = "light.mul";
pub const LIGHTIDX_NAME: &str = "lightidx.mul";
/// The client knows this many light shapes.
pub const LIGHT_SHAPE_COUNT: usize = 100;
/// The brightest a pixel of a shape is.
pub const LIGHT_LEVEL_MAX: u8 = 31;

/// A stored byte above the brightest level is a dark pixel, bits turned.
const LEVEL_MASK: u8 = 0x1F;
/// A level becomes a grey byte by this shift.
const LEVEL_TO_CHANNEL_SHIFT: u8 = 3;
const RGBA_BYTES: usize = 4;
/// The fire and energy fields throw light whatever tiledata says, and all
/// throw the same shape.
const FIELD_GRAPHICS: [std::ops::RangeInclusive<u16>; 3] =
    [0x3E02..=0x3E0B, 0x3914..=0x3929, 0x0B1D..=0x0B1D];
const FIELD_LIGHT_SHAPE: u8 = 2;
/// A light number above this is a colored light: the shape is the small
/// round one and the rest of the number names the color.
const COLORED_LIGHT_FROM: u8 = 200;
const COLORED_LIGHT_SHAPE: u8 = 1;

/// One light shape, one level of light for each pixel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LightShape {
    pub width: usize,
    pub height: usize,
    /// Row order, from zero (no light) to [`LIGHT_LEVEL_MAX`].
    pub levels: Vec<u8>,
}

impl LightShape {
    /// The shape as grey RGBA bytes. A pixel with no light is clear.
    pub fn rgba(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.levels.len() * RGBA_BYTES);
        for &level in &self.levels {
            let grey = level << LEVEL_TO_CHANNEL_SHIFT;
            let alpha = if level == 0 { 0 } else { u8::MAX };
            out.extend_from_slice(&[grey, grey, grey, alpha]);
        }
        out
    }
}

pub struct LightData {
    package: Package,
    sizes: Vec<(usize, usize)>,
}

/// The level of one stored byte.
fn level_of(stored: u8) -> u8 {
    if stored > LIGHT_LEVEL_MAX {
        !stored & LEVEL_MASK
    } else {
        stored
    }
}

fn decode_shape(data: &[u8], width: usize, height: usize) -> Option<LightShape> {
    let pixels = width.checked_mul(height).filter(|n| *n > 0)?;
    Some(LightShape {
        width,
        height,
        levels: data.get(..pixels)?.iter().map(|&b| level_of(b)).collect(),
    })
}

impl LightData {
    pub fn open(uopath: impl AsRef<Path>) -> Result<Self, MapError> {
        let dir = uopath.as_ref();
        let (mul, idx) = (dir.join(LIGHT_NAME), dir.join(LIGHTIDX_NAME));
        if !mul.exists() {
            return Err(MapError::Missing(LIGHT_NAME));
        }
        if !idx.exists() {
            return Err(MapError::Missing(LIGHTIDX_NAME));
        }
        let idx = read_file(&idx)?;
        Ok(Self {
            package: Package::new(&mul, idx_entries(&idx))?,
            sizes: idx_sizes(&idx),
        })
    }

    /// One light shape by its number, or None when the files hold none.
    pub fn shape(&self, light_id: u8) -> Option<LightShape> {
        let (width, height) = *self.sizes.get(usize::from(light_id))?;
        decode_shape(&self.package.read(u32::from(light_id))?, width, height)
    }
}

/// What holds an item that may throw light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightHolder {
    /// A static of the map files, or a piece of a house or a boat.
    MapStatic,
    /// An item lying in the world. `packet_light` is the light byte of the
    /// packet that shows it.
    GroundItem { packet_light: u8 },
    /// An item a mobile wears or holds.
    Held,
}

/// The light an item throws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemLight {
    /// The light shape, for [`LightData::shape`].
    pub shape: u8,
    /// The color number of a colored light, for a client that draws colored
    /// lights. None for a plain light.
    pub color: Option<u8>,
}

fn is_field(graphic: u16) -> bool {
    FIELD_GRAPHICS.iter().any(|range| range.contains(&graphic))
}

/// The light an item of `graphic` throws, or None when it throws none.
pub fn item_light(graphic: u16, tile: &ItemTile, holder: LightHolder) -> Option<ItemLight> {
    let lit = tile.flags.contains(TileFlagSet::LIGHT_SOURCE)
        || (matches!(holder, LightHolder::GroundItem { .. }) && is_field(graphic));
    if !lit {
        return None;
    }
    let number = if is_field(graphic) {
        FIELD_LIGHT_SHAPE
    } else {
        match holder {
            LightHolder::MapStatic => tile.quality,
            LightHolder::GroundItem { packet_light } => packet_light,
            LightHolder::Held => tile.light_index as u8,
        }
    };
    let light = if number > COLORED_LIGHT_FROM {
        ItemLight {
            shape: COLORED_LIGHT_SHAPE,
            color: Some(number - COLORED_LIGHT_FROM),
        }
    } else {
        ItemLight {
            shape: number,
            color: None,
        }
    };
    (usize::from(light.shape) < LIGHT_SHAPE_COUNT).then_some(light)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LANTERN_SHAPE: u8 = 29;
    const HELD_SHAPE: u16 = 6;
    const FIRE_FIELD: u16 = 0x3915;
    const PLAIN_GRAPHIC: u16 = 0x0A22;

    fn lamp() -> ItemTile {
        ItemTile {
            flags: TileFlagSet::LIGHT_SOURCE,
            quality: LANTERN_SHAPE,
            light_index: HELD_SHAPE,
            ..ItemTile::default()
        }
    }

    #[test]
    fn a_dark_pixel_is_stored_with_its_bits_turned() {
        assert_eq!(level_of(0), 0);
        assert_eq!(level_of(LIGHT_LEVEL_MAX), LIGHT_LEVEL_MAX);
        assert_eq!(level_of(0xFF), 0);
        assert_eq!(level_of(0xE1), 30);
    }

    #[test]
    fn a_shape_becomes_grey_light() {
        let shape = decode_shape(&[0, 31, 0xE1, 1, 9], 2, 2).unwrap();
        assert_eq!(shape.levels, vec![0, 31, 30, 1]);
        let rgba = shape.rgba();
        assert_eq!(&rgba[..RGBA_BYTES], &[0, 0, 0, 0]);
        assert_eq!(&rgba[RGBA_BYTES..2 * RGBA_BYTES], &[248, 248, 248, u8::MAX]);
        assert!(decode_shape(&[1, 2, 3], 2, 2).is_none());
        assert!(decode_shape(&[1], 0, 0).is_none());
    }

    #[test]
    fn the_holder_names_the_byte_the_shape_comes_from() {
        let tile = lamp();
        let shape = |holder| item_light(PLAIN_GRAPHIC, &tile, holder).map(|l| l.shape);
        assert_eq!(shape(LightHolder::MapStatic), Some(LANTERN_SHAPE));
        assert_eq!(shape(LightHolder::Held), Some(HELD_SHAPE as u8));
        let ground = LightHolder::GroundItem { packet_light: 4 };
        assert_eq!(shape(ground), Some(4));
        assert!(item_light(PLAIN_GRAPHIC, &ItemTile::default(), ground).is_none());
    }

    #[test]
    fn fields_throw_their_own_shape_and_big_numbers_are_colored_lights() {
        let unlit = ItemTile::default();
        let ground = LightHolder::GroundItem { packet_light: 0 };
        let field = item_light(FIRE_FIELD, &unlit, ground).unwrap();
        assert_eq!(field.shape, FIELD_LIGHT_SHAPE);
        let colored = ItemTile {
            quality: COLORED_LIGHT_FROM + 30,
            ..lamp()
        };
        let light = item_light(PLAIN_GRAPHIC, &colored, LightHolder::MapStatic).unwrap();
        assert_eq!(light.shape, COLORED_LIGHT_SHAPE);
        assert_eq!(light.color, Some(30));
        let too_big = ItemTile {
            quality: LIGHT_SHAPE_COUNT as u8,
            ..lamp()
        };
        assert!(item_light(PLAIN_GRAPHIC, &too_big, LightHolder::MapStatic).is_none());
    }

    #[test]
    fn the_real_files_hold_the_shape_of_a_lantern_when_client_files_are_here() {
        const LIT_LANTERN: u16 = 0x0A22;
        let Some(dir) = crate::client_data_dir_from_env() else {
            return;
        };
        let lights = LightData::open(&dir).unwrap();
        let tiles = crate::tiledata::TileData::open(&dir).unwrap();
        let light = item_light(
            LIT_LANTERN,
            tiles.item(LIT_LANTERN).unwrap(),
            LightHolder::MapStatic,
        )
        .unwrap();
        let shape = lights.shape(light.shape).unwrap();
        assert!(shape.width > 0 && shape.levels.iter().any(|l| *l > 0));
        assert!(lights.shape(u8::MAX).is_none());
    }
}
