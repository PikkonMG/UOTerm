//! The secure trade gump of the classic client, one for each trade that is open: the character's box and
//! the other player's, each item at its place, the accept boxes, and from
//! the 7.0.45.65 clients on the gold and platinum each side offers, with the
//! fields the character types his offer in. An item dropped on his box lands
//! at the place of the mouse. A right click closes the trade.

use super::canvas::Canvas;
use super::item_control::{
    ask_waiting_name, item_look, item_tooltip, pick_up, single_click, started_drag, ClickDelay,
};
use super::manager::GumpManager;
use super::registry::{Closing, GumpBody, GumpContext, GumpId, GumpKind, GumpRules};
use super::text::TextLook;
use super::text_field::TextField;
use crate::view::{WatchFrame, WatchPackItem, WatchTrade};
use crate::window::control::{Act, DropTo};
use crate::window::model::deals::{typed_offer, with_thousands};
use crate::window::settings::Profile;
use eframe::egui::Vec2;

/// The id of the gump of one trade, by the character's box of it.
const TRADE_ID: &str = "trade";

pub const TRADE: GumpKind = GumpKind {
    id: TRADE_ID,
    rules: GumpRules {
        kept: false,
        ..GumpRules::DEFAULT
    },
    open: |serial| Box::new(Trade::new(serial.unwrap_or_default())),
};

/// Opens a trade gump for each trade the shard opened.
pub fn sync(manager: &mut GumpManager, frame: &WatchFrame, profile: &mut Profile) {
    for trade in &frame.trades {
        let id = GumpId::of(TRADE.id, trade.mine);
        if !manager.is_open(&id) {
            manager.open(id, profile);
        }
    }
}

/// The two looks of the trade gump: the gold and platinum look of the newer
/// clients, and the old one.
struct Look {
    background: u16,
    name_font: u8,
    name_hue: u16,
    my_name: (i32, i32),
    /// The other name ends at this x.
    their_name_end: i32,
    their_name_y: i32,
    my_box: (i32, i32),
    their_box: (i32, i32),
    my_check: (i32, i32),
    their_check: (i32, i32),
    coins: bool,
}

const NEW_LOOK: Look = Look {
    background: 0x088A,
    name_font: 3,
    name_hue: 0x0481,
    my_name: (73, 32),
    their_name_end: 250,
    their_name_y: 244,
    my_box: (30, 110),
    their_box: (192, 110),
    my_check: (37, 29),
    their_check: (258, 240),
    coins: true,
};

const OLD_LOOK: Look = Look {
    background: 0x0866,
    name_font: 1,
    name_hue: 0x0386,
    my_name: (84, 40),
    their_name_end: 260,
    their_name_y: 170,
    my_box: (45, 70),
    their_box: (192, 70),
    my_check: (52, 29),
    their_check: (266, 160),
    coins: false,
};

const BOX_SIZE: (i32, i32) = (110, 80);
const CHECK_OFF: (u16, u16) = (0x0867, 0x0868);
const CHECK_ON: (u16, u16) = (0x0869, 0x086A);
const COIN_FONT: u8 = 9;
const COIN_HUE: u16 = 0x0481;
const MY_GOLD_AT: (i32, i32) = (43, 67);
const MY_PLATINUM_AT: (i32, i32) = (180, 67);
const THEIR_GOLD_AT: (i32, i32) = (180, 190);
const THEIR_PLATINUM_AT: (i32, i32) = (180, 210);
const GOLD_FIELD_AT: (i32, i32) = (43, 190);
const PLATINUM_FIELD_AT: (i32, i32) = (43, 210);
const FIELD_SIZE: (i32, i32) = (100, 20);
const HALF: i32 = 2;

/// The place of an item in a box of the trade, kept in the box by the
/// size of its picture.
fn in_box(x: i32, y: i32, picture: Vec2) -> (i32, i32) {
    let (w, h) = BOX_SIZE;
    let x = x.min(w - picture.x as i32).max(0);
    let y = y.min(h - picture.y as i32).max(0);
    (x, y)
}

pub struct Trade {
    /// The character's box, which names the trade.
    mine: u32,
    gold: TextField,
    platinum: TextField,
    /// The gold and platinum sent to the shard.
    offered: (u32, u32),
    clicks: ClickDelay,
}

impl Trade {
    pub fn new(mine: u32) -> Self {
        let field = || {
            let mut field = TextField::new("0");
            field.numeric = true;
            field
        };
        Self {
            mine,
            gold: field(),
            platinum: field(),
            offered: (0, 0),
            clicks: ClickDelay::default(),
        }
    }

    fn trade<'f>(&self, frame: &'f WatchFrame) -> Option<&'f WatchTrade> {
        frame.trades.iter().find(|trade| trade.mine == self.mine)
    }

    /// The items of one side, at their places in its box.
    fn side(
        &mut self,
        g: &mut Canvas<'_>,
        cx: &mut GumpContext<'_>,
        items: &[WatchPackItem],
        (left, top): (i32, i32),
    ) {
        for item in items {
            let picture = g.item_size(item.graphic);
            let (x, y) = in_box(i32::from(item.x), i32::from(item.y), picture);
            let (x, y) = (left + x, top + y);
            let marked = g.hovered(x, y, picture.x as i32, picture.y as i32);
            let look = item_look(g.scene, item, marked, 1.0);
            let Some(response) = g.item_button(("item", item.serial), x, y, look) else {
                continue;
            };
            item_tooltip(g, cx, item);
            if started_drag(&response) {
                pick_up(g, cx, item, response.rect.center());
            } else if response.double_clicked() {
                self.clicks.double_clicked();
                cx.act(Act::Use(item.serial));
            } else if response.clicked() {
                single_click(g, cx, &mut self.clicks, item.serial);
            }
        }
    }

    /// Lands the item on the mouse in the character's box, at the place of
    /// the mouse.
    fn land(&self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, (left, top): (i32, i32)) {
        let (w, h) = BOX_SIZE;
        if cx.desk.carried().is_none() || !g.hovered(left, top, w, h) {
            return;
        }
        let Some((carried, _)) = cx.desk.land(g.ui()) else {
            return;
        };
        let Some(mouse) = g.ui().input(|i| i.pointer.interact_pos()) else {
            return;
        };
        let at = (mouse - g.at(left, top)) / cx.profile.video.ui_scale;
        let picture = g.item_size(carried.graphic);
        let (x, y) = in_box(
            at.x as i32 - picture.x as i32 / HALF,
            at.y as i32 - picture.y as i32 / HALF,
            picture,
        );
        cx.act(Act::Move {
            item: carried.serial,
            amount: carried.amount.max(1),
            to: DropTo::IntoAt {
                container: self.mine,
                x: x as u16,
                y: y as u16,
            },
        });
    }

    /// The fields the character types his gold and platinum offer in. A
    /// change is held to what he has and sent at once.
    fn offer(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>, trade: &WatchTrade) {
        let look = TextLook::ascii(COIN_FONT, 0);
        let (w, h) = FIELD_SIZE;
        let have = (trade.my_gold, trade.my_platinum);
        let mut offered = self.offered;
        for (platinum, (x, y)) in [(false, GOLD_FIELD_AT), (true, PLATINUM_FIELD_AT)] {
            let field = if platinum {
                &mut self.platinum
            } else {
                &mut self.gold
            };
            let typed = g.text_box(("offer", platinum), x, y, w, h, field, &look);
            if !typed.changed {
                continue;
            }
            if let Some(words) = typed_offer(&mut offered, platinum, field.text(), have) {
                field.set_text(&words);
            }
        }
        if offered != self.offered {
            self.offered = offered;
            cx.act(Act::TradeGold {
                trade: self.mine,
                gold: offered.0,
                platinum: offered.1,
            });
        }
    }
}

impl GumpBody for Trade {
    fn draw(&mut self, g: &mut Canvas<'_>, cx: &mut GumpContext<'_>) {
        let Some(trade) = self.trade(cx.frame).cloned() else {
            return;
        };
        let look = if g.gump_size(NEW_LOOK.background).is_some() {
            NEW_LOOK
        } else {
            OLD_LOOK
        };
        g.pic(0, 0, look.background, 0);
        let names = TextLook::ascii(look.name_font, look.name_hue);
        g.label(look.my_name.0, look.my_name.1, &cx.frame.name, &names);
        let their_width = g.measure(&trade.with, &names).x as i32;
        g.label(
            look.their_name_end - their_width,
            look.their_name_y,
            &trade.with,
            &names,
        );
        if look.coins {
            let coins = TextLook::ascii(COIN_FONT, COIN_HUE);
            for ((x, y), amount) in [
                (MY_GOLD_AT, trade.my_gold),
                (MY_PLATINUM_AT, trade.my_platinum),
                (THEIR_GOLD_AT, trade.their_gold),
                (THEIR_PLATINUM_AT, trade.their_platinum),
            ] {
                g.label(x, y, &with_thousands(amount), &coins);
            }
            self.offer(g, cx, &trade);
        }
        let mut accept = trade.i_accept;
        let pictures = if trade.i_accept { CHECK_ON } else { CHECK_OFF };
        if g.checkbox(
            "accept",
            look.my_check.0,
            look.my_check.1,
            pictures,
            &mut accept,
            None,
        ) {
            cx.act(Act::TradeAccept {
                trade: self.mine,
                accept,
            });
        }
        let theirs = if trade.they_accept {
            CHECK_ON.0
        } else {
            CHECK_OFF.0
        };
        g.pic(look.their_check.0, look.their_check.1, theirs, 0);
        self.side(g, cx, &trade.mine_items, look.my_box);
        self.side(g, cx, &trade.their_items, look.their_box);
        ask_waiting_name(g, cx, &mut self.clicks);
        self.land(g, cx, look.my_box);
    }

    fn close(&mut self, cx: &mut GumpContext<'_>) -> Closing {
        cx.act(Act::TradeCancel(self.mine));
        Closing::Wait
    }

    fn alive(&self, frame: &WatchFrame) -> bool {
        self.trade(frame).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_item_stays_inside_its_box() {
        let picture = Vec2::new(20.0, 10.0);
        assert_eq!(in_box(5, 5, picture), (5, 5));
        assert_eq!(in_box(100, 75, picture), (90, 70));
        assert_eq!(in_box(-4, -4, picture), (0, 0));
    }

    #[test]
    fn each_trade_opens_its_gump_and_closes_with_it() {
        use crate::window::classic::testing::draw_frames;
        const MINE: u32 = 0x4000_0C11;
        let frame = WatchFrame {
            trades: vec![WatchTrade {
                with: "Ann".into(),
                mine: MINE,
                they_accept: true,
                their_gold: 1500,
                their_items: vec![WatchPackItem {
                    serial: 0x4000_0C21,
                    graphic: 0x0EED,
                    amount: 1,
                    x: 30,
                    y: 20,
                    ..WatchPackItem::default()
                }],
                ..WatchTrade::default()
            }],
            ..WatchFrame::default()
        };
        let mut manager = GumpManager::default();
        let mut profile = Profile::default();
        let id = GumpId::of(TRADE.id, MINE);
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
