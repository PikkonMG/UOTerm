# Buy playbook

1. Find the vendor (`find_mobiles` with `name`, such as `baker`).
2. Stand next to the vendor. Say `<Name> buy`. Many shards open the shop
   only for this keyword form. `say` in UOTerm already sends the keywords.
3. Wait for `shop_opened`. `observe` `shop` then lists the goods: each
   `serial`, `graphic`, `name`, `amount` and `price`.
4. `vendor_buy` with `vendor` (the vendor's serial), `item` (the `serial`
   of a row of `observe` `shop`) and `amount`. `shop_checkout` buys several
   rows at once: `items` `[{serial, amount}]`.
5. A buy often closes the list. Open it again before a second buy. The
   serials on the list are new each time. Take them from the list that is
   open, not from memory.
6. Check your gold and your pack.

`set_goal` `shop` walks to a nearby innocent, or to the bank, when you have
no vendor in mind.
