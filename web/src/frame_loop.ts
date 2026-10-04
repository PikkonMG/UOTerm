/**
 * The pace of the frames and the drawing of one frame. A frame comes on
 * the display's own frames, no sooner than the interval of the Video page;
 * a frame that fails stops the frames and is told, since the view that
 * failed would fail the same way again.
 */

import type { Point } from './input/pointer';
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
  free(): void;
}

/** The part of the view that runs a frame. */
export interface FrameView {
  tick(now: number, width: number, height: number, mouseX: number, mouseY: number, hasMouse: boolean): DrawnFrame;
}

/** Runs the rules of one frame at `now` and draws it. The draw lists are freed even when the drawing fails. */
export function drawFrame(
  view: FrameView,
  renderer: { draw(buffers: WorldDraw): void },
  now: number,
  size: { width: number; height: number },
  mouse: Point | null,
): void {
  const buffers = view.tick(now, size.width, size.height, mouse?.x ?? 0, mouse?.y ?? 0, mouse !== null);
  try {
    renderer.draw(buffers);
  } finally {
    buffers.free();
  }
}
