// Where the build scripts of the page find the Rust workspace, and how
// they run its tools.
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

/** The folder of the page: `web/`. */
export const WEB_DIR = fileURLToPath(new URL('..', import.meta.url));
/** The folder of the Rust workspace, above `web/`. */
export const REPO_DIR = fileURLToPath(new URL('../..', import.meta.url));

/** Runs `command` in the workspace and gives its output; its log goes to this terminal. */
export function run(command, args) {
  return execFileSync(command, args, { cwd: REPO_DIR, encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] });
}
