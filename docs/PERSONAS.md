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

[play_along]
plans = ["party", "follow", "fight"]
stay_minutes = 30
risk_tolerance = 0.5
reply_style = "friendly, short, jokes a bit"
```

`typo_rate` and both `risk_tolerance` values are clamped to `0.0..=1.0` on load.

## Playing along

The `[play_along]` part is used only when `answer_when_named` and `play_along` are both `true` in `uoterm.toml`. It shapes how the character plays along with a player who spoke to it. A persona without it gets the defaults below.

| Field | Default | Effect |
| --- | --- | --- |
| `plans` | `["party", "follow"]` | What the character may say yes to: `party`, `follow`, `fight`. The client refuses `follow` and a party invite that are not listed. `fight` tells the agent it may `attack` what the player fights. |
| `stay_minutes` | `30` | How long the character follows the player. Then it stops, and the agent gets a `play_along_ended` event. |
| `risk_tolerance` | the persona's own | The risk tolerance while it plays along. When health drops under the flee ratio this gives, the character stops following and the agent gets a `play_along_ended` event. |
| `reply_style` | empty | A few words on how the character talks. The agent sees them as `reply_style` in `observe` and on results with `unanswered` lines. |

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
| `play_along` | See [Playing along](#playing-along) |

Every field above changes behaviour. A persona file carries no other field.

## Goals

| Goal | Reflex |
| --- | --- |
| `idle` | No action |
| `gather` | Use hatchet, then target a tree |
| `hunt` | War mode, attack grey+, bandage when HP is low |
| `travel` | Pathfind to dest. With no dest, the nearest bank |
| `flee` | Step away |
| `bank` | Walk to the nearest bank within 400 tiles, from the banks of the standard towns on every map; the goal ends there. With none that near, or on a shard with its own towns, `set_goal` refuses and the agent must find a banker |
| `shop` | Use a nearby innocent mobile, else walk to the bank when it is near |
| `social` | Say `yo` |
| `ress` | If dead, walk toward the bank when it is near |

Speech rules in code: reject empty text; reject `*…*` unless `allow_emote`; shorten lines over 18 words; apply `typo_rate`.

## Shipped files

| File | Name | Class | Goal from `agent run` |
| --- | --- | --- | --- |
| `personas/lumberjack.toml` | Mara of Yew | lumberjack | `gather` |
| `personas/aldreth.toml` | Aldreth | banker_idle | `social` |
| `personas/cedric.toml` | Cedric | traveler | `travel` |
| `personas/traveler.toml` | Ryn of Minoc | traveler | `travel` |

Use `uoterm agent run --persona personas/lumberjack.toml` against a running `uoterm connect`. That call sends `set_persona`, then `set_goal`.
