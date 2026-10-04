/**
 * The keyboard, as egui reads it in a browser: a key goes by the key it
 * gives (`KeyboardEvent.key`, the logical key), named as egui's
 * `Key::name()` names it, which is the name saved profiles keep.
 */

import type { InputEvent, Mods } from './events';

const LETTERS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
const DIGITS = '0123456789';
const FUNCTION_KEYS = 35;

/**
 * The egui name of each `KeyboardEvent.key`: every word egui 0.31's
 * `Key::from_name` reads, to the name `Key::name` gives it.
 */
const EGUI_KEY: Record<string, string> = {
  '⏷': 'Down',
  ArrowDown: 'Down',
  Down: 'Down',
  '⏴': 'Left',
  ArrowLeft: 'Left',
  Left: 'Left',
  '⏵': 'Right',
  ArrowRight: 'Right',
  Right: 'Right',
  '⏶': 'Up',
  ArrowUp: 'Up',
  Up: 'Up',
  Escape: 'Escape',
  Esc: 'Escape',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Enter: 'Enter',
  Return: 'Enter',
  Help: 'Insert',
  Insert: 'Insert',
  Delete: 'Delete',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
  Copy: 'Copy',
  Cut: 'Cut',
  Paste: 'Paste',
  ' ': 'Space',
  Space: 'Space',
  ':': 'Colon',
  Colon: 'Colon',
  ',': 'Comma',
  Comma: 'Comma',
  '-': 'Minus',
  '−': 'Minus',
  Minus: 'Minus',
  '.': 'Period',
  Period: 'Period',
  '+': 'Plus',
  Plus: 'Plus',
  '=': 'Equals',
  Equal: 'Equals',
  Equals: 'Equals',
  NumpadEqual: 'Equals',
  ';': 'Semicolon',
  Semicolon: 'Semicolon',
  '\\': 'Backslash',
  Backslash: 'Backslash',
  '/': 'Slash',
  Slash: 'Slash',
  '|': 'Pipe',
  Pipe: 'Pipe',
  '?': 'Questionmark',
  Questionmark: 'Questionmark',
  '!': 'Exclamationmark',
  Exclamationmark: 'Exclamationmark',
  '[': 'OpenBracket',
  OpenBracket: 'OpenBracket',
  ']': 'CloseBracket',
  CloseBracket: 'CloseBracket',
  '{': 'OpenCurlyBracket',
  OpenCurlyBracket: 'OpenCurlyBracket',
  '}': 'CloseCurlyBracket',
  CloseCurlyBracket: 'CloseCurlyBracket',
  '`': 'Backtick',
  Backtick: 'Backtick',
  Backquote: 'Backtick',
  Grave: 'Backtick',
  "'": 'Quote',
  Quote: 'Quote',
  ...Object.fromEntries([...DIGITS].flatMap((digit) => [digit, `Digit${digit}`, `Numpad${digit}`].map((word) => [word, digit]))),
  ...Object.fromEntries([...LETTERS].flatMap((letter) => [letter, letter.toLowerCase()].map((word) => [word, letter]))),
  ...Object.fromEntries(Array.from({ length: FUNCTION_KEYS }, (_, at) => `F${at + 1}`).map((name) => [name, name])),
};

/** The keys the browser would act on itself, which the world takes instead (egui's own list). */
const KEYS_THE_WORLD_KEEPS = new Set(['Tab', 'Backspace', 'Up', 'Down', 'Left', 'Right', 'Space']);
/** Keys that, with Ctrl or the command key, the browser would act on (open, print, save). */
const SHORTCUTS_THE_WORLD_KEEPS = new Set(['O', 'P', 'S']);
const MAC_PLATFORM = /Mac|iPhone|iPad|iPod/;

/** The egui name of the key of `event`; null for a key egui does not know. */
export function keyName(event: KeyboardEvent): string | null {
  return Object.hasOwn(EGUI_KEY, event.key) ? EGUI_KEY[event.key] : null;
}

/** The modifier keys of an event. */
type ModifierKeys = Pick<KeyboardEvent, 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>;

/** The modifier keys held. `command` is the command key on a Mac and Ctrl elsewhere, as egui reads it. */
export function mods(event: ModifierKeys): Mods {
  const mac = MAC_PLATFORM.test(navigator.platform);
  return {
    ctrl: event.ctrlKey,
    alt: event.altKey,
    shift: event.shiftKey,
    command: mac ? event.metaKey : event.ctrlKey,
  };
}

/** True when `target` is a field that types the keys itself. */
function isField(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement);
}

/** The characters a key types: one character, with neither Ctrl nor the command key held. */
function typed(event: KeyboardEvent): string | null {
  return [...event.key].length === 1 && !event.ctrlKey && !event.metaKey ? event.key : null;
}

/**
 * Sends the keys of `target` (the window) to the view: each key as it goes
 * down and up, and the characters it types while no field has the keys.
 * A field types its own words; the view hears its keys all the same and
 * reads them by the field that has them, so it is told when a field takes
 * the keys and when it lets them go. Every held key comes up when the page
 * loses the keyboard. Gives the function that stops it.
 */
export function attachKeys(target: Window, send: (event: InputEvent) => void): () => void {
  const held = new Set<string>();
  let inField = false;
  const focus = (field: boolean) => {
    if (field === inField) return;
    inField = field;
    send({ kind: 'Focus', chat_focused: false, other_field_focused: field });
  };
  const focusIn = (event: FocusEvent) => focus(isField(event.target));
  const focusOut = (event: FocusEvent) => focus(isField(event.relatedTarget));
  const down = (event: KeyboardEvent) => {
    if (event.isComposing) return;
    const typing = isField(event.target);
    const text = typing ? null : typed(event);
    if (text !== null) {
      send({ kind: 'Text', text });
      event.preventDefault();
    }
    const key = keyName(event);
    if (key === null) return;
    held.add(key);
    const heldMods = mods(event);
    send({ kind: 'Key', key, mods: heldMods, pressed: true, repeat: event.repeat });
    const shortcut = (heldMods.ctrl || heldMods.command) && SHORTCUTS_THE_WORLD_KEEPS.has(key);
    if (!typing && (KEYS_THE_WORLD_KEEPS.has(key) || shortcut)) event.preventDefault();
  };
  const up = (event: KeyboardEvent) => {
    const key = keyName(event);
    if (key === null) return;
    held.delete(key);
    send({ kind: 'Key', key, mods: mods(event), pressed: false, repeat: false });
  };
  const lost = () => {
    const none: ModifierKeys = { ctrlKey: false, altKey: false, shiftKey: false, metaKey: false };
    for (const key of held) send({ kind: 'Key', key, mods: mods(none), pressed: false, repeat: false });
    held.clear();
  };
  target.addEventListener('keydown', down);
  target.addEventListener('keyup', up);
  target.addEventListener('blur', lost);
  target.addEventListener('focusin', focusIn);
  target.addEventListener('focusout', focusOut);
  return () => {
    target.removeEventListener('keydown', down);
    target.removeEventListener('keyup', up);
    target.removeEventListener('blur', lost);
    target.removeEventListener('focusin', focusIn);
    target.removeEventListener('focusout', focusOut);
  };
}
