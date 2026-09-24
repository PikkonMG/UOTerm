//! The skill buttons of the desktop: a
//! skill dragged out of the skills gump stays on the desktop as a small
//! frame with its name, and uses the skill with a double click, or one
//! click with "Single-click UI buttons". It snaps to the spell and macro
//! buttons, and the profile of the character keeps it.

use super::canvas::Canvas;
use super::registry::{GumpBody, GumpContext, GumpKind, GumpRules};
use super::spell_button::{pressed, BUTTON_GROUP};
use super::text::TextLook;
use crate::view::WatchFrame;
use crate::window::control::Act;
use eframe::egui::{Pos2, Vec2};
use uoterm_nav::TextAlign;

/// The id of the button of one skill, by the number of the skill.
const SKILL_BUTTON_ID: &str = "skill_button";

pub const SKILL_BUTTON: GumpKind = GumpKind {
    id: SKILL_BUTTON_ID,
    rules: GumpRules {
        anchor: Some(BUTTON_GROUP),
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(SkillButton::new(serial.unwrap_or_default())),
};

const FRAME: u16 = 0x24B8;
pub const BUTTON_SIZE: (i32, i32) = (88, 44);
const TEXT_INSET: i32 = 4;
const TEXT_FONT: u8 = 1;
const TEXT_HUE: u16 = 0;
const HALF: i32 = 2;

/// Where a skill button opens for a skill dragged out of the skills gump:
/// the mouse at its middle.
pub fn button_place(mouse: Pos2) -> Pos2 {
    mouse - Vec2::new((BUTTON_SIZE.0 / HALF) as f32, (BUTTON_SIZE.1 / HALF) as f32)
}

pub struct SkillButton {
    skill: u16,
}

impl SkillButton {
    pub fn new(serial: u32) -> Self {
        Self {
            skill: u16::try_from(serial).unwrap_or_default(),
        }
    }
}

impl GumpBody for SkillButton {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(skill) = cx.frame.skills.iter().find(|s| s.id == self.skill) else {
            return;
        };
        let (w, h) = BUTTON_SIZE;
        g.frame(0, 0, w, h, FRAME);
        let look = TextLook::unicode(TEXT_FONT, TEXT_HUE)
            .wrap((w - TEXT_INSET * HALF) as u32)
            .aligned(TextAlign::Center);
        let size = g.measure(&skill.name, &look);
        g.label(
            TEXT_INSET,
            h / HALF - size.y as i32 / HALF,
            &skill.name,
            &look,
        );
        if pressed(g, cx) {
            cx.act(Act::UseSkill(self.skill));
        }
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.skills.is_empty() || frame.skills.iter().any(|s| s.id == self.skill)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dragged_skill_puts_its_button_round_the_mouse() {
        assert_eq!(button_place(Pos2::new(100.0, 100.0)), Pos2::new(56.0, 78.0));
    }
}
