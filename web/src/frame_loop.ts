/**
 * The pace of the frames and the drawing of one frame. A frame comes on
 * the display's own frames, no sooner than the interval of the Video page;
 * a frame that fails stops the frames and is told, since the view that
 * failed would fail the same way again.
 */

import type { Point } from './input/pointer';
import { pointScale } from './points';
import type { PlacedPlate, PlacedWords } from './panels/types';
import type { WorldDraw } from './world/renderer';

/**
 * How early a frame may come. The display's own frames do not fall on the
 * interval to the millisecond; without this slack a rate equal to the
 * display's would skip every other frame.
 */
export const FRAME_SLACK_MS = 2;

export interface FrameSteps {
  /** The time between two frames now, in milliseconds. */
  intervalMs(): number;
  frame(): void;
  /** A frame failed with `error`; no frame comes after it. */
  fault(error: unknown): void;
}

export interface FrameLoop {
  stop(): void;
}

/** Runs `steps.frame` at the pace of `steps.intervalMs`, until stopped or a frame fails. */
export function startFrames(steps: FrameSteps): FrameLoop {
  let lastFrame = -Infinity;
  let running = true;
  let request = 0;
  const tick = (nowMs: number) => {
    if (nowMs - lastFrame >= steps.intervalMs() - FRAME_SLACK_MS) {
      lastFrame = nowMs;
      try {
        steps.frame();
      } catch (error) {
        running = false;
        steps.fault(error);
        return;
      }
    }
    if (running) request = requestAnimationFrame(tick);
  };
  request = requestAnimationFrame(tick);
  return {
    stop() {
      running = false;
      cancelAnimationFrame(request);
    },
  };
}

/** The draw lists of one frame, which the page frees once drawn. */
export interface DrawnFrame extends WorldDraw {
  /** The name plates and the words over heads, which the page draws as text. */
  plates(): PlacedPlate[];
  floats(): PlacedWords[];
  free(): void;
}

/** The words over the world of one frame. */
export interface WorldWords {
  plates: PlacedPlate[];
  floats: PlacedWords[];
}

/** The part of the view that runs a frame. */
export interface FrameView {
  tick(now: number, width: number, height: number, mouseX: number, mouseY: number, hasMouse: boolean): DrawnFrame;
}

/**
 * Runs the rules of one frame at `now` and draws it; gives the words over
 * the world, for the page to draw as text. The draw lists are freed even
 * when the drawing fails.
 */
export function drawFrame(
  view: FrameView,
  renderer: { draw(buffers: WorldDraw): void },
  now: number,
  size: { width: number; height: number },
  mouse: Point | null,
): WorldWords {
  const buffers = view.tick(now, size.width, size.height, mouse?.x ?? 0, mouse?.y ?? 0, mouse !== null);
  try {
    renderer.draw(buffers);
    return { plates: buffers.plates(), floats: buffers.floats() };
  } finally {
    buffers.free();
  }
}

/** The size of the view in points, and the device pixels of one point. */
export interface ViewSize {
  width: number;
  height: number;
  ratio: number;
}

/**
 * Follows the size of `canvas`, the pixels of the screen and the UI scale
 * (`pointScale`): each call gives the size now, in points, and calls
 * `apply` first when it changed since the last call (the first call
 * always). Called once a frame, it follows a resized window, a resized
 * page, a screen of another density and a new UI scale.
 */
export function followSize(canvas: { clientWidth: number; clientHeight: number }, apply: (size: ViewSize) => void): () => ViewSize {
  let size: ViewSize | null = null;
  return () => {
    const scale = pointScale();
    const now = { width: canvas.clientWidth / scale, height: canvas.clientHeight / scale, ratio: window.devicePixelRatio * scale };
    if (size?.width !== now.width || size.height !== now.height || size.ratio !== now.ratio) {
      size = now;
      apply(size);
    }
    return size;
  };
}
