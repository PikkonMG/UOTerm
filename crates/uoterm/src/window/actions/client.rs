//! The controls of the play window, the same in every UI style: the keys,
//! the controller, the macro that runs, and the window commands that are
//! not a style's own (options, zoom, the view range, the selection,
//! screenshots). The commands of a style's windows come back to the window,
//! which gives them to the style that is active.

use super::guard::aim_words;
use super::journal::ClientJournal;
use super::resolve::{opens_doors, runs, Context, Effect};
use super::runner::MacroRunner;
use super::screenshot::Screenshots;
use super::select::select;
use super::switches::{set_switch, switch_on, switched_words};
use super::view_range::ViewRange;
use super::{Look, PointerClick, Switch, WindowCommand, ZoomStep};
use crate::view::WatchFrame;
use crate::window::bridge;
use crate::window::control::{Act, Hand};
use crate::window::keys::{self, KeyDispatch, WarKey};
use crate::window::model::skills::SkillChanges;
use crate::window::model::status::StatChanges;
use crate::window::pad::{self, Pad};
use crate::window::scene::Scene;
use crate::window::settings::{AuraRule, MacroStep, Profile, ProfileHome};
use crate::window::steer::Movement;
use eframe::egui::{self, Event, Id, PointerButton, Rect, ViewportCommand};
use uoterm_view::actions::screenshot::{failed_words, stored_words, DeathWatch};

/// One step of the zoom keys.
const ZOOM_STEP: f32 = 0.1;
/// The camera looks at most this far from the character, in points.
const PEEK_MOST: f32 = 240.0;

const NOTE_SAVED: &str = "The desktop is saved.";
const NOTE_NOTHING_TO_SELECT: &str = "There is nothing of that kind to select.";

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

/// What the controls of one frame ask of the style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameOut {
    /// The commands for the style's own windows.
    pub style: Vec<WindowCommand>,
    /// Ctrl+Q (true) or Ctrl+W (false) in the chat line.
    pub history_older: Option<bool>,
    pub movement: Movement,
}

#[derive(Default)]
pub struct Controls {
    keys: KeyDispatch,
    pad: Pad,
    runner: MacroRunner,
    pub journal: ClientJournal,
    screenshots: Screenshots,
    deaths: DeathWatch,
    view_range: ViewRange,
    /// The target the select actions picked.
    selected: Option<u32>,
    /// The aura rule before the aura key turned auras off.
    aura_before: Option<AuraRule>,
    /// Clicks a controller or a key asked for, for the next frame's input.
    clicks: Vec<PointerClick>,
    /// Macros the controller started, for the next frame.
    pad_macros: Vec<Vec<MacroStep>>,
    pad_walk: Option<(&'static str, bool)>,
    /// The skills of the last frame, to tell of the ones that changed.
    skill_changes: SkillChanges,
    stat_changes: StatChanges,
    /// The journal told why the controller does nothing.
    pad_note_told: bool,
}

impl Controls {
    /// Lays the window's own lines into a new picture and takes out what
    /// lies past the view range. Call it for each picture the session sends.
    pub fn received(&mut self, frame: &mut WatchFrame) {
        super::received(frame, &mut self.journal, &self.view_range);
    }

    /// Reads the controller and puts the clicks and the mouse moves it asks
    /// for into the input of the next frame. The window calls it from its
    /// raw input hook.
    pub fn raw_input(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput, profile: &Profile) {
        let pad = self.pad.poll(profile, raw.predicted_dt);
        pad::leave_pressed(ctx, pad.pressed.clone());
        self.pad_macros.extend(pad.macros);
        self.pad_walk = pad.walk;
        let last = ctx.input(|input| input.pointer.latest_pos());
        let mut at = last.unwrap_or_else(|| ctx.screen_rect().center());
        let pointer = bridge::vec2(pad.pointer);
        if pointer != egui::Vec2::ZERO {
            let room = raw.screen_rect.unwrap_or_else(|| ctx.screen_rect());
            at = room.clamp(at + pointer);
            raw.events.push(Event::PointerMoved(at));
            ctx.send_viewport_cmd(ViewportCommand::CursorPosition(at));
        }
        for click in self.clicks.drain(..) {
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
        self.runner.start(steps);
    }

    /// Runs the keys, the controller and the macro of one frame.
    pub fn frame(&mut self, env: &mut FrameEnv<'_>) -> FrameOut {
        let mut out = FrameOut::default();
        env.hand.watch_over(env.frame, &env.profile.combat);
        let focus = keys::focus(env.ctx, env.chat_id, env.chat_empty);
        let presses = if env.keys_paused {
            Vec::new()
        } else {
            keys::presses(env.ctx)
        };
        let dispatched = self
            .keys
            .dispatch(&presses, focus, env.profile, env.frame.war);
        keys::take_used(env.ctx, &dispatched);
        out.history_older = dispatched.history_older;
        match dispatched.war {
            Some(WarKey::Set(on)) => env.hand.act(Act::War(on)),
            Some(WarKey::Toggle) => env.hand.act(Act::War(!env.frame.war)),
            None => {}
        }
        for steps in dispatched
            .macros
            .into_iter()
            .chain(self.pad_macros.drain(..))
        {
            self.runner.start(steps);
        }
        self.look(env);
        out.movement = Movement {
            keys: keys::walk_keys(focus, env.profile),
            held: self.keys.held_walk(),
            pad: self.pad_walk,
            always_run: runs(env.frame, env.profile),
            auto_move: !env.profile.experimental.disable_click_automove,
            open_doors: opens_doors(env.frame, env.profile),
        };
        let effects = self.runner.tick(
            env.time,
            &Context {
                frame: env.frame,
                profile: env.profile,
                selected: self.selected,
            },
        );
        for effect in effects {
            match effect {
                Effect::Act(act) => env.hand.act(act),
                Effect::Note(words) => self.journal.print(env.frame, words),
                Effect::Window(command) => out.style.extend(self.command(command, env)),
                // The runner keeps the waits.
                Effect::Wait(_) => {}
            }
        }
        let general = &env.profile.general;
        let changes = self.skill_changes.observe(env.frame, general);
        for words in changes
            .into_iter()
            .chain(self.stat_changes.observe(env.frame, general))
        {
            self.journal.print(env.frame, words);
        }
        for words in env.hand.take_notes() {
            self.journal.print(env.frame, words);
        }
        if !self.pad_note_told && !self.pad.note().is_empty() {
            self.pad_note_told = true;
            self.journal.print(env.frame, self.pad.note().to_string());
        }
        if self.deaths.died(env.frame) && env.profile.interface.screenshot_on_death {
            self.screenshots.ask(env.ctx);
        }
        self.take_screenshot(env);
        out
    }

    /// Moves the camera while a look key is held.
    fn look(&self, env: &mut FrameEnv<'_>) {
        let mouse = env.ctx.input(|input| input.pointer.hover_pos());
        let peek = match (self.keys.held_look(), mouse) {
            (Some(look), Some(mouse)) => {
                let toward = mouse - env.view.center();
                let toward = if toward.length() > PEEK_MOST {
                    toward.normalized() * PEEK_MOST
                } else {
                    toward
                };
                match look {
                    Look::Forwards => toward,
                    Look::Backwards => -toward,
                }
            }
            _ => egui::Vec2::ZERO,
        };
        env.scene.set_peek(peek);
    }

    fn take_screenshot(&mut self, env: &FrameEnv<'_>) {
        let Some(saved) = self.screenshots.take(env.ctx) else {
            return;
        };
        match saved {
            Ok(path) if !env.profile.general.hide_screenshot_message => {
                let place = path.display().to_string();
                self.journal.print(env.frame, stored_words(&place));
            }
            Ok(_) => {}
            Err(why) => self.journal.print(env.frame, failed_words(&why)),
        }
    }

    /// Does a window command that is the same in every style. Gives back a
    /// command of the style's own windows.
    pub fn command(
        &mut self,
        command: WindowCommand,
        env: &mut FrameEnv<'_>,
    ) -> Option<WindowCommand> {
        if command.is_for_style() {
            return Some(command);
        }
        let frame = env.frame;
        match command {
            WindowCommand::ToggleOption(switch) => {
                let on = !switch_on(env.profile, switch);
                self.switch(switch, on, env);
            }
            WindowCommand::SetOption(switch, on) => self.switch(switch, on, env),
            WindowCommand::Zoom(step) => {
                let zoom = match step {
                    ZoomStep::Default => env.profile.video.default_zoom,
                    ZoomStep::In => env.scene.zoom() + ZOOM_STEP,
                    ZoomStep::Out => env.scene.zoom() - ZOOM_STEP,
                };
                env.scene.set_zoom(zoom);
            }
            WindowCommand::Screenshot => self.screenshots.ask(env.ctx),
            WindowCommand::SaveDesktop => {
                env.home.save(env.profile);
                self.journal.print(frame, NOTE_SAVED);
            }
            WindowCommand::ViewRange(change) => {
                let tiles = self.view_range.change(change);
                self.journal
                    .print(frame, format!("The view range is now {tiles}."));
            }
            WindowCommand::Select(how, kind) => match select(frame, how, kind, self.selected) {
                Some((serial, name)) => {
                    self.selected = Some(serial);
                    self.journal.print(frame, format!("Target: {name}"));
                }
                None => self.journal.print(frame, NOTE_NOTHING_TO_SELECT),
            },
            WindowCommand::Aim(aim) => {
                env.hand.aim(aim);
                self.journal.print(frame, aim_words(aim));
            }
            WindowCommand::Click(click) => self.clicks.push(click),
            // `is_for_style` gave these back above.
            WindowCommand::Gump(..)
            | WindowCommand::CloseAllGumps
            | WindowCommand::CloseCorpses
            | WindowCommand::CloseHealthBars { .. }
            | WindowCommand::UseCounterSlot(_)
            | WindowCommand::ToggleChat
            | WindowCommand::PasteToChat
            | WindowCommand::QuitGame => {}
        }
        None
    }

    fn switch(&mut self, switch: Switch, on: bool, env: &mut FrameEnv<'_>) {
        set_switch(env.profile, switch, on, &mut self.aura_before);
        env.home.save(env.profile);
        self.journal.print(env.frame, switched_words(switch, on));
    }

    /// Tells the player that the style has no window for a command.
    pub fn style_cannot(&mut self, frame: &WatchFrame, command: &WindowCommand) {
        super::style_cannot(&mut self.journal, frame, command);
    }
}
