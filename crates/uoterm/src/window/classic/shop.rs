//! The shop gump of the classic client, to buy
//! from a shopkeeper or sell to him: his list at the left, each good with
//! its picture, its name and price and how many are left; the deal at the
//! right, each taken good with its count and plus and minus buttons, the
//! total and, when buying, the gold of the character. A double click on a
//! good takes one, or all with Shift; the corner of the deal accepts, and
//! the scroll on it clears the deal. The gripper at the foot makes both
//! lists longer, and the profile keeps the length. The shared cart
//! (`model::deals`) keeps the counts, as the Modern shop panel does.

use super::canvas::{ButtonArt, Canvas};
use super::manager::GumpManager;
use super::registry::{Closing, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use crate::view::{WatchFrame, WatchGood, WatchShop};
use crate::window::control::Act;
use crate::window::model::deals::Cart;
use crate::window::settings::Profile;
use eframe::egui::Vec2;
use uoterm_nav::TextAlign;

/// The id of the shop gump; one list is open at a time.
const SHOP_ID: &str = "shop";

pub const SHOP: GumpKind = GumpKind {
    id: SHOP_ID,
    rules: GumpRules::DEFAULT,
    open: |_| Box::new(Shop::default()),
};

/// Opens the shop gump when the shard opens a list, as the classic client
/// does when the list comes.
pub fn sync(manager: &mut GumpManager, frame: &WatchFrame, profile: &mut Profile) {
    let id = GumpId::one(SHOP.id);
    if frame.shop.is_some() && !manager.is_open(&id) {
        manager.open(id, profile);
    }
}

const BUY_LEFT: u16 = 0x0870;
const BUY_RIGHT: u16 = 0x0871;
const SELL_LEFT: u16 = 0x0872;
const SELL_RIGHT: u16 = 0x0873;
const TOP_HEIGHT: i32 = 64;
const LEFT_BOTTOM_HEIGHT: i32 = 116;
const RIGHT_BOTTOM_HEIGHT: i32 = 93;
const RIGHT_OFFSET: i32 = 32;
/// The length of the lists the classic client starts with.
pub const FIRST_LIST_HEIGHT: i32 = 60;
const MOST_LIST_HEIGHT: i32 = 640;
const LIST_ROOM_BELOW: i32 = 50;
const AREA_WIDER: i32 = 5;
const HALF: i32 = 2;
const EXPANDER: ButtonArt = ButtonArt::new(0x082E, 0x082F, 0x082F);
const EXPANDER_LEFT: i32 = 10;
const EXPANDER_UP: i32 = 5;
const TOTAL_IN: i32 = RIGHT_OFFSET * 2 + 4;
const TOTAL_UP: i32 = RIGHT_OFFSET * 3 - 15;
const GOLD_AFTER_TOTAL: i32 = 120;
const SELL_TOTAL_FROM_RIGHT: i32 = RIGHT_OFFSET * 3;
const TEXT_FONT: u8 = 1;
const TEXT_HUE: u16 = 0x0386;
const ACCEPT_BOX: (i32, i32) = (34, 30);
const ACCEPT_UP: i32 = 50;
const CLEAR_AFTER_ACCEPT: i32 = 175;
const CLEAR_BOX: (i32, i32) = (20, 20);
const SCROLL_BOX: (i32, i32) = (18, 16);
const SCROLL_UP_FROM_RIGHT: i32 = 50;
const SCROLL_UP_ABOVE: i32 = 18;
/// A held scroll arrow scrolls this far this often.
const SCROLL_STEP: i32 = 50;
const SCROLL_SECONDS: f64 = 0.06;
const WORDS_ACCEPT: &str = "Accept";
const WORDS_CLEAR: &str = "Clear";
const WORDS_SCROLL_UP: &str = "Scroll Up";
const WORDS_SCROLL_DOWN: &str = "Scroll Down";
// A good of the list.
const GOOD_X: i32 = 5;
const GOOD_GAP: i32 = 2;
const GOOD_LINE: u16 = 0x0039;
const GOOD_LINE_AT: (i32, i32) = (10, 190);
const GOOD_TEXT_Y: i32 = 15;
const GOOD_NAME_X: i32 = 55;
const GOOD_NAME_WIDTH: u32 = 110;
const GOOD_AMOUNT_X: i32 = 168;
const GOOD_AMOUNT_WIDTH: u32 = 35;
const GOOD_LEAST_TEXT: i32 = 35;
const GOOD_TEXT_ROOM: i32 = 10;
const GOOD_LEAST_HEIGHT: i32 = 50;
const GOOD_ART_BOX: i32 = 50;
const GOOD_ART_LEFT: i32 = 5;
const GOOD_ART_DOWN: i32 = 10;
const CREATURE_AT: (i32, i32) = (-3, 20);
const CREATURE_MOST: f32 = 45.0;
const GOOD_HUE: u16 = 0x0219;
const SELECTED_HUE: u16 = 0x0021;
const QUARTER: i32 = 4;
/// Serials below this are mobiles, which a shopkeeper sells as pets.
const FIRST_ITEM_SERIAL: u32 = 0x4000_0000;
// A good of the deal.
const DEAL_NAME_X: i32 = 50;
const DEAL_NAME_WIDTH: u32 = 140;
const DEAL_AMOUNT_X: i32 = 10;
const DEAL_HUE: u16 = 0x021F;
const PLUS: u16 = 0x0037;
const MINUS: u16 = 0x0038;
const PLUS_AT: (i32, i32) = (190, 5);
const MINUS_AT: (i32, i32) = (210, 5);
/// A held plus or minus starts to repeat after this long, in seconds.
const REPEAT_AFTER: f64 = 0.5;
const REPEAT_EVERY: f64 = 0.045;

/// Every word of a name starts with a capital, as the list writes goods.
fn capitalized(name: &str) -> String {
    name.split(' ')
        .map(|word| {
            let mut letters = word.chars();
            letters.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(letters).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The words of a good of the list: its name and price.
fn good_words(good: &WatchGood) -> String {
    format!("{} at {}gp", capitalized(&good.item.name), good.price)
}

/// A button the player holds, and when it last fired.
#[derive(Clone, Copy, PartialEq)]
struct Held {
    key: (u32, bool),
    next: f64,
}

#[derive(Default)]
pub struct Shop {
    cart: Cart,
    selected: Option<u32>,
    /// The plus or minus of a good the player holds down.
    held: Option<Held>,
    /// The scroll arrow held, and when it scrolls next.
    scrolling: Option<(&'static str, i32, f64)>,
}

impl Shop {
    /// The goods of the list, with the stock that is not in the deal.
    fn list(&mut self, g: &mut Canvas<'_>, shop: &WatchShop, width: i32, map: u8) -> i32 {
        let mut y = 0;
        for good in &shop.goods {
            let serial = good.item.serial;
            let left = self.cart.left(good);
            y += GOOD_GAP;
            let hue = if self.selected == Some(serial) {
                SELECTED_HUE
            } else {
                GOOD_HUE
            };
            let name_look = TextLook::unicode(TEXT_FONT, hue).wrap(GOOD_NAME_WIDTH);
            let name_height = g.measure(&good_words(good), &name_look).y as i32;
            let tile_height = g
                .scene
                .item_tile(good.item.graphic)
                .map_or(0, |tile| i32::from(tile.height));
            let text_height = name_height.max(GOOD_LEAST_TEXT) + GOOD_TEXT_ROOM;
            let height = if serial >= FIRST_ITEM_SERIAL {
                text_height.max(tile_height)
            } else {
                text_height
            };
            let line = g.gump_size(GOOD_LINE).map_or(0, |size| size.y as i32);
            let row_height = height.max(GOOD_LEAST_HEIGHT) + line;
            let (line_x, line_width) = GOOD_LINE_AT;
            g.pic(GOOD_X + line_x, y, GOOD_LINE, 0);
            let left_end = g.gump_size(GOOD_LINE).map_or(0, |size| size.x as i32);
            let right = g.gump_size(GOOD_LINE + 2).map_or(0, |size| size.x as i32);
            g.pic_width(
                GOOD_X + line_x + left_end,
                y,
                GOOD_LINE + 1,
                0,
                line_width - left_end - right,
            );
            g.pic(GOOD_X + line_x + line_width - right, y, GOOD_LINE + 2, 0);
            if serial < FIRST_ITEM_SERIAL && shop.buying {
                if let Some((texture, sprite)) =
                    g.scene
                        .creature_picture(map, good.item.graphic, good.item.hue)
                {
                    let shown = crate::window::atlas::Sprite {
                        width: sprite.width.min(CREATURE_MOST),
                        height: sprite.height.min(CREATURE_MOST),
                        ..sprite
                    };
                    g.sprite(GOOD_X + CREATURE_AT.0, y + CREATURE_AT.1, texture, shown);
                }
            } else {
                let picture = g.item_size(good.item.graphic);
                let fit = (GOOD_ART_BOX as f32 / picture.x.max(1.0))
                    .min(height as f32 / picture.y.max(1.0))
                    .min(1.0);
                let shown = picture * fit;
                let x = GOOD_X + (GOOD_ART_BOX - shown.x as i32) / HALF - GOOD_ART_LEFT;
                let top = y + (height - shown.y as i32) / HALF + GOOD_ART_DOWN;
                g.scaled(fit, |g| {
                    g.item(
                        (x as f32 / fit) as i32,
                        (top as f32 / fit) as i32,
                        good.item.graphic,
                        good.item.hue,
                    );
                });
            }
            g.label(
                GOOD_X + GOOD_NAME_X,
                y + GOOD_TEXT_Y,
                &good_words(good),
                &name_look,
            );
            let amount_look = TextLook::unicode(TEXT_FONT, hue)
                .wrap(GOOD_AMOUNT_WIDTH)
                .aligned(TextAlign::Right);
            g.label(
                GOOD_X + GOOD_AMOUNT_X,
                y + GOOD_TEXT_Y + height / QUARTER,
                &left.to_string(),
                &amount_look,
            );
            let response = g.click_area(("good", serial), GOOD_X, y, width, row_height);
            g.tooltip(&good_words(good));
            if response.double_clicked() && left > 0 {
                let all = g.ui().input(|i| i.modifiers.shift);
                self.cart.take(good, all);
            } else if response.clicked() {
                self.selected = Some(serial);
            }
            y += row_height;
        }
        y
    }

    /// The goods of the deal, each with its count and its buttons.
    fn deal(&mut self, g: &mut Canvas<'_>, shop: &WatchShop, now: f64) -> i32 {
        let mut y = 0;
        let taken: Vec<u32> = self.cart.taken().to_vec();
        for serial in taken {
            let Some(good) = shop.goods.iter().find(|good| good.item.serial == serial) else {
                continue;
            };
            let look = TextLook::unicode(TEXT_FONT, DEAL_HUE).wrap(DEAL_NAME_WIDTH);
            let size = g.label(DEAL_NAME_X, y, &good_words(good), &look);
            let amount_look = TextLook::unicode(TEXT_FONT, DEAL_HUE)
                .wrap(GOOD_AMOUNT_WIDTH)
                .aligned(TextAlign::Right);
            g.label(
                DEAL_AMOUNT_X,
                y,
                &self.cart.count(serial).to_string(),
                &amount_look,
            );
            let all = g.ui().input(|i| i.modifiers.shift);
            for (up, (x, dy), picture) in [(true, PLUS_AT, PLUS), (false, MINUS_AT, MINUS)] {
                let response = g.pic_button(("step", serial, up), x, y + dy, picture, 0);
                let key = (serial, up);
                let fires = if response.is_pointer_button_down_on() {
                    match self.held.filter(|held| held.key == key) {
                        Some(held) if now >= held.next => {
                            self.held = Some(Held {
                                key,
                                next: now + REPEAT_EVERY,
                            });
                            true
                        }
                        Some(_) => false,
                        None => {
                            self.held = Some(Held {
                                key,
                                next: now + REPEAT_AFTER,
                            });
                            true
                        }
                    }
                } else {
                    if self.held.is_some_and(|held| held.key == key) {
                        self.held = None;
                    }
                    false
                };
                if !fires {
                    continue;
                }
                if up {
                    self.cart.add(good, 1);
                } else {
                    let count = self.cart.count(serial);
                    self.cart.put_back(serial, if all { count } else { 1 });
                }
            }
            y += size.y as i32;
        }
        if self.held.is_some() {
            g.ctx().request_repaint();
        }
        y
    }

    /// An arrow of a list that scrolls it while it is held.
    fn scroll_arrow(
        &mut self,
        g: &mut Canvas<'_>,
        (x, y): (i32, i32),
        area: &'static str,
        delta: i32,
        words: &str,
        now: f64,
    ) {
        let (w, h) = SCROLL_BOX;
        let response = g.click_area(("arrow", area, delta), x, y, w, h);
        g.tooltip(words);
        if !response.is_pointer_button_down_on() {
            if self
                .scrolling
                .is_some_and(|(held, by, _)| held == area && by == delta)
            {
                self.scrolling = None;
            }
            return;
        }
        let due = match self.scrolling {
            Some((held, by, next)) if held == area && by == delta => now >= next,
            _ => true,
        };
        if due {
            g.scroll_by(area, delta);
            self.scrolling = Some((area, delta, now + SCROLL_SECONDS));
        }
        g.ctx().request_repaint();
    }
}

const LIST_AREA: &str = "list";
const DEAL_AREA: &str = "deal";

impl GumpBody for Shop {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(shop) = cx.frame.shop.clone() else {
            return;
        };
        self.cart.follow(&shop);
        let now = g.ctx().input(|i| i.time);
        let (left, right) = if shop.buying {
            (BUY_LEFT, BUY_RIGHT)
        } else {
            (SELL_LEFT, SELL_RIGHT)
        };
        let left_size = g.gump_size(left).unwrap_or(Vec2::ZERO);
        let right_size = g.gump_size(right).unwrap_or(Vec2::ZERO);
        let (left_w, left_h) = (left_size.x as i32, left_size.y as i32);
        let (right_w, right_h) = (right_size.x as i32, right_size.y as i32);
        let least = left_h - (LEFT_BOTTOM_HEIGHT + TOP_HEIGHT);
        let right_least = right_h - (RIGHT_BOTTOM_HEIGHT + TOP_HEIGHT);
        let list_height = g
            .size()
            .map_or(FIRST_LIST_HEIGHT, |size| size.y as i32)
            .clamp(least, MOST_LIST_HEIGHT);
        let grown = list_height - least;
        let right_height = (right_least + grown).clamp(right_least, MOST_LIST_HEIGHT - TOP_HEIGHT);
        // The left side: its top, its middle laid down, its foot.
        g.pic_band(0, 0, left, (0, TOP_HEIGHT), TOP_HEIGHT);
        g.pic_band(0, TOP_HEIGHT, left, (TOP_HEIGHT, least), list_height);
        let left_foot = TOP_HEIGHT + list_height;
        g.pic_band(
            0,
            left_foot,
            left,
            (left_h - LEFT_BOTTOM_HEIGHT, LEFT_BOTTOM_HEIGHT),
            LEFT_BOTTOM_HEIGHT,
        );
        // The right side, over the left.
        let right_x = left_w - RIGHT_OFFSET;
        let right_y = left_h / HALF - RIGHT_OFFSET;
        g.pic_band(right_x, right_y, right, (0, TOP_HEIGHT), TOP_HEIGHT);
        g.pic_band(
            right_x,
            right_y + TOP_HEIGHT,
            right,
            (TOP_HEIGHT, right_least),
            right_height,
        );
        let right_foot = right_y + TOP_HEIGHT + right_height;
        g.pic_band(
            right_x,
            right_foot,
            right,
            (right_h - RIGHT_BOTTOM_HEIGHT, RIGHT_BOTTOM_HEIGHT),
            RIGHT_BOTTOM_HEIGHT,
        );
        let right_end = right_foot + RIGHT_BOTTOM_HEIGHT;
        // The two lists.
        let list_width = left_w - RIGHT_OFFSET * HALF + AREA_WIDER;
        g.scroll_area(
            LIST_AREA,
            RIGHT_OFFSET,
            TOP_HEIGHT,
            list_width,
            list_height + LIST_ROOM_BELOW,
            |g| self.list(g, &shop, list_width, cx.frame.map),
        );
        g.scroll_area(
            DEAL_AREA,
            RIGHT_OFFSET / HALF + right_x,
            TOP_HEIGHT + right_y,
            right_w - RIGHT_OFFSET * HALF + RIGHT_OFFSET / HALF + AREA_WIDER,
            right_height,
            |g| self.deal(g, &shop, now),
        );
        // The total, and the gold of a buyer.
        let look = TextLook::unicode(TEXT_FONT, TEXT_HUE);
        let total_y = right_end - TOTAL_UP;
        let total_x = if shop.buying {
            right_x + TOTAL_IN
        } else {
            right_x + right_w - SELL_TOTAL_FROM_RIGHT
        };
        g.label(
            total_x,
            total_y,
            &self.cart.total(&shop.goods).to_string(),
            &look,
        );
        if shop.buying {
            g.label(
                total_x + GOLD_AFTER_TOTAL,
                total_y,
                &cx.frame.gold.to_string(),
                &look,
            );
        }
        // The gripper that makes the lists longer.
        let expander = g.pic_button(
            "expander",
            left_w / HALF - EXPANDER_LEFT,
            left_foot + LEFT_BOTTOM_HEIGHT - EXPANDER_UP,
            EXPANDER.normal,
            0,
        );
        if expander.dragged() {
            let steps = (expander.drag_delta().y / cx.profile.video.ui_scale) as i32;
            let height = (list_height + steps).clamp(least, MOST_LIST_HEIGHT);
            g.keep_size(Vec2::new(left_w as f32, height as f32));
        }
        // Accept and clear.
        let (accept_w, accept_h) = ACCEPT_BOX;
        let accept_x = RIGHT_OFFSET + right_x;
        let accept_y = right_end - ACCEPT_UP;
        let accepted = g
            .click_area("accept", accept_x, accept_y, accept_w, accept_h)
            .clicked();
        g.tooltip(WORDS_ACCEPT);
        let (clear_w, clear_h) = CLEAR_BOX;
        let cleared = g
            .click_area(
                "clear",
                accept_x + CLEAR_AFTER_ACCEPT,
                accept_y,
                clear_w,
                clear_h,
            )
            .clicked();
        g.tooltip(WORDS_CLEAR);
        if accepted {
            let rows = self.cart.rows(&shop.goods);
            if rows.is_empty() {
                cx.act(Act::ShopClose);
            } else {
                cx.act(Act::Checkout(rows));
            }
        } else if cleared {
            self.cart.clear();
        }
        // The arrows of the two lists.
        let left_arrow_x = left_w - SCROLL_UP_FROM_RIGHT;
        let right_arrow_x = right_x + right_w - SCROLL_UP_FROM_RIGHT;
        self.scroll_arrow(
            g,
            (left_arrow_x, TOP_HEIGHT - SCROLL_UP_ABOVE),
            LIST_AREA,
            -SCROLL_STEP,
            WORDS_SCROLL_UP,
            now,
        );
        self.scroll_arrow(
            g,
            (left_arrow_x, left_foot),
            LIST_AREA,
            SCROLL_STEP,
            WORDS_SCROLL_DOWN,
            now,
        );
        self.scroll_arrow(
            g,
            (right_arrow_x, right_y + TOP_HEIGHT - SCROLL_UP_ABOVE),
            DEAL_AREA,
            -SCROLL_STEP,
            WORDS_SCROLL_UP,
            now,
        );
        self.scroll_arrow(
            g,
            (right_arrow_x, right_foot),
            DEAL_AREA,
            SCROLL_STEP,
            WORDS_SCROLL_DOWN,
            now,
        );
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(Act::ShopClose);
        Closing::Wait
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        frame.shop.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchPackItem;

    #[test]
    fn a_good_reads_with_capitals_and_its_price() {
        let good = WatchGood {
            item: WatchPackItem {
                name: "clean bandage".into(),
                ..WatchPackItem::default()
            },
            price: 6,
        };
        assert_eq!(good_words(&good), "Clean Bandage at 6gp");
        assert_eq!(capitalized("a  b"), "A  B");
    }

    #[test]
    fn the_shop_opens_with_a_list_draws_it_and_closes_without_one() {
        use crate::window::classic::testing::draw_frames;
        let frame = WatchFrame {
            shop: Some(WatchShop {
                vendor: 0x0000_0101,
                buying: true,
                goods: vec![WatchGood {
                    item: WatchPackItem {
                        serial: 0x4000_0001,
                        graphic: 0x0E21,
                        amount: 20,
                        name: "bandage".into(),
                        ..WatchPackItem::default()
                    },
                    price: 6,
                }],
                ..WatchShop::default()
            }),
            ..WatchFrame::default()
        };
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let id = GumpId::one(SHOP.id);
        sync(&mut manager, &WatchFrame::default(), &mut profile);
        assert!(!manager.is_open(&id));
        sync(&mut manager, &frame, &mut profile);
        assert!(manager.is_open(&id));
        if !draw_frames(&mut manager, &mut profile, &frame) {
            return;
        }
        assert!(manager.is_open(&id));
        draw_frames(&mut manager, &mut profile, &WatchFrame::default());
        assert!(!manager.is_open(&id));
    }
}
