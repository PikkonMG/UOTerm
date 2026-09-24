//! The macro gump of the classic client: one macro on dark glass, with its key, a button
//! to the Macros page of the Options gump, and its steps. "Fast spell
//! assign" opens it for the macro of a spell. It edits the macro in the
//! profile through the macro editor of the actions.

use super::canvas::Canvas;
use super::macro_steps::{take_capture, StepList, STEPS_WIDTH};
use super::options::{text, Options, BUTTON_HEIGHT};
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use crate::window::actions::editor::{Capture, MacroEditor};
use crate::window::model::key_macros;
use crate::window::options_ui::{WORDS_NO_KEY, WORDS_PRESS_KEY};
use crate::window::settings::{MacroStep, Page};
use eframe::egui::Pos2;

pub const MACRO_ID: &str = "macro";

pub const MACRO: GumpKind = GumpKind {
    id: MACRO_ID,
    rules: GumpRules {
        kept: false,
        first_place: Pos2::new(250.0, 150.0),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(MacroGump::new(String::new())),
};

/// The one macro gump: a new one takes the place of the last.
pub const MACRO_GUMP: GumpId = GumpId::one(MACRO_ID);

const MARGIN: i32 = 20;
const HEIGHT: i32 = 200;
const SHADE_HUE: u16 = 0;
const SHADE_OPACITY: f32 = 0.8;
const TITLE_AT: (i32, i32) = (20, 2);
const TITLE_FONT: u8 = 1;
const TITLE_HUE: u16 = 15;
const KEY_WIDTH: i32 = 170;
const BUTTON_GAP: i32 = 3;
/// The key, "+ Create macro button" and the settings, one row each.
const BUTTON_ROWS: i32 = 3;
const STEPS_TOP: i32 = MARGIN + (BUTTON_HEIGHT + BUTTON_GAP) * BUTTON_ROWS;
const WORDS_TITLE: &str = "Edit macro:";
const WORDS_SETTINGS: &str = "Open Macro Settings";
const WORDS_CREATE_BUTTON: &str = "+ Create macro button";

/// Opens the macro gump for the macro of this name, made with `steps` when
/// the profile has none by the name.
pub fn open_for(cx: &mut GumpContext<'_>, name: &str, steps: Vec<MacroStep>) {
    if key_macros::ensure(&mut cx.profile.macros.key_bindings, name, steps) {
        cx.profile_changed();
    }
    cx.close(MACRO_GUMP);
    cx.open_with(MACRO_GUMP, Box::new(MacroGump::new(name.to_string())));
}

pub struct MacroGump {
    /// The name of the macro it edits.
    name: String,
    editor: MacroEditor,
    steps: StepList,
}

impl MacroGump {
    pub fn new(name: String) -> Self {
        Self {
            name,
            editor: MacroEditor::default(),
            steps: StepList::default(),
        }
    }
}

impl GumpBody for MacroGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let mut keys = cx.profile.macros.key_bindings.clone();
        let Some(at) = keys.iter().position(|binding| binding.name == self.name) else {
            cx.close(cx.me);
            return;
        };
        let width = STEPS_WIDTH + MARGIN * 2;
        g.shade(0, 0, width, HEIGHT, SHADE_HUE, SHADE_OPACITY);
        let title = format!("{WORDS_TITLE} {}", self.name);
        g.label(
            TITLE_AT.0,
            TITLE_AT.1,
            &title,
            &TextLook::unicode(TITLE_FONT, TITLE_HUE),
        );
        let mut changed = take_capture(&mut self.editor, g, &mut keys);
        let look = text().bordered();
        let chord = match (self.editor.capture, &keys[at].chord) {
            (Some(Capture::Chord(_)), _) => WORDS_PRESS_KEY.to_string(),
            (_, Some(chord)) => chord.to_string(),
            (_, None) => WORDS_NO_KEY.to_string(),
        };
        if g.nice_button(
            "key",
            MARGIN,
            MARGIN,
            KEY_WIDTH,
            BUTTON_HEIGHT,
            &chord,
            &look,
            false,
        ) {
            self.editor.capture(Capture::Chord(at));
        }
        let create = g.nice_button(
            "create_button",
            MARGIN,
            MARGIN + BUTTON_HEIGHT + BUTTON_GAP,
            KEY_WIDTH,
            BUTTON_HEIGHT,
            WORDS_CREATE_BUTTON,
            &look,
            false,
        );
        if let Some(mouse) = g
            .ui()
            .input(|i| i.pointer.interact_pos())
            .filter(|_| create)
        {
            super::macro_button::open_for(cx, &self.name, mouse);
        }
        if g.nice_button(
            "settings",
            MARGIN,
            MARGIN + (BUTTON_HEIGHT + BUTTON_GAP) * 2,
            KEY_WIDTH,
            BUTTON_HEIGHT,
            WORDS_SETTINGS,
            &look,
            false,
        ) {
            let options = GumpId::one(well_known::OPTIONS);
            cx.close(options);
            cx.open_with(options, Box::new(Options::at_page(Page::Macros)));
        }
        let steps = &mut self.steps;
        g.scroll_area(
            "steps",
            MARGIN,
            STEPS_TOP,
            width - MARGIN,
            HEIGHT - STEPS_TOP,
            |g| {
                let (height, stepped) = steps.draw(g, &mut keys, at, (0, 0));
                changed |= stepped;
                height
            },
        );
        if changed {
            cx.profile.macros.key_bindings = keys;
            cx.profile_changed();
        }
    }

    fn wants_keys(&self) -> bool {
        self.editor.capture.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::manager::GumpManager;
    use crate::window::settings::{KeyBinding, Profile};

    #[test]
    fn a_macro_gump_draws_the_macro_it_names_and_closes_without_one() {
        let mut profile = Profile::default();
        profile.macros.key_bindings.push(KeyBinding {
            name: "Heal".into(),
            steps: vec![MacroStep::new("cast", "Heal")],
            ..KeyBinding::default()
        });
        let mut manager = GumpManager::default();
        manager.open_body(
            MACRO_GUMP,
            Box::new(MacroGump::new("Heal".into())),
            &mut profile,
        );
        let frame = crate::view::WatchFrame::default();
        if !super::super::testing::draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert!(manager.is_open(&MACRO_GUMP));
        manager.close(&MACRO_GUMP, &mut profile);
        manager.open_body(
            MACRO_GUMP,
            Box::new(MacroGump::new("Missing".into())),
            &mut profile,
        );
        super::super::testing::draw_frames(&mut manager, &mut profile, &frame);
        assert!(!manager.is_open(&MACRO_GUMP));
    }
}
