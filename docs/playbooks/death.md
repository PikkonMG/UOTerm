# Death playbook

A hunt or walk job that ends with `dead` means the character is a ghost.
Do not start hunt or walk on a ghost.

## Recover

1. Read `observe`: `dead` is true.
2. `set_goal` `ress` walks toward the nearest known bank. If none is known,
   find a healer with `find_mobiles`.
3. Use the healer or the shrine. Answer the gump if one opens.
4. Wait until `dead` is false and a `resurrected` event arrives.
5. Restock bandages and reagents at the bank if the pack is empty.
6. Only then start hunt or walk again.

## Do not

- Attack, loot, or start hunt while dead.
- Retrace the line that killed the character without a scan.
- Ignore poison as the cause. If the last fight was poison, change ground.
