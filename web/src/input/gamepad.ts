/**
 * The controller, read once a frame from the Gamepad API. Only a pad with
 * the standard layout is read: its button numbers mean the same buttons on
 * every pad. Buttons and sticks go by the names and the signs of the
 * controller library of the Rust window (gilrs): y up.
 */

import type { InputEvent, PadButton, Sticks } from './events';

/** The buttons of the standard layout, by their numbers. */
const STANDARD_BUTTONS: readonly PadButton[] = [
  'South',
  'East',
  'West',
  'North',
  'LeftTrigger',
  'RightTrigger',
  'LeftTrigger2',
  'RightTrigger2',
  'Select',
  'Start',
  'LeftThumb',
  'RightThumb',
  'DPadUp',
  'DPadDown',
  'DPadLeft',
  'DPadRight',
  'Mode',
];
const STANDARD_LAYOUT = 'standard';
/** The axes of the standard layout: left x, left y, right x, right y, y down. */
const LEFT_X = 0;
const LEFT_Y = 1;
const RIGHT_X = 2;
const RIGHT_Y = 3;
const CENTER = 0;

type PadEvent = Extract<InputEvent, { kind: 'Pad' }>;

/** The first connected pad with the standard layout. */
function standardPad(): Gamepad | undefined {
  const pads = typeof navigator.getGamepads === 'function' ? navigator.getGamepads() : [];
  return pads.find((pad): pad is Gamepad => pad !== null && pad.connected && pad.mapping === STANDARD_LAYOUT);
}

/** Reads the controller and tells the view when it changed. */
export class PadReader {
  /** The buttons down, in the order they went down. */
  private down: PadButton[] = [];
  private sticks: Sticks = [CENTER, CENTER, CENTER, CENTER];

  /** The controller now, when it changed since the last read; null when it did not. */
  read(): PadEvent | null {
    const pad = standardPad();
    const axis = (at: number) => pad?.axes[at] ?? CENTER;
    // A stick pushed up gives a negative axis in the browser; gilrs gives a
    // positive one. A centered stick stays 0, not -0.
    const sticks: Sticks = [axis(LEFT_X), -axis(LEFT_Y) || CENTER, axis(RIGHT_X), -axis(RIGHT_Y) || CENTER];
    const pressed = STANDARD_BUTTONS.filter((_, at) => pad?.buttons[at]?.pressed === true);
    const down = [...this.down.filter((button) => pressed.includes(button)), ...pressed.filter((button) => !this.down.includes(button))];
    const same = sticks.every((value, at) => value === this.sticks[at]) && down.length === this.down.length && down.every((button, at) => button === this.down[at]);
    if (same) return null;
    this.sticks = sticks;
    this.down = down;
    return { kind: 'Pad', sticks, buttons: down };
  }
}
