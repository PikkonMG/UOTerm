# Navigation playbook

| Need | Tool |
| --- | --- |
| Town, no hostiles | `move_to` |
| Wilds, hostiles possible | `job_start` walk. See `walk`. |
| Named place | `move_to` with `name`, or `find_landmarks` (`kind`, `closest`) then walk |
| Near a spot, not on it | `move_to` with `accuracy` (a tree, an anvil, a banker) |
| Keep away from something | `move_to` with `avoid` (areas and creatures) |
| Water, trees, a forge | `find_tiles` with `group` |
| Other city or facet | `moongate` |
| Dungeon hole or pad | `dungeon` |
| Instant return | `runebook` |

`move_to` may return `partial: true`. That is success-shaped: you got closer. Then walk the rest or use a pad. With `exact: true` it fails instead. `route` plans the same walk with no step and says how many nodes the search opened.

Doors: stand next to the door and `open_door`. Do not path around a closed door leaf.

A second `move_to` cancels ordinary travel that is already running. Do not send `move_to` while hunt or walk is the session job.
