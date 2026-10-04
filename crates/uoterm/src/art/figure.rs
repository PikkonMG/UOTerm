//! The picture of one mobile, put together from the animation files: the
//! mount, the body, and each worn item in the order the game paints them.
//! An outline in the color of his notoriety goes round the whole figure, so
//! he stays easy to find on busy ground.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use uoterm_nav::{
    mount_of, Action, AnimData, AnimFrame, Facing, HueData, TileData, TILE_PARTIAL_HUE,
};
use uoterm_protocol::types::{
    LAYER_ARMS, LAYER_BEARD, LAYER_BRACELET, LAYER_CLOAK, LAYER_EARRINGS, LAYER_FACE, LAYER_GLOVES,
    LAYER_HAIR, LAYER_HELMET, LAYER_LEGS, LAYER_NECKLACE, LAYER_ONE_HANDED, LAYER_PANTS,
    LAYER_RING, LAYER_ROBE, LAYER_SHIRT, LAYER_SHOES, LAYER_SKIRT, LAYER_TALISMAN, LAYER_TORSO,
    LAYER_TUNIC, LAYER_TWO_HANDED, LAYER_WAIST,
};
use uoterm_view::art::{mount_item, Paint, Picture, Pose};
use uoterm_view::frame::{WatchEquip, WatchLook};
use uoterm_view::geom::Vector;

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
type Frames = Arc<Vec<Option<AnimFrame>>>;

/// Which frames: the body, the way it faces, what it does, and if it rides.
type FramesOf = (u16, Facing, Action, bool);

/// How many bytes of frames the cache holds before it starts again. A web
/// page can ask for any body, so the cache must not grow without end. At
/// its fullest it holds this much and one more set of frames.
const FRAME_CACHE_BYTES: usize = 64 * 1024 * 1024;

/// Each set of frames the window or the web server has asked for. None
/// marks a body with no pictures. The web server lets many threads make
/// pictures from one `ClientArt` at a time, each with a shared borrow, so
/// the cache has its own lock.
#[derive(Default)]
pub struct FrameCache {
    known: Mutex<KnownFrames>,
}

#[derive(Default)]
struct KnownFrames {
    sets: HashMap<FramesOf, Option<Frames>>,
    /// The bytes of every set, as [`frame_set_bytes`] counts them.
    bytes: usize,
}

impl KnownFrames {
    /// The frames kept under `key`, read with `read` the first time. When
    /// the new set would make the cache hold more than `cap` bytes, the
    /// cache is emptied first.
    fn get_or_read(
        &mut self,
        key: FramesOf,
        read: impl FnOnce() -> Option<Frames>,
        cap: usize,
    ) -> Option<Frames> {
        if let Some(known) = self.sets.get(&key) {
            return known.clone();
        }
        let frames = read();
        let bytes = frame_set_bytes(&frames);
        if self.bytes + bytes > cap {
            self.sets.clear();
            self.bytes = 0;
        }
        self.bytes += bytes;
        self.sets.insert(key, frames.clone());
        frames
    }
}

/// The bytes a set of frames takes in the cache: its pixels, and its place
/// in the table, which a body with no frames takes too.
fn frame_set_bytes(frames: &Option<Frames>) -> usize {
    let pixels: usize = frames
        .iter()
        .flat_map(|frames| frames.iter().flatten())
        .map(|frame| frame.pixels.colors.len() * std::mem::size_of::<u16>())
        .sum();
    std::mem::size_of::<(FramesOf, Option<Frames>)>() + pixels
}

impl FrameCache {
    /// The frames kept under `key`, read with `read` the first time.
    fn get_or_read(&self, key: FramesOf, read: impl FnOnce() -> Option<Frames>) -> Option<Frames> {
        self.known
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_or_read(key, read, FRAME_CACHE_BYTES)
    }
}

pub struct Source<'a> {
    pub anim: &'a AnimData,
    pub hues: Option<&'a HueData>,
    pub tiledata: &'a TileData,
    pub cache: &'a FrameCache,
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
        self.cache.get_or_read((body, facing, action, mounted), || {
            self.anim
                .frames(body, facing, action, mounted)
                .map(Arc::new)
        })
    }

    /// The frame of a body for a pose. None when the body has no picture, or
    /// when this one frame is empty.
    fn frame(&self, body: u16, facing: Facing, pose: Pose, mounted: bool) -> Option<AnimFrame> {
        let frames = self.frames(body, facing, pose.action, mounted)?;
        frames[pose.tick % frames.len()].clone()
    }

    /// How many frames a body has for an action, facing `direction` and
    /// on a mount or not. A window makes one picture for each of them.
    pub fn cycle(&self, body: u16, direction: u8, action: Action, mounted: bool) -> usize {
        let facing = Facing::from_direction(direction);
        self.frames(shown_body(body), facing, action, mounted)
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
        whole_hue: Option<u16>,
    ) -> Option<Part> {
        let tile = self.tiledata.item(item.graphic)?;
        let own = tile.anim_id;
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
        let (hue, partial) = match whole_hue {
            Some(hue) => (hue, false),
            None => {
                let hue = if item.hue == 0 {
                    conv.map_or(0, |c| c.hue)
                } else {
                    item.hue
                };
                let partial = tile.flags.low_bits() & TILE_PARTIAL_HUE != 0;
                (hue, partial)
            }
        };
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
fn shown_body(body: u16) -> u16 {
    uoterm_world::body_when_alive(body).unwrap_or(body)
}

/// The parts of a figure in paint order. `whole_hue` paints every part in
/// one hue, as the client paints a mobile under the mouse or a ghost's
/// world.
fn parts(source: &Source<'_>, look: &WatchLook, pose: Pose, whole_hue: Option<u16>) -> Vec<Part> {
    let facing = Facing::from_direction(look.direction);
    let body = shown_body(look.body);
    // A mount whose body has no pictures is left out, and the rider stands.
    let mount = mount_item(look).and_then(|item| {
        let mount = mount_of(item.graphic)?;
        let frame = source.frame(mount.body, facing, pose, false)?;
        Some((item, mount, frame))
    });
    let rider_drop = mount.as_ref().map(|(_, m, _)| i32::from(m.rider_drop));
    let mut out = Vec::new();
    if let Some((item, _, frame)) = &mount {
        out.push(source.part(frame, facing, whole_hue.unwrap_or(item.hue), false, 0));
    }
    let Some(frame) = source.frame(body, facing, pose, rider_drop.is_some()) else {
        return Vec::new();
    };
    const HUE_PARTIAL_BIT: u16 = 0x8000;
    out.push(source.part(
        &frame,
        facing,
        whole_hue.unwrap_or(look.hue),
        whole_hue.is_none() && look.hue & HUE_PARTIAL_BIT != 0,
        rider_drop.unwrap_or(0),
    ));
    if !source.anim.is_person(body) {
        return out;
    }
    out.extend(
        worn_in_paint_order(look).into_iter().filter_map(|item| {
            source.worn_part(look.body, item, facing, pose, rider_drop, whole_hue)
        }),
    );
    out
}

/// The worn items painted, in paint order: one on each layer, the last the
/// list gives, as the client keeps one item on each layer. A covered layer
/// is left out.
fn worn_in_paint_order(look: &WatchLook) -> Vec<&WatchEquip> {
    paint_order(look.direction)
        .into_iter()
        .filter(|layer| !is_covered(*layer, &look.equipment))
        .filter_map(|layer| look.equipment.iter().rfind(|item| item.layer == layer))
        .collect()
}

/// Paints `part` on the canvas. The canvas starts at `origin`, counted from
/// the point on the tile.
fn paint_part(canvas: &mut [u8], canvas_width: usize, origin: (i32, i32), part: &Part) {
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
fn outline(canvas: &mut [u8], width: usize, height: usize, color: [u8; RGBA]) {
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
                canvas[at..at + RGBA].copy_from_slice(&color);
            }
        }
    }
}

pub fn compose(source: &Source<'_>, look: &WatchLook, pose: Pose, paint: Paint) -> Option<Picture> {
    let parts = parts(source, look, pose, paint.whole_hue);
    let margin = OUTLINE as i32;
    let left = parts.iter().map(|p| p.left).min()? - margin;
    let top = parts.iter().map(|p| p.top).min()? - margin;
    let right = parts.iter().map(|p| p.left + p.width as i32).max()? + margin;
    let bottom = parts.iter().map(|p| p.top + p.height as i32).max()? + margin;
    let (width, height) = ((right - left) as usize, (bottom - top) as usize);
    let mut rgba = vec![0u8; width * height * RGBA];
    for part in &parts {
        paint_part(&mut rgba, width, (left, top), part);
    }
    if paint.outline[ALPHA] > 0 {
        outline(&mut rgba, width, height, paint.outline);
    }
    Some(Picture {
        width,
        height,
        rgba,
        anchor: Vector::new(-left as f32, -top as f32),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_view::settings::WORN_LAYERS;

    const WEST: u8 = 6;

    #[test]
    fn the_frame_cache_starts_again_once_its_frames_fill_it() {
        const SIDE: usize = 10;
        const SETS_THAT_FIT: usize = 3;
        let set = || {
            let frame = AnimFrame {
                center_x: 0,
                center_y: 0,
                pixels: uoterm_nav::ArtPixels {
                    width: SIDE,
                    height: SIDE,
                    colors: vec![0; SIDE * SIDE],
                },
                file_hue: 0,
            };
            Some(Arc::new(vec![Some(frame)]))
        };
        let cap = SETS_THAT_FIT * frame_set_bytes(&set());
        let key = |body: u16| (body, Facing::from_direction(0), Action::Stand, false);
        let mut known = KnownFrames::default();
        for body in 0..=SETS_THAT_FIT as u16 {
            known.get_or_read(key(body), set, cap);
            assert!(known.bytes <= cap);
        }
        assert_eq!(known.sets.len(), 1, "the cache started again");
        assert!(known.sets.contains_key(&key(SETS_THAT_FIT as u16)));
        assert!(
            frame_set_bytes(&None) > 0,
            "a body with no frames counts too"
        );
    }

    #[test]
    fn one_item_is_painted_on_each_layer() {
        let item = |serial: u32, layer: u8| WatchEquip {
            serial,
            graphic: 1,
            layer,
            hue: 0,
        };
        let look = WatchLook {
            equipment: vec![
                item(1, LAYER_HELMET),
                item(2, LAYER_HELMET),
                item(3, LAYER_HELMET),
                item(4, LAYER_SHIRT),
            ],
            ..WatchLook::default()
        };
        let worn: Vec<u32> = worn_in_paint_order(&look)
            .iter()
            .map(|i| i.serial)
            .collect();
        assert_eq!(worn, [4, 3], "the shirt, then the last helmet");
    }

    #[test]
    fn real_files_draw_every_ghost_body() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        let anim = AnimData::open(&dir).expect("animation files open");
        for (ghost, alive) in uoterm_world::GHOST_BODIES {
            let shown = shown_body(ghost);
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
    fn every_painted_layer_has_a_name() {
        let mut painted = paint_order(WEST);
        painted.sort_unstable();
        let named: Vec<u8> = WORN_LAYERS.iter().map(|(layer, _)| *layer).collect();
        assert_eq!(painted, named);
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
        const WHITE: [u8; 4] = [u8::MAX; 4];
        paint_part(&mut canvas, width, (-1, -1), &part);
        outline(&mut canvas, width, height, WHITE);
        let pixel = |x: usize, y: usize| &canvas[(y * width + x) * RGBA..][..RGBA];
        assert_eq!(pixel(2, 1), RED);
        assert_eq!(pixel(1, 1), WHITE);
        assert_eq!(pixel(2, 0), WHITE);
        assert_eq!(pixel(0, 0), [0; 4]);
    }
}
