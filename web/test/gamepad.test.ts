import { afterEach, describe, expect, it, vi } from 'vitest';
import { PadReader } from '../src/input/gamepad';

/** The standard buttons a pad has. */
const STANDARD_BUTTON_COUNT = 17;

/** A standard pad with `pressed` buttons down and its sticks at `axes`. */
function pad(pressed: number[], axes = [0, 0, 0, 0], mapping = 'standard') {
  return {
    connected: true,
    mapping,
    axes,
    buttons: Array.from({ length: STANDARD_BUTTON_COUNT }, (_, at) => ({ pressed: pressed.includes(at), value: 0 })),
  };
}

function plugIn(...pads: unknown[]) {
  vi.stubGlobal('navigator', { ...navigator, getGamepads: () => pads });
}

afterEach(() => vi.unstubAllGlobals());

describe('PadReader', () => {
  it('names_the_standard_buttons_as_the_controller_library_does', () => {
    plugIn(pad([0, 4, 6, 9, 12, 16]));
    expect(new PadReader().read()).toEqual({
      kind: 'Pad',
      sticks: [0, 0, 0, 0],
      buttons: ['South', 'LeftTrigger', 'LeftTrigger2', 'Start', 'DPadUp', 'Mode'],
    });
  });

  it('keeps_the_buttons_in_the_order_they_went_down', () => {
    const reader = new PadReader();
    plugIn(pad([1]));
    reader.read();
    plugIn(pad([0, 1]));
    expect(reader.read()).toMatchObject({ buttons: ['East', 'South'] });
    plugIn(pad([0]));
    expect(reader.read()).toMatchObject({ buttons: ['South'] });
  });

  it('turns_the_sticks_up_as_the_controller_library_does', () => {
    plugIn(pad([], [0.5, -1, -0.25, 0.75]));
    expect(new PadReader().read()).toMatchObject({ sticks: [0.5, 1, -0.25, -0.75] });
  });

  it('tells_nothing_while_the_pad_stays_the_same', () => {
    const reader = new PadReader();
    plugIn(pad([2]));
    expect(reader.read()).not.toBeNull();
    expect(reader.read()).toBeNull();
  });

  it('lets_every_button_go_when_the_pad_goes', () => {
    const reader = new PadReader();
    plugIn(pad([3]));
    reader.read();
    plugIn(null);
    expect(reader.read()).toEqual({ kind: 'Pad', sticks: [0, 0, 0, 0], buttons: [] });
  });

  it('reads_only_a_pad_with_the_standard_layout', () => {
    plugIn(pad([0], [0, 0, 0, 0], ''));
    expect(new PadReader().read()).toBeNull();
  });
});
