# Navigation playbook

| Need | Tool |
| --- | --- |
| Town, no hostiles | `move_to` |
| Wilds, hostiles possible | `job_start` walk. See `walk`. |
| Named place | `find_landmarks`, then walk or `move_to` |
| Other city or facet | `moongate` |
| Dungeon hole or pad | `dungeon` |
| Instant return | `runebook` |

`move_to` may return `partial: true`. That is success-shaped: you got closer. Then walk the rest or use a pad.

Doors: stand next to the door and `open_door`. Do not path around a closed door leaf.

A second `move_to` cancels ordinary travel that is already running. Do not send `move_to` while hunt or walk is the session job.
