//! The two windows where goods change hands: the list of a shopkeeper with
//! a cart, and the trade with another player. Each one shows at all times,
//! so the operator sees what the agent does. The clicks work only while the
//! human has control.

use super::boxes_ui::{scrolled, Tools, CELL, CELL_GAP, CELL_RADIUS};
use super::control::Act;
use super::desk::Zone;
use super::theme::{self, number_font, text_font, title_font};
use crate::view::{WatchFrame, WatchGood, WatchPackItem, WatchShop, WatchTrade};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Vec2};
use std::collections::HashMap;

const SHOP_WIDTH: f32 = 440.0;
const SHOP_ROWS: usize = 8;
const SHOP_ROW: f32 = 34.0;
const TITLE_ROW: f32 = 32.0;
const FOOT_ROW: f32 = 40.0;
const STEP_SIDE: f32 = 22.0;
const COUNT_WIDTH: f32 = 40.0;
const ART_SIDE: f32 = 30.0;
const TRADE_COLUMNS: usize = 4;
const TRADE_ROWS: usize = 3;
const TRADE_GAP: f32 = 24.0;
const GOLD_FIELD_WIDTH: f32 = 110.0;
/// Shift with a step button moves the count by this many.
const BIG_STEP: u16 = 10;

const WORDS_BUY: &str = "Buy";
const WORDS_SELL: &str = "Sell";
const WORDS_CLOSE: &str = "Close";
const WORDS_ACCEPT: &str = "Accept";
const WORDS_CANCEL: &str = "Cancel";
const WORDS_OFFER_GOLD: &str = "Offer gold";
const WORDS_YOU: &str = "You";
const WORDS_ACCEPTED: &str = "accepted";
const WORDS_THINKS: &str = "not yet";
const HINT_GOLD: &str = "gold";

#[derive(Default)]
pub struct DealUi {
    /// The shopkeeper the cart is for. Another shopkeeper empties it.
    cart_of: Option<(u32, bool)>,
    cart: HashMap<u32, u16>,
    first_good: usize,
    gold: String,
}

/// The count of one row after a step, kept from zero to the stock.
fn stepped(count: u16, up: bool, big: bool, stock: u16) -> u16 {
    let by = if big { BIG_STEP } else { 1 };
    if up {
        count.saturating_add(by).min(stock)
    } else {
        count.saturating_sub(by)
    }
}

fn cart_total(goods: &[WatchGood], cart: &HashMap<u32, u16>) -> u64 {
    goods
        .iter()
        .map(|good| {
            let count = cart.get(&good.item.serial).copied().unwrap_or(0);
            u64::from(good.price) * u64::from(count)
        })
        .sum()
}

/// The rows of the cart that hold something, in the order of the list.
fn cart_rows(goods: &[WatchGood], cart: &HashMap<u32, u16>) -> Vec<(u32, u16)> {
    goods
        .iter()
        .filter_map(|good| {
            let count = cart.get(&good.item.serial).copied().filter(|n| *n > 0)?;
            Some((good.item.serial, count))
        })
        .collect()
}

fn item_cell(
    ui: &egui::Ui,
    cell: Rect,
    item: &WatchPackItem,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    with_amount: bool,
) {
    let painter = ui.painter();
    painter.rect_filled(cell, CornerRadius::same(CELL_RADIUS), theme::TRACK);
    if let Some((texture, sprite)) = tools.scene.item_picture(frame.map, item.graphic, item.hue) {
        let area = theme::fit(cell, sprite.width, sprite.height);
        painter.image(texture, area, sprite.uv, Color32::WHITE);
    }
    if with_amount && item.amount > 1 {
        theme::shadowed_text(
            painter,
            cell.right_bottom() - Vec2::splat(theme::CELL_ART_PAD),
            Align2::RIGHT_BOTTOM,
            &item.amount.to_string(),
            number_font(theme::SIZE_SMALL),
            theme::TEXT,
        );
    }
    let hovered = ui
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|mouse| cell.contains(mouse));
    if hovered && !tools.desk.carries() {
        tools
            .tips
            .point_at(ui, tools.hand, item.serial, &item.name, "", tools.time);
    }
}

impl DealUi {
    /// Draws the shop list and the trade, when they are open. Gives the
    /// places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Vec<Rect> {
        let mut covered = Vec::new();
        match &frame.shop {
            Some(shop) => covered.push(self.shop(ui, rect, shop, frame, tools)),
            None => {
                self.cart_of = None;
                self.cart.clear();
            }
        }
        if let Some(trade) = &frame.trade {
            covered.push(self.trade(ui, rect, trade, frame, tools));
        }
        covered
    }

    fn shop(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        shop: &WatchShop,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Rect {
        if self.cart_of != Some((shop.vendor, shop.buying)) {
            self.cart_of = Some((shop.vendor, shop.buying));
            self.cart.clear();
            self.first_good = 0;
        }
        let rows = shop.goods.len().clamp(1, SHOP_ROWS);
        let panel = Rect::from_center_size(
            rect.center(),
            Vec2::new(
                SHOP_WIDTH,
                theme::PANEL_PAD * 2.0 + TITLE_ROW + rows as f32 * SHOP_ROW + FOOT_ROW,
            ),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        let deal_words = if shop.buying { WORDS_BUY } else { WORDS_SELL };
        let link_word = if shop.buying { "from" } else { "to" };
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            format!("{deal_words} {link_word} {}", shop.vendor_name),
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let last_first = shop.goods.len().saturating_sub(SHOP_ROWS);
        self.first_good = scrolled(ui, panel, self.first_good, last_first);
        let live = frame.human_control;
        let big = ui.input(|i| i.modifiers.shift);
        let shown = shop.goods.iter().skip(self.first_good).take(SHOP_ROWS);
        for (i, good) in shown.enumerate() {
            let row = Rect::from_min_size(
                inner.left_top() + Vec2::new(0.0, TITLE_ROW + i as f32 * SHOP_ROW),
                Vec2::new(inner.width(), SHOP_ROW),
            );
            let art = Rect::from_center_size(
                Pos2::new(row.left() + ART_SIDE / 2.0, row.center().y),
                Vec2::splat(ART_SIDE),
            );
            item_cell(ui, art, &good.item, frame, tools, false);
            let painter = ui.painter();
            let name = painter.text(
                Pos2::new(art.right() + theme::ROW_GAP, row.center().y),
                Align2::LEFT_CENTER,
                &good.item.name,
                text_font(theme::SIZE_BODY),
                theme::TEXT,
            );
            painter.text(
                Pos2::new(name.right() + theme::ROW_GAP * 2.0, row.center().y),
                Align2::LEFT_CENTER,
                format!("x{}", good.item.amount),
                number_font(theme::SIZE_SMALL),
                theme::TEXT_FAINT,
            );
            let count = self.cart.get(&good.item.serial).copied().unwrap_or(0);
            let plus = Rect::from_center_size(
                Pos2::new(row.right() - STEP_SIDE / 2.0, row.center().y),
                Vec2::splat(STEP_SIDE),
            );
            let minus = plus.translate(Vec2::new(-(STEP_SIDE + COUNT_WIDTH), 0.0));
            painter.text(
                Pos2::new((plus.left() + minus.right()) / 2.0, row.center().y),
                Align2::CENTER_CENTER,
                count.to_string(),
                number_font(theme::SIZE_BODY),
                if count > 0 {
                    theme::GOAL
                } else {
                    theme::TEXT_DIM
                },
            );
            painter.text(
                Pos2::new(minus.left() - theme::ROW_GAP * 2.0, row.center().y),
                Align2::RIGHT_CENTER,
                format!("{} gp", good.price),
                number_font(theme::SIZE_BODY),
                theme::WAITING,
            );
            if !live {
                continue;
            }
            for (area, words, up) in [(minus, "-", false), (plus, "+", true)] {
                let key = Id::new(("shop-step", good.item.serial, up));
                if theme::segment_keyed(ui, area, key, words, theme::TEXT) {
                    let next = stepped(count, up, big, good.item.amount);
                    self.cart.insert(good.item.serial, next);
                }
            }
        }
        let foot = Rect::from_min_max(
            Pos2::new(inner.left(), inner.bottom() - FOOT_ROW + theme::ROW_GAP),
            inner.right_bottom(),
        );
        ui.painter().text(
            foot.right_center(),
            Align2::RIGHT_CENTER,
            format!("Total  {} gp", cart_total(&shop.goods, &self.cart)),
            number_font(theme::SIZE_BODY),
            theme::TEXT,
        );
        if live {
            let (deal, dealt) = theme::button(ui, foot.left_top(), deal_words, theme::GOAL);
            let (_, closed) = theme::button(
                ui,
                Pos2::new(deal.right() + theme::ROW_GAP, foot.top()),
                WORDS_CLOSE,
                theme::TEXT_DIM,
            );
            let rows = cart_rows(&shop.goods, &self.cart);
            if dealt && !rows.is_empty() {
                tools.hand.act(Act::Checkout(rows));
            } else if closed {
                tools.hand.act(Act::ShopClose);
            }
        }
        panel
    }

    fn trade(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        trade: &WatchTrade,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) -> Rect {
        let side_width = TRADE_COLUMNS as f32 * (CELL + CELL_GAP) - CELL_GAP;
        let side_height = TRADE_ROWS as f32 * (CELL + CELL_GAP) - CELL_GAP;
        let panel = Rect::from_center_size(
            rect.center(),
            Vec2::new(
                side_width * 2.0 + TRADE_GAP + theme::PANEL_PAD * 2.0,
                TITLE_ROW * 2.0 + side_height + FOOT_ROW + theme::PANEL_PAD * 2.0,
            ),
        );
        theme::panel(ui.painter(), panel);
        let inner = panel.shrink(theme::PANEL_PAD);
        ui.painter().text(
            inner.left_top(),
            Align2::LEFT_TOP,
            format!("Trade with {}", trade.with),
            title_font(theme::SIZE_TITLE),
            theme::TEXT,
        );
        let sides = [
            (WORDS_YOU, trade.i_accept, &trade.mine_items, 0.0),
            (
                trade.with.as_str(),
                trade.they_accept,
                &trade.their_items,
                side_width + TRADE_GAP,
            ),
        ];
        for (who, accepted, items, shift) in sides {
            let top = inner.left_top() + Vec2::new(shift, TITLE_ROW);
            let (mark, color) = if accepted {
                (WORDS_ACCEPTED, theme::HITS_POISONED)
            } else {
                (WORDS_THINKS, theme::TEXT_FAINT)
            };
            ui.painter().text(
                top,
                Align2::LEFT_TOP,
                format!("{who}: {mark}"),
                text_font(theme::SIZE_BODY),
                color,
            );
            let cells = Rect::from_min_size(
                top + Vec2::new(0.0, TITLE_ROW),
                Vec2::new(side_width, side_height),
            );
            ui.painter()
                .rect_filled(cells, CornerRadius::same(CELL_RADIUS), theme::TRACK);
            for (i, item) in items.iter().take(TRADE_COLUMNS * TRADE_ROWS).enumerate() {
                let cell = Rect::from_min_size(
                    cells.left_top()
                        + Vec2::new(
                            (i % TRADE_COLUMNS) as f32 * (CELL + CELL_GAP),
                            (i / TRADE_COLUMNS) as f32 * (CELL + CELL_GAP),
                        ),
                    Vec2::splat(CELL),
                );
                item_cell(ui, cell, item, frame, tools, true);
            }
        }
        // An item dropped anywhere on the trade goes on the side of the human.
        tools.desk.zone(panel, Zone::Into(trade.mine));
        if !frame.human_control {
            return panel;
        }
        let foot = Pos2::new(inner.left(), inner.bottom() - FOOT_ROW + theme::ROW_GAP);
        let accept_color = if trade.i_accept {
            theme::TEXT_DIM
        } else {
            theme::GOAL
        };
        let (accept, accepted) = theme::button(ui, foot, WORDS_ACCEPT, accept_color);
        let (cancel, canceled) = theme::button(
            ui,
            Pos2::new(accept.right() + theme::ROW_GAP, foot.y),
            WORDS_CANCEL,
            theme::ALARM,
        );
        let field = Rect::from_min_size(
            Pos2::new(cancel.right() + TRADE_GAP, foot.y),
            Vec2::new(GOLD_FIELD_WIDTH, cancel.height()),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        ui.put(
            field,
            egui::TextEdit::singleline(&mut self.gold)
                .frame(false)
                .hint_text(HINT_GOLD)
                .margin(egui::Margin::symmetric(8, 6))
                .font(number_font(theme::SIZE_BODY))
                .text_color(theme::TEXT),
        );
        let (_, offered) = theme::button(
            ui,
            Pos2::new(field.right() + theme::ROW_GAP, foot.y),
            WORDS_OFFER_GOLD,
            theme::WAITING,
        );
        if accepted {
            tools.hand.act(Act::TradeAccept);
        } else if canceled {
            tools.hand.act(Act::TradeCancel);
        } else if let Some(gold) = self.gold.trim().parse().ok().filter(|_| offered) {
            tools.hand.act(Act::TradeGold { gold, platinum: 0 });
        }
        panel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BANDAGE: u32 = 0x4000_0001;
    const ARROW: u32 = 0x4000_0002;

    fn good(serial: u32, price: u32, stock: u16) -> WatchGood {
        WatchGood {
            item: WatchPackItem {
                serial,
                amount: stock,
                ..WatchPackItem::default()
            },
            price,
        }
    }

    #[test]
    fn a_count_stays_from_zero_to_the_stock() {
        assert_eq!(stepped(0, true, false, 20), 1);
        assert_eq!(stepped(15, true, true, 20), 20);
        assert_eq!(stepped(3, false, true, 20), 0);
        assert_eq!(stepped(0, false, false, 20), 0);
    }

    #[test]
    fn the_cart_is_the_rows_with_a_count_and_their_price() {
        let goods = vec![good(BANDAGE, 6, 20), good(ARROW, 3, 500)];
        let mut cart = HashMap::new();
        cart.insert(ARROW, 100);
        cart.insert(BANDAGE, 0);
        assert_eq!(cart_rows(&goods, &cart), vec![(ARROW, 100)]);
        assert_eq!(cart_total(&goods, &cart), 300);
    }
}
