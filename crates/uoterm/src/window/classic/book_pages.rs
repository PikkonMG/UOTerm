//! The pages of the ability books of the classic client: the corners that
//! turn a page, or go to the first or last page with a double click, and an
//! index name whose click turns to its page once the double click time is
//! over. Each turn rustles.

use super::canvas::Canvas;
use super::item_control::{now, ClickDelay};
use super::registry::GumpContext;

const CORNER_LEFT: u16 = 0x08BB;
const CORNER_RIGHT: u16 = 0x08BC;
const CORNER_LEFT_AT: (i32, i32) = (50, 8);
const CORNER_RIGHT_AT: (i32, i32) = (321, 8);
const PAGE_SOUND: u16 = 0x0055;
pub const FIRST_PAGE: usize = 1;

/// The page a book shows, from one, and an index click that waits.
pub struct BookPages {
    page: usize,
    clicks: ClickDelay,
}

impl Default for BookPages {
    fn default() -> Self {
        Self {
            page: FIRST_PAGE,
            clicks: ClickDelay::default(),
        }
    }
}

impl BookPages {
    pub fn page(&self) -> usize {
        self.page
    }

    fn turn_to(&mut self, cx: &mut GumpContext<'_>, page: usize, last: usize) {
        let page = page.clamp(FIRST_PAGE, last.max(FIRST_PAGE));
        if page != self.page {
            self.page = page;
            cx.play_sound(PAGE_SOUND);
        }
    }

    /// A click on an index name: its page shows once no double click came.
    pub fn index_clicked(&mut self, g: &Canvas<'_>, page: usize) {
        self.clicks.clicked(page as u32, now(g));
    }

    /// Draws the corners and turns the pages, for a book of `last` pages.
    pub fn follow(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, last: usize) {
        self.page = self.page.clamp(FIRST_PAGE, last.max(FIRST_PAGE));
        if self.page != FIRST_PAGE {
            let (x, y) = CORNER_LEFT_AT;
            let left = g.pic_button("corner_left", x, y, CORNER_LEFT, 0);
            if left.double_clicked() {
                self.turn_to(cx, FIRST_PAGE, last);
            } else if left.clicked() {
                self.turn_to(cx, self.page - 1, last);
            }
        }
        if self.page != last {
            let (x, y) = CORNER_RIGHT_AT;
            let right = g.pic_button("corner_right", x, y, CORNER_RIGHT, 0);
            if right.double_clicked() {
                self.turn_to(cx, last, last);
            } else if right.clicked() {
                self.turn_to(cx, self.page + 1, last);
            }
        }
        if let Some(page) = self.clicks.due(now(g)) {
            self.turn_to(cx, page as usize, last);
        }
        if self.clicks.is_waiting() {
            g.ctx().request_repaint();
        }
    }
}
