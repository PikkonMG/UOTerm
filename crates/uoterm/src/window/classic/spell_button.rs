//! The spell buttons of the desktop: an
//! icon dragged out of a spellbook stays on the desktop and casts its spell
//! with a double click, or one click with "Single-click UI buttons". Its
//! icon takes the hue of a spell that stays on. With "Fast spell assign",
//! Ctrl+Alt and a click make a macro of the spell. The buttons of spells,
//! skills and macros snap together, and the profile of the character keeps
//! them where they were left.

use super::canvas::Canvas;
use super::registry::{GumpBody, GumpContext, GumpKind, GumpRules};
use super::spellbook::assign_macro;
use crate::window::control::Act;
use crate::window::model::spell_data::{
    book_name, book_spell, button_tooltip, is_active, ACTIVE_SPELL_HUE,
};
use eframe::egui::Vec2;

/// The id of the button of one spell, by the number of the spell.
const SPELL_BUTTON_ID: &str = "spell_button";
/// The anchor group of the spell, skill and macro buttons.
pub const BUTTON_GROUP: &str = "use_buttons";

pub const SPELL_BUTTON: GumpKind = GumpKind {
    id: SPELL_BUTTON_ID,
    rules: GumpRules {
        anchor: Some(BUTTON_GROUP),
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(SpellButton::new(serial.unwrap_or_default())),
};

/// The mark of "Fast spell assign" with Ctrl+Alt down.
const ASSIGN_MARK: u16 = 0x1086;
const ASSIGN_HOVER_HUE: u16 = 34;

/// True when a button takes a click that did not drag it, by the option
/// "Single-click UI buttons": one click, or else a double click.
pub fn pressed(g: &Canvas<'_>, cx: &GumpContext<'_>) -> bool {
    let alt = g.ui().input(|i| i.modifiers.alt);
    if alt {
        return false;
    }
    if cx.profile.combat.single_click_buttons {
        g.body_click().is_some()
    } else {
        g.body_double_click()
    }
}

pub struct SpellButton {
    spell: u16,
}

impl SpellButton {
    pub fn new(serial: u32) -> Self {
        Self {
            spell: u16::try_from(serial).unwrap_or_default(),
        }
    }
}

impl GumpBody for SpellButton {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some((_, spell)) = book_spell(self.spell) else {
            return;
        };
        let hue = if is_active(cx.frame, self.spell) {
            ACTIVE_SPELL_HUE
        } else {
            0
        };
        let size = g.pic(0, 0, spell.small_icon, hue);
        let name = book_name(self.spell);
        let tip = button_tooltip(self.spell)
            .map_or_else(|| name.clone(), |cliloc| g.words(cliloc, &name));
        g.tooltip(&tip);
        let assigning = cx.profile.combat.fast_spell_assign
            && g.ui().input(|i| i.modifiers.ctrl && i.modifiers.alt);
        if assigning {
            let mark = g.gump_size(ASSIGN_MARK).unwrap_or(Vec2::ZERO);
            let hovered = g.hovered(0, 0, size.x as i32, size.y as i32);
            let mark_hue = if hovered { ASSIGN_HOVER_HUE } else { 0 };
            g.pic((size.x - mark.x) as i32, 0, ASSIGN_MARK, mark_hue);
            if g.body_click().is_some() {
                assign_macro(cx, self.spell);
                return;
            }
        }
        if pressed(g, cx) {
            cx.act(Act::Cast(self.spell));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchFrame;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    #[test]
    fn a_spell_button_opens_by_its_spell_and_keeps_its_place() {
        let id = GumpId::of(SPELL_BUTTON.id, 29);
        assert_eq!(id.place_key(), "spell_button:1D");
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        assert!(manager.open(id, &mut profile));
        assert!(profile.interface.open_panels.contains(&id.place_key()));
        if draw_frames(&mut manager, &mut profile, &WatchFrame::default()) {
            assert!(manager.is_open(&id));
        }
    }
}
