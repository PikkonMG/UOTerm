//! What of the world over the character is cut away, the circle of
//! transparency round him, and how things fade in and out.

use super::build::Standing;
use super::{SceneState, HALF_TILE};
use crate::art::WorldArt;
use crate::frame::WatchFrame;
use crate::geom::{Area, Point, Vector};
use crate::look;
use crate::settings::CircleStyle;
use std::collections::HashMap;
use uoterm_nav::TileFlagSet;

/// The classic client cuts the world over the character from this far
/// above his feet, and takes him to be this tall.
const HEAD_ROOM: i16 = 14;
const BODY_HEIGHT: i16 = 16;
/// Nothing is cut away over the character.
const NO_CEILING: i16 = 127;

/// The heights the world is cut at over the character, as the reference
/// client works them out: things from `max_z` up fade away, and so does land over
/// `max_ground_z`. Under a roof, the roofs fade too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Ceiling {
    pub max_z: i16,
    pub max_ground_z: i16,
    no_draw_roofs: bool,
}

impl Ceiling {
    pub fn open(draw_roofs: bool) -> Self {
        Self {
            max_z: NO_CEILING,
            max_ground_z: NO_CEILING,
            no_draw_roofs: !draw_roofs,
        }
    }
}

/// One thing on a tile, for the ceiling.
#[derive(Clone, Copy, Debug)]
struct Over {
    z: i16,
    flags: TileFlagSet,
}

/// The ceiling over a character at `own_z`, from the land and the things
/// on his tile and on the tile in front of him.
fn ceiling_of(
    own_z: i16,
    land: Option<i16>,
    here: &[Over],
    ahead: &[Over],
    draw_roofs: bool,
) -> Ceiling {
    let head = own_z + HEAD_ROOM;
    let top = own_z + BODY_HEIGHT;
    let mut ceiling = Ceiling::open(draw_roofs);
    if land.is_some_and(|land| top <= land) {
        // He is under the ground, in a cave or a dungeon.
        ceiling.max_ground_z = top;
        ceiling.max_z = top;
    } else {
        let solid = TileFlagSet::FOLIAGE | TileFlagSet::TRANSPARENT;
        for over in here {
            let blocks = over.flags.0 & solid.0 == 0
                && (!over.flags.contains(TileFlagSet::ROOF)
                    || over.flags.contains(TileFlagSet::SURFACE));
            if over.z > head && ceiling.max_z > over.z && blocks {
                ceiling.max_z = over.z;
                ceiling.no_draw_roofs = true;
            }
        }
    }
    let open_roof = TileFlagSet::TRANSPARENT | TileFlagSet::SURFACE;
    let mut near = ceiling.max_z;
    for over in ahead {
        let roof = over.flags.0 & open_roof.0 == 0 && over.flags.contains(TileFlagSet::ROOF);
        if over.z > head && ceiling.max_z > over.z && roof {
            ceiling.max_z = over.z;
            near = over.z;
            ceiling.no_draw_roofs = true;
        }
    }
    ceiling.max_z = near.max(top);
    ceiling
}

/// The circle of transparency round the character.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Circle {
    pub center: Point,
    pub radius: f32,
    pub style: CircleStyle,
}

/// A thing whose alpha fades: a tile of the map, a piece of land, or an
/// object of the shard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum FadeKey {
    Land { x: u16, y: u16 },
    Tile { x: u16, y: u16, z: i8, graphic: u16 },
    Serial(u32),
}

impl SceneState {
    /// The heights the world is cut at over the character, from what stands
    /// on his tile and on the tile in front of him.
    pub(super) fn ceiling_over(
        &self,
        art: &mut dyn WorldArt,
        frame: &WatchFrame,
        standing: &HashMap<(i32, i32), Vec<Standing<'_>>>,
    ) -> Ceiling {
        let draw_roofs = !self.look.general.hide_roofs;
        if !art.has_art() {
            return Ceiling::open(draw_roofs);
        }
        let mut over = |x: i32, y: i32| -> (Option<i16>, Vec<Over>) {
            let (Ok(tile_x), Ok(tile_y)) = (u16::try_from(x), u16::try_from(y)) else {
                return (None, Vec::new());
            };
            let mut land = None;
            let mut found = Vec::new();
            if let Some(cell) = art.cell(frame.map, tile_x, tile_y).ready() {
                land = cell.land_id.map(|_| i16::from(cell.average_z));
                found.extend(cell.statics.iter().map(|s| Over {
                    z: i16::from(s.z),
                    flags: s.flags,
                }));
            }
            for thing in standing.get(&(x, y)).into_iter().flatten() {
                if let Standing::Piece { graphic, z, .. } = thing {
                    found.push(Over {
                        z: *z as i16,
                        flags: art
                            .item_tile(*graphic)
                            .map_or(TileFlagSet::NONE, |tile| tile.flags),
                    });
                }
            }
            (land, found)
        };
        let (x, y) = (self.camera[0].round() as i32, self.camera[1].round() as i32);
        let (land, here) = over(x, y);
        let (_, ahead) = over(x + 1, y + 1);
        ceiling_of(
            self.camera[2].round() as i16,
            land,
            &here,
            &ahead,
            draw_roofs,
        )
    }

    /// The circle of transparency round the character this frame, when the
    /// General page turns it on.
    pub(super) fn circle_round(&self, view: Area) -> Option<Circle> {
        let general = &self.look.general;
        general.circle_of_transparency.then(|| Circle {
            center: self.screen_of(view, self.camera) - Vector::new(0.0, HALF_TILE * self.zoom),
            radius: f32::from(general.circle_radius),
            style: general.circle_style,
        })
    }

    /// How much of a fading thing shows this frame, as it moves toward
    /// `target`. A thing that shows whole is forgotten.
    pub(super) fn alpha_of(&mut self, key: FadeKey, target: f32) -> f32 {
        let before = self.fades.get(&key).map_or(1.0, |(alpha, _)| *alpha);
        let alpha = look::faded(
            before,
            target,
            self.frame_seconds,
            self.look.general.object_fading,
        );
        if alpha >= 1.0 {
            self.fades.remove(&key);
        } else {
            self.fades.insert(key, (alpha, self.frame_count));
        }
        alpha
    }

    /// How much of a thing at `z` should show: none over the ceiling or on
    /// a hidden roof, a part of a translucent one, and all of the rest.
    pub(super) fn target_alpha(&self, z: i16, flags: TileFlagSet) -> f32 {
        let ceiling = self.ceiling;
        if z >= ceiling.max_z || ceiling.no_draw_roofs && flags.contains(TileFlagSet::ROOF) {
            0.0
        } else if flags.contains(TileFlagSet::TRANSLUCENT) {
            look::TRANSLUCENT_ALPHA
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOF: TileFlagSet = TileFlagSet::ROOF;

    fn over(z: i16, flags: TileFlagSet) -> Over {
        Over { z, flags }
    }

    #[test]
    fn open_sky_cuts_nothing_and_a_floor_overhead_cuts_the_world_there() {
        let open = ceiling_of(0, Some(0), &[], &[], true);
        assert_eq!(open, Ceiling::open(true));
        let upstairs = ceiling_of(0, Some(0), &[over(20, TileFlagSet::SURFACE)], &[], true);
        assert_eq!(upstairs.max_z, 20);
        assert!(upstairs.no_draw_roofs, "under a floor the roofs go too");
        // A low thing over the feet cuts nothing.
        let table = ceiling_of(0, Some(0), &[over(10, TileFlagSet::SURFACE)], &[], true);
        assert_eq!(table.max_z, NO_CEILING);
    }

    #[test]
    fn a_roof_in_front_cuts_the_world_and_roofs_may_always_hide() {
        let porch = ceiling_of(0, Some(0), &[], &[over(30, ROOF)], true);
        assert_eq!((porch.max_z, porch.no_draw_roofs), (30, true));
        let always = ceiling_of(0, Some(0), &[], &[], false);
        assert!(always.no_draw_roofs);
        // Under the ground of a cave, the world is cut at the head.
        let cave = ceiling_of(0, Some(40), &[], &[], true);
        assert_eq!((cave.max_z, cave.max_ground_z), (BODY_HEIGHT, BODY_HEIGHT));
    }

    #[test]
    fn a_thing_faded_out_is_kept_and_a_whole_one_is_forgotten() {
        let mut scene = SceneState::new();
        scene.frame_seconds = 10.0;
        let key = FadeKey::Serial(7);
        assert_eq!(scene.alpha_of(key, 0.0), 0.0);
        assert!(scene.fades.contains_key(&key));
        assert_eq!(scene.alpha_of(key, 1.0), 1.0);
        assert!(!scene.fades.contains_key(&key));
    }
}
