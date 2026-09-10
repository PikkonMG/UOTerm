# UOTerm

UOTerm is a headless Ultima Online client. Its main purpose is to let AI agents control player characters and play the game the way a human player does: see the world, walk, fight, gather, talk, use items, and answer gumps. A second purpose is testing and debugging by humans.

It speaks the Ultima Online wire protocol. It is not a graphical client and not a click-macro overlay.

Ultima Online is a trademark of its owners. UOTerm is independent and unaffiliated. This repository does not ship MUL, UOP, or other client data. You supply a legitimate client directory when you need walkability from map files.

Target shards you operate: your own servers, demo servers, and offline worlds. Official Terms of Service may forbid third-party clients. Do not market UOTerm as an official-server bot. See `LEGAL.md`.

## Shard compatibility

UOTerm matches Classic Client packet layouts and login. It does not special-case
a shard by name, so any server that accepts a Classic Client should work.

Nothing here is verified against a live shard yet. Treat every capability as
unproven until a test in this tree proves it.

The mock shard in this repository is not a real server. `cargo test --workspace` covers mock login (account `0x80` and game `0x91`), walk, speech, and a gather loop without official client files.

## What it is not

- Not a graphical client.
- Not a click-macro overlay.
- Not an official-server farm bot.
- Not a cheat tool for EA or Broadsword shards.

## Requirements

- Rust 1.80 or later (stable). `rust-toolchain.toml` pins `stable`.
- Linux or Windows. macOS is untested.
- A TCP port for the mock or private shard (default `2593`).
- A TCP port for the local HTTP API (default `127.0.0.1:7733`).
- Password in an environment variable. Default name: `UO_PASS`. Do not put passwords in git.

Optional:

- A legitimate UO client directory (`--uopath`) for `map0.mul` / UOP, statics, and `tiledata.mul`.
- `UOTERM_API_TOKEN` when the HTTP API must require a bearer token, or when `--api-bind` is not loopback.

## Build

From the repository root:

```bash
cargo build -p uoterm
./target/debug/uoterm --help
```

The examples below write `uoterm`. On a fresh machine use `./target/debug/uoterm` or `cargo run -p uoterm -- <command>`.

## How the process model works

`uoterm connect` and `uoterm populate` stay in the foreground. They log in, then serve HTTP until you press Ctrl+C.

All other commands (`session`, `say`, `move`, `walk`, `open-door`, `look`, `state`, `agent`, `mcp`) are clients. They call that HTTP API. They do not open a second game socket.

Do not start a second `connect` on the same API port.

Global flags (all commands):

| Flag | Environment | Purpose |
| --- | --- | --- |
| `--json` | | Print JSON on stdout |
| `--api <URL>` | `UOTERM_API` | HTTP base. Default `http://127.0.0.1:7733` |
| `--session <id>` | `UOTERM_SESSION` | Session id. Default: first session on the API |

Exit codes: `0` ok, `2` usage, `3` network, `4` protocol, `5` world or precondition.

## HTTP API token

Set `UOTERM_API_TOKEN` to a non-empty secret. When that variable is set, every HTTP route except `GET /health` requires `Authorization: Bearer <token>`. Client commands read the same variable and send the header.

`--api-bind` that is not loopback (`127.0.0.1`, `localhost`, `::1`) is refused unless `UOTERM_API_TOKEN` is set.

Loopback without a token stays open for local tests. Do not bind the API to a public address without a token.

```bash
export UOTERM_API_TOKEN=replace-me
curl -H "Authorization: Bearer replace-me" http://127.0.0.1:7733/v1/sessions
```

## Configuration

Load order for `uoterm.toml`:

1. `./uoterm.toml` in the current working directory.
2. Linux: `~/.config/uoterm/uoterm.toml`. Windows: `%APPDATA%\uoterm\uoterm.toml`.

Copy `uoterm.toml.example` to `uoterm.toml` if you want a local API bind. `connect` uses this file for `host`, `port`, `era`, `uopath`, `api_bind`, `max_sessions`, and `stay_on_socket` when the matching CLI flag is omitted.

`obey_shard_rules` (default `true`): some shards send a list of assistant features they forbid, such as auto-open doors, auto-bandage, and auto-potions. With `true`, the character does not use those features by itself on that shard. With `false`, it ignores the list. The client answers the shard in both cases. A session made with `POST /v1/sessions` takes the same `obey_shard_rules` field.

`answer_when_named` (default `true`): when another character says your character's name, the agent gets a `spoken_to` event, an `unanswered` list on every tool result until your character speaks, and a `spoken_to` list in `observe`, so it can answer. With `false`, the agent is told nothing. `POST /v1/sessions` takes the same field. See `docs/AGENT_API.md` for how the agent should answer.

`play_along` (default `false`): with `answer_when_named` on, lets the agent say yes to a player's plans: join their party, follow them and help them fight. With `false`, the agent answers in a few words and says no to plans, and the client refuses to follow or join the party of a player who asked in chat. `POST /v1/sessions` takes the same field.

Account profile: copy `profiles/example.toml`. Extra `profiles/*.toml` files are gitignored. `--profile` supplies account, character, password env, and shard.

Persona files live in `personas/`. See `docs/PERSONAS.md`. Attach a persona with `connect --persona`. `agent run --persona` loads the persona into the session, then maps `class` to `set_goal`.

Logs: `RUST_LOG` or `log_level`. Passwords are not printed.

## Quick start (mock shard, no UO data)

You need two terminals. The mock character name is `Mara`.

### 1. Start the mock shard

```bash
uoterm mock-shard --bind 127.0.0.1:2593
```

### 2. Connect

```bash
export UO_PASS=test
uoterm connect --host 127.0.0.1 --port 2593 --account test --character Mara --era t2a
```

Expected line: `session s1 started; api 127.0.0.1:7733; encryption none`. Leave this process running.

`--era t2a` matches the mock. The CLI default for `--era` is `modern`.

### 3. Drive the session

```bash
uoterm --json session list
uoterm look
uoterm say "vendor buy"
uoterm state --json
uoterm walk --dir south --run --hold-ms 2000
uoterm open-door
uoterm move --to 1426,1693,0
uoterm agent run --persona personas/lumberjack.toml
uoterm agent stop
```

`look` prints a 21x21 ASCII radar (`@` is self, `i` is a ground item).

### 4. HTTP

```bash
curl http://127.0.0.1:7733/health
curl http://127.0.0.1:7733/v1/sessions
curl http://127.0.0.1:7733/v1/sessions/s1/state
curl -X POST http://127.0.0.1:7733/v1/sessions/s1/tools/say \
  -H 'content-type: application/json' \
  -d '{"text":"vendor buy"}'
```

If `UOTERM_API_TOKEN` is set, add `-H "Authorization: Bearer $UOTERM_API_TOKEN"` to every request except `/health`.

Stop with Ctrl+C on the `connect` process, then on the mock shard.

## Command reference

| Command | Role | Notes |
| --- | --- | --- |
| `uoterm mock-shard [--bind HOST:PORT]` | Server | Unencrypted demo shard. Default `127.0.0.1:2593`. |
| `uoterm connect ...` | Server | Login and HTTP API. Blocks until Ctrl+C. |
| `uoterm populate --manifest PATH` | Server | Start many sessions, then HTTP API. |
| `uoterm session list` | Client | Session ids on the API. |
| `uoterm session attach <id>` | Client | Print state for one id. |
| `uoterm say "text"` | Client | Tool `say`. Persona rejects `*emotes*`. |
| `uoterm move --to x,y,z` | Client | Tool `move_to`. |
| `uoterm walk --dir DIR [--run] [--hold-ms N]` | Client | Tool `walk`. One step or a hold stream of `0x02`. |
| `uoterm open-door` | Client | Tool `open_door` (`0x12`/`0x58`). |
| `uoterm look` | Client | Radar. `--json` prints full observe JSON. |
| `uoterm state` | Client | YAML. `--json` for JSON. Field name is `self_state`. |
| `uoterm agent run --persona FILE [--goal NAME]` | Client | `set_persona` then `set_goal`. |
| `uoterm agent stop` | Client | `cancel_goal`. |
| `uoterm harvest log [--since 1h] [--jsonl]` | Client | Reads `{data_dir}/uoterm/harvest.jsonl`. |
| `uoterm mcp` | Client | MCP stdio. Proxies to `--api`. |

### `connect` flags

| Flag | Default | Meaning |
| --- | --- | --- |
| `--host` | required | Login host |
| `--port` | `2593` | Login port |
| `--account` | required unless `--profile` | Account name |
| `--password-env` | `UO_PASS` | Env var that holds the password |
| `--character` | required unless `--profile` | Character name on the account |
| `--shard` | none | Select by name when the server list has more than one |
| `--version` | era default (`7.0.102.3` for `modern`) | Client version string (`0xBD`) |
| `--era` | `modern` | `t2a` or `modern` |
| `--encryption` | `none` | `none` = nocrypt. `osi` = Classic Client encryption. |
| `--uopath` | none | Client data directory. Without it, nav uses an open mock grid |
| `--profile` | none | TOML profile |
| `--persona` | built-in lumberjack | Persona TOML used by speech and reflex |
| `--api-bind` | from config, `127.0.0.1:7733` | HTTP listen address |

## MCP (LLM attach)

Start `connect` or `populate` first. Then point the model host at `uoterm mcp`.

The process speaks JSON-RPC 2.0 on stdio (`protocolVersion` `2024-11-05`). It accepts newline JSON and MCP `Content-Length` framing. Bodies larger than 1 MiB are rejected. Bad JSON returns JSON-RPC error `-32700`.

It lists tools and proxies `tools/call` to `POST /v1/sessions/{id}/tools/{name}`. Resource URI: `uo://session/{id}/state`.

```json
{
  "mcpServers": {
    "uoterm": {
      "command": "/absolute/path/to/uoterm",
      "args": ["mcp"],
      "env": {
        "UOTERM_API": "http://127.0.0.1:7733",
        "UOTERM_API_TOKEN": "replace-me"
      }
    }
  }
}
```

Give the model `observe` plus `say`, `move_to`, and `set_goal`. Do not ask it to walk tile by tile. Full tool notes: `docs/AGENT_API.md`.

## Personas

Shipped files:

| File | Class | Default goal from `agent run` |
| --- | --- | --- |
| `personas/lumberjack.toml` | lumberjack | `gather` |
| `personas/aldreth.toml` | banker_idle | `social` |
| `personas/cedric.toml` | traveler | `travel` |
| `personas/traveler.toml` | traveler | `travel` |

The persona `name` is speech-policy identity. The shard character is `--character` or `profile.character`.

Goals: `gather` uses the hatchet, then targets a tree. `hunt` attacks grey+ mobiles and bandages. `flee` and `travel` pathfind. `bank` walks toward Britain bank (1425, 1695). `shop` uses a nearby innocent mobile, else the bank. `social` says `yo`. `ress` walks a dead character toward the bank.

Rules in code: reject `*emotes*` unless `allow_emote`; shorten long lines; clamp `typo_rate` to `0.0..=1.0`; skip populate agents outside `active_hours`. See `docs/PERSONAS.md`.

## Scripts, agents and hotkeys

UOTerm has an assistant built in. A script is a plain list of commands, one
on each line, in the command style UO assistant scripts have long used:

```text
while not dead
  if poisoned
    cast 'Cure' 'self'
  elseif hits < maxhits
    bandageself
    pause 10000
  endif
  pause 500
endwhile
```

Run it with the `run_script` tool. Agents loot, pick up, organize, restock,
dress, buy, sell, bandage and remount on their own; hotkeys are named actions
such as `Bandage Self` or `Cast Greater Heal`; and `record_macro` writes what
you do as a script. Everything goes at the pace a person plays.

A shard can send a list of assistant features it forbids. UOTerm obeys the
list by default; set `obey_shard_rules = false` to ignore it.

See `docs/SCRIPTS.md` and `docs/AGENTS.md`.

## Populate (several sessions)

```bash
export UO_PASS=test
uoterm populate --manifest shards/britannia.toml
```

Each `[[agents]]` row needs a working profile and a persona. Agents outside `active_hours` (local clock) are skipped.

## Private shard you operate

```bash
export UO_PASS=your_password
uoterm connect \
  --host 192.168.1.10 --port 2593 \
  --account your_account --character "Mara of Yew" \
  --version 7.0.102.3 --era modern --encryption none \
  --uopath /path/to/uo \
  --persona personas/lumberjack.toml
```

For a shard that uses Classic Client login encryption:

```bash
uoterm connect \
  --host 192.168.1.10 --port 2593 \
  --account your_account --character "Mara of Yew" \
  --version 7.0.102.3 --era modern --encryption osi \
  --uopath /path/to/uo
```

Notes:

- `--encryption none` is nocrypt. This is the usual freeshard mode.
- `--encryption osi` is the Classic Client encryption codec. This repository does not ship official-server keys.
- `--era modern` is the CLI default. Use `--era t2a` for 1.26–2.0.x style packets.
- `--era modern` sends the 21-byte `0xEF` seed. `--era t2a` sends a 4-byte seed.
- `--uopath` is optional. If you pass it and the directory is unreadable, `connect` fails with that error.
- `--era modern` opens a new TCP connection after `0x8C`. If the packet IP is `0.0.0.0`, it reconnects to `--host`. `--era t2a` may stay on the login socket when `stay_on_socket` is set (mock shard).

## Tests

No official client files required:

```bash
cargo test --workspace
```

Some checks compare the map reader against real Ultima Online files. They skip
themselves when they have none. Point `UOTERM_TEST_UOPATH` at a client
directory to run them:

```bash
UOTERM_TEST_UOPATH=/path/to/uo cargo test --workspace
```

CI (`.github/workflows/ci.yml`) builds on Ubuntu and Windows: `cargo fmt`, `clippy`, `cargo test --workspace`, `uoterm --help`. It also rejects committed `.mul` / `.uop` / `.idx` files.

## Troubleshooting

| Symptom | Cause | Action |
| --- | --- | --- |
| `password env UO_PASS is not set` | Env var missing | `export UO_PASS=...` in the same shell as `connect` |
| `no active session; run uoterm connect first` | API down or wrong `--api` | Start `connect`. Check `curl http://127.0.0.1:7733/health` |
| `401 unauthorized` | Token set, header missing | Export `UOTERM_API_TOKEN` in the client shell |
| `non-loopback --api-bind requires UOTERM_API_TOKEN` | Public bind without token | Set the token or bind `127.0.0.1` |
| `Address already in use` on the API | Second `connect`/`populate` | Use one server process |
| `(no sessions)` | Client talks to empty API | Same host/port as `--api-bind` |
| `(no harvest lines)` | No events yet, or wrong data dir | Confirm `connect` is running; Linux dir is `~/.local/share/uoterm` |
| Populate starts 0 sessions | `active_hours` miss local time | Widen hours or run inside the window |
| Speech rejected | `*emotes*` or empty text | Send a short line without asterisks |
| Shard ignores you | Era, seed, or encryption mismatch | Try `--era t2a --encryption none` on the mock first |

## Layout

- `crates/uoterm-protocol` — framing, Huffman, codecs
- `crates/uoterm-world` — serials, journal, radar
- `crates/uoterm-nav` — MUL/UOP, A*, skill names
- `crates/uoterm-assist` — spells, weapon moves, potions and other game data
- `crates/uoterm-script` — the script language: parser and step interpreter
- `crates/uoterm-runtime` — session, mock shard, reflex, scripts, agents, hotkeys, HTTP
- `crates/uoterm` — CLI binary

Docs: `docs/PROTOCOL.md` (wire protocol), `docs/AGENT_API.md` (tools, HTTP, MCP), `docs/SCRIPTS.md` (script language), `docs/AGENTS.md` (agents, hotkeys, recording), `docs/PERSONAS.md` (persona files), `LEGAL.md`.

## License

GNU Affero General Public License, version 3 only. See `LICENSE` and
`LEGAL.md`.
