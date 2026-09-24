//! The book of racial abilities and the buttons dragged out of it: the abilities
//! of the character's race, humans, elves or gargoyles, by the race the
//! shard names in the status. All but a gargoyle's flight are passive; a
//! double click on the flight, in the book or on its button, flies or
//! lands.

use super::book_pages::{BookPages, FIRST_PAGE};
use super::canvas::Canvas;
use super::combat_book::drag_out;
use super::item_control::started_drag;
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::spell_button::BUTTON_GROUP;
use super::text::TextLook;
use crate::window::control::Act;
use crate::window::model::abilities::{race_of, racial_command, racial_tooltip, Race};

pub const RACIAL_ABILITIES: GumpKind = GumpKind {
    id: well_known::RACIAL_ABILITIES,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(RacialBook::default()),
};

/// The button of one racial ability, by the gump of its icon.
pub const RACIAL_BUTTON: GumpKind = GumpKind {
    id: RACIAL_BUTTON_ID,
    rules: GumpRules {
        anchor: Some(BUTTON_GROUP),
        ..GumpRules::DEFAULT
    },
    open: |serial| {
        Box::new(RacialButton {
            icon: u16::try_from(serial.unwrap_or_default()).unwrap_or_default(),
        })
    },
};

const RACIAL_BUTTON_ID: &str = "racial_button";
const BACKGROUND: u16 = 0x2B29;
/// Two pages of index, three names on each side.
const INDEX_PAGES: usize = 2;
const NAMES_PER_SIDE: usize = 3;
const NAMES_PER_PAGE: usize = NAMES_PER_SIDE * 2;
/// Two abilities go on each page after the index.
const ABILITIES_PER_PAGE: usize = 2;
const INDEX_WORDS: &str = "INDEX";
const PASSIVE_WORDS: &str = "Passive";
const INDEX_X: [i32; 2] = [106, 269];
const NAMES_X: [i32; 2] = [62, 225];
const INDEX_Y: i32 = 10;
const NAMES_TOP: i32 = 52;
const NAME_STEP: i32 = 15;
const ICON_X: [i32; 2] = [62, 225];
const ICON_Y: i32 = 40;
const ICON_NAME_X: [i32; 2] = [112, 275];
const ICON_NAME_Y: i32 = 34;
const PASSIVE_Y: i32 = 64;
const RULE: u16 = 0x0835;
const RULE_Y: i32 = 88;
const RULE_SIZE: (i32, i32) = (120, 4);
const INK: u16 = 0x0288;
const INK_OVER: u16 = 0x0033;
const TITLE_FONT: u8 = 6;
const NAME_FONT: u8 = 9;
const NAME_WIDTH: u32 = 100;
/// A dragged button opens this far up and left of the mouse.
const BUTTON_HALF: f32 = 20.0;

/// The page count of a race: the index, then two abilities on each page
/// from the second on, as the classic client counts them.
fn last_page(race: &Race) -> usize {
    INDEX_PAGES + race.names.len() / ABILITIES_PER_PAGE
}

/// The page and the side of an ability, from zero.
fn place_of(index: usize) -> (usize, usize) {
    (
        INDEX_PAGES + index / ABILITIES_PER_PAGE,
        index % ABILITIES_PER_PAGE,
    )
}

/// Flies or lands, when the character is a gargoyle and the icon is the
/// flight.
fn fly(cx: &GumpContext<'_>, icon: u16) {
    if let Some(command) = racial_command(cx.frame, icon) {
        cx.act(Act::Command(command.into()));
    }
}

#[derive(Default)]
pub struct RacialBook {
    pages: BookPages,
}

impl RacialBook {
    fn index(&mut self, g: &mut Canvas<'_>, race: &Race) {
        let page = self.pages.page();
        let title = TextLook::ascii(TITLE_FONT, INK);
        for side in 0..2 {
            g.label(INDEX_X[side], INDEX_Y, INDEX_WORDS, &title);
            for row in 0..NAMES_PER_SIDE {
                let index = (page - FIRST_PAGE) * NAMES_PER_PAGE + side * NAMES_PER_SIDE + row;
                let Some(name) = race.names.get(index) else {
                    break;
                };
                let (x, y) = (NAMES_X[side], NAMES_TOP + row as i32 * NAME_STEP);
                let normal = TextLook::ascii(NAME_FONT, INK);
                let size = g.measure(name, &normal);
                let look = if g.hovered(x, y, size.x as i32, size.y as i32) {
                    TextLook::ascii(NAME_FONT, INK_OVER)
                } else {
                    normal
                };
                g.label(x, y, name, &look);
                if g.click_area((side, row), x, y, size.x as i32, size.y as i32)
                    .clicked()
                {
                    self.pages.index_clicked(g, place_of(index).0);
                }
            }
        }
    }

    fn abilities(&self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, race: &Race) {
        let page = self.pages.page();
        for (index, name) in race.names.iter().enumerate() {
            let (on, side) = place_of(index);
            if on != page {
                continue;
            }
            let icon = race.icon(index);
            let title = TextLook::ascii(TITLE_FONT, INK).wrap(NAME_WIDTH);
            g.label(ICON_NAME_X[side], ICON_NAME_Y, name, &title);
            let passive = race.passive(index);
            if passive {
                g.label(ICON_NAME_X[side], PASSIVE_Y, PASSIVE_WORDS, &title);
            }
            let tip = g.words(race.first_tooltip + index as u32, "");
            if passive {
                g.pic(ICON_X[side], ICON_Y, icon, 0);
            } else {
                let response = g.pic_button(("icon", index), ICON_X[side], ICON_Y, icon, 0);
                if started_drag(&response) {
                    let id = GumpId::of(RACIAL_BUTTON_ID, u32::from(icon));
                    drag_out(g, cx, id, Box::new(RacialButton { icon }), BUTTON_HALF);
                } else if response.double_clicked() {
                    fly(cx, icon);
                }
            }
            g.tooltip(&tip);
            let (w, h) = RULE_SIZE;
            g.pic_tiled(ICON_X[side], RULE_Y, w, h, RULE, 0);
        }
    }
}

impl GumpBody for RacialBook {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        g.pic(0, 0, BACKGROUND, 0);
        let Some(race) = race_of(cx.frame) else {
            return;
        };
        self.pages.follow(g, cx, last_page(race));
        if self.pages.page() <= INDEX_PAGES {
            self.index(g, race);
        }
        self.abilities(g, cx, race);
    }
}

/// A racial ability on the desktop: the flight flies or lands with a
/// double click.
pub struct RacialButton {
    icon: u16,
}

impl GumpBody for RacialButton {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        g.pic(0, 0, self.icon, 0);
        let tip = g.words(racial_tooltip(self.icon), "");
        g.tooltip(&tip);
        if g.body_double_click() {
            fly(cx, self.icon);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;

    #[test]
    fn each_race_has_its_pages() {
        use crate::window::model::abilities::{ELF, HUMAN};
        assert_eq!(last_page(&ELF), 5);
        assert_eq!(place_of(0), (2, 0));
        assert_eq!(place_of(3), (3, 1));
        assert_eq!(last_page(&HUMAN), 4);
    }

    #[test]
    fn the_book_draws_for_a_gargoyle() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        assert!(manager.open(GumpId::one(well_known::RACIAL_ABILITIES), &mut profile));
        const RACE_GARGOYLE: u8 = 3;
        let mut frame = crate::view::WatchFrame::default();
        frame.status.race = RACE_GARGOYLE;
        if draw_frames(&mut manager, &mut profile, &frame) {
            assert_eq!(manager.drawn_of(well_known::RACIAL_ABILITIES).len(), 1);
        }
    }
}
