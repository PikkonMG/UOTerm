// Builds the view for the browser into src/wasm, and copies the fonts of
// the Rust window into src/fonts. Both are build output, not committed.
import { copyFileSync, mkdirSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { REPO_DIR, run, WEB_DIR } from './workspace.mjs';

const WASM_TARGET = 'wasm32-unknown-unknown';
const TARGET_DIR = process.env.CARGO_TARGET_DIR ?? join(REPO_DIR, 'target');
const WASM_FILE = join(TARGET_DIR, WASM_TARGET, 'release', 'uoterm_web.wasm');
const WASM_OUT = join(WEB_DIR, 'src', 'wasm');
const FONTS_FROM = join(REPO_DIR, 'crates', 'uoterm', 'assets', 'fonts');
const FONTS_TO = join(WEB_DIR, 'src', 'fonts');
const FONT_EXTENSION = '.ttf';
const FONT_LICENSE = 'OFL.txt';

run('cargo', ['build', '-p', 'uoterm-web', '--release', '--target', WASM_TARGET]);
run('wasm-bindgen', ['--target', 'web', '--out-dir', WASM_OUT, WASM_FILE]);

mkdirSync(FONTS_TO, { recursive: true });
for (const name of readdirSync(FONTS_FROM)) {
  if (name.endsWith(FONT_EXTENSION) || name === FONT_LICENSE) {
    copyFileSync(join(FONTS_FROM, name), join(FONTS_TO, name));
  }
}
