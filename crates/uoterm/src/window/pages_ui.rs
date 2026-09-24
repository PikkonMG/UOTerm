//! The windows the shard opens that are not gumps, in the Modern style: the
//! old-style menu with a question and a list of answers, the book, the
//! bulletin board, the paperdoll, the dialog for words the shard waits for,
//! the race change, the tip or notice, and the dye picker when the client
//! files have no gump art. Each one shows at all times, and the player
//! moves and locks it. The clicks work only while the human has control.
//! The Classic style shows each of these as a classic gump.

use super::boxes_ui::{ask_waiting_name, scrolled, single_or_double, Tools, CELL_RADIUS};
use super::control::Act;
use super::deck_ui::{is_worn_layer, layer_words};
use super::desk::Zone;
use super::model::clicks::ClickDelay;
use super::model::dolls::{self, DollWatch, GUILD_COMMAND, QUESTS_COMMAND};
use super::model::pages::{
    fitted, last_left, page_count, shown_depth, threaded, turned, BookDraft, PAGES_SHOWN,
};
use super::modern::frame::{self, FrameEvent, PanelSpec};
use super::modern::layout::{self, Spot};
use super::modern::{DyeUi, EntryUi, RaceUi, TipUi};
use super::settings::Profile;
use super::theme::{self, number_font, text_font};
use crate::view::{WatchBoard, WatchBook, WatchEquip, WatchFrame, WatchOldMenu, WatchPackItem};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Vec2};
use uoterm_protocol::BOOK_PAGE_LINE_MAX;

const MENU_ID: &str = "modern:old_menu";
const BOOK_ID: &str = "modern:book";
const BOARD_ID: &str = "modern:board";
const DOLL_ID: &str = "modern:paperdoll";

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
const BOOK_FIELD_WIDTH: f32 = 220.0;
/// The words of a page stand this far in from its edge.
const PAGE_MARGIN: f32 = 8.0;
const PAPER: Color32 = Color32::from_rgb(226, 214, 184);
const INK: Color32 = Color32::from_rgb(46, 36, 24);

const DOLL_PANEL_WIDTH: f32 = 420.0;
const DOLL_PICTURE: Vec2 = Vec2::new(140.0, 220.0);
const DOLL_ROW: f32 = 30.0;
const DOLL_ROWS: usize = 10;
const HEALTH_ROW: f32 = 18.0;
/// A mobile's hits are a share out of this.
const PERCENT: f32 = 100.0;

const WORDS_OUT_OF_SIGHT: &str = "Out of sight.";
const WORDS_WEARS_NOTHING: &str = "Wears nothing.";
const WORDS_STATUS: &str = "Status";
const WORDS_VIRTUES: &str = "Virtues";
const WORDS_QUESTS: &str = "Quests";
const WORDS_GUILD: &str = "Guild";
const HINT_WORN: &str = "Double-click: use.";
const HINT_WORN_LIFT: &str = "Double-click: use.  Drag: take it off.";

const WORDS_CANCEL: &str = "Cancel";
const WORDS_CLOSE: &str = "Close";
const WORDS_FIRST: &str = "First";
const WORDS_BACK: &str = "Back";
const WORDS_NEXT: &str = "Next";
const WORDS_LAST: &str = "Last";
const WORDS_SAVE: &str = "Save";
const WORDS_BY: &str = "by";
const HINT_TITLE: &str = "The title of the book";
const HINT_AUTHOR: &str = "The author";
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

/// The book that is open: the left page that shows, from zero, and what
/// the player wrote in it.
struct OpenBook {
    serial: u32,
    left: usize,
    draft: BookDraft<String>,
}

impl OpenBook {
    fn new(serial: u32) -> Self {
        Self {
            serial,
            left: 0,
            draft: BookDraft::new(String::new(), String::new(), String::new),
        }
    }
}

/// The paperdoll that shows: whose it is, the words the shard put at its
/// top, and whether the shard lets the character dress it.
struct ShownDoll {
    serial: u32,
    text: String,
    can_lift: bool,
}

#[derive(Default)]
pub struct PagesUi {
    first_entry: usize,
    book: Option<OpenBook>,
    first_post: usize,
    subject: String,
    text: String,
    /// The paperdolls the shard opened, and the one that shows.
    dolls: DollWatch,
    doll: Option<ShownDoll>,
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
        self.doll = None;
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
        let fresh_doll = self.dolls.take(frame).cloned();
        let mut covered = Vec::new();
        if classic {
            self.first_entry = 0;
            self.book = None;
            self.first_post = 0;
            self.doll = None;
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
        if let Some(doll) = fresh_doll {
            self.first_worn = 0;
            self.doll = Some(ShownDoll {
                serial: doll.serial,
                text: doll.text,
                can_lift: doll.can_lift,
            });
        }
        if self.doll.is_some() {
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
            default: layout::first_place(
                rect,
                Spot::Middle(0),
                Vec2::new(
                    DOLL_PANEL_WIDTH,
                    theme::PANEL_PAD * 2.0
                        + frame::TITLE_ROW
                        + DOLL_ROWS as f32 * DOLL_ROW
                        + FOOT_ROW,
                ),
            ),
            min_size: None,
            closable: true,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, &title);
        if dresses && live {
            let zone = if own { Zone::Wear } else { Zone::Into(serial) };
            tools.desk.zone(panel, zone);
        }
        let picture = Rect::from_min_size(body.min, DOLL_PICTURE);
        ui.painter()
            .rect_filled(picture, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let look = if own {
            Some(&frame.look)
        } else {
            frame
                .mobiles
                .iter()
                .find(|mobile| mobile.serial == serial)
                .map(|mobile| &mobile.look)
        };
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
                if let Some((texture, sprite)) = tools.scene.doll_picture(frame.map, look) {
                    let area = theme::fit(picture, sprite.width, sprite.height);
                    ui.painter().image(texture, area, sprite.uv, Color32::WHITE);
                }
                let list = Rect::from_min_max(
                    Pos2::new(picture.right() + theme::ROW_GAP * 2.0, body.top()),
                    Pos2::new(body.right(), body.bottom() - FOOT_ROW),
                );
                let worn: Vec<&WatchEquip> = look
                    .equipment
                    .iter()
                    .filter(|item| is_worn_layer(item.layer))
                    .collect();
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
            self.doll = None;
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
            if let Some((texture, sprite)) =
                tools.scene.item_picture(frame.map, item.graphic, item.hue)
            {
                let area = theme::fit(art, sprite.width, sprite.height);
                ui.painter().image(texture, area, sprite.uv, Color32::WHITE);
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
                let footer = match (live, dresses) {
                    (false, _) => "",
                    (true, true) => HINT_WORN_LIFT,
                    (true, false) => HINT_WORN,
                };
                tools
                    .tips
                    .point_at(ui, tools.hand, item.serial, "", footer, tools.time);
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
        let mut buttons = vec![
            (
                WORDS_STATUS,
                Act::MobileStatus {
                    serial,
                    close: false,
                },
            ),
            (WORDS_VIRTUES, Act::VirtueGump(serial)),
        ];
        if own {
            buttons.push((WORDS_QUESTS, Act::Command(QUESTS_COMMAND.into())));
            buttons.push((WORDS_GUILD, Act::Command(GUILD_COMMAND.into())));
        }
        let mut at = foot;
        let mut pressed = None;
        for (words, act) in buttons {
            let (area, clicked) = theme::button(ui, at, words, theme::TEXT);
            at = Pos2::new(area.right() + theme::ROW_GAP, foot.y);
            if clicked {
                pressed = Some(act);
            }
        }
        let (_, closed) = theme::button(ui, at, WORDS_CLOSE, theme::TEXT_DIM);
        if closed {
            self.doll = None;
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
        let rows = menu.entries.len().clamp(1, MENU_ROWS);
        let live = frame.human_control;
        let spec = PanelSpec {
            id: MENU_ID,
            title: &menu.question,
            default: layout::first_place(
                rect,
                Spot::Middle(0),
                Vec2::new(
                    MENU_WIDTH,
                    theme::PANEL_PAD * 2.0 + TITLE_ROW + rows as f32 * MENU_ROW + FOOT_ROW,
                ),
            ),
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
        let mut canceled = false;
        if live {
            let foot = Pos2::new(body.left(), body.bottom() - FOOT_ROW + theme::ROW_GAP);
            (_, canceled) = theme::button(ui, foot, WORDS_CANCEL, theme::TEXT_DIM);
        }
        let closed = frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed);
        if live && (canceled || closed) {
            tools.hand.act(Act::OldMenuPick(None));
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
        if self
            .book
            .as_ref()
            .is_none_or(|open| open.serial != book.serial)
        {
            self.book = Some(OpenBook::new(book.serial));
        }
        let Some(open) = self.book.as_mut() else {
            return Rect::NOTHING;
        };
        open.draft.take_shard_words(book);
        let live = frame.human_control;
        let writing = live && book.writable;
        let count = page_count(book);
        let paper_height = BOOK_LINES as f32 * BOOK_LINE;
        let spec = PanelSpec {
            id: BOOK_ID,
            title: &book.title,
            default: layout::first_place(
                rect,
                Spot::Middle(0),
                Vec2::new(
                    BOOK_PAGE_WIDTH * PAGES_SHOWN as f32 + BOOK_GUTTER + theme::PANEL_PAD * 2.0,
                    frame::TITLE_ROW + TITLE_ROW + paper_height + FOOT_ROW + theme::PANEL_PAD * 2.0,
                ),
            ),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, &book.title);
        let cover_row = Rect::from_min_size(body.left_top(), Vec2::new(body.width(), TITLE_ROW));
        cover(ui, cover_row, &mut open.draft, writing, book.serial);
        let mut acts = Vec::new();
        if live && !book.writable {
            let numbers = open.left + 1..open.left + 1 + PAGES_SHOWN;
            acts.extend(open.draft.ask_missing(book, numbers));
        }
        for side in 0..PAGES_SHOWN {
            let number = open.left + side;
            let paper = Rect::from_min_size(
                body.left_top()
                    + Vec2::new(side as f32 * (BOOK_PAGE_WIDTH + BOOK_GUTTER), TITLE_ROW),
                Vec2::new(BOOK_PAGE_WIDTH, paper_height),
            );
            let fill = if writing { theme::TRACK } else { PAPER };
            ui.painter()
                .rect_filled(paper, CornerRadius::same(CELL_RADIUS), fill);
            let Some(page) = open.draft.pages.get_mut(number) else {
                continue;
            };
            if writing {
                let key = Id::new(("book-page", book.serial, number));
                page.changed |= page_edit(ui, paper, key, &mut page.field);
            } else {
                page_words(ui, paper, &page.field);
            }
            ui.painter().text(
                paper.center_bottom() - Vec2::new(0.0, theme::ROW_GAP),
                Align2::CENTER_BOTTOM,
                (number + 1).to_string(),
                number_font(theme::SIZE_SMALL),
                if writing { theme::TEXT_DIM } else { INK },
            );
        }
        let foot = Pos2::new(body.left(), body.bottom() - FOOT_ROW + theme::ROW_GAP);
        let mut at = foot;
        let mut to = open.left;
        for (words, target) in [
            (WORDS_FIRST, 0),
            (WORDS_BACK, turned(open.left, false, count)),
            (WORDS_NEXT, turned(open.left, true, count)),
            (WORDS_LAST, last_left(count)),
        ] {
            let (area, pressed) = theme::button(ui, at, words, theme::TEXT);
            at = Pos2::new(area.right() + theme::ROW_GAP, foot.y);
            if pressed {
                to = target;
            }
        }
        if to != open.left {
            open.left = to;
            if writing {
                acts.extend(open.draft.written());
            }
        }
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
            format!("{WORDS_BY} {}", draft.author),
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
            INK,
        );
    }
}

/// The field of a page the player writes. A line too wide for the page
/// breaks, and a page that is full takes no more. True when the words
/// changed.
fn page_edit(ui: &mut egui::Ui, paper: Rect, key: Id, words: &mut String) -> bool {
    let before = words.clone();
    let output = ui
        .scope_builder(egui::UiBuilder::new().max_rect(paper), |ui| {
            egui::TextEdit::multiline(words)
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
        return false;
    }
    let caret = output
        .cursor_range
        .map_or(words.chars().count(), |range| range.primary.ccursor.index);
    let room = paper.width() - PAGE_MARGIN * 2.0;
    let fits = |line: &str| {
        ui.fonts(|fonts| {
            fonts
                .layout_no_wrap(line.to_string(), text_font(theme::SIZE_BODY), theme::TEXT)
                .size()
                .x
                <= room
        })
    };
    match fitted(words, caret, fits, BOOK_PAGE_LINE_MAX) {
        Some((kept, new_caret)) => {
            if kept != *words {
                *words = kept;
                let mut state = output.state;
                let at = egui::text::CCursor::new(new_caret);
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(at)));
                state.store(ui.ctx(), key);
            }
            true
        }
        // The page is full: the key does nothing.
        None => {
            *words = before;
            false
        }
    }
}

/// The health of the mobile of a paperdoll: the character's hits, or the
/// share of hits the shard told of another.
fn health_bar(ui: &egui::Ui, area: Rect, frame: &WatchFrame, serial: u32) {
    let share = if serial == frame.serial {
        (frame.hits_max > 0).then(|| f32::from(frame.hits) / f32::from(frame.hits_max))
    } else {
        frame
            .mobiles
            .iter()
            .find(|mobile| mobile.serial == serial)
            .and_then(|mobile| mobile.hits_percent)
            .map(|percent| f32::from(percent) / PERCENT)
    };
    let Some(share) = share else {
        return;
    };
    theme::bar(ui.painter(), area, share, theme::HITS);
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
            default: layout::first_place(
                rect,
                Spot::Middle(0),
                Vec2::new(
                    BOARD_LIST_WIDTH + BOOK_GUTTER + BOARD_TEXT_WIDTH + theme::PANEL_PAD * 2.0,
                    TITLE_ROW + list_height + FOOT_ROW + theme::PANEL_PAD * 2.0,
                ),
            ),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let inner = frame::draw(ui.painter(), panel, &board.name);
        let inner = Rect::from_min_max(inner.min - Vec2::new(0.0, TITLE_ROW), inner.max);
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
        let (_, close_pressed) = theme::button(ui, next, WORDS_CLOSE, theme::TEXT_DIM);
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
        } else if close_pressed || closed {
            tools.hand.act(Act::BoardClose);
        }
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchPaperdoll;
    use crate::window::modern::testing::draw_frames;

    const BOOK: u32 = 0x4000_0B00;
    const STRANGER: u32 = 0x0000_0A11;

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
    fn a_book_takes_the_words_of_the_shard_and_goes_with_the_book() {
        let mut pages = PagesUi::default();
        assert_eq!(draw(&mut pages, &book_frame(true), false).len(), 1);
        let open = pages.book.as_ref().expect("the book is open");
        assert_eq!(open.draft.pages.len(), 5);
        assert_eq!(open.draft.pages[0].field, "Once\nupon");
        assert_eq!(open.draft.author, "Ann");
        assert!(draw(&mut pages, &WatchFrame::default(), false).is_empty());
        assert!(pages.book.is_none());
    }

    #[test]
    fn the_classic_style_draws_none_of_these() {
        let mut pages = PagesUi::default();
        assert!(draw(&mut pages, &book_frame(false), true).is_empty());
        assert!(pages.book.is_none());
    }

    #[test]
    fn a_paperdoll_the_shard_sends_shows_until_closed() {
        let mut pages = PagesUi::default();
        let mut frame = WatchFrame {
            paperdoll: Some(WatchPaperdoll {
                serial: STRANGER,
                text: "Someone the Brave".into(),
                seq: 1,
                can_lift: true,
            }),
            ..WatchFrame::default()
        };
        assert!(draw(&mut pages, &frame, false).is_empty(), "an old doll");
        frame.paperdoll = Some(WatchPaperdoll {
            seq: 2,
            ..frame.paperdoll.clone().expect("a doll")
        });
        assert_eq!(draw(&mut pages, &frame, false).len(), 1);
        let doll = pages.doll.as_ref().expect("the doll shows");
        assert!(doll.can_lift && doll.serial == STRANGER);
        pages.close_doll();
        assert!(
            draw(&mut pages, &frame, false).is_empty(),
            "closed with the rest"
        );
    }
}
