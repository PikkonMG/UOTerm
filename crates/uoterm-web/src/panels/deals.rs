//! The windows where goods change hands: the list of a shopkeeper with a
//! cart, and each trade with another player. Their clicks work only while
//! the human has control, as in the Rust window; the cart, the coins and
//! what a click does are the rules of `uoterm_view::ui::deals` and
//! `model::deals`. A double click on a good takes one, and with Shift all.

use super::{Colored, DropZone, FrameSpec, Framed, TipKey, PANEL_SHOP, PANEL_TRADE_PREFIX};
use crate::WebView;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uoterm_view::act::Act;
use uoterm_view::frame::{WatchFrame, WatchPackItem, WatchTrade};
use uoterm_view::model::clicks::ClickDelay;
use uoterm_view::model::deals::{stepped, with_thousands, Cart};
use uoterm_view::ui::deals::{
    accept_button, deal_act, gold_words, left_words, owned_words, price_words, shop_first_place,
    shop_least, shop_words, side_head, take_good, total_words, trade_first_place, trade_id,
    trade_title, traded_footer, TradeOffer, HINT_GOOD, SHOP_ID, STEP_DOWN, STEP_UP, WORDS_CANCEL,
    WORDS_CLEAR, WORDS_CLOSE, WORDS_GOLD, WORDS_PLATINUM, WORDS_YOU,
};
use uoterm_view::ui::gumps::single_or_double;
use uoterm_view::ui::theme::{css_color, GOAL, TEXT_DIM};

/// What the shop and the trades keep between frames.
#[derive(Default)]
pub(crate) struct DealsState {
    /// The cart of the list that is open. Another list empties it.
    cart: Cart,
    /// What the player typed and offered in each trade, by the
    /// character's box of it.
    offers: HashMap<u32, TradeOffer>,
    /// A click on an item asks its name once no double click follows.
    clicks: ClickDelay,
}

/// The list of a shopkeeper.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ShopData {
    pub live: bool,
    pub goods: Vec<GoodRow>,
    pub total: String,
    /// The gold the character has, while he buys.
    pub gold: Option<String>,
    pub deal: &'static str,
    pub clear: &'static str,
    pub close: &'static str,
    pub step_down: &'static str,
    pub step_up: &'static str,
}

/// One good of the list.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GoodRow {
    pub serial: u32,
    pub picture: Option<String>,
    pub name: String,
    /// How many are left past the cart.
    pub left: String,
    pub price: String,
    /// How many the cart takes, in its color.
    pub count: Colored,
    pub hover: TipKey,
}

/// One trade.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TradeData {
    pub live: bool,
    /// The character's side, then the other one.
    pub sides: Vec<TradeSide>,
    /// The gold and platinum fields of the character's offer.
    pub coins: Vec<CoinField>,
    /// The gold and platinum the other player offers.
    pub theirs: Vec<Fact>,
    pub accept: Option<Colored>,
    pub cancel: Option<&'static str>,
    /// An item dropped anywhere on the trade goes on the character's side.
    pub zone: DropZone,
}

/// One side of a trade: who, whether he accepted, and the items.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TradeSide {
    pub head: Colored,
    pub mine: bool,
    pub items: Vec<TradedItem>,
}

/// One item of a trade.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TradedItem {
    pub serial: u32,
    pub picture: Option<String>,
    pub amount: Option<String>,
    pub hover: TipKey,
}

/// A coin field of the character's offer.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CoinField {
    pub label: &'static str,
    /// Platinum, not gold.
    pub platinum: bool,
    pub words: String,
    pub owned: String,
}

/// Words and a value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Fact {
    pub label: &'static str,
    pub value: String,
}

/// A step button of a good: `{"step": {serial, up, big}}`; Shift makes it
/// a big step.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct Step {
    serial: u32,
    up: bool,
    #[serde(default)]
    big: bool,
}

/// A double click on a good: `{"take": serial, "all": shift}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
struct Take {
    take: u32,
    #[serde(default)]
    all: bool,
}

/// What the player does on the shop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ShopAction {
    Click(u32),
    Step(Step),
    Deal(bool),
    Clear(bool),
}

/// What the player does on a trade.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TradeAction {
    Click(u32),
    Double(u32),
    Drag(u32),
    Gold(String),
    Platinum(String),
    Accept(bool),
    Cancel(bool),
}

impl WebView {
    /// The shop and the trades in one frame: the cart follows the list
    /// that is open, the offers go with their trades, and a click that
    /// waited long enough asks the name.
    pub(crate) fn follow_deals(&mut self, frame: &WatchFrame, time: f64) {
        let deals = &mut self.panels.deals;
        match &frame.shop {
            Some(shop) => {
                deals.cart.follow(shop);
            }
            None => deals.cart.clear(),
        }
        deals
            .offers
            .retain(|mine, _| frame.trades.iter().any(|trade| trade.mine == *mine));
        if let Some(act) = deals.clicks.due_look(time) {
            self.hand.act(act);
        }
    }

    /// How many of a good the cart takes.
    #[cfg(test)]
    pub(crate) fn cart_amount(&self, serial: u32) -> u16 {
        self.panels.deals.cart.count(serial)
    }

    pub(super) fn shop_spec(&self, frame: &WatchFrame) -> Option<FrameSpec> {
        let shop = frame.shop.as_ref()?;
        let (_, title) = shop_words(shop);
        let spec = FrameSpec::fixed(SHOP_ID, &title, shop_first_place(self.panel_room()))
            .sized(shop_least());
        Some(if frame.human_control {
            spec.closable()
        } else {
            spec
        })
    }

    pub(super) fn shop_data(&mut self, frame: &WatchFrame) -> Option<Framed<ShopData>> {
        let shop = frame.shop.as_ref()?;
        let spec = self.shop_spec(frame)?;
        let live = frame.human_control;
        let (deal, _) = shop_words(shop);
        let mut goods = Vec::new();
        for good in &shop.goods {
            let count = self.panels.deals.cart.count(good.item.serial);
            let picture = self.item_picture(&good.item);
            goods.push(GoodRow {
                serial: good.item.serial,
                picture,
                name: good.item.name.clone(),
                left: left_words(self.panels.deals.cart.left(good)),
                price: price_words(good.price),
                count: Colored {
                    words: count.to_string(),
                    color: css_color(if count > 0 { GOAL } else { TEXT_DIM }),
                },
                hover: TipKey::thing(
                    good.item.serial,
                    &good.item.name,
                    if live { HINT_GOOD } else { "" },
                ),
            });
        }
        let body = ShopData {
            live,
            goods,
            total: total_words(self.panels.deals.cart.total(&shop.goods)),
            gold: shop.buying.then(|| gold_words(&with_thousands(frame.gold))),
            deal,
            clear: WORDS_CLEAR,
            close: WORDS_CLOSE,
            step_down: STEP_DOWN,
            step_up: STEP_UP,
        };
        Some(self.framed(PANEL_SHOP, &spec, body))
    }

    /// The picture of an item in a cell of a panel, asked for.
    pub(super) fn item_picture(&mut self, item: &WatchPackItem) -> Option<String> {
        let request = self.item_picture_request(item.graphic, item.hue);
        Some(self.picture_key(&request))
    }

    pub(super) fn shop_action(&mut self, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Some(shop) = frame.shop.as_ref() else {
            return;
        };
        let time = self.hand.time();
        if let Ok(take) = serde_json::from_value::<Take>(action.clone()) {
            let double = (false, true);
            let deals = &mut self.panels.deals;
            let (doubled, _) = single_or_double(double, &mut deals.clicks, &frame, take.take, time);
            let good = shop.goods.iter().find(|good| good.item.serial == take.take);
            if let Some(good) = good.filter(|_| doubled) {
                take_good(&mut deals.cart, good, take.all);
            }
            return;
        }
        let Ok(action) = serde_json::from_value::<ShopAction>(action) else {
            return;
        };
        let deals = &mut self.panels.deals;
        match action {
            ShopAction::Click(serial) => {
                let (_, act) =
                    single_or_double((true, false), &mut deals.clicks, &frame, serial, time);
                if let Some(act) = act {
                    self.hand.act(act);
                }
            }
            ShopAction::Step(step) => {
                if let Some(good) = shop
                    .goods
                    .iter()
                    .find(|good| good.item.serial == step.serial)
                {
                    let count = deals.cart.count(step.serial);
                    let next = stepped(count, step.up, step.big, good.item.amount);
                    deals.cart.set(step.serial, next);
                }
            }
            ShopAction::Deal(_) => {
                let act = deal_act(&deals.cart, shop);
                self.hand.act(act);
            }
            ShopAction::Clear(_) => deals.cart.clear(),
        }
    }

    /// The trade whose panel is `panel`, and where it stands in the list.
    fn trade_of<'a>(frame: &'a WatchFrame, panel: &str) -> Option<(usize, &'a WatchTrade)> {
        let mine: u32 = panel.strip_prefix(PANEL_TRADE_PREFIX)?.parse().ok()?;
        frame
            .trades
            .iter()
            .enumerate()
            .find(|(_, trade)| trade.mine == mine)
    }

    pub(super) fn trade_spec(&self, frame: &WatchFrame, panel: &str) -> Option<FrameSpec> {
        let (index, trade) = Self::trade_of(frame, panel)?;
        let default = trade_first_place(self.panel_room(), index);
        let spec = FrameSpec::fixed(&trade_id(index), &trade_title(trade), default);
        Some(if frame.human_control {
            spec.closable()
        } else {
            spec
        })
    }

    pub(super) fn trades_data(&mut self, frame: &WatchFrame) -> Vec<Framed<TradeData>> {
        let mut trades = Vec::new();
        for trade in &frame.trades {
            let panel = format!("{PANEL_TRADE_PREFIX}{}", trade.mine);
            let Some(spec) = self.trade_spec(frame, &panel) else {
                continue;
            };
            let body = self.trade_body(frame, trade);
            trades.push(self.framed(&panel, &spec, body));
        }
        trades
    }

    fn trade_body(&mut self, frame: &WatchFrame, trade: &WatchTrade) -> TradeData {
        let live = frame.human_control;
        let mut sides = Vec::new();
        for (who, accepted, items, mine) in [
            (WORDS_YOU, trade.i_accept, &trade.mine_items, true),
            (
                trade.with.as_str(),
                trade.they_accept,
                &trade.their_items,
                false,
            ),
        ] {
            let (words, color) = side_head(who, accepted);
            let items = items
                .iter()
                .map(|item| TradedItem {
                    serial: item.serial,
                    picture: self.item_picture(item),
                    amount: (item.amount > 1).then(|| item.amount.to_string()),
                    hover: TipKey::thing(item.serial, &item.name, traded_footer(live, mine)),
                })
                .collect();
            sides.push(TradeSide {
                head: Colored {
                    words,
                    color: css_color(color),
                },
                mine,
                items,
            });
        }
        let offer = self.panels.deals.offers.entry(trade.mine).or_default();
        let coins = vec![
            CoinField {
                label: WORDS_GOLD,
                platinum: false,
                words: offer.gold.clone(),
                owned: owned_words(&with_thousands(trade.my_gold)),
            },
            CoinField {
                label: WORDS_PLATINUM,
                platinum: true,
                words: offer.platinum.clone(),
                owned: owned_words(&with_thousands(trade.my_platinum)),
            },
        ];
        let theirs = vec![
            Fact {
                label: WORDS_GOLD,
                value: with_thousands(trade.their_gold),
            },
            Fact {
                label: WORDS_PLATINUM,
                value: with_thousands(trade.their_platinum),
            },
        ];
        let accept = live.then(|| {
            let (words, color) = accept_button(trade.i_accept);
            Colored {
                words: words.to_string(),
                color: css_color(color),
            }
        });
        TradeData {
            live,
            sides,
            coins,
            theirs,
            accept,
            cancel: live.then_some(WORDS_CANCEL),
            zone: DropZone::Into(trade.mine),
        }
    }

    pub(super) fn trade_action(&mut self, panel: &str, action: Value) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        let Some((_, trade)) = Self::trade_of(&frame, panel) else {
            return;
        };
        let Ok(action) = serde_json::from_value::<TradeAction>(action) else {
            return;
        };
        let time = self.hand.time();
        let clicks = &mut self.panels.deals.clicks;
        match action {
            TradeAction::Click(serial) => {
                let (_, act) = single_or_double((true, false), clicks, &frame, serial, time);
                if let Some(act) = act {
                    self.hand.act(act);
                }
            }
            TradeAction::Double(serial) => {
                single_or_double((false, true), clicks, &frame, serial, time);
                self.hand.act(Act::Use(serial));
            }
            TradeAction::Drag(serial) => {
                if let Some(item) = trade.mine_items.iter().find(|item| item.serial == serial) {
                    self.pick_up(item);
                }
            }
            TradeAction::Gold(words) => self.offer_typed(trade, false, &words),
            TradeAction::Platinum(words) => self.offer_typed(trade, true, &words),
            TradeAction::Accept(_) => self.hand.act(Act::TradeAccept {
                trade: trade.mine,
                accept: !trade.i_accept,
            }),
            TradeAction::Cancel(_) => self.hand.act(Act::TradeCancel(trade.mine)),
        }
    }

    fn offer_typed(&mut self, trade: &WatchTrade, platinum: bool, words: &str) {
        let offer = self.panels.deals.offers.entry(trade.mine).or_default();
        if let Some(act) = offer.typed(trade, platinum, words) {
            self.hand.act(act);
        }
    }

    /// Closes the shop or a trade by its close mark, while the human has
    /// control.
    pub(super) fn close_deal(&mut self, panel: &str) {
        let Some(frame) = self.frame.clone().filter(|frame| frame.human_control) else {
            return;
        };
        if panel == PANEL_SHOP {
            self.hand.act(Act::ShopClose);
        } else if let Some((_, trade)) = Self::trade_of(&frame, panel) {
            self.hand.act(Act::TradeCancel(trade.mine));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{out_acts, press};
    use super::super::{PANEL_SHOP, PANEL_TRADE_PREFIX};
    use crate::tests::fixture_watch_with_backpack;
    use crate::WebView;
    use serde_json::json;
    use uoterm_view::act::Act;
    use uoterm_view::frame::{WatchGood, WatchPackItem};
    use uoterm_view::model::deals::Cart;
    use uoterm_view::ui::deals::{take_good, TradeOffer};

    const BOX: u32 = 0x4000_0300;
    const SWORD: u32 = 0x4000_0301;
    const RING: u32 = 0x4000_0302;

    fn view_with(key: &str, value: serde_json::Value) -> WebView {
        let mut watch: serde_json::Value =
            serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch[key] = value;
        let mut view = crate::tests::settled();
        view.frame(&watch.to_string(), 0.1);
        view
    }

    fn view_with_trade() -> WebView {
        view_with(
            "trades",
            json!([{ "with": "Ann", "box_serial": BOX, "my_gold": 500,
                "mine_items": [{ "serial": SWORD, "graphic": 3937, "amount": 1, "name": "sword" }],
                "their_items": [{ "serial": RING, "graphic": 4234, "amount": 1, "name": "ring" }] }]),
        )
    }

    fn trade_panel() -> String {
        format!("{PANEL_TRADE_PREFIX}{BOX}")
    }

    fn view_with_shop(goods: &[(u32, u16)]) -> WebView {
        let goods: Vec<_> = goods
            .iter()
            .map(|(serial, amount)| {
                json!({"serial": serial, "graphic": 3617, "amount": amount, "price": 2,
                       "name": "Bandage"})
            })
            .collect();
        view_with(
            "shop",
            json!({"vendor": 9, "vendor_name": "Bob", "buying": true, "goods": goods}),
        )
    }

    fn without_control(view: &mut WebView, key: &str, value: serde_json::Value) {
        let mut watch: serde_json::Value =
            serde_json::from_str(&fixture_watch_with_backpack()).unwrap();
        watch[key] = value;
        watch["human_control"] = json!(false);
        view.frame(&watch.to_string(), 0.2);
    }

    #[test]
    fn a_shift_take_puts_the_whole_stack_in_the_cart() {
        let mut view = view_with_shop(&[(5, 100)]);
        view.input_native(
            &json!({"kind": "Panel", "panel": "shop", "action": {"take": 5, "all": true}})
                .to_string(),
            0.0,
        );
        assert_eq!(view.cart_amount(5), 100);
        let out = press(&mut view, PANEL_SHOP, json!({"deal": true}));
        assert_eq!(
            out_acts(&out),
            vec![Act::Checkout(vec![(5, 100)]).for_page()]
        );
    }

    #[test]
    fn a_double_click_takes_one_as_the_window_and_the_steps_count() {
        let mut view = view_with_shop(&[(5, 100)]);
        let shop = view.panel_data(0.0).shop.unwrap();
        assert_eq!(shop.frame.title, "Buy from Bob");
        assert_eq!(shop.body.goods[0].left, "x100");
        press(&mut view, PANEL_SHOP, json!({"take": 5, "all": false}));
        let good = WatchGood {
            item: WatchPackItem {
                serial: 5,
                amount: 100,
                ..WatchPackItem::default()
            },
            price: 2,
        };
        let mut cart = Cart::default();
        take_good(&mut cart, &good, false);
        assert_eq!(view.cart_amount(5), cart.count(5));
        press(
            &mut view,
            PANEL_SHOP,
            json!({"step": {"serial": 5, "up": true, "big": true}}),
        );
        assert_eq!(view.cart_amount(5), 11);
        press(
            &mut view,
            PANEL_SHOP,
            json!({"step": {"serial": 5, "up": false}}),
        );
        assert_eq!(view.cart_amount(5), 10);
        press(&mut view, PANEL_SHOP, json!({"clear": true}));
        assert_eq!(view.cart_amount(5), 0);
        let out = press(&mut view, PANEL_SHOP, json!({"deal": true}));
        assert_eq!(
            out_acts(&out),
            vec![Act::ShopClose.for_page()],
            "an empty cart closes"
        );
        let out = press(&mut view, PANEL_SHOP, json!({"close": true}));
        assert_eq!(out_acts(&out), vec![Act::ShopClose.for_page()]);
    }

    #[test]
    fn the_shop_takes_nothing_without_control() {
        let mut view = view_with_shop(&[(5, 100)]);
        without_control(
            &mut view,
            "shop",
            json!({"vendor": 9, "vendor_name": "Bob", "buying": true, "goods": [
                {"serial": 5, "graphic": 3617, "amount": 100, "price": 2, "name": "Bandage"}]}),
        );
        press(&mut view, PANEL_SHOP, json!({"take": 5, "all": true}));
        assert_eq!(view.cart_amount(5), 0);
        assert!(out_acts(&press(&mut view, PANEL_SHOP, json!({"deal": true}))).is_empty());
        assert!(!view.panel_data(0.0).shop.unwrap().frame.closable);
    }

    #[test]
    fn a_trade_offers_gold_accepts_and_cancels_as_the_window() {
        let mut view = view_with_trade();
        let trade = view.panel_data(0.0).trades.remove(0);
        assert_eq!(trade.frame.title, "Trade with Ann");
        assert_eq!(trade.body.sides[0].items[0].serial, SWORD);
        let panel = trade_panel();
        let out = press(&mut view, &panel, json!({"gold": "900"}));
        let watched = view.frame_ref().unwrap().trades[0].clone();
        let mut offer = TradeOffer::default();
        let same = offer.typed(&watched, false, "900").unwrap();
        assert_eq!(out_acts(&out), vec![same.for_page()]);
        assert_eq!(view.panel_data(0.0).trades[0].body.coins[0].words, "500");
        let out = press(&mut view, &panel, json!({"accept": true}));
        let accept = Act::TradeAccept {
            trade: BOX,
            accept: true,
        };
        assert_eq!(out_acts(&out), vec![accept.for_page()]);
        let out = press(&mut view, &panel, json!({"cancel": true}));
        assert_eq!(out_acts(&out), vec![Act::TradeCancel(BOX).for_page()]);
        let out = press(&mut view, &panel, json!({"close": true}));
        assert_eq!(out_acts(&out), vec![Act::TradeCancel(BOX).for_page()]);
    }

    #[test]
    fn an_own_traded_item_drags_back_and_a_double_click_uses_one() {
        let mut view = view_with_trade();
        let panel = trade_panel();
        press(&mut view, &panel, json!({"drag": RING}));
        assert!(!view.carries(), "the other side stays");
        press(&mut view, &panel, json!({"drag": SWORD}));
        assert!(view.carries());
        let out = press(&mut view, &panel, json!({"double": RING}));
        assert_eq!(out_acts(&out), vec![Act::Use(RING).for_page()]);
    }

    #[test]
    fn a_trade_acts_not_without_control() {
        let mut view = view_with_trade();
        without_control(
            &mut view,
            "trades",
            json!([{ "with": "Ann", "box_serial": BOX, "my_gold": 500 }]),
        );
        let panel = trade_panel();
        for action in [
            json!({"gold": "10"}),
            json!({"accept": true}),
            json!({"cancel": true}),
            json!({"double": RING}),
            json!({"close": true}),
        ] {
            assert!(out_acts(&press(&mut view, &panel, action)).is_empty());
        }
        assert!(view.panel_data(0.0).trades[0].body.accept.is_none());
    }
}
