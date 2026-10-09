# UOTerm

UOTerm is an Ultima Online client with no window of its own. Its main job is
to let AI agents play a character the way a person does: see the world,
walk, fight, gather, talk, use items and answer gumps. Its second job is to
help people test and debug, and to play by hand.

It speaks the Ultima Online wire protocol, as the Classic Client does. When
you want to see the game, or play it, UOTerm has two clients: a native play
window and a web client.

Ultima Online is a trademark of its owners. UOTerm is independent and has no
tie to them. This repository ships no game files. You use your own legal
client folder when you need the map, the art or the walk data. Use UOTerm on
shards you run: your own servers, demo servers and offline worlds. The terms
of an official server may forbid other clients. See [LEGAL.md](LEGAL.md).

## What it is not

- Not a copy of the Classic Client's code.
- Not a click-macro overlay.
- Not a farm bot for official servers.
- Not a cheat tool for EA or Broadsword shards.
- Not a live shard. `uoterm mock-shard` is a small demo for tests.

## The two clients

The **native play window** is a Rust window drawn with egui. `uoterm play`
opens it with the login screens. `uoterm watch` opens it on a session that
already runs. `uoterm connect --view` opens it in the same process as the
session. It has two looks, Modern and Classic.

The **web client** is a page in the browser. `uoterm web` serves it.
Three.js draws the world, and the page has the Modern look only. The page
source is in `web/`.

Both clients get their UI rules from one crate, `crates/uoterm-view`. The web
client runs it as WebAssembly, built by `crates/uoterm-web`.

Use the native window on the machine that runs the session, when you want
the Classic look, or when you want a `--snapshot` picture. Use the web
client to play from a browser, for example from a phone or a laptop on your
home network.

## Shard support

UOTerm uses the packet layouts and the login of the Classic Client. It treats
every shard the same way, whatever its name, so any server that accepts a
Classic Client should work. Still, treat a feature as unproven until a test
in this tree proves it.

[docs/PROTOCOL.md](docs/PROTOCOL.md) explains the eras, the versions and the
encryption.

## What you need

- Rust 1.87 or later (stable). `rust-toolchain.toml` pins `stable`.
- Linux or Windows. Nobody has tested macOS.
- On Linux, the ALSA package for sound: `sudo apt install libasound2-dev`
  (Debian, Ubuntu) or `alsa-lib-devel` (Fedora).
- On Linux, the udev package, which the game controller library (gilrs)
  needs to build: `sudo apt install libudev-dev` (Debian, Ubuntu) or
  `systemd-devel` (Fedora). The CI file `.github/workflows/ci.yml` lists
  every Linux package a bare Ubuntu image needs.
- The password of the account in an environment variable. The default name
  is `UO_PASS`. Never put a password in git.

For the web client you also need:

- Node.js and npm.
- The WebAssembly target: `rustup target add wasm32-unknown-unknown`.
- `wasm-bindgen-cli` 0.2.100. It must be the same version the workspace
  pins for `wasm-bindgen`: `cargo install wasm-bindgen-cli --version 0.2.100`.

You may also want:

- A legal UO client folder (`--uopath`). It gives the map (`map0.mul` or
  UOP), the statics, `tiledata.mul`, the art and the sounds.
- `UOTERM_API_TOKEN`, when the HTTP API must ask for a token, or when it
  binds an address that is not loopback.

## Build

From the root of the repository:

```bash
cargo build -p uoterm
./target/debug/uoterm --help
```

The examples below write `uoterm`. On a fresh machine, use
`./target/debug/uoterm` or `cargo run -p uoterm -- <command>`.

To build the page of the web client:

```bash
cd web && npm ci && npm run build
```

`npm run build` does four things:

1. `npm run wasm` builds `uoterm-web` for `wasm32-unknown-unknown`, runs
   `wasm-bindgen`, and copies the fonts of the Rust window.
2. `npm run theme` writes `web/src/theme.css` from `uoterm_view::ui::theme`
   (it runs `uoterm theme-css`).
3. `tsc --noEmit` checks the types.
4. `vite build` writes the page to `web/dist`.

The WebAssembly output, the fonts, `theme.css` and `dist` are build output.
Git does not track them.

## Quick start: the mock shard

The mock shard is a small fake world for tests. Use it only when no real
shard uses its port. You need two terminals. The mock takes any account and
password, and its character is `Mara`.

1. Start the mock shard:

   ```bash
   uoterm mock-shard --bind 127.0.0.1:2593
   ```

2. In the second terminal, log in:

   ```bash
   export UO_PASS=test
   uoterm connect --host 127.0.0.1 --port 2593 --account test --character Mara
   ```

   You should see: `session s1 started; api 127.0.0.1:7733; encryption none`.
   Leave this process running.

3. In a third terminal, drive the character:

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

   `look` prints a 21 by 21 radar in text (`@` is you, `i` is an item on the
   ground).

4. Or call the HTTP API:

   ```bash
   curl http://127.0.0.1:7733/health
   curl http://127.0.0.1:7733/v1/sessions
   curl http://127.0.0.1:7733/v1/sessions/s1/state
   curl -X POST http://127.0.0.1:7733/v1/sessions/s1/tools/say \
     -H 'content-type: application/json' \
     -d '{"text":"vendor buy"}'
   ```

   If `UOTERM_API_TOKEN` is set, add
   `-H "Authorization: Bearer $UOTERM_API_TOKEN"` to each request but
   `/health`.

5. To see the character, run `uoterm watch` in another terminal.

Stop with Ctrl+C on `connect`, then on the mock shard.

## Quick start: your own shard

Your shard listens on its port, often `2593`. UOTerm only logs in. Do not
start `uoterm mock-shard` on the same port.

```bash
export UO_PASS=your_password
uoterm connect \
  --host 127.0.0.1 --port 2593 \
  --account your_account --character Mara \
  --era modern --encryption none \
  --uopath /path/to/uo \
  --persona personas/traveler.toml
```

You should see: `session s1 started; api 127.0.0.1:7733; encryption none`.
Leave this process running.

- `--host` takes an IP address or a DNS name, such as `192.168.1.10`.
- `--encryption none` is for most free shards. `--encryption osi` is the
  Classic Client encryption. This repository ships no keys of official
  servers.
- `--era modern` sends the 21-byte `0xEF` seed. `--era t2a` is for clients
  from 1.26 to 2.0.x, and sends a 4-byte seed.
- With no `--version`, a `modern` session sends the version of `client.exe`
  in `--uopath`. Many shards kick a client older than their own.
- `--uopath` is optional. If you give it and UOTerm cannot read the folder,
  `connect` fails with that error.
- After `0x8C` both eras open a new TCP connection to the game server. If the
  relay IP is `0.0.0.0`, the new connection goes to `--host`.
- `--view` opens the play window in the same process. `view = true` in
  `uoterm.toml` does the same on each `connect`. `--text-view` prints the
  radar in this terminal. You cannot use both.

To play by hand instead, run `uoterm play`. It opens the login screens.

## How the programs work together

`uoterm connect`, `uoterm populate` and `uoterm web` are servers. They stay
in the front and serve the HTTP API until you press Ctrl+C. `uoterm play`
serves the API too, after the first login.

`connect --view` opens the play window in the same process, so the window
reads the session with no HTTP step. Closing the window ends the program,
and the character leaves the world, as on Ctrl+C. The "Quit" button of the
window does the same: it asks you first, then asks the shard to log the
character out.

All other commands (`session`, `say`, `move`, `walk`, `open-door`, `look`,
`state`, `agent`, `watch`, `mcp`) are clients. They call the HTTP API, and
never open a second game socket. `harvest` only reads a local file.

Run one server on each API port. Do not start `mock-shard` on the port of a
live shard. If the API port of `connect`, `play` or `populate` is taken, the
log shows `api ended` and the program goes on with no API. Only `uoterm web`
stops with the error.

Global flags, for all commands:

| Flag | Environment | Purpose |
| --- | --- | --- |
| `--json` | | Print JSON on stdout. |
| `--api <URL>` | `UOTERM_API` | The HTTP address. Default: `api_bind` in `uoterm.toml`, else `http://127.0.0.1:7733`. |
| `--session <id>` | `UOTERM_SESSION` | The session id. Default: the first session on the API. |

Exit codes: `0` ok, `2` usage, `3` network, `4` protocol, `5` world or
precondition.

## HTTP API access

When the API binds a loopback address, it answers only callers on this
machine, and it needs no token. To bind any other address, set
`UOTERM_API_TOKEN`; without it the bind is refused. With a token set, each
caller must send it, as `Authorization: Bearer <token>` or in the
`uoterm_token` cookie. The client commands read the same variable and send
the header.

```bash
export UOTERM_API_TOKEN=replace-me
curl -H "Authorization: Bearer replace-me" http://127.0.0.1:7733/v1/sessions
```

Do not bind the API to a public address without a token.
[docs/AGENT_API.md](docs/AGENT_API.md#who-the-api-answers) has the full
rules: the token, the loopback rule, the origin rule and the cookie.

## Configuration

UOTerm reads `uoterm.toml` from the first of these places:

1. `./uoterm.toml` in the folder you run it from.
2. On Linux `~/.config/uoterm/uoterm.toml`. On Windows
   `%APPDATA%\uoterm\uoterm.toml`.

Copy `uoterm.toml.example` to `uoterm.toml` to start. The file must have
`host`, `port`, `era`, `log_level`, `api_bind` and `max_sessions`. **If one
of them is missing, or the file has an error, UOTerm skips the file without
a word** and uses the next place, or the defaults.

`connect` takes each key from the file when you do not give the flag:

| Key | Default | What it does |
| --- | --- | --- |
| `host`, `port` | `127.0.0.1`, `2593` | The login server. |
| `era` | `modern` | `t2a` or `modern`. |
| `api_bind` | `127.0.0.1:7733` | Where the HTTP API listens. |
| `max_sessions` | `32` | The most sessions one process runs. |
| `uopath` | none | The client folder. |
| `markers` | none | A marker file of named places to travel to: a UO Auto Map `.map` file or an Ultima Mapper `Waypoints.lua` file. None ships with UOTerm. |
| `obey_shard_rules` | `true` | Some shards send a list of assistant features they forbid, such as auto-open doors, auto-bandage and auto-potions. With `true`, the character does not use them on that shard. With `false`, it ignores the list. The client answers the shard in both cases. |
| `answer_when_named` | `true` | When another character says your character's name, the agent gets a `spoken_to` event, an `unanswered` list on every tool result until your character speaks, and a `spoken_to` list in `observe`. With `false`, the agent hears nothing. |
| `play_along` | `false` | With `answer_when_named` on, the agent may say yes to a player's plans: join their party, follow them, help them fight. With `false`, the agent answers in a few words and says no. |
| `view` | `false` | Open the play window on each `connect`, as `--view` does. |
| `reconnect` | `true` | When the link to the shard drops, log in again. The first try comes after 5 seconds, and each failed try doubles the wait, up to 60 seconds. A logout you or the agent ask for does not log in again. With `false`, the session ends. |
| `proxy` | none | Reach the shard through a proxy: `socks5://host:port` or `http://host:port` (HTTP CONNECT), with `user:password@` before the host when the proxy asks for a login. The login and the game link both go through it. The log shows the proxy without its password. |
| `log_level` | `info` | Must be in the file, but has no effect. Set the log level with `RUST_LOG`, such as `RUST_LOG=debug`. |

A session made with `POST /v1/sessions` takes `obey_shard_rules`,
`answer_when_named`, `play_along`, `reconnect` and `proxy` in its body too.

The program keeps its own files in the UOTerm config folder (Linux
`~/.config/uoterm`, Windows `%APPDATA%\uoterm`) and in its data folder (Linux
`~/.local/share/uoterm`). Tests never touch these folders.

### Saved logins

A saved login is a TOML file. It holds the account, and may hold the host
(an IP address or a DNS name), the port, the shard, the character, the
encryption (`none` or `osi`), the era, the client version and
`password_env`: the name of an environment variable that holds the password.
A saved login never holds the password.

- The login screen of `uoterm play` saves logins in the `logins` folder of
  the config folder, one `<name>.toml` each. In a file name, each mark a
  file name may not hold becomes `%` and two hex digits.
- Older saved logins in the `profiles` folder of the working folder are read
  but never written. When a name is in both folders, the config folder wins.
- `connect --profile`, `play --profile`, `populate` and the runtime tools
  (`profile`) find a saved login by its name. `connect` and `populate` also
  take a path with a folder, such as `profiles/cedric.toml`, and read that
  file as it is.
- A value on the command line wins over the saved login. The saved login
  wins over `uoterm.toml`.
- To make one by hand, copy `profiles/example.toml`. Git ignores other
  `profiles/*.toml` files.

### Personas

A persona file says who the character is and how it talks. Persona files
live in `personas/`. Attach one with `connect --persona`, or send one with
`agent run --persona`. See [docs/PERSONAS.md](docs/PERSONAS.md).

## The native play window

The window has two looks with the same features. Both read the same data
and send the same acts.

- **Modern** (the default) has glass panels over the world: a bar at the
  top, the character sheet, the hotbar, a journal with the chat line under
  it, and a ring of acts on a right click. You move a panel by its title,
  lock it, and it opens where you left it.
- **Classic** looks like the official client: gumps drawn from the gump art
  of your client files, its fonts and mouse pointers, a game window in a
  frame that you move and size, and the chat line at the foot of that
  window. Pick it on the Interface page of the Options ("UI style"). It
  needs the client files. Without them the window keeps the Modern look.

The login screens are one plain UOTerm form for both looks.

### Playing

You walk with the arrow keys (Shift runs), with the right mouse button held
(a left click while you hold it keeps you going), or with a game controller.
A double-click on the ground walks there when "Enable pathfinding" is on
(General page). "Use Shift for pathfinding" asks for Shift too. Alt+click on
the ground runs there. "Click on the ground runs there" and "Use W A S D to
walk" are on the same page.

A double-click uses a thing, or attacks it in war mode. One click looks at
it. There is a target cursor. You drag items between bags, onto your
character, onto other mobiles, onto the ground and into a trade. When you
drop a pile, the window asks how many to move; hold Shift to move the whole
pile. ("Hold Shift to split stacks" turns this round.) A right-click opens a
ring of acts with the shard's context menu (Modern), or the context menu
gump (Classic).

The window has tooltips, the paperdoll with worn items, the status, skills
with locks and groups, spellbooks, the party (with the hits, mana and
stamina of each member, when the shard tells them), health bars, a hotbar,
buff icons, shop and trade windows, gumps with text fields, old-style menus,
books, bulletin boards, prompts, speech over heads, damage numbers, spell
effects, night, rain and snow, houses and boats, and sound. The sounds and
the music come from your client files.

The world is drawn from your client files: the land, the items, and the
mobiles with their mounts and worn items, from the classic `anim*.mul` files
and the newer `AnimationFrame*.uop` packages. A mobile walks, runs, stands,
swings or casts as the shard says, and a corpse shows the fallen body. A
mobile with no picture in these files shows as a plain coloured figure. The
land and the trees change with the season the shard sets. Houses that
players designed show their own walls and floors. A building that waits for
its place shows where the mouse points. Without client files, `uoterm watch`
draws flat colours from the radar.

To zoom, scroll in the Modern look. In the Classic look, turn on "Enable
mouse wheel zoom (Ctrl + Scroll)" (Video page), then use Ctrl and the wheel.

While the agent has the character, the bar at the top has Take control,
Sheet, Map, Macros, Options and Quit. While you have control, it has Bag,
Sheet, Map, Macros, Profile, Chat, Help, War or Peace, Stop, Give back,
Options and Quit. Sheet shows worn items, skills with locks, spells and the
party. The hotbar takes
a dragged item, a pinned skill or spell, a pinned command, a macro, a weapon
move or a racial ability. Click an empty slot to pick one, and right-click a
slot to clear it. The keys 1 to 0 use its slots. Each character's hotbar is
kept in `watch-hotbar.toml`.

### Maps, chat and the house designer

The map of the land walks you where you click it. It has a box to go to a
place, sextant coordinates, and markers (Ctrl+click adds one). A map item,
such as a treasure map, shows its own land with its pins, and a click puts a
pin. An arrow points at a place the shard names. Marks the shard puts on the
map show with their names. The shard's notices and web links go in the
journal. UOTerm never opens a link by itself. The shard's chat shows its
channels, its lines and a box to talk in.

When the shard opens the house designer, both looks show the parts of the
client catalog. Pick a style and a piece, and click the house to build,
erase a part, or pick a part off the house with the eyedropper. A button for
each storey changes how it shows while you design (walls or floor
see-through or hidden, or all hidden). The designer counts the components,
the fixtures (doors and teleporters) and the cost against what the plot
allows. Backup, Restore, Sync, Clear, Commit, Revert and Leave do what they
say.

### Agent control and the chat line

The window only looks until you press "Take control". That stops the agent
and lets you play. "Give back", or 90 seconds with no act, returns the
character to the agent. The window is not a second login.

The chat line says words. In Do mode it runs one script command. When the
shard asks for words, the line answers it.

"Macros" opens the macro editor. Pick a script of the scripts folder, change
its lines, run it once or in a loop, save it, record what you do as a new
macro, or pin it to the hotbar.

### Plain-word fields (TypeSafe)

With a TypeSafe key, some fields take plain words, and TypeSafe's Jev model
picks from a list. Put `TYPESAFE_API_KEY` in the environment, or in a `.env`
file in the folder you run UOTerm from. The words, and the names of the
list, go to `api.typesafe.ai`.

- Order mode of the chat line takes an order such as `attack the orc`, and
  Jev picks the act and the target. The order, the names of the things near,
  your character's name and war mode go to TypeSafe.
- On the login screens, `my miner on the test shard` picks the saved login,
  and then the shard. Jev sees the names of the saved logins (by default
  `account@host`), their characters, shards and `host:port`. It never sees
  the password.
- On a map item, `the bank in britain` picks a place from the named places
  of your marker file.
- In the shard's chat, `the trade one` joins a channel.
- In the house designer, `a stone wall` picks a part from the catalog.
- On the character sheet, plain words pick what to wear or take off.
- In the macro editor, `heal myself with a bandage` picks the hotkey that
  does it, and adds the script lines of that hotkey to the end of the
  macro.

The web client has the same fields. It asks Jev through `uoterm web`, so the
key never goes to the browser.

### Login screens of `uoterm play`

The form has host, port, account, password, shard, character and
encryption, and the list of saved logins (see [Saved logins](#saved-logins)),
each with its name and `account @ host:port`. Click a saved login to fill the
form. The cursor goes to the password field. "Save login" asks for a name
(default `account@host`) and, if you want, the name of a variable that holds
the password. "Edit" puts a saved login in the form, so you can change it
and save it over. "Delete" asks Yes or No first. You can edit a login of the
older `profiles` folder (that saves a copy in the config folder), but not
delete it.

You type the password in a field that hides it. It stays in memory for the
login and is written nowhere. When the field is empty, the password comes
from the environment variable of the saved login. `--encryption` wins over
the encryption of the saved logins.

A shard list with more than one shard shows as a list to click. The
character list shows the slots of the account, with "(empty)" for a free
slot. Click a character to play it. Press Delete, then Delete? again, to
remove one. "New character" opens the creation screens: the look, the
skills, the start town and the name, with Back and Next. "Leave" goes back.
When the shard refuses, the screen says why. After the login the same
window is the game window, you have control, and the HTTP API runs as with
`connect`.

`uoterm play --profile NAME --go` fills the form from a saved login and logs
in at once. `--profile` takes the name of a saved login; a path with a
folder works only when its file name is a saved login.

### Options and profiles

The Options have the pages of the classic client: General, Sound, Video,
Macros, Tooltip, Fonts, Speech, Combat & Spells, Counters, Info Bar,
Containers, Experimental, Ignore List, Interface, Nameplates, Journal, World
Map and Agents. Apply and Okay keep a change, Cancel drops it, and Default resets
one page. "Save as default" makes these options the start of each new
character.

The options, the places of the windows and the gumps you keep open are a
profile for each character of each shard. They live in the `profiles` folder
of the config folder. `default.toml` is where a new character starts. Each
character has `<server>/<character>.toml`, where `<server>` is the login
server as `host:port` in small letters, with marks such as `:` written as
`%` and two hex digits (for example `127.0.0.1%3A2593/Mara.toml`).

### Keys, macros and the controller

The play window reads the keys as the official client does.

The chat line is under the journal in the Modern look. In the Classic look it
is at the foot of the game window, on a dark band ("Hide chat gradient"
removes the band). The shard's own words, and the party, guild and alliance
lines, show over it for ten seconds. Both looks use the same line, with the
same history.

With "Activate chat when pressing Enter" off (the default, Speech page), the
chat line always takes the keys you type. With it on, Enter opens it, Enter
sends and closes it, and Shift+Enter sends and keeps it open ("Use
Shift+Enter to send without closing chat"). The prefix keys also open it
when "Speech prefix keys open the chat (! ; : / \ , . [ | -)" is on.

A line that starts with `! ` is yelled, `; ` whispered, `: ` an emote, `/`
goes to the party (`/2 ` to its second member), `\` to the guild, `|` to the
alliance and `,` to the global chat. Each one goes out in the colour the
Speech page gives it. `/add`, `/rem`, `/loot`, `/accept`, `/decline` and
`/quit` are party orders. One that cannot be done now prints the classic
client's answer in the journal, such as "You are not in a party.". Words to
a party you are not in come back as "Note to self". Ctrl+Q and Ctrl+W bring
back the lines you sent.

A click on the ground with Alt held runs there by pathfinding ("Click with
this key held runs there", General page: None, Ctrl, Shift or Alt). With
"Click on the ground runs there" on, a plain click runs there too. The
"Click-to-run on / off" action (Macros page) switches it from a key, and the
journal says if it is on or off.

Tab holds war mode while it is down. When "Hold Tab for combat" is off
(Combat & Spells page), each press switches it.

The default keys of the official client are Alt+P paperdoll, Alt+O options,
Alt+J journal, Alt+I backpack, Alt+R minimap, Ctrl+B bow and Ctrl+S salute.
The Experimental page can turn off the default keys (and the default
controller buttons), the arrow keys, Tab, Ctrl+Q and Ctrl+W, and the left
click that keeps you walking.

A macro (Options, Macros page) has a name, a key chord or controller
buttons, and steps. Each step is an action with its argument: every macro
type of the reference client (say, walk, open or close any window, cast, use
skill, zoom, toggles for roofs, trees, vegetation, caves, the circle of
transparency, names and auras, select next, previous or nearest, grab, the
view range, delays and "wait for target"), any hotkey of the session, a
saved script, a script line, a screenshot, and mouse clicks for a
controller. Click the key field and press the chord to bind it. While you
type in a field of a panel or a gump, only chords with Ctrl or Alt, and the
F keys, run macros.

Screenshots (a macro step, or "Take a screenshot on death" on the Interface
page) go to the `screenshots` folder of the config folder. The journal says
where, unless "Hide "Screenshot stored in" message" is on.

An attack, or a harmful target, on an innocent asks "This may flag you
criminal!" first ("Query before attack"). A helpful target on a criminal, a
murderer or a gray asks too, when "Query before beneficial acts on
murderers, criminals and grays" is on (Combat & Spells page).

With "Use a game controller" on (Macros page), the left stick walks (pushed
far, it runs), the right stick moves the mouse at the speed set on the page,
and buttons run macros. With the default buttons, South is a left click,
East a right click, West a double click, North war or peace, and Start the
options.

## The web client

Build the page (see [Build](#build)), then start the server:

```bash
uoterm web --uopath /path/to/uo-client-files
```

With the default bind, it prints `Open http://127.0.0.1:7733/`. Open that
address, log in on the page, and pick a character. The page needs the client
files for the world art, the map and the sounds. The sessions the page
starts use the same `uopath`.

`uoterm web` serves the HTTP API, the routes for the client files, and the
page, all from one port. It runs until Ctrl+C. The access rules of
[HTTP API access](#http-api-access) apply to all of it. The page itself
loads with no token. [docs/AGENT_API.md](docs/AGENT_API.md#web-client-routes)
lists the routes.

The page uses the same files of the config folder as the native window: the
saved logins (it can list and save them, not delete them), the profiles and
the hotbar. So the options are the same in both clients.

To play from another machine on your home network, bind to all addresses
and set a token. A bind that is not loopback is refused without
`UOTERM_API_TOKEN`.

```bash
UOTERM_API_TOKEN=replace-me uoterm web --bind 0.0.0.0:7733
```

Open `http://<this machine>:7733/` on the phone or the laptop, and enter the
token once. The page trades it for a cookie. Use plain HTTP only on a home
network you trust: the token and the login password cross the network
without encryption. Do not open the port to the internet.

To work on the page, run `cd web && npm run dev`. It builds the WebAssembly
and the theme, then starts the Vite server. Vite sends `/v1` and `/health` to
`uoterm web` on `127.0.0.1:7733`, so start `uoterm web` first.

A UI rule goes in `crates/uoterm-view`, never in a window. The play window
and the page both use that crate. It has no egui, no Three.js, no tokio and
no file access. The page uses the same theme tokens as the Modern look: the
colours and sizes are in `uoterm_view::ui::theme`, and `npm run theme` writes
them to `web/src/theme.css`. Do not edit that file by hand.

## Command reference

| Command | Kind | Notes |
| --- | --- | --- |
| `uoterm mock-shard [--bind HOST:PORT]` | Demo | A demo shard with no encryption. Default `127.0.0.1:2593`. Do not run it while a live shard uses that port. |
| `uoterm connect ...` | Server | Logs in and serves the HTTP API until Ctrl+C. See the flags below. |
| `uoterm populate --manifest PATH [--api-bind ADDR]` | Server | Starts many sessions, then the HTTP API. See [Running many characters](#running-many-characters). |
| `uoterm play [--profile NAME] [--go] [--encryption none\|osi] [--uopath DIR] [--api-bind ADDR]` | Server | Play by hand in the native window: the login screens, then the game window with control taken. `--profile` fills the form at the start. `--go` logs in at once with `--profile`. |
| `uoterm web [--bind HOST:PORT] [--web-dir DIR] [--uopath DIR]` | Server | The web client: the HTTP API, the client file routes and the page. See the flags below. |
| `uoterm watch [--text] [--uopath DIR] [--open PANEL] [--snapshot FILE.png]` | Client | The native window on the running session. `--uopath` (or `uopath` in `uoterm.toml`) draws the real map. `--open` opens a panel at the start: `sheet`, `map`, `macros`, `profile` or `chat`. Give it once for each panel. `--snapshot` saves one PNG picture and closes. `--text` prints the radar in the terminal. |
| `uoterm session list` | Client | The session ids on the API. |
| `uoterm session attach <id>` | Client | Prints the state of one id. |
| `uoterm say "text"` | Client | The `say` tool. The persona refuses `*emotes*`. |
| `uoterm move --to x,y,z` | Client | The `move_to` tool. |
| `uoterm walk --dir DIR [--run] [--hold-ms N]` | Client | The `walk` tool. One step, or a held walk of `0x02` steps. |
| `uoterm open-door` | Client | The `open_door` tool (`0x12`/`0x58`). |
| `uoterm look` | Client | The radar. `--json` prints the full observe JSON. |
| `uoterm state` | Client | YAML. `--json` for JSON. The field of the character is `self_state`. |
| `uoterm agent run --persona FILE [--goal NAME]` | Client | `set_persona`, then `set_goal`. The goal comes from the persona's class unless you give `--goal`. |
| `uoterm agent stop` | Client | `cancel_goal`. |
| `uoterm harvest log [--since 1h] [--jsonl]` | Local | Reads the `.jsonl` event logs of the data folder (Linux `~/.local/share/uoterm/harvest.jsonl` and the others there). |
| `uoterm mcp` | Client | The MCP server on stdio. It sends each call to `--api`. |
| `uoterm theme-css` | Local | Prints the colours and sizes of the Modern theme as CSS. `npm run theme` uses it. |

### `connect` flags

| Flag | Default | Meaning |
| --- | --- | --- |
| `--host` | from `--profile`, else `uoterm.toml`, else `127.0.0.1` | The login host (an IP address or a DNS name). |
| `--port` | from `--profile`, else `uoterm.toml`, else `2593` | The login port. |
| `--account` | needed unless `--profile` | The account name. |
| `--password-env` | the one of `--profile`, else `UO_PASS` | The environment variable that holds the password. |
| `--character` | needed unless `--profile` | The character name on the account. |
| `--shard` | the one of `--profile`, else none | Picks a shard by name when the server list has more than one. |
| `--version` | the one of `--profile`, else the version of `client.exe` in `--uopath` (modern era only), else the default of the era (`7.0.102.3` for `modern`, `2.0.7.0` for `t2a`) | The client version string (`0xBD`). Shards that check versions kick a client older than their own `client.exe`. |
| `--era` | the one of `--profile`, else `uoterm.toml`, else `modern` | `t2a` or `modern`. |
| `--encryption` | the one of `--profile`, else `none` | `none` = no encryption. `osi` = the Classic Client encryption. |
| `--uopath` | from `uoterm.toml` | The client folder. Without it, routes use an open mock grid. |
| `--markers` | from `uoterm.toml` | A marker file of named places: a UO Auto Map `.map` file or an Ultima Mapper `Waypoints.lua` file. |
| `--profile` | none | A saved login, by name or by file. Its host, port, account, character, shard, encryption, era, version and password variable fill each flag you do not give. |
| `--persona` | the built-in lumberjack | The persona TOML for speech and reflexes. |
| `--api-bind` | from `uoterm.toml`, else `127.0.0.1:7733` | Where the HTTP API listens. |
| `--view` | off | Opens the native play window in the same process. Closing the window ends the program, as Ctrl+C does. |
| `--text-view` | off | Prints a live radar in this terminal. You cannot use it with `--view`. |

### `web` flags

| Flag | Default | Meaning |
| --- | --- | --- |
| `--bind` | `api_bind` in `uoterm.toml`, else `127.0.0.1:7733` | Where it listens. |
| `--web-dir` | `web/dist` | The folder of the built page. |
| `--uopath` | `uopath` in `uoterm.toml` | The client files. |

## Agents over MCP

Start `connect` (or `populate`, `play` or `web`) first. Then point the model
host at `uoterm mcp`:

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

The model gets every session tool, and five runtime tools that need no
session: `connect`, `disconnect`, `characters`, `character_create` and
`character_delete`. With them an agent can list the characters of an
account, make or delete one, and log in. The password stays in an
environment variable (`password_env`) or comes from a saved login
(`profile`), never in a call. The playbooks in `docs/playbooks/` come as MCP
resources.

Have the model read the `driver` playbook first, then `hunt` or `walk`.
[docs/AGENT_API.md](docs/AGENT_API.md) has every tool, event and route.

## Scripts, agents and hotkeys

UOTerm has an assistant built in. A script is a plain list of commands, one
on each line, in the style UO assistant scripts have long used. Run it with
the `run_script` tool, or from the macro editor of a play window. Several
scripts run side by side. Agents loot, pick up, organize, restock, dress,
buy, sell, bandage and mount again on their own. Hotkeys are named actions
such as `Bandage Self` or `Cast Greater Heal`. `record_macro` writes what
you do as a script. All of it goes at the pace a person plays, and obeys the
features the shard forbids unless `obey_shard_rules = false`.

See [docs/SCRIPTS.md](docs/SCRIPTS.md) and [docs/AGENTS.md](docs/AGENTS.md).

## Running many characters

```bash
export UO_PASS=test
uoterm populate --manifest shards/britannia.toml
```

A manifest is a TOML file:

```toml
host = "127.0.0.1"
port = 2593
era = "modern"            # optional; default modern
# uopath = "/path/to/uo"  # optional
# markers = "/path/to/Waypoints.lua"  # optional

[[agents]]
profile = "profiles/example.toml"
persona = "personas/lumberjack.toml"
```

Each `[[agents]]` row needs a saved login (by name or by file) and a
persona. Every agent uses the host, the port, the era, `uopath` and
`markers` of the manifest. The encryption, the client version and the
password variable come from the saved login, else `none`, the version of the
era, and `UO_PASS`. Each session starts with the goal of its persona's
class. The settings `obey_shard_rules`, `answer_when_named`, `play_along`
and `reconnect` keep their defaults. Agents whose `active_hours` miss the
local time are skipped.

## Tests

The Rust tests need no client files:

```bash
cargo test --workspace
```

They cover mock logins, walking, speech and a gather loop, among many other
things. A test never reads or writes your own UOTerm folders. Only the
`uoterm` program uses `~/.config/uoterm` and `~/.local/share/uoterm`. Any
other process, such as a test, keeps its files in a folder of its own under
the temp folder.

Some tests compare the map reader with real Ultima Online files. They skip
themselves when there are none. Point `UOTERM_TEST_UOPATH` at a client
folder to run them:

```bash
UOTERM_TEST_UOPATH=/path/to/uo cargo test --workspace
```

The crates the web client runs must build for the browser target. A crate
that reaches a file, a thread or the wall clock fails here:

```bash
scripts/check-wasm.sh uoterm-view uoterm-web
```

The page has its own tests and lint:

```bash
cd web && npm run test && npm run lint
```

CI (`.github/workflows/ci.yml`) runs on Ubuntu and Windows: `cargo fmt`,
`cargo clippy --workspace --all-targets`, `cargo test --workspace`, and
`uoterm --help`. It also rejects committed `.mul`, `.uop`, `.idx` and `.def`
files, and any shipped file that names another client or assistant tool.
CI does not build the web page.

Some docs are checked by tests: `docs/SCRIPTS.md` must list every script
command, and the TOML example in `docs/AGENTS.md` must read as real
settings. The playbooks are built into the program as MCP resources.

## Troubleshooting

| What you see | Why | What to do |
| --- | --- | --- |
| `usage: password env UO_PASS is not set` | The variable is missing. | `export UO_PASS=...` in the same shell as `connect`. |
| `world: no active session; run uoterm connect first` | The API is down, or `--api` is wrong. | Start `connect`. Check `curl http://127.0.0.1:7733/health`. |
| `http 401` with `unauthorized` | A token is set, and the header or the cookie is missing. | Export `UOTERM_API_TOKEN` in the client shell, or enter the token on the page. |
| `403` `this API answers callers on this machine only` | A loopback bind, and `Host` is not this machine. | Use `127.0.0.1` or `localhost`, or bind another address with a token. |
| `403` `this API answers only the pages it serves` | The `Origin` is not the API itself. | Open the page from the address of `uoterm web`. |
| `a non-loopback bind requires UOTERM_API_TOKEN` | A public bind with no token. | Set the token, or bind `127.0.0.1`. |
| `api ended` in the log, with `Address already in use` | A second server on the same API port. | Run one server process, or give another `--api-bind`. |
| `Address already in use` on `2593` | `mock-shard` and a live shard at once. | Stop `mock-shard`. Start only the live shard. |
| `(no sessions)` | The client talks to an empty API. | Use the same host and port as `--api-bind`. |
| `(no harvest lines)` | No events yet, or the wrong data folder. | Check that `connect` runs. On Linux the folder is `~/.local/share/uoterm`. |
| `uoterm.toml` seems ignored | A needed key is missing, or the file has an error. | Make sure it has `host`, `port`, `era`, `log_level`, `api_bind` and `max_sessions`. Start from `uoterm.toml.example`. |
| `populate` starts 0 sessions | `active_hours` miss the local time. | Widen the hours, or run inside them. |
| Speech is refused | `*emotes*`, empty text, more than 128 characters, or a line said lately. | Send a short new line without stars. |
| The shard ignores you | The era, the seed or the encryption does not match. | Try another `--era` or `--encryption`. Test against the mock first. |
| The shard drops the link every few seconds | It kicks a client version older than its own. | Give `--uopath` so UOTerm sends the version of `client.exe`, or set `--version`. |

## Repository layout

- `crates/uoterm-protocol`: framing, Huffman, encryption, packets.
- `crates/uoterm-world`: the world state, serials, journal, radar, events.
- `crates/uoterm-nav`: MUL and UOP files, routes (A*), line of sight, skill
  names.
- `crates/uoterm-assist`: spells, weapon moves, potions and other game data.
- `crates/uoterm-script`: the script language, its parser and its runner.
- `crates/uoterm-runtime`: the session, the mock shard, reflexes, scripts,
  agents, hotkeys, the HTTP API.
- `crates/uoterm-view`: the UI rules both clients use (no egui; builds for
  WebAssembly).
- `crates/uoterm-web`: the WebAssembly wrapper of `uoterm-view` for the
  browser.
- `crates/uoterm`: the `uoterm` program, the native play window, the
  `uoterm web` routes and the MCP server.
- `web/`: the page of the web client (Three.js, Preact, Vite).
- `personas/`, `profiles/`, `shards/`: example personas, saved logins and a
  populate manifest.

Docs:

- [docs/AGENT_API.md](docs/AGENT_API.md): tools, events, HTTP, MCP.
- [docs/SCRIPTS.md](docs/SCRIPTS.md): the script language.
- [docs/AGENTS.md](docs/AGENTS.md): agents, hotkeys, recording.
- [docs/PERSONAS.md](docs/PERSONAS.md): persona files.
- [docs/PROTOCOL.md](docs/PROTOCOL.md): the wire protocol.
- [docs/playbooks/](docs/playbooks/): short guides for an agent (driver,
  login, hunt, walk, navigation, loot, bank, death, moongate, dungeon,
  mounts, runebook, buy, sell, containers, talk, inspect, equip).
- [LEGAL.md](LEGAL.md).

## License

GNU Affero General Public License, version 3 only. See `LICENSE` and
[LEGAL.md](LEGAL.md).
