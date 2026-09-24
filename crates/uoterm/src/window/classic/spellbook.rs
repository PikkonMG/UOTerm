//! The spellbooks of the classic client, one gump for each book the shard opens, of every school: magery with its
//! circles, necromancy, chivalry, bushido, ninjitsu, spellweaving,
//! mysticism and the masteries. The index pages list the spells the book
//! holds; a click on a name turns to its page, a double click casts it.
//! Each spell page shows its icon, its words, and its reagents or the mana
//! and skill it needs. A double click on an icon casts the spell from this
//! book; dragging an icon out makes a spell button on the desktop. The page
//! corners turn the pages, a double click on one goes to the first or last
//! page, and the book folds into its small picture. With "Fast spell
//! assign" Ctrl+Alt and a click on an icon makes a macro of the spell.

use super::canvas::{ButtonArt, Canvas};
use super::item_control::{now, ClickDelay};
use super::registry::{Closing, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::spell_button::SPELL_BUTTON;
use super::text::TextLook;
use crate::view::{WatchFrame, WatchSpellbook};
use crate::window::control::Act;
use crate::window::model::spell_data::{
    active_masteries, book_info, book_name, icon_hue, needs_words, power_words, reagent_lines,
    spell_macro, upkeep, words_letters, BookInfo, CIRCLE_NAMES, MASTERY_INDEX_PAGES,
};
use eframe::egui::{Pos2, Vec2};
use uoterm_assist::spells::School;

/// The id of the gump of one spellbook, by the serial of the book.
const SPELLBOOK_ID: &str = "spellbook";

pub const SPELLBOOK: GumpKind = GumpKind {
    id: SPELLBOOK_ID,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(Spellbook::new(serial.unwrap_or_default())),
};

const PAGE_SOUND: u16 = 0x0055;
const TEXT_HUE: u16 = 0x0288;
const HOVER_HUE: u16 = 0x0033;
const SMALL_FONT: u8 = 6;
const LETTERS_FONT: u8 = 8;
const LIST_FONT: u8 = 9;
const CORNER_LEFT: u16 = 0x08BB;
const CORNER_RIGHT: u16 = 0x08BC;
const CORNER_LEFT_AT: (i32, i32) = (50, 8);
const CORNER_RIGHT_AT: (i32, i32) = (321, 8);
const MINIMIZE_AT: (i32, i32, i32, i32) = (0, 98, 27, 23);
/// The buttons of the circles of magery: the picture and its place, two
/// for each page of the index.
const CIRCLE_BUTTONS: [(u16, i32); 8] = [
    (0x08B1, 58),
    (0x08B2, 93),
    (0x08B3, 130),
    (0x08B4, 164),
    (0x08B5, 227),
    (0x08B6, 260),
    (0x08B7, 297),
    (0x08B8, 332),
];
const CIRCLE_BUTTONS_Y: i32 = 175;
// The index pages.
const INDEX_X: [i32; 2] = [106, 269];
const DATA_X: [i32; 2] = [62, 225];
const INDEX_Y: i32 = 10;
const HEADING_Y: i32 = 30;
const LIST_Y: i32 = 52;
const LIST_ROW: i32 = 15;
const LIST_WIDTH: u32 = 130;
const TITHING_AT: (i32, i32) = (62, 162);
const ABILITY_X: i32 = 225;
const ABILITY_Y: i32 = 55;
const ABILITY_ROW: i32 = 44;
const ABILITY_TEXT_GAP: i32 = 48;
const ABILITY_TEXT_LIFT: i32 = 2;
// The spell pages: the left side and the right side.
const ICON_X: [i32; 2] = [62, 225];
const TOP_TEXT_X: [i32; 2] = [87, 224];
const ICON_TEXT_X: [i32; 2] = [112, 275];
const TOP_TEXT_Y: i32 = 6;
const CIRCLE_TEXT_LIFT: i32 = 4;
const ICON_Y: i32 = 40;
const NAME_Y: i32 = 34;
const NAME_WIDTH: u32 = 80;
const TALL_NAME: i32 = 24;
const LETTERS_Y_SHORT: i32 = 31;
const LETTERS_Y_TALL: i32 = 26;
const REAGENT_RULE: u16 = 0x0835;
const REAGENT_RULE_Y: i32 = 88;
const REAGENT_RULE_SIZE: (i32, i32) = (120, 5);
const REAGENTS_Y: i32 = 92;
const REAGENT_LIST_Y: i32 = 114;
const NEEDS_Y: i32 = 162;
const UPKEEP_NEEDS_Y: i32 = 148;
/// The mark of "Fast spell assign" on an icon with Ctrl+Alt down.
const ASSIGN_MARK: u16 = 0x09CF;
const ASSIGN_HUE: u16 = 0x0044;
const ASSIGN_HOVER_HUE: u16 = 34;
/// A spell button opens with the mouse at its middle.
pub const BUTTON_HALF: f32 = 22.0;
const WORDS_INDEX: &str = "INDEX";
const WORDS_ABILITIES: &str = "Abilities";
const WORDS_PASSIVE: &str = "Passive";
const WORDS_ACTIVATED: &str = "Activated";
const WORDS_REAGENTS: &str = "Reagents:";
const WORDS_TITHING: &str = "Tithing points\nAvailable: ";
const HALF: usize = 2;
const FIRST_PAGE: usize = 1;

fn text(font: u8) -> TextLook {
    TextLook::ascii(font, TEXT_HUE)
}

/// The layout of one book: which spells it holds and on which pages.
struct Layout {
    /// The places in the school of the spells the book holds, in order.
    owned: Vec<usize>,
    /// The pages of the index.
    index_pages: usize,
    last_page: usize,
}

impl Layout {
    fn of(book: &BookInfo, contents: Option<&WatchSpellbook>) -> Self {
        let owned = book.held_places(contents);
        let index_pages = match book.school {
            School::Mastery => book.index_pages(),
            _ => book.index_pages() / HALF,
        };
        let last_page = index_pages + owned.len().div_ceil(HALF);
        Self {
            owned,
            index_pages,
            last_page: last_page.max(FIRST_PAGE),
        }
    }

    /// The page and the side of the spell at a place in the school.
    fn page_of(&self, place: usize) -> Option<(usize, usize)> {
        let rank = self.owned.iter().position(|owned| *owned == place)?;
        Some((self.index_pages + rank / HALF + 1, rank % HALF))
    }
}

pub struct Spellbook {
    serial: u32,
    page: usize,
    minimized: bool,
    sounded: bool,
    /// A click on a name turns to its page once no double click follows.
    page_clicks: ClickDelay,
    /// The spell whose icon the player drags onto the desktop.
    dragging: Option<u16>,
}

impl Spellbook {
    pub fn new(serial: u32) -> Self {
        Self {
            serial,
            page: FIRST_PAGE,
            minimized: false,
            sounded: false,
            page_clicks: ClickDelay::default(),
            dragging: None,
        }
    }

    fn turn_to(&mut self, cx: &mut GumpContext<'_>, page: usize, last: usize) {
        let page = page.clamp(FIRST_PAGE, last);
        if page != self.page {
            self.page = page;
            cx.play_sound(PAGE_SOUND);
        }
    }

    fn cast(&self, cx: &GumpContext<'_>, spell: u16) {
        cx.act(Act::CastFrom {
            spell,
            book: self.serial,
        });
    }

    /// One name of the index: a click turns to its page, a double click
    /// casts it.
    fn index_name(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        (x, y): (i32, i32),
        spell: u16,
        page: usize,
    ) {
        let name = book_name(spell);
        let look = text(LIST_FONT).cropped(LIST_WIDTH);
        let size = g.measure(&name, &look);
        let (w, h) = (size.x as i32, size.y as i32);
        let hue = if g.hovered(x, y, w, h) {
            HOVER_HUE
        } else {
            TEXT_HUE
        };
        g.label(x, y, &name, &TextLook { hue, ..look });
        let response = g.click_area(("name", spell), x, y, w, h);
        if response.double_clicked() {
            self.page_clicks.double_clicked();
            self.cast(cx, spell);
        } else if response.clicked() {
            self.page_clicks.clicked(page as u32, now(g));
        }
    }

    fn index(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        book: &BookInfo,
        layout: &Layout,
    ) {
        let page = self.page;
        if book.school == School::Magery {
            for (at, (picture, x)) in CIRCLE_BUTTONS.iter().enumerate() {
                let art = ButtonArt::new(*picture, *picture, 0);
                if g.button(("circle", at), *x, CIRCLE_BUTTONS_Y, art) {
                    self.turn_to(cx, at / HALF + 1, layout.last_page);
                }
            }
        }
        if page > layout.index_pages {
            return;
        }
        if page == FIRST_PAGE && book.school == School::Chivalry {
            let words = format!("{WORDS_TITHING}{}", cx.frame.status.tithing);
            g.label(TITHING_AT.0, TITHING_AT.1, &words, &text(SMALL_FONT));
        }
        let per_page = book.spells_on_page();
        for side in 0..HALF {
            g.label(INDEX_X[side], INDEX_Y, WORDS_INDEX, &text(SMALL_FONT));
            let data_x = DATA_X[side];
            if book.school == School::Mastery && side == 1 {
                self.abilities(g, cx, book);
                continue;
            }
            let heading = match book.school {
                School::Magery => CIRCLE_NAMES.get((page - 1) * HALF + side).copied(),
                School::Mastery if page == layout.index_pages => Some(WORDS_PASSIVE),
                School::Mastery => Some(WORDS_ACTIVATED),
                _ => None,
            };
            if let Some(heading) = heading {
                g.label(data_x, HEADING_Y, heading, &text(SMALL_FONT));
            }
            let places: Vec<usize> = match book.school {
                School::Mastery => MASTERY_INDEX_PAGES
                    .get(page - 1)
                    .map(|places| places.iter().map(|place| place - 1).collect())
                    .unwrap_or_default(),
                _ => {
                    let first = ((page - 1) * HALF + side) * per_page;
                    (first..first + per_page).collect()
                }
            };
            let mut y = LIST_Y;
            for place in places {
                let (Some((spell_page, _)), Some(spell)) =
                    (layout.page_of(place), book.spell_at(place))
                else {
                    continue;
                };
                self.index_name(g, cx, (data_x, y), spell.id, spell_page);
                y += LIST_ROW;
            }
        }
    }

    /// The right side of the first index page of a book of masteries: the
    /// masteries of its active one, by the properties of the book.
    fn abilities(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, book: &BookInfo) {
        g.label(DATA_X[1], HEADING_Y, WORDS_ABILITIES, &text(SMALL_FONT));
        let lines: Vec<String> = cx
            .tips
            .lines_of(cx.hand, self.serial)
            .map(<[String]>::to_vec)
            .unwrap_or_default();
        let places = active_masteries(|cliloc| {
            let words = g.words(cliloc, "");
            !words.is_empty() && lines.iter().any(|line| line.eq_ignore_ascii_case(&words))
        });
        for (row, place) in places.iter().enumerate() {
            let Some(spell) = book.spell_at(place - 1) else {
                continue;
            };
            let y = ABILITY_Y + ABILITY_ROW * row as i32;
            self.icon(g, cx, (ABILITY_X, y), spell.small_icon, spell.id, None);
            g.label(
                ABILITY_X + ABILITY_TEXT_GAP,
                y + ABILITY_TEXT_LIFT,
                &book_name(spell.id),
                &text(SMALL_FONT).wrap(NAME_WIDTH),
            );
        }
    }

    /// An icon of a spell: a double click casts it, a drag makes a spell
    /// button, and with "Fast spell assign" Ctrl+Alt and a click makes a
    /// macro of it.
    fn icon(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        (x, y): (i32, i32),
        picture: u16,
        spell: u16,
        tooltip: Option<u32>,
    ) {
        let response = g.pic_button(("icon", spell), x, y, picture, icon_hue(cx.frame, spell));
        if let Some(cliloc) = tooltip {
            let words = g.words(cliloc, &book_name(spell));
            g.tooltip(&words);
        }
        let assigning = cx.profile.combat.fast_spell_assign
            && g.ui().input(|i| i.modifiers.ctrl && i.modifiers.alt);
        if assigning {
            let size = g.gump_size(picture).unwrap_or(Vec2::ZERO);
            let mark = g.gump_size(ASSIGN_MARK).unwrap_or(Vec2::ZERO);
            let mark_hue = if response.hovered() {
                ASSIGN_HOVER_HUE
            } else {
                ASSIGN_HUE
            };
            g.pic(x + (size.x - mark.x) as i32, y, ASSIGN_MARK, mark_hue);
        }
        if response.drag_started() {
            self.dragging = Some(spell);
        } else if response.double_clicked() {
            self.cast(cx, spell);
        } else if response.clicked() && assigning {
            assign_macro(cx, spell);
        }
    }

    fn spell_pages(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        book: &BookInfo,
        layout: &Layout,
    ) {
        for &place in &layout.owned {
            let (Some((page, side)), Some(spell)) = (layout.page_of(place), book.spell_at(place))
            else {
                continue;
            };
            if page != self.page {
                continue;
            }
            let (icon_x, top_x, text_x) = (ICON_X[side], TOP_TEXT_X[side], ICON_TEXT_X[side]);
            let name = book_name(spell.id);
            let words = power_words(spell.id);
            let letters_y = |name_height: i32| {
                let start = if name_height < TALL_NAME {
                    LETTERS_Y_SHORT
                } else {
                    LETTERS_Y_TALL
                };
                start + name_height
            };
            if let Some(group) = book.spell_group(place) {
                g.label(
                    top_x,
                    TOP_TEXT_Y + CIRCLE_TEXT_LIFT,
                    group,
                    &text(SMALL_FONT),
                );
            }
            match book.school {
                School::Magery => {
                    let height = g
                        .label(text_x, NAME_Y, &name, &text(SMALL_FONT).wrap(NAME_WIDTH))
                        .y as i32;
                    g.label(
                        text_x,
                        letters_y(height),
                        &words_letters(&words),
                        &text(LETTERS_FONT),
                    );
                }
                School::Mastery => {
                    let height = g
                        .label(text_x, NAME_Y, &name, &text(SMALL_FONT).wrap(NAME_WIDTH))
                        .y as i32;
                    if !words.is_empty() {
                        g.label(
                            text_x,
                            letters_y(height),
                            &words,
                            &text(SMALL_FONT).wrap(NAME_WIDTH),
                        );
                    }
                }
                _ => {
                    g.label(top_x, TOP_TEXT_Y, &name, &text(SMALL_FONT));
                    if !words.is_empty() {
                        g.label(text_x, NAME_Y, &words, &text(SMALL_FONT).wrap(NAME_WIDTH));
                    }
                }
            }
            self.icon(
                g,
                cx,
                (icon_x, ICON_Y),
                book.icon(place),
                spell.id,
                book.tooltip(place),
            );
            let reagents = reagent_lines(spell);
            if !reagents.is_empty() {
                if book.school != School::Mastery {
                    g.pic_tiled(
                        icon_x,
                        REAGENT_RULE_Y,
                        REAGENT_RULE_SIZE.0,
                        REAGENT_RULE_SIZE.1,
                        REAGENT_RULE,
                        0,
                    );
                }
                g.label(icon_x, REAGENTS_Y, WORDS_REAGENTS, &text(SMALL_FONT));
                g.label(icon_x, REAGENT_LIST_Y, &reagents, &text(LIST_FONT));
            }
            if book.school != School::Magery {
                let upkeep = upkeep(book, spell);
                let y = if upkeep > 0 { UPKEEP_NEEDS_Y } else { NEEDS_Y };
                g.label(
                    icon_x,
                    y,
                    &needs_words(spell.mana, spell.skill, upkeep),
                    &text(SMALL_FONT),
                );
            }
        }
    }

    /// Moves the spell button the player drags out of the book with the
    /// mouse, until the button comes up.
    fn drag_button(&mut self, g: &Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(spell) = self.dragging else {
            return;
        };
        let (mouse, down) = g
            .ui()
            .input(|i| (i.pointer.interact_pos(), i.pointer.primary_down()));
        if let Some(mouse) = mouse {
            cx.open_at(
                GumpId::of(SPELL_BUTTON.id, u32::from(spell)),
                button_place(mouse),
            );
        }
        if !down {
            self.dragging = None;
        }
    }
}

/// Makes a macro that casts the spell, when the profile has none by its
/// name, and opens it in the macro gump, as "Fast spell assign" does.
pub fn assign_macro(cx: &mut GumpContext<'_>, spell: u16) {
    let (name, steps) = spell_macro(spell);
    super::macro_gump::open_for(cx, &name, steps);
}

impl GumpBody for Spellbook {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let contents = cx
            .frame
            .spellbooks
            .iter()
            .find(|book| book.serial == self.serial);
        let graphic = contents.map_or(0, |book| book.graphic);
        let school = contents.map_or("", |book| book.school.as_str());
        let book = book_info(school, graphic);
        let layout = Layout::of(book, contents);
        if !self.sounded {
            self.sounded = true;
            cx.play_sound(PAGE_SOUND);
        }
        if self.minimized {
            g.pic(0, 0, book.minimized, 0);
            if g.body_double_click() {
                self.minimized = false;
            }
            return;
        }
        g.pic(0, 0, book.book, 0);
        self.page = self.page.clamp(FIRST_PAGE, layout.last_page);
        let (x, y, w, h) = MINIMIZE_AT;
        if g.click_area("minimize", x, y, w, h).clicked() {
            self.minimized = true;
        }
        if self.page != FIRST_PAGE {
            let left = g.pic_button(
                "corner_left",
                CORNER_LEFT_AT.0,
                CORNER_LEFT_AT.1,
                CORNER_LEFT,
                0,
            );
            if left.double_clicked() {
                self.turn_to(cx, FIRST_PAGE, layout.last_page);
            } else if left.clicked() {
                self.turn_to(cx, self.page - 1, layout.last_page);
            }
        }
        if self.page != layout.last_page {
            let right = g.pic_button(
                "corner_right",
                CORNER_RIGHT_AT.0,
                CORNER_RIGHT_AT.1,
                CORNER_RIGHT,
                0,
            );
            if right.double_clicked() {
                self.turn_to(cx, layout.last_page, layout.last_page);
            } else if right.clicked() {
                self.turn_to(cx, self.page + 1, layout.last_page);
            }
        }
        self.index(g, cx, book, &layout);
        self.spell_pages(g, cx, book, &layout);
        if let Some(page) = self.page_clicks.due(now(g)) {
            self.turn_to(cx, page as usize, layout.last_page);
        }
        if self.page_clicks.is_waiting() {
            g.ctx().request_repaint();
        }
        self.drag_button(g, cx);
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.play_sound(PAGE_SOUND);
        Closing::Now
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame
            .spellbooks
            .iter()
            .any(|book| book.serial == self.serial)
            || frame.containers.iter().any(|c| c.serial == self.serial)
    }
}

/// Where the spell button of a drag opens: its middle at the mouse.
pub fn button_place(mouse: Pos2) -> Pos2 {
    mouse - Vec2::splat(BUTTON_HALF)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::model::spell_data::BOOKS;

    fn book_with(spells: &[u16]) -> WatchSpellbook {
        WatchSpellbook {
            serial: 1,
            graphic: 0x0EFA,
            first_spell: 1,
            school: "magery".into(),
            spells: spells.iter().map(|n| (*n, String::new())).collect(),
        }
    }

    #[test]
    fn the_spells_follow_the_index_two_to_a_page() {
        let magery = &BOOKS[0];
        let layout = Layout::of(magery, Some(&book_with(&[1, 4, 18, 64])));
        assert_eq!(layout.index_pages, 4);
        assert_eq!(layout.last_page, 6);
        assert_eq!(layout.page_of(0), Some((5, 0)));
        assert_eq!(layout.page_of(3), Some((5, 1)));
        assert_eq!(layout.page_of(17), Some((6, 0)));
        assert_eq!(layout.page_of(63), Some((6, 1)));
        assert_eq!(layout.page_of(2), None);
        let empty = Layout::of(magery, None);
        assert_eq!(empty.last_page, 4);
    }

    #[test]
    fn a_book_of_each_school_draws_and_lives_with_its_book() {
        use crate::window::classic::manager::GumpManager;
        use crate::window::classic::testing::draw_frames;
        use crate::window::settings::Profile;
        let schools = [
            ("magery", 0x0EFA, 1),
            ("necromancy", 0x2253, 101),
            ("chivalry", 0x2252, 201),
            ("bushido", 0x238C, 401),
            ("ninjitsu", 0x23A0, 501),
            ("spellweaving", 0x2D50, 601),
            ("mysticism", 0x2D9D, 678),
            ("mastery", 0x225A, 701),
        ];
        let books: Vec<WatchSpellbook> = schools
            .iter()
            .enumerate()
            .map(|(at, (school, graphic, first))| WatchSpellbook {
                serial: 0x4000_0100 + at as u32,
                graphic: *graphic,
                first_spell: *first,
                school: (*school).into(),
                spells: vec![(*first, String::new()), (*first + 1, String::new())],
            })
            .collect();
        let frame = WatchFrame {
            spellbooks: books.clone(),
            ..WatchFrame::default()
        };
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        for book in &books {
            manager.open(GumpId::of(SPELLBOOK.id, book.serial), &mut profile);
            assert!(Spellbook::new(book.serial).alive(&frame));
        }
        assert!(!Spellbook::new(1).alive(&frame));
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert_eq!(manager.drawn_of(SPELLBOOK.id).len(), books.len());
    }

    #[test]
    fn a_dragged_button_opens_with_its_middle_at_the_mouse() {
        assert_eq!(button_place(Pos2::new(100.0, 100.0)), Pos2::new(78.0, 78.0));
    }
}
