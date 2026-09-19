# Bank playbook

Put loot away, then leave. The client already walks to a known town bank
with `set_goal` `bank`. This playbook is the judgment around that.

## Go to the bank

1. Read `observe.nearest_bank`. If it is there and close enough, `set_goal`
   `bank`.
2. If none is known, `find_mobiles` with name `banker`, then `job_start`
   walk to that tile. Town streets: `move_to` is enough.
3. Wait for `arrived` (bank goal) or `job_ended` `walk: arrived`.
4. Say `bank` beside the banker so the bank box opens.

## Deposit

`deposit` moves the pack into the bank box. Give a graphic to bank only
that kind.

If `job_failed` on deposit, the box was not open or a lift was refused.
Open the box again, then deposit once more.

## After

`set_goal` `idle` or walk back to hunting ground. Do not start hunt on
the bank tiles.
