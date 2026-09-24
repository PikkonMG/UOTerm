//! The cart of a shopkeeper's list, apart from how a window draws it: how
//! many of each good the player takes, kept from nothing to the stock, the
//! rows the checkout sends, and the price of them all. The cart empties
//! when another list opens. The gold a player offers in a trade, read from
//! what he typed.

use crate::view::{WatchGood, WatchShop};
use std::collections::HashMap;

/// Shift with a step button moves the count by this many.
pub const BIG_STEP: u16 = 10;

/// The count of one row after a step, kept from zero to the stock.
pub fn stepped(count: u16, up: bool, big: bool, stock: u16) -> u16 {
    let by = if big { BIG_STEP } else { 1 };
    if up {
        count.saturating_add(by).min(stock)
    } else {
        count.saturating_sub(by)
    }
}

/// The gold a player typed, with its thousands marks, when it reads as a
/// number.
pub fn typed_gold(words: &str) -> Option<u32> {
    let digits: String = words.trim().chars().filter(|c| *c != ',').collect();
    if digits.is_empty() {
        Some(0)
    } else {
        digits.parse().ok()
    }
}

/// A thousands mark stands before each three digits from the right.
const THOUSANDS_DIGITS: usize = 3;
/// One platinum coin is worth this much gold.
pub const PLATINUM_IN_GOLD: u64 = 1_000_000_000;

/// A number with its thousands marked, as a trade writes gold: 1,500.
pub fn with_thousands(amount: u32) -> String {
    let digits = amount.to_string();
    let mut marked = String::with_capacity(digits.len() + digits.len() / THOUSANDS_DIGITS);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && (digits.len() - at).is_multiple_of(THOUSANDS_DIGITS) {
            marked.push(',');
        }
        marked.push(digit);
    }
    marked
}

/// What a player may offer in a trade: the gold or the platinum he typed,
/// held to what he has less what the other coin already offers, as the
/// shard checks the two together.
pub fn held_offer(platinum: bool, typed: u32, other_offer: u32, have: (u32, u32)) -> u32 {
    let wealth = u64::from(have.0) + u64::from(have.1) * PLATINUM_IN_GOLD;
    let most = if platinum {
        wealth.saturating_sub(u64::from(other_offer)) / PLATINUM_IN_GOLD
    } else {
        wealth.saturating_sub(u64::from(other_offer) * PLATINUM_IN_GOLD)
    };
    u32::try_from(u64::from(typed).min(most)).unwrap_or(u32::MAX)
}

/// Takes what the player typed in the gold (or the platinum) field of a
/// trade into the offer of both coins, held to what he has. Gives the words
/// the field must show instead, when the typed ones do not stand.
pub fn typed_offer(
    offered: &mut (u32, u32),
    platinum: bool,
    words: &str,
    have: (u32, u32),
) -> Option<String> {
    let other = if platinum { offered.0 } else { offered.1 };
    let wanted = typed_gold(words).unwrap_or(0);
    let held = held_offer(platinum, wanted, other, have);
    if platinum {
        offered.1 = held;
    } else {
        offered.0 = held;
    }
    (held != wanted || words.is_empty()).then(|| with_thousands(held))
}

#[derive(Default)]
pub struct Cart {
    /// The shopkeeper and the side of the list the cart is for.
    of: Option<(u32, bool)>,
    counts: HashMap<u32, u16>,
    /// The goods in the order the player first took them.
    order: Vec<u32>,
}

impl Cart {
    /// Follows the list that is open: another list empties the cart. True
    /// when it did.
    pub fn follow(&mut self, shop: &WatchShop) -> bool {
        let of = Some((shop.vendor, shop.buying));
        if self.of == of {
            return false;
        }
        self.of = of;
        self.clear();
        true
    }

    pub fn clear(&mut self) {
        self.counts.clear();
        self.order.clear();
    }

    pub fn count(&self, serial: u32) -> u16 {
        self.counts.get(&serial).copied().unwrap_or(0)
    }

    /// Takes this many of a good; none takes it out of the cart.
    pub fn set(&mut self, serial: u32, count: u16) {
        if count == 0 {
            self.counts.remove(&serial);
            self.order.retain(|taken| *taken != serial);
            return;
        }
        if self.counts.insert(serial, count).is_none() {
            self.order.push(serial);
        }
    }

    /// Takes `by` more of a good, no more than its stock.
    pub fn add(&mut self, good: &WatchGood, by: u16) {
        let count = self.count(good.item.serial).saturating_add(by);
        self.set(good.item.serial, count.min(good.item.amount));
    }

    /// How many of a good are left in the stock, past the cart.
    pub fn left(&self, good: &WatchGood) -> u16 {
        good.item
            .amount
            .saturating_sub(self.count(good.item.serial))
    }

    /// Takes one more of a good, or all that are left.
    pub fn take(&mut self, good: &WatchGood, all: bool) {
        let by = if all { self.left(good) } else { 1 };
        self.add(good, by);
    }

    /// Puts `by` of a good back.
    pub fn put_back(&mut self, serial: u32, by: u16) {
        self.set(serial, self.count(serial).saturating_sub(by));
    }

    /// The rows of the cart that hold something, in the order of the list.
    pub fn rows(&self, goods: &[WatchGood]) -> Vec<(u32, u16)> {
        goods
            .iter()
            .filter_map(|good| {
                let count = self.counts.get(&good.item.serial).copied()?;
                Some((good.item.serial, count))
            })
            .collect()
    }

    /// The goods taken, in the order the player took them.
    pub fn taken(&self) -> &[u32] {
        &self.order
    }

    pub fn total(&self, goods: &[WatchGood]) -> u64 {
        goods
            .iter()
            .map(|good| u64::from(good.price) * u64::from(self.count(good.item.serial)))
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::WatchPackItem;

    const BANDAGE: u32 = 0x4000_0001;
    const ARROW: u32 = 0x4000_0002;
    const VENDOR: u32 = 0x0000_0101;

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
        let mut cart = Cart::default();
        cart.set(ARROW, 100);
        cart.set(BANDAGE, 0);
        assert_eq!(cart.rows(&goods), vec![(ARROW, 100)]);
        assert_eq!(cart.total(&goods), 300);
        cart.add(&goods[0], 30);
        assert_eq!(cart.count(BANDAGE), 20);
        assert_eq!(cart.taken(), &[ARROW, BANDAGE]);
        cart.put_back(ARROW, 100);
        assert_eq!(cart.taken(), &[BANDAGE]);
    }

    #[test]
    fn another_list_empties_the_cart() {
        let shop = |buying| WatchShop {
            vendor: VENDOR,
            buying,
            ..WatchShop::default()
        };
        let mut cart = Cart::default();
        assert!(cart.follow(&shop(true)));
        cart.set(ARROW, 2);
        assert!(!cart.follow(&shop(true)));
        assert_eq!(cart.count(ARROW), 2);
        assert!(cart.follow(&shop(false)));
        assert_eq!(cart.count(ARROW), 0);
    }

    #[test]
    fn an_offer_is_held_to_the_wealth_and_marks_its_thousands() {
        assert_eq!(with_thousands(1_500), "1,500");
        assert_eq!(with_thousands(999), "999");
        assert_eq!(with_thousands(1_000_000), "1,000,000");
        assert_eq!(held_offer(false, 5_000, 0, (2_000, 0)), 2_000);
        assert_eq!(held_offer(false, 5_000, 1, (2_000, 1)), 2_000);
        assert_eq!(held_offer(true, 3, 0, (0, 2)), 2);
        assert_eq!(held_offer(true, 1, 2_000, (1_000, 1)), 0);
    }

    #[test]
    fn a_typed_offer_is_held_and_rewritten_when_it_does_not_stand() {
        let mut offered = (0, 0);
        assert_eq!(typed_offer(&mut offered, false, "1500", (2_000, 0)), None);
        assert_eq!(offered, (1_500, 0));
        assert_eq!(
            typed_offer(&mut offered, false, "5,000", (2_000, 0)),
            Some("2,000".into())
        );
        assert_eq!(
            typed_offer(&mut offered, true, "", (2_000, 0)),
            Some("0".into())
        );
        assert_eq!(offered, (2_000, 0));
    }

    #[test]
    fn a_take_is_one_or_all_that_is_left() {
        let bandage = good(BANDAGE, 6, 20);
        let mut cart = Cart::default();
        cart.take(&bandage, false);
        assert_eq!((cart.count(BANDAGE), cart.left(&bandage)), (1, 19));
        cart.take(&bandage, true);
        assert_eq!((cart.count(BANDAGE), cart.left(&bandage)), (20, 0));
    }

    #[test]
    fn typed_gold_reads_its_thousands_marks() {
        assert_eq!(typed_gold("1,500"), Some(1500));
        assert_eq!(typed_gold(""), Some(0));
        assert_eq!(typed_gold("a lot"), None);
    }
}
