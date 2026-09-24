//! The book of weapon abilities and the ability buttons dragged out of
//! it. The index
//! pages name every ability, and show the icons of the primary and the
//! secondary ability of the weapon in hand, red while armed; a double click
//! on one arms it, or lets it go, and a drag puts its button on the
//! desktop. Each ability has a page with its icon and the weapons that have
//! it. The count of abilities follows the icons the client files hold, as
//! older clients hold fewer.

use super::book_pages::{BookPages, FIRST_PAGE};
use super::canvas::Canvas;
use super::item_control::started_drag;
use super::registry::{well_known, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::spell_button::{pressed, BUTTON_GROUP};
use super::text::TextLook;
use super::top_bar::capitalized;
use crate::window::control::Act;
use crate::window::model::abilities::{
    ability_of, icon_of, slot_hue, toggle_command, AbilitySlot, FIRST_ABILITY_ICON,
    FIRST_ABILITY_NAME, FIRST_ABILITY_TOOLTIP,
};
use eframe::egui::Vec2;
use uoterm_assist::abilities::{ability_name, weapons_with};

pub const COMBAT_BOOK: GumpKind = GumpKind {
    id: well_known::COMBAT_BOOK,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(CombatBook::default()),
};

/// The button of the primary (serial 0) or the secondary (serial 1)
/// ability of the weapon in hand.
pub const ABILITY_BUTTON: GumpKind = GumpKind {
    id: ABILITY_BUTTON_ID,
    rules: GumpRules {
        anchor: Some(BUTTON_GROUP),
        ..GumpRules::DEFAULT
    },
    open: |serial| {
        Box::new(AbilityButton::new(AbilitySlot::of(
            serial.unwrap_or_default(),
        )))
    },
};

const ABILITY_BUTTON_ID: &str = "ability_button";
const BACKGROUND: u16 = 0x2B02;
/// The most abilities a book holds, on the newest clients.
const MOST_ABILITIES: u16 = 32;
/// An index page holds nine names at its left and four at its right.
const NAMES_LEFT: usize = 9;
const NAMES_RIGHT: usize = 4;
const NAMES_PER_PAGE: usize = NAMES_LEFT + NAMES_RIGHT;
const INDEX_WORDS: &str = "INDEX";
const INDEX_X: [i32; 2] = [96, 259];
const NAMES_X: [i32; 2] = [52, 215];
const INDEX_Y: i32 = 6;
const NAMES_TOP: i32 = 42;
const NAME_STEP: i32 = 15;
const INK: u16 = 0x0288;
const INK_OVER: u16 = 0x0033;
const TITLE_FONT: u8 = 6;
const NAME_FONT: u8 = 9;
const ICON_LABEL_WIDTH: u32 = 80;
const PRIMARY_AT: (i32, i32) = (215, 105);
const SECONDARY_AT: (i32, i32) = (215, 150);
const ICON_LABEL_X: i32 = 265;
const PRIMARY_WORDS: &str = "Primary Ability Icon";
const SECONDARY_WORDS: &str = "Secondary Ability Icon";
// An ability's own page.
const PAGE_ICON_AT: (i32, i32) = (62, 40);
const PAGE_NAME_AT: (i32, i32) = (110, 34);
const RULE: u16 = 0x0835;
const RULE_AT: (i32, i32) = (62, 88);
const RULE_WIDTH: i32 = 128;
const WEAPONS_AT: (i32, i32) = (62, 98);
const WEAPONS_RIGHT_AT: (i32, i32) = (215, 34);
const WEAPON_STEP: i32 = 16;
/// The weapons of an ability go to the right page after this many.
const WEAPONS_LEFT: usize = 6;
/// A dragged button opens with its middle at the mouse.
const BUTTON_HALF: f32 = 22.0;

/// Arms the ability of a slot, or lets it go when it is armed.
fn use_ability(cx: &GumpContext<'_>, slot: AbilitySlot) {
    cx.act(Act::Command(toggle_command(cx.frame, slot)));
}

/// The name of an ability number, from the client's words.
fn name_of(g: &Canvas<'_>, ability: u8) -> String {
    let fallback = ability_name(ability).unwrap_or_default();
    g.words(
        FIRST_ABILITY_NAME + u32::from(ability.saturating_sub(1)),
        fallback,
    )
}

/// Opens a button dragged out of a book, `half` up and left of the mouse,
/// and lets the mouse drag it on. A button of it that was open goes.
pub fn drag_out(
    g: &Canvas<'_>,
    cx: &mut GumpContext<'_>,
    id: GumpId,
    body: Box<dyn GumpBody>,
    half: f32,
) {
    let Some(mouse) = g.ui().input(|i| i.pointer.interact_pos()) else {
        return;
    };
    cx.close(id);
    cx.open_with(id, body);
    cx.open_at(id, mouse - Vec2::splat(half));
    cx.drag_with_pointer(id);
}

/// The book's pages: the index pages, then one page for each ability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Layout {
    abilities: usize,
    index_pages: usize,
}

impl Layout {
    fn of(abilities: usize) -> Self {
        Self {
            abilities,
            index_pages: abilities.div_ceil(NAMES_PER_PAGE).max(FIRST_PAGE),
        }
    }

    fn last_page(self) -> usize {
        self.index_pages + self.abilities
    }

    /// The page of an ability, by its place from zero.
    fn page_of(self, index: usize) -> usize {
        self.index_pages + index + FIRST_PAGE
    }
}

/// The name of an index that a click turns to, under the mouse in the hue
/// of the classic hovered label.
fn hovered_name(g: &mut Canvas<'_>, key: (usize, usize), (x, y): (i32, i32), words: &str) -> bool {
    let normal = TextLook::ascii(NAME_FONT, INK);
    let size = g.measure(words, &normal);
    let over = g.hovered(x, y, size.x as i32, size.y as i32);
    let look = if over {
        TextLook::ascii(NAME_FONT, INK_OVER)
    } else {
        normal
    };
    g.label(x, y, words, &look);
    g.click_area(key, x, y, size.x as i32, size.y as i32)
        .clicked()
}

#[derive(Default)]
pub struct CombatBook {
    pages: BookPages,
}

impl CombatBook {
    fn index_page(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, layout: Layout) {
        let first = (self.pages.page() - FIRST_PAGE) * NAMES_PER_PAGE;
        let title = TextLook::ascii(TITLE_FONT, INK);
        for (side, count) in [NAMES_LEFT, NAMES_RIGHT].into_iter().enumerate() {
            g.label(INDEX_X[side], INDEX_Y, INDEX_WORDS, &title);
            let start = first + if side == 0 { 0 } else { NAMES_LEFT };
            for row in 0..count {
                let index = start + row;
                if index >= layout.abilities {
                    break;
                }
                let ability = index as u8 + 1;
                let at = (NAMES_X[side], NAMES_TOP + row as i32 * NAME_STEP);
                let name = name_of(g, ability);
                if hovered_name(g, (side, row), at, &name) {
                    self.pages.index_clicked(g, layout.page_of(index));
                }
                let tip = g.words(FIRST_ABILITY_TOOLTIP + u32::from(ability - 1), "");
                g.tooltip(&tip);
            }
        }
        let label = TextLook::ascii(TITLE_FONT, INK).wrap(ICON_LABEL_WIDTH);
        for (slot, at, words) in [
            (AbilitySlot::Primary, PRIMARY_AT, PRIMARY_WORDS),
            (AbilitySlot::Secondary, SECONDARY_AT, SECONDARY_WORDS),
        ] {
            let ability = ability_of(cx.frame, slot);
            let hue = slot_hue(cx.frame, slot);
            let response = g.pic_button(("slot", slot.serial()), at.0, at.1, icon_of(ability), hue);
            let tip = name_of(g, ability);
            g.tooltip(&tip);
            if started_drag(&response) {
                let id = GumpId::of(ABILITY_BUTTON_ID, slot.serial());
                drag_out(g, cx, id, Box::new(AbilityButton::new(slot)), BUTTON_HALF);
            } else if response.double_clicked() {
                use_ability(cx, slot);
            }
            g.label(ICON_LABEL_X, at.1, words, &label);
        }
    }

    fn ability_page(&self, g: &mut Canvas<'_>, index: usize) {
        let ability = index as u8 + 1;
        let (x, y) = PAGE_ICON_AT;
        g.pic(x, y, icon_of(ability), 0);
        let tip = g.words(FIRST_ABILITY_TOOLTIP + index as u32, "");
        g.tooltip(&tip);
        let name = capitalized(&name_of(g, ability));
        let title = TextLook::ascii(TITLE_FONT, INK).wrap(ICON_LABEL_WIDTH);
        g.label(PAGE_NAME_AT.0, PAGE_NAME_AT.1, &name, &title);
        let rule_height = g.gump_size(RULE).map_or(0, |size| size.y as i32);
        g.pic_tiled(RULE_AT.0, RULE_AT.1, RULE_WIDTH, rule_height, RULE, 0);
        let mut names: Vec<String> = Vec::new();
        for graphic in weapons_with(ability) {
            let Some(tile) = g.scene.item_tile(graphic) else {
                continue;
            };
            let name = capitalized(&tile.name);
            if !name.is_empty() && !names.contains(&name) {
                names.push(name);
            }
        }
        let look = TextLook::ascii(NAME_FONT, INK);
        for (row, name) in names.iter().enumerate() {
            let (x, top, row) = if row < WEAPONS_LEFT {
                (WEAPONS_AT.0, WEAPONS_AT.1, row)
            } else {
                (WEAPONS_RIGHT_AT.0, WEAPONS_RIGHT_AT.1, row - WEAPONS_LEFT)
            };
            g.label(x, top + row as i32 * WEAPON_STEP, name, &look);
        }
    }
}

impl GumpBody for CombatBook {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        g.pic(0, 0, BACKGROUND, 0);
        let count = (0..MOST_ABILITIES)
            .take_while(|at| g.gump_size(FIRST_ABILITY_ICON + at).is_some())
            .count();
        let layout = Layout::of(count);
        self.pages.follow(g, cx, layout.last_page());
        let page = self.pages.page();
        if page <= layout.index_pages {
            self.index_page(g, cx, layout);
        } else {
            self.ability_page(g, page - layout.index_pages - FIRST_PAGE);
        }
    }
}

/// A button on the desktop that arms the primary or the secondary ability
/// of the weapon in hand, red while it is armed.
pub struct AbilityButton {
    slot: AbilitySlot,
}

impl AbilityButton {
    pub fn new(slot: AbilitySlot) -> Self {
        Self { slot }
    }
}

impl GumpBody for AbilityButton {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let ability = ability_of(cx.frame, self.slot);
        g.pic(0, 0, icon_of(ability), slot_hue(cx.frame, self.slot));
        let tip = name_of(g, ability);
        g.tooltip(&tip);
        if pressed(g, cx) {
            use_ability(cx, self.slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{WatchAbilities, WatchEquip, WatchFrame, WatchLook};
    use crate::window::classic::manager::GumpManager;
    use crate::window::classic::testing::draw_frames;
    use crate::window::settings::Profile;
    use uoterm_protocol::types::LAYER_ONE_HANDED;

    const KATANA: u16 = 0x13FF;
    const ARMOR_IGNORE: u8 = 1;

    fn armed_frame(armed: Option<u8>) -> WatchFrame {
        WatchFrame {
            look: WatchLook {
                equipment: vec![WatchEquip {
                    serial: 5,
                    graphic: KATANA,
                    layer: LAYER_ONE_HANDED,
                    hue: 0,
                }],
                ..WatchLook::default()
            },
            abilities: WatchAbilities {
                weapon: armed.map(|number| (number, String::new())),
                spells: Vec::new(),
            },
            ..WatchFrame::default()
        }
    }

    #[test]
    fn the_book_has_index_pages_then_a_page_for_each_ability() {
        let full = Layout::of(32);
        assert_eq!((full.index_pages, full.last_page()), (3, 35));
        assert_eq!(full.page_of(0), 4);
        let old = Layout::of(13);
        assert_eq!((old.index_pages, old.last_page()), (1, 14));
        assert_eq!(Layout::of(0).index_pages, FIRST_PAGE);
    }

    #[test]
    fn the_book_and_a_button_draw() {
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let book = GumpId::one(well_known::COMBAT_BOOK);
        let button = GumpId::of(ABILITY_BUTTON_ID, AbilitySlot::Secondary.serial());
        assert!(manager.open(book, &mut profile));
        assert!(manager.open(button, &mut profile));
        if draw_frames(&mut manager, &mut profile, &armed_frame(Some(ARMOR_IGNORE))) {
            assert_eq!(manager.drawn_of(well_known::COMBAT_BOOK).len(), 1);
            assert_eq!(manager.drawn_of(ABILITY_BUTTON_ID).len(), 1);
        }
    }
}
