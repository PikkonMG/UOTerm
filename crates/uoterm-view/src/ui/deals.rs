//! The Modern windows where goods change hands, as both windows show them:
//! the list of a shopkeeper with its cart, and each trade with another
//! player. Where each first stands, its words, what a double click on a
//! good and the deal button do, and the coins the player offers. The cart
//! and the coins follow `model::deals`, as the classic gumps do.

use super::gumps::{CELL, CELL_GAP};
use super::layout::{first_place, Spot};
use super::places::{FOOT_ROW, TITLE_ROW};
use super::theme::{self, PANEL_PAD};
use crate::act::Act;
use crate::frame::{WatchGood, WatchShop, WatchTrade};
use crate::geom::{Area, Rgba, Vector};
use crate::model::deals::{typed_offer, Cart};

/// The id that keeps the place of the shop in the profile.
pub const SHOP_ID: &str = "modern:shop";
const TRADE_PLACE_ID: &str = "modern:trade:";
pub const SHOP_WIDTH: f32 = 460.0;
/// The goods the shop shows as it first opens, and the fewest it shows.
pub const SHOP_ROWS: usize = 8;
pub const SHOP_LEAST_ROWS: usize = 3;
pub const SHOP_ROW: f32 = 34.0;
/// The cells of each side of a trade.
pub const TRADE_COLUMNS: usize = 4;
pub const TRADE_ROWS: usize = 3;
pub const TRADE_GAP: f32 = 24.0;
pub const SIDE_HEAD: f32 = 26.0;
pub const COIN_ROW: f32 = 30.0;
pub const COIN_ROWS: usize = 2;
/// The words of an offer field start as nothing offered.
pub const NOTHING_OFFERED: &str = "0";

pub const WORDS_BUY: &str = "Buy";
pub const WORDS_SELL: &str = "Sell";
const WORDS_FROM: &str = "from";
const WORDS_TO: &str = "to";
pub const WORDS_CLEAR: &str = "Clear";
pub const WORDS_CLOSE: &str = "Close";
const WORDS_TOTAL: &str = "Total";
pub const WORDS_GOLD: &str = "Gold";
pub const WORDS_PLATINUM: &str = "Platinum";
const WORDS_GP: &str = "gp";
const WORDS_OF: &str = "of";
const WORDS_TRADE_WITH: &str = "Trade with";
pub const WORDS_ACCEPT: &str = "Accept";
pub const WORDS_UNDO_ACCEPT: &str = "Undo accept";
pub const WORDS_CANCEL: &str = "Cancel";
pub const WORDS_YOU: &str = "You";
const WORDS_ACCEPTED: &str = "accepted";
const WORDS_THINKS: &str = "not yet";
pub const STEP_DOWN: &str = "-";
pub const STEP_UP: &str = "+";
pub const HINT_GOOD: &str = "Double-click: take one.  Shift: all.";
pub const HINT_TRADED: &str = "Double-click: use.  Drag: take it back.";
pub const HINT_THEIRS: &str = "Double-click: use.";

/// The height of the shop with room for `rows` goods.
pub fn shop_height(rows: usize) -> f32 {
    PANEL_PAD * 2.0 + TITLE_ROW + rows as f32 * SHOP_ROW + FOOT_ROW * 2.0
}

/// Where the shop first stands in `window`.
pub fn shop_first_place(window: Area) -> Area {
    first_place(
        window,
        Spot::Middle(0),
        Vector::new(SHOP_WIDTH, shop_height(SHOP_ROWS)),
    )
}

/// The least size the player may make the shop.
pub fn shop_least() -> Vector {
    Vector::new(SHOP_WIDTH, shop_height(SHOP_LEAST_ROWS))
}

/// How many goods a shop of `height` shows.
pub fn shop_rows(height: f32) -> usize {
    let list = height - shop_height(0);
    ((list / SHOP_ROW).floor() as usize).max(1)
}

/// The words of the deal button and the title of a shop: what the
/// character does and with whom.
pub fn shop_words(shop: &WatchShop) -> (&'static str, String) {
    let (deal, link) = if shop.buying {
        (WORDS_BUY, WORDS_FROM)
    } else {
        (WORDS_SELL, WORDS_TO)
    };
    (deal, format!("{deal} {link} {}", shop.vendor_name))
}

/// How many of a good are left past the cart.
pub fn left_words(left: u16) -> String {
    format!("x{left}")
}

pub fn price_words(price: u32) -> String {
    format!("{price} {WORDS_GP}")
}

pub fn total_words(total: u64) -> String {
    format!("{WORDS_TOTAL}  {total} {WORDS_GP}")
}

pub fn gold_words(gold: &str) -> String {
    format!("{WORDS_GOLD}  {gold}")
}

/// What a coin field says the player has.
pub fn owned_words(owned: &str) -> String {
    format!("{WORDS_OF} {owned}")
}

/// A double click on a good takes one more of it, or with Shift all that
/// is left.
pub fn take_good(cart: &mut Cart, good: &WatchGood, all: bool) {
    if cart.left(good) > 0 {
        cart.take(good, all);
    }
}

/// The deal button sends the cart, or with nothing in it closes the shop.
pub fn deal_act(cart: &Cart, shop: &WatchShop) -> Act {
    let rows = cart.rows(&shop.goods);
    if rows.is_empty() {
        Act::ShopClose
    } else {
        Act::Checkout(rows)
    }
}

/// The id that keeps the place of the `index`th trade, from 0.
pub fn trade_id(index: usize) -> String {
    format!("{TRADE_PLACE_ID}{}", index + 1)
}

pub fn trade_title(trade: &WatchTrade) -> String {
    format!("{WORDS_TRADE_WITH} {}", trade.with)
}

/// The width and the height of the cells of one side of a trade.
pub fn trade_side() -> Vector {
    Vector::new(
        TRADE_COLUMNS as f32 * (CELL + CELL_GAP) - CELL_GAP,
        TRADE_ROWS as f32 * (CELL + CELL_GAP) - CELL_GAP,
    )
}

/// Where the `index`th trade first stands in `window`.
pub fn trade_first_place(window: Area, index: usize) -> Area {
    let side = trade_side();
    let size = Vector::new(
        side.x * 2.0 + TRADE_GAP + PANEL_PAD * 2.0,
        TITLE_ROW + SIDE_HEAD + side.y + COIN_ROWS as f32 * COIN_ROW + FOOT_ROW + PANEL_PAD * 2.0,
    );
    first_place(window, Spot::Middle(index), size)
}

/// The head of a side of a trade: who, whether he accepted, and its color.
pub fn side_head(who: &str, accepted: bool) -> (String, Rgba) {
    let (mark, color) = if accepted {
        (WORDS_ACCEPTED, theme::HITS_POISONED)
    } else {
        (WORDS_THINKS, theme::TEXT_FAINT)
    };
    (format!("{who}: {mark}"), color)
}

/// The accept button: Accept, or Undo accept once the character accepted.
pub fn accept_button(i_accept: bool) -> (&'static str, Rgba) {
    if i_accept {
        (WORDS_UNDO_ACCEPT, theme::WAITING)
    } else {
        (WORDS_ACCEPT, theme::GOAL)
    }
}

/// What a click on an item of a trade does, as the tip says.
pub fn traded_footer(live: bool, mine: bool) -> &'static str {
    match (live, mine) {
        (false, _) => "",
        (true, true) => HINT_TRADED,
        (true, false) => HINT_THEIRS,
    }
}

/// The coins the player typed and offered in one trade.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TradeOffer {
    pub gold: String,
    pub platinum: String,
    /// The gold and platinum sent to the shard.
    pub offered: (u32, u32),
}

impl Default for TradeOffer {
    fn default() -> Self {
        Self {
            gold: NOTHING_OFFERED.into(),
            platinum: NOTHING_OFFERED.into(),
            offered: (0, 0),
        }
    }
}

impl TradeOffer {
    /// The player typed `words` in the gold (or the platinum) field. The
    /// offer is held to what he has; the field shows the held words. Gives
    /// the act that tells the shard, when the offer changed.
    pub fn typed(&mut self, trade: &WatchTrade, platinum: bool, words: &str) -> Option<Act> {
        let have = (trade.my_gold, trade.my_platinum);
        let mut offered = self.offered;
        let shown =
            typed_offer(&mut offered, platinum, words, have).unwrap_or_else(|| words.into());
        if platinum {
            self.platinum = shown;
        } else {
            self.gold = shown;
        }
        (offered != self.offered).then(|| {
            self.offered = offered;
            Act::TradeGold {
                trade: trade.mine,
                gold: offered.0,
                platinum: offered.1,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::WatchPackItem;

    const BOX: u32 = 0x4000_0300;
    const BANDAGE: u32 = 5;

    fn shop() -> WatchShop {
        WatchShop {
            vendor: 9,
            vendor_name: "Bob".into(),
            buying: true,
            goods: vec![WatchGood {
                item: WatchPackItem {
                    serial: BANDAGE,
                    amount: 100,
                    name: "Bandage".into(),
                    ..WatchPackItem::default()
                },
                price: 2,
            }],
        }
    }

    #[test]
    fn the_shop_is_as_tall_as_its_rows_and_its_foot() {
        assert_eq!(
            shop_height(SHOP_ROWS) - shop_height(SHOP_LEAST_ROWS),
            (SHOP_ROWS - SHOP_LEAST_ROWS) as f32 * SHOP_ROW
        );
        assert_eq!(shop_rows(shop_height(SHOP_ROWS)), SHOP_ROWS);
        assert_eq!(shop_words(&shop()).1, "Buy from Bob");
    }

    #[test]
    fn a_double_click_takes_one_and_shift_takes_all_then_the_deal_sends_them() {
        let shop = shop();
        let mut cart = Cart::default();
        cart.follow(&shop);
        assert_eq!(
            deal_act(&cart, &shop),
            Act::ShopClose,
            "an empty cart closes"
        );
        take_good(&mut cart, &shop.goods[0], false);
        assert_eq!(cart.count(BANDAGE), 1);
        take_good(&mut cart, &shop.goods[0], true);
        assert_eq!(cart.count(BANDAGE), 100);
        assert!(matches!(deal_act(&cart, &shop), Act::Checkout(rows) if rows.len() == 1));
    }

    #[test]
    fn an_offer_is_held_to_what_the_player_has_and_sent_once() {
        let trade = WatchTrade {
            mine: BOX,
            my_gold: 500,
            ..WatchTrade::default()
        };
        let mut offer = TradeOffer::default();
        assert_eq!(offer.gold, NOTHING_OFFERED);
        let act = offer.typed(&trade, false, "900");
        assert_eq!(
            act,
            Some(Act::TradeGold {
                trade: BOX,
                gold: 500,
                platinum: 0
            })
        );
        assert_eq!(offer.gold, "500");
        assert_eq!(offer.typed(&trade, false, "500"), None, "no change");
        assert_eq!(accept_button(true).0, WORDS_UNDO_ACCEPT);
        assert_eq!(side_head(WORDS_YOU, false).0, "You: not yet");
    }
}
