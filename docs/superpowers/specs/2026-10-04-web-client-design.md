# Web client design (Three.js)

Date: 2026-10-04
Status: approved in chat, waiting for review of this file

## Goal

A browser version of the UOTerm play window. It looks and works like the
Modern style of the Rust window (`crates/uoterm/src/window/`). Three.js draws
the world. It lives in this repository.

### Decisions made in chat

| Question | Answer |
|---|---|
| Same repository or a new one | Same repository, folder `web/` |
| 3D world or the same 2D look | The same 2D look, drawn by Three.js. A real 3D world is not possible: the client files have flat pictures for walls, items and mobiles, from one angle only. Only the ground has height data. |
| Who uses it | Only the owner: on the same PC, or on a phone or laptop on the home network. No public hosting, no user accounts. |
| Where the logic runs | Rust compiled to WebAssembly runs in the browser. JS and Three.js only draw and read input. |
| Which looks | Modern only. Classic stays in the Rust window. |

### Success criteria

1. `uoterm web` serves the page. The owner logs in, picks a character, and
   plays a full session in the browser on the mock shard and on a real shard.
2. Every Modern feature listed under "Scope" works in the browser.
3. No rule exists twice. A UI rule lives in `uoterm-view` and both windows
   use it.
4. The Rust window works the same as before the split. All present tests pass.
5. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace`, the WebAssembly build, `npm run lint`,
   `npm run test` and `npm run build` all pass.

## Architecture

```
Browser page (web/)                    UOTerm (Rust)
+----------------------------+         +--------------------------+
| Three.js: the world        |         | session: shard link      |
| HTML/CSS: Modern panels    |<-state--| watch frames (push)      |
| WebAssembly (uoterm-web):  |--tools->| tool calls (exists)      |
|   UI rules, walk predict,  |<-art----| art as PNG               |
|   draw order, lights       |<-map----| land + statics blocks    |
+----------------------------+         | settings, screenshots    |
                                       | serves web/dist          |
                                       +--------------------------+
```

### New crate `crates/uoterm-view`

The UI rules both windows use. It has no egui, no Three.js, no tokio and no
file access. It must build for `wasm32-unknown-unknown`.

It may depend on `uoterm-protocol`, `uoterm-world`, `uoterm-nav` and
`uoterm-assist` only.

What moves into it from `crates/uoterm/src/`:

- `view.rs`: the watch frame types (`WatchFrame`, `WatchItem`,
  `WatchMobile`, ...) and their parsing.
- `window/model/`: all of it. The 3 files that use egui today lose their
  egui parts; the drawing stays in the window.
- The rule half of `window/scene.rs`: smooth walking (`Glide`, `Stride`,
  `Waiting`), draw order, lights (`window/lights.rs` rules), roofs and
  ceilings, house design pieces, mouse picking, and the land normals. The
  result is a draw list: sprites with art key, screen place, depth, hue,
  light and alpha; land quads with corners and texture; text plates.
  `window/scene.rs` then only paints that draw list with egui.
- `window/predict.rs` (walk prediction).
- `window/settings/` types and rules (`Profile` and the option pages). The
  file store (`settings/store.rs`) stays native.
- `window/keys.rs`, `window/keys/`, `window/actions/` rules, `window/steer.rs`
  rules and the controller rules of `window/pad.rs`. Each takes plain input
  events. The egui, `gilrs` and file parts stay in the window.
- `window/look.rs`, `window/filters.rs`, `window/figure.rs` rules and the
  sprite keys of `window/atlas.rs` (`ArtKey`). Texture upload stays native.

What moves down out of `uoterm-runtime`, because the view uses it and
`uoterm-runtime` needs tokio:

- `CharacterChoices`, `NewCharacterWish` (from `config.rs`).
- The tool name constants (`TOOL_WATCH`, `TOOL_PROPERTIES`, ...).
- `Landmarks` data and parsing (from `landmarks.rs`; file reads stay in the
  runtime).

These go to `uoterm-world`. `uoterm-runtime` uses them from there. No copy
stays behind.

Also moving in: the pure part of `window/control.rs` (`Act`, `Act::calls`,
`Act::words`, `DropTo`, `Ask`, `Answer`, `Report`, `Tip`) and
`window/actions/guard.rs` (`Guard`); `Hand` with its threads stays native.
The rule parts of the Modern panel files (`control_ui::act_for_click`,
`ring_ui` lines, `deck_ui::Slot::press`, gump tick and radio rules, grid
click rules, `modern/layout.rs`, panel `Launch` flags, `hud` states) move
too, so the browser panels hold no rule of their own.

Geometry and input: rules that use egui `Pos2`, `Vec2`, `Rect`, `Color32`,
`Modifiers` and `Key` get plain types of their own in `uoterm-view`
(`geom.rs`, `input.rs`). Key names stay the egui names that saved profiles
already use (`F1`, `A`, `ArrowUp`, ...). The Rust window converts at its
edge (`window/bridge.rs`).

Time: `std::time::Instant` panics on `wasm32-unknown-unknown`. Only
`WatchStride.slot` uses it. `uoterm-view` takes the clock from the caller as
seconds (`f64`), the same way the rules already take egui's frame time. No
`Instant` is in `uoterm-view`.

Art the scene needs while it builds (sprite sizes, tile data, map cells,
multi pieces, animation rules) comes through a trait, `WorldArt`. The Rust
window implements it with `ClientArt` and the atlas. The browser implements
it with data it fetched; a picture still on its way is `Pending` and is not
drawn in that frame. The atlas shelf packing moves to `uoterm-view`, so both
windows place pictures the same way.

### New crate `crates/uoterm-web`

A thin `wasm-bindgen` wrapper over `uoterm-view`. It exposes:

- `WebView::new(profile_json)`, `frame(watch_json, now)`,
  `tick(now, width, height) -> DrawBuffers`, `panels() -> PanelData`.
- `input(event_json) -> Vec<OutCall>` for keys, mouse, wheel, touch and
  controller. An `OutCall` is a tool call from `Act::calls`, a Jev request,
  or a kept-file save.
- `art_wanted()` and `art_arrived(request, width, height, anchor, rgba)` for
  the picture flow, and `data_arrived(table, json)` for the tables.
- MIDI to PCM with `rustysynth` (the same synth as the Rust window).

The draw buffers cross to JS as typed arrays (vertex positions, UVs, colors,
indices, atlas uploads, light cells). Panel data crosses as JSON.

### UOTerm API additions

All new routes sit behind the same `require_bearer` layer as the present
routes. The loopback rule and the token rule do not change.

Session and login routes go in `crates/uoterm-runtime/src/api.rs`. Routes
that need the client files or the window settings go in a new module
`crates/uoterm/src/web/`, and `uoterm web` merges both routers.

| Route | What it does |
|---|---|
| `GET /v1/sessions/{id}/live` | WebSocket. A session has no change signal today, so the link calls the `watch` tool every 33 ms (the pace of the Rust window in one program) and pushes the frame only when it differs from the last one sent. The page sends tool calls on it and gets each answer with its call id. |
| `POST /v1/art` | One picture, from a JSON `ArtRequest` (the serializable form of `ArtKey`: land, texture, item, gump, cursor, text with its `TextLook`, figure with its `WatchLook`, `Pose` and `Paint`). The answer is a PNG plus the anchor in a header. Hues and figure layers are applied on the CPU, as the Rust window does today (`ArtPixels::rgba`, `figure::compose`), so the browser needs no hue shader. |
| `GET /v1/map/{facet}/{bx}/{by}` | One 8x8 block of `Cell`s (land id, corners, statics, stretch, average z, wet), with map diffs and live map data applied. |
| `GET /v1/data/{table}` | Tables the scene reads while it builds: `tiledata`, `multis/{id}`, `animdata`, `anim-rules`, `radarcol`, `seasons`, `lights`, `cliloc`. |
| `GET /v1/sound/{id}`, `/v1/music/{id}`, `/v1/soundfont` | Sound as WAV, music track as MP3 or MIDI bytes, the MIDI sound font. |
| `GET/PUT /v1/profiles/{shard}/{character}` and `/v1/profiles/default` | The settings profile, the same TOML files as the Rust window, as JSON. |
| `GET/PUT /v1/kept/{name}` | The other kept files: `watch-hotbar.toml`, `watch-grab-bags.toml`. |
| `GET /v1/fonts`, `/v1/fonts/{name}` | The player fonts from the config `Fonts` folder. |
| `POST /v1/screenshots` | Saves a PNG to the `screenshots` folder. |
| `POST /v1/sessions/{id}/jev/{kind}` | Jev orders, picks and macro lines (`orders::ask`, `pick`, `lines_for`). They run in UOTerm, so the TypeSafe key never goes to the browser. |
| `GET /` and assets | The built page from the web folder. |

Art and data routes send `Cache-Control: max-age` with an `ETag` from the
client file version, so the browser asks for each one time.

`create_session` today gives HTTP sessions no client files (`uopath: None`).
Sessions started for the browser use `uopath` from `uoterm.toml`, the same as
`uoterm play`.

Login: `uoterm play` answers the login with a `LoginPicker`, an in-process
channel of `LoginQuestion`s (`crates/uoterm-runtime/src/config.rs`). The
browser gets the same questions over a WebSocket:

| Route | What it does |
|---|---|
| `GET /v1/login/live` | WebSocket. The page sends the login (saved login name, host, port, account, password). UOTerm starts the session with a `LoginPicker` whose questions it sends to the page as JSON (`Shard`, `Characters`, `Character`). The page answers each with the pick or a `CharacterRequest` (`Play`, `Delete`, `Make`, `Leave`). At the end UOTerm sends the session id. |

`LoginQuestion` keeps its `oneshot` reply senders. The serde form for the
wire is a separate data type with the same fields and no senders, in
`uoterm-world` next to `CharacterChoices` and `NewCharacterWish`
(`CharacterRequest` moves there too). The password goes to UOTerm one time
and is never stored, the same rule as now.

Token for the home network: when `--bind` is not loopback,
`UOTERM_API_TOKEN` must be set (present rule). The page asks for the token
one time and `POST /v1/web/token` sets it as an `HttpOnly`, `SameSite=Strict`
cookie. `require_bearer` accepts the cookie as well as the header, so the
WebSocket works without custom headers.

### New command `uoterm web`

`uoterm web [--bind ADDR] [--web-dir DIR]`. It starts the API and serves the
page. `--bind` defaults to `api_bind` from `uoterm.toml`. `--web-dir`
defaults to `web/dist`. The Rust build does not need Node.

### `web/` (TypeScript, Vite, Three.js)

- `web/src/world/`: an orthographic Three.js camera in screen pixels. The
  scene is one `BufferGeometry` built each frame from the draw buffers, in
  the paint order `uoterm-view` gives (the Rust window has no depth buffer
  either). One 4096x4096 atlas `DataTexture` gets the sub-image uploads the
  draw buffers list. The light map is a second texture drawn over the world.
  Vertex colors carry alpha, land light and shadow, as in the Rust window.
- `web/src/panels/`: the Modern panels in HTML and CSS over the canvas,
  written with Preact (a 3 kB UI library), with the Modern theme colors and
  the Barlow fonts. Each panel reads its data from `WebView::panels()`; it
  holds no rule of its own. Name plates and shard gumps are HTML too.
- `web/src/net/`: the WebSocket link with reconnect, and the art and map
  caches (`Map` in memory, the browser HTTP cache below it).
- `web/src/input/`: keyboard, mouse, touch and Gamepad API events, passed to
  `View::input`.
- `web/src/audio/`: Web Audio for sounds and music.

## Scope

The browser has every Modern feature of the Rust window:

- World: land with heights and textures, statics, multis (houses, boats),
  custom house designs, mobiles with UOP and MUL animations, mounts, war
  stances, corpses, effects (0x70, 0xC0, 0xC7), rain, snow, night, lights,
  season art, house placement preview.
- Control: Take control and give back, key and right-mouse steering,
  double-click walk, drag and drop with Shift split, tooltips, the
  right-click ring, targets, walk prediction.
- Panels: sheet (worn items, status, skills, spells, party), hotbar,
  journal with filters, near list, vitals, pack, agent panel, and every
  panel of `window/modern/` (abilities, agents, arrow, ask, bars, buffs,
  combat, counters, damage meter, durability, dye, entry, grid, hue, info
  bar, journal, loot, markers, party, race, radar, skills, spells, stats,
  status, tips).
- Shard windows: shard gumps in their shard layout, shop and trade, old
  menus, books, bulletin boards, chat, profiles, map items, house designer.
- Other: world map with click-to-walk, macros editor, Jev command box,
  options pages, game controller, screenshots, login screen, character
  creation, speech and damage over heads, quest arrow.

Not in scope: the Classic look. Public hosting and user accounts.

## Error handling

- Link lost: the page shows "link lost" and reconnects with backoff. The
  session keeps running in UOTerm. On reconnect, the first push is a full
  frame.
- Art missing in the client files: the API answers 404; the page draws the
  radar color for that tile, as the Rust window does without art.
- No client files at all: the page draws the radar picture, as the Rust
  window does.
- Tool call error: the answer text shows in the result line, as now.
- Wrong or missing token: 401; the page asks for the token again.

## Testing

1. All present Rust tests pass. Tests move with their code.
2. New Rust tests: each art and map route with the small test files the
   `uoterm-nav` tests use; the live link pushes a frame on a state change and
   answers a tool call with its id; the login link against the mock shard
   (shard pick, character list, make, delete, play); token by header and by
   cookie; loopback rule on the new routes.
3. Draw list tests in `uoterm-view` from fixed watch frames: order, depth,
   glide position at a given time, light values.
4. `cargo build -p uoterm-web --target wasm32-unknown-unknown` passes.
5. Vitest: input to tool calls, panel rendering from fixed panel data, art
   cache, reconnect.
6. Live check: `uoterm web` against the mock shard (`mock.rs`), the page in
   the browser pane, screenshots compared with the Rust window
   (`--snapshot`) for the same frame.
