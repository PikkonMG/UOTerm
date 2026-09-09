use serde::{Deserialize, Serialize};

use crate::state::World;

pub const RADAR_SIZE: u16 = 21;
pub const RADAR_DEFAULT: u16 = RADAR_SIZE;

const SYM_SELF: char = '@';
const SYM_MOBILE: char = 'm';
const SYM_ITEM: char = 'i';
const SYM_BLOCK: char = '#';
const SYM_WALK: char = '.';
const SYM_WATER: char = '~';
const SYM_DOOR: char = '+';

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TileKind {
    Walk,
    Block,
    Water,
    Door,
}

impl TileKind {
    pub fn as_char(self) -> char {
        match self {
            Self::Walk => SYM_WALK,
            Self::Block => SYM_BLOCK,
            Self::Water => SYM_WATER,
            Self::Door => SYM_DOOR,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RadarOptions {
    pub size: u16,
}

impl Default for RadarOptions {
    fn default() -> Self {
        Self { size: RADAR_SIZE }
    }
}

pub fn render_radar(world: &World, opts: RadarOptions, tile: impl Fn(u16, u16) -> char) -> String {
    let size = if opts.size == 0 {
        RADAR_SIZE
    } else {
        opts.size
    };
    let half = (size / 2) as i32;
    let origin_x = world.self_state.location.x as i32;
    let origin_y = world.self_state.location.y as i32;
    let mut rows = Vec::with_capacity(size as usize);
    for dy in -half..=half {
        let mut row = String::with_capacity(size as usize);
        for dx in -half..=half {
            let x = origin_x + dx;
            let y = origin_y + dy;
            let ch = if dx == 0 && dy == 0 {
                SYM_SELF
            } else if x < 0 || y < 0 {
                SYM_BLOCK
            } else {
                let ux = x as u16;
                let uy = y as u16;
                if world.mobile_at(ux, uy).is_some() {
                    SYM_MOBILE
                } else if world.item_at(ux, uy).is_some() {
                    SYM_ITEM
                } else {
                    tile(ux, uy)
                }
            };
            row.push(ch);
        }
        rows.push(row);
    }
    rows.join("\n")
}

pub fn default_tile(_x: u16, _y: u16) -> char {
    SYM_WALK
}

pub fn legend() -> String {
    format!(
        "{SYM_SELF} self  {SYM_MOBILE} mobile  {SYM_ITEM} item  {SYM_BLOCK} block  {SYM_WALK} walk  {SYM_WATER} water  {SYM_DOOR} door"
    )
}
