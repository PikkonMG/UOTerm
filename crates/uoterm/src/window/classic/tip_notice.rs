//! A tip of the day or a notice of the shard (`0xA6`), as the reference client
//! draws it: the words on a scroll the player makes longer
//! or shorter, under the title of a tip or of a notice. A tip has arrows
//! that ask the shard for the tip before or after it (`0xA7`); the gump
//! closes then, and the new tip opens it again.

use super::canvas::{ButtonArt, Canvas};
use super::registry::{well_known, GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::WatchFrame;
use crate::window::control::Act;
use eframe::egui::Pos2;

pub const TIP_NOTICE: GumpKind = GumpKind {
    id: well_known::TIP_NOTICE,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(TipNotice),
};

const SCROLL: u16 = 0x0820;
const FIRST_HEIGHT: i32 = 300;
const TIP_TITLE: u16 = 0x09CA;
const NOTICE_TITLE: u16 = 0x09D2;
const PREVIOUS: ButtonArt = ButtonArt::new(0x09CC, 0x09CC, 0x09CC);
const NEXT: ButtonArt = ButtonArt::new(0x09CD, 0x09CD, 0x09CD);
const PREVIOUS_X: i32 = 35;
const NEXT_X: i32 = 240;
const BUTTON_Y: i32 = 0;
// The words scroll in this box; it ends this far above the foot.
const WORDS_AREA_Y: i32 = 32;
const WORDS_AREA_WIDTH: i32 = 272;
const WORDS_AREA_FOOT_ROOM: i32 = 96;
const WORDS_X: i32 = 35;
const WORDS_WIDTH: u32 = 220;
const WORDS_FONT: u8 = 6;
const WORDS_HUE: u16 = 0;
const HALF: i32 = 2;
const NO_HUE: u16 = 0;
/// Where a tip and a notice open, as the classic client puts them.
const TIP_PLACE: Pos2 = Pos2::new(200.0, 100.0);
const NOTICE_PLACE: Pos2 = Pos2::new(20.0, 20.0);

/// The gump shows the last words of the shard; the frame holds them.
pub struct TipNotice;

/// Where the title sits: in the middle of the top of the scroll.
fn title_at(top: (i32, i32), title: (i32, i32)) -> (i32, i32) {
    ((top.0 - title.0) / HALF, (top.1 - title.1) / HALF)
}

impl GumpBody for TipNotice {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(words) = cx.frame.shard_notice.as_deref() else {
            return;
        };
        let tip = cx.frame.shard_tip.is_some();
        let size = g.expandable_scroll(0, 0, SCROLL, FIRST_HEIGHT);
        let title = if tip { TIP_TITLE } else { NOTICE_TITLE };
        let top = g.gump_size(SCROLL).unwrap_or_default();
        let title_size = g.gump_size(title).unwrap_or_default();
        let (x, y) = title_at(
            (top.x as i32, top.y as i32),
            (title_size.x as i32, title_size.y as i32),
        );
        g.pic(x, y, title, NO_HUE);
        let look = TextLook::ascii(WORDS_FONT, WORDS_HUE).wrap(WORDS_WIDTH);
        let height = size.y as i32 - WORDS_AREA_FOOT_ROOM;
        g.scroll_area("words", 0, WORDS_AREA_Y, WORDS_AREA_WIDTH, height, |g| {
            g.label(WORDS_X, 0, words, &look).y as i32
        });
        if !tip {
            return;
        }
        let turn = if g.button("previous", PREVIOUS_X, BUTTON_Y, PREVIOUS) {
            Some(false)
        } else if g.button("next", NEXT_X, BUTTON_Y, NEXT) {
            Some(true)
        } else {
            None
        };
        if let Some(next) = turn {
            cx.act(Act::Tip { next });
            cx.close(cx.me);
        }
    }

    fn first_place(&self, frame: &WatchFrame) -> Option<Pos2> {
        Some(if frame.shard_tip.is_some() {
            TIP_PLACE
        } else {
            NOTICE_PLACE
        })
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.shard_notice.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::registry::GumpId;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    fn frame(tip: Option<u32>) -> WatchFrame {
        WatchFrame {
            shard_notice: Some("Hail, traveler.\nWelcome.".into()),
            shard_tip: tip,
            ..WatchFrame::default()
        }
    }

    #[test]
    fn a_tip_and_a_notice_open_where_the_classic_client_puts_them() {
        assert_eq!(TipNotice.first_place(&frame(Some(3))), Some(TIP_PLACE));
        assert_eq!(TipNotice.first_place(&frame(None)), Some(NOTICE_PLACE));
        assert_eq!(title_at((270, 40), (100, 20)), (85, 10));
    }

    #[test]
    fn the_gump_lives_while_the_shard_has_words() {
        assert!(TipNotice.alive(&frame(None)));
        assert!(!TipNotice.alive(&WatchFrame::default()));
    }

    #[test]
    fn a_tip_and_a_notice_draw_with_the_client_files() {
        for tip in [Some(3), None] {
            let mut profile = Profile::default();
            let mut manager = GumpManager::default();
            let id = GumpId::one(well_known::TIP_NOTICE);
            manager.open(id, &mut profile);
            if !draw_frames(&mut manager, &mut profile, &frame(tip)) {
                return;
            }
            assert!(manager.is_open(&id));
            draw_frames(&mut manager, &mut profile, &WatchFrame::default());
            assert!(!manager.is_open(&id), "no words, no gump");
        }
    }
}
