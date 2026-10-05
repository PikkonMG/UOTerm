/**
 * The play window of one session: the view (WebAssembly) runs the rules,
 * the live link brings the pictures of the session and carries its calls,
 * the art feed fetches what the view wants, the renderer draws each frame
 * and the player plays its sound. The frames come at the pace of the Video
 * page of the profile, and everything counts in the points of its UI scale.
 */

import { Player, resumeOnGesture, type AudioOut } from './audio/player';
import { drawFrame, followSize, startFrames, type WorldWords } from './frame_loop';
import type { InputEvent } from './input/events';
import { PadReader } from './input/gamepad';
import { attachKeys } from './input/keys';
import { attachPointer } from './input/pointer';
import { api } from './net/api';
import { ArtFeed } from './net/art';
import { LiveLink } from './net/live';
import { sendOut, type OutCall, type OutPlaces } from './out_calls';
import { setDragDistance } from './panels/drag';
import { setArtMostScale } from './panels/Picture';
import { bodyMeasure, plateMeasure } from './panels/measure';
import type { CoveredArea } from './panels/Panels';
import type { PanelAction, PanelData } from './panels/types';
import { setPointScale } from './points';
import { takeScreenshot } from './screenshot';
import { tearDown } from './teardown';
import init, { artMostScale, atlasSide, clickDistance, renderMidi, wheelPointsPerNotch, WebView, whiteSide } from './wasm/uoterm_web.js';
import { WorldRenderer } from './world/renderer';

const MS_PER_SECOND = 1000;
/** `Date.getMonth()` counts from 0; the journal counts months from 1. */
const FIRST_MONTH = 1;
/** The folder of the config folder the server keeps screenshots in, for the journal line. */
const SCREENSHOTS_FOLDER = 'screenshots';

/** The profile the view starts with, the path it is kept at (`profilePath` of `screens/login_state`), and where its sound font comes from (`soundFontPath`). */
export interface GameProfile {
  path: string;
  value: unknown;
  soundFont: string;
}

export interface GameHandle {
  /**
   * Comes when the session ends; fails with the error of a frame that
   * failed. The game has stopped either way.
   */
  readonly ended: Promise<void>;
  /** Stops the game: no more frames, no link, no input. */
  stop(): void;
  /** Gives the view an action of the panel `panel`. */
  panel(panel: string, action: PanelAction): void;
  /** Gives the view an input event of a panel. */
  input(event: InputEvent): void;
  /** Tells the view where the panels lie, so the map takes no click there. */
  covered(areas: CoveredArea[]): void;
}

/** What the page draws over the world: the panels, and the words over the world; each comes again only when it changed. */
export interface Shows {
  panels(panels: PanelData): void;
  words(words: WorldWords): void;
}

/** Gives the view the clock of the computer, for the times of the journal. */
function tellLocalTime(view: WebView): void {
  const now = new Date();
  view.setLocalTime(now.getFullYear(), now.getMonth() + FIRST_MONTH, now.getDate(), now.getHours(), now.getMinutes(), now.getSeconds());
}

let loading: Promise<unknown> | undefined;

/** Loads the WebAssembly of the view, once. Call it before `startGame`. */
export async function loadView(): Promise<void> {
  loading ??= init();
  await loading;
}

/** Where the server says whether Jev can answer. */
const JEV_STATE_PATH = '/v1/jev';

/** Whether Jev can answer: the server has a TypeSafe key. A failed read counts as no. */
async function readOrdersOn(): Promise<boolean> {
  try {
    return (await api<{ on: boolean }>(JEV_STATE_PATH)).on;
  } catch {
    return false;
  }
}

/** The clock of the page, in seconds: the one clock of every call of the view. */
const clock = () => performance.now() / MS_PER_SECOND;

/**
 * Plays `session` on `canvas`, which the page sizes; its size in CSS pixels
 * by the UI scale is the size of the view in points. `overlay` holds the
 * words over the world and the panels, for the screenshots. After a frame
 * `show` gets the panels and the words over the world that changed since
 * the last.
 */
export function startGame(session: string, canvas: HTMLCanvasElement, overlay: HTMLElement, profile: GameProfile, show: Shows): GameHandle {
  const view = new WebView(JSON.stringify(profile.value));
  view.setSession(session);
  view.setTextMeasure(plateMeasure());
  view.setBodyMeasure(bodyMeasure());
  setDragDistance(clickDistance());
  setArtMostScale(artMostScale());
  let stopped = false;
  void readOrdersOn().then((on) => {
    if (!stopped) view.setOrdersOn(on);
  });
  // The view knows when the page shows in full screen, also after the
  // player left it himself, so Apply can ask for it again.
  const followFullscreen = () => {
    if (!stopped) view.setFullscreen(Boolean(document.fullscreenElement));
  };
  followFullscreen();
  document.addEventListener('fullscreenchange', followFullscreen);
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
  const places: OutPlaces = { session, profilePath: profile.path, link, answer, input: (event) => send(event) };
  /** A screenshot was asked for: it is taken right after the next draw, while the canvas still shows it. */
  let shotWanted = false;
  const out = (calls: OutCall[]) => {
    shotWanted ||= calls.some((call) => call.kind === 'Screenshot');
    sendOut(calls, places);
  };
  const send = (event: InputEvent) => {
    if (!stopped) out(view.input(JSON.stringify(event), clock()));
  };

  const feed = new ArtFeed(view);
  const renderer = new WorldRenderer(canvas, atlasSide(), whiteSide(), () => view.atlasLost());
  const detachKeys = attachKeys(window, send);
  const pointer = attachPointer(canvas, send, { pointsPerNotch: wheelPointsPerNotch() });
  const pad = new PadReader();
  const sound = new AudioContext();
  const stopResume = resumeOnGesture(window, sound);
  const player = new Player(sound, {
    ended: (voice) => {
      if (!stopped) view.soundEnded(voice);
    },
    soundFontPath: profile.soundFont,
    renderMidi,
  });
  /** Takes the screenshot of the frame just drawn; its journal line comes from the view. */
  const screenshot = () => {
    shotWanted = false;
    takeScreenshot(canvas, overlay).then(
      (file) => {
        if (!stopped) view.screenshotTaken(true, `${SCREENSHOTS_FOLDER}/${file}`);
      },
      (error: unknown) => {
        if (!stopped) view.screenshotTaken(false, error instanceof Error ? error.message : String(error));
      },
    );
  };
  /** Follows the size of the canvas, the pixels of the screen and the UI scale. */
  const fit = followSize(canvas, (size) => {
    renderer.resize(size.width, size.height, size.ratio);
    view.setPixelsPerPoint(size.ratio);
  });

  /** The panels and the words over the world last shown, as JSON. */
  let lastPanels = '';
  let lastWords = '';
  const frames = startFrames({
    intervalMs: () => view.frameIntervalMs(document.hasFocus()),
    // One frame: the controller, the wants of the view, the rules, the drawing, then the calls and the sound of the frame.
    frame: () => {
      setPointScale(view.uiScale());
      const size = fit();
      const padNow = pad.read();
      if (padNow) send(padNow);
      feed.pump();
      tellLocalTime(view);
      view.setFocused(document.hasFocus());
      const words = drawFrame(view, renderer, clock(), size, pointer.mouse());
      out(view.takeOut());
      if (shotWanted) screenshot();
      player.play(view.audioOut() as AudioOut[]);
      const panels = view.panelsJson(clock());
      if (panels !== lastPanels) {
        lastPanels = panels;
        show.panels(JSON.parse(panels) as PanelData);
      }
      const wordsJson = JSON.stringify(words);
      if (wordsJson !== lastWords) {
        lastWords = wordsJson;
        show.words(words);
      }
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
      () => document.removeEventListener('fullscreenchange', followFullscreen),
      stopResume,
      () => player.close(),
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

  return {
    ended,
    stop,
    panel: (panel, action) => send({ kind: 'Panel', panel, action }),
    input: send,
    covered: (areas) => {
      if (!stopped) view.setCovered(JSON.stringify(areas));
    },
  };
}
