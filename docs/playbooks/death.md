# Death playbook

A hunt or walk job that ends with `dead` means the character is a ghost.
Do not start hunt or walk on a ghost.

## Recover

1. Read `observe`: `dead` is true.
2. `set_goal` `ress`. The ghost walks to a healer in view, or to the nearest
   healer the marker file names, or else to the nearest bank, where towns
   keep one. A healer offers when the ghost steps within 2 tiles of him; the
   goal takes the offer, and steps back and in again when none comes.
3. Wait for `job_ended` `ress: alive` and a `resurrected` event. A
   `job_failed` `ress` says no healer is known: find one with `find_mobiles`
   (`name` `healer`) or walk to a shrine.
4. Restock bandages and reagents at the bank if the pack is empty.
5. Only then start hunt or walk again.

## Do not

- Attack, loot, or start hunt while dead.
- Retrace the line that killed the character without a scan.
- Ignore poison as the cause. If the last fight was poison, change ground.
