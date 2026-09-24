//! Four windows the shard opens that are not gumps: the old-style menu
//! with a question and a list of answers, the book, the bulletin board, and
//! the paperdoll. Each one shows at all times. The clicks work only while the
//! human has control; a paperdoll only shows, so anyone may close it.

use super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::control::Act;
use super::deck_ui::{is_worn_layer, layer_words};
use super::theme::{self, number_font, text_font, title_font};
use crate::view::{WatchBoard, WatchBook, WatchFrame, WatchOldMenu, WatchPost};
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

const DOLL_PANEL_WIDTH: f32 = 400.0;
const DOLL_PICTURE: Vec2 = Vec2::new(140.0, 220.0);
const DOLL_ROW: f32 = 30.0;
const DOLL_ROWS: usize = 10;
const WORDS_OUT_OF_SIGHT: &str = "Out of sight.";
const WORDS_WEARS_NOTHING: &str = "Wears nothing.";

const WORDS_CANCEL: &str = "Cancel";
const WORDS_CLOSE: &str = "Close";
const WORDS_BACK: &str = "Back";
const WORDS_NEXT: &str = "Next";
const WORDS_WRITE: &str = "Write";
const WORDS_SAVE_PAGE: &str = "Save page";
const HINT_PAGE: &str = "The words of this page";
const HINT_TITLE: &str = "The title of the book";
const WORDS_NAME_IT: &str = "Name it";
const TITLE_FIELD_WIDTH: f32 = 220.0;
const WORDS_POST: &str = "Post";
const WORDS_REPLY: &str = "Reply";
const WORDS_REMOVE: &str = "Remove";
const WORDS_PICK_ONE: &str = "Click a message to read it.";
const WORDS_LOADING: &str = "The message comes in a moment.";
const HINT_SUBJECT: &str = "Subject";
const HINT_TEXT: &str = "Your message";

const BOARD_LIST_WIDTH: f32 = 300.0;
const BOARD_TEXT_WIDTH: f32 = 330.0;
const BOARD_ROWS: usize = 9;
const BOARD_ROW: f32 = 36.0;
const BOARD_WRITE_ROWS: usize = 4;
/// An answer stands a little to the right of the message it answers.
const REPLY_INDENT: f32 = 14.0;
const MAX_INDENTS: usize = 4;

#[derive(Default)]
pub struct PagesUi {
    first_entry: usize,
    /// The book that is open, and the left page that shows, from zero.
    book_at: Option<(u32, usize)>,
    first_post: usize,
    subject: String,
    text: String,
    /// The page being written, and its words.
    writing: Option<(usize, String)>,
    /// The title being typed for the open book.
    new_title: Option<String>,
    /// The count of the last paperdoll seen, none before the first picture,
    /// and the mobile whose paperdoll shows.
    doll_seen: Option<u64>,
    doll_shown: Option<u32>,
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
        ui: &mut egui::Ui,
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
        match &frame.board {
            Some(board) => covered.push(self.board(ui, rect, board, frame, tools)),
            None => self.first_post = 0,
        }
        self.take_paperdoll(frame);
        if let Some(serial) = self.doll_shown {
            covered.push(self.paperdoll(ui, rect, serial, frame, tools));
        }
        covered
    }

    /// Opens each paperdoll the shard sends after the first picture. One
    /// that came before the window opened is old. The count starts again
    /// when the session logs in again, so any change is a new paperdoll.
    fn take_paperdoll(&mut self, frame: &WatchFrame) {
        let newest = frame.paperdoll.as_ref().map(|doll| doll.seq);
        if let (Some(seen), Some(doll)) = (self.doll_seen, frame.paperdoll.as_ref()) {
            if doll.seq != seen {
                self.doll_shown = Some(doll.serial);
            }
        }
        self.doll_seen = newest.or(self.doll_seen).or(Some(0));
    }

    /// The paperdoll of a mobile: the words the shard put at its top, his
    /// figure, and what he wears.
    fn paperdoll(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        serial: u32,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Rect {
        let panel = Rect::from_center_size(
            rect.center(),
            Vec2::new(
                DOLL_PANEL_WIDTH,
                theme::PANEL_PAD * 2.0 + TITLE_ROW + DOLL_ROWS as f32 * DOLL_ROW + FOOT_ROW,
            ),
        );
        let painter = ui.painter();
        theme::panel(painter, panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        let text = frame
            .paperdoll
            .as_ref()
            .filter(|doll| doll.serial == serial)
            .map_or("", |doll| doll.text.as_str());
        painter.text(
            inner.left_top(),
            Align2::LEFT_TOP,
            text,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let doll = Rect::from_min_size(inner.left_top() + Vec2::new(0.0, TITLE_ROW), DOLL_PICTURE);
        painter.rect_filled(doll, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let look = if serial == frame.serial {
            Some(&frame.look)
        } else {
            frame
                .mobiles
                .iter()
                .find(|mobile| mobile.serial == serial)
                .map(|mobile| &mobile.look)
        };
        match look {
            None => {
                painter.text(
                    doll.center(),
                    Align2::CENTER_CENTER,
                    WORDS_OUT_OF_SIGHT,
                    text_font(theme::SIZE_BODY),
                    theme::TEXT_DIM,
                );
            }
            Some(look) => {
                if let Some((texture, sprite)) = tools.scene.doll_picture(frame.map, look) {
                    let area = theme::fit(doll, sprite.width, sprite.height);
                    painter.image(texture, area, sprite.uv, Color32::WHITE);
                }
                let left = doll.right() + theme::ROW_GAP * 2.0;
                let worn: Vec<_> = look
                    .equipment
                    .iter()
                    .filter(|item| is_worn_layer(item.layer))
                    .collect();
                if worn.is_empty() {
                    painter.text(
                        Pos2::new(left, doll.top()),
                        Align2::LEFT_TOP,
                        WORDS_WEARS_NOTHING,
                        text_font(theme::SIZE_SMALL),
                        theme::TEXT_FAINT,
                    );
                }
                for (i, item) in worn.iter().take(DOLL_ROWS).enumerate() {
                    let row = Rect::from_min_size(
                        Pos2::new(left, doll.top() + i as f32 * DOLL_ROW),
                        Vec2::new(inner.right() - left, DOLL_ROW - theme::ROW_GAP),
                    );
                    let art = Rect::from_min_size(row.min, Vec2::splat(row.height()));
                    if let Some((texture, sprite)) =
                        tools.scene.item_picture(frame.map, item.graphic, item.hue)
                    {
                        let area = theme::fit(art, sprite.width, sprite.height);
                        painter.image(texture, area, sprite.uv, Color32::WHITE);
                    }
                    let response = ui.interact(
                        row,
                        Id::new(("doll-row", item.serial, item.layer)),
                        Sense::hover(),
                    );
                    painter.text(
                        Pos2::new(art.right() + theme::ROW_GAP, row.center().y),
                        Align2::LEFT_CENTER,
                        layer_words(item.layer),
                        text_font(theme::SIZE_SMALL),
                        if response.hovered() {
                            theme::TEXT
                        } else {
                            theme::TEXT_DIM
                        },
                    );
                    if response.hovered() && !tools.desk.carries() && !tools.ring.is_open() {
                        tools
                            .tips
                            .point_at(ui, tools.hand, item.serial, "", "", tools.time);
                    }
                }
            }
        }
        let foot = Pos2::new(inner.left(), inner.bottom() - FOOT_ROW + theme::ROW_GAP);
        let (_, closed) = theme::button(ui, foot, WORDS_CLOSE, theme::TEXT_DIM);
        if closed {
            self.doll_shown = None;
        }
        panel
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
        ui: &mut egui::Ui,
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
        match self.new_title.as_mut() {
            Some(words) => {
                let field = Rect::from_min_size(
                    inner.left_top(),
                    Vec2::new(TITLE_FIELD_WIDTH, TITLE_ROW - theme::ROW_GAP),
                );
                ui.painter()
                    .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
                ui.put(
                    field,
                    egui::TextEdit::singleline(words)
                        .frame(false)
                        .margin(egui::Margin::symmetric(8, 4))
                        .hint_text(HINT_TITLE)
                        .font(text_font(theme::SIZE_BODY))
                        .text_color(theme::TEXT),
                );
            }
            None => {
                ui.painter().text(
                    inner.left_top(),
                    Align2::LEFT_TOP,
                    &book.title,
                    title_font(theme::SIZE_TITLE),
                    theme::TEXT,
                );
            }
        }
        ui.painter().text(
            inner.right_top() + Vec2::new(0.0, theme::ROW_GAP),
            Align2::RIGHT_TOP,
            format!("by {}", book.author),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let painter = ui.painter().clone();
        let painter = &painter;
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
        if let Some((page, words)) = self.writing.as_mut() {
            // The page being written takes the place of the left paper.
            let paper = Rect::from_min_size(
                inner.left_top() + Vec2::new(0.0, TITLE_ROW),
                Vec2::new(BOOK_PAGE_WIDTH, paper_height),
            );
            ui.painter()
                .rect_filled(paper, CornerRadius::same(CELL_RADIUS), theme::TRACK);
            ui.put(
                paper,
                egui::TextEdit::multiline(words)
                    .frame(false)
                    .margin(egui::Margin::symmetric(8, 6))
                    .hint_text(HINT_PAGE)
                    .font(text_font(theme::SIZE_BODY))
                    .text_color(theme::TEXT),
            );
            ui.painter().text(
                paper.center_bottom() - Vec2::new(0.0, theme::ROW_GAP),
                Align2::CENTER_BOTTOM,
                (*page + 1).to_string(),
                number_font(theme::SIZE_SMALL),
                theme::TEXT_DIM,
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
            let writing = self.writing.is_some();
            let words = if writing {
                WORDS_SAVE_PAGE
            } else {
                WORDS_WRITE
            };
            let (write, pressed) = theme::button(
                ui,
                Pos2::new(next.right() + BOOK_GUTTER, foot.y),
                words,
                theme::GOAL,
            );
            if pressed {
                match self.writing.take() {
                    Some((page, words)) => tools.hand.act(Act::BookPage {
                        page: page as u16 + 1,
                        text: words,
                    }),
                    None => {
                        let words = book
                            .pages
                            .get(now_left)
                            .map(|lines| lines.join("\n"))
                            .unwrap_or_default();
                        self.writing = Some((now_left, words));
                    }
                }
            }
            let naming = self.new_title.is_some();
            let (name_it, named) = theme::button(
                ui,
                Pos2::new(write.right() + theme::ROW_GAP, foot.y),
                WORDS_NAME_IT,
                if naming { theme::GOAL } else { theme::TEXT },
            );
            if named {
                match self.new_title.take() {
                    Some(title) => tools.hand.act(Act::BookName {
                        title,
                        author: book.author.clone(),
                    }),
                    None => self.new_title = Some(book.title.clone()),
                }
            }
            let (_, closed) = theme::button(
                ui,
                Pos2::new(name_it.right() + theme::ROW_GAP, foot.y),
                WORDS_CLOSE,
                theme::TEXT_DIM,
            );
            if closed {
                self.writing = None;
                self.new_title = None;
                tools.hand.act(Act::BookClose);
            }
        }
        panel
    }
}

/// The messages of a board with each answer under its message, and how
/// deep each one stands. A message whose parent is gone stands at the left.
fn threaded(posts: &[WatchPost]) -> Vec<(&WatchPost, usize)> {
    fn add<'a>(
        posts: &'a [WatchPost],
        parent: Option<u32>,
        depth: usize,
        out: &mut Vec<(&'a WatchPost, usize)>,
    ) {
        for post in posts.iter().filter(|post| post.parent == parent) {
            out.push((post, depth));
            add(posts, Some(post.serial), depth + 1, out);
        }
    }
    let known = |serial: u32| posts.iter().any(|post| post.serial == serial);
    let mut out = Vec::new();
    add(posts, None, 0, &mut out);
    for orphan in posts
        .iter()
        .filter(|post| post.parent.is_some_and(|parent| !known(parent)))
    {
        out.push((orphan, 0));
        add(posts, Some(orphan.serial), 1, &mut out);
    }
    out
}

impl PagesUi {
    fn board(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        board: &WatchBoard,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Rect {
        let list_height = BOARD_ROWS as f32 * BOARD_ROW;
        let panel = Rect::from_center_size(
            rect.center(),
            Vec2::new(
                BOARD_LIST_WIDTH + BOOK_GUTTER + BOARD_TEXT_WIDTH + theme::PANEL_PAD * 2.0,
                TITLE_ROW + list_height + FOOT_ROW + theme::PANEL_PAD * 2.0,
            ),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        let live = frame.human_control;
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            &board.name,
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let rows = threaded(&board.posts);
        let last_first = rows.len().saturating_sub(BOARD_ROWS);
        self.first_post = scrolled(ui, panel, self.first_post, last_first);
        for (i, (post, depth)) in rows
            .iter()
            .skip(self.first_post)
            .take(BOARD_ROWS)
            .enumerate()
        {
            let indent = REPLY_INDENT * (*depth).min(MAX_INDENTS) as f32;
            let row = Rect::from_min_size(
                inner.left_top() + Vec2::new(indent, TITLE_ROW + i as f32 * BOARD_ROW),
                Vec2::new(BOARD_LIST_WIDTH - indent, BOARD_ROW - theme::ROW_GAP / 2.0),
            );
            let response = ui.interact(row, Id::new(("board-post", post.serial)), Sense::click());
            let fill = match (board.reading == Some(post.serial), response.hovered()) {
                (true, _) => theme::BUTTON_HOVER,
                (false, true) if live => theme::BUTTON_HOVER,
                _ => theme::BUTTON,
            };
            let painter = ui.painter().with_clip_rect(row);
            painter.rect_filled(row, CornerRadius::same(CELL_RADIUS), fill);
            painter.text(
                row.left_top() + Vec2::new(theme::ROW_GAP, 2.0),
                Align2::LEFT_TOP,
                &post.subject,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            painter.text(
                row.left_bottom() + Vec2::new(theme::ROW_GAP, -2.0),
                Align2::LEFT_BOTTOM,
                format!("{}  {}", post.poster, post.time),
                text_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            if live && response.clicked() {
                tools.hand.act(Act::BoardRead(post.serial));
            }
        }
        let reading = board
            .reading
            .and_then(|serial| board.posts.iter().find(|post| post.serial == serial));
        let text_left = inner.left() + BOARD_LIST_WIDTH + BOOK_GUTTER;
        let write_height = BOARD_WRITE_ROWS as f32 * BOOK_LINE + BOARD_ROW;
        let paper = Rect::from_min_max(
            Pos2::new(text_left, inner.top() + TITLE_ROW),
            Pos2::new(
                inner.right(),
                inner.top() + TITLE_ROW + list_height - write_height,
            ),
        );
        ui.painter()
            .rect_filled(paper, CornerRadius::same(CELL_RADIUS), PAPER);
        let words = match reading.map(|post| post.lines.as_ref()) {
            None => WORDS_PICK_ONE.to_string(),
            Some(None) => WORDS_LOADING.to_string(),
            Some(Some(lines)) => lines.join("\n"),
        };
        let mut job = egui::text::LayoutJob::single_section(
            words,
            egui::TextFormat::simple(text_font(theme::SIZE_BODY), INK),
        );
        job.wrap.max_width = paper.width() - theme::PANEL_PAD * 2.0;
        let galley = ui.painter().layout_job(job);
        ui.painter().with_clip_rect(paper).galley(
            paper.left_top() + Vec2::splat(theme::PANEL_PAD / 2.0),
            galley,
            INK,
        );
        if !live {
            return panel;
        }
        let subject_row = Rect::from_min_size(
            Pos2::new(text_left, paper.bottom() + theme::ROW_GAP),
            Vec2::new(BOARD_TEXT_WIDTH, BOARD_ROW - theme::ROW_GAP * 2.0),
        );
        let text_box = Rect::from_min_max(
            Pos2::new(text_left, subject_row.bottom() + theme::ROW_GAP),
            Pos2::new(inner.right(), inner.top() + TITLE_ROW + list_height),
        );
        for field in [subject_row, text_box] {
            ui.painter()
                .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        }
        ui.put(
            subject_row,
            egui::TextEdit::singleline(&mut self.subject)
                .frame(false)
                .hint_text(HINT_SUBJECT)
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        ui.put(
            text_box,
            egui::TextEdit::multiline(&mut self.text)
                .frame(false)
                .hint_text(HINT_TEXT)
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let foot = Pos2::new(inner.left(), inner.bottom() - FOOT_ROW + theme::ROW_GAP);
        let (post_at, posted) = theme::button(ui, foot, WORDS_POST, theme::GOAL);
        let mut next = Pos2::new(post_at.right() + theme::ROW_GAP, foot.y);
        let mut replied = false;
        let mut removed = false;
        if reading.is_some() {
            let (at, pressed) = theme::button(ui, next, WORDS_REPLY, theme::TEXT);
            replied = pressed;
            let (at, pressed) = theme::button(
                ui,
                Pos2::new(at.right() + theme::ROW_GAP, foot.y),
                WORDS_REMOVE,
                theme::ALARM,
            );
            removed = pressed;
            next = Pos2::new(at.right() + theme::ROW_GAP, foot.y);
        }
        let (_, closed) = theme::button(ui, next, WORDS_CLOSE, theme::TEXT_DIM);
        let ready = !self.subject.trim().is_empty();
        if (posted || replied) && ready {
            tools.hand.act(Act::BoardPost {
                subject: self.subject.trim().to_string(),
                text: self.text.clone(),
                reply_to: reading.filter(|_| replied).map(|post| post.serial),
            });
            self.subject.clear();
            self.text.clear();
        } else if let Some(post) = reading.filter(|_| removed) {
            tools.hand.act(Act::BoardRemove(post.serial));
        } else if closed {
            tools.hand.act(Act::BoardClose);
        }
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_stands_under_its_message_and_an_orphan_at_the_left() {
        let post = |serial: u32, parent: Option<u32>| WatchPost {
            serial,
            parent,
            ..WatchPost::default()
        };
        let posts = vec![
            post(1, None),
            post(2, None),
            post(3, Some(1)),
            post(4, Some(3)),
            post(5, Some(99)),
        ];
        let order: Vec<(u32, usize)> = threaded(&posts)
            .into_iter()
            .map(|(post, depth)| (post.serial, depth))
            .collect();
        assert_eq!(order, vec![(1, 0), (3, 1), (4, 2), (2, 0), (5, 0)]);
    }

    #[test]
    fn a_paperdoll_opens_when_the_shard_sends_one_after_the_first_picture() {
        use crate::view::WatchPaperdoll;
        const STRANGER: u32 = 0x0000_0A11;
        let doll = |seq| WatchPaperdoll {
            serial: STRANGER,
            text: "Someone the Brave".into(),
            seq,
        };
        let mut pages = PagesUi::default();
        let mut frame = WatchFrame {
            paperdoll: Some(doll(3)),
            ..WatchFrame::default()
        };
        pages.take_paperdoll(&frame);
        assert_eq!(pages.doll_shown, None, "one from before the window is old");
        frame.paperdoll = Some(doll(4));
        pages.take_paperdoll(&frame);
        assert_eq!(pages.doll_shown, Some(STRANGER));
        pages.doll_shown = None;
        pages.take_paperdoll(&frame);
        assert_eq!(pages.doll_shown, None, "a closed one stays closed");
        frame.paperdoll = Some(doll(1));
        pages.take_paperdoll(&frame);
        assert_eq!(
            pages.doll_shown,
            Some(STRANGER),
            "after a new login the count starts again"
        );
    }

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
