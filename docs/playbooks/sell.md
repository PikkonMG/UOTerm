# Sell playbook

Selling is not buying. The sell list holds items from your pack, and it
often stays open after a sale.

1. Stand next to the vendor.
2. `vendor_sell` with `vendor_name` (the vendor's name) and `graphic`. It
   says `<Name> sell` for you, and sells every item of that graphic the list
   offers.
3. To pick the rows yourself, say `<Name> sell`, wait for `shop_opened`,
   read `observe` `shop` (`buying` is false), and send `shop_checkout` with
   `items` `[{serial, amount}]`.
4. Check your gold. The list may still be open, so you can sell again
   without opening it again.
5. Do not use a serial you remember from a buy list. Those are the vendor's
   goods, not your items.
