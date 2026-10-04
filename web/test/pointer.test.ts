import { afterEach, describe, expect, it, vi } from 'vitest';
import type { InputEvent } from '../src/input/events';
import { attachPointer, LONG_PRESS_MS, type PointerInput } from '../src/input/pointer';

/** The points of a wheel notch the tests use (the view gives 40). */
const POINTS_PER_NOTCH = 40;
const NO_MODS = { ctrl: false, alt: false, shift: false, command: false };

/** A finger at `x`, `y`, as the browser gives it. */
const finger = (x: number, y: number, identifier = 0) => ({ clientX: x, clientY: y, identifier }) as unknown as Touch;

const touch = (type: string, touches: Touch[], changed: Touch[] = touches) =>
  new TouchEvent(type, { touches, changedTouches: changed, cancelable: true });

let attached: PointerInput | undefined;

/** A target with the pointer attached, and the events it sent. */
function attach(): { el: HTMLElement; events: InputEvent[]; pointer: PointerInput } {
  const events: InputEvent[] = [];
  const el = document.createElement('div');
  document.body.append(el);
  attached = attachPointer(el, (event) => events.push(event), { pointsPerNotch: POINTS_PER_NOTCH });
  return { el, events, pointer: attached };
}

afterEach(() => {
  attached?.detach();
  attached = undefined;
  vi.useRealTimers();
  document.body.innerHTML = '';
});

describe('pointer', () => {
  it('turns_a_long_touch_into_a_secondary_press', () => {
    vi.useFakeTimers();
    const { el, events } = attach();
    el.dispatchEvent(new TouchEvent('touchstart', { touches: [{ clientX: 10, clientY: 20, identifier: 0, target: el } as unknown as Touch] }));
    vi.advanceTimersByTime(LONG_PRESS_MS);
    expect(events).toContainEqual(expect.objectContaining({ kind: 'PointerDown', button: 'Secondary' }));
  });

  it('lets_the_secondary_button_go_when_the_long_touch_ends', () => {
    vi.useFakeTimers();
    const { el, events, pointer } = attach();
    el.dispatchEvent(touch('touchstart', [finger(10, 20)]));
    vi.advanceTimersByTime(LONG_PRESS_MS);
    expect(pointer.mouse()).toEqual({ x: 10, y: 20 });
    el.dispatchEvent(touch('touchend', [], [finger(30, 40)]));
    expect(events.at(-1)).toEqual({ kind: 'PointerUp', x: 30, y: 40, button: 'Secondary', mods: NO_MODS });
    expect(pointer.mouse()).toBeNull();
  });

  it('turns_a_short_touch_into_a_click_where_it_lifted', () => {
    vi.useFakeTimers();
    const { el, events } = attach();
    el.dispatchEvent(touch('touchstart', [finger(5, 6)]));
    vi.advanceTimersByTime(LONG_PRESS_MS / 2);
    el.dispatchEvent(touch('touchend', [], [finger(5, 6)]));
    vi.advanceTimersByTime(LONG_PRESS_MS);
    expect(events).toEqual([
      { kind: 'PointerDown', button: 'Primary', mods: NO_MODS, double: false },
      { kind: 'PointerUp', x: 5, y: 6, button: 'Primary', mods: NO_MODS },
    ]);
  });

  it('makes_two_quick_taps_a_double_click', () => {
    const { el, events } = attach();
    for (let tap = 0; tap < 2; tap++) {
      el.dispatchEvent(touch('touchstart', [finger(5, 6)]));
      el.dispatchEvent(touch('touchend', [], [finger(5, 6)]));
    }
    const downs = events.filter((event) => event.kind === 'PointerDown');
    expect(downs.map((event) => event.kind === 'PointerDown' && event.double)).toEqual([false, true]);
  });

  it('turns_a_pinch_into_the_wheel', () => {
    const { el, events } = attach();
    el.dispatchEvent(touch('touchstart', [finger(100, 100), finger(140, 100, 1)]));
    el.dispatchEvent(touch('touchmove', [finger(80, 100), finger(160, 100, 1)]));
    el.dispatchEvent(touch('touchend', [], [finger(80, 100), finger(160, 100, 1)]));
    expect(events).toEqual([{ kind: 'Wheel', notches: 1, mods: NO_MODS }]);
  });

  it('presses_the_mouse_buttons_by_their_names_and_marks_a_double_click', () => {
    const { el, events } = attach();
    el.dispatchEvent(new MouseEvent('mousedown', { button: 2 }));
    el.dispatchEvent(new MouseEvent('mousedown', { button: 0, detail: 2 }));
    expect(events).toEqual([
      { kind: 'PointerDown', button: 'Secondary', mods: NO_MODS, double: false },
      { kind: 'PointerDown', button: 'Primary', mods: NO_MODS, double: true },
    ]);
  });

  it('lets_a_button_go_even_off_the_target', () => {
    const { events } = attach();
    window.dispatchEvent(new MouseEvent('mouseup', { button: 2, clientX: 7, clientY: 9 }));
    expect(events).toEqual([{ kind: 'PointerUp', x: 7, y: 9, button: 'Secondary', mods: NO_MODS }]);
  });

  it('knows_where_the_mouse_is_until_it_leaves', () => {
    const { el, pointer } = attach();
    el.dispatchEvent(new MouseEvent('mousemove', { clientX: 3, clientY: 4 }));
    expect(pointer.mouse()).toEqual({ x: 3, y: 4 });
    el.dispatchEvent(new MouseEvent('mouseleave'));
    expect(pointer.mouse()).toBeNull();
  });

  it('turns_the_wheel_into_notches_away_from_the_player', () => {
    const { el, events } = attach();
    el.dispatchEvent(new WheelEvent('wheel', { deltaY: -100, deltaMode: 0 }));
    el.dispatchEvent(new WheelEvent('wheel', { deltaY: 3, deltaMode: 1 }));
    expect(events).toEqual([
      { kind: 'Wheel', notches: 2.5, mods: NO_MODS },
      { kind: 'Wheel', notches: -3, mods: NO_MODS },
    ]);
  });

  it('keeps_the_menu_of_the_browser_away', () => {
    const { el } = attach();
    const menu = new MouseEvent('contextmenu', { cancelable: true });
    el.dispatchEvent(menu);
    expect(menu.defaultPrevented).toBe(true);
  });

  it('stops_listening_when_detached', () => {
    const { el, events, pointer } = attach();
    pointer.detach();
    el.dispatchEvent(new MouseEvent('mousedown', { button: 0 }));
    expect(events).toEqual([]);
  });
});
