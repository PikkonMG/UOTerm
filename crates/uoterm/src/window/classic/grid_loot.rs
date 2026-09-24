//! The grid loot gump of the classic client:
//! the items of a corpse on a dark glass, each in a cell with a slider for
//! how many of a pile to take. A click on a cell grabs that many into the
//! grab bag; "Set loot bag" asks for the bag. Single items come first, then
//! the piles, over as many pages as they need. The gump closes when the
//! corpse is gone or more than three tiles away, and says so when the
//! corpse is empty. The General option "Grid loot" opens it for corpses.

use super::canvas::{Canvas, SliderStyle};
use super::item_control::{is_stackable, item_tooltip};
use super::registry::{GumpBody, GumpContext, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::{WatchFrame, WatchPackItem};
use crate::window::actions::guard::AIM_GRAB_BAG;
use crate::window::actions::LocalAim;
use crate::window::control::{Act, DropTo};
use crate::window::model::loot::{self, LootAmounts};
use crate::window::scene::Scene;
use eframe::egui::{Color32, Pos2, Vec2};
use uoterm_nav::TextAlign;

/// The id of the grid loot gump of one corpse, by the serial of the corpse.
const GRID_LOOT_ID: &str = "grid_loot";

pub const GRID_LOOT: GumpKind = GumpKind {
    id: GRID_LOOT_ID,
    rules: GumpRules {
        kept: false,
        first_place: Pos2::new(FIRST_PLACE.0, FIRST_PLACE.1),
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(GridLoot::new(serial.unwrap_or_default())),
};

const FIRST_PLACE: (f32, f32) = (100.0, 100.0);
const MAX_WIDTH: i32 = 300;
const MAX_HEIGHT: i32 = 420;
const LEAST_HEIGHT: i32 = 120;
/// A new line past this far from the bottom goes onto the next page.
const PAGE_BOTTOM_ROOM: i32 = 60;
/// The room over and under the grid for the name and the buttons.
const EDGE_ROOM: i32 = 40;
const CELL: i32 = 50;
const GAP: i32 = 20;
/// The slider of a cell sits over the cell.
const SLIDER_ROOM: i32 = 15;
const BACKGROUND_ALPHA: f32 = 0.5;
const HOVER: Color32 = Color32::from_rgba_premultiplied(51, 51, 0, 51);
const BORDER: Color32 = Color32::GRAY;
const LINE: i32 = 1;
const NAME_HUE: u16 = 0x0481;
const PAGE_HUE: u16 = 999;
const BUTTON_HUE: u16 = 0xFFFF;
const SLIDER_TEXT_HUE: u16 = 0xFFFF;
const TEXT_FONT: u8 = 1;
const SLIDER_TEXT_FONT: u8 = 0;
const LOOT_BAG_AT: (i32, i32, i32, i32) = (3, 23, 100, 20);
const PAGE_BUTTON: (i32, i32) = (40, 20);
const PREVIOUS_FROM_RIGHT: i32 = 80;
const PREVIOUS_FROM_BOTTOM: i32 = 23;
const NEXT_FROM_RIGHT: i32 = 40;
const NEXT_FROM_BOTTOM: i32 = 20;
const PAGE_LABEL_FROM_MIDDLE: i32 = 5;
const PAGE_LABEL_FROM_BOTTOM: i32 = 20;
const WORDS_LOOT_BAG: &str = "Set loot bag";
const WORDS_PREVIOUS: &str = "<<";
const WORDS_NEXT: &str = ">>";
const WORDS_EMPTY: &str = "[GridLoot]: Corpse is empty!";
const WORDS_A_CORPSE: &str = "a corpse";
const HALF: i32 = 2;

/// Where each cell of the grid lies: its page and its top left corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cell {
    page: usize,
    x: i32,
    y: i32,
}

/// The cells of `count` items, and how many lines the last page reached
/// and how many pages there are, as the reference client lays them.
fn lay_cells(count: usize) -> (Vec<Cell>, usize, usize) {
    let mut cells = Vec::with_capacity(count);
    let (mut x, mut y) = (GAP, GAP);
    let (mut line, mut page) = (1, 1);
    for _ in 0..count {
        if x >= MAX_WIDTH - GAP {
            x = GAP;
            line += 1;
            y += CELL + SLIDER_ROOM + GAP;
            if y >= MAX_HEIGHT - PAGE_BOTTOM_ROOM {
                page += 1;
                y = GAP;
            }
        }
        cells.push(Cell {
            page,
            x,
            y: y + GAP,
        });
        x += CELL + GAP;
    }
    (cells, line, page)
}

/// The height of the gump for the lines of its grid.
fn gump_height(lines: usize) -> i32 {
    let height = GAP + EDGE_ROOM + (CELL + GAP) * lines as i32 + EDGE_ROOM;
    let height = if height >= MAX_HEIGHT - EDGE_ROOM {
        MAX_HEIGHT
    } else {
        height
    };
    height.max(LEAST_HEIGHT)
}

/// True when the item may be looted: not hair, a beard or a face.
fn lootable(scene: &Scene, item: &WatchPackItem) -> bool {
    let layer = scene.item_tile(item.graphic).map_or(0, |tile| tile.quality);
    loot::lootable(item.graphic, layer)
}

pub struct GridLoot {
    corpse: u32,
    page: usize,
    amounts: LootAmounts,
    told_empty: bool,
}

impl GridLoot {
    pub fn new(corpse: u32) -> Self {
        Self {
            corpse,
            page: 1,
            amounts: LootAmounts::default(),
            told_empty: false,
        }
    }

    fn cell(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        item: &WatchPackItem,
        at: Cell,
    ) {
        let most = if is_stackable(g.scene, item.graphic) {
            i32::from(item.amount.max(1))
        } else {
            1
        };
        let amount = self.amounts.slot(item.serial, most);
        if most > 1 {
            g.slider(
                ("amount", item.serial),
                at.x,
                at.y,
                CELL,
                (1, most),
                amount,
                SliderStyle::Recessed,
            );
            let look = TextLook::unicode(SLIDER_TEXT_FONT, SLIDER_TEXT_HUE);
            let words = amount.to_string();
            let size = g.measure(&words, &look);
            g.label(at.x, at.y - size.y as i32, &words, &look);
        }
        let top = at.y + SLIDER_ROOM;
        g.shade(at.x, top, CELL, CELL, 0, BACKGROUND_ALPHA);
        let picture = g.item_size(item.graphic);
        let fit = (CELL as f32 / picture.x.max(1.0))
            .min(CELL as f32 / picture.y.max(1.0))
            .min(1.0);
        let shown = picture * fit;
        let inset = Vec2::new(CELL as f32 - shown.x, CELL as f32 - shown.y) / HALF as f32;
        g.scaled(fit, |g| {
            g.item(
                ((at.x as f32 + inset.x) / fit) as i32,
                ((top as f32 + inset.y) / fit) as i32,
                item.graphic,
                item.hue,
            );
        });
        g.outline(at.x, top, CELL, CELL, BORDER);
        let response = g.click_area(("grab", item.serial), at.x, top, CELL, CELL);
        if response.hovered() {
            g.fill(at.x + LINE, top, CELL - LINE, CELL, HOVER);
        }
        item_tooltip(g, cx, item);
        if response.clicked() {
            if let Some(bag) = cx.hand.grab_bag() {
                cx.act(Act::Move {
                    item: item.serial,
                    amount: self.amounts.taken(item.serial),
                    to: DropTo::Into(bag),
                });
            }
        }
    }
}

impl GumpBody for GridLoot {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(corpse) = cx.frame.containers.iter().find(|c| c.serial == self.corpse) else {
            return;
        };
        let mut items: Vec<WatchPackItem> = corpse
            .items
            .iter()
            .filter(|item| lootable(g.scene, item))
            .cloned()
            .collect();
        // Single items first, then the piles.
        items.sort_by_key(|item| is_stackable(g.scene, item.graphic));
        if items.is_empty() {
            if !self.told_empty {
                self.told_empty = true;
                cx.hand.report(WORDS_EMPTY);
            }
            cx.close(cx.me);
            return;
        }
        let (cells, lines, pages) = lay_cells(items.len());
        self.page = self.page.clamp(1, pages);
        let width = MAX_WIDTH;
        let height = gump_height(lines);
        g.shade(0, 0, width, height, 0, BACKGROUND_ALPHA);
        g.outline(0, 0, width, height, BORDER);
        let name = cx
            .frame
            .items
            .iter()
            .find(|item| item.serial == self.corpse)
            .map(|item| item.name.clone())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| WORDS_A_CORPSE.to_string());
        let name_look = TextLook::unicode(TEXT_FONT, NAME_HUE)
            .wrap(MAX_WIDTH as u32)
            .aligned(TextAlign::Center);
        g.label(0, 0, &name, &name_look);
        for (item, cell) in items.iter().zip(&cells) {
            if cell.page == self.page {
                self.cell(g, cx, item, *cell);
            }
        }
        let button = TextLook::unicode(TEXT_FONT, BUTTON_HUE).aligned(TextAlign::Center);
        let (bag_x, bag_from_bottom, bag_w, bag_h) = LOOT_BAG_AT;
        if g.nice_button(
            "loot_bag",
            bag_x,
            height - bag_from_bottom,
            bag_w,
            bag_h,
            WORDS_LOOT_BAG,
            &button,
            false,
        ) {
            cx.hand.report(AIM_GRAB_BAG);
            cx.hand.aim(LocalAim::SetGrabBag);
        }
        let (button_w, button_h) = PAGE_BUTTON;
        if self.page > 1
            && g.nice_button(
                "previous",
                width - PREVIOUS_FROM_RIGHT,
                height - PREVIOUS_FROM_BOTTOM,
                button_w,
                button_h,
                WORDS_PREVIOUS,
                &button,
                false,
            )
        {
            self.page -= 1;
        }
        if self.page < pages
            && g.nice_button(
                "next",
                width - NEXT_FROM_RIGHT,
                height - NEXT_FROM_BOTTOM,
                button_w,
                button_h,
                WORDS_NEXT,
                &button,
                false,
            )
        {
            self.page += 1;
        }
        g.label(
            width / HALF - PAGE_LABEL_FROM_MIDDLE,
            height - PAGE_LABEL_FROM_BOTTOM,
            &self.page.to_string(),
            &TextLook::unicode(TEXT_FONT, PAGE_HUE),
        );
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        loot::grid_loot_alive(frame, self.corpse)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_run_in_lines_of_four_and_the_height_follows_the_lines() {
        let (cells, lines, pages) = lay_cells(5);
        assert_eq!(
            cells[0],
            Cell {
                page: 1,
                x: 20,
                y: 40
            }
        );
        assert_eq!(
            cells[3],
            Cell {
                page: 1,
                x: 230,
                y: 40
            }
        );
        assert_eq!(
            cells[4],
            Cell {
                page: 1,
                x: 20,
                y: 125
            }
        );
        assert_eq!((lines, pages), (2, 1));
        assert_eq!(gump_height(1), 170);
        assert_eq!(gump_height(9), MAX_HEIGHT);
    }

    #[test]
    fn the_grid_draws_a_corpse_and_closes_when_it_is_empty_or_gone() {
        use crate::view::WatchContainer;
        use crate::window::classic::manager::GumpManager;
        use crate::window::classic::registry::GumpId;
        use crate::window::classic::testing::draw_frames;
        use crate::window::settings::Profile;
        const CORPSE: u32 = 0x4000_0200;
        let corpse = |items| WatchFrame {
            containers: vec![WatchContainer {
                serial: CORPSE,
                items,
                ..WatchContainer::default()
            }],
            ..WatchFrame::default()
        };
        let full = corpse(vec![WatchPackItem {
            serial: 0x4000_0201,
            graphic: 0x0EED,
            amount: 30,
            ..WatchPackItem::default()
        }]);
        assert!(GridLoot::new(CORPSE).alive(&full));
        assert!(!GridLoot::new(CORPSE).alive(&WatchFrame::default()));
        let id = GumpId::of(GRID_LOOT.id, CORPSE);
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        manager.open(id, &mut profile);
        if !draw_frames(&mut manager, &mut profile, &full) {
            return;
        }
        assert!(manager.is_open(&id));
        draw_frames(&mut manager, &mut profile, &corpse(Vec::new()));
        assert!(!manager.is_open(&id));
    }

    #[test]
    fn many_items_run_onto_more_pages() {
        let (cells, _, pages) = lay_cells(40);
        assert!(pages > 1);
        assert_eq!(cells.last().map(|cell| cell.page), Some(pages));
        let first_of_second = cells.iter().find(|cell| cell.page == 2).unwrap();
        assert_eq!(first_of_second.x, GAP);
    }
}
