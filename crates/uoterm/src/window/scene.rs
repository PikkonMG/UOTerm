//! The map behind the panels: the land, the things on it, the people, and
//! the way to the walk goal. It draws the real pictures when the client
//! files are there, and flat colors from the radar when they are not.

use super::atlas::{Atlas, Sprite};
use super::audio::Step;
use super::client_art::{is_drawn, ClientArt};
use super::figure::{is_mounted, Pose};
use super::theme;
use crate::view::{
    WatchCueKind, WatchFrame, WatchItem, WatchLook, WatchMobile, SYM_BLOCK, SYM_DOOR, SYM_WALK,
    SYM_WATER,
};
use eframe::egui::{
    self,
    epaint::{Mesh, Vertex},
    Align2, Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2,
};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use uoterm_nav::{Action, Deed};

/// Half the side of a tile picture. One tile step moves this far on each
/// screen axis.
const HALF_TILE: f32 = 22.0;
/// One unit of height lifts a thing this many pixels.
const Z_PIXELS: f32 = 4.0;
/// A thing this far above the feet of the character is over his head.
const OVERHEAD: i16 = 16;
const CORPSE_GRAPHIC: u16 = 0x2006;
/// The shard does not tell the window which way a corpse lies.
const CORPSE_FACING: u8 = 3;
/// A paperdoll faces the watcher.
const DOLL_FACING: u8 = 4;

const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 3.0;
const ZOOM_START: f32 = 1.0;
const ZOOM_PER_SCROLL_POINT: f32 = 0.0015;
/// The paces of the game: the time of one tile on foot and on a mount.
const STEP_SECONDS_FOOT_WALK: f64 = 0.4;
const STEP_SECONDS_FOOT_RUN: f64 = 0.2;
const STEP_SECONDS_MOUNT_WALK: f64 = 0.2;
const STEP_SECONDS_MOUNT_RUN: f64 = 0.1;
/// How many tiles wait while a move is under way.
const GLIDE_QUEUE_CAP: usize = 4;
/// A move with no tile queued takes this much longer than its pace.
const GLIDE_STRETCH_ALONE: f64 = 1.02;
/// A gap of this many paces, or more, is a rest between two walks.
const NEWS_REST_FACTOR: f64 = 2.0;
/// How much of each new gap goes into the learned rhythm.
const NEWS_LEARN_SHARE: f64 = 0.35;
/// Each queued tile makes the next move this much faster.
const GLIDE_CATCH_UP_PER_TILE: f64 = 0.08;
/// The next place comes a moment after a move ends. A mobile keeps his walk
/// for this long, so he does not stand still for one frame between two tiles.
const STEP_LINGER_SECONDS: f64 = 0.12;
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
/// One picture of a walk or a run shows for this long at the full pace of
/// the game. The legs go by the ground that was covered, not by the clock:
/// a slow mobile has slow legs, and a mobile that stops has still legs.
const STEP_FRAME_SECONDS: f64 = 0.08;
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
    /// The action each mobile shows now, and when it began.
    shows: HashMap<u32, (Action, f64)>,
    /// The last cue that was taken. None until the first picture.
    last_cue: Option<u64>,
    /// The panels of the last frame. The wheel over one does not zoom.
    panels: Vec<Rect>,
    /// How many screen pixels one point of the window has.
    pixels_per_point: f32,
    /// The season of the world. It swaps some of the art.
    season: u8,
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

/// How much of a building that waits for its place shows.
const PLACING_ALPHA: f32 = 0.55;
const MS_PER_SECOND: f64 = 1000.0;
/// A fire or a fountain shows its next picture this often, so the window
/// draws again at least this often.
const ART_CYCLE_SECONDS: f64 = 0.1;

/// How long a shown action plays, and how long each of its pictures stays.
const SHOW_SECONDS: f64 = 0.9;
const SHOW_FRAME_SECONDS: f64 = 0.1;

/// The pose of a mobile the window has not seen move.
const STANDING: Pose = Pose {
    action: Action::Stand,
    tick: 0,
};

/// A thing that stands on one tile and is drawn in height order.
enum Standing<'a> {
    Art(&'a WatchItem),
    /// One piece of a house or a boat.
    Piece {
        graphic: u16,
        z: f32,
    },
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
            Self::Piece { z, .. } => *z,
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
    /// The tiles that came while the move to `to` was under way.
    queue: [[f32; 3]; GLIDE_QUEUE_CAP],
    queued: usize,
    /// How long one tile takes at the pace of the mobile.
    tile_seconds: f64,
    /// How long one tile took of late, by when the news of each tile came.
    /// The news of a live shard comes at uneven times. A move that is a
    /// little faster than the news stops after each tile.
    news_seconds: f64,
    last_news: f64,
    /// The tiles of the moves that ended, for the pictures of the legs.
    tiles_done: f32,
    running: bool,
}

/// How long one tile takes. These are the paces of the game itself, so the
/// move on the screen does not depend on when the news of it came.
fn tile_seconds(mounted: bool, running: bool) -> f64 {
    match (mounted, running) {
        (false, false) => STEP_SECONDS_FOOT_WALK,
        (false, true) => STEP_SECONDS_FOOT_RUN,
        (true, false) => STEP_SECONDS_MOUNT_WALK,
        (true, true) => STEP_SECONDS_MOUNT_RUN,
    }
}

fn tiles_between(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
}

impl Glide {
    fn resting(at: [f32; 3], time: f64) -> Self {
        Self {
            from: at,
            to: at,
            started: time,
            seconds: 0.0,
            queue: [at; GLIDE_QUEUE_CAP],
            queued: 0,
            tile_seconds: STEP_SECONDS_FOOT_WALK,
            news_seconds: STEP_SECONDS_FOOT_WALK,
            last_news: time,
            tiles_done: 0.0,
            running: false,
        }
    }

    fn at(&self, time: f64) -> [f32; 3] {
        if self.seconds <= 0.0 {
            return self.to;
        }
        let share = ((time - self.started) / self.seconds).clamp(0.0, 1.0) as f32;
        [0, 1, 2].map(|i| self.from[i] + (self.to[i] - self.from[i]) * share)
    }

    fn moving(&self, time: f64) -> bool {
        self.queued > 0 || (self.from != self.to && time - self.started < self.seconds)
    }

    /// The way the mobile moves now, 0 for north and then clockwise to 7.
    /// None when he rests. The shard turns him when it takes his next step,
    /// but the window still draws the step before it. With the way of the
    /// shard he would slide sideways, as on ice.
    fn heading(&self, time: f64) -> Option<u8> {
        /// The ways, by the step south (north, none, south) and then by the
        /// step east (west, none, east). The middle is no move.
        const WAYS: [[u8; 3]; 3] = [[7, 0, 1], [6, 0, 2], [5, 4, 3]];
        let place = |step: f32| match step {
            step if step < 0.0 => 0,
            step if step > 0.0 => 2,
            _ => 1,
        };
        let east = place(self.to[0] - self.from[0]);
        let south = place(self.to[1] - self.from[1]);
        let on_the_move = time - self.started < self.seconds + STEP_LINGER_SECONDS;
        (on_the_move && (east, south) != (1, 1)).then(|| WAYS[south][east])
    }

    /// How many tiles the mobile covered since he last rested.
    fn tiles_covered(&self, time: f64) -> f32 {
        if self.seconds <= 0.0 {
            return self.tiles_done;
        }
        let share = ((time - self.started) / self.seconds).clamp(0.0, 1.0) as f32;
        self.tiles_done + tiles_between(self.from, self.to) * share
    }

    /// What the mobile does now, and which picture of it shows. The legs of
    /// a walk go by the ground he covered. For a short time after a move he
    /// keeps his last picture, so he does not stand up between two tiles.
    fn pose(&self, time: f64) -> Pose {
        let stepping = self.queued > 0
            || (self.from != self.to && time - self.started < self.seconds + STEP_LINGER_SECONDS);
        if !stepping {
            return Pose {
                action: Action::Stand,
                tick: (time / STAND_FRAME_SECONDS) as usize,
            };
        }
        let pictures_per_tile = self.tile_seconds / STEP_FRAME_SECONDS;
        Pose {
            action: if self.running {
                Action::Run
            } else {
                Action::Walk
            },
            tick: (f64::from(self.tiles_covered(time)) * pictures_per_tile) as usize,
        }
    }

    /// The last tile the mobile is on his way to.
    fn goal(&self) -> [f32; 3] {
        match self.queued {
            0 => self.to,
            queued => self.queue[queued - 1],
        }
    }

    /// How long the move to the next tile takes. With nothing queued it is
    /// a little slow, so the next tile comes before this one ends and the
    /// mobile does not stop between two tiles. With tiles queued it is fast,
    /// so the mobile catches up.
    fn leg_seconds(&self, from: [f32; 3], to: [f32; 3]) -> f64 {
        let tiles = f64::from(tiles_between(from, to).max(1.0));
        let pace = match self.queued {
            0 => GLIDE_STRETCH_ALONE,
            queued => 1.0 / (1.0 + GLIDE_CATCH_UP_PER_TILE * queued as f64),
        };
        self.tile_seconds.max(self.news_seconds) * tiles * pace
    }

    /// Takes the tile the mobile is on now. A new tile starts a move, or
    /// waits in the queue for the move under way to end. A jump is not a move.
    fn aim(&mut self, goal: [f32; 3], time: f64, mounted: bool, running: bool) {
        self.tile_seconds = tile_seconds(mounted, running);
        self.running = running;
        if goal != self.goal() {
            if far_apart(self.goal(), goal) {
                *self = Self::resting(goal, time);
                return;
            }
            self.note_news(goal, time);
            let at_rest = self.queued == 0 && time - self.started >= self.seconds;
            if at_rest {
                // A walk after a rest starts its legs from the first picture.
                let rested = time - self.started >= self.seconds + STEP_LINGER_SECONDS;
                self.tiles_done = if rested {
                    0.0
                } else {
                    self.tiles_done + tiles_between(self.from, self.to)
                };
                self.from = self.to;
                self.to = goal;
                self.started = time;
                self.seconds = self.leg_seconds(self.from, goal);
            } else {
                // A full queue loses its last tile. The move then cuts a corner.
                let place = self.queued.min(GLIDE_QUEUE_CAP - 1);
                self.queue[place] = goal;
                self.queued = place + 1;
            }
        }
        self.settle(time);
    }

    /// Learns the rhythm of the news. A gap that is much longer than the
    /// pace is a rest, not a rhythm.
    fn note_news(&mut self, goal: [f32; 3], time: f64) {
        let tiles = f64::from(tiles_between(self.goal(), goal).max(1.0));
        let took = (time - self.last_news) / tiles;
        self.last_news = time;
        let in_rhythm = took < self.tile_seconds * NEWS_REST_FACTOR;
        self.news_seconds = if in_rhythm {
            self.news_seconds + (took - self.news_seconds) * NEWS_LEARN_SHARE
        } else {
            self.tile_seconds
        };
    }

    /// Starts the move to the next queued tile at the moment the last move
    /// ended, so no time is lost between two tiles.
    fn settle(&mut self, time: f64) {
        while self.queued > 0 && time - self.started >= self.seconds {
            let next = self.queue[0];
            self.queue.copy_within(1.., 0);
            self.queued -= 1;
            self.started += self.seconds;
            self.tiles_done += tiles_between(self.from, self.to);
            self.from = self.to;
            self.to = next;
            self.seconds = self.leg_seconds(self.from, next);
        }
    }
}

/// The look of a mobile turned the way he moves. None when he faces that
/// way already, or when he rests.
fn turned_to(look: &WatchLook, heading: Option<u8>) -> Option<WatchLook> {
    let heading = heading.filter(|way| *way != look.direction)?;
    Some(WatchLook {
        direction: heading,
        ..look.clone()
    })
}

fn far_apart(a: [f32; 3], b: [f32; 3]) -> bool {
    tiles_between(a, b) > TELEPORT_TILES
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
            shows: HashMap::new(),
            last_cue: None,
            panels: Vec::new(),
            pixels_per_point: 1.0,
            season: 0,
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
        self.pixels_per_point = ui.ctx().pixels_per_point();
        self.season = frame.season;
        self.now = time;
        self.take_cues(frame, time);
        let moving = self.follow(frame, time) || !self.shows.is_empty();
        // A mobile that stands still moves a little, and a fire burns. Wake
        // for the next picture.
        if self.client.is_some() {
            ui.ctx()
                .request_repaint_after(Duration::from_secs_f64(ART_CYCLE_SECONDS));
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
            let over = i.pointer.hover_pos().is_some_and(|p| {
                rect.contains(p) && !self.panels.iter().any(|panel| panel.contains(p))
            });
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
                glide.aim(goal, time, is_mounted(&frame.look), frame.look.running);
                glide
            }
            None => Glide::resting(goal, time),
        };
        if self
            .camera_glide
            .is_some_and(|before| before.goal() != glide.goal())
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
                    glide.aim(goal, time, is_mounted(&mobile.look), mobile.look.running);
                    if glide.goal() != known.goal() {
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

    /// Takes the new animation cues. A cue from before the window opened
    /// does not play.
    fn take_cues(&mut self, frame: &WatchFrame, time: f64) {
        let newest = frame.cues.iter().map(|cue| cue.seq).max();
        if let Some(seen) = self.last_cue {
            for cue in frame.cues.iter().filter(|cue| cue.seq > seen) {
                let action = match cue.kind {
                    WatchCueKind::Animation(group) => u8::try_from(group).ok().map(Action::Shown),
                    WatchCueKind::Deed(kind, action) => {
                        self.deed_of(frame, cue.serial, kind, action)
                    }
                    _ => None,
                };
                if let Some(action) = action {
                    self.shows.insert(cue.serial, (action, time));
                }
            }
        }
        self.last_cue = newest.or(self.last_cue).or(Some(0));
        self.shows
            .retain(|_, (_, began)| time - *began < SHOW_SECONDS);
    }

    /// The pictures for a deed of the newer animation packet. They depend
    /// on the body of the mobile.
    fn deed_of(&self, frame: &WatchFrame, serial: u32, kind: u16, action: u16) -> Option<Action> {
        let look = if serial == frame.serial {
            &frame.look
        } else {
            &frame.mobiles.iter().find(|m| m.serial == serial)?.look
        };
        self.client
            .as_ref()?
            .deed_action(look, Deed::from_packet(kind, action)?)
    }

    /// The pose of a mobile that shows an action now.
    fn shown_pose(&self, serial: u32) -> Option<Pose> {
        let (action, began) = self.shows.get(&serial)?;
        Some(Pose {
            action: *action,
            tick: ((self.now - began) / SHOW_FRAME_SECONDS) as usize,
        })
    }

    /// Draws the outline of a building where the mouse points, while the
    /// shard waits for its place. Its pieces show as a pale shape, so the
    /// human sees what the building covers before he puts it there.
    pub fn draw_placing(
        &mut self,
        painter: &Painter,
        rect: Rect,
        frame: &WatchFrame,
        mouse: Pos2,
    ) -> bool {
        let Some(placing) = frame.placing else {
            return false;
        };
        let (x, y, z) = self.tile_at(rect, frame, mouse);
        let pieces: Vec<(i16, i16, i16, u16)> = self
            .client
            .as_ref()
            .map(|client| {
                client
                    .multi_pieces(placing.multi_id)
                    .iter()
                    .map(|piece| (piece.dx, piece.dy, piece.dz, piece.graphic))
                    .collect()
            })
            .unwrap_or_default();
        let mut mesh = Mesh::with_texture(match self.atlas.as_ref() {
            Some(atlas) => atlas.texture_id(),
            None => return false,
        });
        let white = self.atlas.as_ref().map(Atlas::white_uv).unwrap_or_default();
        let mut canvas = Canvas { mesh, white };
        for (dx, dy, dz, graphic) in pieces {
            if !is_drawn(graphic) {
                continue;
            }
            let at = [
                f32::from(x) + f32::from(dx),
                f32::from(y) + f32::from(dy),
                f32::from(z) + f32::from(dz),
            ];
            let Some(sprite) = self.item_sprite(frame.map, graphic, placing.hue) else {
                continue;
            };
            let foot = self.project(rect, at);
            let area = Rect::from_min_size(
                foot - sprite.anchor * self.zoom,
                Vec2::new(sprite.width, sprite.height) * self.zoom,
            );
            canvas.sprite(
                sprite,
                area,
                theme::with_alpha(Color32::WHITE, PLACING_ALPHA),
            );
        }
        mesh = canvas.mesh;
        if !mesh.is_empty() {
            painter.add(Shape::mesh(mesh));
        }
        // The tile the building goes on, so the human sees the exact spot.
        let center = self.project(rect, [f32::from(x), f32::from(y), f32::from(z)]);
        painter.circle_stroke(
            center,
            PAWN_RING_RX * self.zoom,
            Stroke::new(PAWN_RING_WIDTH * self.zoom, theme::GOAL),
        );
        true
    }

    /// The picture of a mobile as he stands and faces the watcher, for a
    /// paperdoll. It carries what he wears.
    pub fn doll_picture(
        &mut self,
        map: u8,
        look: &crate::view::WatchLook,
    ) -> Option<(egui::TextureId, Sprite)> {
        let facing = crate::view::WatchLook {
            direction: DOLL_FACING,
            ..look.clone()
        };
        let pose = Pose {
            action: Action::Stand,
            tick: 0,
        };
        let atlas = self.atlas.as_mut()?;
        let sprite =
            self.client
                .as_ref()?
                .figure_sprite(atlas, map, &facing, pose, theme::SELF_FIGURE)?;
        Some((atlas.texture_id(), sprite))
    }

    /// True when gumps can show in their own pictures.
    pub fn has_gump_art(&self) -> bool {
        self.client.as_ref().is_some_and(ClientArt::has_gump_art)
    }

    /// A picture of a gump, for a window that is not the map.
    pub fn gump_picture(&mut self, gump: u16, hue: u16) -> Option<(egui::TextureId, Sprite)> {
        let atlas = self.atlas.as_mut()?;
        let sprite = self.client.as_ref()?.gump_sprite(atlas, gump, hue)?;
        Some((atlas.texture_id(), sprite))
    }

    /// The color of one tile on a map of the world.
    pub fn radar_rgb(&mut self, map: u8, x: u16, y: u16) -> Option<[u8; 3]> {
        self.client.as_mut()?.radar_rgb(map, x, y)
    }

    /// Where a place of the world is on the screen.
    pub fn screen_of(&self, rect: Rect, place: [f32; 3]) -> Pos2 {
        self.project(rect, place)
    }

    /// Where a mobile is drawn now. None for one that is not in view.
    pub fn place_of(&self, frame: &WatchFrame, serial: u32) -> Option<[f32; 3]> {
        if serial == frame.serial {
            return Some(self.camera);
        }
        self.actors.get(&serial).copied()
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// The window tells where its panels are, for the next frame.
    pub fn set_panels(&mut self, panels: Vec<Rect>) {
        self.panels = panels;
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
        let running = glide.running;
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
        let client = self.client.as_ref()?;
        let land_id = client.season_land(self.season, land_id);
        client.land_sprite(self.atlas.as_mut()?, land_id)
    }

    fn item_sprite(&mut self, map: u8, graphic: u16, hue: u16) -> Option<Sprite> {
        let client = self.client.as_ref()?;
        let now_ms = (self.now * MS_PER_SECOND) as u64;
        let graphic = client.season_item(self.season, graphic);
        let shown = client.shown_graphic(map, graphic, now_ms);
        client.item_sprite(self.atlas.as_mut()?, map, shown, hue)
    }

    /// Where a place of the world is in the window. The camera moves in
    /// whole screen pixels, and each place lands on a whole screen pixel.
    /// The art is drawn with sharp pixels, so a part of a pixel would make
    /// the pixels of the art crawl while the map scrolls.
    fn project(&self, rect: Rect, at: [f32; 3]) -> Pos2 {
        let on_plane = |place: [f32; 3]| {
            Vec2::new(
                (place[0] - place[1]) * HALF_TILE,
                (place[0] + place[1]) * HALF_TILE - place[2] * Z_PIXELS,
            ) * self.zoom
        };
        let snap = |v: Vec2| (v * self.pixels_per_point).round() / self.pixels_per_point;
        let camera = snap(on_plane(self.camera));
        rect.center() + snap(on_plane(at) - camera)
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
        // A house a player designed is drawn from its own tiles, not from
        // the plain multi its foundation names.
        let designed: HashMap<u32, &uoterm_world::DesignedHouse> = frame
            .designed_houses
            .iter()
            .map(|house| (house.serial.0, house))
            .collect();
        for (multi, house) in frame
            .multis
            .iter()
            .filter_map(|multi| Some((multi, *designed.get(&multi.serial)?)))
        {
            for tile in house.tiles.iter().filter(|t| is_drawn(t.graphic)) {
                let at = (i32::from(multi.x) + tile.dx, i32::from(multi.y) + tile.dy);
                out.entry(at).or_default().push(Standing::Piece {
                    graphic: tile.graphic,
                    z: f32::from(multi.z) + tile.dz as f32,
                });
            }
        }
        for (multi, piece) in frame.multis.iter().flat_map(|multi| {
            let designed = designed.contains_key(&multi.serial);
            let pieces = self
                .client
                .as_ref()
                .filter(|_| !designed)
                .map_or(&[][..], |client| client.multi_pieces(multi.multi_id));
            pieces.iter().map(move |piece| (multi, piece))
        }) {
            if !is_drawn(piece.graphic) {
                continue;
            }
            let tile = (
                i32::from(multi.x) + i32::from(piece.dx),
                i32::from(multi.y) + i32::from(piece.dy),
            );
            out.entry(tile).or_default().push(Standing::Piece {
                graphic: piece.graphic,
                z: f32::from(multi.z) + f32::from(piece.dz),
            });
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
                Standing::Piece { graphic, z } => {
                    if ceiling.is_some_and(|c| z >= f32::from(c)) {
                        continue;
                    }
                    self.item_art(rect, frame.map, [x, y, z], graphic, 0, canvas);
                }
                Standing::Corpse(item) => {
                    let center = self.project(rect, [x, y, f32::from(item.z)]);
                    let area = self.corpse(canvas, frame.map, item, center);
                    self.picks.push(Pick {
                        area,
                        serial: item.serial,
                        name: item.name.clone(),
                        kind: PickKind::Corpse,
                    });
                }
                Standing::Mobile { mobile, at } => {
                    let target = !frame.combatant.is_empty() && mobile.name == frame.combatant;
                    let color = theme::notoriety_color(mobile.notoriety);
                    let foot = self.project(rect, at);
                    let pose = self.shown_pose(mobile.serial).unwrap_or_else(|| {
                        self.glides
                            .get(&mobile.serial)
                            .map_or(STANDING, |glide| glide.pose(self.now))
                    });
                    let heading = self
                        .glides
                        .get(&mobile.serial)
                        .and_then(|glide| glide.heading(self.now));
                    let moving_look = turned_to(&mobile.look, heading);
                    let look = Looks {
                        look: moving_look.as_ref().unwrap_or(&mobile.look),
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
                    let pose = self.shown_pose(frame.serial).unwrap_or_else(|| {
                        self.camera_glide
                            .map_or(STANDING, |glide| glide.pose(self.now))
                    });
                    let heading = self.camera_glide.and_then(|glide| glide.heading(self.now));
                    let moving_look = turned_to(&frame.look, heading);
                    let look = Looks {
                        look: moving_look.as_ref().unwrap_or(&frame.look),
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
            .and_then(|(client, atlas)| {
                let pose = Pose {
                    action: client.stance_action(look, pose.action),
                    ..pose
                };
                client.figure_sprite(atlas, map, look, pose, color)
            });
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

    /// A corpse is the fallen body. The amount of a corpse item is the body
    /// it was. With no picture of the death, a ring marks the place.
    fn corpse(&mut self, canvas: &mut Canvas, map: u8, item: &WatchItem, center: Pos2) -> Rect {
        let look = WatchLook {
            body: item.amount,
            hue: item.hue,
            direction: CORPSE_FACING,
            ..WatchLook::default()
        };
        let sprite = self
            .client
            .as_ref()
            .zip(self.atlas.as_mut())
            .and_then(|(client, atlas)| client.corpse_sprite(atlas, map, &look, theme::CORPSE));
        let Some(sprite) = sprite else {
            let radius = Vec2::new(PAWN_RING_RX, PAWN_RING_RY) * self.zoom;
            canvas.ring(center, radius, PAWN_RING_WIDTH * self.zoom, theme::CORPSE);
            return Rect::from_center_size(center, radius * 2.0);
        };
        let area = Rect::from_min_size(
            center - sprite.anchor * self.zoom,
            Vec2::new(sprite.width, sprite.height) * self.zoom,
        );
        canvas.sprite(sprite, area, Color32::WHITE);
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

    /// The point over the head of a mobile that is drawn now.
    pub fn head_of(&self, rect: Rect, frame: &WatchFrame, serial: u32) -> Option<Pos2> {
        if serial == frame.serial {
            let foot = self.project(rect, self.camera);
            return Some(self.figure_rect(foot).center_top());
        }
        self.picks
            .iter()
            .find(|pick| pick.serial == serial && pick.kind == PickKind::Mobile)
            .map(|pick| pick.area.center_top())
    }

    /// The color of words in a hue. Without client files, the plain color.
    pub fn words_color(&self, hue: u16) -> Color32 {
        self.client
            .as_ref()
            .and_then(|client| client.text_rgb(hue))
            .map_or(theme::TEXT, |[r, g, b]| Color32::from_rgb(r, g, b))
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
    fn a_piece_of_a_house_is_drawn_in_height_order_with_the_rest() {
        let floor = Standing::Piece {
            graphic: 0x0500,
            z: 7.0,
        };
        let character = Standing::Character {
            at: [0.0, 0.0, 12.0],
        };
        assert!(floor.z() < character.z());
        let without_files = Scene::new(None);
        let frame = WatchFrame {
            multis: vec![crate::view::WatchMulti {
                serial: 50,
                multi_id: 100,
                x: 10,
                y: 10,
                z: 0,
            }],
            ..WatchFrame::default()
        };
        let standing = without_files.standing(&frame);
        assert_eq!(standing.len(), 1, "only the character stands");
    }

    #[test]
    fn a_new_animation_cue_plays_once_and_then_ends() {
        const ORC: u32 = 9;
        const SWING: u16 = 9;
        let mut scene = Scene::new(None);
        let mut frame = WatchFrame::default();
        scene.take_cues(&frame, 0.0);
        frame.cues.push(crate::view::WatchCue {
            seq: 1,
            serial: ORC,
            kind: WatchCueKind::Animation(SWING),
        });
        scene.take_cues(&frame, 1.0);
        scene.now = 1.25;
        let pose = scene.shown_pose(ORC).unwrap();
        assert_eq!((pose.action, pose.tick), (Action::Shown(9), 2));
        scene.take_cues(&frame, 1.0 + SHOW_SECONDS * 2.0);
        assert!(scene.shown_pose(ORC).is_none());
    }

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

    const ON_FOOT: bool = false;
    const WALKS: bool = false;
    const RUNS: bool = true;
    const CLOSE: f32 = 0.001;

    fn east(tiles: f32) -> [f32; 3] {
        [tiles, 0.0, 0.0]
    }

    #[test]
    fn a_mobile_walks_or_runs_by_what_the_shard_says_and_then_stands() {
        let mut walker = Glide::resting(east(0.0), 0.0);
        assert_eq!(walker.pose(0.0).action, Action::Stand);
        walker.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        assert_eq!(walker.pose(0.1).action, Action::Walk);
        // Five pictures for one tile on foot, spread over the move.
        let end_of_move = walker.seconds;
        assert_eq!(walker.pose(end_of_move / 2.0).tick, 2);
        assert_eq!(walker.pose(end_of_move).tick, 5);
        let end = walker.seconds;
        assert_eq!(
            walker.pose(end + STEP_LINGER_SECONDS / 2.0).action,
            Action::Walk
        );
        assert_eq!(walker.pose(end + 2.0).action, Action::Stand);
        let mut runner = Glide::resting(east(0.0), 0.0);
        runner.aim(east(1.0), 0.0, ON_FOOT, RUNS);
        assert_eq!(runner.pose(0.05).action, Action::Run);
        assert!(runner.seconds < walker.seconds);
    }

    #[test]
    fn tiles_that_come_early_wait_and_the_move_does_not_stop_between_them() {
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        let first_leg = glide.seconds;
        assert!((first_leg - STEP_SECONDS_FOOT_WALK * GLIDE_STRETCH_ALONE).abs() < 1e-9);
        // The news of the next tile comes at an uneven time. It waits.
        glide.aim(east(2.0), 0.33, ON_FOOT, WALKS);
        assert_eq!(glide.queued, 1);
        assert!(glide.at(0.33)[0] < 1.0);
        // The next move starts at the moment the first one ended.
        glide.aim(east(2.0), first_leg + 0.05, ON_FOOT, WALKS);
        assert_eq!((glide.queued, glide.started), (0, first_leg));
        let speed = 1.0 / glide.seconds as f32;
        assert!((glide.at(first_leg + 0.05)[0] - (1.0 + 0.05 * speed)).abs() < CLOSE);
        assert!(glide.moving(first_leg + 0.05));
    }

    #[test]
    fn news_that_comes_late_each_time_makes_the_move_slower_so_it_does_not_stop() {
        const LATE: f64 = STEP_SECONDS_FOOT_WALK * 1.2;
        let mut glide = Glide::resting(east(0.0), 0.0);
        for tile in 1..=8 {
            glide.aim(east(tile as f32), LATE * f64::from(tile), ON_FOOT, WALKS);
        }
        assert!(glide.news_seconds > STEP_SECONDS_FOOT_WALK * 1.15);
        assert!(glide.seconds >= LATE, "the move lasts until the next news");
        // A rest does not count as a slow rhythm.
        glide.aim(east(9.0), 100.0, ON_FOOT, WALKS);
        assert_eq!(glide.news_seconds, STEP_SECONDS_FOOT_WALK);
    }

    #[test]
    fn uneven_news_gives_a_move_that_never_stops_between_tiles() {
        const FRAME: f64 = 1.0 / 60.0;
        // The session spaces its steps like a person: some early, some late.
        let gaps = [0.46, 0.35, 0.44, 0.37, 0.45, 0.34, 0.46, 0.40, 0.43, 0.36];
        let mut glide = Glide::resting(east(0.0), 0.0);
        let (mut news_at, mut tile, mut gap) = (0.0, 0.0, 0);
        let (mut time, mut before, mut stalls) = (0.0, 0.0, 0);
        while tile < 40.0 {
            if time >= news_at {
                tile += 1.0;
                news_at += gaps[gap % gaps.len()];
                gap += 1;
            }
            glide.aim(east(tile), time, ON_FOOT, WALKS);
            let at = glide.at(time)[0];
            // The first tiles teach the rhythm.
            if tile > 6.0 && at <= before {
                stalls += 1;
            }
            before = at;
            time += FRAME;
        }
        assert_eq!(stalls, 0);
        assert!(tile - before < 2.0, "the move stays near the news");
    }

    #[test]
    fn the_legs_go_by_the_ground_and_stop_when_the_mobile_stops() {
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, RUNS);
        let end = glide.seconds;
        let at_end = glide.pose(end).tick;
        // After the move the legs keep their picture, then he stands.
        assert_eq!(glide.pose(end + STEP_LINGER_SECONDS / 2.0).tick, at_end);
        assert_eq!(glide.pose(end + 1.0).action, Action::Stand);
        // A slow move has the same pictures for each tile as a fast one.
        let mut slow = Glide::resting(east(0.0), 0.0);
        slow.aim(east(1.0), 0.0, ON_FOOT, RUNS);
        slow.seconds *= 3.0;
        assert_eq!(slow.pose(slow.seconds).tick, at_end);
        // The next tile carries the legs on from where they were.
        glide.aim(east(2.0), end, ON_FOOT, RUNS);
        assert!(glide.pose(end + glide.seconds).tick > at_end);
    }

    #[test]
    fn a_mobile_faces_the_way_he_moves_and_not_the_way_of_his_next_step() {
        const NORTH: u8 = 0;
        const EAST: u8 = 2;
        const SOUTH_WEST: u8 = 5;
        let mut glide = Glide::resting(east(0.0), 0.0);
        assert_eq!(glide.heading(0.0), None);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        assert_eq!(glide.heading(0.1), Some(EAST));
        // The shard turned him north for his next step. He still goes east.
        let shard_look = WatchLook {
            direction: NORTH,
            ..WatchLook::default()
        };
        let drawn = turned_to(&shard_look, glide.heading(0.1)).unwrap();
        assert_eq!(drawn.direction, EAST);
        assert!(turned_to(&shard_look, Some(NORTH)).is_none());
        assert_eq!(glide.heading(glide.seconds + 1.0), None);
        let mut back = Glide::resting([5.0, 5.0, 0.0], 0.0);
        back.aim([4.0, 6.0, 0.0], 0.0, ON_FOOT, WALKS);
        assert_eq!(back.heading(0.1), Some(SOUTH_WEST));
    }

    #[test]
    fn a_mobile_that_is_behind_catches_up() {
        let mut glide = Glide::resting(east(0.0), 0.0);
        glide.aim(east(1.0), 0.0, ON_FOOT, WALKS);
        let alone = glide.seconds;
        for tile in 2..=8 {
            glide.aim(east(tile as f32), 0.01, ON_FOOT, WALKS);
        }
        assert_eq!(glide.queued, GLIDE_QUEUE_CAP);
        assert_eq!(
            glide.goal(),
            east(8.0),
            "a full queue keeps the newest tile"
        );
        glide.aim(east(8.0), alone, ON_FOOT, WALKS);
        assert!(glide.seconds < STEP_SECONDS_FOOT_WALK);
    }

    #[test]
    fn a_step_is_a_steady_move_and_a_teleport_is_not() {
        let leg = STEP_SECONDS_FOOT_WALK * GLIDE_STRETCH_ALONE;
        let mut scene = Scene::new(None);
        let mut frame = WatchFrame {
            x: 100,
            y: 100,
            ..WatchFrame::default()
        };
        scene.follow(&frame, 0.0);
        frame.x += 1;
        assert!(scene.follow(&frame, 1.0));
        assert_eq!(scene.camera[0], 100.0);
        assert!(scene.follow(&frame, 1.0 + leg / 2.0));
        assert!((scene.camera[0] - 100.5).abs() < CLOSE);
        assert!(!scene.follow(&frame, 1.0 + leg * 2.0));
        assert_eq!(scene.camera[0], 101.0);
        frame.x = 900;
        assert!(!scene.follow(&frame, 3.0));
        assert_eq!(scene.camera[0], 900.0);
    }
}
