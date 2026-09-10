# Agent API

Tools return immediately with `action_id`. Completion is an event (`arrived`, `target_requested`, `speech`, and others).

Start `uoterm connect` or `uoterm populate` first. Then drive the session with CLI, HTTP, or `uoterm mcp`.

## Perception

| Tool | Precondition | Result |
| --- | --- | --- |
| `observe` | session exists | self, radar, journal, mobiles, items, target, gumps, doors, buffs, party, prompt, forbidden (assistant features the shard forbids) |
| `find_mobiles` | in world | filter name / graphic / distance |
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
| `gump_respond` / `gump_close` | open gump. button `0` closes |
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
