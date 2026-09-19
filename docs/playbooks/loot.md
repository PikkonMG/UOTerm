# Loot playbook

Hunt already drains corpses it killed. Use this for a corpse you did not just kill, or when hunt is not running.

1. Do not loot while a hostile is adjacent. Finish or flee first.
2. `loot` with the corpse serial. The client walks up, opens, and lifts.
3. If `job_failed`, the corpse decayed, a lift was refused, or the pack is full. Bank or drop junk, then try once more.
4. Bags inside a corpse: `use` the bag, then loot that serial. Do not lift the bag itself if it stays on the corpse.

The autoloot agent can take extra ground rules. Hunt still only takes corpses it made.
