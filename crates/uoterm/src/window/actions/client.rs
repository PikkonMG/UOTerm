//! The controls of the play window. Their rules are
//! `uoterm_view::actions::controls`, which the window and the web client
//! share; here the window reads the keys egui reports and the controller
//! gilrs reads, makes the clicks a controller asks for, and saves the
//! screenshots.

use super::screenshot::Screenshots;
use super::{LocalAim, PointerClick, WindowCommand};
use crate::view::WatchFrame;
use crate::window::bridge;
use crate::window::control::{Act, Hand};
use crate::window::keys;
use crate::window::pad::{self, Pad};
use crate::window::scene::Scene;
use crate::window::settings::{CombatOptions, MacroStep, Profile, ProfileHome};
use eframe::egui::{self, Event, Id, PointerButton, Rect, ViewportCommand};
use uoterm_view::actions::controls::{self, ControlHost, FrameIn};
use uoterm_view::geom::Vector;

pub use uoterm_view::actions::controls::FrameOut;

/// What the window needs to run the controls of one frame.
pub struct FrameEnv<'a> {
    pub ctx: &'a egui::Context,
    pub frame: &'a WatchFrame,
    pub profile: &'a mut Profile,
    pub home: &'a ProfileHome,
    pub hand: &'a Hand,
    pub scene: &'a mut Scene,
    /// Where the world is drawn.
    pub view: Rect,
    pub time: f64,
    /// The chat line of the style: its id, and whether it holds no words.
    pub chat_id: Id,
    pub chat_empty: bool,
    /// The Options screen waits for a key: the keys run nothing.
    pub keys_paused: bool,
}

/// The window as the shared controls see it for one frame.
struct WindowHost<'a> {
    ctx: &'a egui::Context,
    hand: &'a Hand,
    scene: &'a mut Scene,
    home: &'a ProfileHome,
    screenshots: &'a mut Screenshots,
}

impl ControlHost for WindowHost<'_> {
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
        self.scene.set_peek(bridge::vec2(peek));
    }

    fn ask_screenshot(&mut self) {
        self.screenshots.ask(self.ctx);
    }

    fn save_profile(&mut self, profile: &Profile) {
        self.home.save(profile);
    }
}

#[derive(Default)]
pub struct Controls {
    shared: controls::Controls,
    pad: Pad,
    screenshots: Screenshots,
}

impl Controls {
    /// Lays the window's own lines into a new picture and takes out what
    /// lies past the view range. Call it for each picture the session sends.
    pub fn received(&mut self, frame: &mut WatchFrame) {
        self.shared.received(frame);
    }

    /// Reads the controller and puts the clicks and the mouse moves it asks
    /// for into the input of the next frame. The window calls it from its
    /// raw input hook.
    pub fn raw_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput, profile: &Profile) {
        let pad = self
            .shared
            .take_pad(self.pad.poll(profile, raw.predicted_dt));
        pad::leave_pressed(ctx, pad.pressed);
        let last = ctx.input(|input| input.pointer.latest_pos());
        let mut at = last.unwrap_or_else(|| ctx.screen_rect().center());
        let pointer = bridge::vec2(pad.pointer);
        if pointer != egui::Vec2::ZERO {
            let room = raw.screen_rect.unwrap_or_else(|| ctx.screen_rect());
            at = room.clamp(at + pointer);
            raw.events.push(Event::PointerMoved(at));
            ctx.send_viewport_cmd(ViewportCommand::CursorPosition(at));
        }
        for click in self.shared.take_clicks() {
            let (button, times) = match click {
                PointerClick::Left => (PointerButton::Primary, 1),
                PointerClick::Right => (PointerButton::Secondary, 1),
                PointerClick::Double => (PointerButton::Primary, 2),
            };
            for _ in 0..times {
                for pressed in [true, false] {
                    raw.events.push(Event::PointerButton {
                        pos: at,
                        button,
                        pressed,
                        modifiers: raw.modifiers,
                    });
                }
            }
        }
    }

    /// Starts a macro a button of the window runs. It runs from the next
    /// frame on, as the macro of a key does.
    pub fn run_macro(&mut self, steps: Vec<MacroStep>) {
        self.shared.run_macro(steps);
    }

    /// Runs the keys, the controller and the macro of one frame.
    pub fn frame(&mut self, env: &mut FrameEnv<'_>) -> FrameOut {
        let focus = keys::focus(env.ctx, env.chat_id, env.chat_empty);
        let presses = if env.keys_paused {
            Vec::new()
        } else {
            keys::presses(env.ctx)
        };
        let input = FrameIn {
            frame: env.frame,
            profile: env.profile,
            time: env.time,
            presses: &presses,
            focus,
            mouse: env
                .ctx
                .input(|input| input.pointer.hover_pos())
                .map(bridge::point),
            view: bridge::area(env.view),
            pad_note: self.pad.note(),
        };
        let mut host = WindowHost {
            ctx: env.ctx,
            hand: env.hand,
            scene: env.scene,
            home: env.home,
            screenshots: &mut self.screenshots,
        };
        let out = self.shared.frame(input, &mut host);
        keys::take_used(env.ctx, &out.used);
        if let Some(saved) = self.screenshots.take(env.ctx) {
            let saved = saved.map(|path| path.display().to_string());
            self.shared.screenshot_taken(env.frame, env.profile, saved);
        }
        out
    }

    /// Tells the player that the style has no window for a command.
    pub fn style_cannot(&mut self, frame: &WatchFrame, command: &WindowCommand) {
        self.shared.style_cannot(frame, command);
    }
}
