//! The play window of the browser, as WebAssembly. The page gives the view
//! each picture of the session, its input and the art it fetched; each
//! frame the view runs the same rules as the Modern style of the Rust
//! window (`uoterm_view`) and gives back what to draw, the calls to make on
//! the live link, and the data of the panels. No rule lives here: this
//! crate only joins the shared rules to the types of the page.
//!
//! Every method has a native form for the tests; the `#[wasm_bindgen]`
//! methods only turn JavaScript values into Rust ones and back.

mod buffers;
mod input;
mod out;
mod panels;
mod synth;
mod web_art;

pub use buffers::{DrawBuffers, PlacedWords, Shapes};
pub use input::{Click, FrameInput, InputEvent, Inputs};
pub use out::{Hand, OutCall, JEV_ORDER};
pub use panels::{
    ChatData, HotbarAction, HotbarData, HotbarSlot, PanelData, QuestionAction, PANEL_HOTBAR,
    PANEL_QUESTION,
};
pub use synth::{render, render_midi};
pub use web_art::{Post, Upload, Wanted, WebArt, BLOCK_KEEP_RADIUS};

use buffers::MeshArrays;
use serde::Serialize;
use serde_json::Value;
use std::borrow::Cow;
use uoterm_view::act::Act;
use uoterm_view::actions::controls::{ControlHost, Controls, FrameIn};
use uoterm_view::actions::{LocalAim, PointerClick};
use uoterm_view::art::{hue_color, ArtRequest, ItemPaint, TextLook, WorldArt};
use uoterm_view::clicks::{act_for_click, escape_on_map, ChatMode, EscapeOnMap, GroundClicks};
use uoterm_view::floats::{self, Floats, SPEECH_LINE};
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point, Rgba, Vector};
use uoterm_view::guard::KeptGrabBags;
use uoterm_view::guard::GRAB_BAGS_FILE;
use uoterm_view::input::KeyPress;
use uoterm_view::keys::chat::{ChatKey, ChatLine, ChatOut, Said};
use uoterm_view::keys::Focus;
use uoterm_view::model::asked::asked_commands;
use uoterm_view::model::game_view::ShardReports;
use uoterm_view::pad::PadState;
use uoterm_view::scene::plates::lay_out;
use uoterm_view::scene::{
    overlays, SceneDraw, SceneInput, SceneState, DEATH_FONT, DEATH_HUE, DEATH_WORDS,
};
use uoterm_view::settings::{CombatOptions, Profile, UiStyle};
use uoterm_view::sky::{
    drop_color, effect_area, lightning_bolt, lit_effects, shown_effects, storm_tint, weather_drops,
    Drop, ShownEffect, Sky, LIGHTNING_WIDTH, RAIN_WIDTH, SNOW_RADIUS,
};
use uoterm_view::steer::{Movement, Steer, SteerInput};
use uoterm_view::tips::Tips;
use uoterm_view::ui::deck::{hotbar_key_slot, KeptHotbars, HOTBAR_FILE};
use uoterm_view::video::frame_interval;
use wasm_bindgen::prelude::*;

/// The kept files of the config folder the view reads, under this path.
pub(crate) const KEPT_PREFIX: &str = "/v1/kept/";
const ESCAPE_KEY: &str = "Escape";
const ENTER_KEY: &str = "Enter";
const MS_PER_SECOND: f64 = 1000.0;

/// The profile as it is kept: with the UI style the Rust window keeps in
/// it. The browser shows the Modern style only, so the view holds its
/// profile in that style.
fn kept(profile: &Profile, kept_style: UiStyle) -> Cow<'_, Profile> {
    if profile.interface.ui_style == kept_style {
        Cow::Borrowed(profile)
    } else {
        let mut kept = profile.clone();
        kept.interface.ui_style = kept_style;
        Cow::Owned(kept)
    }
}

/// A list for the page, as plain JavaScript values.
fn to_js<T: Serialize>(value: &T) -> JsValue {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .unwrap_or(JsValue::NULL)
}

/// The view as the shared controls see it for one frame.
struct WebHost<'a> {
    hand: &'a mut Hand,
    scene: &'a mut SceneState,
    kept_style: UiStyle,
}

impl ControlHost for WebHost<'_> {
    fn act(&mut self, act: Act) {
        self.hand.act(act);
    }

    fn aim(&mut self, aim: LocalAim) {
        self.hand.aim(aim);
    }

    fn watch_over(&mut self, frame: &WatchFrame, combat: &CombatOptions) {
        self.hand.watch_over(frame, combat);
    }

    fn take_notes(&mut self) -> Vec<String> {
        self.hand.take_notes()
    }

    fn zoom(&self) -> f32 {
        self.scene.zoom()
    }

    fn set_zoom(&mut self, zoom: f32) {
        self.scene.set_zoom(zoom);
    }

    fn set_peek(&mut self, peek: Vector) {
        self.scene.set_peek(peek);
    }

    fn ask_screenshot(&mut self) {
        self.hand.push(OutCall::Screenshot);
    }

    fn save_profile(&mut self, profile: &Profile) {
        if let Ok(profile) = serde_json::to_value(kept(profile, self.kept_style)) {
            self.hand.push(OutCall::SaveProfile { profile });
        }
    }
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
                other => self.inputs.read(other),
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

    /// One frame with a picture of the session, in the order of the frame
    /// of the Rust window: the controls, the map, the shard reports, the
    /// sky, the floats, then the clicks on the map and the chat line.
    fn run_frame(
        &mut self,
        frame: &WatchFrame,
        now: f64,
        view: Area,
        mouse: Option<Point>,
        input: FrameInput,
    ) -> DrawBuffers {
        let focus = self.inputs.focus(self.chat.text.is_empty());
        let mut host = WebHost {
            hand: &mut self.hand,
            scene: &mut self.scene,
            kept_style: self.kept_style,
        };
        let controls = self.controls.frame(
            FrameIn {
                frame,
                profile: &mut self.profile,
                time: now,
                presses: &input.presses,
                focus,
                mouse,
                view,
                pad_note: "",
            },
            &mut host,
        );
        for command in controls.style {
            self.hand.push(OutCall::Window { command });
        }
        match controls.history_older {
            Some(true) => self.chat.older(),
            Some(false) => self.chat.newer(),
            None => {}
        }
        let unused: Vec<KeyPress> = input
            .presses
            .iter()
            .filter(|press| press.pressed)
            .filter(|press| {
                !controls
                    .used
                    .iter()
                    .any(|(mods, key)| *key == press.key && *mods == press.mods)
            })
            .cloned()
            .collect();
        // A key that fired something types nothing, as egui drops the
        // letters of a frame whose key was used.
        let texts: &[String] = if controls.used.is_empty() {
            &input.texts
        } else {
            &[]
        };
        let mods = self.inputs.mods();
        let scene_input = SceneInput {
            mouse,
            scroll: input.scroll,
            zoom_delta: input.zoom_delta,
            ctrl: mods.ctrl,
            shift: mods.shift,
            pixels_per_point: self.pixels_per_point,
        };
        let draw = self
            .scene
            .build(&mut self.art, view, frame, now, &self.profile, &scene_input);
        let mut world = Shapes::new(self.art.white_uv());
        let mut overlay = Shapes::new(self.art.white_uv());
        let mut buffers = DrawBuffers {
            moving: draw.moving,
            ..DrawBuffers::default()
        };
        self.lay_world(view, draw, &mut world, &mut buffers);
        let arrivals = self.scene.take_arrivals();
        for act in self.reports.acts(frame, &self.profile, arrivals, view, now) {
            self.hand.act(act);
        }
        buffers.moving |= self.lay_sky(frame, now, view, &mut world, &mut buffers);
        buffers.floats = self.lay_floats(frame, now, view);
        buffers.moving |= !self.floats.live().is_empty();
        self.tips.begin(self.hand.take_tips(), now);
        self.hand.begin(now);
        if let Some((arrow, _)) = overlays::quest_arrow(&self.scene.projection(view), frame) {
            overlay.overlay(arrow);
        }
        self.tooltip = None;
        // The hotbar, the chat line, then the map, as the Rust window draws
        // them.
        if frame.human_control {
            if focus == Focus::Free {
                for slot in unused
                    .iter()
                    .filter_map(|press| hotbar_key_slot(&press.key))
                {
                    self.press_slot(frame, slot);
                }
            }
            if !self.chat.is_hidden() {
                self.chat_keys(frame, &unused, texts);
            }
            let escape = unused.iter().any(|press| press.key.0 == ESCAPE_KEY);
            let on_map = MapInput {
                view,
                mouse,
                escape,
                movement: &controls.movement,
            };
            self.act_on_map(frame, now, &input, on_map, &mut overlay);
            buffers.moving |= self.steer.walks();
        }
        self.hand.ask_due();
        self.scene.set_panels(self.covered.clone());
        self.art.keep_near(frame.map, frame.x, frame.y);
        buffers.world = MeshArrays::from(world.into_mesh());
        buffers.overlay = MeshArrays::from(overlay.into_mesh());
        buffers.uploads = self.art.take_uploads();
        buffers.atlas_reset = self.art.take_atlas_reset();
        buffers
    }

    /// The world, the marks over it and the name plates; the death screen
    /// in their place while it shows.
    fn lay_world(
        &mut self,
        view: Area,
        draw: SceneDraw,
        world: &mut Shapes,
        buffers: &mut DrawBuffers,
    ) {
        buffers.steps = draw.steps;
        if draw.death.is_some() {
            world.fill(view, BLACK);
            let words = ArtRequest::Text {
                text: DEATH_WORDS.to_string(),
                look: TextLook::ascii(DEATH_FONT, DEATH_HUE),
            };
            if let Some(sprite) = self.art.sprite(&words).ready() {
                let size = Vector::new(sprite.width, sprite.height);
                world.picture(
                    Area::from_center_size(view.center(), size),
                    sprite.uv,
                    Rgba::WHITE,
                );
            }
            buffers.moving = true;
            return;
        }
        world.append(draw.mesh);
        for shape in draw.overlays {
            world.overlay(shape);
        }
        let combat = &self.profile.combat;
        if combat.range_circle {
            let color = hue_color(&self.art, combat.range_circle_hue);
            let projection = self.scene.projection(view);
            world.overlay(overlays::range_diamond(
                &projection,
                combat.range_circle_tiles,
                color,
            ));
        }
        if let Some(measure) = &self.measure {
            let keep_clear = self.scene.character_area(view);
            buffers.plates = lay_out(draw.plates, keep_clear, self.scene.zoom(), measure.as_ref());
        }
    }

    /// The pictures of spells and the weather under the light, and the
    /// light map. True while something of the sky still moves.
    fn lay_sky(
        &mut self,
        frame: &WatchFrame,
        now: f64,
        view: Area,
        world: &mut Shapes,
        buffers: &mut DrawBuffers,
    ) -> bool {
        self.sky.take_in(frame, now);
        let scene = &self.scene;
        let shown = shown_effects(&self.sky, now, |serial| scene.place_of(frame, serial));
        for effect in &shown {
            match *effect {
                ShownEffect::Bolt { struck, born } => {
                    let struck = self.scene.screen_of(view, struck);
                    let bolt = lightning_bolt(view, struck, born);
                    world.line(&bolt, false, LIGHTNING_WIDTH, Rgba::WHITE);
                }
                ShownEffect::Picture {
                    place,
                    graphic,
                    hue,
                } => {
                    let paint = ItemPaint {
                        hue,
                        ..ItemPaint::default()
                    };
                    let picture = self.scene.item_sprite(&mut self.art, graphic, paint, true);
                    if let Some(sprite) = picture.ready() {
                        let foot = self.scene.screen_of(view, place);
                        let zoom = self.scene.zoom();
                        let area = effect_area(foot, sprite.width, sprite.height, zoom);
                        world.picture(area, sprite.uv, Rgba::WHITE);
                    }
                }
            }
        }
        let weather = frame.weather.filter(|_| self.profile.video.weather_effects);
        if let Some((kind, count)) = weather {
            if let Some(tint) = storm_tint(kind) {
                world.fill(view, tint);
            }
            let color = drop_color(kind);
            for drop in weather_drops(view, kind, count, now) {
                match drop {
                    Drop::Flake(middle) => world.disc(middle, SNOW_RADIUS, color),
                    Drop::Streak(tail, head) => world.segment(tail, head, RAIN_WIDTH, color),
                }
            }
        }
        buffers.light = self
            .scene
            .light_cells(&mut self.art, view, frame, &lit_effects(&shown));
        !self.sky.live().is_empty() || weather.is_some()
    }

    /// The words and numbers over heads, laid out for the page to draw.
    fn lay_floats(&mut self, frame: &WatchFrame, now: f64, view: Area) -> Vec<PlacedWords> {
        let art = &self.art;
        self.floats.take_in(
            frame,
            now,
            &self.profile,
            |words, look| art.text_lines(words, look),
            |hue| hue_color(art, hue),
        );
        let live = self.floats.live();
        let heads: Vec<Option<Point>> = live
            .iter()
            .map(|float| self.scene.head_of(view, frame, float.serial))
            .collect();
        let fading = self.profile.general.text_fading;
        floats::lay_out(live, now, fading, |at| heads[at], |_| SPEECH_LINE)
            .into_iter()
            .map(|place| {
                let float = &live[place.index];
                PlacedWords {
                    words: float.words.clone(),
                    x: place.bottom.x,
                    y: place.bottom.y,
                    color: float.color,
                    alpha: place.alpha,
                    number: float.number,
                }
            })
            .collect()
    }

    /// The keys and the clicks of the human on the map: Escape, walking,
    /// a building that waits for its place, the tooltip, and the act of a
    /// click.
    fn act_on_map(
        &mut self,
        frame: &WatchFrame,
        now: f64,
        input: &FrameInput,
        on_map: MapInput<'_>,
        overlay: &mut Shapes,
    ) {
        let MapInput {
            view,
            mouse,
            escape,
            movement,
        } = on_map;
        match escape_on_map(self.hand.aiming().is_some(), frame.target_cursor).filter(|_| escape) {
            Some(EscapeOnMap::CancelAim) => self.hand.cancel_aim(),
            Some(EscapeOnMap::Act(act)) => self.hand.act(act),
            None => {}
        }
        let covered = self.covered.clone();
        let on_map =
            |at: &Point| view.contains(*at) && !covered.iter().any(|area| area.contains(*at));
        let mouse_on_map = mouse.filter(on_map);
        let character = self
            .scene
            .place_of(frame, frame.serial)
            .map_or(view.center(), |place| self.scene.screen_of(view, place));
        let steer_input = SteerInput {
            shift: self.inputs.mods().shift,
            right_down: self.inputs.secondary_down(),
            right_pressed: input.secondary_pressed,
            left_pressed: input.primary_pressed,
            mouse_way: mouse_on_map.map(|mouse| (character, mouse)),
        };
        let keys_down = self.inputs.keys_down().to_vec();
        for act in self.steer.decide(&keys_down, steer_input, now, movement) {
            self.hand.act(act);
        }
        let Some(mouse) = mouse_on_map else {
            return;
        };
        // The house designer takes the clicks on the house while it is open.
        if self.steer.by_mouse() || frame.designing.is_some() {
            return;
        }
        if let Some(shapes) = self
            .scene
            .placing_preview(&mut self.art, view, frame, mouse)
        {
            for shape in shapes {
                overlay.overlay(shape);
            }
            if !input.clicks.is_empty() {
                let (x, y, z) = self.scene.tile_at(&mut self.art, view, frame, mouse);
                self.hand.act(Act::TargetGround { x, y, z });
            }
            return;
        }
        let thing = self.scene.thing_at(mouse).cloned();
        self.tooltip = match (self.hand.aiming(), &thing) {
            (Some(aim), _) => Some(TooltipData {
                lines: vec![uoterm_view::guard::aim_words(aim).to_string()],
                footer: "",
            }),
            (None, Some(thing)) => {
                let hand = &mut self.hand;
                self.tips
                    .rest_on(thing.serial, now, |serial| hand.want_tip(serial));
                Some(TooltipData {
                    lines: self
                        .tips
                        .shown(thing.serial, &thing.name, &[])
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    footer: uoterm_view::clicks::hint_for(frame, thing.kind),
                })
            }
            (None, None) => None,
        };
        // A click acts on what lies where its button came up.
        for click in input.clicks.iter().filter(|click| on_map(&click.at)) {
            let picked = self
                .scene
                .thing_at(click.at)
                .map(|thing| (thing.serial, thing.kind));
            let tile = self.scene.tile_at(&mut self.art, view, frame, click.at);
            let ground = GroundClicks::of(&self.profile.general, click.mods);
            if let Some(act) = act_for_click(frame, picked, tile, click.double, ground) {
                self.hand.act(act);
            }
        }
    }

    /// The keys of the chat line, as the Speech page says: the line that
    /// has the keys takes Escape and Enter; one that has not opens on a
    /// prefix key or on Enter.
    fn chat_keys(&mut self, frame: &WatchFrame, unused: &[KeyPress], texts: &[String]) {
        let speech = &self.profile.speech;
        if self.inputs.chat_focused() {
            for press in unused {
                let out = match press.key.0.as_str() {
                    ESCAPE_KEY => self.chat.key(ChatKey::Escape, speech),
                    ENTER_KEY => self.chat.key(
                        ChatKey::Enter {
                            shift: press.mods.shift,
                        },
                        speech,
                    ),
                    _ => ChatOut::None,
                };
                if let ChatOut::Sent(words) = out {
                    let said = self
                        .chat_mode
                        .said(&words, asked_commands(frame), frame, speech);
                    match said {
                        Some(Said::Act(act)) => self.hand.act(act),
                        Some(Said::Note(words)) => self.hand.note(&words),
                        None => {}
                    }
                }
            }
        } else if !self.inputs.other_field_focused() {
            for text in texts {
                self.chat.open_on(text, speech);
            }
            if unused.iter().any(|press| press.key.0 == ENTER_KEY) {
                self.chat.enter_outside(speech);
            }
        }
    }
}

/// Black, for the death screen.
const BLACK: Rgba = Rgba::from_rgb(0, 0, 0);

/// What the map reads of one frame.
struct MapInput<'a> {
    view: Area,
    mouse: Option<Point>,
    /// Escape was pressed and fired nothing else.
    escape: bool,
    movement: &'a Movement,
}

/// The tooltip of the thing under the mouse on the map: the shard's words
/// or its name, and what a click does.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TooltipData {
    pub lines: Vec<String>,
    pub footer: &'static str,
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

    /// The data of the panels at `now`: `PanelData`.
    pub fn panels(&mut self, now: f64) -> JsValue {
        to_js(&self.panel_data(now))
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
            &event(json!({"kind": "Key", "key": "ArrowUp", "pressed": true})),
            0.0,
        );
        view.tick_native(0.0, VIEW, None);
        let walked = acts(&view.take_out_native());
        assert_eq!(walked.len(), 1);
        assert_eq!(walked[0].calls[0].tool, uoterm_world::tool_names::TOOL_WALK);
        view.input_native(
            &event(json!({"kind": "Key", "key": "ArrowUp", "pressed": false})),
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
}
