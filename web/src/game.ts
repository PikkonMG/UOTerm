/**
 * The play window of one session: the view (WebAssembly) runs the rules,
 * the live link brings the pictures of the session and carries its calls,
 * the art feed fetches what the view wants, and the renderer draws each
 * frame. The frames come at the pace of the Video page of the profile.
 */

import type { InputEvent } from './input/events';
import { PadReader } from './input/gamepad';
import { attachKeys } from './input/keys';
import { attachPointer } from './input/pointer';
import { ArtFeed } from './net/art';
import { LiveLink } from './net/live';
import { sendOut, type OutCall, type OutPlaces } from './out_calls';
import init, { atlasSide, wheelPointsPerNotch, WebView, whiteSide } from './wasm/uoterm_web.js';
import { WorldRenderer } from './world/renderer';

const MS_PER_SECOND = 1000;
/**
 * How early a frame may come. The display's own frames do not fall on the
 * interval to the millisecond; without this slack a rate equal to the
 * display's would skip every other frame.
 */
const FRAME_SLACK_MS = 2;

/** Where the profile of every character is kept until the page knows the shard and the character. */
export const DEFAULT_PROFILE_PATH = '/v1/profiles/default';

/** The profile the view starts with, and the path it is kept at. */
export interface GameProfile {
  path: string;
  value: unknown;
}

export interface GameHandle {
  /** Comes when the session ends; the game has stopped then. */
  readonly ended: Promise<void>;
  /** Stops the game: no more frames, no link, no input. */
  stop(): void;
}

let loading: Promise<unknown> | undefined;

/** Loads the WebAssembly of the view, once. Call it before `startGame`. */
export async function loadView(): Promise<void> {
  loading ??= init();
  await loading;
}

/** The clock of the page, in seconds: the one clock of every call of the view. */
const clock = () => performance.now() / MS_PER_SECOND;

/** Plays `session` on `canvas`, which the page sizes; its size in CSS pixels is the size of the view in points. */
export function startGame(session: string, canvas: HTMLCanvasElement, profile: GameProfile): GameHandle {
  const view = new WebView(JSON.stringify(profile.value));
  let stopped = false;
  let endGame = () => {};
  const ended = new Promise<void>((resolve) => (endGame = resolve));

  const answer = (id: number, ok: boolean, resultJson: string) => {
    if (!stopped) view.answer(id, ok, resultJson, clock());
  };
  const link = new LiveLink(session, {
    frame: (watch) => view.frame(JSON.stringify(watch), clock()),
    answer: (id, ok, result) => answer(id, ok, JSON.stringify(result)),
    ended: () => {
      stop();
      endGame();
    },
    // A lost link comes back by itself; the view keeps its last picture meanwhile.
    state: () => {},
  });
  const places: OutPlaces = { session, profilePath: profile.path, link, answer };
  const out = (calls: OutCall[]) => sendOut(calls, places);
  const send = (event: InputEvent) => {
    if (!stopped) out(view.input(JSON.stringify(event), clock()));
  };

  const feed = new ArtFeed(view);
  const renderer = new WorldRenderer(canvas, atlasSide(), whiteSide());
  const detachKeys = attachKeys(window, send);
  const pointer = attachPointer(canvas, send, { pointsPerNotch: wheelPointsPerNotch() });
  const pad = new PadReader();
  let size = { width: 0, height: 0, ratio: 0 };
  let lastFrame = -Infinity;

  /** Follows the size of the canvas and the pixels of the screen. */
  const fit = () => {
    const now = { width: canvas.clientWidth, height: canvas.clientHeight, ratio: window.devicePixelRatio };
    if (now.width === size.width && now.height === size.height && now.ratio === size.ratio) return;
    size = now;
    renderer.resize(size.width, size.height, size.ratio);
    view.setPixelsPerPoint(size.ratio);
  };

  /** One frame: the controller, the wants of the view, the rules, the drawing, then the calls of the frame. */
  const run = () => {
    fit();
    const padNow = pad.read();
    if (padNow) send(padNow);
    feed.pump();
    const mouse = pointer.mouse();
    const buffers = view.tick(clock(), size.width, size.height, mouse?.x ?? 0, mouse?.y ?? 0, mouse !== null);
    renderer.draw(buffers);
    buffers.free();
    out(view.takeOut());
  };

  // A frame that fails stops the loop: the next one would fail the same way.
  const frame = (nowMs: number) => {
    if (nowMs - lastFrame >= view.frameIntervalMs(document.hasFocus()) - FRAME_SLACK_MS) {
      lastFrame = nowMs;
      run();
    }
    request = requestAnimationFrame(frame);
  };
  let request = requestAnimationFrame(frame);

  function stop() {
    if (stopped) return;
    stopped = true;
    cancelAnimationFrame(request);
    detachKeys();
    pointer.detach();
    link.close();
    feed.close();
    renderer.dispose();
    view.free();
  }

  return { ended, stop };
}
