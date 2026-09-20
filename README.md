# UOTerm

UOTerm is a headless Ultima Online client. Its main purpose is to let AI agents control player characters and play the game the way a human player does: see the world, walk, fight, gather, talk, use items, and answer gumps. A second purpose is testing and debugging by humans.

It speaks the Ultima Online wire protocol. Headless `connect` is the default. Optional `--view` opens a watch window. It draws the real map from your client files. It only looks, until you press "Take control". It is not a second login.

Ultima Online is a trademark of its owners. UOTerm is independent and unaffiliated. This repository does not ship MUL, UOP, or other client data. You supply a legitimate client directory when you need walkability from map files.

Target shards you operate: your own servers, demo servers, and offline worlds. Official Terms of Service may forbid third-party clients. Do not market UOTerm as an official-server bot. See `LEGAL.md`.

## Shard compatibility

UOTerm matches Classic Client packet layouts and login. It does not special-case
a shard by name, so any server that accepts a Classic Client should work.

Treat every capability as unproven until a test in this tree proves it.

`cargo test --workspace` covers mock login (account `0x80` and game `0x91`), walk, speech, and a gather loop without official client files.

`uoterm mock-shard` is a local demo for those tests. It is not a live world. It binds `127.0.0.1:2593` by default. Do not start it while a private shard already uses that port. Stop the demo before you start a live shard.

## What it is not

- Not a copy of the classic client. `--view` / `uoterm watch` shows what the agent does, and it is a full way to play when you press "Take control". You walk with a click, with the arrow keys or W A S D, or with the right mouse button held. You drag items between bags, onto your character, onto other mobiles, onto the ground and into a trade. A right-click opens a ring of acts with the context menu of the shard. It has tooltips, a character sheet with worn items, skills with locks, spells and the party, a hotbar, shop and trade windows, gumps with text fields, old-style menus, books, prompts, speech over heads, damage numbers, spell effects, night, rain and snow, houses and boats, and sound. It draws the land, the items, and the mobiles from your client files. Each mobile shows with its mount and worn items, and it walks, runs, stands, or swings by what the shard sends. The pictures come from the classic `anim*.mul` files and from the newer `AnimationFrame*.uop` packages. A person in war mode stands ready, a person with a weapon walks armed, a swing or a cast plays when the shard sends it, and a corpse shows the fallen body. A mobile with no picture in these files shows as a plain colored figure. A Map button opens a map of the land round you, and a click on it walks you there. A map item you open, such as a treasure map, shows its own land with its pins; a click puts a pin, and with a TypeSafe key a field takes the place in plain words, such as `the bank in britain`, and Jev picks it from the named places of your marker file. An arrow points at a place the shard names, and it waits at the edge of the window when that place is out of view. Marks the shard puts on the map show with their names, and its notices and its web links go in the journal; UOTerm never opens a link by itself. A Chat button opens the chat of the shard: its channels, its lines and a box to talk in; with a TypeSafe key a field joins a channel from plain words such as `the trade one`. A Help button asks the shard for its help menu. When the shard opens the house designer, a Build panel shows the parts of the client catalog; you pick a style and a piece and click the house to build, and a field takes the part in plain words, such as `a stone wall`, which Jev picks from the catalog. A Profile button shows what a player wrote about his character, and a right-click on a mobile shows his; you may change your own. The land and the trees change with the season the shard sets, houses players designed show with their own walls and floors, and a building that waits for its place shows where the mouse points before you put it there. Gumps show in their shard layout, from the gump art of the client; without the client files they show as lists. A bulletin board shows its messages with each answer under its message, and you can read, post, answer and remove. It does not have these parts yet: the house designer itself, and custom house design.
- Not a click-macro overlay.
- Not an official-server farm bot.
- Not a cheat tool for EA or Broadsword shards.
- Not a replacement for a live shard. `mock-shard` is a demo only.

## Requirements

- Rust 1.87 or later (stable). `rust-toolchain.toml` pins `stable`.
- On Linux, the ALSA developer package for the sound of the watch window: `sudo apt install libasound2-dev` (Debian, Ubuntu) or `alsa-lib-devel` (Fedora).
- Linux or Windows. macOS is untested.
- A TCP port for the private shard (default `2593`). The mock demo uses the same default; run only one of them.
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

`uoterm connect` and `uoterm populate` stay in the foreground. They log in, then serve HTTP until you press Ctrl+C. `connect --view` opens the watch window in the same process, so the window reads the session with no HTTP step. Closing the window ends the program. The "Quit" button of the window does the same, after it asks the shard to log the character out. `uoterm watch` is a different process: it reads a running session through the HTTP API.

All other commands (`session`, `say`, `move`, `walk`, `open-door`, `look`, `state`, `agent`, `watch`, `mcp`) are clients. They call that HTTP API. They do not open a second game socket.

Do not start a second `connect` on the same API port. Do not start `mock-shard` on the same game port as a live shard.

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

Copy `uoterm.toml.example` to `uoterm.toml` if you want a local API bind. `connect` uses this file for every `AppConfig` key when the matching CLI flag is omitted: `host`, `port`, `era`, `log_level`, `api_bind`, `max_sessions`, `obey_shard_rules`, `answer_when_named`, `play_along`, `uopath`, `markers`.

`obey_shard_rules` (default `true`): some shards send a list of assistant features they forbid, such as auto-open doors, auto-bandage, and auto-potions. With `true`, the character does not use those features by itself on that shard. With `false`, it ignores the list. The client answers the shard in both cases. A session made with `POST /v1/sessions` takes the same `obey_shard_rules` field.

`answer_when_named` (default `true`): when another character says your character's name, the agent gets a `spoken_to` event, an `unanswered` list on every tool result until your character speaks, and a `spoken_to` list in `observe`, so it can answer. With `false`, the agent is told nothing. `POST /v1/sessions` takes the same field. See `docs/AGENT_API.md` for how the agent should answer.

`play_along` (default `false`): with `answer_when_named` on, lets the agent say yes to a player's plans: join their party, follow them and help them fight. With `false`, the agent answers in a few words and says no to plans, and the client refuses to follow or join the party of a player who asked in chat. `POST /v1/sessions` takes the same field.

Account profile: copy `profiles/example.toml`. Extra `profiles/*.toml` files are gitignored. `--profile` supplies account, character, password env, and shard.

Persona files live in `personas/`. See `docs/PERSONAS.md`. Attach a persona with `connect --persona`. `agent run --persona` loads the persona into the session, then maps `class` to `set_goal`. Extra persona and shard files are gitignored; the shipped files stay tracked.

Logs: `RUST_LOG` or `log_level`. Passwords are not printed.

## Quick start (private shard you operate)

The shard process listens on `2593`. UOTerm only logs in. Do not start `uoterm mock-shard` in this path: both want port `2593`.

```bash
export UO_PASS=your_password
uoterm connect \
  --host 127.0.0.1 --port 2593 \
  --account your_account --character Mara \
  --version 7.0.116.0 --era modern --encryption none \
  --uopath /path/to/uo \
  --persona personas/traveler.toml
```

Expected line: `session s1 started; api 127.0.0.1:7733; encryption none`. Leave this process running.

Optional `--view` opens the watch window in the same process. Closing the window ends the program. `view = true` in `uoterm.toml` does the same on each `connect`. Optional `--text-view` prints the radar in that terminal. Those flags conflict with each other.

Password is `UO_PASS`. Never put it in a file.

## Quick start (mock demo, tests only)

Use this only when no live shard is using `2593`. The mock is not a real world. Stop it before you start a live shard.

You need two terminals. The mock character name is `Mara`. `--era t2a` matches the mock. The CLI default for `--era` is `modern`.

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

Stop with Ctrl+C on the `connect` process, then on the mock shard if you used one.

## Command reference

| Command | Role | Notes |
| --- | --- | --- |
| `uoterm mock-shard [--bind HOST:PORT]` | Demo | Unencrypted demo shard. Default `127.0.0.1:2593`. Do not run this while a live shard uses that port. |
| `uoterm connect ...` | Server | Login and HTTP API. Blocks until Ctrl+C. `--view` opens the watch window in the same process. `--text-view` prints the radar in that terminal. |
| `uoterm populate --manifest PATH` | Server | Start many sessions, then HTTP API. |
| `uoterm session list` | Client | Session ids on the API. |
| `uoterm session attach <id>` | Client | Print state for one id. |
| `uoterm say "text"` | Client | Tool `say`. Persona rejects `*emotes*`. |
| `uoterm move --to x,y,z` | Client | Tool `move_to`. |
| `uoterm walk --dir DIR [--run] [--hold-ms N]` | Client | Tool `walk`. One step or a hold stream of `0x02`. |
| `uoterm open-door` | Client | Tool `open_door` (`0x12`/`0x58`). |
| `uoterm look` | Client | Radar. `--json` prints full observe JSON. |
| `uoterm play [--profile FILE] [--go] [--encryption none\|osi] [--uopath DIR]` | Server | Play by hand. It opens the login screens: a form with host, port, account, password, shard and character, and the saved logins of the `profiles` folder. You type the password in a field that hides it; it stays in memory for the login and is written nowhere. When the field is empty, the password comes from the environment variable of the saved login. A shard list with more than one shard, and an account with more than one character, show as lists to click. With a TypeSafe key, a field takes plain words such as `my miner on the test shard`; Jev picks the saved login, and later the shard and the character, from the lists. Jev sees the words and the names of the lists, never the account or the password. After the login the same window is the game window, you have control, and the HTTP API runs as with `connect`. `--go` logs in at once with `--profile`. |
| `uoterm watch [--text] [--uopath DIR] [--open sheet\|map\|macros\|profile\|chat] [--snapshot FILE.png]` | Client | Live window of the running session: the map, vitals, what the agent does, who is near, the journal, the pack. With client files (`--uopath`, or `uopath` in `uoterm.toml`) it draws the real map. Without them it draws flat colors from the radar. Scroll to zoom. "Take control" stops the agent and lets you click: the ground to walk, a double-click to use (or to attack in war mode), one click to look, and the target cursor. You also walk with the arrow keys or W A S D (Shift runs) and with the right mouse button held. You drag items to move, wear, give, trade or drop them; hold Shift to split a pile. A right-click on a thing opens a ring of acts with the context menu of the shard. "Bag", "Sheet" and "Map" open the backpack, the character sheet (worn items, skills with locks, spells, party) and the map of the land; `--open` opens the sheet or the map at the start. The hotbar takes a dragged item, a pinned skill or spell, or a pinned command; the keys 1 to 0 use its slots, and it is saved in `watch-hotbar.toml`. "Macros" opens the macro editor: pick a script of the scripts folder, change its lines, run it once or in a loop, save it, record what you do as a new macro, or pin it to the hotbar. With a TypeSafe key, a field takes the next step in plain words, such as `heal myself with a bandage`; Jev picks the hotkey that does it, and the script lines of that hotkey go into the macro. The chat box says words. In Do mode it runs one script command. When the shard asks for words, the box answers it. In Order mode it takes a plain order such as `attack the orc`; TypeSafe's Jev model picks the act and the target, and the order and the names of the things near go to `api.typesafe.ai`. Order mode is on only when `TYPESAFE_API_KEY` is set (environment or `.env`). "Give back", or 90 s with no act, returns the character to the agent. The "Options" button opens the sound panel: a master volume, and one volume each for music, sound effects, and footsteps, with a switch for silence. The sounds and the music come from your client files. The settings are saved in `watch-audio.toml` in the UOTerm config folder. `--snapshot` saves one PNG picture and closes. `--text` prints the radar in the terminal. |
| `uoterm state` | Client | YAML. `--json` for JSON. Field name is `self_state`. |
| `uoterm agent run --persona FILE [--goal NAME]` | Client | `set_persona` then `set_goal`. |
| `uoterm agent stop` | Client | `cancel_goal`. |
| `uoterm harvest log [--since 1h] [--jsonl]` | Client | Reads `{data_dir}/uoterm/harvest.jsonl`. |
| `uoterm mcp` | Client | MCP stdio. Proxies to `--api`. |

### `connect` flags

| Flag | Default | Meaning |
| --- | --- | --- |
| `--host` | from config, `127.0.0.1` | Login host |
| `--port` | from config, `2593` | Login port |
| `--account` | required unless `--profile` | Account name |
| `--password-env` | `UO_PASS` | Env var that holds the password |
| `--character` | required unless `--profile` | Character name on the account |
| `--shard` | none | Select by name when the server list has more than one |
| `--version` | era default (`7.0.102.3` for `modern`) | Client version string (`0xBD`) |
| `--era` | `modern` | `t2a` or `modern` |
| `--encryption` | `none` | `none` = nocrypt. `osi` = Classic Client encryption. |
| `--uopath` | from config | Client data directory. Without it, nav uses an open mock grid |
| `--profile` | none | TOML profile |
| `--persona` | built-in lumberjack | Persona TOML used by speech and reflex |
| `--api-bind` | from config, `127.0.0.1:7733` | HTTP listen address |
| `--view` | off | Open the watch window in the same process. Closing the window ends the program, as Ctrl+C does. |
| `--text-view` | off | Print a live radar in this terminal. Conflicts with `--view`. |

## MCP (LLM attach)

Start `connect` or `populate` first. Then point the model host at `uoterm mcp`.

The process speaks JSON-RPC 2.0 on stdio (`protocolVersion` `2024-11-05`). It accepts newline JSON and MCP `Content-Length` framing. Bodies larger than 1 MiB are rejected. Bad JSON returns JSON-RPC error `-32700`.

It lists tools and proxies `tools/call` to `POST /v1/sessions/{id}/tools/{name}`. Resources: `uo://session/{id}/state` (observe JSON) and `uo://playbook/{name}` (markdown in `docs/playbooks/`). Read `driver` first, then `hunt` or `walk`.

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

Give the model `observe` plus `say`, `move_to`, `job_start`, and `next_event`. Read playbook `driver` first. Do not ask it to walk tile by tile. Full tool notes: `docs/AGENT_API.md`.

## Personas

Shipped files:

| File | Class | Default goal from `agent run` |
| --- | --- | --- |
| `personas/lumberjack.toml` | lumberjack | `gather` |
| `personas/aldreth.toml` | banker_idle | `social` |
| `personas/cedric.toml` | traveler | `travel` |
| `personas/traveler.toml` | traveler | `travel` |

The persona `name` is speech-policy identity. The shard character is `--character` or `profile.character`.

Goals: `gather` uses the hatchet, then targets a tree. `hunt` starts the melee hunt job (kill, loot own kills, flee, then `job_ended`). `flee` and `travel` pathfind. `bank` walks toward Britain bank (1425, 1695). `shop` uses a nearby innocent mobile, else the bank. `social` says `yo`. `ress` walks a dead character toward the bank.

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

## Private shard on another host

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
- Both eras open a new TCP connection to the game server after `0x8C`. If the packet IP is `0.0.0.0`, it reconnects to `--host`.

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
| `Address already in use` on `2593` | `mock-shard` and a live shard together | Stop `mock-shard`. Start only the live shard |
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

Docs: `docs/PROTOCOL.md` (wire protocol), `docs/AGENT_API.md` (tools, HTTP, MCP), `docs/SCRIPTS.md` (script language), `docs/AGENTS.md` (agents, hotkeys, recording), `docs/PERSONAS.md` (persona files), `docs/playbooks/` (driver, login, hunt, walk, navigation, loot, bank, death, moongate, dungeon, mounts, runebook, buy, sell, containers, talk, inspect, equip), `LEGAL.md`.

## License

GNU Affero General Public License, version 3 only. See `LICENSE` and
`LEGAL.md`.
