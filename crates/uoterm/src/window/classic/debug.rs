//! The debug gump of the classic client: frames each second and the zoom,
//! on a dark glass; a double click shows more: where the character is and
//! what the pointer is on. The words are `model::stats`', shared with the
//! Modern panel.

use super::canvas::Canvas;
use super::registry::{well_known, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::window::model::stats::{self, DebugFacts};

pub const DEBUG: GumpKind = GumpKind {
    id: well_known::DEBUG,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(Debug::default()),
};

const SHADE_OPACITY: f32 = 0.7;
const TEXT_AT: i32 = 10;
const FONT: u8 = 1;
const WHITE: u16 = 0xFFFF;

#[derive(Default)]
pub struct Debug {
    /// Shows every line, not only the frames and the zoom.
    full: bool,
}

impl GumpBody for Debug {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        if g.body_double_click() {
            self.full = !self.full;
        }
        let facts = DebugFacts {
            fps: stats::fps(g.ctx().input(|i| i.stable_dt)),
            zoom: g.scene.zoom(),
            selected: g
                .ctx()
                .pointer_hover_pos()
                .and_then(|at| g.scene.thing_at(at))
                .map(|pick| pick.serial),
        };
        let words = stats::debug_words(cx.frame, &facts, self.full);
        let look = TextLook::unicode(FONT, WHITE).bordered();
        let size = g.measure(&words, &look);
        let (w, h) = (size.x as i32 + TEXT_AT * 2, size.y as i32 + TEXT_AT * 2);
        g.shade(0, 0, w, h, 0, SHADE_OPACITY);
        g.label(TEXT_AT, TEXT_AT, &words, &look);
    }
}
