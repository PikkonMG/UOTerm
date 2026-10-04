/**
 * The calls the view asks the page to make (`OutCall` of crates/uoterm-web):
 * acts and reads go on the live link, questions for Jev and the files to
 * keep go to the API. Each answer the view waits for goes back to it by
 * the id of its call.
 */

import { api, jsonInit, METHOD_POST, METHOD_PUT } from './net/api';
import type { LiveOut, PageCall } from './net/live';

const KEPT_PATH = '/v1/kept/';
const SAVE_FAILED = 'UOTerm could not keep';

export type OutCall =
  | { kind: 'Act'; id: number; act: { calls: PageCall[]; words: string } }
  | { kind: 'Read'; id: number; tool: string; args: unknown }
  | { kind: 'Jev'; id: number; route: string; body: unknown }
  | { kind: 'SaveProfile'; profile: unknown }
  | { kind: 'SaveKept'; name: string; data: unknown }
  | { kind: 'Screenshot' }
  | { kind: 'Window'; command: unknown };

/** Where the calls of one session go. */
export interface OutPlaces {
  session: string;
  /** Where the profile of the view is kept: `/v1/profiles/default` or `/v1/profiles/{shard}/{character}`. */
  profilePath: string;
  link: { send(message: LiveOut): void };
  /** Gives the view the answer of the call `id`, its result as JSON (`{error: words}` on failure). */
  answer(id: number, ok: boolean, resultJson: string): void;
}

/** The words of a fault. */
function wordsOf(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Puts `value` at `path`; a refusal goes to the console, as the window logs a file it could not write. */
function keep(path: string, value: unknown): void {
  api<void>(path, jsonInit(METHOD_PUT, value)).catch((error: unknown) => console.error(`${SAVE_FAILED} ${path}: ${wordsOf(error)}`));
}

/**
 * Makes each call. Window commands and screenshots are the page's own
 * work, done by the panels and the screenshot of the page, not here.
 */
export function sendOut(calls: OutCall[], places: OutPlaces): void {
  for (const call of calls) {
    switch (call.kind) {
      case 'Act':
        places.link.send({ kind: 'act', id: call.id, calls: call.act.calls });
        break;
      case 'Read':
        places.link.send({ kind: 'call', id: call.id, tool: call.tool, args: call.args });
        break;
      case 'Jev': {
        const path = `/v1/sessions/${encodeURIComponent(places.session)}/jev/${encodeURIComponent(call.route)}`;
        api<unknown>(path, jsonInit(METHOD_POST, call.body)).then(
          (result) => places.answer(call.id, true, JSON.stringify(result)),
          (error: unknown) => places.answer(call.id, false, JSON.stringify({ error: wordsOf(error) })),
        );
        break;
      }
      case 'SaveProfile':
        keep(places.profilePath, call.profile);
        break;
      case 'SaveKept':
        keep(`${KEPT_PATH}${encodeURIComponent(call.name)}`, call.data);
        break;
      case 'Screenshot':
      case 'Window':
        break;
    }
  }
}
