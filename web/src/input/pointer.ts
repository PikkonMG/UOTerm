/**
 * The mouse, the wheel and the fingers over the world, as the view reads
 * them. Positions are in points (CSS pixels) from the top left of the
 * target.
 *
 * A finger is the mouse: a short touch is a click where it lifted, a touch
 * held `LONG_PRESS_MS` is the secondary button (held, it walks toward the
 * finger), a finger that moves first drags with the primary button, and two
 * fingers that pinch turn the wheel.
 */

import type { InputEvent, Mods, PointerButton } from './events';
import { mods } from './keys';

export const LONG_PRESS_MS = 500;
/** Two taps this close in time are a double click, as egui counts one. */
export const DOUBLE_TAP_MS = 300;
/** A finger that moves this far, in points, before its long press is a drag. */
const TOUCH_SLOP = 10;
/** The buttons by `MouseEvent.button`. */
const MOUSE_BUTTONS: readonly PointerButton[] = ['Primary', 'Middle', 'Secondary'];
/** `MouseEvent.detail` of the second press of a double click. */
const DOUBLE_CLICK_DETAIL = 2;
/** `WheelEvent.deltaMode`: pixels, lines or pages. */
const DELTA_PIXEL = 0;
const DELTA_LINE = 1;
/**
 * What one notch of a mouse wheel gives, by `deltaMode`: 100 pixels in
 * Chrome, 3 lines in Firefox, or a page. One notch of a wheel then turns
 * the map as one notch turns it in the Rust window.
 */
const PIXELS_PER_NOTCH = 100;
const LINES_PER_NOTCH = 3;
const PAGES_PER_NOTCH = 1;
const NO_TURN = 0;
const FINGERS_OF_A_PINCH = 2;
const NO_MODS: Mods = { ctrl: false, alt: false, shift: false, command: false };
/** Listeners that call `preventDefault`, so they may not be passive. */
const ACTIVE: AddEventListenerOptions = { passive: false };

export interface Point {
  x: number;
  y: number;
}

export interface PointerOptions {
  /** The points of the wheel one notch turns (`wheelPointsPerNotch()` of the view): a pinch turns one notch for each of these points. */
  pointsPerNotch: number;
}

export interface PointerInput {
  /** Where the mouse or the finger is over the target; null when it is off it. */
  mouse(): Point | null;
  /** Stops listening. */
  detach(): void;
}

/** What one finger does now. */
type Touching =
  | { kind: 'none' }
  /** Down, not yet long, not yet moved: a tap so far. */
  | { kind: 'tap'; from: Point; timer: ReturnType<typeof setTimeout> }
  | { kind: 'held'; button: PointerButton }
  /** Two fingers, this far apart. */
  | { kind: 'pinch'; apart: number };

const distance = (a: Point, b: Point) => Math.hypot(a.x - b.x, a.y - b.y);

/** The notches a wheel turned, positive away from the player, as one notch of a mouse wheel is one. */
export function wheelNotches(event: WheelEvent): number {
  const perNotch = event.deltaMode === DELTA_PIXEL ? PIXELS_PER_NOTCH : event.deltaMode === DELTA_LINE ? LINES_PER_NOTCH : PAGES_PER_NOTCH;
  return -event.deltaY / perNotch;
}

/** Sends the pointer of `target` to the view. */
export function attachPointer(target: HTMLElement, send: (event: InputEvent) => void, options: PointerOptions): PointerInput {
  let mouse: Point | null = null;
  let touching: Touching = { kind: 'none' };
  let lastTap = -Infinity;

  const at = (client: { clientX: number; clientY: number }): Point => {
    const box = target.getBoundingClientRect();
    return { x: client.clientX - box.left, y: client.clientY - box.top };
  };
  const press = (button: PointerButton, held: Mods, at: Point, double = false) => send({ kind: 'PointerDown', button, mods: held, double, ...at });
  const release = (button: PointerButton, point: Point, held: Mods) => send({ kind: 'PointerUp', ...point, button, mods: held });

  const mouseDown = (event: MouseEvent) => {
    const button = MOUSE_BUTTONS[event.button];
    if (!button) return;
    // The middle button would start the browser's own scrolling.
    if (button === 'Middle') event.preventDefault();
    mouse = at(event);
    press(button, mods(event), mouse, event.detail === DOUBLE_CLICK_DETAIL);
  };
  const mouseUp = (event: MouseEvent) => {
    const button = MOUSE_BUTTONS[event.button];
    if (button) release(button, at(event), mods(event));
  };
  const mouseMove = (event: MouseEvent) => {
    mouse = at(event);
  };
  const mouseLeave = () => {
    mouse = null;
  };
  const menu = (event: Event) => event.preventDefault();
  const wheel = (event: WheelEvent) => {
    event.preventDefault();
    // The wheel turns over the map where the mouse is, even before it moved.
    mouse = at(event);
    if (event.deltaY === NO_TURN) return;
    send({ kind: 'Wheel', notches: wheelNotches(event), mods: mods(event) });
  };

  const fingersApart = (touches: TouchList) => distance(at(touches[0]), at(touches[1]));
  const touchStart = (event: TouchEvent) => {
    event.preventDefault();
    const { touches } = event;
    if (touches.length >= FINGERS_OF_A_PINCH && (touching.kind === 'none' || touching.kind === 'tap')) {
      if (touching.kind === 'tap') clearTimeout(touching.timer);
      touching = { kind: 'pinch', apart: fingersApart(touches) };
      return;
    }
    if (touches.length !== 1 || touching.kind !== 'none') return;
    const from = at(touches[0]);
    mouse = from;
    const timer = setTimeout(() => {
      touching = { kind: 'held', button: 'Secondary' };
      press('Secondary', NO_MODS, from);
    }, LONG_PRESS_MS);
    touching = { kind: 'tap', from, timer };
  };
  const touchMove = (event: TouchEvent) => {
    event.preventDefault();
    const { touches } = event;
    if (touching.kind === 'pinch') {
      if (touches.length < FINGERS_OF_A_PINCH) return;
      const apart = fingersApart(touches);
      const notches = (apart - touching.apart) / options.pointsPerNotch;
      touching = { kind: 'pinch', apart };
      if (notches !== 0) send({ kind: 'Wheel', notches, mods: NO_MODS });
      return;
    }
    if (touches.length === 0) return;
    mouse = at(touches[0]);
    if (touching.kind === 'tap' && distance(mouse, touching.from) > TOUCH_SLOP) {
      const { from, timer } = touching;
      clearTimeout(timer);
      touching = { kind: 'held', button: 'Primary' };
      // The finger went down where the drag began.
      press('Primary', NO_MODS, from);
    }
  };
  /** The fingers lifted: a tap clicks, a held button comes up. */
  const touchEnd = (event: TouchEvent) => endTouch(event, true);
  /** The browser took the touch (a system gesture): a held button comes up, nothing clicks. */
  const touchCancel = (event: TouchEvent) => endTouch(event, false);
  const endTouch = (event: TouchEvent, clicks: boolean) => {
    event.preventDefault();
    if (event.touches.length > 0) return;
    const lifted = event.changedTouches.length > 0 ? at(event.changedTouches[0]) : mouse;
    const ended = touching;
    touching = { kind: 'none' };
    mouse = null;
    if (ended.kind === 'tap') clearTimeout(ended.timer);
    if (!lifted) return;
    if (ended.kind === 'tap' && clicks) {
      const now = performance.now();
      const double = now - lastTap <= DOUBLE_TAP_MS;
      lastTap = double ? -Infinity : now;
      press('Primary', NO_MODS, lifted, double);
      release('Primary', lifted, NO_MODS);
    } else if (ended.kind === 'held') {
      release(ended.button, lifted, NO_MODS);
    }
  };

  const listeners: [EventTarget, string, EventListener][] = [
    [target, 'mousedown', mouseDown as EventListener],
    [window, 'mouseup', mouseUp as EventListener],
    [target, 'mousemove', mouseMove as EventListener],
    [target, 'mouseleave', mouseLeave],
    [target, 'contextmenu', menu],
    [target, 'wheel', wheel as EventListener],
    [target, 'touchstart', touchStart as EventListener],
    [target, 'touchmove', touchMove as EventListener],
    [target, 'touchend', touchEnd as EventListener],
    [target, 'touchcancel', touchCancel as EventListener],
  ];
  for (const [on, type, listener] of listeners) on.addEventListener(type, listener, ACTIVE);
  return {
    mouse: () => mouse,
    detach() {
      if (touching.kind === 'tap') clearTimeout(touching.timer);
      for (const [on, type, listener] of listeners) on.removeEventListener(type, listener, ACTIVE);
    },
  };
}
