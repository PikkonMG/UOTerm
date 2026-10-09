# Walk playbook

Use the walk job to travel on ground where hostiles may be. Plain `move_to`
is for towns, and for a walk you make while you already fight.

## Start

```json
{ "job": "walk", "x": 1425, "y": 1695, "watch": false }
```

Or a named place from the marker file (`markers` in `uoterm.toml`, or
`--markers`):

```json
{ "job": "walk", "name": "britain bank" }
```

The walk arrives when the character is within 1 tile of the spot.
`watch: true` stays at the spot and still hands back when a hostile comes
near.

Do not call `move_to` while a walk runs.

A second `job_start` is refused unless `replace` is true. With `replace`,
the old job ends first (`job_ended` reason `stopped`), then the new one
starts.

## Reasons on `job_ended`

| Reason | What you do |
| --- | --- |
| `arrived` | You are there. Start a hunt, bank, or the next leg. |
| `hostile` | A mobile you may fight came within 15 tiles, and the character stepped away. A run away that finds no path still says `hostile`. Scan, fight, or pick another way. |
| `unreachable` | No path to the spot, and no hostile. Try a gate, a pad, or a nearer spot. |
| `dead` | Come back from death. |
| `stopped` | You stopped it, or `replace` took it over. |

## Driver

1. `job_start` walk.
2. `next_event` until `job_ended`.
3. Act on the reason. After `hostile`, do not start a walk again on the
   same line.
