//! The map behind the panels: the land, the things on it, the people, and
//! the way to the walk goal. It draws the real pictures when the client
//! files are there, and flat colors from the radar when they are not.

use super::atlas::{Atlas, Sprite};
use super::audio::Step;
use super::client_art::{is_drawn, ClientArt};
use super::figure::{is_mounted, Pose};
use super::theme;
use crate::view::{
    WatchFrame, WatchItem, WatchLook, WatchMobile, SYM_BLOCK, SYM_DOOR, SYM_WALK, SYM_WATER,
};
use eframe::egui::{
    self,
    epaint::{Mesh, Vertex},
    Align2, Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2,
};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use uoterm_nav::Action;

/// Half the side of a tile picture. One tile step moves this far on each
/// screen axis.
const HALF_TILE: f32 = 22.0;
/// One unit of height lifts a thing this many pixels.
const Z_PIXELS: f32 = 4.0;
/// A thing this far above the feet of the character is over his head.
const OVERHEAD: i16 = 16;
const CORPSE_GRAPHIC: u16 = 0x2006;

const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 3.0;
const ZOOM_START: f32 = 1.0;
const ZOOM_PER_SCROLL_POINT: f32 = 0.0015;
/// One tile takes 0.1 s on a fast mount and 0.4 s on foot. A move is never
/// quicker or slower than these limits.
const GLIDE_MIN_SECONDS: f64 = 0.1;
const GLIDE_MAX_SECONDS: f64 = 0.5;
/// The next place comes a moment after a move ends. A mobile keeps his walk
/// for this long, so he does not stand still for one frame between two tiles.
const STEP_LINGER_SECONDS: f64 = 0.2;
/// A tile in less time than this is a run: on foot, and on a mount.
const RUN_TILE_SECONDS_ON_FOOT: f64 = 0.3;
const RUN_TILE_SECONDS_MOUNTED: f64 = 0.15;
/// How long one picture of each action shows.
const STAND_FRAME_SECONDS: f64 = 0.35;
/// The least time between two footstep sounds of one walker, as the game
/// client has it.
const STEP_GAP_ON_FOOT: f64 = 0.52;
const STEP_GAP_MOUNT_RUN: f64 = 0.195;
const STEP_GAP_MOUNT_WALK: f64 = 0.455;
/// A walker who made no step for this long is forgotten.
const STEP_MEMORY_SECONDS: f64 = 5.0;
/// The character has no serial in the frame. No mobile has this one.
const SELF_STEP_KEY: u32 = 0;
const WALK_FRAME_SECONDS: f64 = 0.09;
const RUN_FRAME_SECONDS: f64 = 0.06;
/// A jump longer than this is a teleport. The camera does not slide over it.
const TELEPORT_TILES: f32 = 12.0;

// Rows of tiles drawn past the window edge. Tall things and high ground
// reach up into the window from below it.
const MARGIN_ROWS_TOP: i32 = 4;
const MARGIN_ROWS_BOTTOM: i32 = 14;
const MARGIN_COLUMNS: i32 = 2;

const LAND_LIGHT_BASE: f32 = 0.84;
const LAND_LIGHT_SIDE: f32 = 0.030;
const LAND_LIGHT_FRONT: f32 = 0.015;
const LAND_LIGHT_MIN: f32 = 0.45;

const FLAT_BLOCK_HEIGHT: f32 = 16.0;
const FLAT_DOOR_HEIGHT: f32 = 5.0;
const FLAT_ITEM_RADIUS: f32 = 4.0;
const FLAT_GRID_EVERY: i32 = 8;
const FLAT_GRID_ALPHA: f32 = 0.05;

const PAWN_RING_RX: f32 = 15.0;
const PAWN_RING_RY: f32 = 7.5;
const PAWN_RING_WIDTH: f32 = 2.0;
const PAWN_RING_FILL_ALPHA: f32 = 0.28;
const PAWN_FOOT_HALF: f32 = 7.0;
const PAWN_SHOULDER_HALF: f32 = 5.0;
const PAWN_FOOT_Y: f32 = -3.0;
const PAWN_SHOULDER_Y: f32 = -36.0;
const PAWN_HEAD_Y: f32 = -44.0;
const PAWN_HEAD_RADIUS: f32 = 7.0;
const PAWN_OUTLINE: f32 = 1.5;
const PAWN_TOP_Y: f32 = -56.0;
const PAWN_HIDDEN_ALPHA: f32 = 0.45;
const PAWN_OUTLINE_COLOR: Color32 = Color32::from_rgba_premultiplied(4, 6, 10, 235);
const FACING_NEAR: f32 = 1.15;
const FACING_FAR: f32 = 1.9;
const FACING_HALF_WIDTH: f32 = 0.32;
/// The second ring of the character, drawn over everything so that a wall or
/// a neighbor never hides where he is.
const FOCUS_RING_GROW: f32 = 1.5;
const GHOST_ALPHA: f32 = 0.35;
const TARGET_RING_GROW: f32 = 1.45;
const ELLIPSE_POINTS: usize = 24;

// The search for the tile under the mouse. A floor above the character is
// drawn higher on the screen, so its tile is some rows further down the map.
const PICK_ROW_STEP: f32 = 0.5;
const PICK_ROWS_UP: i32 = -12;
const PICK_ROWS_DOWN: i32 = 40;

const PLATE_BAR_WIDTH: f32 = 46.0;
const PLATE_GAP: f32 = 3.0;
const PLATE_PAD: f32 = 3.0;
const PERCENT: f32 = 100.0;
const HAIRLINE: f32 = 1.0;

const PATH_DASH: f32 = 9.0;
const PATH_GAP: f32 = 7.0;
const PATH_WIDTH: f32 = 2.0;
const BEACON_HEIGHT: f32 = 46.0;
const BEACON_RING: f32 = 0.5;
const EDGE_INSET: f32 = 28.0;
const EDGE_MARK_RADIUS: f32 = 6.0;

pub const NOTE_NO_UOPATH: &str = "No client files. Give --uopath to see the real map.";

pub struct Scene {
    client: Option<ClientArt>,
    atlas: Option<Atlas>,
    note: String,
    /// Where the character is drawn now. The map is drawn round this point.
    camera: [f32; 3],
    camera_glide: Option<Glide>,
    camera_map: Option<u8>,
    zoom: f32,
    /// Where each mobile is drawn now, under its serial.
    actors: HashMap<u32, [f32; 3]>,
    glides: HashMap<u32, Glide>,
    /// The things drawn this frame, in paint order. The last one is on top.
    picks: Vec<Pick>,
    /// The time of the frame that is drawn now.
    now: f64,
    /// The footsteps the sound has not taken yet.
    steps: Vec<Step>,
    /// When each walker last made a footstep sound.
    last_step: HashMap<u32, f64>,
}

/// What kind of thing the mouse is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    Mobile,
    Item,
    Corpse,
}

/// One thing on the map that the mouse can point at.
#[derive(Clone, Debug)]
pub struct Pick {
    area: Rect,
    pub serial: u32,
    pub name: String,
    pub kind: PickKind,
}

/// A name and a health bar that float above one mobile.
struct Plate {
    top: Pos2,
    name: String,
    color: Color32,
    hits_percent: Option<u8>,
    target: bool,
}

/// How one mobile is drawn: what he looks like, what he does, his color.
struct Looks<'a> {
    look: &'a WatchLook,
    pose: Pose,
    color: Color32,
    alpha: f32,
}

/// The pose of a mobile the window has not seen move.
const STANDING: Pose = Pose {
    action: Action::Stand,
    tick: 0,
};

/// A thing that stands on one tile and is drawn in height order.
enum Standing<'a> {
    Art(&'a WatchItem),
    Corpse(&'a WatchItem),
    Mobile {
        mobile: &'a WatchMobile,
        at: [f32; 3],
    },
    Character {
        at: [f32; 3],
    },
}

impl Standing<'_> {
    fn z(&self) -> f32 {
        match self {
            Self::Art(item) | Self::Corpse(item) => f32::from(item.z),
            Self::Mobile { at, .. } | Self::Character { at } => at[2],
        }
    }
}

struct Canvas {
    mesh: Mesh,
    white: Pos2,
}

impl Canvas {
    fn vertex(&mut self, pos: Pos2, uv: Pos2, color: Color32) -> u32 {
        self.mesh.vertices.push(Vertex { pos, uv, color });
        self.mesh.vertices.len() as u32 - 1
    }

    fn quad(&mut self, points: [Pos2; 4], uvs: [Pos2; 4], color: Color32) {
        let first = self.vertex(points[0], uvs[0], color);
        for i in 1..points.len() {
            self.vertex(points[i], uvs[i], color);
        }
        self.mesh
            .indices
            .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }

    /// A filled shape with no picture. The points must go round a convex
    /// shape.
    fn fill(&mut self, points: &[Pos2], color: Color32) {
        let white = self.white;
        let first = self.vertex(points[0], white, color);
        for point in &points[1..] {
            self.vertex(*point, white, color);
        }
        for i in 1..points.len() as u32 - 1 {
            self.mesh.indices.extend([first, first + i, first + i + 1]);
        }
    }

    fn ring(&mut self, center: Pos2, radius: Vec2, width: f32, color: Color32) {
        let outer = ellipse(center, radius);
        let inner = ellipse(center, radius - Vec2::splat(width));
        for i in 0..ELLIPSE_POINTS {
            let next = (i + 1) % ELLIPSE_POINTS;
            self.fill(&[outer[i], outer[next], inner[next], inner[i]], color);
        }
    }

    fn sprite(&mut self, sprite: Sprite, rect: Rect, color: Color32) {
        let uv = sprite.uv;
        self.quad(
            [
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
            ],
            [
                uv.left_top(),
                uv.right_top(),
                uv.right_bottom(),
                uv.left_bottom(),
            ],
            color,
        );
    }
}

fn ellipse(center: Pos2, radius: Vec2) -> Vec<Pos2> {
    (0..ELLIPSE_POINTS)
        .map(|i| {
            let angle = i as f32 / ELLIPSE_POINTS as f32 * std::f32::consts::TAU;
            center + Vec2::new(angle.cos() * radius.x, angle.sin() * radius.y)
        })
        .collect()
}

/// The way the character faces, as one step on the tile grid.
fn facing_step(facing: &str) -> Option<(f32, f32)> {
    Some(match facing {
        "north" => (0.0, -1.0),
        "northeast" => (1.0, -1.0),
        "east" => (1.0, 0.0),
        "southeast" => (1.0, 1.0),
        "south" => (0.0, 1.0),
        "southwest" => (-1.0, 1.0),
        "west" => (-1.0, 0.0),
        "northwest" => (-1.0, -1.0),
        _ => return None,
    })
}

/// A steady move from one place to the next. The session tells where a
/// thing is a few times each second. A thing that walks changes its tile at
/// an even pace, so the move to the new tile takes as long as the last tile
/// took. The thing then moves without a stop between two tiles.
#[derive(Clone, Copy, Debug)]
struct Glide {
    from: [f32; 3],
    to: [f32; 3],
    started: f64,
    seconds: f64,
}

impl Glide {
    fn resting(at: [f32; 3], time: f64) -> Self {
        Self {
            from: at,
            to: at,
            started: time,
            seconds: GLIDE_MIN_SECONDS,
        }
    }

    fn at(&self, time: f64) -> [f32; 3] {
        let share = ((time - self.started) / self.seconds).clamp(0.0, 1.0) as f32;
        [0, 1, 2].map(|i| self.from[i] + (self.to[i] - self.from[i]) * share)
    }

    fn moving(&self, time: f64) -> bool {
        self.from != self.to && time - self.started < self.seconds
    }

    /// What the mobile does now, and which picture of it shows.
    fn pose(&self, time: f64, mounted: bool) -> Pose {
        let stepping =
            self.from != self.to && time - self.started < self.seconds + STEP_LINGER_SECONDS;
        let tiles = (self.to[0] - self.from[0])
            .abs()
            .max((self.to[1] - self.from[1]).abs())
            .max(1.0);
        let run_under = if mounted {
            RUN_TILE_SECONDS_MOUNTED
        } else {
            RUN_TILE_SECONDS_ON_FOOT
        };
        let (action, frame_seconds) = if !stepping {
            (Action::Stand, STAND_FRAME_SECONDS)
        } else if self.seconds / f64::from(tiles) < run_under {
            (Action::Run, RUN_FRAME_SECONDS)
        } else {
            (Action::Walk, WALK_FRAME_SECONDS)
        };
        Pose {
            action,
            tick: (time / frame_seconds) as usize,
        }
    }

    /// Starts a new move when the goal changed. A jump is not a move.
    fn aim(&mut self, goal: [f32; 3], time: f64) {
        if goal == self.to {
            return;
        }
        if far_apart(self.to, goal) {
            *self = Self::resting(goal, time);
            return;
        }
        *self = Self {
            from: self.at(time),
            to: goal,
            started: time,
            seconds: (time - self.started).clamp(GLIDE_MIN_SECONDS, GLIDE_MAX_SECONDS),
        };
    }
}

fn far_apart(a: [f32; 3], b: [f32; 3]) -> bool {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs()) > TELEPORT_TILES
}

impl Scene {
    pub fn new(uopath: Option<&Path>) -> Self {
        let (client, note) = match uopath.map(ClientArt::open) {
            Some(Ok(client)) => (Some(client), String::new()),
            Some(Err(e)) => (None, format!("No real map: {e}.")),
            None => (None, NOTE_NO_UOPATH.to_string()),
        };
        Self {
            client,
            atlas: None,
            note,
            camera: [0.0; 3],
            camera_glide: None,
            camera_map: None,
            zoom: ZOOM_START,
            actors: HashMap::new(),
            glides: HashMap::new(),
            picks: Vec::new(),
            now: 0.0,
            steps: Vec::new(),
            last_step: HashMap::new(),
        }
    }

    /// Why the map shows flat colors. Empty when it shows the real map.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Draws the map into `rect`. True while something still moves, so the
    /// window must draw the next frame at once.
    pub fn draw(&mut self, ui: &egui::Ui, rect: Rect, frame: &WatchFrame, time: f64) -> bool {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, theme::VOID);
        self.read_zoom(ui, rect);
        self.now = time;
        let moving = self.follow(frame, time);
        // A mobile that stands still moves a little. Wake for its next picture.
        if self.client.is_some() {
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(STAND_FRAME_SECONDS));
        }
        let atlas = self.atlas.get_or_insert_with(|| Atlas::new(ui.ctx()));
        let mut canvas = Canvas {
            mesh: Mesh::with_texture(atlas.texture_id()),
            white: atlas.white_uv(),
        };
        let mut plates = Vec::new();
        self.build(rect, frame, &mut canvas, &mut plates);
        painter.add(Shape::mesh(canvas.mesh));
        self.draw_walk_goal(&painter, rect, frame);
        let character = self.project(rect, self.camera);
        self.focus_ring(&painter, character);
        draw_plates(&painter, plates, self.figure_rect(character), self.zoom);
        moving
    }

    fn read_zoom(&mut self, ui: &egui::Ui, rect: Rect) {
        let scroll = ui.input(|i| {
            let over = i.pointer.hover_pos().is_some_and(|p| rect.contains(p));
            if over {
                i.smooth_scroll_delta.y
            } else {
                0.0
            }
        });
        self.zoom = (self.zoom * (1.0 + scroll * ZOOM_PER_SCROLL_POINT)).clamp(ZOOM_MIN, ZOOM_MAX);
    }

    /// Moves the camera and every mobile toward where the frame says it is.
    fn follow(&mut self, frame: &WatchFrame, time: f64) -> bool {
        let goal = [f32::from(frame.x), f32::from(frame.y), f32::from(frame.z)];
        let same_map = self.camera_map == Some(frame.map);
        self.camera_map = Some(frame.map);
        let glide = match self.camera_glide.filter(|_| same_map) {
            Some(mut glide) => {
                glide.aim(goal, time);
                glide
            }
            None => Glide::resting(goal, time),
        };
        if self
            .camera_glide
            .is_some_and(|before| before.to != glide.to)
            && !frame.dead
            && !frame.hidden
        {
            self.step(SELF_STEP_KEY, &frame.look, &glide, 0.0, time);
        }
        self.camera_glide = Some(glide);
        self.camera = glide.at(time);
        let mut moving = glide.moving(time);
        let mut glides = HashMap::with_capacity(frame.mobiles.len());
        self.actors.clear();
        for mobile in &frame.mobiles {
            let goal = [
                f32::from(mobile.x),
                f32::from(mobile.y),
                f32::from(mobile.z),
            ];
            let glide = match self.glides.get(&mobile.serial).filter(|_| same_map) {
                Some(known) => {
                    let mut glide = *known;
                    glide.aim(goal, time);
                    if glide.to != known.to {
                        self.step(
                            mobile.serial,
                            &mobile.look,
                            &glide,
                            f32::from(mobile.dist),
                            time,
                        );
                    }
                    glide
                }
                None => Glide::resting(goal, time),
            };
            moving |= glide.moving(time);
            self.actors.insert(mobile.serial, glide.at(time));
            glides.insert(mobile.serial, glide);
        }
        self.glides = glides;
        self.last_step
            .retain(|_, at| time - *at < STEP_MEMORY_SECONDS);
        moving
    }

    /// Notes one footstep for the sound, when the walker is a person and his
    /// last footstep is long enough ago. A jump makes no footstep.
    fn step(&mut self, walker: u32, look: &WatchLook, glide: &Glide, tiles_away: f32, time: f64) {
        let person = self
            .client
            .as_ref()
            .is_some_and(|client| client.is_person(look.body));
        if !person || glide.from == glide.to {
            return;
        }
        let mounted = is_mounted(look);
        let running = glide.pose(time, mounted).action == Action::Run;
        let gap = match (mounted, running) {
            (true, true) => STEP_GAP_MOUNT_RUN,
            (true, false) => STEP_GAP_MOUNT_WALK,
            (false, _) => STEP_GAP_ON_FOOT,
        };
        if self
            .last_step
            .get(&walker)
            .is_some_and(|at| time - at < gap)
        {
            return;
        }
        self.last_step.insert(walker, time);
        self.steps.push(Step {
            tiles_away,
            mounted,
            running,
        });
    }

    /// The footsteps since the last call, for the sound.
    pub fn take_steps(&mut self) -> Vec<Step> {
        std::mem::take(&mut self.steps)
    }

    fn land_sprite(&mut self, land_id: u16) -> Option<Sprite> {
        self.client
            .as_ref()?
            .land_sprite(self.atlas.as_mut()?, land_id)
    }

    fn item_sprite(&mut self, map: u8, graphic: u16, hue: u16) -> Option<Sprite> {
        self.client
            .as_ref()?
            .item_sprite(self.atlas.as_mut()?, map, graphic, hue)
    }

    fn project(&self, rect: Rect, at: [f32; 3]) -> Pos2 {
        let dx = at[0] - self.camera[0];
        let dy = at[1] - self.camera[1];
        let dz = at[2] - self.camera[2];
        rect.center()
            + Vec2::new((dx - dy) * HALF_TILE, (dx + dy) * HALF_TILE - dz * Z_PIXELS) * self.zoom
    }

    /// The things that stand on each tile this frame.
    fn standing<'a>(&self, frame: &'a WatchFrame) -> HashMap<(i32, i32), Vec<Standing<'a>>> {
        let mut out: HashMap<(i32, i32), Vec<Standing<'a>>> = HashMap::new();
        let tile = |at: [f32; 3]| (at[0].round() as i32, at[1].round() as i32);
        for item in frame.items.iter().filter(|i| is_drawn(i.graphic)) {
            let thing = if item.graphic == CORPSE_GRAPHIC {
                Standing::Corpse(item)
            } else {
                Standing::Art(item)
            };
            out.entry((i32::from(item.x), i32::from(item.y)))
                .or_default()
                .push(thing);
        }
        for mobile in &frame.mobiles {
            let at = self.actors[&mobile.serial];
            out.entry(tile(at))
                .or_default()
                .push(Standing::Mobile { mobile, at });
        }
        out.entry(tile(self.camera))
            .or_default()
            .push(Standing::Character { at: self.camera });
        out
    }

    /// The lowest thing over the head of the character. Everything from
    /// there up is left out, so a roof does not hide him.
    fn ceiling(&mut self, frame: &WatchFrame) -> Option<i16> {
        let client = self.client.as_mut()?;
        let head = i16::from(frame.z) + OVERHEAD;
        let mut lowest: Option<i16> = None;
        for (x, y) in [
            (frame.x, frame.y),
            (frame.x.saturating_add(1), frame.y.saturating_add(1)),
        ] {
            let Some(cell) = client.cell(frame.map, x, y) else {
                continue;
            };
            for z in cell.statics.iter().map(|s| i16::from(s.z)) {
                if z >= head {
                    lowest = Some(lowest.map_or(z, |found| found.min(z)));
                }
            }
        }
        lowest
    }

    fn build(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        self.picks.clear();
        let ceiling = self.ceiling(frame);
        let mut standing = self.standing(frame);
        let half = HALF_TILE * self.zoom;
        let rows = (rect.height() / 2.0 / half).ceil() as i32;
        let columns = (rect.width() / 2.0 / half).ceil() as i32 + MARGIN_COLUMNS;
        let center = (self.camera[0].round() as i32, self.camera[1].round() as i32);
        let radar_half = frame.radar.len() as i32 / 2;
        for row in -rows - MARGIN_ROWS_TOP..=rows + MARGIN_ROWS_BOTTOM {
            for column in (-columns..=columns).filter(|c| (c + row) % 2 == 0) {
                let (dx, dy) = ((row + column) / 2, (row - column) / 2);
                let (x, y) = (center.0 + dx, center.1 + dy);
                let (Ok(tile_x), Ok(tile_y)) = (u16::try_from(x), u16::try_from(y)) else {
                    continue;
                };
                let things = standing.remove(&(x, y)).unwrap_or_default();
                if self.client.is_some() {
                    self.real_tile(rect, frame, (tile_x, tile_y), ceiling, canvas);
                } else {
                    let symbol = radar_symbol(
                        frame,
                        radar_half + i32::from(tile_x) - i32::from(frame.x),
                        radar_half + i32::from(tile_y) - i32::from(frame.y),
                    );
                    self.flat_tile(rect, (tile_x, tile_y), symbol, canvas);
                }
                self.things(
                    rect,
                    frame,
                    (tile_x, tile_y),
                    things,
                    ceiling,
                    canvas,
                    plates,
                );
            }
        }
    }

    fn real_tile(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        (x, y): (u16, u16),
        ceiling: Option<i16>,
        canvas: &mut Canvas,
    ) {
        let Some(cell) = self
            .client
            .as_mut()
            .and_then(|client| client.cell(frame.map, x, y).cloned())
        else {
            return;
        };
        let (fx, fy) = (f32::from(x), f32::from(y));
        let under_ceiling = |z: i16| ceiling.is_none_or(|c| z < c);
        if let Some(sprite) = cell
            .land_id
            .filter(|_| under_ceiling(i16::from(cell.corners[0])))
            .and_then(|id| self.land_sprite(id))
        {
            let [top, right, bottom, left] = cell.corners.map(f32::from);
            let light = (LAND_LIGHT_BASE
                + LAND_LIGHT_SIDE * (right - left)
                + LAND_LIGHT_FRONT * (top - bottom))
                .clamp(LAND_LIGHT_MIN, 1.0);
            let uv = sprite.uv;
            canvas.quad(
                [
                    self.project(rect, [fx - 0.5, fy - 0.5, top]),
                    self.project(rect, [fx + 0.5, fy - 0.5, right]),
                    self.project(rect, [fx + 0.5, fy + 0.5, bottom]),
                    self.project(rect, [fx - 0.5, fy + 0.5, left]),
                ],
                [
                    Pos2::new(uv.center().x, uv.top()),
                    Pos2::new(uv.right(), uv.center().y),
                    Pos2::new(uv.center().x, uv.bottom()),
                    Pos2::new(uv.left(), uv.center().y),
                ],
                Color32::WHITE.gamma_multiply(light).to_opaque(),
            );
        }
        for piece in cell
            .statics
            .iter()
            .filter(|s| under_ceiling(i16::from(s.z)))
        {
            let at = [fx, fy, f32::from(piece.z)];
            self.item_art(rect, frame.map, at, piece.graphic, piece.hue, canvas);
        }
    }

    /// Where the picture of an item goes. Its bottom edge lies on the bottom
    /// point of its tile, and it is centered on the tile.
    fn art_rect(&self, tile_center: Pos2, sprite: Sprite) -> Rect {
        let size = Vec2::new(sprite.width, sprite.height) * self.zoom;
        let bottom = tile_center.y + HALF_TILE * self.zoom;
        Rect::from_min_size(
            Pos2::new(tile_center.x - size.x / 2.0, bottom - size.y),
            size,
        )
    }

    fn flat_tile(&self, rect: Rect, (x, y): (u16, u16), symbol: Option<char>, canvas: &mut Canvas) {
        let (fx, fy) = (f32::from(x), f32::from(y));
        let corner = |dx: f32, dy: f32, lift: f32| {
            self.project(rect, [fx + dx, fy + dy, self.camera[2]])
                - Vec2::new(0.0, lift * self.zoom)
        };
        let diamond = |lift: f32| {
            [
                corner(-0.5, -0.5, lift),
                corner(0.5, -0.5, lift),
                corner(0.5, 0.5, lift),
                corner(-0.5, 0.5, lift),
            ]
        };
        let (color, height) = match symbol {
            None => (theme::FLAT_UNKNOWN, 0.0),
            Some(SYM_BLOCK) => (theme::FLAT_BLOCK_TOP, FLAT_BLOCK_HEIGHT),
            Some(SYM_DOOR) => (theme::FLAT_DOOR, FLAT_DOOR_HEIGHT),
            Some(SYM_WATER) => (theme::FLAT_WATER, 0.0),
            _ if (x + y) % 2 == 0 => (theme::FLAT_WALK, 0.0),
            _ => (theme::FLAT_WALK_ALT, 0.0),
        };
        if height > 0.0 {
            let [_, right, bottom, left] = diamond(0.0);
            let [_, top_right, top_bottom, top_left] = diamond(height);
            canvas.fill(
                &[top_left, top_bottom, bottom, left],
                theme::FLAT_BLOCK_LEFT,
            );
            canvas.fill(
                &[top_bottom, top_right, right, bottom],
                theme::FLAT_BLOCK_RIGHT,
            );
        }
        canvas.fill(&diamond(height), color);
        let grid = Color32::WHITE.gamma_multiply(FLAT_GRID_ALPHA);
        let [top, right, _, left] = diamond(height);
        let hair = Vec2::new(0.0, HAIRLINE);
        if i32::from(y) % FLAT_GRID_EVERY == 0 {
            canvas.fill(&[top, right, right + hair, top + hair], grid);
        }
        if i32::from(x) % FLAT_GRID_EVERY == 0 {
            canvas.fill(&[left, top, top + hair, left + hair], grid);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn things(
        &mut self,
        rect: Rect,
        frame: &WatchFrame,
        tile: (u16, u16),
        mut things: Vec<Standing<'_>>,
        ceiling: Option<i16>,
        canvas: &mut Canvas,
        plates: &mut Vec<Plate>,
    ) {
        let (x, y) = (f32::from(tile.0), f32::from(tile.1));
        things.sort_by(|a, b| a.z().total_cmp(&b.z()));
        for thing in things {
            match thing {
                Standing::Art(item) => {
                    if ceiling.is_some_and(|c| i16::from(item.z) >= c) {
                        continue;
                    }
                    let at = [x, y, f32::from(item.z)];
                    let drawn = self.item_art(rect, frame.map, at, item.graphic, item.hue, canvas);
                    self.picks.extend(drawn.map(|area| Pick {
                        area,
                        serial: item.serial,
                        name: item.name.clone(),
                        kind: PickKind::Item,
                    }));
                }
                Standing::Corpse(item) => {
                    let center = self.project(rect, [x, y, f32::from(item.z)]);
                    let radius = Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * self.zoom;
                    canvas.ring(center, radius, PAWN_RING_WIDTH * self.zoom, theme::CORPSE);
                    self.picks.push(Pick {
                        area: Rect::from_center_size(center, radius * 2.0),
                        serial: item.serial,
                        name: item.name.clone(),
                        kind: PickKind::Corpse,
                    });
                }
                Standing::Mobile { mobile, at } => {
                    let target = !frame.combatant.is_empty() && mobile.name == frame.combatant;
                    let color = theme::notoriety_color(mobile.notoriety);
                    let foot = self.project(rect, at);
                    let pose = self.glides.get(&mobile.serial).map_or(STANDING, |glide| {
                        glide.pose(self.now, is_mounted(&mobile.look))
                    });
                    let look = Looks {
                        look: &mobile.look,
                        pose,
                        color,
                        alpha: 1.0,
                    };
                    let area = self.figure(canvas, frame.map, look, foot, target);
                    self.picks.push(Pick {
                        area,
                        serial: mobile.serial,
                        name: mobile.name.clone(),
                        kind: PickKind::Mobile,
                    });
                    plates.push(Plate {
                        top: area.center_top(),
                        name: mobile.name.clone(),
                        color,
                        hits_percent: mobile.hits_percent,
                        target,
                    });
                }
                Standing::Character { at } => {
                    let foot = self.project(rect, at);
                    let alpha = if frame.dead {
                        GHOST_ALPHA
                    } else if frame.hidden {
                        PAWN_HIDDEN_ALPHA
                    } else {
                        1.0
                    };
                    if frame.dead {
                        let radius = Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * self.zoom;
                        canvas.ring(foot, radius, PAWN_RING_WIDTH * self.zoom, theme::CORPSE);
                    } else {
                        self.facing_mark(canvas, foot, &frame.facing);
                    }
                    let pose = self.camera_glide.map_or(STANDING, |glide| {
                        glide.pose(self.now, is_mounted(&frame.look))
                    });
                    let look = Looks {
                        look: &frame.look,
                        pose,
                        color: theme::SELF_FIGURE,
                        alpha,
                    };
                    self.figure(canvas, frame.map, look, foot, false);
                }
            }
        }
    }

    fn item_art(
        &mut self,
        rect: Rect,
        map: u8,
        at: [f32; 3],
        graphic: u16,
        hue: u16,
        canvas: &mut Canvas,
    ) -> Option<Rect> {
        let tile_center = self.project(rect, at);
        if let Some(sprite) = self.item_sprite(map, graphic, hue) {
            let area = self.art_rect(tile_center, sprite);
            canvas.sprite(sprite, area, Color32::WHITE);
            return Some(area);
        }
        if self.client.is_some() {
            return None;
        }
        let radius = Vec2::new(FLAT_ITEM_RADIUS * 2.0, FLAT_ITEM_RADIUS) * self.zoom;
        canvas.fill(&ellipse(tile_center, radius), theme::FLAT_ITEM);
        Some(Rect::from_center_size(tile_center, radius * 2.0))
    }

    /// One mobile: a ring on the ground in `color`, and on it his real
    /// picture with an outline in `color`. When the client files hold no
    /// picture of him, a plain figure stands in. Gives the area he covers.
    fn figure(
        &mut self,
        canvas: &mut Canvas,
        map: u8,
        looks: Looks<'_>,
        foot: Pos2,
        target: bool,
    ) -> Rect {
        let Looks {
            look,
            pose,
            color,
            alpha,
        } = looks;
        let zoom = self.zoom;
        let ring = Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * zoom;
        if target {
            canvas.ring(
                foot,
                ring * TARGET_RING_GROW,
                PAWN_RING_WIDTH * zoom,
                theme::with_alpha(theme::ALARM, alpha),
            );
        }
        canvas.fill(
            &ellipse(foot, ring),
            theme::with_alpha(color, alpha * PAWN_RING_FILL_ALPHA),
        );
        canvas.ring(
            foot,
            ring,
            PAWN_RING_WIDTH * zoom,
            theme::with_alpha(color, alpha),
        );
        let sprite = self
            .client
            .as_ref()
            .zip(self.atlas.as_mut())
            .and_then(|(client, atlas)| client.figure_sprite(atlas, map, look, pose, color));
        let Some(sprite) = sprite else {
            self.plain_figure(canvas, foot, color, alpha);
            return self.figure_rect(foot);
        };
        let area = Rect::from_min_size(
            foot - sprite.anchor * zoom,
            Vec2::new(sprite.width, sprite.height) * zoom,
        );
        canvas.sprite(sprite, area, theme::with_alpha(Color32::WHITE, alpha));
        area
    }

    /// A body and a head in one color, for a mobile with no real picture.
    fn plain_figure(&self, canvas: &mut Canvas, foot: Pos2, color: Color32, alpha: f32) {
        let zoom = self.zoom;
        let body = |grow: f32| {
            [
                foot + Vec2::new(-PAWN_FOOT_HALF - grow, PAWN_FOOT_Y + grow) * zoom,
                foot + Vec2::new(PAWN_FOOT_HALF + grow, PAWN_FOOT_Y + grow) * zoom,
                foot + Vec2::new(PAWN_SHOULDER_HALF + grow, PAWN_SHOULDER_Y - grow) * zoom,
                foot + Vec2::new(-PAWN_SHOULDER_HALF - grow, PAWN_SHOULDER_Y - grow) * zoom,
            ]
        };
        let head = |grow: f32| {
            ellipse(
                foot + Vec2::new(0.0, PAWN_HEAD_Y) * zoom,
                Vec2::splat((PAWN_HEAD_RADIUS + grow) * zoom),
            )
        };
        let outline = theme::with_alpha(PAWN_OUTLINE_COLOR, alpha);
        canvas.fill(&body(PAWN_OUTLINE), outline);
        canvas.fill(&head(PAWN_OUTLINE), outline);
        canvas.fill(&body(0.0), theme::with_alpha(color, alpha));
        canvas.fill(&head(0.0), theme::with_alpha(color, alpha));
    }

    /// The screen area of one figure, from the top of its head to its feet.
    fn figure_rect(&self, foot: Pos2) -> Rect {
        Rect::from_min_max(
            foot + Vec2::new(-PAWN_RING_RX, PAWN_TOP_Y) * self.zoom,
            foot + Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * self.zoom,
        )
    }

    fn focus_ring(&self, painter: &Painter, foot: Pos2) {
        let radius = Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * FOCUS_RING_GROW * self.zoom;
        painter.add(Shape::closed_line(
            ellipse(foot, radius),
            Stroke::new(PAWN_RING_WIDTH * self.zoom, theme::SELF_FIGURE),
        ));
    }

    /// The picture of an item, for a window that is not the map.
    pub fn item_picture(
        &mut self,
        map: u8,
        graphic: u16,
        hue: u16,
    ) -> Option<(egui::TextureId, Sprite)> {
        let sprite = self.item_sprite(map, graphic, hue)?;
        Some((self.atlas.as_ref()?.texture_id(), sprite))
    }

    /// The thing on top under the mouse.
    pub fn thing_at(&self, mouse: Pos2) -> Option<&Pick> {
        self.picks
            .iter()
            .rev()
            .find(|pick| pick.area.contains(mouse))
    }

    /// The tile under the mouse, and the height of its floor. The ground is
    /// not flat, so the tile is the nearest one whose own floor lies under
    /// the mouse. With no client files the floor of the character serves.
    pub fn tile_at(&mut self, rect: Rect, frame: &WatchFrame, mouse: Pos2) -> (u16, u16, i8) {
        let on_plane = (mouse - rect.center()) / (HALF_TILE * self.zoom);
        let camera = self.camera;
        let tile_on_row = |rows_down: f32| {
            let sum = on_plane.y + rows_down;
            let x = camera[0] + (on_plane.x + sum) / 2.0;
            let y = camera[1] + (sum - on_plane.x) / 2.0;
            (x.round().max(0.0) as u16, y.round().max(0.0) as u16)
        };
        let level = (tile_on_row(0.0), frame.z);
        let mut best = level;
        for step in PICK_ROWS_UP..=PICK_ROWS_DOWN {
            let rows_down = step as f32 * PICK_ROW_STEP;
            let (x, y) = tile_on_row(rows_down);
            let Some(floor) = self.floor_near(frame, x, y) else {
                continue;
            };
            let lift_rows = (f32::from(floor) - camera[2]) * Z_PIXELS / HALF_TILE;
            if (lift_rows - rows_down).abs() <= PICK_ROW_STEP {
                best = ((x, y), floor);
            }
        }
        (best.0 .0, best.0 .1, best.1)
    }

    /// The floor of a tile that is nearest to the height of the character.
    fn floor_near(&mut self, frame: &WatchFrame, x: u16, y: u16) -> Option<i8> {
        let cell = self.client.as_mut()?.cell(frame.map, x, y)?;
        let land = cell.land_id.map(|_| i16::from(cell.corners[0]));
        let here = i16::from(frame.z);
        cell.statics
            .iter()
            .filter_map(|s| s.floor)
            .chain(land)
            .min_by_key(|floor| (floor - here).abs())
            .map(|floor| floor.clamp(i16::from(i8::MIN), i16::from(i8::MAX)) as i8)
    }

    /// A wedge on the ground that points the way the character faces.
    fn facing_mark(&self, canvas: &mut Canvas, foot: Pos2, facing: &str) {
        let Some((dx, dy)) = facing_step(facing) else {
            return;
        };
        let length = (dx * dx + dy * dy).sqrt();
        let (dx, dy) = (dx / length, dy / length);
        let on_ground = |along: f32, across: f32| {
            let (x, y) = (dx * along - dy * across, dy * along + dx * across);
            foot + Vec2::new((x - y) * HALF_TILE, (x + y) * HALF_TILE) * self.zoom
        };
        canvas.fill(
            &[
                on_ground(FACING_FAR, 0.0),
                on_ground(FACING_NEAR, FACING_HALF_WIDTH),
                on_ground(FACING_NEAR, -FACING_HALF_WIDTH),
            ],
            theme::SELF_FIGURE,
        );
    }

    /// A dashed line on the ground to the walk goal, and a beacon on it.
    /// When the goal is outside the window, a mark on the edge points to it.
    fn draw_walk_goal(&self, painter: &Painter, rect: Rect, frame: &WatchFrame) {
        let (Some(x), Some(y)) = (frame.dest_x, frame.dest_y) else {
            return;
        };
        let from = self.project(rect, self.camera);
        let goal = self.project(rect, [f32::from(x), f32::from(y), self.camera[2]]);
        let inside = rect.shrink(EDGE_INSET);
        let end = if inside.contains(goal) {
            goal
        } else {
            clamp_to(inside, from, goal)
        };
        let stroke = Stroke::new(PATH_WIDTH, theme::GOAL);
        painter.extend(Shape::dashed_line(
            &[from, end],
            stroke,
            PATH_DASH,
            PATH_GAP,
        ));
        if !inside.contains(goal) {
            painter.circle_filled(end, EDGE_MARK_RADIUS, theme::GOAL);
            return;
        }
        let half = HALF_TILE * self.zoom * BEACON_RING;
        let diamond = vec![
            goal + Vec2::new(0.0, -half),
            goal + Vec2::new(half, 0.0),
            goal + Vec2::new(0.0, half),
            goal + Vec2::new(-half, 0.0),
        ];
        painter.add(Shape::closed_line(diamond, stroke));
        painter.line_segment(
            [goal, goal - Vec2::new(0.0, BEACON_HEIGHT * self.zoom)],
            stroke,
        );
    }
}

/// The point where the line from `from` to `to` leaves `rect`.
fn clamp_to(rect: Rect, from: Pos2, to: Pos2) -> Pos2 {
    let delta = to - from;
    let reach = |delta: f32, low: f32, high: f32| {
        if delta > 0.0 {
            high / delta
        } else if delta < 0.0 {
            low / delta
        } else {
            f32::INFINITY
        }
    };
    let t = reach(delta.x, rect.left() - from.x, rect.right() - from.x)
        .min(reach(delta.y, rect.top() - from.y, rect.bottom() - from.y))
        .clamp(0.0, 1.0);
    from + delta * t
}

fn radar_symbol(frame: &WatchFrame, column: i32, row: i32) -> Option<char> {
    let line = frame.radar.get(usize::try_from(row).ok()?)?;
    let symbol = line.chars().nth(usize::try_from(column).ok()?)?;
    Some(match symbol {
        SYM_BLOCK | SYM_DOOR | SYM_WATER => symbol,
        _ => SYM_WALK,
    })
}

/// Draws the names from the top of the window down. A name that would lie on
/// one already drawn, or on the character, moves up until it is clear.
fn draw_plates(painter: &Painter, mut plates: Vec<Plate>, keep_clear: Rect, zoom: f32) {
    plates.sort_by(|a, b| a.top.y.total_cmp(&b.top.y));
    let font = theme::title_font(theme::SIZE_PLATE);
    let mut taken = vec![keep_clear];
    for plate in plates {
        let galley = painter.layout_no_wrap(plate.name.clone(), font.clone(), plate.color);
        let bar_height = if plate.hits_percent.is_some() {
            theme::PIP_HEIGHT + PLATE_GAP
        } else {
            0.0
        };
        let size = Vec2::new(
            galley.size().x.max(PLATE_BAR_WIDTH),
            galley.size().y + bar_height,
        ) + Vec2::splat(PLATE_PAD * 2.0);
        let mut area = Rect::from_center_size(
            plate.top - Vec2::new(0.0, size.y / 2.0 + PLATE_GAP * zoom),
            size,
        );
        while let Some(hit) = taken.iter().find(|r| r.intersects(area)) {
            area = area.translate(Vec2::new(0.0, hit.top() - area.bottom() - 1.0));
        }
        taken.push(area);
        painter.rect_filled(area, theme::BAR_RADIUS, theme::PLATE_BACK);
        let name_at = Pos2::new(area.center().x, area.top() + PLATE_PAD);
        let color = if plate.target {
            theme::ALARM
        } else {
            plate.color
        };
        theme::shadowed_text(
            painter,
            name_at,
            Align2::CENTER_TOP,
            &plate.name,
            font.clone(),
            color,
        );
        if let Some(percent) = plate.hits_percent {
            let track = Rect::from_min_size(
                Pos2::new(
                    area.center().x - PLATE_BAR_WIDTH / 2.0,
                    area.bottom() - PLATE_PAD - theme::PIP_HEIGHT,
                ),
                Vec2::new(PLATE_BAR_WIDTH, theme::PIP_HEIGHT),
            );
            painter.rect_filled(track.expand(1.0), theme::BAR_RADIUS, theme::TEXT_SHADOW);
            let mut fill = track;
            fill.set_width(PLATE_BAR_WIDTH * f32::from(percent) / PERCENT);
            painter.rect_filled(fill, theme::BAR_RADIUS, plate.color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(100.0, 100.0));
    const MIDDLE: Pos2 = Pos2::new(50.0, 50.0);

    #[test]
    fn a_goal_outside_the_window_is_marked_on_the_edge() {
        let far_right = Pos2::new(250.0, 50.0);
        assert_eq!(clamp_to(WINDOW, MIDDLE, far_right), Pos2::new(100.0, 50.0));
        let far_up_left = Pos2::new(-50.0, -150.0);
        assert_eq!(clamp_to(WINDOW, MIDDLE, far_up_left), Pos2::new(25.0, 0.0));
    }

    #[test]
    fn north_points_up_and_to_the_right_on_the_screen() {
        let (dx, dy) = facing_step("north").unwrap();
        assert!(dx - dy > 0.0 && dx + dy < 0.0);
        assert!(facing_step("up").is_none());
    }

    #[test]
    fn radar_cells_outside_the_radar_draw_nothing() {
        let frame = WatchFrame {
            radar: vec!["#.".into(), "m~".into()],
            ..WatchFrame::default()
        };
        assert_eq!(radar_symbol(&frame, 0, 0), Some(SYM_BLOCK));
        assert_eq!(radar_symbol(&frame, 0, 1), Some(SYM_WALK));
        assert_eq!(radar_symbol(&frame, 1, 1), Some(SYM_WATER));
        assert_eq!(radar_symbol(&frame, 2, 0), None);
        assert_eq!(radar_symbol(&frame, -1, 0), None);
    }

    #[test]
    fn the_tile_under_the_mouse_is_the_tile_that_is_drawn_there() {
        let mut scene = Scene::new(None);
        let frame = WatchFrame {
            x: 100,
            y: 100,
            z: 5,
            ..WatchFrame::default()
        };
        scene.follow(&frame, 0.0);
        assert_eq!(scene.tile_at(WINDOW, &frame, MIDDLE), (100, 100, 5));
        let one_east_one_north = MIDDLE + Vec2::new(HALF_TILE * 2.0, 0.0);
        assert_eq!(
            scene.tile_at(WINDOW, &frame, one_east_one_north),
            (101, 99, 5)
        );
        let drawn_at = scene.project(WINDOW, [97.0, 104.0, 5.0]);
        assert_eq!(scene.tile_at(WINDOW, &frame, drawn_at), (97, 104, 5));
    }

    #[test]
    fn real_files_pick_the_floor_of_a_building_and_not_the_land_under_it() {
        let Some(dir) = uoterm_nav::client_data_dir_from_env() else {
            return;
        };
        const BANK_FLOOR: i8 = 20;
        let mut scene = Scene::new(Some(&dir));
        let frame = WatchFrame {
            x: 3484,
            y: 2570,
            z: BANK_FLOOR,
            map: 1,
            ..WatchFrame::default()
        };
        scene.follow(&frame, 0.0);
        let drawn_at = scene.project(WINDOW, [3486.0, 2572.0, f32::from(BANK_FLOOR)]);
        assert_eq!(
            scene.tile_at(WINDOW, &frame, drawn_at),
            (3486, 2572, BANK_FLOOR)
        );
    }

    #[test]
    fn a_mobile_walks_runs_and_stands_by_how_fast_his_tiles_come() {
        const ON_FOOT: bool = false;
        const MOUNTED: bool = true;
        let step = |seconds: f64| Glide {
            from: [0.0; 3],
            to: [1.0, 0.0, 0.0],
            started: 0.0,
            seconds,
        };
        let walk = step(0.4);
        assert_eq!(walk.pose(0.1, ON_FOOT).action, Action::Walk);
        assert_eq!(step(0.2).pose(0.1, ON_FOOT).action, Action::Run);
        assert_eq!(step(0.2).pose(0.1, MOUNTED).action, Action::Walk);
        assert_eq!(step(0.1).pose(0.05, MOUNTED).action, Action::Run);
        let just_after = 0.4 + STEP_LINGER_SECONDS / 2.0;
        assert_eq!(walk.pose(just_after, ON_FOOT).action, Action::Walk);
        assert_eq!(walk.pose(2.0, ON_FOOT).action, Action::Stand);
        assert_eq!(
            Glide::resting([0.0; 3], 0.0).pose(0.0, ON_FOOT).action,
            Action::Stand
        );
        let later = walk.pose(0.1 + WALK_FRAME_SECONDS, ON_FOOT).tick;
        assert_eq!(later, walk.pose(0.1, ON_FOOT).tick + 1);
    }

    #[test]
    fn a_step_is_a_steady_move_and_a_teleport_is_not() {
        const STEP_SECONDS: f64 = 0.4;
        const CLOSE: f32 = 0.001;
        let mut scene = Scene::new(None);
        let mut frame = WatchFrame {
            x: 100,
            y: 100,
            ..WatchFrame::default()
        };
        scene.follow(&frame, 0.0);
        frame.x += 1;
        assert!(scene.follow(&frame, STEP_SECONDS));
        assert_eq!(scene.camera[0], 100.0);
        assert!(scene.follow(&frame, STEP_SECONDS * 1.5));
        assert!((scene.camera[0] - 100.5).abs() < CLOSE);
        assert!(!scene.follow(&frame, STEP_SECONDS * 2.5));
        assert_eq!(scene.camera[0], 101.0);
        frame.x = 900;
        assert!(!scene.follow(&frame, STEP_SECONDS * 3.0));
        assert_eq!(scene.camera[0], 900.0);
    }
}
