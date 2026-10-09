# Loot playbook

A hunt already empties the corpses it made. Use this for a corpse you did
not just kill, or when no hunt runs.

1. Do not loot while a hostile stands next to you. Finish it or run first.
2. `loot` with the corpse serial. The client walks up, opens the corpse, and
   lifts each item in it, a bag as one item. It ends with `job_ended`
   `loot: done`.
3. A `job_failed` says why: the corpse is gone, the shard refused every
   lift, or the job took too long (60 seconds). When the pack is too heavy,
   bank or drop junk, then try once more.

The scavenger agent picks up items from the ground by its list. The
autoloot agent loots corpses by its list. A hunt still loots only the
corpses it made.
