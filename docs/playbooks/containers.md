# Containers playbook

1. `use` the container (corpse, chest, pack, bank box).
2. Read `observe.containers`. `total` is how many it really holds. The list is cut.
3. `lift` an item, then `drop` into the pack or onto the ground.
4. A nested bag: `use` it in place, then lift from that serial. Do not assume a bag in a corpse comes out as one item.
5. If a lift is refused (`lift_rejected`), the item is gone, too heavy, or out of reach.

The bank box is the worn container on the bank layer after you say `bank`.
