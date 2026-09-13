# Agent API

Tools return immediately with `action_id`. Completion is an event (`arrived`, `target_requested`, `speech`, and others).

## Banks

`observe` shows `nearest_bank`: the town, the `location` where its banker stands, and the `dist`, for the nearest bank on this map within 400 tiles. The banks are those of the standard towns on every map; a shard with its own towns has others, which you find by their bankers (`find_mobiles` with `name` `banker`). The `bank` goal walks to `nearest_bank` and ends there.

## Gumps

A gump is a window the shard opens: a moongate, a bank question, a vendor menu. `observe` `gumps` and the `next_event` state show each open gump in words, with the text numbers read from the client files:

- `texts`: the words that are not a label.
- `buttons`: each `id` with the `label` beside it. A button with `to_page` only shows another page.
- `choices`: each round or square button with its `switch`, its `label`, and the `section`, the label of the page it is on.

To take the moongate to Moonglow on Trammel, tick the choice with that label and section and press OKAY: `gump_respond` with `button` 1 and `switches` [0].

## Trades

When another player opens a secure trade, you get a `trade_opened` event, and `observe` shows `trade`: the player, what `theirs` and `mine` hold, and `i_accept` and `they_accept`. Read what they offer before you agree. `trade_accept` ticks your accept box (`accept: false` unticks it); `trade_cancel` closes the trade. A change to either side clears both accept boxes, so accept again after it.

## The agent loop

An agent that drives a character must not miss what happens between its calls. Run one loop:

1. Call `next_event`. It returns the moment something important happens, or after `timeout_ms` (default 5000, max 7000) with no events.
2. Read `events` and `state`. Act on them in this order:
   1. Danger: `died`, `low_health`, `damaged`, `enemy_near`, `combatant_changed`, `pk_flag`.
   2. Something waits for an answer: `target_requested`, `gump_opened`, `prompt_opened`, `trade_opened`, `party_invite`.
   3. Chat: `spoken_to`, and `state.unanswered`. Answer with `reply`.
   4. Your own task: `item_added`, `arrived`, `path_failed`, `lift_rejected`, `play_along_ended`, `map_changed` (a moongate or recall took her to another map; the old map's mobiles and items are gone, and a walk or follow there is dropped).
3. Go back to 1.

Events wait in the session for you. When you are slow, the next call gives you all of them, in order, 50 at most per call. `missed` counts events that were dropped before you asked; the session keeps the last 256.

`state` holds `hits`, `mana` and `stam` with their maximums, `war`, `dead`, `location`, `combatant`, `enemies_near` (the 5 nearest mobiles you may fight, within 10 tiles, not party members or friends), `unanswered`, `chat_mode`, `target_cursor`, `pack` (`items`, `weight`, `weight_max`), and `doing`: the `goal`, the tile she is `walking_to`, whom she is `following` or `playing_along_with`, the corpse she is `looting`, whether she is `banking`, and the `script` running. Health under half is `low_health`. A loot or bank job that gives up sends `job_failed` with the reason.

Read `doing` before you answer a player. Her words must match what she does: when she already follows the player, say "right behind you", not "I will stay here"; when she is already at the cows, do not say "lead the way". Never say she will do something unless you start it in the same step.

## Spoken to by name

When another character says your character's name, the session sends a `spoken_to` event. Each tool result then carries these lines in `unanswered`, whatever the tool, so an agent that is busy still sees them. A line leaves the list when it is answered (see `reply` below) or after three minutes. `observe` lists the lines of the last three minutes in `spoken_to`, answered or not. Each line has the speaker's `serial` and `name`, the `text`, the `channel` it came in (`say`, `whisper`, `yell`, `party`, `party_private`, `guild` or `alliance`), and `asks_if_bot`. Party, guild and alliance lines count even when the speaker is out of sight. To start a line yourself, use `say` with `channel`: `say` (default), `party`, `guild` or `alliance`. Answer with `reply`: it sends your text back in the channel the line came in (a yell is answered in a normal voice, a private party line only to that member). Give `to`, a name or serial, to pick the speaker when more than one waits; without it, the newest line is answered. A reply answers that speaker's lines only. A `say`, `whisper` or `emote` answers every waiting line said nearby, and a party, guild or alliance line answers the lines of that channel. A result with `unanswered` lines also has `chat_mode`, which `observe` shows too: `basic` or `play_along`. The name counts only as a whole word: "Tamara" does not name "Mara". System lines, spell words and your own lines never count. The `answer_when_named` setting turns this off.

Answer with `reply` the way a player would. What you may agree to depends on `chat_mode`:

- `basic` (the `play_along` setting is off): answer in a few friendly words and say no to every plan: hunting, following, a party, a trade, "come here". For example "not right now, busy" or "maybe later". Keep doing your own task. The client refuses `follow` and `partyaccept` for a player who asked in chat, unless that player is on the friends list.
- `play_along`: you may say yes to the plans the persona lists (see `docs/PERSONAS.md`): join the player's party (the client accepts the invite of a player who spoke to you), `follow` them, and `attack` what they fight when `fight` is listed. The client refuses a plan that is not listed. `observe` shows `playing_along` (the player and the minutes left). When the persona's time is up, or the character is hurt past its play-along risk, the character stops following and you get a `play_along_ended` event: say a short goodbye and go back to your own task. Talk the way `reply_style` says, when it is given.

In both modes:

- Keep it short and plain. One line is enough; no long talk.
- Stay in character as a player. Never bring up AI, bots, programs or "a game" on your own.
- When `asks_if_bot` is true, laugh it off ("lol, you're funny"), dodge ("too busy chopping to chat"), or just keep working. Never flatly say "I am a real person" or "I am not a bot".
- Never say the same line twice. Change the words each time. The session refuses a line said a short time before.

Start `uoterm connect` or `uoterm populate` first. Then drive the session with CLI, HTTP, or `uoterm mcp`.

## Perception

| Tool | Precondition | Result |
| --- | --- | --- |
| `observe` | session exists | self, radar, journal, mobiles, items, target, gumps, doors, buffs, party, prompt, forbidden (assistant features the shard forbids) |
| `find_mobiles` | in world | filter name / graphic / distance; `name` also matches the title, so `banker` finds "Kate the banker"; each has its title |
| `find_items` | in world | filter graphic / container / name |
| `journal_search` | session exists | matching lines |
| `map_tile` / `can_walk` | map or mock grid | walkable, z, door |

## Action

| Tool | Precondition |
| --- | --- |
| `say` / `whisper` | in world; persona rejects `*emotes*` and empty text; `say` is rate-limited |
| `emote` | `persona.allow_emote` |
| `move_to` | in world; args `x` and `y` are required |
| `walk` | in world. `direction` (`n`/`ne`/`e`/`se`/`s`/`sw`/`w`/`nw`), `running`, `hold_ms` (0 = one step) |
| `open_door` | in world; stand next to the door and face it (`0x12`/`0x58`) |
| `follow` / `stop` | `follow` needs a mobile serial |
| `use` / `single_click` / `attack` / `war_mode` | serial / in world |
| `lift` / `drop` / `equip` / `unequip` | item serial. `drop` takes `dest`: a container, or a mobile to give to; none drops at your feet. `unequip` needs a valid `layer`. Empty layer returns `layer empty`. It does not unequip the backpack |
| `cast` / `use_skill` | in world; `spell` or `skill` number is required |
| `wait_target` | none |
| `target` | a target cursor must be pending |
| `open_container` / `loot` / `trade_offer` | serial |
| `gump_respond` / `gump_close` | open gump. `button` is a button id, `switches` the choices to tick; button `0` closes |
| `set_goal` | in world; `idle` `travel` `hunt` `gather` `bank` `shop` `social` `flee` `ress` |
| `set_persona` | session exists; JSON persona body. `typo_rate` is clamped to `0.0..=1.0` |
| `cancel_goal` | session exists |

## Scripts, agents, hotkeys and macros

See [SCRIPTS.md](SCRIPTS.md) for the script language and
[AGENTS.md](AGENTS.md) for agents, hotkeys and recording.

| Tool | Precondition | Result |
| --- | --- | --- |
| `run_script` | no script running | `name` or `text`; `loop` runs it again each time it ends |
| `stop_script` / `script_status` / `list_scripts` | session exists | stop; status, line, error, output; saved names |
| `hotkeys` / `hotkey` | session exists / in world | list by `group`; press by `name` |
| `agents` / `agent_set` / `agent_on` | session exists | settings; replace `settings` of an `agent` (or its `list`); switch `on` |
| `agent_run` / `agent_stop` | in world | run a job once: organizer, restock, dress, undress, autoloot |
| `damage_meter` / `target_filter` | session exists / in world | `action` start, pause, resume, stop, report; pick with filter `name` |
| `record_macro` | session exists | `action` start (with `name`), stop (saves), cancel |

## HTTP

Default bind: `http://127.0.0.1:7733`.

| Route | Notes |
| --- | --- |
| `GET /health` | No token |
| `GET /v1/sessions` | Session ids |
| `GET /v1/sessions/{id}/state` | Observe JSON |
| `POST /v1/sessions/{id}/tools/{name}` | Tool body is JSON args |
| `POST /v1/sessions` | Create a session. Password is in the body |

When `UOTERM_API_TOKEN` is set, every route except `/health` requires `Authorization: Bearer <token>`. A non-loopback `--api-bind` is refused unless that variable is set. CLI and MCP send the same header when the variable is set.

## MCP

```
uoterm mcp
```

JSON-RPC 2.0 on stdio (`protocolVersion` `2024-11-05`). Newline JSON and `Content-Length` framing. A blank line is skipped. Bad JSON returns `-32700`. Bodies larger than 1 MiB are rejected.

Methods: `initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`. Resource URI: `uo://session/{id}/state`.

## CLI against a running process

```
uoterm --json session list
uoterm session attach s1
uoterm say "vendor buy"
uoterm move --to 1425,1680,0
uoterm walk --dir south --run --hold-ms 2000
uoterm open-door
uoterm look
uoterm state --json
uoterm agent run --persona personas/lumberjack.toml
uoterm agent stop
uoterm harvest log --since 1h --jsonl
```

`--api`, `--session`, `UOTERM_API`, `UOTERM_SESSION`, and `UOTERM_API_TOKEN` apply to these commands.

Exit codes: 0 ok, 2 usage, 3 network, 4 protocol, 5 world/precondition.
