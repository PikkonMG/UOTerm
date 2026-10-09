# Death playbook

A hunt or walk job that ends with `dead` means the character is a ghost.
Do not start a hunt or a walk on a ghost.

## Recover

1. Read `observe`: `self_state.dead` is true.
2. `set_goal` `ress`. The ghost walks to a healer in view, or to the nearest
   healer the marker file names, or else to the nearest bank, where towns
   keep one. A healer offers when the ghost steps within 2 tiles. The goal
   takes the offer. When no offer comes, it steps back and in again.
3. Wait for `job_ended` `ress: alive` and a `resurrected` event. A
   `job_failed` `ress` says no healer is known. Find one with `find_mobiles`
   (`name` `healer`), or walk to a shrine.
4. Restock bandages and reagents at the bank if the pack is empty.
5. Only then start a hunt or a walk again.

## Do not

- Attack, loot, or start a hunt while dead.
- Walk back the same way that killed the character without a scan first.
- Forget poison as the cause. If the last fight was poison, change ground.
