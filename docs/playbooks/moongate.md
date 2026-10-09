# Moongate playbook

A gate fires when the character **steps onto its tile**. Do not `use` a gate
while you already stand on it.

1. `find_landmarks` with name `moongate`, or `find_items` with graphic
   `0x0F6C` (blue) or `0x0DDA` (red).
2. Walk to a tile next to the gate, then step onto it. If you already stand
   on it and no gump opened, step off and back on.
3. Read the gump. The button numbers change from shard to shard. Pick the
   place by its text, then `gump_respond`.
4. Wait for `map_changed` or a new location. The old mobiles and items are
   gone.
5. If you change your mind, close the gump (button 0 on most shards).

If the journal says the gate is too far, walk one tile closer.

Bank before a long trip. The way back is the same trip from the gate at the
far end.
