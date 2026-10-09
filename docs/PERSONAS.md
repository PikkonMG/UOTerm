# Personas

A persona is a TOML file that says who the character is: its class and
tier, the hours it plays, and how it talks. `uoterm connect --persona` loads
one, and so does `uoterm agent run --persona`. The `set_persona` tool sends
one as JSON. `connect` with no `--persona` uses a built-in lumberjack named
Mara of Yew.

The persona `name` is only the name the speech rules use. The character on
the shard is the one of `--character`, or of the saved login.

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

`name`, `class` and `tier` must be there. UOTerm clamps `typo_rate` and
both `risk_tolerance` values to `0.0..=1.0` when it loads the file. It skips
a key it does not know without a word, so a key with a typo quietly keeps
its default.

## Fields

| Field | Default | Effect |
| --- | --- | --- |
| `class` | | `agent run` and `populate` turn it into a goal. See the next table. |
| `tier` | | The base health share at which the character runs away: novice 0.55, apprentice 0.45, journeyman 0.35, expert 0.28, adept 0.22, master and grandmaster 0.18. Any other word gives 0.35. |
| `risk_tolerance` | `0.35` | Scales that share: the share is the base times `1.2 - risk_tolerance`, kept between 0.7 and 1.3 times the base. |
| `active_hours` | always | `populate` skips the character when the local time is outside every window. A window may pass midnight, such as `"22:00-02:00"`. |
| `chat_rate_per_hour` | `8` | The most plain `say` lines in one clock hour. Two lines are also at least half of `3600 / rate` seconds apart. Replies, yells, whispers and emotes do not count. |
| `typo_rate` | `0.02` | The chance of a one-letter typo in a line. |
| `allow_emote` | `false` | When false, `*emotes*` and the `emote` tool are refused. |
| `play_along` | | See [Playing along](#playing-along). |

The flee share counts only while a goal runs. A hunt, walk, loot, deposit
or follow job has its own rules. The hunt job, for example, runs away under
45% health (see the hunt playbook).

## Class to goal

| Class | Goal |
| --- | --- |
| `lumberjack`, `gatherer` | `gather` |
| `miner` | `mine` |
| `warrior`, `pk` | `hunt` |
| `traveler` | `travel` |
| `sitter`, `banker`, `banker_idle` | `social` |
| any other | `idle` |

`uoterm agent run --goal NAME` picks another goal. [AGENT_API.md](AGENT_API.md)
explains each goal under `set_goal`.

## Speech rules

UOTerm checks each line the character says:

- An empty line is refused.
- A line in stars, such as `*waves*`, is refused unless `allow_emote` is true.
- A line of more than 18 words is cut to its first 8 words.
- A line of more than 128 characters is refused.
- A line the character said lately is refused. UOTerm keeps the last 32.
- `typo_rate` may change one letter (a, s, e, t, n or o) to a key next to it.

## Playing along

The `[play_along]` part counts only when `answer_when_named` and
`play_along` are both `true` in `uoterm.toml`. Their defaults are `true`
and `false`. The part shapes how the character plays along with a player
who spoke to it. A persona without it gets the defaults below.

| Field | Default | Effect |
| --- | --- | --- |
| `plans` | `["party", "follow"]` | What the character may say yes to: `party`, `follow`, `fight`. With `party`, the client joins the party of a player who spoke to it. A `follow` that is not listed is refused, and so is the script command `partyaccept`. `fight` tells the agent it may `attack` what the player fights. The client does not check `fight`. |
| `stay_minutes` | `30` | How long the character follows the player. Then it stops, and the agent gets a `play_along_ended` event. |
| `risk_tolerance` | the persona's own | The risk tolerance while it plays along. When health drops under the flee share this gives, the character stops following, and the agent gets a `play_along_ended` event. |
| `reply_style` | empty | A few words on how the character talks. The agent sees them as `reply_style` in `observe`, and on results with `unanswered` lines. |

A player on the friends list ([AGENTS.md](AGENTS.md)) is never refused.

## Shipped files

| File | Name | Class | Goal from `agent run` |
| --- | --- | --- | --- |
| `personas/lumberjack.toml` | Mara of Yew | lumberjack | `gather` |
| `personas/aldreth.toml` | Aldreth | banker_idle | `social` |
| `personas/cedric.toml` | Cedric | traveler | `travel` |
| `personas/traveler.toml` | Ryn of Minoc | traveler | `travel` |

Run `uoterm agent run --persona personas/lumberjack.toml` against a running
`uoterm connect`. It sends `set_persona`, then `set_goal`. `uoterm agent
stop` sends `cancel_goal`.

Other files you put in `personas/` are ignored by git. The shipped ones stay
tracked.
