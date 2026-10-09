# Hunt playbook

The MCP agent starts a hunt job, then waits. It does not drive each swing.
The client stays in melee range, loots the corpses it made, and runs away
when it must.

## Driver loop

1. Call `next_event` in a loop.
2. On danger (`died`, `low_health`), act first.
3. On `job_ended`, read `text`. It is `hunt: <reason>`.
4. Start a hunt again only after you chose new ground, restocked, or
   came back from death.

Do not call `attack` or `move_to` while a hunt runs. Those calls take the
walk away from the job.

## Start

```json
{ "job": "hunt", "include": ["species:zombie"], "replace": false }
```

`job_start` with `job` `hunt`. Give `include` or `avoid`, not both. Each
term must name what it matches on:

| Term | Matches |
| --- | --- |
| `species:zombie` | the species word. UOTerm reads it from the body graphic, and else from the name without a leading "a", "an" or "the" |
| `name:gruuk` | the exact name, in any case |
| `graphic:3` or `graphic:0x3` | the body graphic |
| `any:orc` | part of the name or of the species word |

With empty lists, it fights any gray, criminal, enemy or murderer.

`set_goal` with `hunt` starts the same job with empty lists.

`jobs` shows the job, its phase (`kill`, `loot`, `escape`) and the lists.

`job_stop` ends it. So do `cancel_goal`, `stop`, and `set_goal` with another
goal. The event reason is `stopped`.

A second `job_start` hunt is refused unless `replace` is true. With
`replace`, the old job ends first (`job_ended` `hunt: stopped` or
`walk: stopped`), then the new one starts.

## When it runs away

The job runs away (the `escape` phase) when:

- health is under 45%. It fights again at 80%.
- 4 or more foes stand within 2 tiles and health is 60% or less.
- a blocked creature (one on the `avoid` list, or not on the `include`
  list) is in war mode within 12 tiles, or any blocked creature is within
  2 tiles.

A target it cannot find a path to twice is left alone for 5 minutes.

## Reasons on `job_ended`

| Reason | What you do |
| --- | --- |
| `empty` | It fought, then nothing allowed stayed within 10 tiles. Walk to new ground, then start a hunt again. A hunt does not end before it has seen a target. |
| `unreachable` | A creature it may fight is in range, but there is no path. Find a gate or other ground. |
| `avoided` | It ran from a blocked creature, and is clear: 15 seconds with none near, and health at 80% or more. Do not walk back the same way. Keep 10 tiles from where you last saw it. Scan, then start a hunt again. |
| `dead` | Come back from death. Do not start a hunt on a ghost. |
| `stopped` | You, `cancel_goal`, or `replace` stopped it. |

The job does not end only because the target died. It loots that corpse,
then looks for the next target it may fight.

## While it runs

The session's own care goes on during the job: it bandages under 70%
health, and drinks cure, heal and refresh potions when the shard allows
them. The bandage agent heals too when it is on.

A hunt loots only corpses of mobiles it fought. Other corpses stay for the
autoloot agent.

`observe.doing.job` names the job and the phase. Read it before you speak
for the character.

## After `avoided`

1. Do not stand still. Start a walk to the hunting ground from another side.
2. Keep away from where you last saw the blocked creature.
3. Scan with `find_mobiles` when you get there. The old sighting is stale.
4. Then `job_start` hunt again with the same lists.
