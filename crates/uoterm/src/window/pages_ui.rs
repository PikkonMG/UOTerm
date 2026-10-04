//! The windows the shard opens that are not gumps, in the Modern style: the
//! old-style menu with a question and a list of answers, the book, the
//! bulletin board, the paperdoll, the dialog for words the shard waits for,
//! the race change, the tip or notice, and the dye picker when the client
//! files have no gump art. Each one shows at all times, and the player
//! moves and locks it. The clicks work only while the human has control.
//! The Classic style shows each of these as a classic gump.

use super::boxes_ui::{ask_waiting_name, scrolled, single_or_double, Tools, CELL_RADIUS};
use super::control::Act;
use super::deck_ui::layer_words;
use super::model::clicks::ClickDelay;
use super::model::dolls;
use super::model::pages::{shown_depth, threaded, BookDraft, PAGES_SHOWN};
use super::modern::frame::{self, FrameEvent, PanelSpec};
use super::modern::{DyeUi, EntryUi, RaceUi, TipUi};
use super::settings::Profile;
use super::theme::{self, number_font, text_font};
use super::tips;
use crate::view::{WatchBoard, WatchBook, WatchEquip, WatchFrame, WatchOldMenu, WatchPackItem};
use crate::window::bridge;
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Vec2};
use uoterm_view::ui::doll::{
    doll_buttons, doll_first_place, doll_health, doll_look, doll_worn, doll_zone, worn_footer,
    DollPanel, DOLL_ID, DOLL_PICTURE, DOLL_ROW, WORDS_CLOSE as WORDS_DOLL_CLOSE,
    WORDS_OUT_OF_SIGHT, WORDS_WEARS_NOTHING,
};
use uoterm_view::ui::pages::{
    board_first_place, board_text, book_first_place, by_words, menu_cancel_act, menu_first_place,
    menu_pick_act, page_line_room, poster_words, BoardDraft, BoardPress, OpenBook, BOARD_ID,
    BOARD_LIST_WIDTH, BOARD_ROW, BOARD_ROWS, BOARD_TEXT_WIDTH, BOARD_WRITE_ROWS, BOOK_GUTTER,
    BOOK_ID, BOOK_LINE, BOOK_LINES, BOOK_PAGE_WIDTH, COVER_ROW, HINT_AUTHOR, HINT_SUBJECT,
    HINT_TEXT, HINT_TITLE, INK, MENU_ID, MENU_ROW, MENU_ROWS, PAGE_MARGIN, PAPER, REPLY_INDENT,
    WORDS_BY, WORDS_CANCEL, WORDS_CLOSE, WORDS_POST, WORDS_REMOVE, WORDS_REPLY, WORDS_SAVE,
};
use uoterm_view::ui::places::FOOT_ROW;

const ART_SIDE: f32 = 30.0;
const HEALTH_ROW: f32 = 18.0;
const BOOK_FIELD_WIDTH: f32 = 220.0;
const PAPER_COLOR: Color32 = bridge::color(PAPER);
const INK_COLOR: Color32 = bridge::color(INK);

#[derive(Default)]
pub struct PagesUi {
    first_entry: usize,
    book: Option<OpenBook>,
    first_post: usize,
    board: BoardDraft,
    /// The paperdolls the shard opened, and the one that shows.
    doll: DollPanel,
    first_worn: usize,
    clicks: ClickDelay,
    entry: EntryUi,
    race: RaceUi,
    tip: TipUi,
    dye: DyeUi,
}

impl PagesUi {
    /// Closes the paperdoll that shows, as "close all gumps" does.
    pub fn close_doll(&mut self) {
        self.doll.close();
    }

    /// Draws the windows that show in the Modern style. The dye picker
    /// draws here only when the client files have no gump art, as the
    /// classic color picker shows in both styles when they do.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        classic: bool,
        gump_art: bool,
    ) -> Vec<Rect> {
        let fresh_doll = self.doll.follow(frame);
        let mut covered = Vec::new();
        if classic {
            self.first_entry = 0;
            self.book = None;
            self.first_post = 0;
            self.doll.close();
            return covered;
        }
        match &frame.old_menu {
            Some(menu) => covered.push(self.menu(ui, rect, menu, frame, tools, profile)),
            None => self.first_entry = 0,
        }
        match &frame.book {
            Some(book) => covered.push(self.book(ui, rect, book, frame, tools, profile)),
            None => self.book = None,
        }
        match &frame.board {
            Some(board) => covered.push(self.board(ui, rect, board, frame, tools, profile)),
            None => self.first_post = 0,
        }
        if fresh_doll {
            self.first_worn = 0;
        }
        if self.doll.doll.is_some() {
            covered.push(self.paperdoll(ui, rect, frame, tools, profile));
        }
        covered.extend(self.race.draw(ui, rect, frame, tools, profile));
        covered.extend(self.tip.draw(ui, rect, frame, tools, profile));
        if !gump_art {
            covered.extend(self.dye.draw(ui, rect, frame, tools, profile));
        }
        covered.extend(self.entry.draw(ui, rect, frame, tools, profile));
        ask_waiting_name(ui, &mut self.clicks, tools.hand, tools.time);
        covered
    }

    /// The paperdoll of a mobile: the words the shard put at its top, his
    /// figure and health, what he wears, and the buttons of the doll.
    fn paperdoll(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let Some((serial, title, can_lift)) = self
            .doll
            .doll
            .as_ref()
            .map(|doll| (doll.serial, doll.text.clone(), doll.can_lift))
        else {
            return Rect::NOTHING;
        };
        let own = serial == frame.serial;
        let live = frame.human_control;
        let dresses = dolls::dresses(frame, serial, can_lift);
        let spec = PanelSpec {
            id: DOLL_ID,
            title: &title,
            default: bridge::rect(doll_first_place(bridge::area(rect))),
            min_size: None,
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, &title);
        if dresses && live {
            tools
                .desk
                .zone(bridge::area(panel), doll_zone(frame, serial));
        }
        let picture = Rect::from_min_size(body.min, bridge::vec2(DOLL_PICTURE));
        ui.painter()
            .rect_filled(picture, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let look = doll_look(frame, serial);
        let health = Rect::from_min_size(
            Pos2::new(picture.left(), picture.bottom() + theme::ROW_GAP),
            Vec2::new(picture.width(), HEALTH_ROW),
        );
        health_bar(ui, health, frame, serial);
        match look {
            None => {
                ui.painter().text(
                    picture.center(),
                    Align2::CENTER_CENTER,
                    WORDS_OUT_OF_SIGHT,
                    text_font(theme::SIZE_BODY),
                    theme::TEXT_DIM,
                );
            }
            Some(look) => {
                if let Some((texture, sprite)) = tools.scene.doll_picture(look) {
                    let area = theme::fit(picture, sprite.width, sprite.height);
                    ui.painter()
                        .image(texture, area, bridge::rect(sprite.uv), Color32::WHITE);
                }
                let list = Rect::from_min_max(
                    Pos2::new(picture.right() + theme::ROW_GAP * 2.0, body.top()),
                    Pos2::new(body.right(), body.bottom() - FOOT_ROW),
                );
                let worn = doll_worn(look);
                self.worn_rows(ui, list, &worn, frame, tools, dresses);
            }
        }
        if live {
            self.doll_buttons(
                ui,
                Pos2::new(body.left(), body.bottom() - FOOT_ROW + theme::ROW_GAP),
                serial,
                own,
                tools,
            );
        }
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) {
            self.doll.close();
        }
        panel
    }

    /// The worn items of a paperdoll, a row each. A click targets or names
    /// one, a double click uses it, and a drag takes it off a doll the
    /// character dresses.
    fn worn_rows(
        &mut self,
        ui: &egui::Ui,
        list: Rect,
        worn: &[&WatchEquip],
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        dresses: bool,
    ) {
        if worn.is_empty() {
            ui.painter().text(
                list.left_top(),
                Align2::LEFT_TOP,
                WORDS_WEARS_NOTHING,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            return;
        }
        let rows = ((list.height() / DOLL_ROW).floor() as usize).max(1);
        self.first_worn = scrolled(ui, list, self.first_worn, worn.len().saturating_sub(rows));
        let live = frame.human_control;
        for (at, item) in worn.iter().skip(self.first_worn).take(rows).enumerate() {
            let row = Rect::from_min_size(
                list.left_top() + Vec2::new(0.0, at as f32 * DOLL_ROW),
                Vec2::new(list.width(), DOLL_ROW - theme::ROW_GAP),
            );
            let response = ui.interact(
                row,
                Id::new(("doll-row", item.serial, item.layer)),
                Sense::click_and_drag(),
            );
            let art = Rect::from_min_size(row.min, Vec2::splat(row.height()));
            if let Some((texture, sprite)) = tools.scene.item_picture(item.graphic, item.hue) {
                let area = theme::fit(art, sprite.width, sprite.height);
                ui.painter()
                    .image(texture, area, bridge::rect(sprite.uv), Color32::WHITE);
            }
            ui.painter().text(
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
                tips::point_at(
                    tools.tips,
                    ui,
                    tools.hand,
                    item.serial,
                    "",
                    worn_footer(live, dresses),
                    tools.time,
                );
            }
            if !live {
                continue;
            }
            if dresses && response.drag_started_by(egui::PointerButton::Primary) {
                tools.desk.pick_up(&WatchPackItem {
                    serial: item.serial,
                    graphic: item.graphic,
                    hue: item.hue,
                    amount: 1,
                    ..WatchPackItem::default()
                });
            } else if single_or_double(
                &response,
                &mut self.clicks,
                frame,
                tools.hand,
                item.serial,
                tools.time,
            ) {
                tools.hand.act(Act::Use(item.serial));
            }
        }
    }

    /// The buttons of a paperdoll: the status of the mobile and his
    /// virtues, and the quests and the guild of the character on his own.
    fn doll_buttons(
        &mut self,
        ui: &egui::Ui,
        foot: Pos2,
        serial: u32,
        own: bool,
        tools: &Tools<'_>,
    ) {
        let buttons = doll_buttons(serial, own);
        let mut at = foot;
        let mut pressed = None;
        for (words, act) in buttons {
            let (area, clicked) = theme::button(ui, at, words, theme::TEXT);
            at = Pos2::new(area.right() + theme::ROW_GAP, foot.y);
            if clicked {
                pressed = Some(act);
            }
        }
        let (_, closed) = theme::button(ui, at, WORDS_DOLL_CLOSE, theme::TEXT_DIM);
        if closed {
            self.doll.close();
        }
        if let Some(act) = pressed {
            tools.hand.act(act);
        }
    }

    fn menu(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        menu: &WatchOldMenu,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let live = frame.human_control;
        let spec = PanelSpec {
            id: MENU_ID,
            title: &menu.question,
            default: bridge::rect(menu_first_place(bridge::area(rect), menu.entries.len())),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, &menu.question);
        let last_first = menu.entries.len().saturating_sub(MENU_ROWS);
        self.first_entry = scrolled(ui, panel, self.first_entry, last_first);
        let shown = menu
            .entries
            .iter()
            .enumerate()
            .skip(self.first_entry)
            .take(MENU_ROWS);
        for (row_index, (entry_index, entry)) in shown.enumerate() {
            let row = Rect::from_min_size(
                body.left_top() + Vec2::new(0.0, row_index as f32 * MENU_ROW),
                Vec2::new(body.width(), MENU_ROW - theme::ROW_GAP / 2.0),
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
                if let Some((texture, sprite)) = tools.scene.item_picture(entry.graphic, entry.hue)
                {
                    let area = theme::fit(art, sprite.width, sprite.height);
                    painter.image(texture, area, bridge::rect(sprite.uv), Color32::WHITE);
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
                tools.hand.act(menu_pick_act(entry_index));
            }
        }
        let mut canceled = false;
        if live {
            let foot = Pos2::new(body.left(), body.bottom() - FOOT_ROW + theme::ROW_GAP);
            (_, canceled) = theme::button(ui, foot, WORDS_CANCEL, theme::TEXT_DIM);
        }
        let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if live && (canceled || closed) {
            tools.hand.act(menu_cancel_act());
        }
        panel
    }

    /// The open book, two pages at a time. In a book the player may write
    /// in, the title, the author and both pages take words, and each change
    /// goes to the shard when the pages turn, on Save, and as the book
    /// closes. A sealed book asks the shard for the pages that come in
    /// sight.
    fn book(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        book: &WatchBook,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let open = OpenBook::follow(&mut self.book, book);
        let live = frame.human_control;
        let writing = live && book.writable;
        let paper_height = BOOK_LINES as f32 * BOOK_LINE;
        let spec = PanelSpec {
            id: BOOK_ID,
            title: &book.title,
            default: bridge::rect(book_first_place(bridge::area(rect))),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, &book.title);
        let cover_row = Rect::from_min_size(body.left_top(), Vec2::new(body.width(), COVER_ROW));
        cover(ui, cover_row, &mut open.draft, writing, book.serial);
        let mut acts = open.asks(book, live);
        for side in 0..PAGES_SHOWN {
            let number = open.left + side;
            let paper = Rect::from_min_size(
                body.left_top()
                    + Vec2::new(side as f32 * (BOOK_PAGE_WIDTH + BOOK_GUTTER), COVER_ROW),
                Vec2::new(BOOK_PAGE_WIDTH, paper_height),
            );
            let fill = if writing { theme::TRACK } else { PAPER_COLOR };
            ui.painter()
                .rect_filled(paper, CornerRadius::same(CELL_RADIUS), fill);
            let Some(page) = open.draft.pages.get(number) else {
                continue;
            };
            if writing {
                let key = Id::new(("book-page", book.serial, number));
                page_edit(ui, paper, key, open, number);
            } else {
                page_words(ui, paper, &page.field);
            }
            ui.painter().text(
                paper.center_bottom() - Vec2::new(0.0, theme::ROW_GAP),
                Align2::CENTER_BOTTOM,
                (number + 1).to_string(),
                number_font(theme::SIZE_SMALL),
                if writing { theme::TEXT_DIM } else { INK_COLOR },
            );
        }
        let foot = Pos2::new(body.left(), body.bottom() - FOOT_ROW + theme::ROW_GAP);
        let mut at = foot;
        let mut to = open.left;
        for (words, target) in open.turns(book) {
            let (area, pressed) = theme::button(ui, at, words, theme::TEXT);
            at = Pos2::new(area.right() + theme::ROW_GAP, foot.y);
            if pressed {
                to = target;
            }
        }
        acts.extend(open.turn(to, writing));
        let mut closing = false;
        if live {
            at.x += BOOK_GUTTER;
            if writing {
                let (area, saved) = theme::button(ui, at, WORDS_SAVE, theme::GOAL);
                at = Pos2::new(area.right() + theme::ROW_GAP, foot.y);
                if saved {
                    acts.extend(open.draft.written());
                }
            }
            let (_, closed) = theme::button(ui, at, WORDS_CLOSE, theme::TEXT_DIM);
            closing = closed;
        }
        closing |= frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if live && closing {
            acts.extend(open.draft.closing());
        }
        for act in acts {
            tools.hand.act(act);
        }
        panel
    }
}

/// The row over the pages: the author, or in a book the player writes in,
/// the fields of the title and the author.
fn cover(ui: &mut egui::Ui, row: Rect, draft: &mut BookDraft<String>, writing: bool, serial: u32) {
    if !writing {
        ui.painter().text(
            row.left_center(),
            Align2::LEFT_CENTER,
            by_words(&draft.author),
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        return;
    }
    let row_height = row.height() - theme::ROW_GAP;
    let title = Rect::from_min_size(row.left_top(), Vec2::new(BOOK_FIELD_WIDTH, row_height));
    let by = ui.painter().text(
        Pos2::new(title.right() + theme::ROW_GAP, title.center().y),
        Align2::LEFT_CENTER,
        WORDS_BY,
        text_font(theme::SIZE_BODY),
        theme::TEXT_DIM,
    );
    let author = Rect::from_min_max(
        Pos2::new(by.right() + theme::ROW_GAP, title.top()),
        Pos2::new(row.right(), title.bottom()),
    );
    let fields = [
        (title, HINT_TITLE, &mut draft.title, "book-title"),
        (author, HINT_AUTHOR, &mut draft.author, "book-author"),
    ];
    let mut changed = false;
    for (area, hint, words, key) in fields {
        ui.painter()
            .rect_filled(area, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        changed |= ui
            .put(
                area,
                egui::TextEdit::singleline(words)
                    .id(Id::new((key, serial)))
                    .frame(false)
                    .margin(egui::Margin::symmetric(8, 4))
                    .hint_text(hint)
                    .font(text_font(theme::SIZE_BODY))
                    .text_color(theme::TEXT),
            )
            .changed();
    }
    draft.cover_changed |= changed;
}

/// The words of a page, a line each.
fn page_words(ui: &egui::Ui, paper: Rect, words: &str) {
    for (at, line) in words.lines().take(BOOK_LINES).enumerate() {
        ui.painter().with_clip_rect(paper).text(
            paper.left_top() + Vec2::new(PAGE_MARGIN, theme::ROW_GAP + at as f32 * BOOK_LINE),
            Align2::LEFT_TOP,
            line,
            text_font(theme::SIZE_BODY),
            INK_COLOR,
        );
    }
}

/// The field of a page the player writes. A line too wide for the page
/// breaks, and a page that is full takes no more.
fn page_edit(ui: &mut egui::Ui, paper: Rect, key: Id, open: &mut OpenBook, number: usize) {
    let Some(page) = open.draft.pages.get(number) else {
        return;
    };
    let mut words = page.field.clone();
    let output = ui
        .scope_builder(egui::UiBuilder::new().max_rect(paper), |ui| {
            egui::TextEdit::multiline(&mut words)
                .id(key)
                .frame(false)
                .desired_width(paper.width())
                .desired_rows(BOOK_LINES)
                .margin(egui::Margin::same(PAGE_MARGIN as i8))
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT)
                .show(ui)
        })
        .inner;
    if !output.response.changed() {
        return;
    }
    let caret = output
        .cursor_range
        .map_or(words.chars().count(), |range| range.primary.ccursor.index);
    let room = page_line_room();
    let fits = |line: &str| {
        ui.fonts(|fonts| {
            fonts
                .layout_no_wrap(line.to_string(), text_font(theme::SIZE_BODY), theme::TEXT)
                .size()
                .x
                <= room
        })
    };
    // A page that is full takes no more: the key does nothing.
    let Some(new_caret) = open.write_page(number, &words, caret, fits) else {
        return;
    };
    if open.draft.pages[number].field != words {
        let mut state = output.state;
        let at = egui::text::CCursor::new(new_caret);
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::one(at)));
        state.store(ui.ctx(), key);
    }
}

/// The health of the mobile of a paperdoll.
fn health_bar(ui: &egui::Ui, area: Rect, frame: &WatchFrame, serial: u32) {
    if let Some(share) = doll_health(frame, serial) {
        theme::bar(ui.painter(), area, share, theme::HITS);
    }
}

impl PagesUi {
    fn board(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        board: &WatchBoard,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let list_height = BOARD_ROWS as f32 * BOARD_ROW;
        let live = frame.human_control;
        let spec = PanelSpec {
            id: BOARD_ID,
            title: &board.name,
            default: bridge::rect(board_first_place(bridge::area(rect))),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let inner = frame::draw(ui.painter(), panel, &board.name);
        let inner = Rect::from_min_max(inner.min - Vec2::new(0.0, COVER_ROW), inner.max);
        let rows = threaded(&board.posts);
        let last_first = rows.len().saturating_sub(BOARD_ROWS);
        self.first_post = scrolled(ui, panel, self.first_post, last_first);
        for (i, (post, depth)) in rows
            .iter()
            .skip(self.first_post)
            .take(BOARD_ROWS)
            .enumerate()
        {
            let indent = REPLY_INDENT * shown_depth(*depth) as f32;
            let row = Rect::from_min_size(
                inner.left_top() + Vec2::new(indent, COVER_ROW + i as f32 * BOARD_ROW),
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
                poster_words(post),
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
            Pos2::new(text_left, inner.top() + COVER_ROW),
            Pos2::new(
                inner.right(),
                inner.top() + COVER_ROW + list_height - write_height,
            ),
        );
        ui.painter()
            .rect_filled(paper, CornerRadius::same(CELL_RADIUS), PAPER_COLOR);
        let mut job = egui::text::LayoutJob::single_section(
            board_text(reading),
            egui::TextFormat::simple(text_font(theme::SIZE_BODY), INK_COLOR),
        );
        job.wrap.max_width = paper.width() - theme::PANEL_PAD * 2.0;
        let galley = ui.painter().layout_job(job);
        ui.painter().with_clip_rect(paper).galley(
            paper.left_top() + Vec2::splat(theme::PANEL_PAD / 2.0),
            galley,
            INK_COLOR,
        );
        let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if !live {
            return panel;
        }
        let subject_row = Rect::from_min_size(
            Pos2::new(text_left, paper.bottom() + theme::ROW_GAP),
            Vec2::new(BOARD_TEXT_WIDTH, BOARD_ROW - theme::ROW_GAP * 2.0),
        );
        let text_box = Rect::from_min_max(
            Pos2::new(text_left, subject_row.bottom() + theme::ROW_GAP),
            Pos2::new(inner.right(), inner.top() + COVER_ROW + list_height),
        );
        for field in [subject_row, text_box] {
            ui.painter()
                .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        }
        ui.put(
            subject_row,
            egui::TextEdit::singleline(&mut self.board.subject)
                .frame(false)
                .hint_text(HINT_SUBJECT)
                .font(text_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        ui.put(
            text_box,
            egui::TextEdit::multiline(&mut self.board.text)
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
        let (_, close_pressed) = theme::button(ui, next, WORDS_CLOSE, theme::TEXT_DIM);
        let press = [
            (posted, BoardPress::Post),
            (replied, BoardPress::Reply),
            (removed, BoardPress::Remove),
            (close_pressed || closed, BoardPress::Close),
        ]
        .into_iter()
        .find_map(|(pressed, press)| pressed.then_some(press));
        if let Some(act) =
            press.and_then(|press| self.board.press(press, reading.map(|post| post.serial)))
        {
            tools.hand.act(act);
        }
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::modern::testing::draw_frames;

    const BOOK: u32 = 0x4000_0B00;

    fn book_frame(writable: bool) -> WatchFrame {
        WatchFrame {
            human_control: true,
            book: Some(WatchBook {
                serial: BOOK,
                title: "Tales".into(),
                author: "Ann".into(),
                page_count: 5,
                pages: vec![vec!["Once".into(), "upon".into()]],
                arrived: vec![true],
                writable,
            }),
            ..WatchFrame::default()
        }
    }

    fn draw(pages: &mut PagesUi, frame: &WatchFrame, classic: bool) -> Vec<Rect> {
        let mut profile = Profile::default();
        let mut covered = Vec::new();
        draw_frames(&mut profile, &[Vec::new()], |ui, rect, tools, profile| {
            covered = pages.draw(ui, rect, frame, tools, profile, classic, true);
        });
        covered
    }

    #[test]
    fn a_book_shows_while_the_shard_has_it_open() {
        let mut pages = PagesUi::default();
        assert_eq!(draw(&mut pages, &book_frame(true), false).len(), 1);
        assert!(pages.book.is_some());
        assert!(draw(&mut pages, &WatchFrame::default(), false).is_empty());
        assert!(pages.book.is_none());
    }

    #[test]
    fn the_classic_style_draws_none_of_these() {
        let mut pages = PagesUi::default();
        assert!(draw(&mut pages, &book_frame(false), true).is_empty());
        assert!(pages.book.is_none());
    }
}
