# Hunt playbook

The MCP agent starts a hunt job, then waits. It does not drive each swing.
The client stays in melee range, loots corpses it made, and flees when it
must. Bandage stays with the bandage agent.

## Driver loop

1. Call `next_event` in a loop.
2. On danger (`died`, `low_health`), act first.
3. On `job_ended`, read `text`. It is `hunt: <reason>`.
4. Start hunt again only after you have chosen new ground, restocked, or
   recovered from death.

Do not call `attack` or `move_to` while hunt is running. Those calls
steal the walk the job owns.

## Start

```json
{ "job": "hunt", "include": ["species:zombie"], "replace": false }
```

`job_start` with `job` `hunt`. Optional `include` or `avoid` (not both).
Each term must name its axis:

| Term | Matches |
| --- | --- |
| `species:zombie` | the name without a leading "a"/"an"/"the" |
| `name:gruuk` | exact name, any case |
| `graphic:3` or `graphic:0x3` | body graphic |
| `any:orc` | substring of the name or of the species word |

Empty lists: fight any grey, criminal, enemy or murderer.

`set_goal` with `hunt` starts the same job with empty lists.

`jobs` shows the job, its phase (`kill`, `loot`, `escape`), and the lists.

`job_stop` ends it. So does `cancel_goal`. The event reason is `stopped`.

A second `job_start hunt` is refused unless `replace` is true. Replace
stops the old job first (`job_ended` `hunt: stopped` or `walk: stopped`),
then starts the new one.

## Reasons on `job_ended`

| Reason | What you do |
| --- | --- |
| `empty` | It fought, then nothing allowed stayed in 10 tiles. Walk to new ground, then start hunt again. Hunt does not end before it has seen a target. |
| `unreachable` | Allowed spawn is in range but no path. Find a gate or other ground. |
| `avoided` | Fled a blocked creature and is clear. Do not retrace that line. Keep 10 tiles from the last sighting. Scan, then start hunt again. |
| `dead` | Recover from death. Do not restart hunt on a ghost. |
| `stopped` | You, `cancel_goal`, or `replace` stopped it. |

The job does not end only because the current target died. It loots that
corpse, then looks for the next allowed target.

## While it runs

The bandage agent heals. Hunt never bandages.

Hunt loots only corpses of mobiles it was fighting. Other corpses stay for
the autoloot agent.

`observe.doing.job` names the job and the phase. Read it before you speak
for the character.

## After `avoided`

1. Do not stand still. Start a walk to the hunting ground from a different
   side.
2. Keep off the last sighting of the blocked creature.
3. Scan `find_mobiles` on arrival. The sighting is old.
4. Then `job_start` hunt again with the same lists.
