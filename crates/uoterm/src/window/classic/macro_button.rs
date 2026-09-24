//! The macro buttons of the desktop: a
//! dark box with the name of a macro of the profile, made with "+ Create
//! macro button" of the macro gump. A double click, or one click with
//! "Single-click UI buttons", runs the macro through the macro runner of
//! the actions, as its key would. It snaps to the spell and skill buttons,
//! and the profile of the character keeps it. A button whose macro is gone
//! closes.

use super::canvas::Canvas;
use super::registry::{GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::spell_button::{pressed, BUTTON_GROUP};
use super::text::TextLook;
use crate::window::settings::KeyBinding;
use eframe::egui::{Color32, Pos2};
use uoterm_nav::TextAlign;

/// The id of the button of one macro, by the number its name makes.
const MACRO_BUTTON_ID: &str = "macro_button";

pub const MACRO_BUTTON: GumpKind = GumpKind {
    id: MACRO_BUTTON_ID,
    rules: GumpRules {
        anchor: Some(BUTTON_GROUP),
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(MacroButton::new(serial.unwrap_or_default())),
};

const SIZE: (i32, i32) = (88, 44);
const BACK: Color32 = Color32::from_rgb(30, 30, 30);
const BACK_HOVER: Color32 = Color32::from_rgb(105, 105, 105);
const BORDER: Color32 = Color32::GRAY;
const TEXT_FONT: u8 = 1;
const TEXT_HUE: u16 = 0x03B2;
const TEXT_HOVER_HUE: u16 = 53;
const TEXT_ROOM: i32 = 10;
const HALF: i32 = 2;
// The 32-bit FNV-1a hash.
const FNV_OFFSET: u32 = 0x811C_9DC5;
const FNV_PRIME: u32 = 0x0100_0193;

/// The number the button of a macro is kept under: the same for the same
/// name in every run.
pub fn macro_number(name: &str) -> u32 {
    name.bytes().fold(FNV_OFFSET, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

/// Opens the button of a macro with its middle at a window place, as
/// "+ Create macro button" does at the mouse.
pub fn open_for(cx: &mut GumpContext<'_>, name: &str, mouse: Pos2) {
    let half = eframe::egui::Vec2::new((SIZE.0 / HALF) as f32, (SIZE.1 / HALF) as f32);
    cx.open_at(
        GumpId::of(MACRO_BUTTON.id, macro_number(name)),
        mouse - half,
    );
}

pub struct MacroButton {
    number: u32,
}

impl MacroButton {
    pub fn new(number: u32) -> Self {
        Self { number }
    }

    fn binding<'p>(&self, bindings: &'p [KeyBinding]) -> Option<&'p KeyBinding> {
        bindings
            .iter()
            .find(|binding| macro_number(&binding.name) == self.number)
    }
}

impl GumpBody for MacroButton {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(binding) = self.binding(&cx.profile.macros.key_bindings).cloned() else {
            cx.close(cx.me);
            return;
        };
        let (w, h) = SIZE;
        let hovered = g.hovered(0, 0, w, h);
        g.fill(0, 0, w, h, if hovered { BACK_HOVER } else { BACK });
        g.outline(0, 0, w, h, BORDER);
        let hue = if hovered { TEXT_HOVER_HUE } else { TEXT_HUE };
        let look = TextLook::unicode(TEXT_FONT, hue)
            .wrap((w - TEXT_ROOM) as u32)
            .aligned(TextAlign::Center)
            .bordered();
        let size = g.measure(&binding.name, &look);
        g.label(0, h / HALF - size.y as i32 / HALF, &binding.name, &look);
        if pressed(g, cx) {
            cx.run_macro(binding.steps);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_macro_keeps_its_number_by_its_name() {
        assert_eq!(macro_number("Heal"), macro_number("Heal"));
        assert_ne!(macro_number("Heal"), macro_number("Cure"));
        let button = MacroButton::new(macro_number("Cure"));
        let bindings = vec![
            KeyBinding {
                name: "Heal".into(),
                ..KeyBinding::default()
            },
            KeyBinding {
                name: "Cure".into(),
                ..KeyBinding::default()
            },
        ];
        assert_eq!(
            button.binding(&bindings).map(|b| b.name.as_str()),
            Some("Cure")
        );
    }
}
