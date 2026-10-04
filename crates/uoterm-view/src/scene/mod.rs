//! The map behind the panels, as a list of things to paint: the land, the
//! things on it, the people, the names over them and the marks on the
//! ground. It uses the real pictures when the client files are there, and
//! flat colors from the radar when they are not. [`SceneState::build`]
//! makes the list each frame; a window paints it and holds no rule of the
//! map itself.

mod build;
mod canvas;
mod ceiling;
mod glide;
pub mod overlays;
mod pick;
pub mod plates;

pub use canvas::{Mesh, Vertex};
pub use overlays::Overlay;
pub use pick::Pick;
pub use plates::Plate;

use crate::art::{Pose, WorldArt};
use crate::audio::Step;
use crate::frame::WatchFrame;
use crate::geom::{Area, Point, Rgba, Vector};
use crate::lights::LightSource;
use crate::look::WorldLook;
use crate::model::house_design::{StoreyLook, STOREYS};
use crate::predict::WalkPrediction;
use crate::settings::Profile;
use canvas::Canvas;
use ceiling::{Ceiling, Circle, FadeKey};
use glide::{stride_lag, Glide};
use std::collections::{HashMap, HashSet};
use uoterm_nav::Action;

/// Half the side of a tile picture. One tile step moves this far on each
/// screen axis.
const HALF_TILE: f32 = 22.0;
/// One unit of height lifts a thing this many pixels.
const Z_PIXELS: f32 = 4.0;
/// The bits of a direction byte that name the way.
const DIRECTION_MASK: u8 = 0x07;
const CORPSE_GRAPHIC: u16 = 0x2006;
const PERCENT_MAX: u8 = 100;
/// The ring on the ground under a figure of the Modern style.
const PAWN_RING_RX: f32 = 15.0;
const PAWN_RING_RY: f32 = 7.5;
const PAWN_RING_WIDTH: f32 = 2.0;
/// The top of a figure with no picture, over his feet.
const PAWN_TOP_Y: f32 = -56.0;
/// The longest step of time one frame fades things by.
const MAX_FRAME_SECONDS: f64 = 0.25;
/// How long the death screen shows.
const DEATH_SCREEN_SECONDS: f64 = 1.5;
/// What the death screen says, in a UO font and hue.
pub const DEATH_WORDS: &str = "You are dead.";
pub const DEATH_FONT: u8 = 3;
pub const DEATH_HUE: u16 = 0;

const ZOOM_MIN: f32 = 0.5;
const ZOOM_MAX: f32 = 3.0;
const ZOOM_START: f32 = 1.0;
const ZOOM_PER_SCROLL_POINT: f32 = 0.0015;
/// How many points of [`SceneInput::scroll`] one notch of a wheel turns,
/// as egui counts a line of a wheel on a desktop.
pub const WHEEL_POINTS_PER_NOTCH: f32 = 40.0;
/// How much one point of the wheel with Ctrl held zooms, as egui counts
/// it: the zoom grows by `e` to the power of this times the points.
pub const WHEEL_ZOOM_PER_POINT: f32 = 1.0 / 200.0;

/// The pose of a mobile the window has not seen move.
pub const STANDING: Pose = Pose {
    action: Action::Stand,
    tick: 0,
};

/// A color faded to `alpha`, held between none and all of it.
fn faded(color: Rgba, alpha: f32) -> Rgba {
    color.with_alpha(alpha.clamp(0.0, 1.0))
}

/// What the window read from the mouse and the keys this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneInput {
    /// Where the mouse is, when it is over the window.
    pub mouse: Option<Point>,
    /// How far the wheel turned this frame, in points, as egui's
    /// `smooth_scroll_delta.y` gives it: positive when the wheel turns away
    /// from the player, which zooms the Modern map in. One notch of a wheel
    /// is [`WHEEL_POINTS_PER_NOTCH`]. A browser's `WheelEvent.deltaY` has
    /// the other sign and may count lines, so the web client turns it into
    /// notches first. The wheel with Ctrl held zooms by `zoom_delta` and
    /// scrolls nothing.
    pub scroll: f32,
    /// How much a pinch, or the wheel with Ctrl held, zooms this frame. One
    /// is no zoom.
    pub zoom_delta: f32,
    pub ctrl: bool,
    pub shift: bool,
    /// How many screen pixels one point of the window has.
    pub pixels_per_point: f32,
}

impl Default for SceneInput {
    fn default() -> Self {
        Self {
            mouse: None,
            scroll: 0.0,
            zoom_delta: 1.0,
            ctrl: false,
            shift: false,
            pixels_per_point: 1.0,
        }
    }
}

/// What one frame of the map paints, in this order: the mesh of the world
/// with the texture of the pictures, the overlays, then the plates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneDraw {
    /// The world, far rows first. Untextured shapes read the white point
    /// of the texture.
    pub mesh: Mesh,
    /// The names and hit points over the things that have them.
    pub plates: Vec<Plate>,
    /// The marks over the world: the way to the walk goal, and the ring of
    /// the character in the Modern style.
    pub overlays: Vec<Overlay>,
    /// The footsteps this frame, for the sound.
    pub steps: Vec<Step>,
    /// Something still moves, so the next frame must come at once.
    pub moving: bool,
    /// Some while the death screen shows: how long it has shown, in
    /// seconds. The world is not drawn then.
    pub death: Option<f64>,
}

/// Where the world lies on the screen this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    pub view: Area,
    /// Where the character is drawn now. The map is drawn round this point.
    pub camera: [f32; 3],
    pub zoom: f32,
    pub pixels_per_point: f32,
    /// How far the camera looks away from the character, in points.
    pub peek: Vector,
}

impl Projection {
    /// The point of the screen the camera looks at.
    pub fn center(&self) -> Point {
        self.view.center() - self.peek
    }

    /// Where a place of the world is in the window. The camera moves in
    /// whole screen pixels, and each place lands on a whole screen pixel.
    /// The art is drawn with sharp pixels, so a part of a pixel would make
    /// the pixels of the art crawl while the map scrolls.
    pub fn project(&self, at: [f32; 3]) -> Point {
        let on_plane = |place: [f32; 3]| {
            Vector::new(
                (place[0] - place[1]) * HALF_TILE,
                (place[0] + place[1]) * HALF_TILE - place[2] * Z_PIXELS,
            ) * self.zoom
        };
        let ppp = self.pixels_per_point;
        let snap = |v: Vector| Vector::new((v.x * ppp).round(), (v.y * ppp).round()) / ppp;
        let camera = snap(on_plane(self.camera));
        self.center() + snap(on_plane(at) - camera)
    }
}

/// The state the map keeps from one frame to the next: where each mobile
/// is drawn and how he moves, what fades, what was drawn where, and the
/// zoom.
pub struct SceneState {
    camera: [f32; 3],
    camera_glide: Option<Glide>,
    camera_map: Option<u8>,
    zoom: f32,
    peek: Vector,
    /// Where each mobile is drawn now, under its serial.
    actors: HashMap<u32, [f32; 3]>,
    glides: HashMap<u32, Glide>,
    /// The things drawn this frame, in paint order. The last one is on top.
    picks: Vec<Pick>,
    /// The time of the frame that is drawn now.
    now: f64,
    /// The footsteps of this frame.
    steps: Vec<Step>,
    /// When each walker last made a footstep sound.
    last_step: HashMap<u32, f64>,
    /// The action each mobile shows now, and when it began.
    shows: HashMap<u32, (Action, f64)>,
    /// The last cue that was taken. None until the first picture.
    last_cue: Option<u64>,
    /// The panels of the last frame. The wheel over one does not zoom.
    panels: Vec<Area>,
    pixels_per_point: f32,
    /// The season of the world. It swaps some of the art.
    season: u8,
    /// The choices of the Options screen, as this frame draws them.
    look: WorldLook,
    /// The lights this frame found.
    light_sources: Vec<LightSource>,
    /// How much of each fading thing shows, and the frame it last showed in.
    fades: HashMap<FadeKey, (f32, u64)>,
    frame_count: u64,
    /// The time from the last frame to this one.
    frame_seconds: f32,
    /// The thing under the mouse in the last frame.
    hovered: Option<u32>,
    ctrl_shift: bool,
    /// Ctrl was held in the last frame.
    ctrl_before: bool,
    /// The heights the world is cut at over the character.
    ceiling: Ceiling,
    circle: Option<Circle>,
    walk: WalkPrediction,
    /// How long the character waits after the slot of a step he was sent
    /// before he takes it.
    stride_lag: f64,
    /// The slot of the newest step whose move has started, in seconds on
    /// the clock of the window.
    stride_taken: Option<f64>,
    /// How each storey of the house being designed shows.
    storey_looks: [StoreyLook; STOREYS],
    /// When the shard last showed the death screen.
    died_at: Option<f64>,
    /// The mobiles and corpses in view. None until the first picture.
    in_view: Option<HashSet<u32>>,
    arrivals: Vec<u32>,
}

impl Default for SceneState {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneState {
    /// A map that has drawn nothing yet. Until the window tells how often
    /// it reads the session, a step is news with no poll gap.
    pub fn new() -> Self {
        Self {
            camera: [0.0; 3],
            camera_glide: None,
            camera_map: None,
            zoom: ZOOM_START,
            peek: Vector::ZERO,
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
            look: WorldLook::of(&Profile::default()),
            light_sources: Vec::new(),
            fades: HashMap::new(),
            frame_count: 0,
            frame_seconds: 0.0,
            hovered: None,
            ctrl_shift: false,
            ctrl_before: false,
            ceiling: Ceiling::open(true),
            circle: None,
            walk: WalkPrediction::default(),
            stride_lag: stride_lag(0.0),
            stride_taken: None,
            storey_looks: [StoreyLook::Normal; STOREYS],
            died_at: None,
            in_view: None,
            arrivals: Vec::new(),
        }
    }

    /// Builds one frame of the map in `view`, as the profile says. `time`
    /// is the clock of the window, in seconds.
    pub fn build(
        &mut self,
        art: &mut dyn WorldArt,
        view: Area,
        frame: &WatchFrame,
        time: f64,
        profile: &Profile,
        input: &SceneInput,
    ) -> SceneDraw {
        self.look = WorldLook::of(profile);
        self.read_input(view, input);
        self.pixels_per_point = input.pixels_per_point;
        self.season = frame.season;
        self.frame_seconds = (time - self.now).clamp(0.0, MAX_FRAME_SECONDS) as f32;
        self.now = time;
        self.frame_count += 1;
        self.take_cues(art, frame, time);
        art.take_live_map(&frame.live_map);
        self.note_arrivals(frame);
        let moving = self.follow(art, frame, time) || !self.shows.is_empty();
        let death = self.death_shown(frame, time);
        if death.is_some() {
            return SceneDraw {
                steps: std::mem::take(&mut self.steps),
                moving,
                death,
                ..SceneDraw::default()
            };
        }
        let mut canvas = Canvas::new(art.white_uv());
        let mut plates = Vec::new();
        self.build_world(art, view, frame, &mut canvas, &mut plates);
        let projection = self.projection(view);
        let mut overlays = overlays::walk_goal(&projection, frame);
        if !self.look.classic() {
            let character = projection.project(self.camera);
            overlays.push(overlays::focus_ring(character, self.zoom));
        }
        SceneDraw {
            mesh: canvas.mesh,
            plates,
            overlays,
            steps: std::mem::take(&mut self.steps),
            moving,
            death: None,
        }
    }

    /// How long the death screen has shown: for a moment after the shard
    /// says the character died, the world is black and says so.
    fn death_shown(&self, frame: &WatchFrame, time: f64) -> Option<f64> {
        let since = time - self.died_at?;
        (frame.dead && self.look.video.death_screen && since < DEATH_SCREEN_SECONDS)
            .then_some(since)
    }

    /// Reads the mouse and the keys for this frame: what the mouse is on,
    /// Ctrl and Shift, and the wheel. In the Classic style the wheel zooms
    /// while Ctrl is held when the Video page allows it, and the zoom may go
    /// back to its default when Ctrl is let go. The Modern style zooms with
    /// the wheel alone.
    fn read_input(&mut self, view: Area, input: &SceneInput) {
        let on_world = input.mouse.filter(|p| self.is_on_world(view, *p));
        self.hovered = on_world
            .and_then(|p| self.thing_at(p))
            .map(|pick| pick.serial);
        let ctrl = input.ctrl;
        self.ctrl_shift = ctrl && input.shift;
        let video = &self.look.video;
        let zoom = if !self.look.classic() {
            let scroll = if on_world.is_some() {
                input.scroll
            } else {
                0.0
            };
            self.zoom * (1.0 + scroll * ZOOM_PER_SCROLL_POINT)
        } else if video.wheel_zoom && ctrl && on_world.is_some() {
            self.zoom * input.zoom_delta
        } else if video.wheel_zoom && video.ctrl_release_restores_zoom && self.ctrl_before && !ctrl
        {
            video.default_zoom
        } else {
            self.zoom
        };
        self.ctrl_before = ctrl;
        self.zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
    }

    /// The choices of the Options screen the last frame drew with.
    pub fn look(&self) -> &WorldLook {
        &self.look
    }

    /// Where the world lies in `view` this frame.
    pub fn projection(&self, view: Area) -> Projection {
        Projection {
            view,
            camera: self.camera,
            zoom: self.zoom,
            pixels_per_point: self.pixels_per_point,
            peek: self.peek,
        }
    }

    /// Where a place of the world is on the screen.
    pub fn screen_of(&self, view: Area, place: [f32; 3]) -> Point {
        self.projection(view).project(place)
    }

    /// Where a mobile is drawn now. None for one that is not in view.
    pub fn place_of(&self, frame: &WatchFrame, serial: u32) -> Option<[f32; 3]> {
        if serial == frame.serial {
            return Some(self.camera);
        }
        self.actors.get(&serial).copied()
    }

    /// True when a point of the window shows the world and no panel.
    pub fn is_on_world(&self, view: Area, point: Point) -> bool {
        view.contains(point) && !self.panels.iter().any(|panel| panel.contains(point))
    }

    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Sets the zoom, inside the range the wheel allows.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
    }

    /// Moves the camera this far from the character, in points. Zero puts
    /// the character back in the middle.
    pub fn set_peek(&mut self, peek: Vector) {
        self.peek = peek;
    }

    /// The window tells where its panels are, for the next frame.
    pub fn set_panels(&mut self, panels: Vec<Area>) {
        self.panels = panels;
    }

    /// Tells how often, in seconds, the window reads the session. The news
    /// of a step comes that much later.
    pub fn set_poll_every(&mut self, seconds: f64) {
        self.stride_lag = stride_lag(seconds);
    }

    /// How each storey of the house being designed shows, as the designer
    /// sets it.
    pub fn set_storey_looks(&mut self, looks: [StoreyLook; STOREYS]) {
        self.storey_looks = looks;
    }
}

#[cfg(test)]
pub(crate) mod test_art {
    //! Art with no client files, so the map is flat colors from the radar.

    use crate::art::{Art, ArtRequest, Cell, Sprite, TextLook, WorldArt};
    use crate::frame::{WatchLiveMap, WatchLook};
    use crate::geom::Point;
    use uoterm_nav::{Action, AnimRules, ItemTile, LandTile, LightShape, MultiPiece};

    /// The point of the texture the flat shapes read.
    pub const WHITE_UV: Point = Point::new(0.5, 0.5);
    const TEXT_RGB: [u8; 3] = [200, 200, 200];

    #[derive(Default)]
    pub struct NoFiles {
        anim: AnimRules,
        files: bool,
        /// The answer to each picture asked for. None is missing.
        sprites: Option<Art<Sprite>>,
    }

    impl NoFiles {
        /// Art that says there are client files, which hold no animation
        /// files and none of the pictures asked for.
        pub fn with_files() -> Self {
            Self {
                files: true,
                ..Self::default()
            }
        }

        /// Client files that give `answer` to each picture asked for.
        pub fn answering(answer: Art<Sprite>) -> Self {
            Self {
                sprites: Some(answer),
                ..Self::with_files()
            }
        }
    }

    impl WorldArt for NoFiles {
        fn has_art(&self) -> bool {
            self.files
        }
        fn has_anim(&self) -> bool {
            false
        }
        fn sprite(&mut self, _: &ArtRequest) -> Art<Sprite> {
            self.sprites.unwrap_or(Art::Missing)
        }
        fn white_uv(&self) -> Point {
            WHITE_UV
        }
        fn cell(&mut self, _: u8, _: u16, _: u16) -> Art<&Cell> {
            Art::Missing
        }
        fn take_live_map(&mut self, _: &WatchLiveMap) {}
        fn item_tile(&self, _: u16) -> Option<&ItemTile> {
            None
        }
        fn land_tile(&self, _: u16) -> Option<&LandTile> {
            None
        }
        fn multi_pieces(&mut self, _: u16) -> Art<&[MultiPiece]> {
            Art::Missing
        }
        fn shown_graphic(&self, graphic: u16, _: u64) -> u16 {
            graphic
        }
        fn season_land(&self, _: u8, land_id: u16) -> u16 {
            land_id
        }
        fn season_item(&self, _: u8, graphic: u16) -> u16 {
            graphic
        }
        fn radar_rgb(&mut self, _: u8, _: u16, _: u16) -> Option<[u8; 3]> {
            None
        }
        fn land_z(&mut self, _: u8, _: u16, _: u16) -> Option<i8> {
            None
        }
        fn light_shape(&mut self, _: u8) -> Art<&LightShape> {
            Art::Missing
        }
        fn anim(&self) -> &AnimRules {
            &self.anim
        }
        fn frame_count(&mut self, _: &WatchLook, _: Action) -> Art<usize> {
            Art::Missing
        }
        fn line_height(&self, _: &TextLook) -> f32 {
            1.0
        }
        fn text_lines(&self, text: &str, _: &TextLook) -> Vec<String> {
            text.split(' ').map(str::to_string).collect()
        }
        fn text_rgb(&self, _: u16) -> [u8; 3] {
            TEXT_RGB
        }
        fn gump_drawn_at(&self, _: u16, _: usize, _: usize) -> bool {
            false
        }
        fn has_gump_art(&self) -> bool {
            false
        }
    }
}

#[cfg(test)]
mod draw_tests {
    use super::test_art::NoFiles;
    use super::*;

    const VIEW: Area = Area {
        min: Point { x: 0.0, y: 0.0 },
        max: Point { x: 640.0, y: 480.0 },
    };
    const POLL: f64 = 0.033;

    fn frame_at(x: u16, y: u16) -> WatchFrame {
        WatchFrame {
            serial: 1,
            name: "Mara".into(),
            x,
            y,
            radar: vec![
                ".....".into(),
                ".....".into(),
                "..@..".into(),
                ".....".into(),
                ".....".into(),
            ],
            ..WatchFrame::default()
        }
    }

    fn build(scene: &mut SceneState, frame: &WatchFrame, time: f64) -> SceneDraw {
        scene.build(
            &mut NoFiles::default(),
            VIEW,
            frame,
            time,
            &Profile::default(),
            &SceneInput::default(),
        )
    }

    #[test]
    fn the_same_frame_builds_the_same_draw() {
        let frame = frame_at(1424, 1693);
        let draw_a = build(&mut SceneState::new(), &frame, 1.0);
        let draw_b = build(&mut SceneState::new(), &frame, 1.0);
        assert_eq!(draw_a.mesh, draw_b.mesh);
        assert!(!draw_a.mesh.indices.is_empty());
    }

    #[test]
    fn the_far_row_is_painted_before_the_near_row() {
        let draw = build(&mut SceneState::new(), &frame_at(100, 100), 1.0);
        let first_y = draw.mesh.vertices.first().unwrap().pos[1];
        let last_y = draw.mesh.vertices.last().unwrap().pos[1];
        assert!(first_y < last_y);
    }

    #[test]
    fn a_walk_glides_between_tiles() {
        let mut scene = SceneState::new();
        scene.set_poll_every(POLL);
        build(&mut scene, &frame_at(100, 100), 0.0);
        let half = build(&mut scene, &frame_at(101, 100), 0.2);
        assert!(half.moving);
    }

    #[test]
    fn the_modern_style_rings_the_character_and_the_flat_map_is_white_lit() {
        let draw = build(&mut SceneState::new(), &frame_at(100, 100), 1.0);
        assert!(draw
            .overlays
            .iter()
            .any(|overlay| matches!(overlay, Overlay::ClosedLine { .. })));
        assert!(draw
            .mesh
            .vertices
            .iter()
            .all(|vertex| vertex.uv == [super::test_art::WHITE_UV.x, super::test_art::WHITE_UV.y]));
        assert!(draw.death.is_none());
    }

    #[test]
    fn a_thing_whose_picture_is_on_its_way_shows_when_it_comes() {
        const ITEM: u32 = 0x4000_0001;
        const BARREL: u16 = 0x0E77;
        const SIDE: f32 = 44.0;
        let mut frame = frame_at(100, 100);
        frame.items.push(crate::frame::WatchItem {
            serial: ITEM,
            name: "a barrel".into(),
            graphic: BARREL,
            x: 101,
            y: 100,
            ..crate::frame::WatchItem::default()
        });
        let profile = Profile::default();
        let input = SceneInput::default();
        let mut scene = SceneState::new();
        let named = |draw: &SceneDraw| draw.plates.iter().any(|plate| plate.name == "a barrel");
        let mut on_its_way = NoFiles::answering(crate::art::Art::Pending);
        let draw = scene.build(&mut on_its_way, VIEW, &frame, 0.0, &profile, &input);
        assert!(scene.picks().iter().all(|pick| pick.serial != ITEM));
        assert!(!named(&draw));
        let sprite = crate::art::Sprite::whole(SIDE, SIDE);
        let mut came = NoFiles::answering(crate::art::Art::Ready(sprite));
        let draw = scene.build(&mut came, VIEW, &frame, 0.1, &profile, &input);
        assert!(scene.picks().iter().any(|pick| pick.serial == ITEM));
        assert!(named(&draw));
    }

    #[test]
    fn the_wheel_zooms_the_modern_map_but_not_over_a_panel() {
        const SCROLL: f32 = 100.0;
        let mut scene = SceneState::new();
        let input = SceneInput {
            mouse: Some(VIEW.center()),
            scroll: SCROLL,
            ..SceneInput::default()
        };
        let frame = frame_at(100, 100);
        let profile = Profile::default();
        scene.build(&mut NoFiles::default(), VIEW, &frame, 0.0, &profile, &input);
        assert!(scene.zoom() > ZOOM_START);
        let zoomed = scene.zoom();
        scene.set_panels(vec![VIEW]);
        scene.build(&mut NoFiles::default(), VIEW, &frame, 0.1, &profile, &input);
        assert_eq!(scene.zoom(), zoomed);
    }
}
