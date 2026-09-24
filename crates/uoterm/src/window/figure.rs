//! The picture of one mobile, put together from the animation files: the
//! mount, the body, and each worn item in the order the game paints them.
//! An outline in the color of his notoriety goes round the whole figure, so
//! he stays easy to find on busy ground.

use super::atlas::Picture;
use crate::view::{WatchEquip, WatchLook};
use eframe::egui::{Color32, Vec2};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use uoterm_nav::{
    mount_of, Action, AnimData, AnimFrame, Facing, HueData, MulMap, TILE_PARTIAL_HUE,
};
use uoterm_protocol::types::{
    LAYER_ARMS, LAYER_BEARD, LAYER_BRACELET, LAYER_CLOAK, LAYER_EARRINGS, LAYER_FACE, LAYER_GLOVES,
    LAYER_HAIR, LAYER_HELMET, LAYER_LEGS, LAYER_MOUNT, LAYER_NECKLACE, LAYER_ONE_HANDED,
    LAYER_PANTS, LAYER_RING, LAYER_ROBE, LAYER_SHIRT, LAYER_SHOES, LAYER_SKIRT, LAYER_TALISMAN,
    LAYER_TORSO, LAYER_TUNIC, LAYER_TWO_HANDED, LAYER_WAIST,
};

/// The order the game paints worn items, first to last. The cloak is not
/// here: its place depends on the way the mobile faces.
const PAINT_ORDER: [u8; 22] = [
    LAYER_SHIRT,
    LAYER_PANTS,
    LAYER_SHOES,
    LAYER_LEGS,
    LAYER_ARMS,
    LAYER_TORSO,
    LAYER_TUNIC,
    LAYER_RING,
    LAYER_BRACELET,
    LAYER_FACE,
    LAYER_GLOVES,
    LAYER_SKIRT,
    LAYER_ROBE,
    LAYER_WAIST,
    LAYER_NECKLACE,
    LAYER_HAIR,
    LAYER_BEARD,
    LAYER_EARRINGS,
    LAYER_HELMET,
    LAYER_ONE_HANDED,
    LAYER_TWO_HANDED,
    LAYER_TALISMAN,
];
const FACING_NORTH: u8 = 0;
const FACING_SOUTH_EAST: u8 = 3;

/// The game lifts each mobile this far above the center of his tile.
const LIFT: i32 = 3;
const OUTLINE: usize = 1;
const RGBA: usize = 4;
const ALPHA: usize = 3;

/// The frames of one body for one action, read from the files one time.
type Frames = Rc<Vec<Option<AnimFrame>>>;

/// Which frames: the body, the way it faces, what it does, and if it rides.
type FramesOf = (u16, Facing, Action, bool);

/// Each set of frames the window has asked for. None marks a body with no
/// pictures.
#[derive(Default)]
pub struct FrameCache {
    known: RefCell<HashMap<FramesOf, Option<Frames>>>,
}

pub struct Source<'a> {
    pub anim: &'a AnimData,
    pub hues: Option<&'a HueData>,
    pub tiledata: &'a MulMap,
    pub cache: &'a FrameCache,
}

/// Which picture of a mobile to make: what he does, and how far into it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pose {
    pub action: Action,
    /// The frame count since the window opened. Each part takes this modulo
    /// its own number of frames.
    pub tick: usize,
}

/// One part of the figure, placed from the point on the tile.
struct Part {
    left: i32,
    top: i32,
    width: usize,
    height: usize,
    rgba: Vec<u8>,
    mirrored: bool,
}

impl Source<'_> {
    fn frames(&self, body: u16, facing: Facing, action: Action, mounted: bool) -> Option<Frames> {
        self.cache
            .known
            .borrow_mut()
            .entry((body, facing, action, mounted))
            .or_insert_with(|| self.anim.frames(body, facing, action, mounted).map(Rc::new))
            .clone()
    }

    /// The frame of a body for a pose. None when the body has no picture, or
    /// when this one frame is empty.
    fn frame(&self, body: u16, facing: Facing, pose: Pose, mounted: bool) -> Option<AnimFrame> {
        let frames = self.frames(body, facing, pose.action, mounted)?;
        frames[pose.tick % frames.len()].clone()
    }

    /// How many frames the body of this look has for an action. The window
    /// makes one picture for each of them.
    pub fn cycle(&self, look: &WatchLook, action: Action) -> usize {
        let facing = Facing::from_direction(look.direction);
        let mounted = mount_item(look).is_some();
        self.frames(shown_body(look), facing, action, mounted)
            .map_or(1, |frames| frames.len())
    }

    /// `drop` moves the part down. A rider sits lower on some mounts.
    fn part(&self, frame: &AnimFrame, facing: Facing, hue: u16, partial: bool, drop: i32) -> Part {
        let hue = if hue == 0 { frame.file_hue } else { hue };
        let ramp = self.hues.and_then(|h| h.ramp(hue, partial));
        let width = frame.pixels.width;
        let left = if facing.mirrored {
            frame.center_x - width as i32
        } else {
            -frame.center_x
        };
        Part {
            left,
            top: -(frame.pixels.height as i32 + frame.center_y) - LIFT + drop,
            width,
            height: frame.pixels.height,
            rgba: frame.pixels.rgba(ramp),
            mirrored: facing.mirrored,
        }
    }

    fn worn_part(
        &self,
        body: u16,
        item: &WatchEquip,
        facing: Facing,
        pose: Pose,
        rider_drop: Option<i32>,
    ) -> Option<Part> {
        let own = self.tiledata.item_anim(item.graphic);
        if own == 0 {
            return None;
        }
        let conv = self.anim.equip_conv(body, own);
        let frame = self.frame(
            conv.map_or(own, |c| c.anim),
            facing,
            pose,
            rider_drop.is_some(),
        )?;
        let hue = if item.hue == 0 {
            conv.map_or(0, |c| c.hue)
        } else {
            item.hue
        };
        let partial = self.tiledata.item_flags(item.graphic) & TILE_PARTIAL_HUE != 0;
        Some(self.part(&frame, facing, hue, partial, rider_drop.unwrap_or(0)))
    }
}

/// The layers a worn item hides. A robe hides the chest and the arms. Leg
/// armor hides the pants and the shoes.
fn is_covered(layer: u8, worn: &[WatchEquip]) -> bool {
    let wears = |wanted: u8| worn.iter().any(|item| item.layer == wanted);
    match layer {
        LAYER_TORSO | LAYER_ARMS | LAYER_TUNIC => wears(LAYER_ROBE),
        LAYER_PANTS | LAYER_SHOES => wears(LAYER_LEGS),
        _ => false,
    }
}

/// The worn layers in paint order for one facing. A cloak is behind a mobile
/// who faces south-east, on top of one who faces north, and under the helmet
/// for the rest.
fn paint_order(direction: u8) -> Vec<u8> {
    let mut order = PAINT_ORDER.to_vec();
    let cloak_at = match direction {
        FACING_NORTH => order.len(),
        FACING_SOUTH_EAST => 0,
        _ => order
            .iter()
            .position(|layer| *layer == LAYER_HELMET)
            .unwrap_or(order.len()),
    };
    order.insert(cloak_at, LAYER_CLOAK);
    order
}

/// A ghost shows as the body he had, and the window makes him pale. A ghost
/// body has no pictures of its own in the client files.
fn shown_body(look: &WatchLook) -> u16 {
    uoterm_world::body_when_alive(look.body).unwrap_or(look.body)
}

fn mount_item(look: &WatchLook) -> Option<&WatchEquip> {
    look.equipment
        .iter()
        .find(|item| item.layer == LAYER_MOUNT && mount_of(item.graphic).is_some())
}

/// True when the mobile sits on a mount.
pub fn is_mounted(look: &WatchLook) -> bool {
    mount_item(look).is_some()
}

fn parts(source: &Source<'_>, look: &WatchLook, pose: Pose) -> Vec<Part> {
    let facing = Facing::from_direction(look.direction);
    let body = shown_body(look);
    // A mount whose body has no pictures is left out, and the rider stands.
    let mount = mount_item(look).and_then(|item| {
        let mount = mount_of(item.graphic)?;
        let frame = source.frame(mount.body, facing, pose, false)?;
        Some((item, mount, frame))
    });
    let rider_drop = mount.as_ref().map(|(_, m, _)| i32::from(m.rider_drop));
    let mut out = Vec::new();
    if let Some((item, _, frame)) = &mount {
        out.push(source.part(frame, facing, item.hue, false, 0));
    }
    let Some(frame) = source.frame(body, facing, pose, rider_drop.is_some()) else {
        return Vec::new();
    };
    const HUE_PARTIAL_BIT: u16 = 0x8000;
    out.push(source.part(
        &frame,
        facing,
        look.hue,
        look.hue & HUE_PARTIAL_BIT != 0,
        rider_drop.unwrap_or(0),
    ));
    if !source.anim.is_person(body) {
        return out;
    }
    for layer in paint_order(look.direction) {
        if is_covered(layer, &look.equipment) {
            continue;
        }
        let worn = look.equipment.iter().filter(|item| item.layer == layer);
        out.extend(
            worn.filter_map(|item| source.worn_part(look.body, item, facing, pose, rider_drop)),
        );
    }
    out
}

/// Paints `part` on the canvas. The canvas starts at `origin`, counted from
/// the point on the tile.
fn paint(canvas: &mut [u8], canvas_width: usize, origin: (i32, i32), part: &Part) {
    for row in 0..part.height {
        for column in 0..part.width {
            let from_column = if part.mirrored {
                part.width - 1 - column
            } else {
                column
            };
            let from = (row * part.width + from_column) * RGBA;
            if part.rgba[from + ALPHA] == 0 {
                continue;
            }
            let x = (part.left - origin.0) as usize + column;
            let y = (part.top - origin.1) as usize + row;
            let to = (y * canvas_width + x) * RGBA;
            canvas[to..to + RGBA].copy_from_slice(&part.rgba[from..from + RGBA]);
        }
    }
}

/// Colors each clear pixel that touches the figure.
fn outline(canvas: &mut [u8], width: usize, height: usize, color: Color32) {
    let solid: Vec<bool> = canvas.chunks_exact(RGBA).map(|p| p[ALPHA] != 0).collect();
    let touches = |x: usize, y: usize| {
        (x > 0 && solid[y * width + x - 1])
            || (x + 1 < width && solid[y * width + x + 1])
            || (y > 0 && solid[(y - 1) * width + x])
            || (y + 1 < height && solid[(y + 1) * width + x])
    };
    for y in 0..height {
        for x in 0..width {
            if !solid[y * width + x] && touches(x, y) {
                let at = (y * width + x) * RGBA;
                canvas[at..at + RGBA].copy_from_slice(&color.to_array());
            }
        }
    }
}

pub fn compose(
    source: &Source<'_>,
    look: &WatchLook,
    pose: Pose,
    outline_color: Color32,
) -> Option<Picture> {
    let parts = parts(source, look, pose);
    let margin = OUTLINE as i32;
    let left = parts.iter().map(|p| p.left).min()? - margin;
    let top = parts.iter().map(|p| p.top).min()? - margin;
    let right = parts.iter().map(|p| p.left + p.width as i32).max()? + margin;
    let bottom = parts.iter().map(|p| p.top + p.height as i32).max()? + margin;
    let (width, height) = ((right - left) as usize, (bottom - top) as usize);
    let mut rgba = vec![0u8; width * height * RGBA];
    for part in &parts {
        paint(&mut rgba, width, (left, top), part);
    }
    outline(&mut rgba, width, height, outline_color);
    Some(Picture {
        width,
        height,
        rgba,
        anchor: Vec2::new(-left as f32, -top as f32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEST: u8 = 6;

    #[test]
    fn real_files_draw_every_ghost_body() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        let anim = AnimData::open(&dir).expect("animation files open");
        for (ghost, alive) in uoterm_world::GHOST_BODIES {
            let shown = shown_body(&WatchLook {
                body: ghost,
                ..WatchLook::default()
            });
            assert_eq!(shown, alive, "ghost {ghost:#06x}");
            let frames = anim.frames(shown, Facing::from_direction(0), Action::Stand, false);
            assert!(frames.is_some(), "body {shown:#06x} for ghost {ghost:#06x}");
        }
    }

    #[test]
    fn the_cloak_moves_with_the_facing() {
        assert_eq!(paint_order(FACING_SOUTH_EAST).first(), Some(&LAYER_CLOAK));
        assert_eq!(paint_order(FACING_NORTH).last(), Some(&LAYER_CLOAK));
        let west = paint_order(WEST);
        let cloak = west.iter().position(|l| *l == LAYER_CLOAK).unwrap();
        assert_eq!(west[cloak + 1], LAYER_HELMET);
    }

    #[test]
    fn a_robe_hides_the_chest_armor() {
        let robe = [WatchEquip {
            layer: LAYER_ROBE,
            ..WatchEquip::default()
        }];
        assert!(is_covered(LAYER_TORSO, &robe));
        assert!(!is_covered(LAYER_HELMET, &robe));
        assert!(!is_covered(LAYER_TORSO, &[]));
    }

    #[test]
    fn the_outline_goes_round_the_figure_and_a_mirrored_part_is_flipped() {
        const RED: [u8; 4] = [255, 0, 0, 255];
        let part = Part {
            left: 0,
            top: 0,
            width: 2,
            height: 1,
            rgba: [RED, [0; 4]].concat(),
            mirrored: true,
        };
        let (width, height) = (4, 3);
        let mut canvas = vec![0u8; width * height * RGBA];
        paint(&mut canvas, width, (-1, -1), &part);
        outline(&mut canvas, width, height, Color32::WHITE);
        let pixel = |x: usize, y: usize| &canvas[(y * width + x) * RGBA..][..RGBA];
        assert_eq!(pixel(2, 1), RED);
        assert_eq!(pixel(1, 1), Color32::WHITE.to_array());
        assert_eq!(pixel(2, 0), Color32::WHITE.to_array());
        assert_eq!(pixel(0, 0), [0; 4]);
    }
}
