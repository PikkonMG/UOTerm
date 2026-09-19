# Walk playbook

Use the walk job for travel on ground that may have hostiles. Ordinary
`move_to` stays for towns and for a walk you make while already fighting.

## Start

```json
{ "job": "walk", "x": 1425, "y": 1695, "watch": false }
```

Or a named place from the marker file:

```json
{ "job": "walk", "name": "britain bank" }
```

`watch: true` stands at the destination and still hands back if a hostile
comes near.

Do not call `move_to` while walk is running.

A second `job_start` is refused unless `replace` is true. Replace stops
the old job first (`job_ended` reason `stopped`), then starts the new one.

## Reasons on `job_ended`

| Reason | What you do |
| --- | --- |
| `arrived` | You are there. Start hunt, bank, or the next leg. |
| `hostile` | A fightable mobile came within 15 tiles. The character stepped off. A flee that cannot path still reports `hostile`. Scan, fight, or pick another line. |
| `unreachable` | No path to the dest (and not a hostile). Try a gate, a pad, or a shorter dest. |
| `dead` | Recover from death. |
| `stopped` | You stopped it, or `replace` took it over. |

## Driver

1. `job_start` walk.
2. `next_event` until `job_ended`.
3. Act on the reason. Do not restart walk on the same line after `hostile`.
