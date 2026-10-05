//! The spells tab of the sheet: one book for each spellbook the shard
//! told of, of any school, with only the spells it holds. A click on a
//! spell shows what it is: its icon, its words of power, its circle or
//! group, its reagents, and the mana, skill and tithing it asks for. A
//! double click, or Cast, casts it from that book, as a click in the
//! classic book does. A spell drags or pins onto the hotbar with its icon.
//! With "Fast spell assign" Ctrl+Alt and a click makes a macro of it.

use super::super::boxes_ui::{scrolled, Tools, CELL_RADIUS};
use super::super::control::Act;
use super::super::deck_ui::{Offer, Slot, ROW, TAB_GAP};
use super::super::model::key_macros;
use super::super::model::spell_data::{
    book_info, book_name, icon_hue, needs_words, power_words, reagent_lines, spell_macro, upkeep,
    BookInfo, BOOKS,
};
use super::super::settings::Profile;
use super::super::theme::{self, text_font, title_font};
use crate::view::{WatchFrame, WatchSpellbook};
use crate::window::bridge;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Vec2};
use uoterm_assist::spells::School;
use uoterm_view::ui::lists::{spell_book_words, SpellTab};
use uoterm_view::ui::sheet::{
    assigned_words, assigns_spell, HINT_ASSIGN, HINT_SPELL, WORDS_ASSIGN, WORDS_CAST,
    WORDS_EMPTY_BOOK, WORDS_NO_BOOK, WORDS_PICK_SPELL as WORDS_PICK, WORDS_PIN, WORDS_REAGENTS,
    WORDS_TITHING_COST, WORDS_TITHING_HAVE,
};

const LIST_WIDTH: f32 = 180.0;
const ICON_SIDE: f32 = 44.0;
const BOOK_BUTTON_WIDTH: f32 = 96.0;
const DETAIL_BUTTON_WIDTH: f32 = 72.0;
const LINE: f32 = 16.0;

#[derive(Default)]
pub struct SpellsTab {
    /// The book the tab shows.
    book: SpellTab,
    /// The spell the player clicked.
    selected: Option<u16>,
    first_row: usize,
}

impl SpellsTab {
    /// Shows the book of a school. False when the character has none yet;
    /// the tab turns to one when the shard tells of it.
    pub fn choose_school(&mut self, frame: &WatchFrame, school: School) -> bool {
        let chosen = self.book.choose_school(frame, school);
        if chosen {
            self.first_row = 0;
        }
        chosen
    }

    /// The school of the book the tab shows.
    pub fn school(&self, frame: &WatchFrame) -> Option<School> {
        self.book.school(frame)
    }

    /// Draws the tab in `body`. Gives what the player pinned or dragged
    /// toward the hotbar.
    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        body: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Option<Offer> {
        if self.book.follow(frame) {
            self.first_row = 0;
        }
        let books: Vec<&WatchSpellbook> = frame.spellbooks.iter().collect();
        let Some(contents) = self.book.shown(frame) else {
            self.no_book(ui, body, frame, tools);
            return None;
        };
        let bar = Rect::from_min_size(body.min, Vec2::new(body.width(), ROW));
        for at in 0..books.len() {
            let area = Rect::from_min_size(
                bar.left_top() + Vec2::new(at as f32 * (BOOK_BUTTON_WIDTH + TAB_GAP), 0.0),
                Vec2::new(BOOK_BUTTON_WIDTH, ROW - TAB_GAP),
            );
            if area.right() > bar.right() {
                break;
            }
            let color = if books[at].serial == contents.serial {
                theme::GOAL
            } else {
                theme::TEXT_DIM
            };
            let words = spell_book_words(&books, at);
            if theme::segment_keyed(
                ui,
                area,
                Id::new(("spell-book", books[at].serial)),
                &words,
                color,
            ) {
                self.book.book = Some(books[at].serial);
                self.first_row = 0;
            }
        }
        let book = book_info(&contents.school, contents.graphic);
        let held = book.held_places(Some(contents));
        let rest = Rect::from_min_max(Pos2::new(body.left(), bar.bottom()), body.max);
        let list = Rect::from_min_max(rest.min, Pos2::new(rest.left() + LIST_WIDTH, rest.bottom()));
        let detail = Rect::from_min_max(
            Pos2::new(list.right() + theme::ROW_GAP * 2.0, rest.top()),
            rest.max,
        );
        if held.is_empty() {
            ui.painter().text(
                list.left_top(),
                Align2::LEFT_TOP,
                WORDS_EMPTY_BOOK,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            return None;
        }
        let offer = self.list(
            ui,
            list,
            frame,
            tools,
            profile,
            (book, contents.serial),
            &held,
        );
        let place = self.selected.and_then(|spell| {
            held.iter()
                .copied()
                .find(|place| book.spells[*place].id == spell)
        });
        match place {
            Some(place) => {
                offer.or(self.detail(ui, detail, frame, tools, (book, contents.serial), place))
            }
            None => {
                ui.painter().text(
                    detail.left_top(),
                    Align2::LEFT_TOP,
                    WORDS_PICK,
                    text_font(theme::SIZE_SMALL),
                    theme::TEXT_FAINT,
                );
                offer
            }
        }
    }

    /// Without a book, the buttons that ask the shard to open one.
    fn no_book(&self, ui: &egui::Ui, body: Rect, frame: &WatchFrame, tools: &Tools<'_>) {
        ui.painter().text(
            body.left_top(),
            Align2::LEFT_TOP,
            WORDS_NO_BOOK,
            text_font(theme::SIZE_SMALL),
            theme::TEXT_FAINT,
        );
        if !frame.human_control {
            return;
        }
        let openable = BOOKS.iter().filter(|book| book.school != School::Mastery);
        let columns = ((body.width() + TAB_GAP) / (BOOK_BUTTON_WIDTH + TAB_GAP))
            .floor()
            .max(1.0) as usize;
        for (at, book) in openable.enumerate() {
            let area = Rect::from_min_size(
                body.left_top()
                    + Vec2::new(
                        (at % columns) as f32 * (BOOK_BUTTON_WIDTH + TAB_GAP),
                        ROW * (1 + at / columns) as f32,
                    ),
                Vec2::new(BOOK_BUTTON_WIDTH, ROW - TAB_GAP),
            );
            if theme::segment_keyed(
                ui,
                area,
                Id::new(("open-book", book.name)),
                book.title,
                theme::GOAL,
            ) {
                tools.hand.act(Act::OpenSpellbook(book.name));
            }
        }
    }

    /// The spells the book holds: a click reads one, a double click casts
    /// it, a drag carries it to the hotbar.
    #[allow(clippy::too_many_arguments)]
    fn list(
        &mut self,
        ui: &egui::Ui,
        list: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
        (book, serial): (&BookInfo, u32),
        held: &[usize],
    ) -> Option<Offer> {
        let rows = ((list.height() / ROW).floor() as usize).max(1);
        let last_first = held.len().saturating_sub(rows);
        self.first_row = scrolled(ui, list, self.first_row.min(last_first), last_first);
        let live = frame.human_control;
        let mods = bridge::mods(ui.input(|i| i.modifiers));
        let assigning = assigns_spell(profile.combat.fast_spell_assign, mods);
        let mut offer = None;
        for (at, place) in held.iter().skip(self.first_row).take(rows).enumerate() {
            let spell = &book.spells[*place];
            let row = Rect::from_min_size(
                list.left_top() + Vec2::new(0.0, at as f32 * ROW),
                Vec2::new(list.width(), ROW - TAB_GAP / 2.0),
            );
            let response = ui.interact(
                row,
                Id::new(("spell-row", serial, spell.id)),
                Sense::click_and_drag(),
            );
            let fill = if self.selected == Some(spell.id) {
                theme::BUTTON_HOVER
            } else if response.hovered() {
                theme::BUTTON
            } else {
                Color32::TRANSPARENT
            };
            ui.painter()
                .rect_filled(row, CornerRadius::same(CELL_RADIUS), fill);
            let icon = Rect::from_min_size(row.min, Vec2::splat(row.height()));
            if let Some((texture, sprite)) = tools
                .scene
                .gump_picture(spell.small_icon, icon_hue(frame, spell.id))
            {
                ui.painter().image(
                    texture,
                    theme::fit(icon, sprite.width, sprite.height),
                    bridge::rect(sprite.uv),
                    Color32::WHITE,
                );
            }
            if assigning {
                ui.painter().text(
                    icon.right_top(),
                    Align2::RIGHT_TOP,
                    WORDS_ASSIGN,
                    title_font(theme::SIZE_SMALL),
                    theme::WAITING,
                );
            }
            let name = book_name(spell.id);
            ui.painter().text(
                Pos2::new(icon.right() + theme::ROW_GAP, row.center().y),
                Align2::LEFT_CENTER,
                &name,
                text_font(theme::SIZE_SMALL),
                theme::TEXT,
            );
            if !live {
                if response.clicked() {
                    self.selected = Some(spell.id);
                }
                continue;
            }
            if response.hovered() && !response.dragged() {
                let footer = if profile.combat.fast_spell_assign {
                    HINT_ASSIGN
                } else {
                    HINT_SPELL
                };
                super::super::tips::label(ui, &name, footer);
            }
            if response.drag_started() {
                offer = Some(Offer::Drag(Slot::Spell { id: spell.id, name }));
            } else if response.double_clicked() {
                tools.hand.act(Act::CastFrom {
                    spell: spell.id,
                    book: serial,
                });
            } else if response.clicked() && assigning {
                let (name, steps) = spell_macro(spell.id);
                if key_macros::ensure(&mut profile.macros.key_bindings, &name, steps) {
                    tools.keep_profile(profile);
                }
                tools.hand.report(&assigned_words(&name));
            } else if response.clicked() {
                self.selected = Some(spell.id);
            }
        }
        offer
    }

    /// What the selected spell is, with Cast and Pin.
    fn detail(
        &self,
        ui: &egui::Ui,
        detail: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        (book, serial): (&BookInfo, u32),
        place: usize,
    ) -> Option<Offer> {
        let spell = &book.spells[place];
        let painter = ui.painter();
        let icon = Rect::from_min_size(detail.min, Vec2::splat(ICON_SIDE));
        painter.rect_filled(icon, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        if let Some((texture, sprite)) = tools
            .scene
            .gump_picture(book.icon(place), icon_hue(frame, spell.id))
        {
            painter.image(
                texture,
                theme::fit(icon, sprite.width, sprite.height),
                bridge::rect(sprite.uv),
                Color32::WHITE,
            );
        }
        let name = book_name(spell.id);
        let beside = icon.right() + theme::ROW_GAP;
        painter.text(
            Pos2::new(beside, icon.top()),
            Align2::LEFT_TOP,
            &name,
            title_font(theme::SIZE_BODY),
            theme::TEXT,
        );
        if let Some(group) = book.spell_group(place) {
            painter.text(
                Pos2::new(beside, icon.center().y),
                Align2::LEFT_TOP,
                group,
                text_font(theme::SIZE_SMALL),
                theme::TEXT_DIM,
            );
        }
        let mut lines: Vec<(String, FontId, Color32)> = Vec::new();
        let words = power_words(spell.id);
        if !words.is_empty() {
            lines.push((words, title_font(theme::SIZE_SMALL), theme::GOAL));
        }
        let reagents = reagent_lines(spell);
        if !reagents.is_empty() {
            lines.push((
                WORDS_REAGENTS.to_string(),
                text_font(theme::SIZE_SMALL),
                theme::TEXT_DIM,
            ));
            lines.extend(
                reagents
                    .lines()
                    .map(|line| (line.to_string(), text_font(theme::SIZE_SMALL), theme::TEXT)),
            );
        }
        if book.school != School::Magery {
            let needs = needs_words(spell.mana, spell.skill, upkeep(book, spell));
            lines.extend(
                needs
                    .lines()
                    .map(|line| (line.to_string(), text_font(theme::SIZE_SMALL), theme::TEXT)),
            );
        }
        if book.school == School::Chivalry {
            lines.push((
                format!("{WORDS_TITHING_COST}: {}", spell.tithing),
                text_font(theme::SIZE_SMALL),
                theme::TEXT,
            ));
            lines.push((
                format!("{WORDS_TITHING_HAVE}: {}", frame.status.tithing),
                text_font(theme::SIZE_SMALL),
                theme::TEXT_DIM,
            ));
        }
        let buttons_top = detail.bottom() - ROW;
        let mut y = icon.bottom() + theme::ROW_GAP;
        for (line, font, color) in lines {
            if y + LINE > buttons_top {
                break;
            }
            painter.text(
                Pos2::new(detail.left(), y),
                Align2::LEFT_TOP,
                line,
                font,
                color,
            );
            y += LINE;
        }
        if !frame.human_control {
            return None;
        }
        let cast = Rect::from_min_size(
            Pos2::new(detail.left(), buttons_top),
            Vec2::new(DETAIL_BUTTON_WIDTH, ROW - TAB_GAP),
        );
        let pin = cast.translate(Vec2::new(DETAIL_BUTTON_WIDTH + TAB_GAP, 0.0));
        if theme::segment_keyed(
            ui,
            cast,
            Id::new(("spell-cast", spell.id)),
            WORDS_CAST,
            theme::GOAL,
        ) {
            tools.hand.act(Act::CastFrom {
                spell: spell.id,
                book: serial,
            });
        }
        theme::segment_keyed(
            ui,
            pin,
            Id::new(("spell-pin", spell.id)),
            WORDS_PIN,
            theme::TEXT,
        )
        .then_some(Offer::Pin(Slot::Spell { id: spell.id, name }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(serial: u32, school: &str) -> WatchSpellbook {
        WatchSpellbook {
            serial,
            school: school.into(),
            ..WatchSpellbook::default()
        }
    }

    #[test]
    fn a_chosen_school_shows_its_book_from_the_top() {
        let frame = WatchFrame {
            spellbooks: vec![book(1, "magery"), book(2, "chivalry")],
            ..WatchFrame::default()
        };
        let mut tab = SpellsTab {
            first_row: 3,
            ..SpellsTab::default()
        };
        assert!(!tab.choose_school(&frame, School::Bushido));
        assert_eq!(tab.first_row, 3, "no book yet: the list stays");
        assert!(tab.choose_school(&frame, School::Chivalry));
        assert_eq!(tab.school(&frame), Some(School::Chivalry));
        assert_eq!(tab.first_row, 0);
    }

    #[test]
    fn a_click_reads_a_spell_of_the_book_and_every_part_draws() {
        use super::super::testing::{click, draw_frames};
        let frame = WatchFrame {
            human_control: true,
            spellbooks: vec![WatchSpellbook {
                serial: 7,
                school: "chivalry".into(),
                spells: vec![(201, String::new()), (203, String::new())],
                ..WatchSpellbook::default()
            }],
            ..WatchFrame::default()
        };
        let body = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(400.0, 300.0));
        let first_row = Pos2::new(body.left() + LIST_WIDTH / 2.0, body.top() + ROW + ROW / 3.0);
        let mut tab = SpellsTab::default();
        let mut profile = Profile::default();
        draw_frames(&mut profile, &click(first_row), |ui, _, tools, profile| {
            tab.draw(ui, body, &frame, tools, profile);
        });
        assert_eq!(tab.selected, Some(201));
        draw_frames(&mut profile, &[Vec::new()], |ui, _, tools, profile| {
            tab.draw(ui, body, &WatchFrame::default(), tools, profile);
        });
    }
}
