//! Two windows the shard opens that are not gumps: the old-style menu with
//! a question and a list of answers, and the book. Each one shows at all
//! times. The clicks work only while the human has control.

use super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::control::Act;
use super::theme::{self, number_font, text_font, title_font};
use crate::view::{WatchBook, WatchFrame, WatchOldMenu};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Vec2};

const MENU_WIDTH: f32 = 340.0;
const MENU_ROWS: usize = 8;
const MENU_ROW: f32 = 34.0;
const ART_SIDE: f32 = 30.0;
const TITLE_ROW: f32 = 32.0;
const FOOT_ROW: f32 = 40.0;
const BOOK_PAGE_WIDTH: f32 = 250.0;
const BOOK_LINES: usize = 10;
const BOOK_LINE: f32 = 20.0;
const BOOK_GUTTER: f32 = 28.0;
/// The book shows two pages at a time, as a book that lies open.
const PAGES_SHOWN: usize = 2;
const PAPER: Color32 = Color32::from_rgb(226, 214, 184);
const INK: Color32 = Color32::from_rgb(46, 36, 24);

const WORDS_CANCEL: &str = "Cancel";
const WORDS_CLOSE: &str = "Close";
const WORDS_BACK: &str = "Back";
const WORDS_NEXT: &str = "Next";

#[derive(Default)]
pub struct PagesUi {
    first_entry: usize,
    /// The book that is open, and the left page that shows, from zero.
    book_at: Option<(u32, usize)>,
}

/// The left page after a turn, kept inside the book.
fn turned(left_page: usize, forward: bool, page_count: usize) -> usize {
    let last_left = page_count.saturating_sub(1) / PAGES_SHOWN * PAGES_SHOWN;
    if forward {
        (left_page + PAGES_SHOWN).min(last_left)
    } else {
        left_page.saturating_sub(PAGES_SHOWN)
    }
}

impl PagesUi {
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Vec<Rect> {
        let mut covered = Vec::new();
        match &frame.old_menu {
            Some(menu) => covered.push(self.menu(ui, rect, menu, frame, tools)),
            None => self.first_entry = 0,
        }
        match &frame.book {
            Some(book) => covered.push(self.book(ui, rect, book, frame, tools)),
            None => self.book_at = None,
        }
        covered
    }

    fn menu(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        menu: &WatchOldMenu,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Rect {
        let rows = menu.entries.len().clamp(1, MENU_ROWS);
        let panel = Rect::from_center_size(
            rect.center(),
            Vec2::new(
                MENU_WIDTH,
                theme::PANEL_PAD * 2.0 + TITLE_ROW + rows as f32 * MENU_ROW + FOOT_ROW,
            ),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            &menu.question,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let last_first = menu.entries.len().saturating_sub(MENU_ROWS);
        self.first_entry = scrolled(ui, panel, self.first_entry, last_first);
        let live = frame.human_control;
        let shown = menu
            .entries
            .iter()
            .enumerate()
            .skip(self.first_entry)
            .take(MENU_ROWS);
        for (row_index, (entry_index, entry)) in shown.enumerate() {
            let row = Rect::from_min_size(
                inner.left_top() + Vec2::new(0.0, TITLE_ROW + row_index as f32 * MENU_ROW),
                Vec2::new(inner.width(), MENU_ROW - theme::ROW_GAP / 2.0),
            );
            let response = ui.interact(row, Id::new(("old-menu", entry_index)), Sense::click());
            let fill = if live && response.hovered() {
                theme::BUTTON_HOVER
            } else {
                theme::BUTTON
            };
            let painter = ui.painter();
            painter.rect_filled(row, CornerRadius::same(CELL_RADIUS), fill);
            let mut words_left = row.left() + theme::ROW_GAP;
            if entry.graphic != 0 {
                let art = Rect::from_center_size(
                    Pos2::new(row.left() + ART_SIDE / 2.0 + theme::ROW_GAP, row.center().y),
                    Vec2::splat(ART_SIDE),
                );
                if let Some((texture, sprite)) =
                    tools
                        .scene
                        .item_picture(frame.map, entry.graphic, entry.hue)
                {
                    let area = theme::fit(art, sprite.width, sprite.height);
                    painter.image(texture, area, sprite.uv, Color32::WHITE);
                }
                words_left = art.right() + theme::ROW_GAP;
            }
            painter.text(
                Pos2::new(words_left, row.center().y),
                Align2::LEFT_CENTER,
                &entry.name,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            if live && response.clicked() {
                // The shard counts the entries from one.
                let index = u16::try_from(entry_index + 1).unwrap_or(u16::MAX);
                tools.hand.act(Act::OldMenuPick(Some(index)));
            }
        }
        if live {
            let foot = Pos2::new(inner.left(), inner.bottom() - FOOT_ROW + theme::ROW_GAP);
            let (_, canceled) = theme::button(ui, foot, WORDS_CANCEL, theme::TEXT_DIM);
            if canceled {
                tools.hand.act(Act::OldMenuPick(None));
            }
        }
        panel
    }

    fn book(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        book: &WatchBook,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Rect {
        let left_page = match self.book_at {
            Some((serial, page)) if serial == book.serial => page,
            _ => 0,
        };
        let page_count = usize::from(book.page_count).max(book.pages.len());
        let paper_height = TITLE_ROW + BOOK_LINES as f32 * BOOK_LINE;
        let panel = Rect::from_center_size(
            rect.center(),
            Vec2::new(
                BOOK_PAGE_WIDTH * PAGES_SHOWN as f32 + BOOK_GUTTER + theme::PANEL_PAD * 2.0,
                TITLE_ROW + paper_height + FOOT_ROW + theme::PANEL_PAD * 2.0,
            ),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        let painter = ui.painter();
        painter.text(
            inner.left_top(),
            Align2::LEFT_TOP,
            &book.title,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        painter.text(
            inner.right_top() + Vec2::new(0.0, theme::ROW_GAP),
            Align2::RIGHT_TOP,
            format!("by {}", book.author),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        for side in 0..PAGES_SHOWN {
            let number = left_page + side;
            let paper = Rect::from_min_size(
                inner.left_top()
                    + Vec2::new(side as f32 * (BOOK_PAGE_WIDTH + BOOK_GUTTER), TITLE_ROW),
                Vec2::new(BOOK_PAGE_WIDTH, paper_height),
            );
            painter.rect_filled(paper, CornerRadius::same(CELL_RADIUS), PAPER);
            if number >= page_count {
                continue;
            }
            let lines = book.pages.get(number).map_or(&[][..], Vec::as_slice);
            for (i, line) in lines.iter().take(BOOK_LINES).enumerate() {
                painter.text(
                    paper.left_top()
                        + Vec2::new(theme::PANEL_PAD, theme::ROW_GAP + i as f32 * BOOK_LINE),
                    Align2::LEFT_TOP,
                    line,
                    text_font(theme::SIZE_BODY),
                    INK,
                );
            }
            painter.text(
                paper.center_bottom() - Vec2::new(0.0, theme::ROW_GAP),
                Align2::CENTER_BOTTOM,
                (number + 1).to_string(),
                number_font(theme::SIZE_SMALL),
                INK,
            );
        }
        let foot = Pos2::new(inner.left(), inner.bottom() - FOOT_ROW + theme::ROW_GAP);
        let (back, went_back) = theme::button(ui, foot, WORDS_BACK, theme::TEXT);
        let (next, went_on) = theme::button(
            ui,
            Pos2::new(back.right() + theme::ROW_GAP, foot.y),
            WORDS_NEXT,
            theme::TEXT,
        );
        let mut now_left = left_page;
        if went_back || went_on {
            now_left = turned(left_page, went_on, page_count);
        }
        self.book_at = Some((book.serial, now_left));
        if frame.human_control {
            let (_, closed) = theme::button(
                ui,
                Pos2::new(next.right() + BOOK_GUTTER, foot.y),
                WORDS_CLOSE,
                theme::TEXT_DIM,
            );
            if closed {
                tools.hand.act(Act::BookClose);
            }
        }
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_book_turns_two_pages_and_stops_at_its_ends() {
        assert_eq!(turned(0, true, 5), 2);
        assert_eq!(turned(2, true, 5), 4);
        assert_eq!(turned(4, true, 5), 4);
        assert_eq!(turned(4, false, 5), 2);
        assert_eq!(turned(0, false, 5), 0);
        assert_eq!(turned(0, true, 2), 0);
        assert_eq!(turned(0, true, 0), 0);
    }
}
