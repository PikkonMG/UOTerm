/**
 * The input the page gives the view (`InputEvent` of crates/uoterm-web),
 * as JSON. Keys go by the names egui gives them, as saved profiles keep
 * them; where the mouse is goes with each tick.
 */

/** The modifier keys held. `command` is Ctrl, or the command key on a Mac. */
export interface Mods {
  ctrl: boolean;
  alt: boolean;
  shift: boolean;
  command: boolean;
}

export type PointerButton = 'Primary' | 'Secondary' | 'Middle';

/** A button of a controller, as the controller library of the Rust window names it. */
export type PadButton =
  | 'South'
  | 'East'
  | 'North'
  | 'West'
  | 'LeftTrigger'
  | 'LeftTrigger2'
  | 'RightTrigger'
  | 'RightTrigger2'
  | 'Select'
  | 'Start'
  | 'Mode'
  | 'LeftThumb'
  | 'RightThumb'
  | 'DPadUp'
  | 'DPadDown'
  | 'DPadLeft'
  | 'DPadRight';

/** Left x, left y, right x, right y of the sticks, from -1 to 1, y up. */
export type Sticks = [number, number, number, number];

export type InputEvent =
  | { kind: 'Key'; key: string; mods: Mods; pressed: boolean; repeat: boolean }
  /** The characters a key types. */
  | { kind: 'Text'; text: string }
  /** `double` is the second press of a double click. */
  | { kind: 'PointerDown'; button: PointerButton; mods: Mods; double: boolean }
  /** A button came up at `x`, `y` of the view, in points. */
  | { kind: 'PointerUp'; x: number; y: number; button: PointerButton; mods: Mods }
  /** Notches of a wheel, positive when it turns away from the player. */
  | { kind: 'Wheel'; notches: number; mods: Mods }
  /** The controller now: the buttons down in the order they went down. */
  | { kind: 'Pad'; sticks: Sticks; buttons: PadButton[] }
  /** Which field has the keys: the view reads the keys by it. */
  | { kind: 'Focus'; chat_focused: boolean; other_field_focused: boolean };
