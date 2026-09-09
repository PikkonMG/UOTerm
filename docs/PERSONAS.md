# Personas

A persona is a TOML file loaded by `uoterm connect --persona` or by `uoterm agent run --persona`. The shard character name is `--character` (or the profile). The persona `name` is only speech-policy identity.

```toml
name = "Mara of Yew"
class = "lumberjack"
tier = "journeyman"
active_hours = ["14:00-18:00", "21:00-23:30"]
risk_tolerance = 0.35
chat_rate_per_hour = 8
typo_rate = 0.02
allow_emote = false
```

`typo_rate` is clamped to `0.0..=1.0` on load and on `set_persona`.

## Fields the runtime uses

| Field | Effect |
| --- | --- |
| `class` | `agent run` and `populate` map it to a goal: lumberjack/miner/gatherer → `gather`; warrior/pk → `hunt`; traveler → `travel`; sitter/banker/banker_idle → `social`; other → `idle` |
| `tier` | Flee HP ratio (novice 0.55 … grandmaster 0.18) |
| `risk_tolerance` | Scales that flee ratio |
| `active_hours` | `populate` skips the agent when local time is outside the windows |
| `chat_rate_per_hour` | Caps `say` rate |
| `typo_rate` | Chance of a one-character typo on speech |
| `allow_emote` | If false, `*emotes*` and the `emote` tool are rejected |

Every field above changes behaviour. A persona file carries no other field.

## Goals

| Goal | Reflex |
| --- | --- |
| `idle` | No action |
| `gather` | Use hatchet, then target a tree |
| `hunt` | War mode, attack grey+, bandage when HP is low |
| `travel` | Pathfind to dest |
| `flee` | Step away |
| `bank` | Walk toward Britain bank (1425, 1695, 0) |
| `shop` | Use a nearby innocent mobile, else walk to the bank |
| `social` | Say `yo` |
| `ress` | If dead, walk toward the bank |

Speech rules in code: reject empty text; reject `*…*` unless `allow_emote`; shorten lines over 18 words; apply `typo_rate`.

## Shipped files

| File | Name | Class | Goal from `agent run` |
| --- | --- | --- | --- |
| `personas/lumberjack.toml` | Mara of Yew | lumberjack | `gather` |
| `personas/aldreth.toml` | Aldreth | banker_idle | `social` |
| `personas/cedric.toml` | Cedric | traveler | `travel` |
| `personas/traveler.toml` | Ryn of Minoc | traveler | `travel` |

Use `uoterm agent run --persona personas/lumberjack.toml` against a running `uoterm connect`. That call sends `set_persona`, then `set_goal`.
