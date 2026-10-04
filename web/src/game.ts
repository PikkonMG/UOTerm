/**
 * The play window of one session: the view (WebAssembly) runs the rules,
 * the live link brings the pictures of the session and carries its calls,
 * the art feed fetches what the view wants, and the renderer draws each
 * frame. The frames come at the pace of the Video page of the profile.
 */

import { drawFrame, startFrames } from './frame_loop';
import type { InputEvent } from './input/events';
import { PadReader } from './input/gamepad';
import { attachKeys } from './input/keys';
import { attachPointer } from './input/pointer';
import { ArtFeed } from './net/art';
import { LiveLink } from './net/live';
import { sendOut, type OutCall, type OutPlaces } from './out_calls';
import { tearDown } from './teardown';
import init, { atlasSide, wheelPointsPerNotch, WebView, whiteSide } from './wasm/uoterm_web.js';
import { WorldRenderer } from './world/renderer';

const MS_PER_SECOND = 1000;

/** The profile the view starts with, and the path it is kept at (`profilePath` of `screens/login_state`). */
export interface GameProfile {
  path: string;
  value: unknown;
}

export interface GameHandle {
  /**
   * Comes when the session ends; fails with the error of a frame that
   * failed. The game has stopped either way.
   */
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
  let failGame: (error: unknown) => void = () => {};
  const ended = new Promise<void>((resolve, reject) => {
    endGame = resolve;
    failGame = reject;
  });

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
  const renderer = new WorldRenderer(canvas, atlasSide(), whiteSide(), () => view.atlasLost());
  const detachKeys = attachKeys(window, send);
  const pointer = attachPointer(canvas, send, { pointsPerNotch: wheelPointsPerNotch() });
  const pad = new PadReader();
  let size = { width: 0, height: 0, ratio: 0 };

  /** Follows the size of the canvas and the pixels of the screen. */
  const fit = () => {
    const now = { width: canvas.clientWidth, height: canvas.clientHeight, ratio: window.devicePixelRatio };
    if (now.width === size.width && now.height === size.height && now.ratio === size.ratio) return;
    size = now;
    renderer.resize(size.width, size.height, size.ratio);
    view.setPixelsPerPoint(size.ratio);
  };

  const frames = startFrames({
    intervalMs: () => view.frameIntervalMs(document.hasFocus()),
    // One frame: the controller, the wants of the view, the rules, the drawing, then the calls of the frame.
    frame: () => {
      fit();
      const padNow = pad.read();
      if (padNow) send(padNow);
      feed.pump();
      drawFrame(view, renderer, clock(), size, pointer.mouse());
      out(view.takeOut());
    },
    fault: (error) => {
      failGame(error);
      stop();
    },
  });

  function stop() {
    if (stopped) return;
    stopped = true;
    tearDown([
      () => frames.stop(),
      detachKeys,
      () => pointer.detach(),
      () => link.close(),
      () => feed.close(),
      () => renderer.dispose(),
      () => {
        try {
          view.free();
        } catch {
          // A view whose WebAssembly failed may fail to free too; it is gone either way.
        }
      },
    ]);
  }

  return { ended, stop };
}
