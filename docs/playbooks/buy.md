# Buy playbook

1. Find the vendor (`find_mobiles` with `name`, such as `baker`).
2. Stand next to them. Say `<Name> buy` (the speech keyword form). Plain words without the keyword packet do not open the shop on many shards. `say` in UOTerm already encodes keywords.
3. Wait for `shop_opened`. `observe` `shop` then lists the goods: each `serial`, `graphic`, `name`, `amount` and `price`.
4. `vendor_buy` with `vendor` (the vendor's serial), `item` (the `serial` of a row of `observe` `shop`) and `amount`. `shop_checkout` buys several rows at once: `items` `[{serial, amount}]`.
5. A buy often closes the list. Open it again before a second buy. Serials on the list are new each time; take them from the list that is open, not from memory.
6. Confirm gold and pack.

`set_goal` `shop` walks to a nearby innocent or the bank when you have no vendor in mind.
