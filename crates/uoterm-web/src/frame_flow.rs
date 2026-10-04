//! One frame of the view, in the order of the frame of the Rust window:
//! the controls, the map, the shard reports, the sky, the floats, then the
//! hotbar keys, the chat line and the clicks on the map. Every decision is
//! a rule of `uoterm_view`; here the rules meet the input of the page and
//! give what to draw and the calls for the page.

use crate::buffers::{DrawBuffers, MeshArrays, PlacedWords, Shapes};
use crate::input::FrameInput;
use crate::out::{Hand, OutCall};
use crate::{kept, TooltipData, WebView};
use uoterm_view::act::Act;
use uoterm_view::actions::controls::{ControlHost, FrameIn};
use uoterm_view::actions::{LocalAim, WindowCommand};
use uoterm_view::art::{hue_color, ArtRequest, ItemPaint, TextLook, WorldArt};
use uoterm_view::clicks::{act_for_click, escape_on_map, EscapeOnMap, GroundClicks};
use uoterm_view::floats::{self, SPEECH_LINE};
use uoterm_view::frame::WatchFrame;
use uoterm_view::geom::{Area, Point, Rgba, Vector};
use uoterm_view::input::KeyPress;
use uoterm_view::keys::chat::{LineKey, Said};
use uoterm_view::keys::Focus;
use uoterm_view::model::asked::asked_commands;
use uoterm_view::model::counters::slot_act;
use uoterm_view::scene::plates::lay_out;
use uoterm_view::scene::{
    overlays, SceneDraw, SceneInput, SceneState, DEATH_FONT, DEATH_HUE, DEATH_WORDS,
};
use uoterm_view::settings::{CombatOptions, Profile, UiStyle};
use uoterm_view::sky::{
    drop_color, effect_area, lightning_bolt, lit_effects, shown_effects, storm_tint, weather_drops,
    Drop, ShownEffect, LIGHTNING_WIDTH, RAIN_WIDTH, SNOW_RADIUS,
};
use uoterm_view::steer::{Movement, SteerInput};
use uoterm_view::ui::deck::hotbar_key_slot;

const ESCAPE_KEY: &str = "Escape";
const ENTER_KEY: &str = "Enter";

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

impl WebView {
    /// One frame with a picture of the session, in the order of the frame
    /// of the Rust window: the controls, the map, the shard reports, the
    /// sky, the floats, then the clicks on the map and the chat line.
    pub(crate) fn run_frame(
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
            self.style_command(frame, command);
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
                self.chat_keys(frame, focus, &unused, texts);
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
        self.art.end_frame();
        buffers.world = MeshArrays::from(world.into_mesh());
        buffers.overlay = MeshArrays::from(overlay.into_mesh());
        buffers.uploads = self.art.take_uploads();
        buffers.atlas_reset = self.art.take_atlas_reset();
        buffers
    }

    /// Does a command of the windows of the Modern style. The chat line
    /// and the counter bar are the view's; the page does the others.
    fn style_command(&mut self, frame: &WatchFrame, command: WindowCommand) {
        match command {
            WindowCommand::ToggleChat => self.chat.toggle_hidden(),
            WindowCommand::UseCounterSlot(slot) => {
                let act = slot_act(frame, &self.profile.counters, slot);
                if let Some(act) = act.filter(|_| frame.human_control) {
                    self.hand.act(act);
                }
            }
            command => self.hand.push(OutCall::Window { command }),
        }
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
            // The building goes where the click came up.
            if let Some(click) = input.clicks.iter().find(|click| on_map(&click.at)) {
                let (x, y, z) = self.scene.tile_at(&mut self.art, view, frame, click.at);
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

    /// The keys of the chat line, by the field that has them, and the
    /// lines it sends.
    fn chat_keys(
        &mut self,
        frame: &WatchFrame,
        focus: Focus,
        unused: &[KeyPress],
        texts: &[String],
    ) {
        let typed = texts.iter().cloned().map(LineKey::Typed);
        let pressed = unused
            .iter()
            .filter_map(|press| match press.key.0.as_str() {
                ESCAPE_KEY => Some(LineKey::Escape),
                ENTER_KEY => Some(LineKey::Enter {
                    shift: press.mods.shift,
                }),
                _ => None,
            });
        let keys: Vec<LineKey> = typed.chain(pressed).collect();
        let speech = &self.profile.speech;
        let out = self.chat.take_keys(focus, &keys, speech);
        for words in out.sent {
            match self
                .chat_mode
                .said(&words, asked_commands(frame), frame, speech)
            {
                Some(Said::Act(act)) => self.hand.act(act),
                Some(Said::Note(words)) => self.hand.note(&words),
                None => {}
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
