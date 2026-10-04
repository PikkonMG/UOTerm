// Writes src/theme.css from the theme of the Rust view, so the colors and
// sizes live in one place: uoterm_view::ui::theme.
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { run, WEB_DIR } from './workspace.mjs';

const THEME_FILE = join(WEB_DIR, 'src', 'theme.css');
const INDENT = '  ';

const tokens = run('cargo', ['run', '--quiet', '-p', 'uoterm', '--', 'theme-css'])
  .split('\n')
  .filter((line) => line.trim() !== '')
  .map((line) => `${INDENT}${line}`)
  .join('\n');
writeFileSync(
  THEME_FILE,
  `/* Made by \`npm run theme\` from uoterm_view::ui::theme. Do not edit. */\n:root {\n${tokens}\n}\n`,
);
