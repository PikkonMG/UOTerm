//! The play window of the browser, as WebAssembly, and the screens before
//! it: the rules of the login screens (`login`) and the making of a new
//! character (`creation`). The page gives the view
//! each picture of the session, its input and the art it fetched; each
//! frame the view runs the same rules as the Modern style of the Rust
//! window (`uoterm_view`) and gives back what to draw, the calls to make on
//! the live link, and the data of the panels. No rule lives here: this
//! crate only joins the shared rules to the types of the page.
//!
//! Every method has a native form for the tests; the `#[wasm_bindgen]`
//! methods only turn JavaScript values into Rust ones and back.

mod buffers;
mod creation;
mod frame_flow;
mod input;
mod login;
mod out;
mod panels;
mod synth;
mod web_art;

pub use buffers::{DrawBuffers, PlacedWords, Shapes};
pub use creation::{CreationScreen, CreationView};
pub use input::{Click, FrameInput, InputEvent, Inputs};
pub use out::{Hand, OutCall, JEV_ORDER};
pub use panels::{
    ChatData, FrameAction, FrameData, Framed, HotbarAction, HotbarData, HotbarSlot, PanelData,
    Place, PANEL_HOTBAR, PANEL_QUESTION,
};
pub use synth::{render, render_midi};
pub use web_art::{Post, Upload, Wanted, WebArt, BLOCK_KEEP_FRAMES, MEASURES_KEPT};

use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;
use uoterm_view::actions::controls::Controls;
use uoterm_view::actions::PointerClick;
use uoterm_view::art::{ArtRequest, WorldArt};
use uoterm_view::atlas::{ATLAS_SIDE, WHITE_SIDE};
use uoterm_view::clicks::ChatMode;
use uoterm_view::floats::Floats;
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point, Vector};
use uoterm_view::guard::KeptGrabBags;
use uoterm_view::guard::GRAB_BAGS_FILE;
use uoterm_view::keys::chat::ChatLine;
use uoterm_view::model::game_view::ShardReports;
use uoterm_view::pad::PadState;
use uoterm_view::scene::{SceneState, WHEEL_POINTS_PER_NOTCH};
use uoterm_view::settings::{Profile, UiStyle};
use uoterm_view::sky::Sky;
use uoterm_view::steer::Steer;
use uoterm_view::tips::Tips;
use uoterm_view::ui::deck::{KeptHotbars, HOTBAR_FILE};
use uoterm_view::video::frame_interval;
use wasm_bindgen::prelude::*;

/// The kept files of the config folder the view reads, under this path.
pub(crate) const KEPT_PREFIX: &str = "/v1/kept/";
const MS_PER_SECOND: f64 = 1000.0;

/// The profile as it is kept: with the UI style the Rust window keeps in
/// it. The browser shows the Modern style only, so the view holds its
/// profile in that style.
pub(crate) fn kept(profile: &Profile, kept_style: UiStyle) -> Cow<'_, Profile> {
    if profile.interface.ui_style == kept_style {
        Cow::Borrowed(profile)
    } else {
        let mut kept = profile.clone();
        kept.interface.ui_style = kept_style;
        Cow::Owned(kept)
    }
}

/// A value for the page, as plain JavaScript values: objects, arrays,
/// numbers, and null for a None (never undefined, never a Map). Every
/// value the view gives the page goes through here.
pub(crate) fn to_js<T: Serialize>(value: &T) -> JsValue {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .unwrap_or(JsValue::NULL)
}

/// The side of the texture the page keeps the pictures in, in pixels: the
/// places of `DrawBuffers.uploads()` and the u, v of the meshes count in it.
#[wasm_bindgen(js_name = atlasSide)]
pub fn atlas_side() -> usize {
    ATLAS_SIDE
}

/// The side of the white square the page lays in the top left corner of
/// the texture, in pixels.
#[wasm_bindgen(js_name = whiteSide)]
pub fn white_side() -> usize {
    WHITE_SIDE
}

/// The points of the wheel one notch turns, as egui counts a line of a
/// wheel: the page turns a browser's pixels and pages into notches by it.
#[wasm_bindgen(js_name = wheelPointsPerNotch)]
pub fn wheel_points_per_notch() -> f32 {
    WHEEL_POINTS_PER_NOTCH
}

/// How many times its size a small picture of a panel grows at most to
/// fit its cell, as the Rust window fits one.
#[wasm_bindgen(js_name = artMostScale)]
pub fn art_most_scale() -> f32 {
    uoterm_view::ui::theme::ART_MAX_SCALE
}

/// How far, in points, a button that went down moves before it drags and
/// makes no click, as egui counts it: a drag of the page starts there.
#[wasm_bindgen(js_name = clickDistance)]
pub fn click_distance() -> f32 {
    input::CLICK_DISTANCE
}

/// Measures words in the font the page draws the name plates in.
type Measure = Box<dyn Fn(&str) -> Vector>;

/// The play window of the browser.
#[wasm_bindgen]
pub struct WebView {
    /// The profile, in the Modern style.
    profile: Profile,
    /// The UI style the profile keeps for the Rust window.
    kept_style: UiStyle,
    /// None until the session sends the first picture.
    frame: Option<WatchFrame>,
    art: WebArt,
    scene: SceneState,
    controls: Controls,
    hand: Hand,
    inputs: Inputs,
    steer: Steer,
    pad: PadState,
    chat: ChatLine,
    chat_mode: ChatMode,
    sky: Sky,
    floats: Floats,
    tips: Tips,
    reports: ShardReports,
    hotbars: KeptHotbars,
    /// The Modern panels the view keeps open and how each stands.
    panels: panels::PanelState,
    /// The view of the last tick, in points.
    view: Area,
    /// The kept files to read, each one time.
    kept_wanted: Vec<String>,
    /// Where the panels of the page lie: the world takes no clicks there.
    covered: Vec<Area>,
    pixels_per_point: f32,
    measure: Option<Measure>,
    /// The clock of the last tick.
    last_tick: Option<f64>,
    /// The tooltip of the thing under the mouse on the map.
    tooltip: Option<TooltipData>,
}

/// The pointer clicks a controller or a key asked for, as the input of the
/// next frame: at the pointer, or nowhere when the pointer is not over the
/// page.
fn clicks_of(asked: Vec<PointerClick>, at: Option<Point>, input: &mut FrameInput) {
    let Some(at) = at else {
        return;
    };
    for click in asked {
        match click {
            PointerClick::Left => input.clicks.push(Click {
                at,
                double: false,
                mods: Default::default(),
            }),
            PointerClick::Double => {
                for double in [false, true] {
                    input.clicks.push(Click {
                        at,
                        double,
                        mods: Default::default(),
                    });
                }
            }
            PointerClick::Right => input.secondary_pressed = true,
        }
    }
}

impl WebView {
    /// A view with the profile in `profile_json`. A profile that does not
    /// read is the default one.
    pub fn with_profile(profile_json: &str) -> Self {
        let profile: Profile = serde_json::from_str(profile_json).unwrap_or_default();
        let mut view = Self {
            profile: Profile::default(),
            kept_style: UiStyle::Modern,
            frame: None,
            art: WebArt::default(),
            scene: SceneState::new(),
            controls: Controls::default(),
            hand: Hand::default(),
            inputs: Inputs::default(),
            steer: Steer::default(),
            pad: PadState::default(),
            chat: ChatLine::default(),
            chat_mode: ChatMode::default(),
            sky: Sky::default(),
            floats: Floats::default(),
            tips: Tips::default(),
            reports: ShardReports::default(),
            hotbars: KeptHotbars::default(),
            panels: panels::PanelState::default(),
            view: Area::default(),
            kept_wanted: [HOTBAR_FILE, GRAB_BAGS_FILE]
                .iter()
                .map(|name| format!("{KEPT_PREFIX}{name}"))
                .collect(),
            covered: Vec::new(),
            pixels_per_point: 1.0,
            measure: None,
            last_tick: None,
            tooltip: None,
        };
        view.take_profile(profile);
        view
    }

    /// Takes a profile the page loaded, and its zoom.
    fn take_profile(&mut self, mut profile: Profile) {
        self.kept_style = profile.interface.ui_style;
        profile.interface.ui_style = UiStyle::Modern;
        self.scene.set_zoom(profile.video.default_zoom);
        self.profile = profile;
    }

    /// Takes one picture of the `watch` tool, as JSON, at the clock of the
    /// page in seconds.
    pub fn take_frame(&mut self, watch_json: &str, now: f64) {
        let mut frame = match serde_json::from_str::<Value>(watch_json) {
            Ok(watch) => {
                let frame = WatchFrame::from_observe(&watch, now);
                if let Some(live) = watch.get("live_map") {
                    self.art.see_live_map(live);
                }
                self.hand.set_watch(watch);
                frame
            }
            Err(error) => WatchFrame::error_frame(error.to_string()),
        };
        self.controls.received(&mut frame);
        self.frame = Some(frame);
    }

    /// The picture the view shows now.
    pub fn frame_ref(&self) -> Option<&WatchFrame> {
        self.frame.as_ref()
    }

    /// Measures the words of the name plates as the page draws them.
    pub fn set_measure(&mut self, measure: Measure) {
        self.measure = Some(measure);
    }

    /// Takes one event of the page. Gives the calls for the page that are
    /// waiting, with the ones a panel action made.
    pub fn input_native(&mut self, event_json: &str, now: f64) -> Vec<OutCall> {
        if let Ok(event) = serde_json::from_str::<InputEvent>(event_json) {
            match event {
                InputEvent::Panel { panel, action } => {
                    self.hand.begin(now);
                    self.panel_action(&panel, action);
                }
                InputEvent::ChatWords { text } => self.chat.text = text,
                other => self.inputs.read(other, now),
            }
        }
        self.take_out_native()
    }

    /// The calls for the page that are waiting.
    pub fn take_out_native(&mut self) -> Vec<OutCall> {
        self.hand.take_out()
    }

    /// The answer to call `id` came, as JSON.
    pub fn answer_native(&mut self, id: u64, ok: bool, result_json: &str, now: f64) {
        let result = serde_json::from_str(result_json)
            .unwrap_or_else(|_| Value::String(result_json.to_string()));
        self.hand.answered(id, ok, result, now);
    }

    /// The paths of the tables and kept files to get, each one time.
    pub fn data_wanted_native(&mut self) -> Vec<String> {
        let mut paths = std::mem::take(&mut self.kept_wanted);
        paths.extend(self.art.take_data_wanted());
        paths
    }

    /// The answer of a path came.
    pub fn data_arrived_native(&mut self, path: &str, answer: &Value) {
        match path.strip_prefix(KEPT_PREFIX) {
            Some(HOTBAR_FILE) => {
                self.keep_hotbars(serde_json::from_value(answer.clone()).unwrap_or_default());
            }
            Some(GRAB_BAGS_FILE) => {
                let bags: KeptGrabBags = serde_json::from_value(answer.clone()).unwrap_or_default();
                self.hand.keep_grab_bags(bags);
            }
            _ => {
                self.art.data_arrived(path, answer);
            }
        }
    }

    /// The server has no answer for a path. A kept file that is not there
    /// keeps nothing yet.
    pub fn data_missing_native(&mut self, path: &str) {
        if !path.starts_with(KEPT_PREFIX) {
            self.art.data_missing(path);
        }
    }

    /// Asks the art for a picture, and gives the key the page keeps its
    /// pixels under.
    fn picture_key(&mut self, request: &ArtRequest) -> String {
        let _ = self.art.sprite(request);
        request.key().to_string()
    }

    /// Runs one frame at `now` in a view of `view`, with the mouse at
    /// `mouse` when it is over the page. Gives what to draw.
    pub fn tick_native(&mut self, now: f64, view: Area, mouse: Option<Point>) -> DrawBuffers {
        let mut input = self.inputs.take_frame();
        // The clicks a controller or a key asked for in the last frame.
        clicks_of(self.controls.take_clicks(), mouse, &mut input);
        let seconds = self.last_tick.map_or(0.0, |last| (now - last).max(0.0)) as f32;
        self.last_tick = Some(now);
        self.view = view;
        let (sticks, buttons) = self.inputs.pad();
        let buttons = buttons.to_vec();
        let pad = self.pad.read(sticks, &buttons, &self.profile, seconds);
        self.controls.take_pad(pad);
        let frame = match self.frame.take() {
            Some(frame) if frame.error.is_empty() => frame,
            other => {
                self.frame = other;
                return self.art_only();
            }
        };
        let buffers = self.run_frame(&frame, now, view, mouse, input);
        self.frame = Some(frame);
        buffers
    }

    /// The buffers of a frame with no world: the art of the panels only.
    fn art_only(&mut self) -> DrawBuffers {
        DrawBuffers {
            uploads: self.art.take_uploads(),
            atlas_reset: self.art.take_atlas_reset(),
            ..DrawBuffers::default()
        }
    }
}

/// The tooltip of the thing under the mouse, on the map or on a panel:
/// the shard's words or its name, and what a click does.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TooltipData {
    pub lines: Vec<String>,
    pub footer: String,
}

/// The view for the page. Each method turns the values of the page into
/// the native ones and back.
#[wasm_bindgen]
impl WebView {
    /// A view with the profile, as JSON.
    #[wasm_bindgen(constructor)]
    pub fn new(profile_json: &str) -> WebView {
        Self::with_profile(profile_json)
    }

    /// One picture of the `watch` tool, as JSON, at the clock of the page
    /// in seconds. The same clock goes to every method that takes `now`.
    pub fn frame(&mut self, watch_json: &str, now: f64) {
        self.take_frame(watch_json, now);
    }

    /// Runs one frame in a view `width` by `height` points, with the mouse
    /// at `mouse_x`, `mouse_y` when `has_mouse`.
    pub fn tick(
        &mut self,
        now: f64,
        width: f32,
        height: f32,
        mouse_x: f32,
        mouse_y: f32,
        has_mouse: bool,
    ) -> DrawBuffers {
        let view = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(width, height));
        let mouse = has_mouse.then(|| Point::new(mouse_x, mouse_y));
        self.tick_native(now, view, mouse)
    }

    /// One `InputEvent`, as JSON. Gives the waiting `OutCall[]`.
    pub fn input(&mut self, event_json: &str, now: f64) -> JsValue {
        to_js(&self.input_native(event_json, now))
    }

    /// The waiting `OutCall[]`: the calls of the last tick and of the
    /// answers that came.
    #[wasm_bindgen(js_name = takeOut)]
    pub fn take_out(&mut self) -> JsValue {
        to_js(&self.take_out_native())
    }

    /// The answer to the call with `id`, its result as JSON. A failed
    /// answer carries the words of the fault.
    pub fn answer(&mut self, id: f64, ok: bool, result_json: &str, now: f64) {
        self.answer_native(id as u64, ok, result_json, now);
    }

    /// The pictures to post to `/v1/art`, each one time:
    /// `{key, request}[]`.
    #[wasm_bindgen(js_name = artWanted)]
    pub fn art_wanted(&mut self) -> JsValue {
        to_js(&self.art.take_wanted())
    }

    /// The picture of `key` came: its size and the point of it that goes
    /// on the tile (the `x-uoterm-anchor` header).
    #[wasm_bindgen(js_name = artArrived)]
    pub fn art_arrived(
        &mut self,
        key: &str,
        width: u32,
        height: u32,
        anchor_x: f32,
        anchor_y: f32,
    ) {
        if let Ok(key) = key.parse() {
            self.art
                .arrived(key, width as usize, height as usize, anchor_x, anchor_y);
        }
    }

    /// The server has no picture for `key`, or refused it.
    #[wasm_bindgen(js_name = artMissing)]
    pub fn art_missing(&mut self, key: &str) {
        if let Ok(key) = key.parse() {
            self.art.missing(key);
        }
    }

    /// The `/v1/...` paths to get, each one time: `string[]`.
    #[wasm_bindgen(js_name = dataWanted)]
    pub fn data_wanted(&mut self) -> JsValue {
        to_js(&self.data_wanted_native())
    }

    /// The answer of a path came, as JSON.
    #[wasm_bindgen(js_name = dataArrived)]
    pub fn data_arrived(&mut self, path: &str, json: &str) {
        match serde_json::from_str::<Value>(json) {
            Ok(answer) => self.data_arrived_native(path, &answer),
            Err(_) => self.data_missing_native(path),
        }
    }

    /// The server has no answer for a path.
    #[wasm_bindgen(js_name = dataMissing)]
    pub fn data_missing(&mut self, path: &str) {
        self.data_missing_native(path);
    }

    /// The keys of the pictures the view forgot: `string[]`. The page may
    /// drop their pixels; a picture forgotten comes again in `artWanted`
    /// when it is needed.
    #[wasm_bindgen(js_name = artForgotten)]
    pub fn art_forgotten(&mut self) -> JsValue {
        to_js(&self.art.take_forgotten())
    }

    /// The page lost the texture of the pictures (its WebGL context came
    /// back empty): the next frame clears it and places the pictures again,
    /// from the pixels the page keeps.
    #[wasm_bindgen(js_name = atlasLost)]
    pub fn atlas_lost(&mut self) {
        self.art.atlas_lost();
    }

    /// The bodies to post, each one time: `{key, path, body}[]`.
    #[wasm_bindgen(js_name = postsWanted)]
    pub fn posts_wanted(&mut self) -> JsValue {
        to_js(&self.art.take_posts())
    }

    /// The answer of the post of `key` came, as JSON.
    #[wasm_bindgen(js_name = postArrived)]
    pub fn post_arrived(&mut self, key: &str, json: &str) {
        let (Ok(key), Ok(answer)) = (key.parse(), serde_json::from_str::<Value>(json)) else {
            return self.post_missing(key);
        };
        self.art.post_arrived(key, &answer);
    }

    /// The post of `key` had no answer.
    #[wasm_bindgen(js_name = postMissing)]
    pub fn post_missing(&mut self, key: &str) {
        if let Ok(key) = key.parse() {
            self.art.post_missing(key);
        }
    }

    /// The data of the panels at `now`, as JSON `PanelData`: the page
    /// compares it with the last one and draws the panels only when it
    /// changed.
    #[wasm_bindgen(js_name = panelsJson)]
    pub fn panels_json(&mut self, now: f64) -> String {
        serde_json::to_string(&self.panel_data(now)).unwrap_or_default()
    }

    /// The clock of the computer now, as the page reads it, for the times
    /// of the journal: the year, the month and the day from one, the
    /// hours, the minutes and the seconds.
    #[wasm_bindgen(js_name = setLocalTime)]
    pub fn set_local_time_js(
        &mut self,
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) {
        self.set_local_time(uoterm_view::model::journal::LocalTime {
            year,
            month,
            day,
            hour,
            minute,
            second,
        });
    }

    /// The profile as JSON, as it is kept.
    pub fn profile(&self) -> String {
        serde_json::to_string(&kept(&self.profile, self.kept_style)).unwrap_or_default()
    }

    /// Takes the profile of the character the page loaded, as JSON, and its
    /// zoom. A profile that does not read changes nothing.
    #[wasm_bindgen(js_name = setProfile)]
    pub fn set_profile(&mut self, json: &str) {
        if let Ok(profile) = serde_json::from_str::<Profile>(json) {
            self.take_profile(profile);
        }
    }

    /// A screenshot the view asked for was saved at `words`, or failed for
    /// the reason in `words`.
    #[wasm_bindgen(js_name = screenshotTaken)]
    pub fn screenshot_taken(&mut self, ok: bool, words: &str) {
        let Some(frame) = self.frame.as_ref() else {
            return;
        };
        let saved = if ok {
            Ok(words.to_string())
        } else {
            Err(words.to_string())
        };
        self.controls.screenshot_taken(frame, &self.profile, saved);
    }

    /// Where the panels of the page lie, as JSON `Area[]`: the world takes
    /// no clicks and no wheel there.
    #[wasm_bindgen(js_name = setCovered)]
    pub fn set_covered(&mut self, json: &str) {
        self.covered = serde_json::from_str(json).unwrap_or_default();
    }

    /// How many screen pixels one point has, so the map moves in whole
    /// pixels.
    #[wasm_bindgen(js_name = setPixelsPerPoint)]
    pub fn set_pixels_per_point(&mut self, pixels_per_point: f32) {
        self.pixels_per_point = pixels_per_point;
    }

    /// The function that measures a name plate as the page draws it:
    /// `(text: string) => [width, height]`.
    #[wasm_bindgen(js_name = setTextMeasure)]
    pub fn set_text_measure(&mut self, measure: js_sys::Function) {
        self.set_measure(Box::new(move |text: &str| {
            let size = measure
                .call1(&JsValue::NULL, &JsValue::from_str(text))
                .ok()
                .and_then(|size| serde_wasm_bindgen::from_value::<[f32; 2]>(size).ok())
                .unwrap_or_default();
            Vector::new(size[0], size[1])
        }));
    }

    /// The time between two frames the Video page asks for, in
    /// milliseconds, while the page is focused or not.
    #[wasm_bindgen(js_name = frameIntervalMs)]
    pub fn frame_interval_ms(&self, focused: bool) -> f64 {
        frame_interval(&self.profile.video, focused).as_secs_f64() * MS_PER_SECOND
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::json;
    use uoterm_view::act::Act;
    use uoterm_view::actions::WindowCommand;
    use uoterm_view::art::Cell;
    use uoterm_view::input::Mods;
    use uoterm_view::settings::{KeyBinding, KeyChord, MacroStep};

    pub const MARA: &str = "Mara";
    pub const HATCHET: u32 = 0x4000_0010;
    const BACKPACK: u32 = 0x4000_0001;
    const ME: u32 = 0x0000_0001;
    pub const VIEW: Area = Area {
        min: Point { x: 0.0, y: 0.0 },
        max: Point { x: 800.0, y: 600.0 },
    };

    /// A picture of Mara in the world, with control, and a hatchet in her
    /// backpack.
    pub fn fixture_watch_with_backpack() -> String {
        json!({
            "self_state": {
                "serial": ME,
                "name": MARA,
                "location": { "x": 1000, "y": 1000, "z": 0 },
                "map": 0,
                "hits": 50, "hits_max": 50
            },
            "human_control": true,
            "radar": ".....\n.....\n..@..\n.....\n.....",
            "backpack": {
                "serial": BACKPACK,
                "items": [{ "serial": HATCHET, "graphic": 0x0F43, "name": "hatchet", "amount": 1 }]
            }
        })
        .to_string()
    }

    fn event(value: serde_json::Value) -> String {
        value.to_string()
    }

    fn view_in_world() -> WebView {
        let mut view = WebView::new(&serde_json::to_string(&Profile::default()).unwrap());
        view.frame(&fixture_watch_with_backpack(), 0.0);
        view
    }

    /// A view that drew its first frame, with what the first frame tells
    /// the shard by itself sent.
    pub fn settled() -> WebView {
        let mut view = view_in_world();
        view.tick_native(0.0, VIEW, None);
        view.take_out_native();
        view
    }

    #[test]
    fn the_first_frame_of_a_human_tells_the_shard_the_house_choice() {
        let mut view = view_in_world();
        view.tick_native(0.0, VIEW, None);
        let house = Profile::default().general.show_house_content;
        assert_eq!(
            acts(&view.take_out_native()),
            [Act::HouseContent(house).for_page()]
        );
    }

    fn acts(out: &[OutCall]) -> Vec<uoterm_view::act::PageAct> {
        out.iter()
            .filter_map(|call| match call {
                OutCall::Act { act, .. } => Some(act.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_frame_draws_the_world_and_lays_the_white_square_first() {
        let mut view = view_in_world();
        let buffers = view.tick_native(0.0, VIEW, None);
        assert!(!buffers.world.indices.is_empty());
        assert_eq!(buffers.world.positions.len(), buffers.world.uvs.len());
        assert!(buffers.atlas_reset);
        let next = view.tick_native(0.1, VIEW, None);
        assert!(!next.atlas_reset, "once");
    }

    #[test]
    fn no_picture_draws_no_world() {
        let mut view = WebView::new("{}");
        let buffers = view.tick_native(0.0, VIEW, None);
        assert!(buffers.world.indices.is_empty());
        view.frame("not json", 0.0);
        assert!(!view.frame_ref().unwrap().error.is_empty());
        assert!(view.tick_native(0.1, VIEW, None).world.indices.is_empty());
    }

    #[test]
    fn tab_held_is_war_as_in_the_window() {
        let mut view = settled();
        view.input_native(
            &event(json!({"kind": "Key", "key": "Tab", "pressed": true})),
            0.0,
        );
        view.tick_native(0.0, VIEW, None);
        assert_eq!(acts(&view.take_out_native()), [Act::War(true).for_page()]);
    }

    #[test]
    fn a_held_arrow_walks_and_its_release_stops() {
        let mut view = settled();
        view.input_native(
            &event(json!({"kind": "Key", "key": "Up", "pressed": true})),
            0.0,
        );
        view.tick_native(0.0, VIEW, None);
        let walked = acts(&view.take_out_native());
        assert_eq!(walked.len(), 1);
        assert_eq!(walked[0].calls[0].tool, uoterm_world::tool_names::TOOL_WALK);
        view.input_native(
            &event(json!({"kind": "Key", "key": "Up", "pressed": false})),
            0.1,
        );
        view.tick_native(0.1, VIEW, None);
        assert_eq!(acts(&view.take_out_native()), [Act::Stop.for_page()]);
    }

    #[test]
    fn a_double_click_on_the_ground_walks_there_when_pathfinding_is_on() {
        let mut view = settled();
        let mut profile = Profile::default();
        profile.general.pathfinding = true;
        profile.general.shift_pathfinding = false;
        profile.general.click_to_run = false;
        view.set_profile(&serde_json::to_string(&profile).unwrap());
        let at = VIEW.center() + Vector::new(0.0, 66.0);
        for double in [false, true] {
            let down = json!({"kind": "PointerDown", "button": "Primary", "double": double});
            let up = json!({"kind": "PointerUp", "x": at.x, "y": at.y, "button": "Primary"});
            view.input_native(&event(down), 0.1);
            view.input_native(&event(up), 0.1);
        }
        view.tick_native(0.1, VIEW, Some(at));
        let walked = acts(&view.take_out_native());
        assert_eq!(walked.len(), 1, "{walked:?}");
        assert_eq!(
            walked[0].calls[0].tool,
            uoterm_world::tool_names::TOOL_MOVE_TO
        );
    }

    #[test]
    fn a_building_goes_where_a_click_on_the_map_came_up() {
        let mut view = WebView::new("{}");
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["placing"] = json!({ "multi_id": 1 });
        view.frame(&watch.to_string(), 0.0);
        view.tick_native(0.0, VIEW, None);
        view.take_out_native();
        let panel = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(100.0, 100.0));
        view.set_covered(&serde_json::to_string(&[panel]).unwrap());
        let click = |view: &mut WebView, at: Point| {
            view.input_native(
                &event(json!({"kind": "PointerDown", "button": "Primary"})),
                0.1,
            );
            let up = json!({"kind": "PointerUp", "x": at.x, "y": at.y, "button": "Primary"});
            view.input_native(&event(up), 0.1);
            view.tick_native(0.1, VIEW, Some(VIEW.center()));
            acts(&view.take_out_native())
        };
        assert!(
            click(&mut view, panel.center()).is_empty(),
            "a click on a panel"
        );
        let placed = click(&mut view, VIEW.center());
        assert_eq!(placed.len(), 1);
        assert_eq!(
            placed[0].calls[0].tool,
            uoterm_world::tool_names::TOOL_TARGET
        );
    }

    #[test]
    fn a_block_in_a_wide_view_at_the_lowest_zoom_is_not_asked_for_again() {
        const WIDE: Area = Area {
            min: Point { x: 0.0, y: 0.0 },
            max: Point {
                x: 1920.0,
                y: 1080.0,
            },
        };
        const LOWEST_ZOOM: f32 = 0.1;
        const FRAMES: usize = 5;
        let mut profile = Profile::default();
        profile.video.default_zoom = LOWEST_ZOOM;
        let mut view = WebView::new(&serde_json::to_string(&profile).unwrap());
        view.frame(&fixture_watch_with_backpack(), 0.0);
        view.data_arrived_native("/v1/data/tiledata", &json!(uoterm_nav::TileData::default()));
        let cells = json!(vec![Cell::default(); 64]);
        let mut asked = 0;
        for at in 0..FRAMES {
            view.tick_native(at as f64 / 10.0, WIDE, None);
            for path in view.data_wanted_native() {
                if path.starts_with("/v1/map/") {
                    asked += usize::from(at > 0);
                    view.data_arrived_native(&path, &cells);
                }
            }
        }
        assert_eq!(asked, 0, "every block came in the first frame and stays");
    }

    #[test]
    fn a_chat_line_sends_its_words_on_enter() {
        let mut view = settled();
        view.input_native(
            &event(json!({"kind": "Focus", "chat_focused": true, "other_field_focused": false})),
            0.0,
        );
        view.input_native(&event(json!({"kind": "ChatWords", "text": "hail"})), 0.0);
        view.input_native(
            &event(json!({"kind": "Key", "key": "Enter", "pressed": true})),
            0.0,
        );
        view.tick_native(0.0, VIEW, None);
        let said = acts(&view.take_out_native());
        assert_eq!(said.len(), 1);
        assert_eq!(said[0].words, "Said: hail");
        assert!(view.panel_data(0.0).chat.text.is_empty());
    }

    #[test]
    fn escape_cancels_the_target_cursor_of_the_shard() {
        let mut view = WebView::new("{}");
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["pending_target"] = json!(true);
        view.frame(&watch.to_string(), 0.0);
        view.input_native(
            &event(json!({"kind": "Key", "key": "Escape", "pressed": true})),
            0.0,
        );
        view.tick_native(0.0, VIEW, None);
        assert!(acts(&view.take_out_native()).contains(&Act::CancelTarget.for_page()));
    }

    #[test]
    fn the_wheel_zooms_the_map_and_a_screenshot_goes_to_the_journal() {
        let mut view = view_in_world();
        let zoom = view.scene.zoom();
        view.input_native(&event(json!({"kind": "Wheel", "notches": 2})), 0.0);
        view.tick_native(0.0, VIEW, Some(VIEW.center()));
        assert!(view.scene.zoom() > zoom);
        view.screenshot_taken(true, "shot.png");
        view.frame(&fixture_watch_with_backpack(), 0.1);
        let journal = &view.frame_ref().unwrap().journal;
        assert!(
            journal.iter().any(|line| line.contains("shot.png")),
            "{journal:?}"
        );
    }

    #[test]
    fn the_name_plates_are_laid_out_with_the_measure_of_the_page() {
        let mut view = WebView::new("{}");
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["mobiles"] = json!([{
            "serial": 0x0000_0002, "name": "an orc", "notoriety": 6,
            "location": { "x": 1001, "y": 1000, "z": 0 }, "body": 17
        }]);
        view.frame(&watch.to_string(), 0.0);
        view.set_measure(Box::new(|text: &str| Vector::new(text.len() as f32, 10.0)));
        view.tick_native(0.0, VIEW, None);
        // The server has no animation files: the orc is a plain figure.
        for path in view.data_wanted_native() {
            if path.starts_with("/v1/data/frames/") {
                view.data_missing_native(&path);
            }
        }
        let buffers = view.tick_native(0.1, VIEW, None);
        assert!(
            buffers.plates.iter().any(|plate| plate.name == "an orc"),
            "{:?}",
            buffers.plates
        );
    }

    #[test]
    fn the_browser_shows_the_modern_style_and_keeps_the_style_of_the_profile() {
        let mut profile = Profile::default();
        profile.interface.ui_style = UiStyle::Classic;
        profile.macros.key_bindings.push(KeyBinding {
            name: "save".into(),
            chord: Some("F5".parse::<KeyChord>().unwrap()),
            pad: None,
            steps: vec![MacroStep::new("save_desktop", "")],
        });
        let mut view = WebView::new(&serde_json::to_string(&profile).unwrap());
        view.frame(&fixture_watch_with_backpack(), 0.0);
        assert_eq!(view.profile.interface.ui_style, UiStyle::Modern);
        let kept: Profile = serde_json::from_str(&view.profile()).unwrap();
        assert_eq!(kept.interface.ui_style, UiStyle::Classic);
        view.input_native(
            &event(json!({"kind": "Key", "key": "F5", "pressed": true})),
            0.0,
        );
        view.tick_native(0.0, VIEW, None);
        let saved: Vec<Value> = view
            .take_out_native()
            .into_iter()
            .filter_map(|call| match call {
                OutCall::SaveProfile { profile } => Some(profile),
                _ => None,
            })
            .collect();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0]["interface"]["ui_style"], json!("Classic"));
    }

    #[test]
    fn a_controller_button_runs_its_macro_and_a_stick_walks() {
        let mut profile = Profile::default();
        profile.macros.controller_enabled = true;
        let mut view = WebView::new(&serde_json::to_string(&profile).unwrap());
        view.frame(&fixture_watch_with_backpack(), 0.0);
        view.tick_native(0.0, VIEW, None);
        view.take_out_native();
        let north = json!({"kind": "Pad", "sticks": [0, 0, 0, 0], "buttons": ["North"]});
        view.input_native(&event(north), 0.1);
        view.tick_native(0.1, VIEW, None);
        assert_eq!(acts(&view.take_out_native()), [Act::War(true).for_page()]);
        let left = json!({"kind": "Pad", "sticks": [0, 1, 0, 0], "buttons": []});
        view.input_native(&event(left), 0.3);
        view.tick_native(0.3, VIEW, None);
        let walked = acts(&view.take_out_native());
        assert_eq!(walked.len(), 1);
        assert_eq!(walked[0].calls[0].tool, uoterm_world::tool_names::TOOL_WALK);
    }

    #[test]
    fn the_view_hides_its_chat_line_and_the_page_closes_the_corpses() {
        let mut profile = Profile::default();
        for (key, action) in [("F7", "toggle_chat"), ("F8", "close_corpses")] {
            profile.macros.key_bindings.push(KeyBinding {
                name: action.into(),
                chord: Some(key.parse::<KeyChord>().unwrap()),
                pad: None,
                steps: vec![MacroStep::new(action, "")],
            });
        }
        let mut view = WebView::new(&serde_json::to_string(&profile).unwrap());
        view.frame(&fixture_watch_with_backpack(), 0.0);
        // One macro runs at a time, so each key has a frame of its own.
        for (at, key) in ["F7", "F8"].into_iter().enumerate() {
            let now = at as f64;
            view.input_native(
                &event(json!({"kind": "Key", "key": key, "pressed": true})),
                now,
            );
            view.tick_native(now, VIEW, None);
        }
        assert!(view.panel_data(0.0).chat.hidden);
        assert!(view.take_out_native().contains(&OutCall::Window {
            command: WindowCommand::CloseCorpses
        }));
    }

    #[test]
    fn a_screenshot_key_asks_the_page_for_a_picture() {
        let mut profile = Profile::default();
        profile.macros.key_bindings.push(KeyBinding {
            name: "shot".into(),
            chord: Some("F6".parse::<KeyChord>().unwrap()),
            pad: None,
            steps: vec![MacroStep::new("screenshot", "")],
        });
        let mut view = WebView::new(&serde_json::to_string(&profile).unwrap());
        view.frame(&fixture_watch_with_backpack(), 0.0);
        view.input_native(
            &event(json!({"kind": "Key", "key": "F6", "pressed": true})),
            0.0,
        );
        view.tick_native(0.0, VIEW, None);
        assert!(view.take_out_native().contains(&OutCall::Screenshot));
    }

    #[test]
    fn the_kept_files_and_the_tables_are_asked_for_once() {
        let mut view = WebView::new("{}");
        let paths = view.data_wanted_native();
        assert!(paths.contains(&format!("{KEPT_PREFIX}{HOTBAR_FILE}")));
        assert!(paths.contains(&format!("{KEPT_PREFIX}{GRAB_BAGS_FILE}")));
        assert!(paths.iter().any(|path| path.starts_with("/v1/data/")));
        assert!(view.data_wanted_native().is_empty());
    }

    #[test]
    fn a_used_key_types_nothing_in_the_chat_line() {
        let mut view = view_in_world();
        let mut profile = Profile::default();
        profile.speech.chat_on_enter = true;
        profile.speech.chat_prefix_keys = true;
        view.set_profile(&serde_json::to_string(&profile).unwrap());
        let shift = Mods {
            shift: true,
            ..Mods::default()
        };
        let semicolon = json!({"kind": "Key", "key": "Semicolon", "pressed": true, "mods": shift});
        view.input_native(&event(semicolon), 0.0);
        view.input_native(&event(json!({"kind": "Text", "text": ":"})), 0.0);
        view.tick_native(0.0, VIEW, None);
        assert!(
            view.panel_data(0.0).chat.open,
            "a prefix key opens the line"
        );
    }

    const ORC: u32 = 0x0000_0002;

    /// A view of Mara in control with an orc beside her, drawn as a plain
    /// figure, and the place of the orc on the map.
    fn view_with_orc() -> (WebView, Point) {
        let mut view = WebView::new("{}");
        let mut watch: Value = serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch["mobiles"] = json!([{
            "serial": ORC, "name": "an orc", "notoriety": 6,
            "location": { "x": 1001, "y": 1000, "z": 0 }, "body": 17
        }]);
        view.frame(&watch.to_string(), 0.0);
        view.tick_native(0.0, VIEW, None);
        for path in view.data_wanted_native() {
            if path.starts_with("/v1/data/frames/") {
                view.data_missing_native(&path);
            }
        }
        view.tick_native(0.1, VIEW, None);
        view.take_out_native();
        let orc = view
            .scene
            .picks()
            .iter()
            .find(|pick| pick.serial == ORC)
            .map(|pick| pick.area.center())
            .expect("the orc is drawn");
        (view, orc)
    }

    #[test]
    fn a_right_click_on_a_mobile_of_the_map_opens_its_ring() {
        let (mut view, orc) = view_with_orc();
        let down = json!({"kind": "PointerDown", "button": "Secondary", "x": orc.x, "y": orc.y});
        let up = json!({"kind": "PointerUp", "x": orc.x, "y": orc.y, "button": "Secondary"});
        view.input_native(&event(down), 0.2);
        view.input_native(&event(up), 0.25);
        view.tick_native(0.3, VIEW, Some(orc));
        assert!(acts(&view.take_out_native()).contains(&Act::Menu(ORC).for_page()));
        assert_eq!(view.panel_data(0.3).ring.unwrap().name, "an orc");
    }

    #[test]
    fn a_drag_off_a_mobile_of_the_map_pulls_out_its_health_bar() {
        let (mut view, orc) = view_with_orc();
        let down = json!({"kind": "PointerDown", "button": "Primary", "x": orc.x, "y": orc.y});
        view.input_native(&event(down), 0.2);
        let away = orc + Vector::new(80.0, 40.0);
        view.tick_native(0.3, VIEW, Some(away));
        view.tick_native(0.4, VIEW, Some(away));
        let bars = view.panel_data(0.4).bars;
        assert_eq!(bars.len(), 1);
        let up = json!({"kind": "PointerUp", "x": away.x, "y": away.y, "button": "Primary"});
        view.input_native(&event(up), 0.5);
        view.tick_native(0.5, VIEW, Some(away));
        let clicked = acts(&view.take_out_native()).into_iter().any(|act| {
            act.calls.iter().any(|call| {
                [
                    uoterm_world::tool_names::TOOL_USE,
                    uoterm_world::tool_names::TOOL_MOVE_TO,
                ]
                .contains(&call.tool.as_str())
            })
        });
        assert!(!clicked, "a drag is no click");
    }

    #[test]
    fn the_window_commands_of_the_panels_are_the_views_and_the_rest_the_pages() {
        let mut view = settled();
        let frame = view.frame_ref().unwrap().clone();
        let toggle = |kind| WindowCommand::Gump(uoterm_view::actions::GumpOp::Toggle, kind);
        view.style_command(&frame, toggle(uoterm_view::actions::GumpKind::Journal));
        assert!(view.panel_data(0.0).journal.is_none());
        view.style_command(&frame, toggle(uoterm_view::actions::GumpKind::Skills));
        let sheet = view.panel_data(0.0).sheet.unwrap().body;
        assert!(sheet.skills.is_some());
        view.style_command(&frame, WindowCommand::QuitGame);
        assert!(view.panel_data(0.0).question.is_some());
        view.take_out_native();
        view.style_command(&frame, toggle(uoterm_view::actions::GumpKind::WorldMap));
        assert!(matches!(
            view.take_out_native().as_slice(),
            [OutCall::Window { .. }]
        ));
    }

    #[test]
    fn the_chat_line_asks_its_field_for_the_keys_and_lets_them_go_on_escape() {
        let mut view = settled();
        view.tick_native(0.1, VIEW, None);
        view.panel_data(0.1);
        view.panel_data(0.1);
        assert!(view
            .take_out_native()
            .contains(&OutCall::ChatFocus { take: true }));
        view.input_native(
            &event(json!({"kind": "Focus", "chat_focused": true, "other_field_focused": false})),
            0.2,
        );
        view.input_native(
            &event(json!({"kind": "Key", "key": "Escape", "pressed": true})),
            0.2,
        );
        view.tick_native(0.2, VIEW, None);
        assert!(view
            .take_out_native()
            .contains(&OutCall::ChatFocus { take: false }));
    }
}
