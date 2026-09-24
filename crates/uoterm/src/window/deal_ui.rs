//! The windows where goods change hands: the list of a shopkeeper with a
//! cart, and each trade with another player. Each one shows at all times,
//! so the operator sees what the agent does, and the player moves, locks
//! and (the shop) sizes it. The clicks work only while the human has
//! control.
//!
//! As in the classic client: a click on an item targets it while the shard
//! waits for a target, and else asks its name; a double click on a good
//! takes one more (all with Shift), and on a traded item uses it. The cart
//! and the offer of coins follow `model::deals`, as the classic gumps do.

use super::boxes_ui::{
    ask_waiting_name, scrolled, single_or_double, Tools, CELL, CELL_GAP, CELL_RADIUS,
};
use super::control::Act;
use super::desk::Zone;
use super::model::clicks::ClickDelay;
use super::model::deals::{stepped, typed_offer, with_thousands, Cart};
use super::modern::frame::{self, FrameEvent, PanelSpec};
use super::modern::layout::{self, Spot};
use super::settings::Profile;
use super::theme::{self, number_font, text_font};
use crate::view::{WatchFrame, WatchGood, WatchPackItem, WatchShop, WatchTrade};
use eframe::egui::{self, Align2, Color32, CornerRadius, Id, Pos2, Rect, Sense, Vec2};
use std::collections::HashMap;

const SHOP_ID: &str = "modern:shop";
const TRADE_PLACE_ID: &str = "modern:trade:";
const SHOP_WIDTH: f32 = 460.0;
const SHOP_ROWS: usize = 8;
const SHOP_LEAST_ROWS: usize = 3;
const SHOP_ROW: f32 = 34.0;
const FOOT_ROW: f32 = 40.0;
const STEP_SIDE: f32 = 22.0;
const COUNT_WIDTH: f32 = 40.0;
const ART_SIDE: f32 = 30.0;
const TRADE_COLUMNS: usize = 4;
const TRADE_ROWS: usize = 3;
const TRADE_GAP: f32 = 24.0;
const SIDE_HEAD: f32 = 26.0;
const COIN_ROW: f32 = 30.0;
const COIN_ROWS: usize = 2;
const COIN_LABEL_WIDTH: f32 = 70.0;
const COIN_FIELD_WIDTH: f32 = 90.0;
/// The words of an offer field start as nothing offered.
const NOTHING_OFFERED: &str = "0";

const WORDS_BUY: &str = "Buy";
const WORDS_SELL: &str = "Sell";
const WORDS_FROM: &str = "from";
const WORDS_TO: &str = "to";
const WORDS_CLEAR: &str = "Clear";
const WORDS_CLOSE: &str = "Close";
const WORDS_TOTAL: &str = "Total";
const WORDS_GOLD: &str = "Gold";
const WORDS_PLATINUM: &str = "Platinum";
const WORDS_GP: &str = "gp";
const WORDS_OF: &str = "of";
const WORDS_TRADE_WITH: &str = "Trade with";
const WORDS_ACCEPT: &str = "Accept";
const WORDS_UNDO_ACCEPT: &str = "Undo accept";
const WORDS_CANCEL: &str = "Cancel";
const WORDS_YOU: &str = "You";
const WORDS_ACCEPTED: &str = "accepted";
const WORDS_THINKS: &str = "not yet";
const HINT_GOOD: &str = "Double-click: take one.  Shift: all.";
const HINT_TRADED: &str = "Double-click: use.  Drag: take it back.";
const HINT_THEIRS: &str = "Double-click: use.";

/// What the player typed and offered in one trade.
struct TradeState {
    gold: String,
    platinum: String,
    /// The gold and platinum sent to the shard.
    offered: (u32, u32),
    /// The first row shown of the character's side and of the other one.
    first_rows: [usize; 2],
}

impl Default for TradeState {
    fn default() -> Self {
        Self {
            gold: NOTHING_OFFERED.into(),
            platinum: NOTHING_OFFERED.into(),
            offered: (0, 0),
            first_rows: [0; 2],
        }
    }
}

#[derive(Default)]
pub struct DealUi {
    /// The cart of the list that is open. Another list empties it.
    cart: Cart,
    first_good: usize,
    /// Each trade that is open, by the character's box of it.
    trades: HashMap<u32, TradeState>,
    clicks: ClickDelay,
}

/// The height of the shop panel with room for `rows` goods.
fn shop_height(rows: usize) -> f32 {
    theme::PANEL_PAD * 2.0 + frame::TITLE_ROW + rows as f32 * SHOP_ROW + FOOT_ROW * 2.0
}

/// The picture of an item in a cell, with its amount when asked.
fn item_art(
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
}

/// The tooltip of an item under the mouse: the words of the shard, or its
/// name until they come.
fn item_tip(
    ui: &egui::Ui,
    response: &egui::Response,
    item: &WatchPackItem,
    tools: &mut Tools<'_>,
    footer: &str,
) {
    if response.hovered() && !tools.desk.carries() && !tools.ring.is_open() {
        tools
            .tips
            .point_at(ui, tools.hand, item.serial, &item.name, footer, tools.time);
    }
}

impl DealUi {
    /// Draws the shop list and each trade, when they are open. Gives the
    /// places they cover.
    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Vec<Rect> {
        let mut covered = Vec::new();
        match &frame.shop {
            Some(shop) => covered.push(self.shop(ui, rect, shop, frame, tools, profile)),
            None => self.cart.clear(),
        }
        self.trades
            .retain(|mine, _| frame.trades.iter().any(|trade| trade.mine == *mine));
        for (index, trade) in frame.trades.iter().enumerate() {
            covered.push(self.trade(ui, rect, index, trade, frame, tools, profile));
        }
        ask_waiting_name(ui, &mut self.clicks, tools.hand, tools.time);
        covered
    }

    fn shop(
        &mut self,
        ui: &egui::Ui,
        rect: Rect,
        shop: &WatchShop,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        if self.cart.follow(shop) {
            self.first_good = 0;
        }
        let live = frame.human_control;
        let (deal_words, link_word) = if shop.buying {
            (WORDS_BUY, WORDS_FROM)
        } else {
            (WORDS_SELL, WORDS_TO)
        };
        let title = format!("{deal_words} {link_word} {}", shop.vendor_name);
        let spec = PanelSpec {
            id: SHOP_ID,
            title: &title,
            default: layout::first_place(
                rect,
                Spot::Middle(0),
                Vec2::new(SHOP_WIDTH, shop_height(SHOP_ROWS)),
            ),
            min_size: Some(Vec2::new(SHOP_WIDTH, shop_height(SHOP_LEAST_ROWS))),
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, &title);
        let list = Rect::from_min_max(
            body.min,
            Pos2::new(body.right(), body.bottom() - FOOT_ROW * 2.0),
        );
        let rows = ((list.height() / SHOP_ROW).floor() as usize).max(1);
        let last_first = shop.goods.len().saturating_sub(rows);
        self.first_good = scrolled(ui, list, self.first_good, last_first);
        let shown = shop.goods.iter().skip(self.first_good).take(rows);
        for (at, good) in shown.enumerate() {
            let row = Rect::from_min_size(
                list.left_top() + Vec2::new(0.0, at as f32 * SHOP_ROW),
                Vec2::new(list.width(), SHOP_ROW - theme::ROW_GAP / 2.0),
            );
            self.good_row(ui, row, good, frame, tools);
        }
        let totals = Rect::from_min_size(
            Pos2::new(body.left(), list.bottom()),
            Vec2::new(body.width(), FOOT_ROW),
        );
        ui.painter().text(
            totals.right_center(),
            Align2::RIGHT_CENTER,
            format!("{WORDS_TOTAL}  {} {WORDS_GP}", self.cart.total(&shop.goods)),
            number_font(theme::SIZE_BODY),
            theme::TEXT,
        );
        if shop.buying {
            ui.painter().text(
                totals.left_center(),
                Align2::LEFT_CENTER,
                format!("{WORDS_GOLD}  {}", with_thousands(frame.gold)),
                number_font(theme::SIZE_BODY),
                theme::WAITING,
            );
        }
        if live {
            let foot = Pos2::new(body.left(), totals.bottom() + theme::ROW_GAP);
            let (deal, dealt) = theme::button(ui, foot, deal_words, theme::GOAL);
            let (clear, cleared) = theme::button(
                ui,
                Pos2::new(deal.right() + theme::ROW_GAP, foot.y),
                WORDS_CLEAR,
                theme::TEXT,
            );
            let (_, closed) = theme::button(
                ui,
                Pos2::new(clear.right() + theme::ROW_GAP, foot.y),
                WORDS_CLOSE,
                theme::TEXT_DIM,
            );
            let rows = self.cart.rows(&shop.goods);
            if dealt && !rows.is_empty() {
                tools.hand.act(Act::Checkout(rows));
            } else if dealt || closed {
                tools.hand.act(Act::ShopClose);
            } else if cleared {
                self.cart.clear();
            }
        }
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) && live {
            tools.hand.act(Act::ShopClose);
        }
        panel
    }

    /// One good of the list: its picture, name, stock, price, and the
    /// count taken with the buttons that change it.
    fn good_row(
        &mut self,
        ui: &egui::Ui,
        row: Rect,
        good: &WatchGood,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
    ) {
        let live = frame.human_control;
        let serial = good.item.serial;
        let response = ui.interact(row, Id::new(("shop-good", serial)), Sense::click());
        if live && response.hovered() {
            ui.painter()
                .rect_filled(row, CornerRadius::same(CELL_RADIUS), theme::BUTTON);
        }
        let art = Rect::from_center_size(
            Pos2::new(row.left() + ART_SIDE / 2.0, row.center().y),
            Vec2::splat(ART_SIDE),
        );
        item_art(ui, art, &good.item, frame, tools, false);
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
            format!("x{}", self.cart.left(good)),
            number_font(theme::SIZE_SMALL),
            theme::TEXT_FAINT,
        );
        let count = self.cart.count(serial);
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
            format!("{} {WORDS_GP}", good.price),
            number_font(theme::SIZE_BODY),
            theme::WAITING,
        );
        item_tip(
            ui,
            &response,
            &good.item,
            tools,
            if live { HINT_GOOD } else { "" },
        );
        if !live {
            return;
        }
        let all = ui.input(|i| i.modifiers.shift);
        if single_or_double(
            &response,
            &mut self.clicks,
            frame,
            tools.hand,
            serial,
            tools.time,
        ) && self.cart.left(good) > 0
        {
            self.cart.take(good, all);
        }
        for (area, words, up) in [(minus, "-", false), (plus, "+", true)] {
            let key = Id::new(("shop-step", serial, up));
            if theme::segment_keyed(ui, area, key, words, theme::TEXT) {
                let next = stepped(count, up, all, good.item.amount);
                self.cart.set(serial, next);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn trade(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        index: usize,
        trade: &WatchTrade,
        frame: &WatchFrame,
        tools: &mut Tools<'_>,
        profile: &mut Profile,
    ) -> Rect {
        let live = frame.human_control;
        let side_width = TRADE_COLUMNS as f32 * (CELL + CELL_GAP) - CELL_GAP;
        let side_height = TRADE_ROWS as f32 * (CELL + CELL_GAP) - CELL_GAP;
        let size = Vec2::new(
            side_width * 2.0 + TRADE_GAP + theme::PANEL_PAD * 2.0,
            frame::TITLE_ROW
                + SIDE_HEAD
                + side_height
                + COIN_ROWS as f32 * COIN_ROW
                + FOOT_ROW
                + theme::PANEL_PAD * 2.0,
        );
        let id = format!("{TRADE_PLACE_ID}{}", index + 1);
        let title = format!("{WORDS_TRADE_WITH} {}", trade.with);
        let spec = PanelSpec {
            id: &id,
            title: &title,
            default: layout::first_place(rect, Spot::Middle(index), size),
            min_size: None,
            closable: live,
        };
        let panel = frame::place(rect, &spec, profile);
        let body = frame::draw(ui.painter(), panel, &title);
        // An item dropped anywhere on the trade goes on the side of the human.
        tools.desk.zone(panel, Zone::Into(trade.mine));
        let state = self.trades.entry(trade.mine).or_default();
        let sides = [
            (WORDS_YOU, trade.i_accept, &trade.mine_items, true),
            (
                trade.with.as_str(),
                trade.they_accept,
                &trade.their_items,
                false,
            ),
        ];
        for (at, (who, accepted, items, mine)) in sides.into_iter().enumerate() {
            let left = body.left() + at as f32 * (side_width + TRADE_GAP);
            let (mark, color) = if accepted {
                (WORDS_ACCEPTED, theme::HITS_POISONED)
            } else {
                (WORDS_THINKS, theme::TEXT_FAINT)
            };
            ui.painter().text(
                Pos2::new(left, body.top()),
                Align2::LEFT_TOP,
                format!("{who}: {mark}"),
                text_font(theme::SIZE_BODY),
                color,
            );
            let cells = Rect::from_min_size(
                Pos2::new(left, body.top() + SIDE_HEAD),
                Vec2::new(side_width, side_height),
            );
            ui.painter()
                .rect_filled(cells, CornerRadius::same(CELL_RADIUS), theme::TRACK);
            let all_rows = items.len().div_ceil(TRADE_COLUMNS);
            let first = &mut state.first_rows[at];
            *first = scrolled(ui, cells, *first, all_rows.saturating_sub(TRADE_ROWS));
            let shown = items
                .iter()
                .skip(*first * TRADE_COLUMNS)
                .take(TRADE_COLUMNS * TRADE_ROWS);
            for (place, item) in shown.enumerate() {
                let cell = Rect::from_min_size(
                    cells.left_top()
                        + Vec2::new(
                            (place % TRADE_COLUMNS) as f32 * (CELL + CELL_GAP),
                            (place / TRADE_COLUMNS) as f32 * (CELL + CELL_GAP),
                        ),
                    Vec2::splat(CELL),
                );
                traded_cell(ui, cell, item, mine, frame, tools, &mut self.clicks);
            }
            let coins_top = cells.bottom() + theme::ROW_GAP;
            if mine {
                offer_fields(ui, Pos2::new(left, coins_top), trade, state, tools, live);
            } else {
                their_offer(ui, Pos2::new(left, coins_top), trade);
            }
        }
        if live {
            let foot = Pos2::new(body.left(), body.bottom() - FOOT_ROW + theme::ROW_GAP);
            let (words, color) = if trade.i_accept {
                (WORDS_UNDO_ACCEPT, theme::WAITING)
            } else {
                (WORDS_ACCEPT, theme::GOAL)
            };
            let (accept, accepted) = theme::button(ui, foot, words, color);
            let (_, canceled) = theme::button(
                ui,
                Pos2::new(accept.right() + theme::ROW_GAP, foot.y),
                WORDS_CANCEL,
                theme::ALARM,
            );
            if accepted {
                tools.hand.act(Act::TradeAccept {
                    trade: trade.mine,
                    accept: !trade.i_accept,
                });
            } else if canceled {
                tools.hand.act(Act::TradeCancel(trade.mine));
            }
        }
        if frame::controls(ui, panel, &spec, profile, tools) == Some(FrameEvent::Closed) && live {
            tools.hand.act(Act::TradeCancel(trade.mine));
        }
        panel
    }
}

/// One item of a trade. The character drags his own items back out.
fn traded_cell(
    ui: &egui::Ui,
    cell: Rect,
    item: &WatchPackItem,
    mine: bool,
    frame: &WatchFrame,
    tools: &mut Tools<'_>,
    clicks: &mut ClickDelay,
) {
    let response = ui.interact(
        cell,
        Id::new(("trade-item", item.serial)),
        Sense::click_and_drag(),
    );
    item_art(ui, cell, item, frame, tools, true);
    let live = frame.human_control;
    let footer = match (live, mine) {
        (false, _) => "",
        (true, true) => HINT_TRADED,
        (true, false) => HINT_THEIRS,
    };
    item_tip(ui, &response, item, tools, footer);
    if !live {
        return;
    }
    if mine && response.drag_started_by(egui::PointerButton::Primary) {
        tools.desk.pick_up(item);
    } else if single_or_double(
        &response,
        clicks,
        frame,
        tools.hand,
        item.serial,
        tools.time,
    ) {
        tools.hand.act(Act::Use(item.serial));
    }
}

/// The gold and platinum fields of the character's offer, each with what
/// he has. A change is held to what he has and sent at once.
fn offer_fields(
    ui: &mut egui::Ui,
    left_top: Pos2,
    trade: &WatchTrade,
    state: &mut TradeState,
    tools: &Tools<'_>,
    live: bool,
) {
    let have = (trade.my_gold, trade.my_platinum);
    let mut offered = state.offered;
    let fields = [(WORDS_GOLD, false, have.0), (WORDS_PLATINUM, true, have.1)];
    for (at, (words, platinum, owned)) in fields.into_iter().enumerate() {
        let row = Rect::from_min_size(
            left_top + Vec2::new(0.0, at as f32 * COIN_ROW),
            Vec2::new(
                COIN_LABEL_WIDTH + COIN_FIELD_WIDTH,
                COIN_ROW - theme::ROW_GAP,
            ),
        );
        ui.painter().text(
            row.left_center(),
            Align2::LEFT_CENTER,
            words,
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        let field = Rect::from_min_max(
            Pos2::new(row.left() + COIN_LABEL_WIDTH, row.top()),
            row.right_bottom(),
        );
        ui.painter()
            .rect_filled(field, CornerRadius::same(CELL_RADIUS), theme::TRACK);
        let typed = if platinum {
            &mut state.platinum
        } else {
            &mut state.gold
        };
        let edit = ui.add_enabled_ui(live, |ui| {
            ui.put(
                field,
                egui::TextEdit::singleline(typed)
                    .id(Id::new(("trade-offer", trade.mine, platinum)))
                    .frame(false)
                    .margin(egui::Margin::symmetric(8, 4))
                    .font(number_font(theme::SIZE_BODY))
                    .text_color(theme::TEXT),
            )
        });
        if edit.inner.changed() {
            if let Some(words) = typed_offer(&mut offered, platinum, typed, have) {
                *typed = words;
            }
        }
        ui.painter().text(
            Pos2::new(field.right() + theme::ROW_GAP, row.center().y),
            Align2::LEFT_CENTER,
            format!("{WORDS_OF} {}", with_thousands(owned)),
            number_font(theme::SIZE_SMALL),
            theme::TEXT_FAINT,
        );
    }
    if offered != state.offered {
        state.offered = offered;
        tools.hand.act(Act::TradeGold {
            trade: trade.mine,
            gold: offered.0,
            platinum: offered.1,
        });
    }
}

/// The gold and platinum the other player offers.
fn their_offer(ui: &egui::Ui, left_top: Pos2, trade: &WatchTrade) {
    for (at, (words, amount)) in [
        (WORDS_GOLD, trade.their_gold),
        (WORDS_PLATINUM, trade.their_platinum),
    ]
    .into_iter()
    .enumerate()
    {
        let y = left_top.y + at as f32 * COIN_ROW + (COIN_ROW - theme::ROW_GAP) / 2.0;
        ui.painter().text(
            Pos2::new(left_top.x, y),
            Align2::LEFT_CENTER,
            words,
            text_font(theme::SIZE_BODY),
            theme::TEXT_DIM,
        );
        ui.painter().text(
            Pos2::new(left_top.x + COIN_LABEL_WIDTH, y),
            Align2::LEFT_CENTER,
            with_thousands(amount),
            number_font(theme::SIZE_BODY),
            theme::WAITING,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shop_is_as_tall_as_its_rows_and_its_foot() {
        assert_eq!(
            shop_height(SHOP_ROWS) - shop_height(SHOP_LEAST_ROWS),
            (SHOP_ROWS - SHOP_LEAST_ROWS) as f32 * SHOP_ROW
        );
        assert!(shop_height(1) > FOOT_ROW * 2.0);
    }

    #[test]
    fn an_offer_starts_at_nothing() {
        let state = TradeState::default();
        assert_eq!(
            (state.gold.as_str(), state.offered),
            (NOTHING_OFFERED, (0, 0))
        );
    }
}
