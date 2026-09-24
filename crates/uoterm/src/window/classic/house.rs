//! The house designer of the Classic style, as the reference client
//! draws it: the stone frame with the buttons of
//! the walls, doors, floors, stairs, roofs and the rest, erase, the
//! eyedropper and the system menu at the left, with the count of the
//! components and fixtures under them; the parts of the picked kind in the
//! middle, a page at a time; the storeys of the house at the right, each
//! with the button that turns how it shows, and the cost under them. It
//! opens while the shard has the designer open, and a right click leaves
//! the designer.
//!
//! It changes the design the Modern panel changes (`model::house_design`),
//! which a click on the house reads.

use super::canvas::{ButtonArt, Canvas};
use super::manager::GumpManager;
use super::registry::{Closing, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::WatchFrame;
use crate::window::control::Act;
use crate::window::model::house_design::{
    design_counts, limits_words, plot_limits, styles_of, SharedDesign, ACTION_EXIT,
    DESIGNER_FLOORS, DESIGN_COMMANDS,
};
use crate::window::settings::Profile;
use eframe::egui::Pos2;
use uoterm_nav::{HousePart, HousePartKind};

pub const HOUSE_ID: &str = "house_customization";

pub const HOUSE: GumpKind = GumpKind {
    id: HOUSE_ID,
    rules: GumpRules {
        kept: false,
        first_place: Pos2::new(0.0, 60.0),
        ..GumpRules::DEFAULT
    },
    open: |_| Box::new(HouseGump::new(SharedDesign::default())),
};

pub const HOUSE_GUMP: GumpId = GumpId::one(HOUSE_ID);

// The frame.
const BACK_TILE: u16 = 0x0E14;
const BACK_AREA: (i32, i32, i32, i32) = (121, 36, 397, 120);
const LEFT_PIECE: u16 = 0x55F0;
const LEFT_AT: (i32, i32) = (0, 17);
/// The right side of a house of four storeys, and of three.
const RIGHT_PIECE: u16 = 0x55F2;
const RIGHT_PIECE_THREE: u16 = 0x55F9;
const RIGHT_AT: (i32, i32) = (486, 17);
const MIDDLE_TILE: u16 = 0x55F1;
const MIDDLE_AREA: (i32, i32, i32, i32) = (153, 17, 333, 154);

// The buttons of the left side.
const STATES: [(Show, ButtonArt, (i32, i32), &str); 7] = [
    (
        Show::Kind(Group::Walls),
        ButtonArt::new(0x5654, 0x5656, 0x5655),
        (9, 41),
        "Walls",
    ),
    (
        Show::Kind(Group::Doors),
        ButtonArt::new(0x5657, 0x5659, 0x5658),
        (39, 40),
        "Doors",
    ),
    (
        Show::Kind(Group::Floors),
        ButtonArt::new(0x565A, 0x565C, 0x565B),
        (70, 40),
        "Floors",
    ),
    (
        Show::Kind(Group::Stairs),
        ButtonArt::new(0x565D, 0x565F, 0x565E),
        (9, 72),
        "Stairs",
    ),
    (
        Show::Kind(Group::Roofs),
        ButtonArt::new(0x5788, 0x578A, 0x5789),
        (39, 72),
        "Roofs",
    ),
    (
        Show::Kind(Group::Misc),
        ButtonArt::new(0x5663, 0x5665, 0x5664),
        (69, 72),
        "Miscellaneous",
    ),
    (
        Show::Menu,
        ButtonArt::new(0x566C, 0x566E, 0x566D),
        (69, 100),
        "System Menu",
    ),
];
const ERASE: u16 = 0x5666;
const ERASE_PRESSED: u16 = 0x5668;
const ERASE_OVER: u16 = 0x5667;
const ERASE_AT: (i32, i32) = (9, 100);
const WORDS_ERASE: &str = "Erase";
const EYEDROPPER: u16 = 0x5669;
const EYEDROPPER_PRESSED: u16 = 0x566B;
const EYEDROPPER_OVER: u16 = 0x566A;
const EYEDROPPER_AT: (i32, i32) = (39, 100);
const WORDS_EYEDROPPER: &str = "Eyedropper Tool";

// The counts under the left side and the cost under the right.
const COUNT_FONT: u8 = 9;
const COUNT_HUE: u16 = 0x0481;
const COUNT_FULL_HUE: u16 = 0x0026;
/// The components end at this x; the colon and the fixtures follow.
const COMPONENTS_END_X: i32 = 82;
const COLON_AT: (i32, i32) = (84, 142);
const FIXTURES_X: i32 = 94;
const COUNT_Y: i32 = 142;
const COST_AT: (i32, i32) = (524, 142);
const WORDS_COST: &str = "Cost";

// How each storey shows: its button, from the ground up, in the three
// pictures of the reference client's table (all shown, see-through, hidden).
const STOREY_ART_GROUND: [u16; 3] = [0x572E, 0x5734, 0x5731];
const STOREY_ART_UPPER: [u16; 3] = [0x5725, 0x5728, 0x572B];
const STOREY_PRESSED_STEP: u16 = 2;
const STOREY_OVER_STEP: u16 = 1;
const STOREY_BUTTONS_X: i32 = 533;
const STOREY_BUTTONS_Y: [i32; 4] = [108, 86, 64, 42];
const WORDS_STOREY_VISIBILITY: &str = "Story";
const WORDS_VISIBILITY: &str = "Visibility";

// The floors of the right side, from the ground up: the small button and
// the large one, each moved on by these when it is the floor shown.
const FLOOR_SMALL: [(u16, u16, (i32, i32)); 4] = [
    (0x56CD, 0x56D1, (583, 96)),
    (0x56CE, 0x56D2, (583, 73)),
    (0x56CE, 0x56D2, (582, 56)),
    (0x56D0, 0x56D4, (583, 42)),
];
const FLOOR_LARGE: [(u16, (i32, i32)); 4] = [
    (0x56F6, (623, 103)),
    (0x56F0, (623, 86)),
    (0x56F0, (623, 69)),
    (0x56EA, (623, 50)),
];
const SMALL_SHOWN_STEP: u16 = 4;
const LARGE_SHOWN_STEP: u16 = 3;
const LARGE_PRESSED: u16 = 2;
const LARGE_OVER: u16 = 1;
const WORDS_GO_TO_FLOOR: &str = "Go To Story";

// The parts in the middle.
const LIST_X: i32 = 121;
const LIST_Y: i32 = 36;
const LIST_WIDTH: i32 = 384;
const CELL_WIDTH: i32 = 48;
const CELL_HEIGHT: i32 = 60;
const ROW_HEIGHT: i32 = 120;
const STYLES_ON_A_PAGE: usize = 16;
const PIECES_ON_A_PAGE: usize = 8;
const PIECES_X: i32 = 130;
const DOORS_X: i32 = 138;
/// Doors past the fourth lean left, as the classic designer lays them.
const DOORS_LEAN: i32 = 20;
const DOORS_BEFORE_LEAN: usize = 4;
const HALF: i32 = 2;
const PAGE_LEFT: ButtonArt = ButtonArt::new(0x5625, 0x5627, 0x5626);
const PAGE_LEFT_AT: (i32, i32) = (110, 63);
const PAGE_RIGHT: ButtonArt = ButtonArt::new(0x5628, 0x562A, 0x5629);
const PAGE_RIGHT_AT: (i32, i32) = (510, 63);
const CATEGORY_TOP: u16 = 0x55F3;
const CATEGORY_TOP_AT: (i32, i32) = (152, 0);
const TO_CATEGORIES: ButtonArt = ButtonArt::new(0x5622, 0x5624, 0x5623);
const TO_CATEGORIES_AT: (i32, i32) = (167, 5);
const CATEGORY_CAP: u16 = 0x55F4;
const CATEGORY_CAP_AT: (i32, i32) = (218, 4);

// The system menu.
const MENU_BUTTON: ButtonArt = ButtonArt::new(0x098D, 0x098D, 0x098D);
const MENU_FONT: u8 = 0;
const MENU_HUE: u16 = 0;
const MENU_HUE_OVER: u16 = 0x0036;
/// Where each step of `DESIGN_COMMANDS` stands in the system menu.
const MENU_PLACES: [(i32, i32); 6] = [
    (150, 50),
    (150, 90),
    (270, 50),
    (270, 90),
    (390, 50),
    (390, 90),
];

/// A button of the left side: the parts of one group, or the system menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Show {
    Kind(Group),
    Menu,
}

/// The groups of parts of the classic designer. Floors hold the
/// teleporters too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    Walls,
    Doors,
    Floors,
    Stairs,
    Roofs,
    Misc,
}

impl Group {
    fn kinds(self) -> &'static [HousePartKind] {
        match self {
            Group::Walls => &[HousePartKind::Wall],
            Group::Doors => &[HousePartKind::Door],
            Group::Floors => &[HousePartKind::Floor, HousePartKind::Teleporter],
            Group::Stairs => &[HousePartKind::Stair],
            Group::Roofs => &[HousePartKind::Roof],
            Group::Misc => &[HousePartKind::Misc],
        }
    }

    /// The groups whose styles show as a grid of categories first.
    fn has_categories(self) -> bool {
        matches!(self, Group::Walls | Group::Roofs | Group::Misc)
    }

    fn of(kind: HousePartKind) -> Self {
        match kind {
            HousePartKind::Wall => Group::Walls,
            HousePartKind::Door => Group::Doors,
            HousePartKind::Floor | HousePartKind::Teleporter => Group::Floors,
            HousePartKind::Stair => Group::Stairs,
            HousePartKind::Roof => Group::Roofs,
            HousePartKind::Misc => Group::Misc,
        }
    }
}

/// The styles of a group in the order the designer shows them, each with
/// its kind and its place among the styles of its kind.
fn group_styles(parts: &[HousePart], group: Group) -> Vec<(HousePartKind, usize, &HousePart)> {
    group
        .kinds()
        .iter()
        .flat_map(|kind| {
            styles_of(parts, *kind)
                .into_iter()
                .enumerate()
                .map(move |(at, part)| (*kind, at, part))
        })
        .collect()
}

/// How many pages the middle has: pages of the category grid, or one page
/// for each style.
fn page_count(styles: usize, grid: bool) -> usize {
    if grid {
        styles.div_ceil(STYLES_ON_A_PAGE).max(1)
    } else {
        styles.max(1)
    }
}

/// The page one arrow turns to, round the ends.
fn turned(page: usize, pages: usize, forward: bool) -> usize {
    match (forward, page) {
        (true, _) => (page + 1) % pages.max(1),
        (false, 0) => pages.saturating_sub(1),
        (false, _) => page - 1,
    }
}

/// Opens the designer gump while the shard has the designer open, with the
/// design the Modern panel shares.
pub fn follow(
    manager: &mut GumpManager,
    frame: &WatchFrame,
    profile: &mut Profile,
    design: &SharedDesign,
) {
    if frame.designing.is_some() && !manager.is_open(&HOUSE_GUMP) {
        let body = HouseGump::new(SharedDesign::clone(design));
        manager.open_body(HOUSE_GUMP, Box::new(body), profile);
    }
}

pub struct HouseGump {
    design: SharedDesign,
    show: Show,
    /// The style picked in the grid of a group with categories, showing
    /// its pieces.
    category: Option<usize>,
    page: usize,
}

impl HouseGump {
    pub fn new(design: SharedDesign) -> Self {
        let group = Group::of(design.borrow().kind);
        Self {
            design,
            show: Show::Kind(group),
            category: None,
            page: 0,
        }
    }

    /// The styles of the group in the middle: the grid of categories, or the
    /// pieces of one style.
    fn parts(&mut self, g: &mut Canvas<'_>, group: Group, parts: &[HousePart]) -> usize {
        let styles = group_styles(parts, group);
        let grid = group.has_categories() && self.category.is_none();
        if grid {
            let first = self.page * STYLES_ON_A_PAGE;
            let mut picked = None;
            for (at, (_, _, part)) in styles.iter().enumerate().skip(first).take(STYLES_ON_A_PAGE) {
                let slot = (at - first) as i32;
                let columns = LIST_WIDTH / CELL_WIDTH;
                let (x, y) = (
                    LIST_X + (slot % columns) * CELL_WIDTH,
                    LIST_Y + (slot / columns) * CELL_HEIGHT,
                );
                let Some(graphic) = part.pieces.first().copied() else {
                    continue;
                };
                let size = g.item_size(graphic);
                let left = x + (CELL_WIDTH - size.x as i32) / HALF;
                g.clipped(x, y, CELL_WIDTH, CELL_HEIGHT, |g| {
                    g.item(left, y, graphic, 0);
                });
                if g.hit_box(("style", at), x, y, CELL_WIDTH, CELL_HEIGHT)
                    .clicked()
                {
                    picked = Some(at);
                }
                g.tooltip_at(x, y, CELL_WIDTH, CELL_HEIGHT, &part.name);
            }
            if let Some(at) = picked {
                let (kind, style, _) = styles[at];
                let mut design = self.design.borrow_mut();
                design.set_kind(kind);
                design.set_style(style);
                self.category = Some(at);
            }
            return page_count(styles.len(), true);
        }
        let shown = self.category.unwrap_or(self.page);
        if let Some((kind, style, part)) = styles.get(shown).copied() {
            let base = if group == Group::Doors {
                DOORS_X
            } else {
                PIECES_X
            };
            let mut picked = None;
            for (at, graphic) in part.pieces.iter().take(PIECES_ON_A_PAGE).enumerate() {
                let size = g.item_size(*graphic);
                let mut x = base + at as i32 * CELL_WIDTH + (CELL_WIDTH - size.x as i32) / HALF;
                if group == Group::Doors && at >= DOORS_BEFORE_LEAN {
                    x -= DOORS_LEAN;
                }
                let y = LIST_Y + (ROW_HEIGHT - size.y as i32) / HALF;
                g.clipped(LIST_X, LIST_Y, LIST_WIDTH, ROW_HEIGHT, |g| {
                    g.item(x, y, *graphic, 0);
                });
                if g.hit_box(("piece", at), x, y, size.x as i32, size.y as i32)
                    .clicked()
                {
                    picked = Some(at);
                }
            }
            if let Some(piece) = picked {
                let mut design = self.design.borrow_mut();
                design.set_kind(kind);
                design.set_style(style);
                design.piece = piece;
                design.removing = false;
                design.picking = false;
            }
        }
        if group.has_categories() {
            1
        } else {
            page_count(styles.len(), false)
        }
    }
}

impl GumpBody for HouseGump {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let (x, y, w, h) = BACK_AREA;
        g.pic_tiled(x, y, w, h, BACK_TILE, 0);
        let pages = match self.show {
            Show::Kind(group) => self.parts(g, group, &cx.frame.house_parts),
            Show::Menu => {
                let look = TextLook::unicode(MENU_FONT, MENU_HUE);
                for ((words, action), (bx, by)) in DESIGN_COMMANDS.into_iter().zip(MENU_PLACES) {
                    if g.caption_button(words, bx, by, MENU_BUTTON, words, &look, MENU_HUE_OVER) {
                        cx.act(Act::HouseCommand(action));
                    }
                }
                1
            }
        };
        let designing = cx.frame.designing;
        let limits = designing
            .and_then(|d| d.plot)
            .map(|(w, h)| plot_limits(w, h));
        let storeys = limits.map_or(DESIGNER_FLOORS, |limits| limits.storeys);
        let right = if storeys == DESIGNER_FLOORS {
            RIGHT_PIECE
        } else {
            RIGHT_PIECE_THREE
        };
        g.pic(LEFT_AT.0, LEFT_AT.1, LEFT_PIECE, 0);
        g.pic(RIGHT_AT.0, RIGHT_AT.1, right, 0);
        let (x, y, w, h) = MIDDLE_AREA;
        g.pic_tiled(x, y, w, h, MIDDLE_TILE, 0);
        for (show, art, (bx, by), words) in STATES {
            if g.button(words, bx, by, art) {
                self.show = show;
                self.category = None;
                self.page = 0;
            }
            g.tooltip(words);
        }
        let (removing, picking) = {
            let design = self.design.borrow();
            (design.removing, design.picking)
        };
        let erase = ButtonArt::new(ERASE + u16::from(removing), ERASE_PRESSED, ERASE_OVER);
        if g.button("erase", ERASE_AT.0, ERASE_AT.1, erase) {
            self.design.borrow_mut().toggle_removing();
        }
        g.tooltip(WORDS_ERASE);
        let eyedropper = ButtonArt::new(
            EYEDROPPER + u16::from(picking),
            EYEDROPPER_PRESSED,
            EYEDROPPER_OVER,
        );
        if g.button("eyedropper", EYEDROPPER_AT.0, EYEDROPPER_AT.1, eyedropper) {
            self.design.borrow_mut().toggle_picking();
        }
        g.tooltip(WORDS_EYEDROPPER);
        let in_category = matches!(self.show, Show::Kind(group) if group.has_categories());
        if in_category && self.category.is_some() {
            g.pic(CATEGORY_TOP_AT.0, CATEGORY_TOP_AT.1, CATEGORY_TOP, 0);
            if g.button(
                "categories",
                TO_CATEGORIES_AT.0,
                TO_CATEGORIES_AT.1,
                TO_CATEGORIES,
            ) {
                self.category = None;
                self.page = 0;
            }
            g.pic(CATEGORY_CAP_AT.0, CATEGORY_CAP_AT.1, CATEGORY_CAP, 0);
        }
        let current = designing.map_or(1, |designing| designing.floor);
        for at in 0..usize::from(storeys) {
            let level = at as u8 + 1;
            // The top storey has the top pictures, in the place of its own
            // storey.
            let art = if level == storeys {
                FLOOR_SMALL.len() - 1
            } else {
                at
            };
            let ((small, small_pressed, _), (large, _)) = (FLOOR_SMALL[art], FLOOR_LARGE[art]);
            let (small_at, large_at) = (FLOOR_SMALL[at].2, FLOOR_LARGE[at].1);
            let shown = level == current;
            let small = small + if shown { SMALL_SHOWN_STEP } else { 0 };
            let large = large + if shown { LARGE_SHOWN_STEP } else { 0 };
            let words = format!("{WORDS_GO_TO_FLOOR} {level}");
            let small_art = ButtonArt::new(small, small_pressed, small);
            let large_art = ButtonArt::new(large, large + LARGE_PRESSED, large + LARGE_OVER);
            let pressed_small = g.button(("floor-small", at), small_at.0, small_at.1, small_art);
            g.tooltip(&words);
            let pressed_large = g.button(("floor-large", at), large_at.0, large_at.1, large_art);
            g.tooltip(&words);
            if pressed_small || pressed_large {
                let act = self.design.borrow_mut().go_to_floor(level);
                cx.act(act);
            }
            let look = self.design.borrow().storeys[at];
            let pictures = if at == 0 {
                STOREY_ART_GROUND
            } else {
                STOREY_ART_UPPER
            };
            let picture = pictures[look.button_look()];
            let storey_art = ButtonArt::new(
                picture,
                picture + STOREY_PRESSED_STEP,
                picture + STOREY_OVER_STEP,
            );
            if g.button(
                ("storey", at),
                STOREY_BUTTONS_X,
                STOREY_BUTTONS_Y[at],
                storey_art,
            ) {
                self.design.borrow_mut().turn_storey(at);
            }
            g.tooltip(&format!(
                "{WORDS_STOREY_VISIBILITY} {level} {WORDS_VISIBILITY}: {}",
                look.words()
            ));
        }
        if let Some(limits) = limits {
            let counts = design_counts(cx.frame);
            let hue = |count: u32, most: u32| {
                if count >= most {
                    COUNT_FULL_HUE
                } else {
                    COUNT_HUE
                }
            };
            let tip = limits_words(limits);
            let components = counts.components.to_string();
            let look = TextLook::ascii(COUNT_FONT, hue(counts.components, limits.components));
            let width = g.measure(&components, &look).x as i32;
            g.label(COMPONENTS_END_X - width, COUNT_Y, &components, &look);
            g.tooltip(&tip);
            g.label(
                COLON_AT.0,
                COLON_AT.1,
                ":",
                &TextLook::ascii(COUNT_FONT, COUNT_HUE),
            );
            let look = TextLook::ascii(COUNT_FONT, hue(counts.fixtures, limits.fixtures));
            g.label(FIXTURES_X, COUNT_Y, &counts.fixtures.to_string(), &look);
            g.tooltip(&tip);
            let cost = counts.cost().to_string();
            g.label(
                COST_AT.0,
                COST_AT.1,
                &cost,
                &TextLook::ascii(COUNT_FONT, COUNT_HUE),
            );
            g.tooltip(WORDS_COST);
        }
        if pages > 1 {
            if g.button("page-left", PAGE_LEFT_AT.0, PAGE_LEFT_AT.1, PAGE_LEFT) {
                self.page = turned(self.page, pages, false);
            }
            if g.button("page-right", PAGE_RIGHT_AT.0, PAGE_RIGHT_AT.1, PAGE_RIGHT) {
                self.page = turned(self.page, pages, true);
            }
        }
    }

    /// A right click leaves the designer; the gump goes when the shard
    /// closes it.
    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(Act::HouseCommand(ACTION_EXIT));
        Closing::Wait
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.designing.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::model::house_design::HouseDesign;

    fn catalog() -> Vec<HousePart> {
        let part = |kind, name: &str| HousePart {
            kind,
            name: name.into(),
            pieces: vec![0x0064, 0x0065],
        };
        vec![
            part(HousePartKind::Floor, "Stone"),
            part(HousePartKind::Wall, "Brick"),
            part(HousePartKind::Teleporter, "Pad"),
            part(HousePartKind::Floor, "Wood"),
        ]
    }

    #[test]
    fn the_floors_hold_the_teleporters_after_the_floors() {
        let parts = catalog();
        let floors: Vec<(HousePartKind, usize)> = group_styles(&parts, Group::Floors)
            .into_iter()
            .map(|(kind, at, _)| (kind, at))
            .collect();
        assert_eq!(
            floors,
            vec![
                (HousePartKind::Floor, 0),
                (HousePartKind::Floor, 1),
                (HousePartKind::Teleporter, 0)
            ]
        );
        assert_eq!(Group::of(HousePartKind::Teleporter), Group::Floors);
    }

    #[test]
    fn pages_turn_round_the_ends() {
        assert_eq!(page_count(17, true), 2);
        assert_eq!(page_count(0, true), 1);
        assert_eq!(page_count(5, false), 5);
        assert_eq!(turned(0, 3, false), 2);
        assert_eq!(turned(2, 3, true), 0);
        assert_eq!(turned(1, 3, true), 2);
    }

    #[test]
    fn the_gump_opens_with_the_shared_design_while_the_shard_designs() {
        let shared = SharedDesign::default();
        shared.borrow_mut().set_kind(HousePartKind::Roof);
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        follow(&mut manager, &WatchFrame::default(), &mut profile, &shared);
        assert!(!manager.is_open(&HOUSE_GUMP));
        let frame = WatchFrame {
            designing: Some(crate::view::WatchDesigning {
                serial: 70,
                floor: 2,
                ..crate::view::WatchDesigning::default()
            }),
            house_parts: catalog(),
            ..WatchFrame::default()
        };
        follow(&mut manager, &frame, &mut profile, &shared);
        assert!(manager.is_open(&HOUSE_GUMP));
        let gump = HouseGump::new(shared.clone());
        assert_eq!(gump.show, Show::Kind(Group::Roofs));
        assert!(!gump.alive(&WatchFrame::default()));
        if super::super::testing::draw_frames(&mut manager, &mut profile, &frame) {
            assert!(manager.is_open(&HOUSE_GUMP));
        }
        assert_eq!(*shared.borrow(), {
            let mut roof = HouseDesign::default();
            roof.set_kind(HousePartKind::Roof);
            roof
        });
    }
}
