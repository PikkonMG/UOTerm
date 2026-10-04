//! The controls of a play window, the same in every UI style and in each
//! client: the keys, the controller, the macro that runs, and the window
//! commands that are not a style's own (options, zoom, the view range, the
//! selection, screenshots). The commands of a style's windows go back to
//! the caller, which gives them to the style that is active.
//!
//! Each client reads its own keys and controller and gives them here once
//! in each frame. What the controls ask of the client (an act, the zoom,
//! a screenshot, the kept profile) goes through its [`ControlHost`].

use super::journal::ClientJournal;
use super::resolve::{opens_doors, runs, Context, Effect};
use super::runner::MacroRunner;
use super::screenshot::{failed_words, stored_words, DeathWatch};
use super::select::select;
use super::switches::{set_switch, switch_on, switched_words};
use super::view_range::ViewRange;
use super::{LocalAim, Look, PointerClick, Switch, WindowCommand, ZoomStep};
use crate::act::Act;
use crate::frame::WatchFrame;
use crate::geom::{Area, Point, Vector};
use crate::guard::aim_words;
use crate::input::{KeyName, KeyPress, Mods};
use crate::keys::{walk_keys, Focus, KeyDispatch, WarKey};
use crate::model::skills::SkillChanges;
use crate::model::status::StatChanges;
use crate::pad::PadFrame;
use crate::settings::{AuraRule, CombatOptions, MacroStep, Profile};
use crate::steer::Movement;

/// One step of the zoom keys.
pub const ZOOM_STEP: f32 = 0.1;
/// The camera looks at most this far from the character, in points.
pub const PEEK_MOST: f32 = 240.0;

pub const NOTE_SAVED: &str = "The desktop is saved.";
pub const NOTE_NOTHING_TO_SELECT: &str = "There is nothing of that kind to select.";

/// What the controls ask of the client that runs them.
pub trait ControlHost {
    /// Sends one act of the character, after the guard checked it.
    fn act(&mut self, act: Act);
    /// The next click on a thing does this in place of its usual act.
    fn aim(&mut self, aim: LocalAim);
    /// Gives the guard the newest picture and the combat options.
    fn watch_over(&mut self, frame: &WatchFrame, combat: &CombatOptions);
    /// The words of the client for the journal since the last frame.
    fn take_notes(&mut self) -> Vec<String>;
    fn zoom(&self) -> f32;
    /// Sets the zoom of the map.
    fn set_zoom(&mut self, zoom: f32);
    /// Moves the camera this far from the character, in points.
    fn set_peek(&mut self, peek: Vector);
    /// Asks for a picture of the window. It comes in a later frame, and the
    /// client gives it to [`Controls::screenshot_taken`].
    fn ask_screenshot(&mut self);
    /// Keeps the profile where the client keeps it.
    fn save_profile(&mut self, profile: &Profile);
}

/// What the client read for the controls of one frame.
pub struct FrameIn<'a> {
    pub frame: &'a WatchFrame,
    pub profile: &'a mut Profile,
    /// The clock of the client, in seconds.
    pub time: f64,
    /// The key presses of this frame. None go here while the Options
    /// screen waits for a key.
    pub presses: &'a [KeyPress],
    /// Where the keys go now.
    pub focus: Focus,
    /// Where the mouse is, when it is over the window.
    pub mouse: Option<Point>,
    /// Where the world is drawn.
    pub view: Area,
    /// Why the controller does nothing, or nothing.
    pub pad_note: &'a str,
}

/// What the controls of one frame ask of the style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameOut {
    /// The commands for the style's own windows.
    pub style: Vec<WindowCommand>,
    /// Ctrl+Q (true) or Ctrl+W (false) in the chat line.
    pub history_older: Option<bool>,
    pub movement: Movement,
    /// The keys that fired something. No field and no other part of the
    /// window may act on them.
    pub used: Vec<(Mods, KeyName)>,
}

#[derive(Default)]
pub struct Controls {
    keys: KeyDispatch,
    runner: MacroRunner,
    pub journal: ClientJournal,
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

    /// Takes the macros and the walk of one read of the controller, for
    /// the next frame. Gives back the read with its macros taken out.
    pub fn take_pad(&mut self, mut pad: PadFrame) -> PadFrame {
        self.pad_macros.append(&mut pad.macros);
        self.pad_walk = pad.walk;
        pad
    }

    /// The clicks a controller or a key asked for since the last call, for
    /// the client to make at the mouse.
    pub fn take_clicks(&mut self) -> Vec<PointerClick> {
        std::mem::take(&mut self.clicks)
    }

    /// Starts a macro a button of the window runs. It runs from the next
    /// frame on, as the macro of a key does.
    pub fn run_macro(&mut self, steps: Vec<MacroStep>) {
        self.runner.start(steps);
    }

    /// Runs the keys, the controller and the macro of one frame.
    pub fn frame(&mut self, input: FrameIn<'_>, host: &mut dyn ControlHost) -> FrameOut {
        let FrameIn {
            frame,
            profile,
            time,
            presses,
            focus,
            mouse,
            view,
            pad_note,
        } = input;
        let mut out = FrameOut::default();
        host.watch_over(frame, &profile.combat);
        let dispatched = self.keys.dispatch(presses, focus, profile, frame.war);
        out.used = dispatched.used;
        out.history_older = dispatched.history_older;
        match dispatched.war {
            Some(WarKey::Set(on)) => host.act(Act::War(on)),
            Some(WarKey::Toggle) => host.act(Act::War(!frame.war)),
            None => {}
        }
        for steps in dispatched
            .macros
            .into_iter()
            .chain(self.pad_macros.drain(..))
        {
            self.runner.start(steps);
        }
        host.set_peek(peek(self.keys.held_look(), mouse, view));
        out.movement = Movement {
            keys: walk_keys(focus, profile),
            held: self.keys.held_walk(),
            pad: self.pad_walk,
            always_run: runs(frame, profile),
            auto_move: !profile.experimental.disable_click_automove,
            open_doors: opens_doors(frame, profile),
        };
        let effects = self.runner.tick(
            time,
            &Context {
                frame,
                profile,
                selected: self.selected,
            },
        );
        for effect in effects {
            match effect {
                Effect::Act(act) => host.act(act),
                Effect::Note(words) => self.journal.print(frame, words),
                Effect::Window(command) => {
                    out.style
                        .extend(self.command(command, frame, profile, host));
                }
                // The runner keeps the waits.
                Effect::Wait(_) => {}
            }
        }
        let general = &profile.general;
        let changes = self.skill_changes.observe(frame, general);
        for words in changes
            .into_iter()
            .chain(self.stat_changes.observe(frame, general))
        {
            self.journal.print(frame, words);
        }
        for words in host.take_notes() {
            self.journal.print(frame, words);
        }
        if !self.pad_note_told && !pad_note.is_empty() {
            self.pad_note_told = true;
            self.journal.print(frame, pad_note.to_string());
        }
        if self.deaths.died(frame) && profile.interface.screenshot_on_death {
            host.ask_screenshot();
        }
        out
    }

    /// Tells the player where a screenshot went, or why it did not, as the
    /// General page says. `saved` is the place of the file, or the words of
    /// the fault.
    pub fn screenshot_taken(
        &mut self,
        frame: &WatchFrame,
        profile: &Profile,
        saved: Result<String, String>,
    ) {
        match saved {
            Ok(place) if !profile.general.hide_screenshot_message => {
                self.journal.print(frame, stored_words(&place));
            }
            Ok(_) => {}
            Err(why) => self.journal.print(frame, failed_words(&why)),
        }
    }

    /// Does a window command that is the same in every style. Gives back a
    /// command of the style's own windows.
    pub fn command(
        &mut self,
        command: WindowCommand,
        frame: &WatchFrame,
        profile: &mut Profile,
        host: &mut dyn ControlHost,
    ) -> Option<WindowCommand> {
        if command.is_for_style() {
            return Some(command);
        }
        match command {
            WindowCommand::ToggleOption(switch) => {
                let on = !switch_on(profile, switch);
                self.switch(switch, on, frame, profile, host);
            }
            WindowCommand::SetOption(switch, on) => self.switch(switch, on, frame, profile, host),
            WindowCommand::Zoom(step) => {
                let zoom = match step {
                    ZoomStep::Default => profile.video.default_zoom,
                    ZoomStep::In => host.zoom() + ZOOM_STEP,
                    ZoomStep::Out => host.zoom() - ZOOM_STEP,
                };
                host.set_zoom(zoom);
            }
            WindowCommand::Screenshot => host.ask_screenshot(),
            WindowCommand::SaveDesktop => {
                host.save_profile(profile);
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
                host.aim(aim);
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

    fn switch(
        &mut self,
        switch: Switch,
        on: bool,
        frame: &WatchFrame,
        profile: &mut Profile,
        host: &mut dyn ControlHost,
    ) {
        set_switch(profile, switch, on, &mut self.aura_before);
        host.save_profile(profile);
        self.journal.print(frame, switched_words(switch, on));
    }

    /// Tells the player that the style has no window for a command.
    pub fn style_cannot(&mut self, frame: &WatchFrame, command: &WindowCommand) {
        super::style_cannot(&mut self.journal, frame, command);
    }
}

/// How far the camera looks from the character while a look key is held:
/// toward the mouse, or away from it, at most [`PEEK_MOST`].
pub fn peek(look: Option<Look>, mouse: Option<Point>, view: Area) -> Vector {
    let (Some(look), Some(mouse)) = (look, mouse) else {
        return Vector::ZERO;
    };
    let toward = mouse - view.center();
    let toward = if toward.length() > PEEK_MOST {
        toward / toward.length() * PEEK_MOST
    } else {
        toward
    };
    match look {
        Look::Forwards => toward,
        Look::Backwards => toward * -1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{KeyBinding, KeyChord};

    const VIEW: Area = Area {
        min: Point { x: 0.0, y: 0.0 },
        max: Point { x: 800.0, y: 600.0 },
    };

    /// A client that keeps what the controls asked of it.
    #[derive(Default)]
    struct Kept {
        acts: Vec<Act>,
        aims: Vec<LocalAim>,
        zoom: f32,
        peek: Vector,
        screenshots: usize,
        saved: usize,
        notes: Vec<String>,
    }

    impl ControlHost for Kept {
        fn act(&mut self, act: Act) {
            self.acts.push(act);
        }
        fn aim(&mut self, aim: LocalAim) {
            self.aims.push(aim);
        }
        fn watch_over(&mut self, _: &WatchFrame, _: &CombatOptions) {}
        fn take_notes(&mut self) -> Vec<String> {
            std::mem::take(&mut self.notes)
        }
        fn zoom(&self) -> f32 {
            self.zoom
        }
        fn set_zoom(&mut self, zoom: f32) {
            self.zoom = zoom;
        }
        fn set_peek(&mut self, peek: Vector) {
            self.peek = peek;
        }
        fn ask_screenshot(&mut self) {
            self.screenshots += 1;
        }
        fn save_profile(&mut self, _: &Profile) {
            self.saved += 1;
        }
    }

    fn press(name: &str) -> KeyPress {
        KeyPress {
            key: KeyName(name.into()),
            pressed: true,
            ..KeyPress::default()
        }
    }

    fn run(
        controls: &mut Controls,
        frame: &WatchFrame,
        profile: &mut Profile,
        presses: &[KeyPress],
        host: &mut Kept,
    ) -> FrameOut {
        let input = FrameIn {
            frame,
            profile,
            time: 0.0,
            presses,
            focus: Focus::Free,
            mouse: None,
            view: VIEW,
            pad_note: "",
        };
        controls.frame(input, host)
    }

    #[test]
    fn a_bound_key_runs_its_macro_and_is_used() {
        let mut profile = Profile::default();
        profile.macros.key_bindings.push(KeyBinding {
            name: "zoom in".into(),
            chord: Some("F5".parse::<KeyChord>().unwrap()),
            pad: None,
            steps: vec![MacroStep::new("zoom", "Zoom in")],
        });
        let mut host = Kept {
            zoom: 1.0,
            ..Kept::default()
        };
        let mut controls = Controls::default();
        let frame = WatchFrame::default();
        let out = run(
            &mut controls,
            &frame,
            &mut profile,
            &[press("F5")],
            &mut host,
        );
        assert_eq!(out.used.len(), 1);
        assert_eq!(host.zoom, 1.0 + ZOOM_STEP);
    }

    #[test]
    fn holding_tab_is_war_and_a_style_command_goes_back() {
        let mut profile = Profile::default();
        let mut host = Kept::default();
        let mut controls = Controls::default();
        let frame = WatchFrame::default();
        run(
            &mut controls,
            &frame,
            &mut profile,
            &[press("Tab")],
            &mut host,
        );
        assert_eq!(host.acts, vec![Act::War(true)]);
        let back = controls.command(WindowCommand::CloseCorpses, &frame, &mut profile, &mut host);
        assert_eq!(back, Some(WindowCommand::CloseCorpses));
        let shot = controls.command(WindowCommand::Screenshot, &frame, &mut profile, &mut host);
        assert_eq!((shot, host.screenshots), (None, 1));
    }

    #[test]
    fn the_notes_of_the_host_and_a_screenshot_go_in_the_journal() {
        let mut profile = Profile::default();
        let mut host = Kept {
            notes: vec!["You are not in a party.".into()],
            ..Kept::default()
        };
        let mut controls = Controls::default();
        let frame = WatchFrame::default();
        run(&mut controls, &frame, &mut profile, &[], &mut host);
        controls.screenshot_taken(&frame, &profile, Ok("shot.png".into()));
        let mut next = WatchFrame::default();
        controls.received(&mut next);
        assert_eq!(
            next.journal,
            [
                "You are not in a party.".to_string(),
                stored_words("shot.png")
            ]
        );
    }

    #[test]
    fn a_look_key_turns_the_camera_toward_the_mouse_at_most_so_far() {
        let center = VIEW.center();
        let near = center + Vector::new(30.0, 0.0);
        assert_eq!(
            peek(Some(Look::Forwards), Some(near), VIEW),
            Vector::new(30.0, 0.0)
        );
        assert_eq!(
            peek(Some(Look::Backwards), Some(near), VIEW),
            Vector::new(-30.0, 0.0)
        );
        let far = center + Vector::new(0.0, PEEK_MOST * 2.0);
        assert_eq!(
            peek(Some(Look::Forwards), Some(far), VIEW),
            Vector::new(0.0, PEEK_MOST)
        );
        assert_eq!(peek(None, Some(far), VIEW), Vector::ZERO);
    }

    #[test]
    fn a_controller_click_waits_for_the_client() {
        let mut profile = Profile::default();
        let mut host = Kept::default();
        let mut controls = Controls::default();
        let frame = WatchFrame::default();
        let command = WindowCommand::Click(PointerClick::Double);
        assert_eq!(
            controls.command(command, &frame, &mut profile, &mut host),
            None
        );
        assert_eq!(controls.take_clicks(), vec![PointerClick::Double]);
        assert!(controls.take_clicks().is_empty());
    }
}
