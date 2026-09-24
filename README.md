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

- Not a copy of the classic client's code. The play window (`--view`, `uoterm watch`, `uoterm play`) shows what the agent does, and it is a full way to play when you press "Take control". See "The play window" below.
- Not a click-macro overlay.
- Not an official-server farm bot.
- Not a cheat tool for EA or Broadsword shards.
- Not a replacement for a live shard. `mock-shard` is a demo only.

## The play window

The window has two looks, and each has every feature of the classic client:

- **Modern** (the default): glass panels over the world, a bar at the top, the character sheet, the hotbar, a journal with the chat box under it, and a ring of acts on a right click. The panels move by their titles, lock, and open where you left them.
- **Classic**: the look of the official client. Floating gumps drawn from the gump art of your client files, its fonts and its mouse pointers, a game window in a frame that you move and size, and the chat line at the foot of that window. Pick it on the Interface page of the Options ("UI style"). It needs the client files; without them the window keeps the Modern look.

Both looks read the same data and send the same acts. The login screens are one plain UOTerm form for both looks, with no music.

You walk with the arrow keys, with the right mouse button held, with a double-click on the ground when pathfinding is on, or with a game controller; Alt+click on the ground runs there, and W A S D and a plain click that runs are options. You drag items between bags, onto your character, onto other mobiles, onto the ground and into a trade. A right-click opens a ring of acts (Modern) or the context menu of the shard (Classic). There are tooltips, the paperdoll with worn items, the status, skills with locks and groups, spellbooks, the party with the hits, mana and stamina of each member the shard tells, health bars, a hotbar, buff icons, shop and trade windows, gumps with text fields, old-style menus, books, bulletin boards, prompts, speech over heads, damage numbers, spell effects, night, rain and snow, houses and boats, and sound.

The world is drawn from your client files: the land, the items, and the mobiles with their mounts and worn items, from the classic `anim*.mul` files and the newer `AnimationFrame*.uop` packages. A mobile walks, runs, stands, swings or casts by what the shard sends, and a corpse shows the fallen body. A mobile with no picture in these files shows as a plain colored figure. The land and the trees change with the season the shard sets, houses players designed show with their own walls and floors, and a building that waits for its place shows where the mouse points.

A map of the land round you walks you where you click it. A map item, such as a treasure map, shows its own land with its pins; a click puts a pin, and with a TypeSafe key a field takes a place in plain words, such as `the bank in britain`, and Jev picks it from the named places of your marker file. An arrow points at a place the shard names. Marks the shard puts on the map show with their names, and its notices and web links go in the journal; UOTerm never opens a link by itself. The chat of the shard shows its channels, its lines and a box to talk in; with a TypeSafe key a field joins a channel from plain words such as `the trade one`.

When the shard opens the house designer, both looks show the parts of the client catalog: pick a style and a piece and click the house to build, erase a part, or pick a part off the house with the eyedropper. A button for each storey turns how it shows while you design (walls or floor see-through or hidden, or all hidden), and the designer counts the components, the fixtures (doors and teleporters) and the cost against what the plot allows, as the classic designer does. A field takes a part in plain words, such as `a stone wall`, which Jev picks from the catalog.

The Options have the pages of the classic client: General, Sound, Video, Macros, Tooltip, Fonts, Speech, Combat & Spells, Counters, Info Bar, Containers, Experimental, Ignore List, Interface, Nameplates, Journal, World Map and Agents. Apply and Okay keep a change, Cancel drops it, and Default puts one page back. The options, the places of the windows and the gumps you keep open are a profile for each character of each shard, in the `profiles` folder of the UOTerm config folder (Linux `~/.config/uoterm`, Windows `%APPDATA%\uoterm`): `default.toml` is where a new character starts, and each character has `<shard>/<character>.toml`. The hotbar of each character is kept in `watch-hotbar.toml`.

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

Copy `uoterm.toml.example` to `uoterm.toml` if you want a local API bind. `connect` uses this file for every `AppConfig` key when the matching CLI flag is omitted: `host`, `port`, `era`, `log_level`, `api_bind`, `max_sessions`, `obey_shard_rules`, `answer_when_named`, `play_along`, `view`, `reconnect`, `uopath`, `markers`, `proxy`.

`obey_shard_rules` (default `true`): some shards send a list of assistant features they forbid, such as auto-open doors, auto-bandage, and auto-potions. With `true`, the character does not use those features by itself on that shard. With `false`, it ignores the list. The client answers the shard in both cases. A session made with `POST /v1/sessions` takes the same `obey_shard_rules` field.

`answer_when_named` (default `true`): when another character says your character's name, the agent gets a `spoken_to` event, an `unanswered` list on every tool result until your character speaks, and a `spoken_to` list in `observe`, so it can answer. With `false`, the agent is told nothing. `POST /v1/sessions` takes the same field. See `docs/AGENT_API.md` for how the agent should answer.

`play_along` (default `false`): with `answer_when_named` on, lets the agent say yes to a player's plans: join their party, follow them and help them fight. With `false`, the agent answers in a few words and says no to plans, and the client refuses to follow or join the party of a player who asked in chat. `POST /v1/sessions` takes the same field.

`reconnect` (default `true`): when the link to the shard drops, the session logs in again by itself. It waits 5 s before the first try, and it doubles the wait after each failed try, up to 60 s. While it waits, the agent gets a `disconnected` event and each tool call says the session waits to log in again. A logout that the agent or you ask for does not log in again. With `false`, the session ends when the link drops. `POST /v1/sessions` takes the same field.

`proxy` (default none): reach the shard through a proxy, `socks5://host:port` or `http://host:port` (HTTP CONNECT), with `user:password@` before the host when the proxy asks for a login. The login and the game link both go through it. `POST /v1/sessions` takes the same field. The log shows the proxy with its password left out.

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
| `uoterm populate --manifest PATH [--api-bind ADDR]` | Server | Start many sessions, then HTTP API. |
| `uoterm session list` | Client | Session ids on the API. |
| `uoterm session attach <id>` | Client | Print state for one id. |
| `uoterm say "text"` | Client | Tool `say`. Persona rejects `*emotes*`. |
| `uoterm move --to x,y,z` | Client | Tool `move_to`. |
| `uoterm walk --dir DIR [--run] [--hold-ms N]` | Client | Tool `walk`. One step or a hold stream of `0x02`. |
| `uoterm open-door` | Client | Tool `open_door` (`0x12`/`0x58`). |
| `uoterm look` | Client | Radar. `--json` prints full observe JSON. |
| `uoterm play [--profile FILE] [--go] [--encryption none\|osi] [--uopath DIR] [--api-bind ADDR]` | Server | Play by hand. It opens the login screens: a form with host, port, account, password, shard and character, and the saved logins of the `profiles` folder. You type the password in a field that hides it; it stays in memory for the login and is written nowhere. When the field is empty, the password comes from the environment variable of the saved login. A shard list with more than one shard shows as a list to click. The characters of the account show with an empty slot for each free place: click one to play, press Delete twice to remove one, or type a name and press "New character" to make one. When the shard refuses, it says why. With a TypeSafe key, a field takes plain words such as `my miner on the test shard`; Jev picks the saved login, and later the shard and the character, from the lists. Jev sees the words and the names of the lists, never the account or the password. After the login the same window is the game window, you have control, and the HTTP API runs as with `connect`. `--go` logs in at once with `--profile`. |
| `uoterm watch [--text] [--uopath DIR] [--open sheet\|map\|macros\|profile\|chat] [--snapshot FILE.png]` | Client | Live window of the running session: the map, vitals, what the agent does, who is near, the journal, the pack. With client files (`--uopath`, or `uopath` in `uoterm.toml`) it draws the real map. Without them it draws flat colors from the radar. Scroll to zoom. "Take control" stops the agent and lets you click: a double-click to use (or to attack in war mode), one click to look, and the target cursor. You walk with the arrow keys (Shift runs) and with the right mouse button held; a left click while it is held keeps you going. A double-click on the ground walks there when "Enable pathfinding" is on (General page; "Use Shift for pathfinding" asks for Shift too), Alt+click on the ground runs there, and "Click on the ground runs there" and "Use W A S D to walk" are options of the same page. See "Keys, macros and the controller" below. You drag items to move, wear, give, trade or drop them; hold Shift to split a pile. A right-click on a thing opens a ring of acts with the context menu of the shard (in the Classic look, the context menu gump). "Bag", "Sheet" and "Map" open the backpack, the character sheet (worn items, skills with locks, spells, party) and the map of the land; `--open` opens the sheet or the map at the start. The hotbar takes a dragged item, a pinned skill or spell, or a pinned command; the keys 1 to 0 use its slots, it moves and locks like the other panels, and it is saved in `watch-hotbar.toml`. "Macros" opens the macro editor: pick a script of the scripts folder, change its lines, run it once or in a loop, save it, record what you do as a new macro, or pin it to the hotbar. With a TypeSafe key, a field takes the next step in plain words, such as `heal myself with a bandage`; Jev picks the hotkey that does it, and the script lines of that hotkey go into the macro. The chat box says words. In Do mode it runs one script command. When the shard asks for words, the box answers it. In Order mode it takes a plain order such as `attack the orc`; TypeSafe's Jev model picks the act and the target, and the order and the names of the things near go to `api.typesafe.ai`. Order mode is on only when `TYPESAFE_API_KEY` is set (environment or `.env`). "Give back", or 90 s with no act, returns the character to the agent. The "Options" button opens the Options with all their pages (see "The play window" above); the sounds and the music come from your client files, and the options are kept in the profile of the character. `--snapshot` saves one PNG picture and closes. `--text` prints the radar in the terminal. |
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
| `--version` | the version of `client.exe` in `--uopath`; else the era default (`7.0.102.3` for `modern`) | Client version string (`0xBD`). Shards that check versions kick a client older than their own `client.exe`. |
| `--era` | `modern` | `t2a` or `modern` |
| `--encryption` | `none` | `none` = nocrypt. `osi` = Classic Client encryption. |
| `--uopath` | from config | Client data directory. Without it, nav uses an open mock grid |
| `--markers` | from config | Marker file of named places to travel to: a UO Auto Map `.map` file or an Ultima Mapper `Waypoints.lua` file |
| `--profile` | none | TOML profile |
| `--persona` | built-in lumberjack | Persona TOML used by speech and reflex |
| `--api-bind` | from config, `127.0.0.1:7733` | HTTP listen address |
| `--view` | off | Open the watch window in the same process. Closing the window ends the program, as Ctrl+C does. |
| `--text-view` | off | Print a live radar in this terminal. Conflicts with `--view`. |

## MCP (LLM attach)

Start `connect` or `populate` first. Then point the model host at `uoterm mcp`.

The process speaks JSON-RPC 2.0 on stdio (`protocolVersion` `2024-11-05`). It accepts newline JSON and MCP `Content-Length` framing. Bodies larger than 1 MiB are rejected. Bad JSON returns JSON-RPC error `-32700`.

It lists tools and proxies `tools/call` to `POST /v1/sessions/{id}/tools/{name}`. The runtime's own tools, `connect`, `disconnect`, `characters`, `character_create` and `character_delete`, need no session and go to `POST /v1/tools/{name}`: an agent can list the characters of an account, make or delete one, and log in, with the password in an environment variable (`password_env`) or a saved login (`profile`), never in a call. Resources: `uo://session/{id}/state` (observe JSON) and `uo://playbook/{name}` (markdown in `docs/playbooks/`). Read `driver` first, then `hunt` or `walk`.

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

Tools for the map and the world beyond `observe`: `find_tiles` (water, trees, ore, a forge or an anvil, by kind, graphic, tiledata flag or name, on any map), `map_tile` (everything on one tile), `multi_parts` (the parts of houses and boats), `find_entrances` (stairs, ladders and pads into dungeons), `line_of_sight` (any point to any point, by the rules of RunUO, POL or Sphere, with a trace), `find_landmarks` and `landmarks_info`. `move_to` and `route` take `run`, `accuracy`, `open_doors`, `avoid`, `roads`, `exact` and a landmark `name`, and say how long the route took to plan. One-click acts are tools too: `virtue` (all eight), `virtue_gump`, `skill_lock`, `stat_lock`, `rename`, `set_ability`, `emote_action`, `fly`, `menu_button`, `target_resource`, `use_type`, `use_on`, `mount`, `dismount`, `attack_nearest`, `catch_bag`, `ignore_list` and `skill_gains`. The windows the shard opens have tools of their own: `dye`, `race_change`, `trade_gold`, `book_write`, `board_post`, `map_pin`, `profile`, `house_edit`, `chat`, `tip` and `quest_arrow`.

## Personas

Shipped files:

| File | Class | Default goal from `agent run` |
| --- | --- | --- |
| `personas/lumberjack.toml` | lumberjack | `gather` |
| `personas/aldreth.toml` | banker_idle | `social` |
| `personas/cedric.toml` | traveler | `travel` |
| `personas/traveler.toml` | traveler | `travel` |

The persona `name` is speech-policy identity. The shard character is `--character` or `profile.character`.

Goals: `gather` finds the nearest tree of the map in 12 tiles, walks beside it, uses an axe and aims at the tree, one swing at a time. `mine` does the same with a pickaxe or a shovel on rock and cave floors. A spot the shard says is empty is left alone for 20 minutes. Both goals end with `job_failed` when there is no tool or nothing to gather near. `hunt` starts the melee hunt job (kill, loot own kills, flee, then `job_ended`). `flee` walks away from the threats near, and `travel` pathfinds. `bank` walks to the nearest bank of the standard towns within 400 tiles. `shop` uses a nearby innocent mobile, else the bank. `social` says `yo`. `ress` walks a ghost to a healer in view and takes the offer to live again; with no healer in view it walks to a healer of the marker file, else to the nearest bank. It ends with `job_ended` when the character is alive. The bandage and the potions are used during every goal, when the shard allows them.

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

Run it with the `run_script` tool. Several scripts run side by side, each in
a named slot (a healer beside a task), and `for` and `iterations` bound a run.
Agents loot, pick up, organize, restock,
dress, buy, sell, bandage and remount on their own; hotkeys are named actions
such as `Bandage Self` or `Cast Greater Heal`; and `record_macro` writes what
you do as a script. Everything goes at the pace a person plays.

A shard can send a list of assistant features it forbids. UOTerm obeys the
list by default; set `obey_shard_rules = false` to ignore it.

See `docs/SCRIPTS.md` and `docs/AGENTS.md`.

## Keys, macros and the controller

The play window reads the keys as the official client does:

- **Chat line.** The Modern look has it under the journal; the Classic look
  at the foot of the game window, on a dark band ("Hide chat gradient" takes
  the band away), with the shard's own words and the party, guild and
  alliance lines over it for ten seconds. Both are the same line, with the
  same history. With "Activate chat when pressing Enter" off (the default,
  Speech page) the chat line always takes the keys you type; with it on,
  Enter opens it, Enter sends and closes it, and Shift+Enter sends and keeps
  it open ("Use Shift+Enter to send without closing chat"). The prefix keys
  also open it when "Speech prefix keys open the chat" is on. A line that
  starts with `! ` is yelled, `; ` whispered, `: ` an emote, `/` goes to the
  party (`/2 ` to its second member), `\` to the guild, `|` to the alliance
  and `,` to the global chat. Each goes out in the color the Speech page
  gives it. `/add`, `/rem`, `/loot`, `/accept`, `/decline` and `/quit` are
  party orders; one that cannot be done now prints the classic client's
  answer in the journal, such as "You are not in a party.", and words to a
  party you are not in come back as "Note to self". Ctrl+Q and Ctrl+W bring
  back the lines you sent.
- **Click to run.** A click on the ground with Alt held runs there by
  pathfinding ("Click with this key held runs there", General page: None,
  Ctrl, Shift or Alt). With "Click on the ground runs there" on, a plain
  click runs there too; the "Click-to-run on / off" action (Macros page)
  switches it from a key, and the journal says if it is on or off.
- **Tab** holds war mode while it is down, or switches it at each press when
  "Hold Tab for combat" is off (Combat & Spells page).
- **Default keys** of the official client: Alt+P paperdoll, Alt+O options,
  Alt+J journal, Alt+I backpack, Alt+R minimap, Ctrl+B bow, Ctrl+S salute.
- **Macros** (Options, Macros page): a macro has a name, a key chord or
  controller buttons, and steps. Each step is an action with its argument:
  every macro type of the reference client (say, walk, open or close any window, cast,
  use skill, zoom, toggles for roofs, trees, vegetation, caves, the circle
  of transparency, names and auras, select next / previous / nearest, grab,
  the view range, delays and "wait for target"), any hotkey of the session,
  a saved script, a script line, a screenshot, and mouse clicks for a
  controller. Click the key field and press the chord to bind it. While you
  type in a field of a panel or a gump, only chords with Ctrl or Alt and the
  F keys run macros.
- The **Experimental** page turns off the default keys, the arrow keys, Tab
  and Ctrl+Q / Ctrl+W, and the left click that keeps you walking.
- **Screenshots** (a macro step, or "Take a screenshot on death" on the
  Interface page) go to the `screenshots` folder of the config folder, and
  the journal tells where unless "Hide "Screenshot stored in" message" is on.
- **Criminal actions.** An attack, or a harmful target, on an innocent asks
  "This may flag you criminal!" first ("Query before attack"), as does a
  beneficial target on a criminal, a murderer or a gray one when "Query
  before beneficial acts" is on (Combat & Spells page).
- **Game controller** ("Use a game controller", Macros page): the left stick
  walks (pushed far, it runs), the right stick moves the mouse at the speed
  of the page, and buttons run macros. With the default buttons, South is a
  left click, East a right click, West a double click, North war / peace and
  Start the options.

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
  --era modern --encryption none \
  --uopath /path/to/uo \
  --persona personas/lumberjack.toml
```

For a shard that uses Classic Client login encryption:

```bash
uoterm connect \
  --host 192.168.1.10 --port 2593 \
  --account your_account --character "Mara of Yew" \
  --era modern --encryption osi \
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

A test never reads or writes your own UOTerm folders: only the `uoterm`
program uses `~/.config/uoterm` and `~/.local/share/uoterm`, and any other
process, such as a test, keeps its files in a folder of its own under the
temp folder.

Some checks compare the map reader against real Ultima Online files. They skip
themselves when they have none. Point `UOTERM_TEST_UOPATH` at a client
directory to run them:

```bash
UOTERM_TEST_UOPATH=/path/to/uo cargo test --workspace
```

CI (`.github/workflows/ci.yml`) builds on Ubuntu and Windows: `cargo fmt`, `clippy`, `cargo test --workspace`, `uoterm --help`. It also rejects committed `.mul` / `.uop` / `.idx` / `.def` files, and any shipped file that names another client or assistant tool.

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
