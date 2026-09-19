# Moongate playbook

A gate fires when the character **steps onto its tile**. Do not `use` a gate while already standing on it.

1. `find_landmarks` with name `moongate`, or `find_items` graphic `0x0F6C` (blue) or `0x0DDA` (red).
2. Walk to a tile next to the gate, then step onto it. If you already stand on it and no gump opened, step off and back on.
3. Read the gump. Button numbers change per shard. Pick the destination by its text, then `gump_respond`.
4. Wait for `map_changed` or a new location. Old mobiles and items are gone.
5. If you change your mind, close the gump (button 0 on most shards).

Too far: the journal says the gate is too far. Walk one tile closer.

Bank before a long trip. Getting back is the same trip from the gate at the far end.
