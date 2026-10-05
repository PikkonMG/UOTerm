/**
 * The calls the view asks the page to make (`OutCall` of crates/uoterm-web):
 * acts and reads go on the live link, questions for Jev and the files to
 * keep go to the API. Each answer the view waits for goes back to it by
 * the id of its call.
 */

import { api, jsonInit, METHOD_POST, METHOD_PUT, TokenNeeded, whenTokenGiven, wordsOf } from './net/api';
import type { InputEvent } from './input/events';
import { CHAT_ATTRIBUTE } from './input/keys';
import { askChatFocus } from './panels/ChatLine';
import type { LiveOut, PageCall } from './net/live';

const KEPT_PATH = '/v1/kept/';
/** Where the profile every new character starts with is kept. */
const DEFAULT_PROFILE_PATH = '/v1/profiles/default';
const TEXT_FILE = 'text/plain';
const SAVE_FAILED = 'UOTerm could not keep';
/**
 * How long the address of a downloaded file lives after its click: the
 * browser reads the file after the click returns, so the address must
 * outlive it.
 */
export const DOWNLOAD_URL_LIFE_MS = 1_000;

export type OutCall =
  | { kind: 'Act'; id: number; act: { calls: PageCall[]; words: string } }
  | { kind: 'Read'; id: number; tool: string; args: unknown }
  | { kind: 'Jev'; id: number; route: string; body: unknown }
  | { kind: 'SaveProfile'; profile: unknown }
  | { kind: 'SaveKept'; name: string; data: unknown }
  | { kind: 'Screenshot' }
  | { kind: 'Download'; name: string; text: string }
  | { kind: 'ChatFocus'; take: boolean }
  | { kind: 'ChatPaste' }
  | { kind: 'SaveDefaultProfile'; profile: unknown }
  | { kind: 'Fullscreen'; on: boolean };

/** Where the calls of one session go. */
export interface OutPlaces {
  session: string;
  /** Where the profile of the view is kept: `/v1/profiles/default` or `/v1/profiles/{shard}/{character}`. */
  profilePath: string;
  link: { send(message: LiveOut): void };
  /** Gives the view the answer of the call `id`, its result as JSON (`{error: words}` on failure). */
  answer(id: number, ok: boolean, resultJson: string): void;
  /** Gives the view an input event, as the words of the chat line. */
  input(event: InputEvent): void;
}

/** The saves the API refused for want of its token, the newest value by path: they go again once it has it. */
const parkedSaves = new Map<string, unknown>();
let stopHearingToken: (() => void) | undefined;

/** Sends every parked save again, now the API has its token. */
function resendParked(): void {
  stopHearingToken?.();
  stopHearingToken = undefined;
  const saves = [...parkedSaves];
  parkedSaves.clear();
  for (const [path, value] of saves) keep(path, value);
}

/**
 * Puts `value` at `path`. A save refused for want of the token waits for
 * it, the newest by path; any other refusal goes to the console, as the
 * window logs a file it could not write.
 */
function keep(path: string, value: unknown): void {
  parkedSaves.delete(path);
  api<void>(path, jsonInit(METHOD_PUT, value)).catch((error: unknown) => {
    if (!(error instanceof TokenNeeded)) {
      console.error(`${SAVE_FAILED} ${path}: ${wordsOf(error)}`);
      return;
    }
    parkedSaves.set(path, value);
    stopHearingToken ??= whenTokenGiven(resendParked);
  });
}

/** Gives the player a text file to keep, as the browser downloads one. */
function download(name: string, text: string): void {
  const url = URL.createObjectURL(new Blob([text], { type: TEXT_FILE }));
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), DOWNLOAD_URL_LIFE_MS);
}

/** The field of the chat line, when it shows. */
const chatField = () => document.querySelector<HTMLInputElement>(`[${CHAT_ATTRIBUTE}]`);

/** Pastes the clipboard into the chat line: its words go to the view. */
function paste(input: OutPlaces['input']): void {
  navigator.clipboard?.readText().then(
    (pasted) => input({ kind: 'ChatWords', text: `${chatField()?.value ?? ''}${pasted}` }),
    () => {},
  );
}

/**
 * Asks the browser for the full screen, or lets it go. The browser grants
 * it only during a press of the player, which the call follows at once; a
 * refusal leaves the window as it is.
 */
function fullscreen(on: boolean): void {
  const shown = Boolean(document.fullscreenElement);
  if (on && !shown) document.documentElement.requestFullscreen?.().catch(() => {});
  if (!on && shown) document.exitFullscreen?.().catch(() => {});
}

/** Makes each call. A screenshot is taken by the game right after its next draw (`screenshot.ts`), not here. */
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
      case 'Download':
        download(call.name, call.text);
        break;
      case 'ChatFocus':
        askChatFocus(call.take);
        break;
      case 'ChatPaste':
        paste(places.input);
        break;
      case 'SaveDefaultProfile':
        keep(DEFAULT_PROFILE_PATH, call.profile);
        break;
      case 'Fullscreen':
        fullscreen(call.on);
        break;
      case 'Screenshot':
        break;
    }
  }
}
