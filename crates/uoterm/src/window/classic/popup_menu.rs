//! The context menu the shard sends for a thing, in the Classic style, as
//! the reference client draws it: its lines on a see-through stone
//! frame by the pointer. The line under the pointer is the one a left click
//! picks; a right click closes the menu. The lines and the acts that pick
//! them are those of the ring of the Modern style (`ring_ui::shard_lines`).

use super::canvas::Canvas;
use super::registry::{Closing, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::WatchFrame;
use crate::window::control::Act;
use crate::window::ring_ui::shard_lines;
use eframe::egui::PointerButton;

pub const POPUP_MENU_ID: &str = "popup_menu";

pub const POPUP_MENU: GumpKind = GumpKind {
    id: POPUP_MENU_ID,
    rules: GumpRules {
        movable: false,
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(PopupMenu::new(serial.unwrap_or_default())),
};

/// The one menu of the shard: a new one takes the place of the last.
pub const POPUP_MENU_GUMP: GumpId = GumpId::one(POPUP_MENU_ID);

const FRAME: u16 = 0x0A3C;
const FRAME_ALPHA: f32 = 0.75;
const PAD: i32 = 10;
const FIRST_LINE: i32 = 10;
const ROOM_BELOW: i32 = 20;
const FONT: u8 = 1;
const HUE: u16 = 0xFFFF;
const HUE_OFF: u16 = 0x0386;
/// A menu too small for a line does not show.
const LEAST_SIDE: i32 = 10;

/// The menu of one thing, while the shard's lines for it come and while the
/// player picks.
pub struct PopupMenu {
    serial: u32,
    /// The line under the pointer, by its place.
    hovered: Option<usize>,
}

impl PopupMenu {
    pub fn new(serial: u32) -> Self {
        Self {
            serial,
            hovered: None,
        }
    }
}

impl GumpBody for PopupMenu {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let lines = shard_lines(cx.frame, self.serial);
        if lines.is_empty() {
            return;
        }
        let looks: Vec<TextLook> = lines
            .iter()
            .map(|(_, enabled, _)| TextLook::unicode(FONT, if *enabled { HUE } else { HUE_OFF }))
            .collect();
        let sizes: Vec<(i32, i32)> = lines
            .iter()
            .zip(&looks)
            .map(|((words, _, _), look)| {
                let size = g.measure(words, look);
                (size.x as i32, size.y as i32)
            })
            .collect();
        let width = sizes.iter().map(|(w, _)| *w).max().unwrap_or(0) + PAD * 2;
        let height = sizes.iter().map(|(_, h)| *h).sum::<i32>() + ROOM_BELOW;
        if width <= PAD * 2 || height <= LEAST_SIDE {
            return;
        }
        g.faded(FRAME_ALPHA, |g| g.frame(0, 0, width, height, FRAME));
        let mut y = FIRST_LINE;
        for (at, (((words, _, _), look), (_, line_height))) in
            lines.iter().zip(&looks).zip(&sizes).enumerate()
        {
            if g.hit_box(("line", at), PAD, y, width - PAD * 2, *line_height)
                .hovered()
            {
                self.hovered = Some(at);
            }
            g.label(PAD, y, words, look);
            y += line_height;
        }
        let released = g
            .ctx()
            .input(|i| i.pointer.button_released(PointerButton::Primary));
        let on_menu = g.hovered(0, 0, width, height);
        if released && on_menu {
            if let Some((_, true, act)) = self.hovered.and_then(|at| lines.get(at)).cloned() {
                cx.act(act);
            }
            cx.close(cx.me);
        } else if released {
            cx.act(Act::MenuClose);
            cx.close(cx.me);
        }
    }

    /// A right click closes the menu and tells the shard.
    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(Act::MenuClose);
        Closing::Now
    }

    /// The menu goes when the shard takes it away, or the human gives the
    /// character back.
    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.human_control
            && frame
                .context_menu
                .as_ref()
                .is_none_or(|menu| menu.serial == self.serial)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchMenu, WatchMenuLine};

    const ORC: u32 = 9;

    #[test]
    fn the_menu_lives_while_the_shard_keeps_it_for_its_thing() {
        let menu = PopupMenu::new(ORC);
        let mut frame = WatchFrame {
            human_control: true,
            ..WatchFrame::default()
        };
        assert!(menu.alive(&frame), "the lines have not come yet");
        frame.context_menu = Some(WatchMenu {
            serial: ORC,
            lines: vec![WatchMenuLine {
                index: 1,
                words: "Open Paperdoll".into(),
                enabled: true,
            }],
        });
        assert!(menu.alive(&frame));
        frame.context_menu.as_mut().unwrap().serial = ORC + 1;
        assert!(!menu.alive(&frame), "a menu for another thing replaced it");
        frame.human_control = false;
        frame.context_menu = None;
        assert!(!menu.alive(&frame));
    }

    #[test]
    fn a_menu_of_the_shard_draws_on_the_client_files() {
        let mut manager = super::super::manager::GumpManager::default();
        let mut profile = crate::window::settings::Profile::default();
        manager.open_body(POPUP_MENU_GUMP, Box::new(PopupMenu::new(ORC)), &mut profile);
        let frame = WatchFrame {
            human_control: true,
            context_menu: Some(WatchMenu {
                serial: ORC,
                lines: vec![WatchMenuLine {
                    index: 1,
                    words: "Open Paperdoll".into(),
                    enabled: true,
                }],
            }),
            ..WatchFrame::default()
        };
        if !super::super::testing::draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert!(manager.is_open(&POPUP_MENU_GUMP));
    }
}
