# Navigation playbook

| Need | Tool |
| --- | --- |
| A town, no hostiles | `move_to` |
| The wilds, where hostiles may be | `job_start` walk. See `walk`. |
| A named place | `move_to` with `name`, or `find_landmarks` (`kind`, `closest`), then walk |
| Near a spot, not on it | `move_to` with `accuracy` (a tree, an anvil, a banker) |
| Keep away from something | `move_to` with `avoid` (areas and creatures) |
| Water, trees, a forge | `find_tiles` with `group` |
| Another city or facet | `moongate` |
| A dungeon hole or pad | `dungeon` |
| A fast way back | `runebook` |

`move_to` may answer `partial: true`. That counts as a success: you got
closer. Then walk the rest, or use a pad. With `exact: true` it fails
instead. `route` plans the same walk with no step, and says how many nodes
the search opened.

Doors: `move_to` opens doors on the way by default. To open one by hand,
stand next to it and call `open_door`. Do not plan a path around a shut
door.

A second `move_to` replaces a walk that already runs. Do not send `move_to`
while a hunt or a walk job runs.
