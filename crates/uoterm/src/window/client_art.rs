//! The client files as the map needs them: what lies on each tile, and the
//! picture of each thing. The window reads the files itself, so the session
//! sends no pictures.

use super::atlas::{ArtKey, Atlas, Picture, Sprite};
use super::figure::{self, FrameCache, Pose};
use crate::view::WatchLook;
use eframe::egui::{Color32, Vec2};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use uoterm_nav::{
    land_is_ignored, Action, AnimData, ArtCycles, ArtData, ArtPixels, Deed, GumpArt, HueData,
    MulMap, MultiData, MultiPiece, RadarColors, Stance, TileQuery, TILE_ANIMATED, TILE_PARTIAL_HUE,
};

/// How many tiles the window remembers. A full window shows about four
/// thousand, so this is a few windows of walking.
/// The layers of the two hands. A weapon or a shield is on one of them.
const WEAPON_LAYERS: [u8; 2] = [1, 2];
const CELL_CACHE_CAP: usize = 24_000;
/// Item graphics that the client never draws.
const NO_DRAW_GRAPHICS: [u16; 3] = [0x0001, 0x21BC, 0x63D3];
const NO_DRAW_RANGE: std::ops::RangeInclusive<u16> = 0x2198..=0x21A4;

#[derive(Clone, Copy, Debug)]
pub struct CellStatic {
    pub graphic: u16,
    pub hue: u16,
    pub z: i8,
    /// The height of its top, when a person can stand on it.
    pub floor: Option<i16>,
}

/// One map tile as the window draws it.
#[derive(Clone, Debug)]
pub struct Cell {
    /// None when the land of the tile draws nothing.
    pub land_id: Option<u16>,
    /// The heights of the top, right, bottom and left points of the diamond.
    pub corners: [i8; 4],
    pub statics: Vec<CellStatic>,
}

pub struct ClientArt {
    uopath: PathBuf,
    art: ArtData,
    /// None when the client files hold no classic animation files.
    anim: Option<AnimData>,
    frames: FrameCache,
    hues: Option<HueData>,
    /// None when the client files hold no picture cycles.
    cycles: Option<ArtCycles>,
    /// None when the client files hold no houses and boats.
    multis: Option<MultiData>,
    /// None when the client files hold no gump pictures.
    gumps: Option<GumpArt>,
    /// None when the client files hold no colors for a world map.
    radar: Option<RadarColors>,
    /// None marks a map the client files do not hold.
    maps: HashMap<u8, Option<MulMap>>,
    cells: HashMap<(u8, u16, u16), Cell>,
}

pub fn is_drawn(graphic: u16) -> bool {
    !NO_DRAW_GRAPHICS.contains(&graphic) && !NO_DRAW_RANGE.contains(&graphic)
}

impl ClientArt {
    pub fn open(uopath: &Path) -> Result<Self, String> {
        Ok(Self {
            uopath: uopath.to_path_buf(),
            art: ArtData::open(uopath).map_err(|e| e.to_string())?,
            anim: AnimData::open(uopath).ok(),
            frames: FrameCache::default(),
            hues: HueData::open(uopath).ok(),
            cycles: ArtCycles::open(uopath).ok(),
            multis: MultiData::open(uopath).ok(),
            radar: RadarColors::open(uopath).ok(),
            gumps: GumpArt::open(uopath).ok(),
            maps: HashMap::new(),
            cells: HashMap::new(),
        })
    }

    pub fn cell(&mut self, map_index: u8, x: u16, y: u16) -> Option<&Cell> {
        let key = (map_index, x, y);
        if !self.cells.contains_key(&key) {
            let uopath = &self.uopath;
            let map = self
                .maps
                .entry(map_index)
                .or_insert_with(|| MulMap::open(uopath, map_index).ok())
                .as_ref()?;
            if !map.in_bounds(x, y) {
                return None;
            }
            let column = map.column(x, y);
            let statics = map
                .statics_at(x, y)
                .into_iter()
                .filter(|s| is_drawn(s.graphic))
                .map(|s| CellStatic {
                    graphic: s.graphic,
                    hue: s.hue,
                    z: s.z,
                    floor: s.surface().then(|| i16::from(s.z) + i16::from(s.height)),
                })
                .collect();
            let cell = Cell {
                land_id: (!land_is_ignored(column.land_id)).then_some(column.land_id),
                corners: [
                    column.land.north_west,
                    column.land.north_east,
                    column.land.south_east,
                    column.land.south_west,
                ],
                statics,
            };
            if self.cells.len() >= CELL_CACHE_CAP {
                self.cells.clear();
            }
            self.cells.insert(key, cell);
        }
        self.cells.get(&key)
    }

    pub fn land_sprite(&self, atlas: &mut Atlas, land_id: u16) -> Option<Sprite> {
        atlas.sprite(ArtKey::Land(land_id), || {
            self.art.land(land_id).map(|art| picture_of(&art, None))
        })
    }

    fn item_flags(&self, map_index: u8, graphic: u16) -> u32 {
        self.tiledata(map_index)
            .map_or(0, |map| map.item_flags(graphic))
    }

    /// Each map holds the same tiledata, so the map of the character serves.
    fn tiledata(&self, map_index: u8) -> Option<&MulMap> {
        self.maps.get(&map_index).and_then(Option::as_ref)
    }

    pub fn item_sprite(
        &self,
        atlas: &mut Atlas,
        map_index: u8,
        graphic: u16,
        hue: u16,
    ) -> Option<Sprite> {
        atlas.sprite(ArtKey::Item { graphic, hue }, || {
            let art = self.art.item(graphic)?;
            let partial = self.item_flags(map_index, graphic) & TILE_PARTIAL_HUE != 0;
            let ramp = self.hues.as_ref().and_then(|h| h.ramp(hue, partial));
            Some(picture_of(&art, ramp))
        })
    }

    /// True when the client files hold the pictures of gumps.
    pub fn has_gump_art(&self) -> bool {
        self.gumps.is_some()
    }

    pub fn gump_sprite(&self, atlas: &mut Atlas, gump: u16, hue: u16) -> Option<Sprite> {
        atlas.sprite(ArtKey::Gump { gump, hue }, || {
            let art = self.gumps.as_ref()?.gump(gump)?;
            let ramp = self.hues.as_ref().and_then(|h| h.ramp(hue, false));
            Some(picture_of(&art, ramp))
        })
    }

    /// The color of one tile on a map of the world: the color of its
    /// highest item, or of its land. None past the edge of the map.
    pub fn radar_rgb(&mut self, map_index: u8, x: u16, y: u16) -> Option<[u8; 3]> {
        let radar = self.radar.as_ref()?;
        let uopath = &self.uopath;
        let map = self
            .maps
            .entry(map_index)
            .or_insert_with(|| MulMap::open(uopath, map_index).ok())
            .as_ref()?;
        if !map.in_bounds(x, y) {
            return None;
        }
        let top = map
            .statics_at(x, y)
            .into_iter()
            .filter(|s| is_drawn(s.graphic))
            .max_by_key(|s| s.z);
        match top {
            Some(item) => radar.item(item.graphic),
            None => radar.land(map.column(x, y).land_id),
        }
    }

    /// The pieces of a house or a boat.
    pub fn multi_pieces(&self, multi_id: u16) -> &[MultiPiece] {
        self.multis
            .as_ref()
            .map_or(&[], |multis| multis.pieces(multi_id))
    }

    /// The picture an item shows now. A fire or a fountain goes through
    /// the pictures of its cycle.
    pub fn shown_graphic(&self, map_index: u8, graphic: u16, time_ms: u64) -> u16 {
        match &self.cycles {
            Some(cycles) if self.item_flags(map_index, graphic) & TILE_ANIMATED != 0 => {
                cycles.graphic_at(graphic, time_ms)
            }
            _ => graphic,
        }
    }

    /// The color of words written in a hue.
    pub fn text_rgb(&self, hue: u16) -> Option<[u8; 3]> {
        self.hues.as_ref()?.text_rgb(hue)
    }

    /// The action that shows a deed of a mobile, such as a swing or a cast.
    pub fn deed_action(&self, look: &WatchLook, deed: Deed) -> Option<Action> {
        self.anim
            .as_ref()?
            .deed_action(look.body, deed, figure::is_mounted(look))
    }

    /// The action of a mobile for the way he holds himself. A person on
    /// foot in war mode stands ready, and one with a weapon walks armed.
    pub fn stance_action(&self, look: &WatchLook, action: Action) -> Action {
        let Some(anim) = self.anim.as_ref().filter(|_| !figure::is_mounted(look)) else {
            return action;
        };
        let armed = look
            .equipment
            .iter()
            .any(|item| WEAPON_LAYERS.contains(&item.layer));
        let stance = Stance {
            armed,
            war: look.war,
        };
        anim.stance_action(look.body, action, stance)
    }

    /// The picture of a fallen body: the last picture of its death.
    pub fn corpse_sprite(
        &self,
        atlas: &mut Atlas,
        map_index: u8,
        look: &WatchLook,
        outline: Color32,
    ) -> Option<Sprite> {
        let action = self.deed_action(look, Deed::Die)?;
        let source = figure::Source {
            anim: self.anim.as_ref()?,
            hues: self.hues.as_ref(),
            tiledata: self.tiledata(map_index)?,
            cache: &self.frames,
        };
        let last = source.cycle(look, action).saturating_sub(1);
        self.figure_sprite(atlas, map_index, look, Pose { action, tick: last }, outline)
    }

    /// True when the body is a person: a human, an elf, a gargoyle.
    pub fn is_person(&self, body: u16) -> bool {
        self.anim.as_ref().is_some_and(|anim| anim.is_person(body))
    }

    /// The picture of one mobile in one pose: his body, his mount and what
    /// he wears, with an outline in `outline`.
    pub fn figure_sprite(
        &self,
        atlas: &mut Atlas,
        map_index: u8,
        look: &WatchLook,
        pose: Pose,
        outline: Color32,
    ) -> Option<Sprite> {
        let source = figure::Source {
            anim: self.anim.as_ref()?,
            hues: self.hues.as_ref(),
            tiledata: self.tiledata(map_index)?,
            cache: &self.frames,
        };
        // Two ticks that show the same frame share one picture.
        let pose = Pose {
            tick: pose.tick % source.cycle(look, pose.action),
            ..pose
        };
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (look, pose, outline.to_array()).hash(&mut hasher);
        atlas.sprite(ArtKey::Figure(hasher.finish()), || {
            figure::compose(&source, look, pose, outline)
        })
    }
}

fn picture_of(art: &ArtPixels, ramp: Option<uoterm_nav::HueRamp<'_>>) -> Picture {
    Picture {
        width: art.width,
        height: art.height,
        rgba: art.rgba(ramp),
        anchor: Vec2::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_draw_graphics_are_left_out() {
        const BARREL: u16 = 0x0E77;
        assert!(is_drawn(BARREL));
        assert!(!is_drawn(NO_DRAW_GRAPHICS[0]));
        assert!(!is_drawn(*NO_DRAW_RANGE.start()));
    }

    #[test]
    fn real_files_give_the_cell_under_a_bank() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        const TRAMMEL: u8 = 1;
        let mut client = ClientArt::open(&dir).unwrap();
        let cell = client.cell(TRAMMEL, 3484, 2570).unwrap();
        assert!(!cell.statics.is_empty());
    }
}
