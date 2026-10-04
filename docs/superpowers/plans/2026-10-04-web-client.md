# Web Client (Three.js) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A browser client that looks and works like the Modern style of the Rust play window, drawn by Three.js, with every UI rule shared with the Rust window through a new WebAssembly-safe crate.

**Architecture:** UI rules move out of `crates/uoterm/src/window/` into a new crate `uoterm-view` (no egui, no tokio, no file access, no `Instant`). The Rust window and a thin `wasm-bindgen` crate `uoterm-web` both use it. UOTerm gains a live WebSocket per session, a login WebSocket, art/map/data routes and a `uoterm web` command. The `web/` folder holds a TypeScript + Vite + Three.js + Preact page that only draws, plays sound and reads input.

**Tech Stack:** Rust 1.87+ stable (workspace), axum 0.8 with `ws`, tower-http `fs`, image 0.25 (png), wasm-bindgen, wasm32-unknown-unknown, rustysynth; Node 24, npm 11, TypeScript 5, Vite, Three.js, Preact, Vitest, ESLint.

**Spec:** `docs/superpowers/specs/2026-10-04-web-client-design.md`

## Global Constraints

- Modern style only in the browser. Classic stays in the Rust window.
- No rule exists twice. A UI rule lives in `uoterm-view`; the Rust window and `uoterm-web` call it.
- `uoterm-view` depends only on `uoterm-protocol`, `uoterm-world`, `uoterm-nav`, `uoterm-assist`, `serde`, `serde_json`. It has no egui, gilrs, rodio, tokio, reqwest, `std::fs`, `config_dir`, `std::time::Instant`, `chrono::Local`, `rand::thread_rng`.
- `cargo build -p uoterm-view --target wasm32-unknown-unknown` and `cargo build -p uoterm-web --target wasm32-unknown-unknown` pass.
- The Rust window works the same after every task. All present tests pass after every task (`cargo test --workspace`; 1,909+ tests).
- New API routes sit behind `require_bearer`. Loopback rule and token rule unchanged. A non-loopback bind still needs `UOTERM_API_TOKEN`.
- The password is sent to UOTerm one time and never stored. The TypeSafe key never goes to the browser.
- Profiles stay in the same TOML files: `config_dir()/profiles/default.toml` and `config_dir()/profiles/<file_safe("host:port")>/<file_safe(name)>.toml`.
- Key names stay the egui names saved profiles use (`F1`, `A`, `ArrowUp`, `Tab`, ...).
- Code, comments, docs and tests in English. Named constants instead of literals. No dead code, no placeholders, no copies.
- Finish rules: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, both wasm builds, `npm run lint`, `npm run test`, `npm run build` (in `web/`).
- Git: run git only when the user allows commits for this build. Commit format `<type>(<scope>): <subject>`, imperative, ≤50 chars, no period; no Co-Authored-By line.
- Keep the folder small: no scratch files in the repo; `cargo clean` and stop test processes at the end.

## Review Focus

1. **The page closes in the middle of a two-step act** (lift sent, drop not yet sent). Expected: the drop still happens. UOTerm, not the page, runs the steps of one act and the gap between them. Pinned in Task 11 (`a_closed_link_still_finishes_a_started_act`).
2. **Two browser tabs on one session.** Expected: both get every frame and both can act. Pinned in Task 11 (`two_links_on_one_session_both_get_frames`).
3. **A home-network caller with a wrong or old token cookie, and a WebSocket upgrade that carries only the cookie.** Expected: 401, and the page asks for the token again; the right cookie opens the socket. Pinned in Task 11 (`a_cookie_token_opens_the_live_link`, `a_wrong_cookie_is_refused`) and Task 16 (`asks_for_the_token_again_after_401`).
4. **A picture the client files do not hold.** Expected: the API answers 404 one time, the browser marks it `Missing` and never asks again, and the scene draws the radar color as the Rust window does. Pinned in Task 13 (`a_missing_item_is_not_found`) and Task 15 (`a_missing_picture_is_asked_one_time`).
5. **A keyboard that is not US-English** (AZERTY, QWERTZ) and a hotkey saved in the Rust window. Expected: `Ctrl+A` fires on the key that types "a", as in the Rust window (egui reads logical keys). Pinned in Task 17 (`maps_the_logical_key_not_the_key_position`).

---

## File Structure

### New crate `crates/uoterm-view/` (WebAssembly-safe rules)

| Path | Responsibility | Comes from |
|---|---|---|
| `Cargo.toml` | crate manifest | new |
| `src/lib.rs` | module list and re-exports | new |
| `src/geom.rs` | `Point`, `Vector`, `Area`, `Rgba` | new (replaces egui `Pos2`, `Vec2`, `Rect`, `Color32` in rules) |
| `src/input.rs` | `Mods`, `KeyName`, `KeyPress`, `PointerButton` | new (replaces egui `Modifiers`, `Key`) |
| `src/frame.rs` | `WatchFrame` and all `Watch*` types, `from_observe(&Value, now)` | `crates/uoterm/src/view.rs` (data and parse part) |
| `src/act.rs` | `Act`, `DropTo`, `Channel`, `Tip`, `Asker`, `Ask`, `Answer`, `Report`, `Act::calls`, `Act::words`, `quoted`, `LIFT_TO_DROP`, `STEP_HOLD_MS`, `WHOLE_PILE`, `answer_partition` | `window/control.rs:43-830,1146-1240` |
| `src/guard.rs` | `Guard`, `Checked`, `Seen`, `KeptGrabBags` (data), `GRAB_BAGS_FILE` | `window/actions/guard.rs` (file I/O removed) |
| `src/settings/` (`mod.rs`, `choices.rs`, `keys.rs`, `pages.rs`, `table.rs`) | `Profile` and every option page, chords, rows | `window/settings.rs`, `window/settings/{choices,keys,pages,table}.rs` |
| `src/actions/` (`mod.rs`, `arguments.rs`, `editor.rs`, `journal.rs`, `resolve.rs`, `runner.rs`, `select.rs`, `view_range.rs`, `screenshot.rs`, `windows.rs`) | action table, macro runner, resolve, editor, select, view range, screenshot words, Modern window-command helpers | `window/actions.rs`, `window/actions/*.rs` pure parts, `window/actions/modern.rs:49-120` helpers |
| `src/keys/` (`mod.rs`, `chat.rs`) | `KeyDispatch`, `Focus`, `WalkKeys`, `ChatLine`, chat parsing | `window/keys.rs`, `window/keys/chat.rs` pure parts |
| `src/steer.rs` | `Movement`, `Steer::decide`, `way_of*` | `window/steer.rs` pure parts |
| `src/pad.rs` | `PadFrame`, `PadState`, stick and chord rules | `window/pad.rs` pure parts |
| `src/desk.rs` | `Zone`, `Split`, `Desk` bookkeeping, drop rules | `window/desk.rs` pure parts |
| `src/clicks.rs` | `GroundClicks`, `act_for_click`, `grabbed`, `hint_for`, `ChatMode`, bar button lists | `window/control_ui.rs:139,223-310,595-631,851-856` |
| `src/look.rs` | look rules | `window/look.rs` |
| `src/filters.rs` | tile filters, `Seat` | `window/filters.rs` |
| `src/lights.rs` | light rules and the light cell grid | `window/lights.rs` minus `LightMap::draw` upload |
| `src/predict.rs` | walk prediction | `window/predict.rs` |
| `src/art.rs` | `ArtRequest`, `Sprite`, `Picture`, `Art<T>`, `WorldArt` trait, `Cell`, `CellStatic`, `Stretch`, `ItemPaint`, `AnimRules` | `window/atlas.rs:17-70`, `window/client_art.rs:38-130` |
| `src/atlas.rs` | `ShelfPacker` (atlas placement and reset) | `window/atlas.rs:108-165` pure parts |
| `src/scene/` (`mod.rs`, `glide.rs`, `build.rs`, `ceiling.rs`, `canvas.rs`, `pick.rs`, `plates.rs`, `overlays.rs`) | the scene rules; output `SceneDraw` | `window/scene.rs` rule half |
| `src/sky.rs`, `src/floats.rs`, `src/cursor.rs` | cue intake, effects, weather, float rules, cursor shape rules | `window/sky.rs`, `window/floats.rs`, `window/cursor.rs` pure parts |
| `src/map_lay.rs` | world map projection `Lay`, zoom rules, marks layout | `window/map_view.rs:62-160,380,428-600` pure parts |
| `src/audio.rs` | step sound, nearness, gain, rain, `new_cues`, `EffectCue`, `room`, `Score`, `music_file` | `window/audio.rs`, `window/audio/{effects,midi,score}.rs` pure parts |
| `src/model/` | all of `window/model/` (host-bound parts split out) | `window/model/*.rs` |
| `src/ui/` (`mod.rs`, `layout.rs`, `places.rs`, `launch.rs`, `ring.rs`, `deck.rs`, `gumps.rs`, `grid_clicks.rs`, `hud.rs`, `bars.rs`, `lists.rs`, `text_field.rs`, `theme.rs`) | rules of the Modern panels that now sit in drawing files | listed in Task 8 |
| `src/video.rs` | `frame_interval` and the clamp constants | `window/video.rs:10-14,93` |
| `tests/wasm_build.rs` | none (wasm build is a script) | — |

### New crate `crates/uoterm-web/` (wasm-bindgen wrapper)

| Path | Responsibility |
|---|---|
| `Cargo.toml` | `crate-type = ["cdylib", "rlib"]`, deps `uoterm-view`, `wasm-bindgen`, `serde-wasm-bindgen`, `rustysynth`, `js-sys` |
| `src/lib.rs` | `WebView` export |
| `src/web_art.rs` | `WebArt`: `WorldArt` from fetched pictures and tables |
| `src/buffers.rs` | `DrawBuffers`: typed arrays for JS |
| `src/out.rs` | `OutCall` |
| `src/synth.rs` | MIDI to PCM |

### Changed crates

| Path | Change |
|---|---|
| `Cargo.toml` (workspace) | members `crates/uoterm-view`, `crates/uoterm-web`; deps `axum = { version = "0.8", features = ["ws"] }`, `tower-http = { version = "0.6", features = ["fs"] }`, `image`, `wasm-bindgen`, `serde-wasm-bindgen`, `js-sys`, `futures-util` |
| `crates/uoterm-world/src/tool_names.rs` | new: every `TOOL_*` constant and `ARG_HUMAN` (moved from `uoterm-runtime/src/tools.rs` and `characters.rs`) |
| `crates/uoterm-world/src/login.rs` | new: `CharacterChoices`, `NewCharacterWish`, `CharacterRequest`, `LoginAsk` (wire form of `LoginQuestion`), `LoginReply` |
| `crates/uoterm-world/src/landmarks.rs` | new: `Landmarks` data and parsing (moved from `uoterm-runtime/src/landmarks.rs`; `load(path)` stays in the runtime as a free function) |
| `crates/uoterm-world/src/hotkeys.rs` | new: `FIXED_HOTKEY_NAMES` (moved from `uoterm-runtime` `session::fixed_hotkey_names`) |
| `crates/uoterm-nav/src/*.rs` | `Serialize, Deserialize` on `ItemTile`, `LandTile`, `MultiPiece`, `LightShape`, `TileFlagSet`; new `AnimRules` extracted from `AnimData` (`anim.rs`) |
| `crates/uoterm-runtime/src/api.rs` | `ws` live link, login link, cookie token, `POST /v1/web/token`, `router_for(runtime, token, local_only)` public so `uoterm web` can merge routes |
| `crates/uoterm-runtime/src/api/live.rs` | new: session live link |
| `crates/uoterm-runtime/src/api/login.rs` | new: login live link |
| `crates/uoterm-runtime/src/api/auth.rs` | new: `require_bearer`, cookie read, `same_secret`, loopback checks (moved from `api.rs`) |
| `crates/uoterm/src/art/` (`mod.rs`, `client_art.rs`, `figure.rs`, `text.rs`, `png.rs`) | native art: `ClientArt` (moved out of `window/`), `figure::compose`, `UoFonts` (moved from `window/classic/text.rs`), PNG encode |
| `crates/uoterm/src/web/` (`mod.rs`, `art_routes.rs`, `map_routes.rs`, `data_routes.rs`, `sound_routes.rs`, `profile_routes.rs`, `jev_routes.rs`, `files.rs`) | native routes for `uoterm web` |
| `crates/uoterm/src/main.rs` | `web` subcommand |
| `crates/uoterm/src/window/bridge.rs` | new: egui ↔ `uoterm-view` type conversions |
| `crates/uoterm/src/window/*` | rule code removed; calls `uoterm-view` |
| `scripts/check-wasm.sh` | builds both crates for `wasm32-unknown-unknown` |

### New folder `web/`

| Path | Responsibility |
|---|---|
| `package.json`, `tsconfig.json`, `vite.config.ts`, `eslint.config.js`, `vitest.config.ts`, `index.html` | tooling |
| `scripts/build-wasm.mjs` | `cargo build -p uoterm-web --release --target wasm32-unknown-unknown` + `wasm-bindgen --target web --out-dir web/src/wasm` |
| `src/main.tsx` | app start, screen switch (token → login → game) |
| `src/net/api.ts` | fetch helpers with 401 handling |
| `src/net/live.ts` | session live link with reconnect |
| `src/net/login.ts` | login live link |
| `src/net/art.ts` | art and data fetch queue feeding `WebView` |
| `src/world/renderer.ts` | Three.js renderer, camera, geometry, atlas texture |
| `src/world/lights.ts` | light overlay texture |
| `src/input/keys.ts` | `KeyboardEvent` → egui key name |
| `src/input/pointer.ts` | mouse, wheel, touch → input events |
| `src/input/gamepad.ts` | Gamepad API → input events |
| `src/audio/player.ts` | Web Audio |
| `src/screens/Token.tsx`, `Login.tsx`, `Creation.tsx` | screens |
| `src/panels/*.tsx` | one file per Modern panel (list in Tasks 19-20) |
| `src/panels/Frame.tsx` | shared panel frame |
| `src/theme.css` | Modern theme tokens |
| `src/fonts/` | Barlow files (copied from `crates/uoterm/assets/fonts/` by the build script, not committed twice) |
| `test/*.test.ts(x)` | Vitest tests |

---

## Build order

Tasks 1-10 move rules into `uoterm-view`; the Rust window keeps working after each one. Tasks 11-14 add the server side. Tasks 15-21 build the wasm wrapper and the page. Task 22 is the live check and the docs.

---

### Task 1: `uoterm-view` crate, geometry, input types, wasm check

**Files:**
- Create: `crates/uoterm-view/Cargo.toml`, `crates/uoterm-view/src/lib.rs`, `crates/uoterm-view/src/geom.rs`, `crates/uoterm-view/src/input.rs`
- Create: `crates/uoterm/src/window/bridge.rs`
- Create: `scripts/check-wasm.sh`
- Modify: `Cargo.toml` (workspace members and `uoterm-view` path dep), `crates/uoterm/Cargo.toml` (dep `uoterm-view.workspace = true`), `crates/uoterm/src/window.rs` (`mod bridge;`)

**Interfaces:**
- Produces:
  - `uoterm_view::geom::{Point { x: f32, y: f32 }, Vector { x: f32, y: f32 }, Area { min: Point, max: Point }, Rgba(pub [u8; 4])}` with `Point::new`, `Vector::new`, `Area::from_min_size(Point, Vector)`, `Area::from_center_size`, `Area::contains(Point) -> bool`, `Area::width()`, `Area::height()`, `Area::size() -> Vector`, `Area::center() -> Point`, `Area::translate(Vector) -> Area`, `Area::intersect(Area) -> Area`, `Area::expand(f32) -> Area`, `Point + Vector`, `Point - Point -> Vector`, `Vector * f32`, `Vector::length()`, `Rgba::from_rgba_premultiplied`, `Rgba::to_array`, `Rgba::with_alpha(f32)`. All `Copy, Clone, Debug, PartialEq, Default, Serialize, Deserialize`.
  - `uoterm_view::input::{Mods { ctrl: bool, alt: bool, shift: bool, command: bool }, KeyName(pub String), KeyPress { key: KeyName, mods: Mods, pressed: bool, repeat: bool }, PointerButton::{Primary, Secondary, Middle}}`.
  - `crate::window::bridge` (native): `fn point(egui::Pos2) -> Point`, `fn pos2(Point) -> egui::Pos2`, `fn vector(egui::Vec2) -> Vector`, `fn vec2(Vector) -> egui::Vec2`, `fn area(egui::Rect) -> Area`, `fn rect(Area) -> egui::Rect`, `fn rgba(egui::Color32) -> Rgba`, `fn color(Rgba) -> egui::Color32`, `fn mods(egui::Modifiers) -> Mods`, `fn key_name(egui::Key) -> KeyName` (uses `Key::name()`), `fn egui_key(&KeyName) -> Option<egui::Key>` (uses `Key::from_name`).

- [ ] **Step 1: Add the wasm target and wasm-bindgen CLI**

Run:
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.100
```
Expected: target installed; `wasm-bindgen --version` prints `wasm-bindgen 0.2.100`. The workspace `wasm-bindgen` dep in Task 15 must use `=0.2.100` to match the CLI.

- [ ] **Step 2: Write the failing tests** in `crates/uoterm-view/src/geom.rs` (module `tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_area_holds_its_far_corner() {
        let area = Area::from_min_size(Point::new(1.0, 2.0), Vector::new(3.0, 4.0));
        assert!(area.contains(Point::new(4.0, 6.0)));
        assert!(!area.contains(Point::new(4.1, 6.0)));
    }

    #[test]
    fn an_area_moves_by_a_vector() {
        let area = Area::from_min_size(Point::new(0.0, 0.0), Vector::new(2.0, 2.0));
        let moved = area.translate(Vector::new(5.0, -1.0));
        assert_eq!(moved.min, Point::new(5.0, -1.0));
        assert_eq!(moved.size(), Vector::new(2.0, 2.0));
    }

    #[test]
    fn alpha_scales_every_premultiplied_channel() {
        let half = Rgba([200, 100, 50, 255]).with_alpha(0.5);
        assert_eq!(half, Rgba([100, 50, 25, 128]));
    }
}
```
And in `crates/uoterm/src/window/bridge.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    #[test]
    fn a_rect_survives_the_round_trip() {
        let rect = egui::Rect::from_min_max(egui::pos2(1.0, 2.0), egui::pos2(30.0, 40.0));
        assert_eq!(rect(area(rect)), rect);
    }

    #[test]
    fn a_key_keeps_its_egui_name() {
        assert_eq!(key_name(egui::Key::F1).0, "F1");
        assert_eq!(egui_key(&KeyName("ArrowUp".into())), Some(egui::Key::ArrowUp));
    }

    #[test]
    fn a_color_survives_the_round_trip() {
        let c = egui::Color32::from_rgba_premultiplied(10, 20, 30, 40);
        assert_eq!(color(rgba(c)), c);
    }
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p uoterm-view geom && cargo test -p uoterm bridge`
Expected: FAIL (crate and module do not exist).

- [ ] **Step 4: Write the crate**

`crates/uoterm-view/Cargo.toml`:
```toml
[package]
name = "uoterm-view"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
authors.workspace = true
rust-version.workspace = true
description = "UI rules shared by the UOTerm play window and the web client"

[dependencies]
uoterm-protocol.workspace = true
uoterm-world.workspace = true
uoterm-nav.workspace = true
uoterm-assist.workspace = true
serde.workspace = true
serde_json.workspace = true
```
`src/lib.rs`:
```rust
//! The rules of the play window that do not depend on how a window draws:
//! the watch frame, acts, settings, keys, the scene and the panels. The
//! Rust window and the web client both use them, so a rule lives in one
//! place. Nothing here opens a file, starts a thread or reads a clock; the
//! caller gives the time in seconds.

pub mod geom;
pub mod input;
```
`src/geom.rs`: the types listed under Interfaces. `Rgba::with_alpha(f32)` multiplies each channel and rounds (`(c as f32 * a).round() as u8`). `src/input.rs`: the input types with `Serialize, Deserialize`.

`scripts/check-wasm.sh`:
```bash
#!/usr/bin/env bash
# Builds the crates the web client runs for the browser target. A crate
# that reaches a file, a thread or the wall clock fails here, not in the
# browser.
set -euo pipefail
cd "$(dirname "$0")/.."
for crate in "$@"; do
  cargo build -p "$crate" --target wasm32-unknown-unknown
done
```
Make it executable (`chmod +x scripts/check-wasm.sh`).

`crates/uoterm/src/window/bridge.rs`: the conversions listed under Interfaces, with a file header:
```rust
//! The edge between egui and the shared rules: each egui type the window
//! holds becomes the plain type of `uoterm-view` here, and back.
```

- [ ] **Step 5: Run the tests and the wasm build**

Run: `cargo test -p uoterm-view && cargo test -p uoterm bridge && scripts/check-wasm.sh uoterm-view`
Expected: PASS; wasm build finishes.

- [ ] **Step 6: Full check**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: PASS.

- [ ] **Step 7: Commit**
```bash
git add Cargo.toml Cargo.lock crates/uoterm-view crates/uoterm/Cargo.toml crates/uoterm/src/window.rs crates/uoterm/src/window/bridge.rs scripts/check-wasm.sh
git commit -m "feat(view): add the shared view crate"
```

---

### Task 2: Move tool names, login types, landmarks and hotkey names down into `uoterm-world`

**Files:**
- Create: `crates/uoterm-world/src/tool_names.rs`, `crates/uoterm-world/src/login.rs`, `crates/uoterm-world/src/landmarks.rs`, `crates/uoterm-world/src/hotkeys.rs`
- Modify: `crates/uoterm-world/src/lib.rs` (modules and re-exports)
- Modify: `crates/uoterm-runtime/src/tools.rs` (remove the 125 `TOOL_*` definitions, `pub use uoterm_world::tool_names::*;`), `crates/uoterm-runtime/src/characters.rs` (remove its 5 `TOOL_*` definitions, use the world ones), `crates/uoterm-runtime/src/config.rs:186-288` (remove `CharacterChoices`, `NewCharacterWish`, `CharacterRequest`; `pub use uoterm_world::login::{CharacterChoices, NewCharacterWish, CharacterRequest};`), `crates/uoterm-runtime/src/landmarks.rs` (keep only `pub fn load(path: &Path) -> std::io::Result<Landmarks>`; data and parse use the world module), `crates/uoterm-runtime/src/session.rs` (`fixed_hotkey_names()` returns `uoterm_world::hotkeys::FIXED_HOTKEY_NAMES`), `crates/uoterm-runtime/src/lib.rs` (re-exports keep the same public names)
- Test: unit tests in each new world file

**Interfaces:**
- Produces:
  - `uoterm_world::tool_names::{TOOL_WATCH, TOOL_PROPERTIES, ..., ARG_HUMAN}`: every constant with the same name and value as today. `uoterm_runtime::tools::TOOL_*` keeps working through `pub use`.
  - `uoterm_world::login::{CharacterChoices, NewCharacterWish, CharacterRequest}`: same fields as today plus `Serialize, Deserialize`.
  - `uoterm_world::login::LoginAsk` (serde, tag `"kind"`): `Shard { names: Vec<String> }`, `Characters { names: Vec<String>, refused: Option<String>, choices: CharacterChoices }`, `Character { names: Vec<String> }`.
  - `uoterm_world::login::LoginReply` (serde, tag `"kind"`): `Pick { index: usize }`, `Request { request: CharacterRequest }`.
  - `uoterm_world::landmarks::Landmarks` with `parse_uoam_map(&str) -> Landmarks`, `parse_waypoints_lua(&str) -> Landmarks`, `from_text(name: &str, text: &str) -> Landmarks` (extension `.lua` picks the Lua reader), `find`, `find_of_kind`, `kinds`, `per_map`, `len`, `is_empty`.
  - `uoterm_world::hotkeys::FIXED_HOTKEY_NAMES: &[&str]`.
  - `uoterm-protocol::StartTown` gets `Serialize, Deserialize` if it lacks them (needed by `CharacterChoices`).

- [ ] **Step 1: Write the failing tests** in `crates/uoterm-world/src/login.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_shard_question_goes_on_the_wire_by_kind() {
        let ask = LoginAsk::Shard { names: vec!["Atlantic".into()] };
        assert_eq!(serde_json::to_value(&ask).unwrap(), json!({"kind": "Shard", "names": ["Atlantic"]}));
    }

    #[test]
    fn a_make_request_comes_back_from_the_wire() {
        let wish = NewCharacterWish { name: "Mara".into(), ..NewCharacterWish::default() };
        let reply = LoginReply::Request { request: CharacterRequest::Make(Box::new(wish.clone())) };
        let text = serde_json::to_string(&reply).unwrap();
        let back: LoginReply = serde_json::from_str(&text).unwrap();
        assert_eq!(back, LoginReply::Request { request: CharacterRequest::Make(Box::new(wish)) });
    }
}
```
`NewCharacterWish` gets `#[derive(Default)]` if it lacks it.

In `crates/uoterm-world/src/tool_names.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_watch_tool_keeps_its_name() {
        assert_eq!(TOOL_WATCH, "watch");
        assert_eq!(ARG_HUMAN, "human");
    }
}
```
Move the existing landmark tests from `crates/uoterm-runtime/src/landmarks.rs` into `crates/uoterm-world/src/landmarks.rs` unchanged; keep the file-reading test in the runtime.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-world login tool_names landmarks`
Expected: FAIL (modules missing).

- [ ] **Step 3: Move the code**

Cut each item from its runtime file and paste it into the world file. Do not leave a copy: the runtime keeps only `pub use` lines so its present callers compile unchanged. Check `ARG_HUMAN`'s value in `crates/uoterm-runtime/src/tools.rs` and keep it.

- [ ] **Step 4: Run all tests and check for copies**

Run:
```bash
cargo test --workspace
grep -rn "pub const TOOL_" crates/uoterm-runtime/src | wc -l
```
Expected: tests PASS; the grep prints `0`.

- [ ] **Step 5: Check the runtime-free use from the view crate**

Run: `scripts/check-wasm.sh uoterm-world`
Expected: wasm build finishes.

- [ ] **Step 6: Commit**
```bash
git add crates/uoterm-world crates/uoterm-runtime crates/uoterm-protocol
git commit -m "refactor(world): move shared login and tool names"
```

---

### Task 3: Move the watch frame into `uoterm-view` with the clock given by the caller

**Files:**
- Create: `crates/uoterm-view/src/frame.rs`
- Modify: `crates/uoterm/src/view.rs` (keep only the window size constants that the window uses, the text radar `text()` printer used by `main.rs` text watch, and `pub use uoterm_view::frame::*;`), `crates/uoterm/src/window.rs` (clock), `crates/uoterm/src/window/scene.rs:308,1449-1462` (`stride_taken: Option<f64>`, `is_fresh`, `on_window_clock`), `crates/uoterm/src/main.rs:641-667` (text watch passes its clock)
- Test: move every test of `view.rs` that checks parsing into `frame.rs`

**Interfaces:**
- Consumes: `uoterm_view::geom` (Task 1); `uoterm_world` types already used by `view.rs`.
- Produces:
  - `uoterm_view::frame::WatchFrame::from_observe(value: &serde_json::Value, now: f64) -> WatchFrame` (`now` = seconds on the caller's clock).
  - `uoterm_view::frame::WatchStride { slot: f64, lasts: f64 }` (seconds; `slot = now - ago_ms / 1000`).
  - Every other `Watch*` type, `Danger`, `SYM_*`, `carried`, `backpack`, `danger`, `error_frame`, unchanged names.
  - Native: `crate::window::Clock` (`struct Clock(std::time::Instant)` with `fn seconds(&self) -> f64`), created once in `WatchApp::start`, shared by `poll_loop` (for `from_observe`) and `update` (for `time`), so both read one clock.

- [ ] **Step 1: Write the failing tests** in `crates/uoterm-view/src/frame.rs`

```rust
#[cfg(test)]
mod stride_tests {
    use super::*;
    use serde_json::json;

    const NOW: f64 = 10.0;

    #[test]
    fn a_stride_is_placed_on_the_callers_clock() {
        let value = json!({"stride": {"ago_ms": 250, "ms": 400}});
        let stride = WatchStride::read(&value, NOW).unwrap();
        assert!((stride.slot - 9.75).abs() < 1e-9);
        assert!((stride.lasts - 0.4).abs() < 1e-9);
    }

    #[test]
    fn a_frame_without_a_stride_has_none() {
        let frame = WatchFrame::from_observe(&json!({}), NOW);
        assert_eq!(frame.stride, None);
    }
}
```
In `crates/uoterm/src/window/scene.rs` tests (replace the `Instant` tests at `:4179-4207`):
```rust
#[test]
fn a_stride_is_fresh_only_within_its_length_and_lag() {
    let mut scene = Scene::new(None);
    scene.set_poll_every(std::time::Duration::from_millis(33));
    let stride = WatchStride { slot: 5.0, lasts: 0.4 };
    assert!(scene.is_fresh(&stride, 5.2));
    assert!(!scene.is_fresh(&stride, 6.0));
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-view stride_tests && cargo test -p uoterm a_stride_is_fresh`
Expected: FAIL.

- [ ] **Step 3: Move the code**

1. Move all data types and parse functions of `view.rs` into `frame.rs`. Change `WatchStride::read(value, now: f64)`: `slot: now - ms("ago_ms")? / 1000.0`, `lasts: ms("ms")? / 1000.0` where `ms` returns `f64`. Remove `Instant` and `Duration` from the stride.
2. `from_observe(value, now)` passes `now` to `WatchStride::read`.
3. `scene.rs`: `stride_taken: Option<f64>`; `fn is_fresh(&self, stride: &WatchStride, time: f64) -> bool { let fresh_for = stride.lasts + self.stride_lag; self.stride_taken != Some(stride.slot) && time - stride.slot <= fresh_for }`; `fn on_window_clock(&self, stride: WatchStride, _time: f64) -> Stride { Stride { starts: stride.slot + self.stride_lag, tile_seconds: stride.lasts } }`. Update callers to pass `time`.
4. `window.rs`: add `Clock`; `poll_loop` gets a `Clock` clone and calls `WatchFrame::from_observe(&value, clock.seconds())`; `update` uses `let time = self.clock.seconds();` where it used `i.time`. `Snapshot.first_picture` keeps `Instant` (native only).
5. `main.rs` text watch: create a `Clock` and pass `clock.seconds()`.

- [ ] **Step 4: Run all tests and the wasm build**

Run: `cargo test --workspace && scripts/check-wasm.sh uoterm-view`
Expected: PASS.

- [ ] **Step 5: Check the window by eye**

Run (two terminals, from a scratch folder outside the repo with its own `uoterm.toml` per the watch-window memory):
```bash
cargo run -p uoterm -- mock-shard
```
```bash
WAYLAND_DISPLAY= cargo run -p uoterm -- play --profile mock --go --open sheet --snapshot /tmp/claude-1000/uoterm-task3.png
```
Expected: the PNG shows the character standing, the sheet open, as before the change. Walk once with a `step` tool call over the API and take a second snapshot: the character moved smoothly (no jump).

- [ ] **Step 6: Commit**
```bash
git add crates/uoterm-view crates/uoterm/src
git commit -m "refactor(view): move the watch frame to the view crate"
```

---

### Task 4: Move settings into `uoterm-view`

**Files:**
- Create: `crates/uoterm-view/src/settings/{mod.rs,choices.rs,keys.rs,pages.rs,table.rs}`
- Modify: `crates/uoterm/src/window/settings.rs` (becomes `pub use uoterm_view::settings::*; pub mod store;`), delete-by-move `window/settings/{choices,keys,pages,table}.rs` (their content moves; the files are removed from the window), `window/settings/store.rs` (stays native)
- Modify: every window file that called `KeyChord::from_egui` or `ModifierKey::is_held(egui::Modifiers)`: call `KeyChord::from_press(&bridge::key_name(key), bridge::mods(m))` and `ModifierKey::is_held(bridge::mods(m))`
- Test: move the tests of the moved files with them

**Interfaces:**
- Consumes: `uoterm_view::input::{KeyName, Mods}`; `uoterm_view::frame::{WINDOW_WIDTH, WINDOW_HEIGHT}` (move these two constants into `frame.rs` from `view.rs`).
- Produces:
  - `uoterm_view::settings::{Profile, SkillGroupSet, AnchorCell, GumpPlace, SoundKind, ...}` and every page struct, choice enum, `KeyChord`, `KeyChordError`, `PadChord`, `MacroStep`, `KeyBinding`, `OptionRow`, `OptionKind`, `OptionValue`, `Unit`, `rows_on(Page)`.
  - `KeyChord::from_press(key: &KeyName, mods: Mods) -> KeyChord` (replaces `from_egui`).
  - `ModifierKey::is_held(self, mods: Mods) -> bool`.
  - `uoterm_view::settings::WORN_LAYERS` (moved from `window/figure.rs:52`), `uoterm_view::settings::MAP_ZOOMS` (moved from `model/world_map.rs:63` `ZOOMS`), `WINDOW_MIN_WIDTH`, `WINDOW_MIN_HEIGHT` (moved from `window.rs:74-75`), because `table.rs` needs them. The old places `pub use` them.
  - `SoundOptions.midi_sound_font` and `FontOptions.truetype_font` stay `Option<PathBuf>` (serde form unchanged; the browser ignores the path and uses `/v1/soundfont` and `/v1/fonts/{name}`, keyed by file name).

- [ ] **Step 1: Write the failing tests** in `crates/uoterm-view/src/settings/keys.rs`

```rust
#[cfg(test)]
mod press_tests {
    use super::*;
    use crate::input::{KeyName, Mods};

    #[test]
    fn a_press_becomes_the_saved_chord_words() {
        let mods = Mods { ctrl: true, shift: true, ..Mods::default() };
        let chord = KeyChord::from_press(&KeyName("F1".into()), mods);
        assert_eq!(chord.to_string(), "Ctrl+Shift+F1");
    }

    #[test]
    fn a_saved_profile_reads_the_same_after_the_move() {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/profile_before_move.toml")).unwrap();
        let profile: Profile = toml::from_str(&text).unwrap();
        assert_eq!(toml::to_string(&profile).unwrap(), text);
    }
}
```
Create `crates/uoterm-view/tests/data/profile_before_move.toml` before the move with: `cargo run -p uoterm -- ...` is not needed: write it from a test in the window crate before moving (`toml::to_string(&Profile::default())` plus one key binding `Ctrl+Shift+F1` → `cast_spell` `Heal`), then keep that file as the fixture. Add `toml` as a dev-dependency of `uoterm-view` (tests only; it is pure Rust). `std::fs` in a test is fine: the test does not build for wasm.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-view press_tests`
Expected: FAIL.

- [ ] **Step 3: Move the code**

Move the four files. Replace `egui::Modifiers` with `Mods` and `egui::Key` with `KeyName` in the moved code. Keep `#[serde(default)]` everywhere. `store.rs` keeps `ProfileStore`, `ProfileHome`, `shard_address`, `CharacterKey`, `OldAudio` in the window and imports `uoterm_view::settings::Profile`.

- [ ] **Step 4: Run all tests and the wasm build**

Run: `cargo test --workspace && scripts/check-wasm.sh uoterm-view`
Expected: PASS; the fixture round-trips byte for byte.

- [ ] **Step 5: Commit**
```bash
git add crates/uoterm-view crates/uoterm/src
git commit -m "refactor(view): move settings to the view crate"
```

---

### Task 5: Move `Act`, its tool calls and the guard into `uoterm-view`

**Files:**
- Create: `crates/uoterm-view/src/act.rs`, `crates/uoterm-view/src/guard.rs`
- Modify: `crates/uoterm/src/window/control.rs` (keeps `Hand`, `AnswerBox` wrapper, `work`, `read_tips`, `answer_asks`, `answer_one`, `pick_one`, `place_on_map`, `perform`; `pub use uoterm_view::act::*;`), `crates/uoterm/src/window/actions/guard.rs` (keeps only grab-bag file load/save: `fn load_grab_bags() -> KeptGrabBags`, `fn save_grab_bags(&KeptGrabBags)`; `Guard` comes from the view crate)
- Test: move the `Act::calls`, `Act::words`, `Guard::check` tests

**Interfaces:**
- Consumes: `uoterm_world::tool_names::*` (Task 2), `uoterm_view::frame::WatchFrame` (Task 3), `uoterm_view::settings` (Task 4).
- Produces:
  - `uoterm_view::act::Act` (all ~90 variants), `Act::calls(&self) -> Vec<(&'static str, serde_json::Value)>` (now `pub`), `Act::words(&self) -> String`, `Act::is_two_step(&self) -> bool` (true when `calls()` is a lift then a drop), `DropTo`, `Channel`, `say_channel`, `quoted`, `Tip`, `Asker`, `Ask`, `Answer`, `Report { text, failed }`, `places_in`, `string_list`, `status_words`, `tip_lines`, constants `LIFT_TO_DROP: Duration` (650 ms), `STEP_HOLD_MS` (600), `WHOLE_PILE`.
  - `uoterm_view::act::with_human(args: Value) -> Value` (adds `ARG_HUMAN: true`; `perform` used to do this inline).
  - `uoterm_view::act::split_answers(items: Vec<Answer>, wanted: &Asker) -> (Vec<Answer>, Vec<Answer>)` (the partition of `AnswerBox::take`).
  - `uoterm_view::guard::{Guard, Checked, Seen, KeptGrabBags, GRAB_BAGS_FILE, TARGET_FLAG_HARMFUL, TARGET_FLAG_BENEFICIAL, QUESTION_CRIMINAL, aim_words}`. `Guard::grab_bag(&self) -> Option<u32>` and `Guard::set_grab_bag(&mut self, serial: Option<u32>) -> KeptGrabBags` return the data to save; the caller saves it (native: `kept::save`; browser: `PUT /v1/kept/watch-grab-bags.toml`). `Guard::new(kept: KeptGrabBags)`.

- [ ] **Step 1: Write the failing tests** in `crates/uoterm-view/src/act.rs`

```rust
#[cfg(test)]
mod moved_tests {
    use super::*;
    use serde_json::json;
    use uoterm_world::tool_names::{TOOL_DROP, TOOL_LIFT, TOOL_USE, ARG_HUMAN};

    #[test]
    fn a_move_lifts_then_drops() {
        let act = Act::Move { item: 0x4000_0001, amount: 5, to: DropTo::Ground { x: 10, y: 20, z: 0 } };
        let names: Vec<&str> = act.calls().iter().map(|(name, _)| *name).collect();
        assert_eq!(names, vec![TOOL_LIFT, TOOL_DROP]);
        assert!(act.is_two_step());
    }

    #[test]
    fn a_use_is_one_call() {
        let act = Act::Use(0x4000_0002);
        assert_eq!(act.calls().len(), 1);
        assert_eq!(act.calls()[0].0, TOOL_USE);
        assert!(!act.is_two_step());
    }

    #[test]
    fn a_human_call_is_marked() {
        assert_eq!(with_human(json!({"serial": 1})), json!({"serial": 1, ARG_HUMAN: true}));
    }
}
```
Use the exact variant shapes `Act::Move` and `DropTo::Ground` have in `control.rs` today; if a field name differs, use the present one.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-view moved_tests`
Expected: FAIL.

- [ ] **Step 3: Move the code**

Move `control.rs:43-830` and `:1146-1240` pure items and the pure part of `guard.rs`. `Act::Order(String, Box<WatchFrame>)` stays a variant; its `calls()` returns no call (as today the order goes through `orders::ask` in `perform`). `perform` in the window calls `with_human`. Move existing tests with the code.

- [ ] **Step 4: Run all tests and the wasm build**

Run: `cargo test --workspace && scripts/check-wasm.sh uoterm-view`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add crates/uoterm-view crates/uoterm/src
git commit -m "refactor(view): move acts and the guard"
```

---

### Task 6: Move the input rules: actions, keys, chat, steering, controller, desk, clicks, tips, video

**Files:**
- Create: `crates/uoterm-view/src/actions/{mod.rs,arguments.rs,editor.rs,journal.rs,resolve.rs,runner.rs,select.rs,view_range.rs,screenshot.rs,windows.rs}`, `src/keys/{mod.rs,chat.rs}`, `src/steer.rs`, `src/pad.rs`, `src/desk.rs`, `src/clicks.rs`, `src/tips.rs`, `src/video.rs`
- Modify (native parts only stay): `window/actions.rs`, `window/actions/{client.rs,modern.rs,screenshot.rs}`, `window/keys.rs`, `window/keys/chat.rs`, `window/steer.rs`, `window/pad.rs`, `window/desk.rs`, `window/control_ui.rs`, `window/tips.rs`, `window/video.rs`, `window/macros_ui.rs` (`play_line` moves to `actions/resolve.rs`)

**Interfaces:**
- Consumes: Tasks 1-5.
- Produces (names unchanged unless listed):
  - `uoterm_view::actions::{Group, ActionSpec, ActionId, ACTIONS, step_action, new_step, GumpOp, Switch, RangeChange, SelectHow, LocalAim, PointerClick, WindowCommand, StyleWindows}`; `arguments::*` (uses `uoterm_world::hotkeys::FIXED_HOTKEY_NAMES`); `editor::MacroEditor`; `journal::ClientJournal`; `resolve::{Context, Wait, Effect, resolve, play_line}`; `runner::MacroRunner`; `select::{candidates, select}`; `view_range::ViewRange`; `screenshot::{ScreenshotWords, stored_words, died}`; `windows::{wanted, shown_panel, switch_panel, deck_tab, character_view}` (from `actions/modern.rs:49-120`; `Tab` and `CharacterView` enums move here from `deck_ui.rs`).
  - `uoterm_view::actions::switches::{switch_words, switch_on, set_switch}` (from `actions/client.rs:88-150`), `received(&mut WatchFrame, &mut ClientJournal, &ViewRange)`, `style_cannot`.
  - `uoterm_view::keys::{Focus, KeyDispatch, Dispatched, WalkKeys, Held, WarKey, DEFAULT_KEYS, HISTORY_OLDER, HISTORY_NEWER, WAR_KEY, cannot_type, is_function_key, bound_steps, default_keys, held_step, walk_keys}`. `KeyPress` is `uoterm_view::input::KeyPress`. `Dispatched.used: Vec<(Mods, KeyName)>`. `Focus::of(chat_focused: bool, chat_empty: bool, other_field_focused: bool) -> Focus` (pure; the window computes the three flags from egui).
  - `uoterm_view::keys::chat::{PartyOrder, Spoken, Said, ChatLine, parse_line, party_line, channel_hue, typed_hue, party_order}`; `ChatLine::key(&mut self, key: ChatKey) -> ChatOut` where `ChatKey::{Text(String), Escape, Enter { shift: bool }, Paste(String)}` and `ChatOut::{None, Close, Sent(String)}` (the egui event reading of `take_keys` stays in the window and feeds `key`).
  - `uoterm_view::steer::{Movement, Steer, SEND_EVERY, RUN_DISTANCE, DEAD_ZONE, way_of, way_of_keys, way_of_mouse}`; `Steer::decide(&mut self, keys_down: &[KeyName], mouse_way: Option<(Point, Point)>, time: f64, movement: &Movement) -> Vec<Act>` (the decision logic of `steer.rs:115-151`; the window passes the keys egui reports down).
  - `uoterm_view::pad::{PadFrame, PadState, STICK_DEAD_ZONE, STICK_RUN, DEFAULT_BUTTONS, stick, stick_walk, bound_steps, default_buttons}`; `PadState::read(&mut self, sticks: [f32; 4], buttons_down: &[PadButton], profile: &Profile) -> PadFrame`; `PadButton` enum with the gilrs button names (`South`, `East`, `West`, `North`, `LeftTrigger`, ...). Native `Pad` maps gilrs to `PadButton`; the browser maps the standard Gamepad layout.
  - `uoterm_view::desk::{Zone, Split, Desk}`; `Desk::zone(&mut self, area: Area, zone: Zone)`, `Desk::zone_at(&self, at: Point) -> Option<Zone>`, `Desk::landing(&mut self, ...)` returns `Landing::{Nothing, AskAmount(Split), Act(Act)}` (the drop rules of `carry_and_land:170-210`).
  - `uoterm_view::clicks::{ChatMode, GroundClicks, act_for_click, grabbed, hint_for, beside_bar, bar_buttons}`; `GroundClicks::of(&GeneralOptions, Mods)`; `PickKind` moves here from `scene.rs:331`.
  - `uoterm_view::tips::Tips` (cache and rest timing); `uoterm_view::video::{frame_interval, FPS_MIN, FPS_MAX}`.

- [ ] **Step 1: Write the failing tests** (new behavior: the pure entry points that replace egui reads)

In `crates/uoterm-view/src/steer.rs`:
```rust
#[cfg(test)]
mod decide_tests {
    use super::*;
    use crate::input::KeyName;

    #[test]
    fn a_held_arrow_steps_then_waits_for_the_send_gap() {
        let mut steer = Steer::default();
        let movement = Movement { keys: WalkKeysOn::Arrows, ..Movement::default() };
        let up = [KeyName("ArrowUp".into())];
        let first = steer.decide(&up, None, 0.0, &movement);
        assert!(matches!(first.as_slice(), [Act::Step { .. }]));
        assert!(steer.decide(&up, None, SEND_EVERY / 2.0, &movement).is_empty());
        assert!(matches!(steer.decide(&[], None, SEND_EVERY, &movement).as_slice(), [Act::Stop]));
    }
}
```
Use the real field shapes of `Movement` after reading `steer.rs:32`; `WalkKeysOn::Arrows` stands for whatever field turns arrow walking on today (`WalkKeys { arrows: true, wasd: false }`) — write it with the real type.

In `crates/uoterm-view/src/keys/chat.rs`:
```rust
#[cfg(test)]
mod key_tests {
    use super::*;

    #[test]
    fn enter_sends_the_typed_line() {
        let mut line = ChatLine::default();
        line.key(ChatKey::Text("hail".into()));
        assert_eq!(line.key(ChatKey::Enter { shift: false }), ChatOut::Sent("hail".into()));
    }

    #[test]
    fn escape_closes_and_keeps_nothing() {
        let mut line = ChatLine::default();
        line.key(ChatKey::Text("bank".into()));
        assert_eq!(line.key(ChatKey::Escape), ChatOut::Close);
        assert_eq!(line.key(ChatKey::Enter { shift: false }), ChatOut::None);
    }
}
```
In `crates/uoterm-view/src/pad.rs`:
```rust
#[cfg(test)]
mod read_tests {
    use super::*;
    use crate::settings::Profile;

    #[test]
    fn a_stick_pushed_far_runs() {
        let mut pad = PadState::default();
        let frame = pad.read([0.0, -1.0, 0.0, 0.0], &[], &Profile::default());
        assert!(frame.walk.map(|w| w.run).unwrap_or(false));
    }
}
```
(`frame.walk` has the shape `stick_walk` returns today; use it.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-view decide_tests key_tests read_tests`
Expected: FAIL.

- [ ] **Step 3: Move the code**

Move each pure item; move its tests with it. In the window, each egui-bound function becomes a thin reader that builds the plain input and calls the view function: `Steer::run` reads `keys_down` from egui then calls `decide` and sends each `Act` with `hand.act`; `ChatLine::take_keys` reads egui events, maps them to `ChatKey`, calls `key`; `Pad::poll` reads gilrs, calls `PadState::read`; `Desk::carry_and_land` paints and calls `landing`. No logic stays in the readers.

- [ ] **Step 4: Run all tests and the wasm build**

Run: `cargo test --workspace && scripts/check-wasm.sh uoterm-view`
Expected: PASS.

- [ ] **Step 5: Check the window by eye** with the mock shard as in Task 3 Step 5: take control, walk with the arrows, send a chat line, drag the hatchet into the backpack. Expected: each works as before.

- [ ] **Step 6: Commit**
```bash
git add crates/uoterm-view crates/uoterm/src
git commit -m "refactor(view): move input and action rules"
```

---

### Task 7: Move `model/` into `uoterm-view`

**Files:**
- Create: `crates/uoterm-view/src/model/*.rs` (all 40 files)
- Modify: `crates/uoterm/src/window/model.rs` (becomes `pub use uoterm_view::model::*;` plus `pub mod host;`), create `crates/uoterm/src/window/model/host.rs` for the host-bound parts listed below
- Modify: window files that used the moved host parts

**Interfaces:**
- Consumes: Tasks 1-6.
- Produces: every present `model::*` item at `uoterm_view::model::*`, with these splits:
  - `map_item`: `pixel_at(area: Area, ...)`, `point_of(area: Area, ...) -> Point`, `land_rgba(...) -> (usize, usize, Vec<u8>)` (the pixel data of `land_image`), `UNKNOWN: Rgba`. `LandPicture::texture` (egui upload) moves to `window/model/host.rs`.
  - `health_bars`: `drag_select_allowed(&GeneralOptions, Mods)`, `select_start -> Point`, `select_layout` on `Area`, `MapAsk::{Pull { mouse: Point }, Selecting(Area), Selected(Area)}`, `Pointer { at: Point, mods: Mods }`, `MapBars::follow`, `MapDrag { from: Point }` (moved from `scene.rs:324`).
  - `places`: on `Area`/`Point`/`Vector`.
  - `fonts`: `resolve(names: &[String], wanted: &str) -> Option<String>`; `fonts_dir`, `fonts_in`, `load` move to `host.rs`.
  - `journal`: all rules stay; `JournalFile`, `save`, `stamp_now` move to `host.rs`; `Entry` time stamps take `stamp: String` from the caller.
  - `world_map`: `parse_markers_csv(&str)`, `parse_zones_json(&str)`, `markers_csv(&[Marker]) -> String`, `group_on_map`, `sextant`, `parse_goto`; `map_dir`, `load_markers`, `load_zones`, `save_markers` move to `host.rs`. `Landmarks` comes from `uoterm_world::landmarks`.
  - `reads`: `ReadCache` (cache, throttle, `want`, `failure`, `refresh`, `due(now) -> Vec<ReadKey>`, `arrived(key, Result<Value, String>, now)`); `Readings` (the thread and tokio worker) moves to `host.rs` and wraps a `ReadCache`.
  - `properties`: unchanged (`TOOL_PROPERTIES` from `uoterm_world::tool_names`).
  - `creation`: `CreationFiles` data stays; `CreationFiles::read(uopath)` moves to `host.rs` as `read_creation_files(uopath)`; `CreationFiles` gets `Serialize, Deserialize` (the browser fetches it from `/v1/data/creation`, Task 13).
  - `compare`: `ItemLayers` data stays with `Serialize, Deserialize`; `ItemLayers::load(uopath)` moves to `host.rs` as `load_item_layers(uopath)`; the browser fetches it from `/v1/data/item-layers`.
  - `house_design`: unchanged (`Rc<RefCell<...>>` is fine on wasm).
- `info_bar` uses `uoterm_view::look::notoriety_hue` — so `look.rs` moves in this task too (whole file, it is pure) — and `durability` uses `is_worn_layer`, which moves from `deck_ui.rs:371` to `uoterm_view::model::durability::is_worn_layer`.

- [ ] **Step 1: Write the failing tests** for the new pure entry points

In `crates/uoterm-view/src/model/reads.rs`:
```rust
#[cfg(test)]
mod cache_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_wanted_read_is_due_once_until_it_arrives() {
        let mut cache = ReadCache::default();
        let key = ReadKey::tool("damage_meter");
        cache.want(key.clone(), 0.0);
        assert_eq!(cache.due(0.0), vec![key.clone()]);
        assert!(cache.due(0.1).is_empty());
        cache.arrived(key.clone(), Ok(json!({"total": 3})), 0.2);
        assert_eq!(cache.value(&key), Some(&json!({"total": 3})));
    }
}
```
(Use the real `ReadKey` constructor; if `ReadKey` is an enum with a tool variant, write that variant.)

In `crates/uoterm-view/src/model/world_map.rs`:
```rust
#[cfg(test)]
mod text_tests {
    use super::*;

    #[test]
    fn markers_survive_the_csv_round_trip() {
        let text = "1424,1693,0,Britain bank,bank,green\n";
        let markers = parse_markers_csv(text);
        assert_eq!(markers_csv(&markers), text);
    }
}
```
(Match the present CSV column order of `load_markers`.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-view cache_tests text_tests`
Expected: FAIL.

- [ ] **Step 3: Move the code** as listed. Every file that reads a file or starts a thread keeps that part in `window/model/host.rs` and calls the pure part.

- [ ] **Step 4: Run all tests and the wasm build**

Run: `cargo test --workspace && scripts/check-wasm.sh uoterm-view && grep -rn "egui\|std::fs\|config_dir\|uoterm_runtime\|Instant" crates/uoterm-view/src | grep -v "^.*//" | grep -v "#\[cfg(test)\]" ; true`
Expected: tests PASS; wasm build finishes; the grep prints only lines inside test modules (check each one by eye).

- [ ] **Step 5: Commit**
```bash
git add crates/uoterm-view crates/uoterm/src
git commit -m "refactor(view): move the panel models"
```

---

### Task 8: Move the rules of the Modern panel files into `uoterm-view::ui`

**Files:**
- Create: `crates/uoterm-view/src/ui/{mod.rs,layout.rs,places.rs,launch.rs,ring.rs,deck.rs,gumps.rs,grid_clicks.rs,hud.rs,bars.rs,lists.rs,text_field.rs,theme.rs}`
- Modify (rule parts removed, they call `uoterm_view::ui`): `window/modern/layout.rs`, `window/modern/frame.rs`, `window/modern.rs` (`Launch`), `window/ring_ui.rs`, `window/deck_ui.rs`, `window/boxes_ui.rs`, `window/gump_ui.rs`, `window/modern/grid_ui.rs`, `window/hud.rs`, `window/modern/bars_ui.rs`, `window/modern/{party_ui,spells_ui,skills_ui,status_ui,agents_ui,abilities_ui,journal_ui,combat_ui,durability_ui,stats_ui,info_ui,race_ui,radar_ui}.rs`, `window/classic/text_field.rs` (moves whole), `window/theme.rs` (colors and sizes move; egui fonts and painters stay)

**Interfaces:**
- Consumes: Tasks 1-7.
- Produces:
  - `ui::layout::{Plan, Spot, first_place(window: Area, spot: Spot, size: Vector) -> Area, CASCADE_STEP, STRIP_STEP, GAP}` (whole of `modern/layout.rs` on `Area`).
  - `ui::places::{placed_rect, shown_rect, TITLE_ROW, FOLDED_HEIGHT, PANEL_MIN_WIDTH}` (from `modern/frame.rs:12,72,82` and `model/places.rs`).
  - `ui::launch::{Launch, LAUNCHES, Launch::shows(&Profile) -> bool, Launch::set(&mut Profile, bool), close_all(&mut Profile)}`.
  - `ui::ring::{RingLine, shard_lines, opens_menu, own_lines(subject, &WatchFrame) -> Vec<RingLine>}`; `RingLine { words: String, act: Option<Act>, menu: Option<u16> }` (the data each ring line has today).
  - `ui::deck::{Slot, Slot::press(&self, &WatchFrame) -> Vec<Act>, slot_choices, layer_words, wear_choices, KeptHotbars, HOTBAR_FILE ("watch-hotbar.toml"), HOTBAR_SLOTS}`. Loading and saving `KeptHotbars` stays in the window (`kept`) and in the browser (`/v1/kept`).
  - `ui::gumps::{ticked, flip, next_first_row, on_page, single_or_double, ask_waiting_name, picture_hue, shown_hue, click_box, CELL, CELL_GAP}` (from `boxes_ui.rs:85-178` and `gump_ui.rs:54-140`).
  - `ui::grid_clicks::{GridClick, grid_click(button: PointerButton, mods: Mods, double: bool, ...) -> GridClick}` (from `grid_ui.rs:615-650`), `cell_side`, `grid_size`, `first_size`, `corpse_place_id`, `grid_title`, `preview_words`, `hover_lines`.
  - `ui::hud::{Bar::follow, share, capitalized, states, WEIGHT_WARN_SHARE, activity_details, ACTIVITY_HEIGHT, VITALS_HEIGHT, PACK_HEIGHT}` (from `hud.rs:42-44,61,89-510`).
  - `ui::bars::{bar_id, bar_serial, line_count, party_buttons, hits_color, restore_bars, close_bars}` (from `bars_ui.rs:67-241`).
  - `ui::lists::{party_near, party_entries, INVITE_TILES, spell_book_words, spell_chosen, choose_school, skill_entries, next_sort, status_lines, column_room, agent_bag_items, agent_name_of, agent_dress_of_worn, agent_settings_rows, agent_list_rows, ability_slot_words, journal_waiting_words, journal_wheel_turns, journal_words_color, FilterOption, spell_color, wear_color, ping_color, info_highlight, race_change_words, race_preview_look, radar_panel_size, open_corpses}` (sources: `modern/party_ui.rs:66,85`, `modern/spells_ui.rs:47,64,86`, `modern/skills_ui.rs:52,85`, `modern/status_ui.rs:28,45`, `modern/agents_ui.rs:92,119,349-630,860`, `modern/abilities_ui.rs:58,223`, `modern/journal_ui.rs:90,99,445`, `modern/combat_ui.rs:36`, `modern/durability_ui.rs:24`, `modern/stats_ui.rs:28`, `modern/info_ui.rs:36-60`, `modern/race_ui.rs:48,63`, `modern/radar_ui.rs:35`, `modern/loot_ui.rs:41`; one function per present function, same body).
  - `ui::text_field::{TextField, FieldKey}` (whole of `classic/text_field.rs`).
  - `ui::theme::{VOID, GLASS, GLASS_EDGE, BUTTON, BUTTON_HOVER, TRACK, CHOSEN, DARK_GLASS, TEXT, TEXT_DIM, TEXT_FAINT, TEXT_SHADOW, ALARM, GOAL, WAITING, HITS, HITS_POISONED, MANA, STAM, BAR_GHOST, NOTO_SELF, SELF_FIGURE, PLATE_BACK, FLAT_*, CORPSE, PANEL_RADIUS, PANEL_PAD, SCREEN_MARGIN, ROW_GAP, EDGE_WIDTH, BAR_HEIGHT, BAR_HEIGHT_MAIN, BAR_RADIUS, PIP_HEIGHT, SIZE_TITLE, SIZE_HEADING, SIZE_BODY, SIZE_SMALL, SIZE_PLATE, notoriety_color(u8) -> Rgba, fit, fit_up_to}` as `Rgba` and `f32`; `css_tokens() -> String` (one `--name: rgba(...)` line per color and `--name: Npx` per size, used by Task 16 to write `web/src/theme.css` so the two windows never hold two copies of the colors).

- [ ] **Step 1: Write the failing tests**

In `crates/uoterm-view/src/ui/layout.rs`:
```rust
#[cfg(test)]
mod place_tests {
    use super::*;
    use crate::geom::{Area, Point, Vector};

    const WINDOW: Vector = Vector { x: 1280.0, y: 800.0 };

    #[test]
    fn every_spot_lands_inside_the_window() {
        let window = Area::from_min_size(Point::default(), WINDOW);
        for spot in Spot::every_kind() {
            let placed = first_place(window, spot, Vector::new(300.0, 200.0));
            assert!(window.contains(placed.min) && window.contains(placed.max), "{spot:?}");
        }
    }

    #[test]
    fn a_small_window_still_holds_the_journal() {
        let window = Area::from_min_size(Point::default(), Vector::new(800.0, 600.0));
        let journal = first_place(window, Spot::Journal, Vector::new(360.0, 240.0));
        assert!(window.contains(journal.max));
    }
}
```
`Spot::every_kind()` is a test helper in the same file that lists one value of each `Spot` variant (n = 0 and 3 for the cascading ones).

In `crates/uoterm-view/src/ui/theme.rs`:
```rust
#[cfg(test)]
mod css_tests {
    use super::*;

    #[test]
    fn css_tokens_carry_every_color_once() {
        let css = css_tokens();
        assert!(css.contains("--hits: rgba(222, 58, 64, 1)"));
        assert_eq!(css.matches("--void:").count(), 1);
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-view place_tests css_tests`
Expected: FAIL.

- [ ] **Step 3: Move the code** as listed; move each present test with its function. In the window, each panel calls the moved function and converts with `bridge`.

- [ ] **Step 4: Run all tests, the wasm build, and look at the Modern window**

Run: `cargo test --workspace && scripts/check-wasm.sh uoterm-view`
Then the mock-shard snapshot of Task 3 Step 5 with `--open sheet --open map`.
Expected: PASS; the panels sit where they sat before.

- [ ] **Step 5: Commit**
```bash
git add crates/uoterm-view crates/uoterm/src
git commit -m "refactor(view): move Modern panel rules"
```

---

### Task 9: Move world rules: filters, lights, predict, sky, floats, cursor, map projection, audio rules, atlas packing, art requests

**Files:**
- Create: `crates/uoterm-view/src/{filters.rs,lights.rs,predict.rs,sky.rs,floats.rs,cursor.rs,map_lay.rs,audio.rs,atlas.rs,art.rs}`
- Create: `crates/uoterm/src/art/{mod.rs,client_art.rs,figure.rs,text.rs,png.rs}` (native art moves out of `window/` so `uoterm web` can use it without the window)
- Modify: `window/{filters,lights,predict,sky,floats,cursor,map_view,audio,atlas,client_art,figure}.rs`, `window/audio/{effects,midi,score}.rs`, `window/classic/text.rs` (`UoFont`, `TextLook` move to `uoterm_view::art`; `UoFonts` moves to `crates/uoterm/src/art/text.rs`; egui `TextKit`/`TextTexture` stay), `crates/uoterm/src/main.rs` (`mod art;`), `crates/uoterm-nav/src/{tiledata.rs,multi.rs,lights.rs,anim.rs,lib.rs}` (serde derives, `AnimRules`)
- Modify: `crates/uoterm/Cargo.toml` (nothing new: `image` is already there)

**Interfaces:**
- Consumes: Tasks 1-8.
- Produces:
  - `uoterm_view::filters::*` (whole file; `Seat.offset: Vector`).
  - `uoterm_view::lights::{LIGHT_LEVEL_DARKEST, LightRules, WorldLight, world_light, light_rgb, shown_light_color, flicker, LightSource { center: Point, shape, color, strength }, add_light, overlay_pixel -> Rgba, LightCells { width, height, cells: Vec<Rgba> }, light_cells(area: Area, light: WorldLight, alternative: bool, sources: &[LightSource], zoom: f32, shape_of: impl Fn(u8) -> Option<LightShape>) -> LightCells}` (the pure grid of `LightMap::draw:325-341`). Native `LightMap::draw` uploads `LightCells`.
  - `uoterm_view::predict::*` (whole file).
  - `uoterm_view::sky::{Sky, Sky::take_in, seconds_of, scatter, effect_place, weather_drops(...) -> Vec<Drop>, lightning_flash(...) -> Option<f32>}` (painting stays native in `window/sky.rs`).
  - `uoterm_view::floats::{speech_seconds, speech_look, shortened, alpha, floating_hue, Float { words, look: TextLook, color: Rgba, ... }, Floats::take_in(&mut self, frame, time, profile, lines: impl Fn(&str, &TextLook) -> Vec<String>)}`.
  - `uoterm_view::cursor::{cursor_shape(target: bool, character: Point, mouse: Point) -> CursorShape, aura_hue, distance_to}`.
  - `uoterm_view::map_lay::{Lay, turned, unturned, zoomed, whole_tile, place_words, landmarks, zone_at, Marks, MarkLook { colors only }, mark_layout(...) -> Vec<MarkPlace>}`.
  - `uoterm_view::audio::{Step, step_sound, nearness, gain, rain_sound, rain_volume, new_cues, EffectCue, Room, room, Score, Moment, COMBAT_MUSIC, DEATH_MUSIC, MusicFile, music_file, pick_combat_track(seed: u32) -> u16}` (`pick_combat_track` replaces `rand::thread_rng` with a caller seed).
  - `uoterm_view::atlas::{ShelfPacker, Placement { x, y, width, height }, ATLAS_SIDE (4096), GUTTER, WHITE_SIDE}`; `ShelfPacker::place(width, height) -> Option<Placement>`, `ShelfPacker::reset()`, `ShelfPacker::white() -> Placement`.
  - `uoterm_view::art::{ArtRequest, Picture { width, height, rgba, anchor: Vector }, Sprite { uv: Area, width, height, anchor: Vector }, Art<T>::{Ready(T), Pending, Missing}, UoFont, TextLook, Cell, CellStatic, Stretch, ItemPaint, Pose, Paint { outline: [u8; 4], whole_hue: Option<u16> }, WorldArt}`.
  - `ArtRequest` (serde, tag `"kind"`): `Land { land_id, hue }`, `Texture { texture_id, hue }`, `Item { graphic, hue, whole_hue, border }`, `Gump { gump, hue, partial }`, `Cursor { shape, war, hue }`, `Text { text: String, look: TextLook }`, `Figure { look: WatchLook, pose: Pose, paint: Paint }`. `ArtRequest::key(&self) -> u64` (stable hash used as the cache key; replaces the `ArtKey::Text(u64)` and `ArtKey::Figure(u64)` hashing; `DefaultHasher` with fixed keys is stable within one build, which is all the caches need).
  - `WorldArt` trait (every method the scene calls on `ClientArt` today):
    ```rust
    pub trait WorldArt {
        fn has_art(&self) -> bool;
        fn sprite(&mut self, request: &ArtRequest) -> Art<Sprite>;
        fn cell(&mut self, map: u8, x: u16, y: u16) -> Art<&Cell>;
        fn take_live_map(&mut self, live: &WatchLiveMap);
        fn item_tile(&self, graphic: u16) -> Option<&uoterm_nav::ItemTile>;
        fn land_tile(&self, land_id: u16) -> Option<&uoterm_nav::LandTile>;
        fn multi_pieces(&mut self, multi: u16) -> Art<&[uoterm_nav::MultiPiece]>;
        fn shown_graphic(&self, map: u8, graphic: u16, time_ms: u64) -> u16;
        fn season_land(&self, season: u8, land_id: u16) -> u16;
        fn season_item(&self, season: u8, graphic: u16) -> u16;
        fn radar_rgb(&self, map: u8, x: u16, y: u16) -> Option<[u8; 3]>;
        fn land_z(&mut self, map: u8, x: u16, y: u16) -> Option<i8>;
        fn light_shape(&mut self, id: u8) -> Art<&uoterm_nav::LightShape>;
        fn anim(&self) -> &uoterm_nav::AnimRules;
        fn frame_count(&mut self, look: &WatchLook, action: uoterm_nav::Action) -> Art<usize>;
        fn line_height(&self, look: &TextLook) -> f32;
        fn text_lines(&self, text: &str, look: &TextLook) -> Vec<String>;
        fn text_rgb(&self, hue: u16) -> [u8; 3];
        fn gump_drawn_at(&self, gump: u16, x: usize, y: usize) -> bool;
        fn has_gump_art(&self, gump: u16) -> bool;
    }
    ```
  - `uoterm_nav::AnimRules` (serde): the tables `AnimData` holds for `equip_conv`, `deed_action`, `stance_action`, `is_person`, `mount_of`, with those methods; `AnimData::rules(&self) -> &AnimRules`. `AnimData` keeps delegating methods so present callers compile.
  - `uoterm_nav` serde derives on `ItemTile`, `LandTile`, `TileFlagSet`, `MultiPiece`, `LightShape`, `Action`, `Facing`, `CursorShape`, `HousePart`, `StartTown` (if not done in Task 2).
  - Native `crate::art::client_art::ClientArt` implements a native-only `fn picture(&self, request: &ArtRequest) -> Option<Picture>` (the CPU work of `land_sprite`, `texture_sprite`, `item_sprite`, `gump_sprite`, `cursor_sprite`, `text_sprite`, `figure_sprite`, `corpse_sprite` without the atlas), plus `fn cell_block(&mut self, map: u8, bx: u16, by: u16) -> Vec<Cell>`, `fn frame_count(...) -> Option<usize>`, and the table getters for Task 13.
  - Native `crate::window::art_host::NativeArt { client: ClientArt, atlas: Atlas, packer: ShelfPacker }` implements `WorldArt`; `sprite()` calls `client.picture()` then `atlas` upload at the `packer` placement. `Atlas` keeps only the egui texture and `set_partial`.
  - `crate::art::png::encode(picture: &Picture) -> Vec<u8>` (image crate, RGBA8).

- [ ] **Step 1: Write the failing tests**

In `crates/uoterm-view/src/atlas.rs`:
```rust
#[cfg(test)]
mod packer_tests {
    use super::*;

    #[test]
    fn pictures_fill_a_shelf_then_open_the_next() {
        let mut packer = ShelfPacker::new(64);
        let a = packer.place(30, 10).unwrap();
        let b = packer.place(30, 12).unwrap();
        let c = packer.place(30, 5).unwrap();
        assert_eq!((a.y, b.y), (a.y, a.y));
        assert!(c.y > a.y);
    }

    #[test]
    fn a_full_atlas_refuses_until_reset() {
        let mut packer = ShelfPacker::new(16);
        assert!(packer.place(64, 64).is_none());
        packer.reset();
        assert!(packer.place(8, 8).is_some());
    }
}
```
(`ShelfPacker::new(side)` keeps the white square and `GUTTER` rules of `atlas.rs:108-165`; adjust the expected `y` values to those rules once you read them — the test checks the shelf behavior, not exact pixels.)

In `crates/uoterm-view/src/art.rs`:
```rust
#[cfg(test)]
mod request_tests {
    use super::*;

    #[test]
    fn a_request_goes_on_the_wire_by_kind() {
        let request = ArtRequest::Item { graphic: 0x0F6C, hue: 0, whole_hue: false, border: None };
        let text = serde_json::to_string(&request).unwrap();
        assert!(text.contains("\"kind\":\"Item\""));
        assert_eq!(serde_json::from_str::<ArtRequest>(&text).unwrap(), request);
    }

    #[test]
    fn equal_requests_share_one_key() {
        let a = ArtRequest::Land { land_id: 3, hue: 0 };
        assert_eq!(a.key(), ArtRequest::Land { land_id: 3, hue: 0 }.key());
        assert_ne!(a.key(), ArtRequest::Land { land_id: 4, hue: 0 }.key());
    }
}
```
(`border` has the type `ItemPaint.border` has today.)

In `crates/uoterm/src/art/client_art.rs`, a test with small byte fixtures in the style of `uoterm-nav` `art.rs:201` (`words(&[u16])`): write a 2-item `art.mul`/`artidx.mul` into a temp dir, open `ClientArt`, and assert `picture(&ArtRequest::Item { graphic: 1, .. })` gives the right width, height and first RGBA pixel, and `graphic: 2` (no entry) gives `None`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-view packer_tests request_tests && cargo test -p uoterm art::client_art`
Expected: FAIL.

- [ ] **Step 3: Move the code** as listed; keep present tests with their code. `ArtKey` is removed; every user takes `ArtRequest` (no copy of the key type remains).

- [ ] **Step 4: Run all tests and the wasm build**

Run: `cargo test --workspace && scripts/check-wasm.sh uoterm-view`
Expected: PASS.

- [ ] **Step 5: Check the window by eye** with the mock shard and real client files (`uopath` set): trees, the character with gear, a light at night (`night` via the shard command if the mock has none, else skip and say so in the report).

- [ ] **Step 6: Commit**
```bash
git add crates/uoterm-view crates/uoterm crates/uoterm-nav
git commit -m "refactor(view): move world rules and art requests"
```

---

### Task 10: Split `scene.rs`: rules build a `SceneDraw`, the window only paints it

**Files:**
- Create: `crates/uoterm-view/src/scene/{mod.rs,glide.rs,build.rs,ceiling.rs,canvas.rs,pick.rs,plates.rs,overlays.rs}`
- Modify: `crates/uoterm/src/window/scene.rs` (keeps: egui `Painter` calls, `read_input` reading egui, `death_screen` painting, `Words` egui text, `make_atlas`, `*_picture` egui wrappers that now call `NativeArt`; everything else moves), `window/sky.rs`, `window/floats.rs`, `window/cursor.rs`, `window/control_ui.rs` (`draw_placing`), `window.rs`
- Test: move the present scene tests; add the draw-list tests below

**Interfaces:**
- Consumes: Tasks 1-9 (`WorldArt`, `ShelfPacker`, `Sprite`, look, filters, lights, predict, house design).
- Produces:
  - `uoterm_view::scene::SceneState` (the state `Scene` holds today minus egui: glides, fades, shows, picks, plates, light sources, zoom, peek, panels, hovered, death time, poll gap).
  - `SceneState::new() -> SceneState`, `set_poll_every(seconds: f64)`, `set_panels(Vec<Area>)`, `set_peek(Vector)`, `zoom() -> f32`, `set_zoom(f32)`, `wheel(notches: f32, ctrl: bool)`.
  - `SceneState::build(&mut self, art: &mut dyn WorldArt, view: Area, frame: &WatchFrame, time: f64, profile: &Profile, input: &SceneInput) -> SceneDraw`, where `SceneInput { mouse: Option<Point>, ctrl_shift: bool, pixels_per_point: f32 }`.
  - `SceneDraw { mesh: Mesh, picks: Vec<Pick>, plates: Vec<Plate>, light: LightCells, overlays: Mesh, steps: Vec<Step>, moving: bool, death: Option<f64> }`; `Mesh { vertices: Vec<Vertex { pos: [f32; 2], uv: [f32; 2], rgba: [u8; 4] }>, indices: Vec<u32> }` in paint order.
  - `Pick { area: Area, serial: u32, name: String, kind: PickKind }`.
  - `SceneState::thing_at(&self, at: Point) -> Option<(u32, PickKind)>`, `mobiles_in(&self, area: Area) -> Vec<u32>`, `head_of(&self, serial: u32) -> Option<Point>`, `tile_at(&self, art: &mut dyn WorldArt, frame: &WatchFrame, at: Point) -> Option<(u16, u16, i8)>`, `place_of`, `screen_of`.
  - `Plate { text: String, at: Point, color: Rgba, back: Rgba, hits: Option<(f32, Rgba)>, focus: bool }`; `scene::plates::lay_out(plates: Vec<Plate>, keep_clear: Area, zoom: f32, measure: &dyn Fn(&str) -> Vector) -> Vec<PlacedPlate>` (the overlap layout of `draw_plates:3467`, with text size from the host).
  - `scene::overlays::{walk_goal(...) -> Mesh, quest_arrow(...) -> Mesh, range_diamond(...) -> Mesh, placing_preview(...) -> Mesh, focus_ring(...) -> Mesh, arrow_at, clamp_to, range_corners}`.
  - Native: `Scene::draw(&mut self, ui, rect, frame, time, profile) -> bool` keeps its signature; inside it builds `SceneInput` from egui, calls `SceneState::build` with `NativeArt`, then paints `SceneDraw.mesh` as one `egui::Mesh` with the atlas texture, the overlays, the plates (egui text), and `LightCells` through `LightMap`.

- [ ] **Step 1: Write the failing tests** in `crates/uoterm-view/src/scene/mod.rs`

A test `WorldArt` that has no client files (`has_art() == false`) uses the flat radar colors path, so the tests need no files:
```rust
#[cfg(test)]
mod draw_tests {
    use super::*;
    use crate::art::{Art, WorldArt};
    use crate::frame::WatchFrame;
    use crate::geom::{Area, Point, Vector};
    use crate::settings::Profile;
    use serde_json::json;

    struct NoFiles;
    impl WorldArt for NoFiles { /* has_art -> false; every lookup -> Missing or None; text_lines splits on spaces */ }

    fn frame_at(x: u16, y: u16) -> WatchFrame {
        WatchFrame::from_observe(&json!({
            "self_state": {"serial": 1, "name": "Mara", "location": {"x": x, "y": y, "z": 0, "map": 0}},
            "radar": {"size": 5, "rows": ["....." , ".....", "..@..", ".....", "....."]}
        }), 0.0)
    }

    const VIEW: Area = Area { min: Point { x: 0.0, y: 0.0 }, max: Point { x: 640.0, y: 480.0 } };

    #[test]
    fn the_same_frame_builds_the_same_draw() {
        let mut a = SceneState::new();
        let mut b = SceneState::new();
        let frame = frame_at(1424, 1693);
        let draw_a = a.build(&mut NoFiles, VIEW, &frame, 1.0, &Profile::default(), &SceneInput::default());
        let draw_b = b.build(&mut NoFiles, VIEW, &frame, 1.0, &Profile::default(), &SceneInput::default());
        assert_eq!(draw_a.mesh, draw_b.mesh);
        assert!(!draw_a.mesh.indices.is_empty());
    }

    #[test]
    fn the_far_row_is_painted_before_the_near_row() {
        let mut scene = SceneState::new();
        let draw = scene.build(&mut NoFiles, VIEW, &frame_at(100, 100), 1.0, &Profile::default(), &SceneInput::default());
        let first_y = draw.mesh.vertices.first().unwrap().pos[1];
        let last_y = draw.mesh.vertices.last().unwrap().pos[1];
        assert!(first_y < last_y);
    }

    #[test]
    fn a_walk_glides_between_tiles() {
        let mut scene = SceneState::new();
        scene.set_poll_every(0.033);
        let profile = Profile::default();
        scene.build(&mut NoFiles, VIEW, &frame_at(100, 100), 0.0, &profile, &SceneInput::default());
        let moved = frame_at(101, 100);
        let half = scene.build(&mut NoFiles, VIEW, &moved, 0.2, &profile, &SceneInput::default());
        assert!(half.moving);
    }
}
```
Use the radar JSON shape `WatchFrame::from_observe` reads today (look at the present scene tests for a ready-made frame builder and reuse it instead of the literal above if one exists).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-view draw_tests`
Expected: FAIL.

- [ ] **Step 3: Move the code**

Order inside the step (each sub-step keeps `cargo test --workspace` green):
1. `Canvas` → `scene/canvas.rs` on the plain `Mesh` (methods unchanged: `vertex`, `quad`, `quad_colors`, `sprite_grid`, `shadow`, `water`, `glow`, `sitting`, `fill`, `ring`, `sprite`, `ellipse`).
2. `Glide`, `Stride`, `Waiting`, `tile_seconds`, `turned_to`, `stride_lag`, `far_apart`, `follow`, `step`, `note_arrivals`, `take_cues` → `scene/glide.rs`.
3. `ceiling_of`, `Over`, `Ceiling`, `ceiling`, `target_alpha`, `alpha_of`, `circle` → `scene/ceiling.rs`.
4. `build`, `standing`, `real_tile`, `flat_tile`, `things`, `land`, `static_art`, `ground_item`, `corpse`, `mobile`, `character`, `lay`, `figure`, `plain_figure`, `paint`, `outline`, `seat_of`, `project`, `art_rect`, `add_ground_light`, `held_lights`, `light_of`, `light_hidden`, `land_light`, `held_light_offset`, `tile_seed`, `corners`, `tiles_from` → `scene/build.rs`, calling `WorldArt` instead of `ClientArt`/`Atlas`. A sprite that is `Pending` is skipped for this frame; `Missing` takes the same path as "no art" today.
5. `Pick`, `PickKind` (already in `clicks`), `thing_at`, `mobiles_in`, `head_of`, `tile_at`, `floor_near` → `scene/pick.rs`.
6. `Plate`, `plate()`, the layout of `draw_plates` and `draw_overheads` → `scene/plates.rs`.
7. Overlays → `scene/overlays.rs`.
8. The window `Scene` becomes the painter described under Interfaces.

- [ ] **Step 4: Run all tests and the wasm build**

Run: `cargo test --workspace && scripts/check-wasm.sh uoterm-view`
Expected: PASS.

- [ ] **Step 5: Compare pictures before and after**

Before Step 3, take `--snapshot /tmp/claude-1000/scene-before.png` on the mock shard with real client files. After Step 3, take `scene-after.png` with the same frame. Compare:
```bash
python3 -c "from PIL import Image, ImageChops; a=Image.open('/tmp/claude-1000/scene-before.png').convert('RGB'); b=Image.open('/tmp/claude-1000/scene-after.png').convert('RGB'); print(ImageChops.difference(a,b).getbbox())"
```
Expected: `None` (no pixel differs). If the light flicker or a cue makes a difference, take both snapshots with `--open` off and at the same `time` of day; any remaining difference is a bug to fix before the commit. Delete the two PNGs after.

- [ ] **Step 6: Commit**
```bash
git add crates/uoterm-view crates/uoterm/src
git commit -m "refactor(view): build the scene as a draw list"
```

---

### Task 11: Live session link, cookie token and router extension in the runtime API

**Files:**
- Create: `crates/uoterm-runtime/src/api/{auth.rs,live.rs}`
- Modify: `crates/uoterm-runtime/src/api.rs` → `crates/uoterm-runtime/src/api/mod.rs` (routes, `router_for`, `serve`), `Cargo.toml` (workspace `axum = { version = "0.8", features = ["ws"] }`, `futures-util = "0.3"`), `crates/uoterm-runtime/Cargo.toml` (`futures-util.workspace = true`)
- Test: `crates/uoterm-runtime/src/api/live.rs` and `auth.rs` test modules

**Interfaces:**
- Consumes: `Runtime::get(id) -> Option<SessionHandle>`, `SessionHandle::call(ToolCall) -> ToolResult`, `TOOL_WATCH`, `uoterm_view::act::{LIFT_TO_DROP}` is not available here (runtime must not depend on the view crate); define `pub const ACT_STEP_GAP_MS: u64 = 650;` in `uoterm_world::tool_names` and make `uoterm_view::act::LIFT_TO_DROP` read it (one value).
- Produces:
  - `pub fn router_for(runtime: Runtime, token: Option<String>, local_only: bool) -> axum::Router` (today `router_with_token`, now public) and `pub fn guard_layer(...)` so the `uoterm web` router gets the same `require_bearer`.
  - `GET /v1/sessions/{id}/live` (WebSocket). Server → page messages (JSON text frames):
    - `{"kind": "frame", "watch": <watch tool result>}` — sent at connect and then whenever the `watch` result differs from the last one sent on this socket. The poll gap is `LIVE_POLL_MS = 33`.
    - `{"kind": "answer", "id": <u64>, "ok": bool, "result": <Value>}`.
    - `{"kind": "ended"}` when the session is gone.
  - Page → server messages:
    - `{"kind": "call", "id": <u64>, "tool": "<name>", "args": {...}}`.
    - `{"kind": "act", "id": <u64>, "calls": [{"tool": "...", "args": {...}}, ...]}` — the server runs the calls in order with `ACT_STEP_GAP_MS` between them, in a task that keeps running when the socket closes, and answers with the result of the last call.
    - `{"kind": "size", "size": <u16>}` — the radar size for `watch` (default `WINDOW_RADAR_SIZE_WITH_ART` = 5; the page sends 41 when it has no art).
  - `POST /v1/web/token` with `{"token": "..."}`: when it equals the server token, answers 204 with `Set-Cookie: uoterm_token=<token>; HttpOnly; SameSite=Strict; Path=/`; else 401.
  - `require_bearer` accepts `Authorization: Bearer` or the `uoterm_token` cookie.
  - `/v1/web/token` is exempt from the token check (like `/health`) but not from the loopback check.

- [ ] **Step 1: Write the failing tests**

Tests start a real server on `127.0.0.1:0` with a `Runtime` that has a mock-shard session (pattern of `manager.rs:257-277`: `MockServer::start()`, `mock_opts`, `rt.connect`, `wait_for_login`), then connect with `tokio-tungstenite` (dev-dependency `tokio-tungstenite = "0.26"`).

In `crates/uoterm-runtime/src/api/live.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockServer;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use tokio_tungstenite::tungstenite::Message;

    async fn next_json(ws: &mut WsClient) -> Value {
        loop {
            if let Message::Text(text) = ws.next().await.unwrap().unwrap() {
                return serde_json::from_str(&text).unwrap();
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn the_live_link_sends_a_frame_first() {
        let (server, addr, id, _shard) = serve_with_mock_session().await;
        let mut ws = connect_live(addr, &id, None).await;
        let first = next_json(&mut ws).await;
        assert_eq!(first["kind"], "frame");
        assert!(first["watch"].get("self_state").is_some() || first["watch"].get("me").is_some());
        drop(server);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_call_is_answered_with_its_id() {
        let (_server, addr, id, _shard) = serve_with_mock_session().await;
        let mut ws = connect_live(addr, &id, None).await;
        next_json(&mut ws).await;
        ws.send(Message::Text(json!({"kind": "call", "id": 7, "tool": "observe", "args": {}}).to_string().into())).await.unwrap();
        loop {
            let message = next_json(&mut ws).await;
            if message["kind"] == "answer" {
                assert_eq!(message["id"], 7);
                assert_eq!(message["ok"], true);
                break;
            }
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn two_links_on_one_session_both_get_frames() {
        let (_server, addr, id, _shard) = serve_with_mock_session().await;
        let mut a = connect_live(addr, &id, None).await;
        let mut b = connect_live(addr, &id, None).await;
        assert_eq!(next_json(&mut a).await["kind"], "frame");
        assert_eq!(next_json(&mut b).await["kind"], "frame");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_closed_link_still_finishes_a_started_act() {
        let (_server, addr, id, shard) = serve_with_mock_session().await;
        let mut ws = connect_live(addr, &id, None).await;
        next_json(&mut ws).await;
        let hatchet = mock_hatchet_serial(&shard);
        let backpack = mock_backpack_serial(&shard);
        ws.send(Message::Text(json!({"kind": "act", "id": 1, "calls": [
            {"tool": "lift", "args": {"serial": hatchet, "amount": 1, "human": true}},
            {"tool": "drop", "args": {"serial": hatchet, "container": backpack, "human": true}}
        ]}).to_string().into())).await.unwrap();
        drop(ws);
        tokio::time::sleep(std::time::Duration::from_millis(ACT_STEP_GAP_MS * 3)).await;
        assert!(shard.heard().iter().any(|p| p.first() == Some(&DROP_PACKET_ID)));
    }
}
```
Helpers in the same test module: `serve_with_mock_session() -> (JoinHandle, SocketAddr, String, MockServer)` (binds `127.0.0.1:0`, `axum::serve(listener, router_for(rt, None, true))`), `connect_live(addr, id, cookie: Option<&str>) -> WsClient` (`tokio_tungstenite::connect_async` with a `Host: 127.0.0.1` header and an optional `Cookie` header), `type WsClient = WebSocketStream<MaybeTlsStream<TcpStream>>`, `mock_hatchet_serial`/`mock_backpack_serial` (read from the mock's known serials in `mock.rs`; add `pub const` for them there if they are literals today), `DROP_PACKET_ID` (`0x08`, from `uoterm_protocol`'s named constant). Use the tool argument names `lift` and `drop` take in `tools/args.rs`.

In `crates/uoterm-runtime/src/api/auth.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "s3cret-token";

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_cookie_token_opens_the_live_link() {
        let (_server, addr, id, _shard) = live::tests::serve_with_mock_session_token(TOKEN).await;
        let ws = live::tests::try_connect_live(addr, &id, Some(&format!("uoterm_token={TOKEN}"))).await;
        assert!(ws.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_wrong_cookie_is_refused() {
        let (_server, addr, id, _shard) = live::tests::serve_with_mock_session_token(TOKEN).await;
        let refused = live::tests::try_connect_live(addr, &id, Some("uoterm_token=wrong")).await;
        assert_eq!(refused.unwrap_err().status(), Some(401));
    }

    #[tokio::test]
    async fn the_token_page_sets_the_cookie_only_for_the_right_token() {
        let st = state_with_token(TOKEN);
        let good = give_token(State(st.clone()), Json(TokenBody { token: TOKEN.into() })).await.into_response();
        assert_eq!(good.status(), StatusCode::NO_CONTENT);
        assert!(good.headers()["set-cookie"].to_str().unwrap().contains("HttpOnly"));
        let bad = give_token(State(st), Json(TokenBody { token: "no".into() })).await.into_response();
        assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
    }
}
```
The `serve_with_mock_session_token` helper binds `127.0.0.1:0` with `local_only = false` and the token (so the cookie path is what is checked). `try_connect_live` returns the HTTP status of a refused upgrade (map `tungstenite::Error::Http(response)` to `response.status().as_u16()`).

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-runtime api::`
Expected: FAIL (routes and helpers missing).

- [ ] **Step 3: Implement**

`live.rs`: handler `live(ws: WebSocketUpgrade, State(st), Path(id))` → 404 if `runtime.get(&id)` is `None`; else `ws.on_upgrade(move |socket| run_live(socket, handle))`. `run_live` splits the socket; one task loops every `LIVE_POLL_MS`: `handle.call(ToolCall { name: TOOL_WATCH, args: json!({"size": size}) })`, serializes the result, compares with the last string sent, sends on change; ends with `{"kind":"ended"}` when the call fails because the session ended. The read half handles `call` (answers from the same task through an mpsc to the writer), `act` (spawns a detached `tokio::spawn` that runs the calls with `tokio::time::sleep(Duration::from_millis(ACT_STEP_GAP_MS))` between them and sends the answer through the writer channel if it is still open), and `size`. Module header comment explains the 33 ms poll and why (no change signal in the session).

`auth.rs`: move `require_bearer`, `same_secret`, `host_is_loopback`, `bind_is_loopback`, `api_token_from_env` from `api.rs`; add cookie reading (`uoterm_token` from the `Cookie` header, split on `;`, trim); add `give_token` and `TokenBody`.

`mod.rs`: routes `.route("/v1/sessions/{id}/live", get(live::live))` and `.route("/v1/web/token", post(auth::give_token))`; `router_for` public.

- [ ] **Step 4: Run all tests**

Run: `cargo test -p uoterm-runtime && cargo test --workspace`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add Cargo.toml Cargo.lock crates/uoterm-runtime crates/uoterm-world crates/uoterm-view
git commit -m "feat(api): add the live session link"
```

---

### Task 12: Login link and client files for browser sessions

**Files:**
- Create: `crates/uoterm-runtime/src/api/login.rs`
- Modify: `crates/uoterm-runtime/src/api/mod.rs` (route), `crates/uoterm-runtime/src/config.rs` (`LoginQuestion::ask(&self) -> LoginAsk` and `LoginQuestion::answer(self, LoginReply) -> Result<(), LoginQuestion>`), `crates/uoterm-runtime/src/api/mod.rs` `ApiState` (add `uopath: Option<PathBuf>`, `markers: Option<PathBuf>` filled from `load_app_config(None)` in `serve`/`router_for`), `create_session` (uses `st.uopath`, `st.markers`)
- Test: `crates/uoterm-runtime/src/api/login.rs` test module

**Interfaces:**
- Consumes: `uoterm_world::login::{LoginAsk, LoginReply, CharacterRequest}` (Task 2), `LoginPicker`, `ConnectOptions`, `Runtime::connect`.
- Produces:
  - `GET /v1/login/live` (WebSocket). Page → server first message: `{"kind": "login", "host": "...", "port": 2593, "account": "...", "password": "...", "shard": null | "...", "character": null | "...", "era": null | "...", "version": null | "..."}` (same optional fields as `CreateBody`). Server → page: `{"kind": "ask", "ask": <LoginAsk>}` per question; page → server `{"kind": "reply", "reply": <LoginReply>}`; server → page at the end `{"kind": "ready", "session": "sN"}` or `{"kind": "failed", "words": "..."}`.
  - The socket closing before `ready` answers the open question with `CharacterRequest::Leave` (or index 0 is never guessed: the picker's sender drops, and the login ends with the present "screen is gone" path).
  - `create_session` and the login link both set `uopath` and `markers` from `uoterm.toml` (closes the gap the API map found).

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockServer, MOCK_CHAR};
    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_login_asks_for_the_character_and_ends_ready() {
        let shard = MockServer::start().await;
        let (addr, _server) = serve_runtime().await;
        let mut ws = connect(addr, "/v1/login/live").await;
        ws.send(text(json!({"kind": "login", "host": shard.addr.ip().to_string(), "port": shard.addr.port(),
            "account": "test", "password": "test", "era": "t2a"}))).await.unwrap();
        let ask = next_json(&mut ws).await;
        assert_eq!(ask["kind"], "ask");
        assert_eq!(ask["ask"]["kind"], "Characters");
        let slot = ask["ask"]["names"].as_array().unwrap().iter().position(|n| n == MOCK_CHAR).unwrap();
        ws.send(text(json!({"kind": "reply", "reply": {"kind": "Request", "request": {"Play": slot}}}))).await.unwrap();
        let end = next_json(&mut ws).await;
        assert_eq!(end["kind"], "ready");
        assert!(end["session"].as_str().unwrap().starts_with('s'));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_taken_name_comes_back_as_a_refusal() {
        let shard = MockServer::start().await;
        let (addr, _server) = serve_runtime().await;
        let mut ws = connect(addr, "/v1/login/live").await;
        ws.send(text(json!({"kind": "login", "host": shard.addr.ip().to_string(), "port": shard.addr.port(),
            "account": "test", "password": "test", "era": "t2a"}))).await.unwrap();
        next_json(&mut ws).await;
        let wish = json!({"name": MOCK_CHAR, "female": false, "race": 1, "strength": 60, "dexterity": 10,
            "intelligence": 10, "skills": [[0, 50], [1, 50], [2, 0]], "skin_hue": 1002, "hair": 0, "hair_hue": 0,
            "beard": 0, "beard_hue": 0, "shirt_hue": 0, "pants_hue": 0, "profession": 0, "start_city": 0, "slot": 1});
        ws.send(text(json!({"kind": "reply", "reply": {"kind": "Request", "request": {"Make": wish}}}))).await.unwrap();
        let again = next_json(&mut ws).await;
        assert_eq!(again["ask"]["kind"], "Characters");
        assert!(again["ask"]["refused"].is_string());
    }
}
```
(The serde shape of `CharacterRequest` follows its derive; if it is externally tagged, `{"Play": n}` is right. Read Task 2's derive and match it. The mock refuses a taken name with code 5 per `mock.rs`.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-runtime api::login`
Expected: FAIL.

- [ ] **Step 3: Implement** `login.rs`: on upgrade, read the first message into `LoginBody`; build `ConnectOptions` exactly as `play_options` in `crates/uoterm/src/main.rs:773-815` does, with `picker: Some(LoginPicker(tx))`. To keep one copy of that code, move the body of `play_options` into `uoterm_runtime::config::ConnectOptions::for_screen(form: ScreenLogin, cfg: &AppConfig, picker: LoginPicker) -> ConnectOptions` and make `main.rs` call it. Spawn `runtime.connect(opts)`; loop with `tokio::select!` over the picker receiver (send `ask`, store the question) and the socket (match `reply` to the stored question with `LoginQuestion::answer`) and the connect future (send `ready` or `failed`).

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add crates/uoterm-runtime crates/uoterm/src/main.rs
git commit -m "feat(api): add the login link"
```

---

### Task 13: Art, map and data routes

**Files:**
- Create: `crates/uoterm/src/web/{mod.rs,art_routes.rs,map_routes.rs,data_routes.rs}`
- Modify: `crates/uoterm/src/main.rs` (`mod web;`), `crates/uoterm/src/art/client_art.rs` (table getters), `crates/uoterm/Cargo.toml` (`axum.workspace = true`)
- Test: test modules in each route file

**Interfaces:**
- Consumes: `crate::art::{ClientArt, png::encode}`, `uoterm_view::art::{ArtRequest, Cell}`, `uoterm_runtime::api::guard_layer`.
- Produces:
  - `crate::web::WebState { art: Arc<Mutex<ClientArt>>, files_tag: String }` where `files_tag` is a hex of the `uopath` modification times of `tiledata.mul`, `art*`, `gump*`, `anim*`, `map*` (the `ETag`).
  - `crate::web::router(state: WebState) -> Router` with:
    - `POST /v1/art` body `ArtRequest` → 200 `image/png` with header `x-uoterm-anchor: <x>,<y>` and `Cache-Control: public, max-age=31536000, immutable`, `ETag: "<files_tag>-<request.key()>"`; 404 when `picture()` is `None`; 503 when there are no client files.
    - `GET /v1/map/{map}/{bx}/{by}` → JSON `Vec<Cell>` (64 cells, row by row) from `ClientArt::cell_block`; 404 outside the map.
    - `GET /v1/data/tiledata` → `{"land": [LandTile...], "items": [ItemTile...]}`; `/v1/data/multis/{id}` → `[MultiPiece]`; `/v1/data/animdata` → `{graphic: [frames]}` of `ArtCycles`; `/v1/data/anim-rules` → `AnimRules`; `/v1/data/radarcol` → `{"land": [[r,g,b]...], "items": [...]}`; `/v1/data/seasons` → `SeasonArt` tables; `/v1/data/lights/{id}` → `LightShape`; `/v1/data/cliloc` → `{number: text}`; `/v1/data/frames/{body}/{action}/{facing}/{mounted}` → `usize`; `/v1/data/creation` → `CreationFiles`; `/v1/data/item-layers` → `ItemLayers`; `/v1/data/hues-text/{hue}` → `[r,g,b]`. All with the `ETag` and cache header.
  - `ClientArt` gets `serde` table getters: `tiledata_tables()`, `radar_tables()`, `season_tables()`, `cliloc_table()`, `art_cycles_table()`; `uoterm_nav` gets the needed iterators (`ClilocData::entries()`, `TileData::lands()`, `TileData::items()`, `ArtCycles::entries()`) — small additions with a test each in `uoterm-nav`.

- [ ] **Step 1: Write the failing tests**

Fixture: the 2-item art files of Task 9 Step 1 plus a 1-block `map0.mul`/`statics0.mul`/`staidx0.mul` (8×8 land, one static) and a 2-entry `tiledata.mul`, all written into a temp dir by a helper `fixture_uopath() -> TempDir` in `crates/uoterm/src/web/mod.rs` tests (reuse `uoterm-nav`'s test builders if they are `pub(crate)` there; else make them `#[cfg(any(test, feature = "test-fixtures"))] pub` in `uoterm-nav` with a `test-fixtures` feature so no copy is made).

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn an_item_picture_comes_as_png_with_its_anchor() {
        let app = router(test_state());
        let body = serde_json::to_vec(&ArtRequest::Item { graphic: 1, hue: 0, whole_hue: false, border: None }).unwrap();
        let answer = app.oneshot(Request::post("/v1/art").header("content-type", "application/json").body(Body::from(body)).unwrap()).await.unwrap();
        assert_eq!(answer.status(), StatusCode::OK);
        assert_eq!(answer.headers()["content-type"], "image/png");
        assert!(answer.headers().contains_key("x-uoterm-anchor"));
        let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[tokio::test]
    async fn a_missing_item_is_not_found() {
        let app = router(test_state());
        let body = serde_json::to_vec(&ArtRequest::Item { graphic: 2, hue: 0, whole_hue: false, border: None }).unwrap();
        let answer = app.oneshot(Request::post("/v1/art").header("content-type", "application/json").body(Body::from(body)).unwrap()).await.unwrap();
        assert_eq!(answer.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_map_block_has_sixty_four_cells() {
        let app = router(test_state());
        let answer = app.oneshot(Request::get("/v1/map/0/0/0").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(answer.status(), StatusCode::OK);
        let cells: Vec<Cell> = serde_json::from_slice(&axum::body::to_bytes(answer.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert_eq!(cells.len(), 64);
        assert_eq!(cells.iter().filter(|c| !c.statics.is_empty()).count(), 1);
    }

    #[tokio::test]
    async fn tables_are_cached_by_the_files_tag() {
        let app = router(test_state());
        let answer = app.oneshot(Request::get("/v1/data/tiledata").body(Body::empty()).unwrap()).await.unwrap();
        assert!(answer.headers()["etag"].to_str().unwrap().contains(&test_state().files_tag));
    }
}
```
`test_state()` opens `ClientArt` on `fixture_uopath()`. Add `tower = { version = "0.5", features = ["util"] }` as a dev-dependency of `uoterm`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm web::`
Expected: FAIL.

- [ ] **Step 3: Implement** the routes. Each handler locks `art`, calls one `ClientArt` method, maps `None` to 404. PNG encode runs in `tokio::task::spawn_blocking` (the art lock is a `std::sync::Mutex`; hold it only inside the blocking task).

- [ ] **Step 4: Run all tests**

Run: `cargo test --workspace`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add crates/uoterm crates/uoterm-nav
git commit -m "feat(web): serve art, map blocks and tables"
```

---

### Task 14: Sound, profiles, kept files, fonts, screenshots, Jev routes, static page, `uoterm web`

**Files:**
- Create: `crates/uoterm/src/web/{sound_routes.rs,profile_routes.rs,jev_routes.rs,files.rs}`
- Modify: `crates/uoterm/src/web/mod.rs` (merge), `crates/uoterm/src/main.rs` (`Web` subcommand), `crates/uoterm/src/window/settings/store.rs` (expose `ProfileStore::path_of_default()`, `path_of_character(shard, name)` for the routes; no new store logic), `crates/uoterm/src/window/kept.rs` (move to `crates/uoterm/src/kept.rs` so `web` does not depend on the window; window `pub use`), `crates/uoterm/src/window/orders.rs` (move the pure parts `request`, `decide`, `group_request`, `hotkey_choices`, `pick_request`, `picked`, `sure_choice`, `things`, `hotkey_groups` and the `ASK_*`, `ACTS`, `SURE_ENOUGH` constants into a new `crates/uoterm-view/src/orders.rs`; move the async functions `ask`, `pick`, `lines_for`, `api_key`, `post` to `crates/uoterm/src/orders.rs`; the window `pub use`s both), `crates/uoterm/src/window/model/host.rs` (font dir functions used by the routes), `Cargo.toml` (`tower-http = { version = "0.6", features = ["fs"] }`), `crates/uoterm/Cargo.toml` (`tower-http.workspace = true`)
- Test: test modules in each new file, and `crates/uoterm/src/main.rs` arg test

**Interfaces:**
- Consumes: Tasks 9, 11-13; `ProfileStore`, `kept`, `orders`, `SoundData`, `MusicList`, `SoundFonts`.
- Produces:
  - `GET /v1/sound/{id}` → `audio/wav` (RIFF header + the `SoundData::samples` i16 PCM at `SOUND_SAMPLE_RATE`); `GET /v1/music/{id}` → the MP3 bytes (`audio/mpeg`) or MIDI bytes (`audio/midi`) chosen by `uoterm_view::audio::music_file`; `GET /v1/soundfont` → the sound font file bytes or 404.
  - `GET /v1/profiles/default`, `PUT /v1/profiles/default`, `GET /v1/profiles/{shard}/{character}`, `PUT /v1/profiles/{shard}/{character}`: JSON `Profile`. GET of a character without a file answers the default profile (the present `ProfileHome::follow` rule). PUT writes TOML through `kept::save_to`. Path parts go through `file_safe`.
  - `GET/PUT /v1/kept/{name}` for `name` in `KEPT_FILES = ["watch-hotbar.toml", "watch-grab-bags.toml"]` only; any other name 404. JSON in and out, TOML on disk.
  - `GET /v1/fonts` → `["name.ttf", ...]`; `GET /v1/fonts/{name}` → the font bytes; the name must be one of the listed names.
  - `POST /v1/screenshots` body `image/png` → 201 with `{"file": "Screenshot_<time>.png"}`; the body must start with the PNG signature, else 400; size limit `SCREENSHOT_MAX_BYTES = 32 * 1024 * 1024`.
  - `POST /v1/sessions/{id}/jev/order` `{"words": "...", "frame": <watch value>}` → `{"act": <Act>}` or 409 `{"error": "..."}`; `/jev/pick` `{"question": "shard"|"character"|..., "names": [...], "wish": "..."}` → `{"index": n | null}`; `/jev/lines` `{"wish": "...", "hotkeys": [...]}` → `{"lines": [...]}`. 503 when no TypeSafe key is set.
  - `crate::web::serve_page(web_dir: PathBuf) -> Router` (tower-http `ServeDir` with `index.html` fallback).
  - `uoterm web [--bind ADDR] [--web-dir DIR] [--uopath DIR]`: loads config, builds `Runtime`, merges `uoterm_runtime::api::router_for(...)` with `crate::web::router(...)` and `serve_page(...)`, all under the same guard, and serves. Defaults: `--bind` = `api_bind`; `--web-dir` = `web/dist`. It refuses a non-loopback bind without `UOTERM_API_TOKEN` (present rule, same words). It prints `Open http://<bind>/` when it starts.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/uoterm/src/web/sound_routes.rs
#[tokio::test]
async fn a_sound_comes_as_a_wave_file() {
    let app = router(test_state());
    let answer = app.oneshot(Request::get("/v1/sound/1").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(answer.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(answer.into_body(), usize::MAX).await.unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
}

// crates/uoterm/src/web/profile_routes.rs
#[tokio::test]
async fn a_saved_profile_reads_back_the_same() {
    let home = test_config_dir();
    let app = profile_router(home.path());
    let mut profile = Profile::default();
    profile.video.zoom = 1.5;
    let put = app.clone().oneshot(Request::put("/v1/profiles/127.0.0.1:2593/Mara")
        .header("content-type", "application/json").body(Body::from(serde_json::to_vec(&profile).unwrap())).unwrap()).await.unwrap();
    assert_eq!(put.status(), StatusCode::NO_CONTENT);
    let get = app.oneshot(Request::get("/v1/profiles/127.0.0.1:2593/Mara").body(Body::empty()).unwrap()).await.unwrap();
    let back: Profile = serde_json::from_slice(&axum::body::to_bytes(get.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(back, profile);
    assert!(home.path().join("profiles").join(file_safe("127.0.0.1:2593")).join("Mara.toml").exists());
}

#[tokio::test]
async fn only_known_kept_files_are_served() {
    let home = test_config_dir();
    let app = profile_router(home.path());
    let answer = app.oneshot(Request::get("/v1/kept/uoterm.toml").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(answer.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_screenshot_must_be_a_png() {
    let home = test_config_dir();
    let app = profile_router(home.path());
    let answer = app.oneshot(Request::post("/v1/screenshots").body(Body::from("not a picture")).unwrap()).await.unwrap();
    assert_eq!(answer.status(), StatusCode::BAD_REQUEST);
}
```
(Use the real zoom field name of `VideoOptions`. `Profile` needs `PartialEq`; add it if missing. `test_config_dir()` makes a temp dir; `profile_router(dir)` builds the routes with an explicit config dir — the routes take the dir from `WebState.config_dir`, which `uoterm web` fills with `config_dir()`; this keeps tests off `~/.config/uoterm`, as the memory about a past overwrite asks.)

```rust
// crates/uoterm/src/main.rs tests
#[test]
fn web_takes_a_bind_and_a_page_folder() {
    let cli = Cli::try_parse_from(["uoterm", "web", "--bind", "0.0.0.0:7733", "--web-dir", "/tmp/page"]).unwrap();
    assert!(matches!(cli.command, Command::Web { bind: Some(_), web_dir: Some(_), .. }));
}
```
(Use the real `Cli`/`Command` type names in `main.rs`.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm web:: web_takes`
Expected: FAIL.

- [ ] **Step 3: Implement** the routes and the command as listed.

- [ ] **Step 4: Run all tests, then a manual start**

Run: `cargo test --workspace`
Then:
```bash
mkdir -p /tmp/claude-1000/emptypage && echo '<!doctype html><title>t</title>' > /tmp/claude-1000/emptypage/index.html
cargo run -p uoterm -- web --web-dir /tmp/claude-1000/emptypage
```
In a second terminal: `curl -s http://127.0.0.1:7733/ && curl -s http://127.0.0.1:7733/health`
Expected: the test page and `ok`. Stop the server; delete `/tmp/claude-1000/emptypage`.

- [ ] **Step 5: Commit**
```bash
git add Cargo.toml Cargo.lock crates/uoterm
git commit -m "feat(web): add uoterm web and its file routes"
```

---

### Task 15: `uoterm-web` WebAssembly crate

**Files:**
- Create: `crates/uoterm-web/Cargo.toml`, `crates/uoterm-web/src/{lib.rs,web_art.rs,buffers.rs,out.rs,synth.rs,input.rs,panels.rs}`
- Modify: `Cargo.toml` (member; `wasm-bindgen = "=0.2.100"`, `serde-wasm-bindgen = "0.6"`, `js-sys = "0.3"`), `rustysynth` workspace dep (move from `crates/uoterm/Cargo.toml` to the workspace table)
- Test: native unit tests (`cargo test -p uoterm-web`) for everything but the `#[wasm_bindgen]` glue

**Interfaces:**
- Consumes: all of `uoterm-view`.
- Produces (JS names in camelCase via `#[wasm_bindgen(js_name = ...)]`):
  - `WebView::new(profile_json: &str) -> WebView`.
  - `WebView::frame(&mut self, watch_json: &str, now: f64)`.
  - `WebView::tick(&mut self, now: f64, width: f32, height: f32, mouse_x: f32, mouse_y: f32, has_mouse: bool) -> DrawBuffers`.
  - `DrawBuffers` getters: `positions() -> Float32Array`, `uvs() -> Float32Array`, `colors() -> Uint8Array`, `indices() -> Uint32Array`, `overlay_*` (same four), `uploads() -> JsValue` (array of `{key, x, y, width, height}` for pictures placed this frame; the JS takes the pixels from its own cache by `key`), `atlas_reset() -> bool`, `light_width()`, `light_height()`, `light_cells() -> Uint8Array`, `plates() -> JsValue` (`PlacedPlate[]`), `steps() -> JsValue`, `moving() -> bool`.
  - `WebView::input(&mut self, event_json: &str, now: f64) -> JsValue` → `OutCall[]`. `InputEvent` (serde, tag `"kind"`): `Key { key, mods, pressed, repeat }`, `Text { text }`, `PointerMove { x, y }`, `PointerDown { x, y, button, mods, double }`, `PointerUp { x, y, button, mods }`, `Wheel { notches, mods }`, `Pad { sticks: [f32; 4], buttons: Vec<PadButton> }`, `Focus { chat_focused, chat_empty, other_field_focused }`, `Panel { panel: String, action: Value }` (panel buttons: the panel sends its own small action; `panels.rs` maps it to the same `Act` the egui panel makes, calling the moved `ui::*` and `model::*` functions).
  - `OutCall` (serde, tag `"kind"`): `Act { calls: Vec<ToolCallOut>, words: String }` (one message to the live link; a one-call act is an `act` with one call), `Read { id: u64, tool: String, args: Value }`, `Jev { kind: String, body: Value }`, `SaveProfile { profile: Value }`, `SaveKept { name: String, data: Value }`, `Screenshot`, `Window { command: WindowCommand }`, `Quit`.
  - `WebView::answer(&mut self, id: u64, ok: bool, result_json: &str, now: f64)` (feeds `ReadCache` and `Answer`s).
  - `WebView::art_wanted(&mut self) -> JsValue` → `{key: string (u64 as decimal), request: ArtRequest}[]` (each key one time).
  - `WebView::art_arrived(&mut self, key: &str, width: u32, height: u32, anchor_x: f32, anchor_y: f32)` and `art_missing(&mut self, key: &str)`.
  - `WebView::data_wanted(&mut self) -> JsValue` (`string[]` of `/v1/...` paths) and `data_arrived(&mut self, path: &str, json: &str)`, `data_missing(&mut self, path: &str)`.
  - `WebView::panels(&self, now: f64) -> JsValue` → `PanelData` (one field per Modern panel; each field is the data the egui panel draws, built by the moved `model::*` and `ui::*` functions).
  - `WebView::profile(&self) -> String` (JSON) and `set_profile(&mut self, json: &str)`.
  - `synth::render_midi(midi: &[u8], sound_font: &[u8], sample_rate: u32) -> Float32Array` (stereo interleaved).
  - `WebArt` (in `web_art.rs`) implements `WorldArt`: `sprite()` returns `Ready` from its sprite map, `Pending` (and queues the request) when not seen, `Missing` after `art_missing`; `cell()` reads loaded blocks and queues `/v1/map/{map}/{bx}/{by}` for a missing block; tables load once through `data_wanted`. Blocks farther than `BLOCK_KEEP_RADIUS = 6` blocks from the player are dropped.

- [ ] **Step 1: Write the failing tests** in `crates/uoterm-web/src/web_art.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use uoterm_view::art::{Art, ArtRequest, WorldArt};

    #[test]
    fn a_missing_picture_is_asked_one_time() {
        let mut art = WebArt::default();
        let request = ArtRequest::Item { graphic: 9, hue: 0, whole_hue: false, border: None };
        assert!(matches!(art.sprite(&request), Art::Pending));
        assert_eq!(art.take_wanted().len(), 1);
        art.missing(request.key());
        assert!(matches!(art.sprite(&request), Art::Missing));
        assert!(art.take_wanted().is_empty());
    }

    #[test]
    fn an_arrived_picture_gets_an_atlas_place() {
        let mut art = WebArt::default();
        let request = ArtRequest::Land { land_id: 3, hue: 0 };
        let _ = art.sprite(&request);
        art.take_wanted();
        art.arrived(request.key(), 44, 44, 22.0, 22.0);
        assert!(matches!(art.sprite(&request), Art::Ready(_)));
        assert_eq!(art.take_uploads().len(), 1);
    }

    #[test]
    fn a_far_block_is_dropped() {
        let mut art = WebArt::default();
        art.block_arrived(0, 0, 0, vec![Cell::default(); 64]);
        art.keep_near(0, 100 * 8, 100 * 8);
        assert!(matches!(art.cell(0, 0, 0), Art::Pending));
    }
}
```
In `crates/uoterm-web/src/panels.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_hotbar_press_makes_the_same_act_as_the_window() {
        let mut view = WebView::new(&serde_json::to_string(&Profile::default()).unwrap());
        view.frame(&fixture_watch_with_backpack(), 0.0);
        let out = view.input_native(&json!({"kind": "Panel", "panel": "hotbar", "action": {"press": 0}}).to_string(), 0.0);
        let expected = Slot::default_for(0).press(&view.frame_ref().unwrap());
        assert_eq!(out_acts(&out), expected);
    }
}
```
(`input_native` returns `Vec<OutCall>` and `input` wraps it for JS, so tests run natively. `fixture_watch_with_backpack()` returns a JSON string with a self state and a backpack; `Slot::default_for` is whatever the hotbar's default slot constructor is after Task 8.)

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p uoterm-web`
Expected: FAIL.

- [ ] **Step 3: Implement** the crate. `lib.rs` keeps `SceneState`, `WebArt`, `Profile`, `Guard`, `MacroRunner`, `KeyDispatch`, `Steer`, `PadState`, `ChatLine`, `ReadCache`, `ClientJournal`, `ViewRange`, `Desk`, `Tips`, `Sky`, `Floats` and every model state the egui `WatchApp` holds (list them from `window.rs:228-275`), and runs them in the same order as `WatchApp::update` (`window.rs:423-858`), minus drawing.

- [ ] **Step 4: Run tests and the wasm build**

Run: `cargo test -p uoterm-web && scripts/check-wasm.sh uoterm-view uoterm-web`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add Cargo.toml Cargo.lock crates/uoterm-web crates/uoterm/Cargo.toml
git commit -m "feat(web): add the WebAssembly view"
```

---

### Task 16: `web/` scaffold, wasm build, network layer, token screen

**Files:**
- Create: `web/package.json`, `web/tsconfig.json`, `web/vite.config.ts`, `web/vitest.config.ts`, `web/eslint.config.js`, `web/index.html`, `web/.gitignore` (`node_modules`, `dist`, `src/wasm`, `src/fonts`, `src/theme.css`), `web/scripts/build-wasm.mjs`, `web/scripts/theme-css.mjs`, `web/src/main.tsx`, `web/src/net/{api.ts,live.ts,login.ts,art.ts}`, `web/src/screens/Token.tsx`, `web/test/{api.test.ts,live.test.ts,art.test.ts}`
- Modify: `crates/uoterm/src/main.rs` (subcommand `uoterm theme-css` that prints `uoterm_view::ui::theme::css_tokens()`; `web/scripts/theme-css.mjs` runs it to write `web/src/theme.css`, so the colors live only in Rust)

**Interfaces:**
- Consumes: Tasks 11-15.
- Produces:
  - `package.json` scripts: `"wasm": "node scripts/build-wasm.mjs"`, `"theme": "node scripts/theme-css.mjs"`, `"dev": "npm run wasm && npm run theme && vite"`, `"build": "npm run wasm && npm run theme && tsc --noEmit && vite build"`, `"test": "vitest run"`, `"lint": "eslint ."`. Dependencies: `three`, `preact`; dev: `typescript`, `vite`, `@preact/preset-vite`, `vitest`, `jsdom`, `eslint`, `typescript-eslint`, `@types/three`. Exact versions: the newest at install time, pinned with `npm install --save-exact`.
  - `vite.config.ts`: dev server proxy of `/v1` and `/health` to `http://127.0.0.1:7733` with `ws: true`.
  - `build-wasm.mjs`: runs `cargo build -p uoterm-web --release --target wasm32-unknown-unknown`, then `wasm-bindgen --target web --out-dir src/wasm ../target/wasm32-unknown-unknown/release/uoterm_web.wasm`; copies `../crates/uoterm/assets/fonts/*.ttf` and `OFL.txt` to `src/fonts/` (build output, not committed).
  - `net/api.ts`: `export async function api<T>(path: string, init?: RequestInit): Promise<T>`; on 401 it throws `TokenNeeded`; `export class TokenNeeded extends Error`; `export async function giveToken(token: string): Promise<boolean>` (`POST /v1/web/token`).
  - `net/live.ts`: `export class LiveLink { constructor(session: string, handlers: { frame(watch: unknown): void; answer(id: number, ok: boolean, result: unknown): void; ended(): void; state(s: 'open' | 'lost'): void }); send(message: LiveOut): void; close(): void }`, reconnect with backoff `RECONNECT_MS = [250, 500, 1000, 2000, 4000]` (last value repeats); messages typed `LiveIn`/`LiveOut` as in Task 11.
  - `net/login.ts`: `export function login(form: LoginForm, onAsk: (ask: LoginAsk) => Promise<LoginReply>): Promise<string>` (resolves with the session id, rejects with the words).
  - `net/art.ts`: `export class ArtFeed { constructor(view: WebView); pump(): void }` — each frame takes `view.artWanted()` and `view.dataWanted()`, fetches with at most `ART_PARALLEL = 8` requests in flight, decodes PNGs with `createImageBitmap`, keeps the pixels in `Map<string, ImageBitmap>`, calls `view.artArrived`/`artMissing`/`dataArrived`/`dataMissing`. `export function pixelsOf(key: string): ImageBitmap | undefined`.
  - `screens/Token.tsx`: one password field and a button; calls `giveToken`; on `false` shows "Wrong token."

- [ ] **Step 1: Scaffold** with the exact commands:
```bash
cd web
npm init -y
npm install --save-exact three preact
npm install --save-exact --save-dev typescript vite @preact/preset-vite vitest jsdom eslint typescript-eslint @types/three
```
Then write the config files.

- [ ] **Step 2: Write the failing tests**

`web/test/api.test.ts`:
```ts
import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, TokenNeeded } from '../src/net/api';

afterEach(() => vi.restoreAllMocks());

describe('api', () => {
  it('asks_for_the_token_again_after_401', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"error":"unauthorized"}', { status: 401 }));
    await expect(api('/v1/sessions')).rejects.toBeInstanceOf(TokenNeeded);
  });

  it('returns_the_json_body', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"sessions":["s1"]}', { status: 200 }));
    await expect(api<{ sessions: string[] }>('/v1/sessions')).resolves.toEqual({ sessions: ['s1'] });
  });
});
```
`web/test/live.test.ts`:
```ts
import { describe, expect, it, vi } from 'vitest';
import { LiveLink, RECONNECT_MS } from '../src/net/live';
import { FakeSocket } from './fake_socket';

describe('LiveLink', () => {
  it('reconnects_after_a_drop_and_reports_lost_then_open', async () => {
    vi.useFakeTimers();
    const states: string[] = [];
    FakeSocket.install();
    const link = new LiveLink('s1', { frame() {}, answer() {}, ended() {}, state: (s) => states.push(s) });
    FakeSocket.last().open();
    FakeSocket.last().drop();
    await vi.advanceTimersByTimeAsync(RECONNECT_MS[0]);
    FakeSocket.last().open();
    expect(states).toEqual(['open', 'lost', 'open']);
    link.close();
  });
});
```
`web/test/fake_socket.ts`: a small `WebSocket` stand-in with `install()`, `last()`, `open()`, `drop()`, `receive(json)` and a `sent: string[]` list (test helper only).

`web/test/art.test.ts`:
```ts
import { describe, expect, it, vi } from 'vitest';
import { ArtFeed, ART_PARALLEL } from '../src/net/art';

describe('ArtFeed', () => {
  it('keeps_at_most_the_parallel_limit_in_flight', () => {
    const wanted = Array.from({ length: 20 }, (_, i) => ({ key: String(i), request: { kind: 'Land', land_id: i, hue: 0 } }));
    const view = { artWanted: vi.fn().mockReturnValueOnce(wanted).mockReturnValue([]), dataWanted: () => [], artArrived: vi.fn(), artMissing: vi.fn(), dataArrived: vi.fn(), dataMissing: vi.fn() };
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockReturnValue(new Promise(() => {}));
    new ArtFeed(view as never).pump();
    expect(fetchSpy).toHaveBeenCalledTimes(ART_PARALLEL);
  });

  it('marks_a_404_as_missing', async () => {
    const view = { artWanted: vi.fn().mockReturnValueOnce([{ key: '9', request: { kind: 'Item', graphic: 9 } }]).mockReturnValue([]), dataWanted: () => [], artArrived: vi.fn(), artMissing: vi.fn(), dataArrived: vi.fn(), dataMissing: vi.fn() };
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 404 }));
    const feed = new ArtFeed(view as never);
    feed.pump();
    await vi.waitFor(() => expect(view.artMissing).toHaveBeenCalledWith('9'));
  });
});
```

- [ ] **Step 3: Run them to see them fail**

Run: `cd web && npm run test`
Expected: FAIL (modules missing).

- [ ] **Step 4: Implement** the files listed.

- [ ] **Step 5: Run tests, lint and build**

Run: `cd web && npm run test && npm run lint && npm run build`
Expected: PASS; `web/dist/index.html` exists.

- [ ] **Step 6: Commit**
```bash
git add web crates/uoterm/src/main.rs
git commit -m "feat(web): scaffold the browser client"
```
(`web/package-lock.json` is committed; `node_modules`, `dist`, `src/wasm`, `src/fonts`, `src/theme.css` are not.)

---

### Task 17: World renderer and input

**Files:**
- Create: `web/src/world/{renderer.ts,lights.ts,atlas.ts}`, `web/src/input/{keys.ts,pointer.ts,gamepad.ts}`, `web/src/game.ts` (the frame loop), `web/test/{keys.test.ts,atlas.test.ts,pointer.test.ts}`
- Modify: `web/src/main.tsx` (a `?session=<id>` query opens that running session with `startGame`, after a `GET /v1/sessions` check that it exists; without the query the login screen shows)

**Interfaces:**
- Consumes: Tasks 15-16 (`WebView`, `DrawBuffers`, `LiveLink`, `ArtFeed`, `pixelsOf`).
- Produces:
  - `world/renderer.ts`: `export class WorldRenderer { constructor(canvas: HTMLCanvasElement); draw(buffers: DrawBuffers): void; resize(width: number, height: number, pixelRatio: number): void; dispose(): void }`. One `THREE.WebGLRenderer`, one `THREE.OrthographicCamera(0, width, 0, height, -1, 1)` (y down, pixel units, the same coordinates `SceneDraw` uses), one `THREE.Mesh` with a `BufferGeometry` (`position` 2D via a custom `ShaderMaterial`, `uv`, `color` as normalized `Uint8` RGBA, index `Uint32`), `transparent: true`, `depthTest: false`, premultiplied alpha blending (`THREE.CustomBlending`, `OneFactor`, `OneMinusSrcAlphaFactor`) to match egui's premultiplied colors. A second mesh for overlays, drawn after the light overlay.
  - `world/atlas.ts`: `export class AtlasTexture { readonly texture: THREE.Texture; upload(list: Upload[], pixelsOf: (key: string) => ImageBitmap | undefined): void; reset(): void }` — a 4096² texture filled by `renderer.copyTextureToTexture` from small textures made of each `ImageBitmap`, at the `x, y` the wasm packer gave; `NearestFilter` (the Rust atlas uses `TextureOptions::NEAREST`).
  - `world/lights.ts`: `export class LightLayer { draw(width: number, height: number, cells: Uint8Array): void }` — a `DataTexture` of the cells stretched over the view with multiply blending, the same as `LightMap::draw` paints.
  - `input/keys.ts`: `export function keyName(event: KeyboardEvent): string | null` — logical key (`event.key`) to the egui name: letters → upper case (`'a'` → `'A'`), digits → `'Num0'`..`'Num9'`, `'ArrowUp'` → `'ArrowUp'`, `'Escape'`, `'Enter'`, `'Tab'`, `' '` → `'Space'`, `'F1'`..`'F35'`, `'Backspace'`, `'Delete'`, `'Insert'`, `'Home'`, `'End'`, `'PageUp'`, `'PageDown'`, punctuation by the egui names (`'Minus'`, `'Plus'`, `'Equals'`, `'Comma'`, `'Period'`, `'Slash'`, `'Backslash'`, `'Semicolon'`, `'Quote'`, `'Backtick'`, `'OpenBracket'`, `'CloseBracket'`); anything else `null`. Numpad keys by `event.code` (`'Numpad0'` → `'Num0'`) only when `event.key` is not a digit. The full table lives in one `const EGUI_KEY: Record<string, string>` taken from egui's `Key::name()` list.
  - `export function mods(event: KeyboardEvent | MouseEvent): { ctrl: boolean; alt: boolean; shift: boolean; command: boolean }` — `command` is `metaKey` on macOS and `ctrlKey` elsewhere (egui's rule).
  - `input/pointer.ts`: mouse down/up/move/dblclick/wheel/contextmenu (prevented) and touch (one finger = primary, long press `LONG_PRESS_MS = 500` = secondary, two-finger pinch = wheel) → `InputEvent`s.
  - `input/gamepad.ts`: `navigator.getGamepads()` each frame; standard mapping buttons 0-3 → `South`, `East`, `West`, `North`, 4/5 → `LeftShoulder`/`RightShoulder`, 6/7 → `LeftTrigger`/`RightTrigger`, 8/9 → `Select`/`Start`, 10/11 → `LeftThumb`/`RightThumb`, 12-15 → `DPadUp`/`DPadDown`/`DPadLeft`/`DPadRight`; axes 0-3 → sticks.
  - `game.ts`: `export function startGame(session: string, canvas: HTMLCanvasElement, profile: unknown): GameHandle` — creates `WebView`, `LiveLink` (frames → `view.frame`, answers → `view.answer`), `ArtFeed`, `WorldRenderer`; `requestAnimationFrame` loop: `pump`, `tick`, `draw`, send each `OutCall` (`Act` → live `act` message; `Read` → live `call`; `Jev` → `POST /v1/sessions/{id}/jev/{kind}` then `view.answer`; `SaveProfile` → `PUT /v1/profiles/...`; `SaveKept` → `PUT /v1/kept/...`), frame pacing by `frame_interval` from the profile (wasm export `frameIntervalMs(focused)`).

- [ ] **Step 1: Write the failing tests**

`web/test/keys.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
import { keyName, mods } from '../src/input/keys';

const press = (key: string, code: string, init: KeyboardEventInit = {}) => new KeyboardEvent('keydown', { key, code, ...init });

describe('keyName', () => {
  it('maps_the_logical_key_not_the_key_position', () => {
    expect(keyName(press('a', 'KeyQ'))).toBe('A');
  });
  it('maps_function_and_arrow_keys_to_egui_names', () => {
    expect(keyName(press('F1', 'F1'))).toBe('F1');
    expect(keyName(press('ArrowUp', 'ArrowUp'))).toBe('ArrowUp');
    expect(keyName(press(' ', 'Space'))).toBe('Space');
  });
  it('maps_digits_to_num_names', () => {
    expect(keyName(press('1', 'Digit1'))).toBe('Num1');
  });
  it('ignores_keys_egui_does_not_know', () => {
    expect(keyName(press('Dead', 'BracketLeft'))).toBeNull();
  });
  it('reads_ctrl_as_command_off_mac', () => {
    expect(mods(press('a', 'KeyA', { ctrlKey: true })).command).toBe(true);
  });
});
```
`web/test/pointer.test.ts`:
```ts
import { describe, expect, it, vi } from 'vitest';
import { attachPointer, LONG_PRESS_MS } from '../src/input/pointer';

describe('pointer', () => {
  it('turns_a_long_touch_into_a_secondary_press', () => {
    vi.useFakeTimers();
    const events: unknown[] = [];
    const el = document.createElement('div');
    attachPointer(el, (e) => events.push(e));
    el.dispatchEvent(new TouchEvent('touchstart', { touches: [{ clientX: 10, clientY: 20, identifier: 0, target: el } as unknown as Touch] }));
    vi.advanceTimersByTime(LONG_PRESS_MS);
    expect(events).toContainEqual(expect.objectContaining({ kind: 'PointerDown', button: 'Secondary' }));
  });
});
```
`web/test/atlas.test.ts`: a test with a fake renderer object that records `copyTextureToTexture` calls: `upload([{key:'1',x:4,y:8,width:2,height:2}], () => bitmap)` records one copy at `(4, 8)`; a key with no bitmap records none.

- [ ] **Step 2: Run them to see them fail**

Run: `cd web && npm run test`
Expected: FAIL.

- [ ] **Step 3: Implement** the files listed.

- [ ] **Step 4: Run tests, lint, build; then look at it**

Run: `cd web && npm run test && npm run lint && npm run build`
Then start the mock shard and `cargo run -p uoterm -- web` (with `uopath` set), open `http://127.0.0.1:7733/` in the browser pane, open the running session with `?session=s1` (a feature of `main.tsx`: it reopens a running session after a page reload, so a reload never needs a new login), and take a screenshot.
Expected: the world shows the land, the tree and the character in the same place as the Rust window's snapshot of the same frame.

- [ ] **Step 5: Commit**
```bash
git add web
git commit -m "feat(web): draw the world with Three.js"
```

---

### Task 18: Login and character creation screens

**Files:**
- Create: `web/src/screens/{Login.tsx,Picking.tsx,Characters.tsx,Creation.tsx}`, `web/src/screens/login_state.ts`, `web/test/{login_state.test.ts,creation.test.tsx}`
- Modify: `web/src/main.tsx` (screen flow: token → login → game), `crates/uoterm-web/src/lib.rs` (exports for creation: `CreationView` wrapping `uoterm_view::model::creation::Creation` with the same methods the egui `creation_ui.rs` calls; the rules `plain_words`, `first_sentence`, `profession_name`, `name_case`, `name_rules` move to `uoterm_view::model::creation` in this task if Task 8 did not take them)
- Modify: `crates/uoterm/src/web/profile_routes.rs`: `GET /v1/logins` → the `LoginStore::standard()` rows (name, host, port, account, shard, character; never a password), `PUT /v1/logins/{name}` (the same save `uoterm play` does)

**Interfaces:**
- Consumes: `login()` from `net/login.ts`, `LoginAsk`/`LoginReply` JSON (Task 12), `CreationFiles` from `/v1/data/creation`.
- Produces:
  - `Login.tsx`: saved login list, fields host, port, account, password (type `password`, never stored in the browser), shard, character; Connect button; error line. The same fields and order as `login_ui.rs`.
  - `Picking.tsx`: list of names; click picks.
  - `Characters.tsx`: slots, Play, Delete (asks twice, as in `login_ui.rs`), New character, Leave; shows `refused` words.
  - `Creation.tsx`: the full creation flow of `creation_ui.rs` driven by `CreationView` (steps, stats, skills, hues, hair, profession, town, name rules).
  - `login_state.ts`: `export function nextScreen(ask: LoginAsk): 'picking' | 'characters'` and the reply builders `pick(index)`, `play(slot)`, `remove(slot)`, `make(wish)`, `leave()`.

- [ ] **Step 1: Write the failing tests**

`web/test/login_state.test.ts`:
```ts
import { describe, expect, it } from 'vitest';
import { nextScreen, play, remove } from '../src/screens/login_state';

describe('login state', () => {
  it('shows_the_character_list_for_a_characters_question', () => {
    expect(nextScreen({ kind: 'Characters', names: ['Mara'], refused: null, choices: { towns: [], features: 0, list_flags: 0 } })).toBe('characters');
  });
  it('builds_the_wire_replies_the_server_reads', () => {
    expect(play(2)).toEqual({ kind: 'Request', request: { Play: 2 } });
    expect(remove(1)).toEqual({ kind: 'Request', request: { Delete: 1 } });
  });
});
```
(Match the `CharacterRequest` serde shape from Task 2.)

`web/test/creation.test.tsx`: render `Creation` with a `CreationView` fake whose `nameFault('M')` returns `'TooShort'` and assert the fault words appear and the Next button is disabled.

- [ ] **Step 2: Run them to see them fail** — `cd web && npm run test` — FAIL.

- [ ] **Step 3: Implement.**

- [ ] **Step 4: Run tests, lint, build; then live**

Run: `cd web && npm run test && npm run lint && npm run build`
Then with the mock shard: log in from the page, see the character list, make a character with a taken name (refusal shows), play `Mara`, reach the world.
Expected: each step works.

- [ ] **Step 5: Commit**
```bash
git add web crates/uoterm crates/uoterm-web crates/uoterm-view
git commit -m "feat(web): add login and character creation"
```

---

### Task 19: Modern panels, part 1 (play core)

**Files:**
- Create: `web/src/panels/{Frame.tsx,ControlBar.tsx,ChatLine.tsx,Report.tsx,Activity.tsx,Vitals.tsx,Pack.tsx,Near.tsx,TargetBar.tsx,Journal.tsx,Radar.tsx,Hotbar.tsx,Sheet.tsx,Ring.tsx,Tooltip.tsx,Plates.tsx,Question.tsx,Launcher.tsx,TitleBar.tsx,Panels.tsx}`, `web/src/panels/drag.ts`, `web/test/panels/*.test.tsx`
- Modify: `crates/uoterm-web/src/panels.rs` (one `PanelData` field and one `Panel` action mapper per panel in this task)

**Interfaces:**
- Consumes: `WebView.panels(now)`, `WebView.input({kind: 'Panel', ...})`, `pixelsOf` for item and gump pictures (a panel picture is an `ArtRequest` the panel data names; the panel shows it with a `<canvas>` drawn from the `ImageBitmap`).
- Produces: one Preact component per Modern panel, each a pure function of its `PanelData` field plus a `send(action)` callback. Rules stay in Rust: a component may format text with CSS only, never decide what to show or what an action does.
  - `Frame.tsx`: title drag, size grip, lock, fold, close; reports `{panel, action: {place: {x, y, w, h}}}`, `{fold: bool}`, `{lock: bool}`, `{close: true}`, `{reset: true}` (title double-click); the wasm side applies `ui::places` and saves the profile (`SaveProfile` out call), so the Rust rules decide the place.
  - `Panels.tsx`: lays each open panel at the `Area` the panel data gives (from `ui::layout::first_place` / `ui::places::placed_rect`).
  - `ControlBar.tsx`: Take/Give back, War, Stop, Help, chat mode, quit; button list from `clicks::bar_buttons`.
  - `ChatLine.tsx`: input sends `Text`, `Enter`, `Escape` as `InputEvent`s (Task 15) so `ChatLine::key` decides; focus state sent as `Focus`.
  - `Report.tsx`, `Question.tsx` (Yes/No of `Guard`), `Tooltip.tsx` (`Tips` lines), `Ring.tsx` (`ui::ring` lines), `Plates.tsx` (`PlacedPlate[]` from `DrawBuffers.plates()` as absolutely placed divs), `Activity.tsx`, `Vitals.tsx`, `Pack.tsx` (from `ui::hud`), `Near.tsx` and `TargetBar.tsx` (`ui::bars`, `model::health_bars`), `Journal.tsx` (`model::journal`, tabs, search, filters), `Radar.tsx` (radar picture via `ArtRequest`-free RGBA from a `radar_rgba` panel field: the wasm side builds it with `map_lay` and `WorldArt::radar_rgb`), `Hotbar.tsx` (`ui::deck::Slot`), `Sheet.tsx` (worn, status, skills, spells, party tabs; `ui::deck`, `ui::lists`, `model::status`, `model::skills`, `model::spell_data`, `model::party`), `Launcher.tsx` (`ui::launch`), `TitleBar.tsx` (`model::info_bar::title_words`).
  - `drag.ts`: HTML drag of an item from a panel into a panel zone or onto the canvas: reports `PointerDown`/`PointerMove`/`PointerUp` with the zone the drop lands in (`Zone` ids the panel data gives), so `Desk::landing` decides.

- [ ] **Step 1: Write the failing tests** (one per component; the pattern is the same for each — render with fixed panel data, check the text, click, check the action sent). Example `web/test/panels/Hotbar.test.tsx`:
```tsx
import { render, fireEvent } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Hotbar } from '../../src/panels/Hotbar';

describe('Hotbar', () => {
  it('sends_the_slot_index_on_click', () => {
    const send = vi.fn();
    const data = { slots: [{ words: 'Heal', key: '1', picture: null }, { words: '', key: '2', picture: null }] };
    const { getByText } = render(<Hotbar data={data} send={send} />);
    fireEvent.click(getByText('Heal'));
    expect(send).toHaveBeenCalledWith({ press: 0 });
  });
});
```
Example `web/test/panels/Frame.test.tsx`:
```tsx
it('reports_a_title_drag_as_a_new_place', () => {
  const send = vi.fn();
  const { getByText } = render(<Frame title="Journal" area={{ x: 10, y: 10, w: 300, h: 200 }} send={send}><p>x</p></Frame>);
  const title = getByText('Journal');
  fireEvent.pointerDown(title, { clientX: 20, clientY: 15 });
  fireEvent.pointerMove(window, { clientX: 70, clientY: 45 });
  fireEvent.pointerUp(window, { clientX: 70, clientY: 45 });
  expect(send).toHaveBeenLastCalledWith({ place: { x: 60, y: 40, w: 300, h: 200 } });
});
```
Add `@testing-library/preact` as an exact dev dependency. In Rust, `crates/uoterm-web/src/panels.rs` gets one native test per panel action mapper, in the style of Task 15's hotbar test: the same input gives the same `Act` the egui panel makes.

- [ ] **Step 2: Run them to see them fail** — `cd web && npm run test && cargo test -p uoterm-web` — FAIL.

- [ ] **Step 3: Implement** each panel. CSS uses only the tokens in `src/theme.css` (Task 16) and the Barlow fonts in `src/fonts/` (`Barlow-Medium` text, `BarlowCondensed-SemiBold` titles). Panel opacity and UI scale come from the profile through CSS variables set on the root.

- [ ] **Step 4: Run tests, lint, build; then live**

Run: `cd web && npm run test && npm run lint && npm run build && cd .. && cargo test -p uoterm-web`
Then on the mock shard, side by side with the Rust window (`uoterm play` and the page on the same session through `?session=`): take control, walk, chop the tree with the hatchet from the hotbar, read the journal, open the sheet tabs, right-click the tree for the ring, drag logs from the pack to the ground.
Expected: each works, and the panels sit where the Rust window puts them.

- [ ] **Step 5: Commit**
```bash
git add web crates/uoterm-web
git commit -m "feat(web): add the core Modern panels"
```

---

### Task 20: Modern panels, part 2 (every other panel and window)

**Files:**
- Create: `web/src/panels/{Grid.tsx,Loot.tsx,Shop.tsx,Trade.tsx,OldMenu.tsx,Book.tsx,Board.tsx,Paperdoll.tsx,Entry.tsx,Race.tsx,Tip.tsx,Dye.tsx,HuePicker.tsx,ShardGump.tsx,Build.tsx,Chat.tsx,WorldMap.tsx,Markers.tsx,MapItem.tsx,Profile.tsx,Macros.tsx,Options.tsx,KeyCapture.tsx,Agents.tsx,Abilities.tsx,Buffs.tsx,Combat.tsx,Counters.tsx,Dps.tsx,Durability.tsx,InfoBar.tsx,PartyInvite.tsx,Stats.tsx,QuestArrow.tsx,Split.tsx}`, `web/test/panels/*.test.tsx`
- Modify: `crates/uoterm-web/src/panels.rs`

**Interfaces:**
- Consumes: as Task 19.
- Produces: one component per panel, same rule as Task 19 (data in, small action out, rules in Rust). The source of each panel's data and actions:

| Component | Rust source of data and actions |
|---|---|
| `Grid`, `Loot`, `Split` | `model::grid`, `model::loot`, `model::clicks`, `model::compare`, `model::highlight`, `model::properties`, `ui::grid_clicks`, `desk::Split` |
| `Shop`, `Trade` | `model::deals` (`stepped`, `typed_gold`, `Cart`); double click takes one, Shift takes all |
| `OldMenu`, `Book`, `Board`, `Paperdoll`, `Entry`, `Race`, `Tip`, `Dye`, `HuePicker` | `model::pages`, `model::dolls`, `model::asked`, `model::race_change`, `model::hue_grid`, `ui::lists::race_*` |
| `ShardGump` | `frame.gump_layouts` (`uoterm_world::GumpLayout`) and `ui::gumps`; each gump element is HTML: pictures by `ArtRequest::Gump`, HTML text by `UoFonts` text pictures (`ArtRequest::Text`) or by the `html_lines` rules (moved with `classic/text.rs` pure parts in Task 9), text fields by `ui::text_field::TextField` driven through `Panel` actions |
| `Build`, `Chat` | `model::house_design`, `model::chat` |
| `WorldMap`, `Markers` | `map_lay`, `model::world_map` (markers come from `GET /v1/kept/markers` — add `"markers.csv"` and the zone file names the window reads from `world_map::map_dir` to `KEPT_FILES` in Task 14's route as read-only entries, with a test that PUT on them is 405), world map picture tiles from a new `GET /v1/map-picture/{map}/{tx}/{ty}` route (PNG of `radar_rgb` per tile block, `MAP_TILE_SIDE = 256`; add it to `crates/uoterm/src/web/map_routes.rs` with a test, built from the same `grow_world` sampling rules moved to `map_lay` in Task 9) |
| `MapItem`, `Profile` | `model::map_item` (`land_rgba`), profile tools |
| `Macros` | script tools and `/jev/lines` |
| `Options`, `KeyCapture` | `settings::table::rows_on`, `model::options_draft`, `actions::editor::MacroEditor`; key capture uses `keyName` and gamepad capture uses `gamepad.ts` |
| `Agents` | `model::agents`, `ui::lists::agent_*` |
| `Abilities`, `Buffs`, `Combat`, `Counters`, `Dps`, `Durability`, `InfoBar`, `PartyInvite`, `Stats`, `QuestArrow` | `model::abilities`, `model::buffs`, `model::casting`, `model::cooldowns`, `model::counters`, `model::dps`, `model::durability`, `model::info_bar`, `model::party`, `model::stats`, `ui::lists` |

- [ ] **Step 1: Write the failing tests** — one component test per panel in the Task 19 pattern, plus one native Rust test per action mapper. Two examples that pin rules the spec names:

`web/test/panels/Shop.test.tsx`:
```tsx
it('sends_take_one_on_double_click_and_take_all_with_shift', () => {
  const send = vi.fn();
  const data = { items: [{ serial: 5, words: 'Bandage', price: 2, amount: 100, picture: null }], cart: [], total: 0 };
  const { getByText } = render(<Shop data={data} send={send} />);
  fireEvent.dblClick(getByText('Bandage'));
  expect(send).toHaveBeenLastCalledWith({ take: 5, all: false });
  fireEvent.dblClick(getByText('Bandage'), { shiftKey: true });
  expect(send).toHaveBeenLastCalledWith({ take: 5, all: true });
});
```
`crates/uoterm-web/src/panels.rs`:
```rust
#[test]
fn a_shift_take_puts_the_whole_stack_in_the_cart() {
    let mut view = view_with_shop(&[(5, 100)]);
    view.input_native(&json!({"kind": "Panel", "panel": "shop", "action": {"take": 5, "all": true}}).to_string(), 0.0);
    assert_eq!(view.cart_amount(5), 100);
}
```

- [ ] **Step 2: Run them to see them fail** — FAIL.

- [ ] **Step 3: Implement** each panel and route listed.

- [ ] **Step 4: Run tests, lint, build; then live** on the mock shard for the panels the mock can drive (grid, loot, paperdoll, journal, world map, options, macros, counters) and on a real private shard for shop, trade, gumps, books, boards, house design, chat, party (per the live-test memory: run them back to back; stop only for a real blocker).
Expected: each panel matches the Rust window's Modern panel for the same frame.

- [ ] **Step 5: Commit**
```bash
git add web crates/uoterm-web crates/uoterm
git commit -m "feat(web): add every other Modern panel"
```

---

### Task 21: Sound, music, screenshots, video options, controller

**Files:**
- Create: `web/src/audio/{player.ts,music.ts}`, `web/src/screenshot.ts`, `web/test/{player.test.ts,screenshot.test.ts}`
- Modify: `web/src/game.ts`, `crates/uoterm-web/src/lib.rs` (`audio_out(now) -> JsValue`: the sounds and music changes of this frame, from `uoterm_view::audio` rules — `new_cues`, `Score::follow`, `step_sound`, `nearness`, `gain`, `room`, rain), `crates/uoterm-web/src/synth.rs`

**Interfaces:**
- Consumes: `/v1/sound/{id}`, `/v1/music/{id}`, `/v1/soundfont`, `OutCall::Screenshot`, `VideoOptions`.
- Produces:
  - `AudioOut` (serde, tag `"kind"`): `Effect { sound: u16, volume: f32, replace: Option<u64> }`, `Stop { voice: u64 }`, `Music { track: u16, volume: f32, repeat: bool }`, `MusicStop`, `Rain { volume: f32 }`.
  - `audio/player.ts`: `export class Player { play(out: AudioOut[]): void; setFocused(focused: boolean): void }` — Web Audio `AudioContext` (resumed on the first user gesture, as browsers ask), sound buffers cached by id, voice ids from the wasm `room` rule.
  - `audio/music.ts`: MP3 tracks through `decodeAudioData`; MIDI tracks through `render_midi` (wasm) with the sound font from `/v1/soundfont`, played as an `AudioBuffer`.
  - `screenshot.ts`: `export async function takeScreenshot(canvas: HTMLCanvasElement, overlay: HTMLElement): Promise<string>` — renders the WebGL canvas (with `preserveDrawingBuffer: false`, read right after a draw) plus the panel layer into one PNG with an offscreen canvas, then `POST /v1/screenshots`; returns the file name for the journal line `stored_words`.
  - Video: frame rate cap from `frame_interval` (focused and not focused), UI scale as CSS `zoom` on the panel layer and as the scene zoom, panel opacity as a CSS variable. Window mode (fullscreen) uses `document.documentElement.requestFullscreen()` when the profile asks; the browser allows it only after a user gesture, so the Options page button calls it.
  - Gamepad: `gamepad.ts` from Task 17 feeds `Pad` events each frame; the stick moves a soft pointer drawn by the renderer (the browser cannot move the real mouse).

- [ ] **Step 1: Write the failing tests**

`web/test/player.test.ts` (with a fake `AudioContext`): `play([{kind:'Effect', sound: 3, volume: 0.5, replace: null}])` fetches `/v1/sound/3` one time and starts one source at gain 0.5; a second play of sound 3 does not fetch again.
`web/test/screenshot.test.ts`: `takeScreenshot` posts a body that starts with the PNG signature and resolves with the file name from the answer.
`crates/uoterm-web/src/lib.rs` native test:
```rust
#[test]
fn a_new_sound_cue_is_played_once() {
    let mut view = WebView::new(&serde_json::to_string(&Profile::default()).unwrap());
    view.frame(&watch_with_cues(&[(1, 0x2E)]), 0.0);
    view.frame(&watch_with_cues(&[(1, 0x2E), (2, 0x57)]), 0.1);
    let out = view.audio_out_native(0.1);
    assert_eq!(effects(&out), vec![0x57]);
}
```
(First frame plays nothing, per `new_cues`.)

- [ ] **Step 2: Run them to see them fail** — FAIL.

- [ ] **Step 3: Implement.**

- [ ] **Step 4: Run tests, lint, build; then live**: hear footsteps when walking on the mock shard; hear region music; take a screenshot with the bound key and find it in `~/.config/uoterm/screenshots/`.

- [ ] **Step 5: Commit**
```bash
git add web crates/uoterm-web
git commit -m "feat(web): add sound, screenshots and controller"
```

---

### Task 22: Full live check, docs, cleanup

**Files:**
- Modify: `README.md` (section "Web client": build with `cd web && npm install && npm run build`; start with `uoterm web`; home network with `UOTERM_API_TOKEN` and `--bind 0.0.0.0:7733`; Modern only), `docs/AGENT_API.md` (the new routes, with the message shapes of Tasks 11-12), `AGENTS.md` (where the shared rules live: a UI rule goes in `crates/uoterm-view`, never in a window), `DESIGN.md` (the web client section: same theme tokens, generated from `ui::theme`)

- [ ] **Step 1: Run every finish check**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/check-wasm.sh uoterm-view uoterm-web
cd web && npm run lint && npm run test && npm run build && cd ..
```
Expected: all PASS.

- [ ] **Step 2: Real shard session in the browser** (a private shard, per the assistant-compliance memory): log in from the page, play 10 minutes with every panel group of Tasks 19-20 used at least once, with sound on. Then reload the page during a drag (Review Focus 1), open a second tab on the same session (Review Focus 2), and open the page from a phone on the home network with the token (Review Focus 3).
Expected: no error lines in the browser console (`read_console_messages` with `onlyErrors`), no `ERROR` lines in the `uoterm web` log, and each Review Focus case behaves as written.

- [ ] **Step 3: Side-by-side pictures**: Rust window `--snapshot` and a page screenshot of the same frame with the same profile, both saved in `/tmp/claude-1000/`, shown to the user, then deleted.

- [ ] **Step 4: Update the docs** listed under Files.

- [ ] **Step 5: Clean up**

```bash
cargo clean
```
Stop the mock shard, `uoterm web`, and any Vite server. Remove scratch files in `/tmp/claude-1000/` made by this build.

- [ ] **Step 6: Commit**
```bash
git add README.md docs AGENTS.md DESIGN.md
git commit -m "docs(web): document the browser client"
```

---

## Self-review record

- **Spec coverage:** architecture → Tasks 1-10, 15; API routes → 11 (live, token), 12 (login, uopath), 13 (art, map, data), 14 (sound, profiles, kept, fonts, screenshots, Jev, page, command); `web/` parts → 16-21; scope list → 17 (world, control), 19-20 (panels, shard windows), 18 (login, creation), 21 (sound, controller, screenshots, video); error handling → 11 (ended, reconnect), 13 (404 art), 16 (401, reconnect), 17 (missing art draws radar color via `Art::Missing`); testing → each task, wasm builds, live checks in 17-22.
- **Placeholders:** none. Where a present name must be read from the code (a variant field, a CSV column order), the step names the file and line to read it from.
- **Type names used across tasks:** `Point`, `Vector`, `Area`, `Rgba`, `Mods`, `KeyName` (Task 1); `WatchFrame::from_observe(&Value, f64)` (Task 3); `Act::calls`, `with_human`, `LIFT_TO_DROP` reading `ACT_STEP_GAP_MS` (Tasks 5, 11); `ArtRequest`, `Art<T>`, `WorldArt`, `ShelfPacker` (Task 9); `SceneState::build -> SceneDraw` (Task 10); `LoginAsk`, `LoginReply` (Tasks 2, 12, 18); `WebView`, `OutCall`, `InputEvent`, `DrawBuffers`, `AudioOut` (Tasks 15, 17, 19-21).
- **Review Focus:** five lines, each pinned by a named test in its owning task.
