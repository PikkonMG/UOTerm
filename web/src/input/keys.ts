/**
 * The keyboard, as egui reads it in the Rust window (egui-winit): a key
 * goes by the key it gives (`KeyboardEvent.key`, the logical key), else by
 * where it sits (`KeyboardEvent.code`, the physical key), so the digit row
 * of a French keyboard and the letters of a Russian one still give the
 * names hotkeys use. The names are egui's `Key::name()`, which saved
 * profiles keep.
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

/**
 * The egui name of each `KeyboardEvent.code`: the physical keys egui-winit
 * 0.31's `key_from_key_code` names.
 */
const EGUI_CODE: Record<string, string> = {
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  ArrowUp: 'Up',
  Escape: 'Escape',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Enter: 'Enter',
  NumpadEnter: 'Enter',
  Insert: 'Insert',
  Delete: 'Delete',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
  Space: 'Space',
  Comma: 'Comma',
  Period: 'Period',
  Semicolon: 'Semicolon',
  Backslash: 'Backslash',
  Slash: 'Slash',
  NumpadDivide: 'Slash',
  BracketLeft: 'OpenBracket',
  BracketRight: 'CloseBracket',
  Backquote: 'Backtick',
  Quote: 'Quote',
  Cut: 'Cut',
  Copy: 'Copy',
  Paste: 'Paste',
  Minus: 'Minus',
  NumpadSubtract: 'Minus',
  NumpadAdd: 'Plus',
  Equal: 'Equals',
  ...Object.fromEntries([...DIGITS].flatMap((digit) => [`Digit${digit}`, `Numpad${digit}`].map((code) => [code, digit]))),
  ...Object.fromEntries([...LETTERS].map((letter) => [`Key${letter}`, letter])),
  ...Object.fromEntries(Array.from({ length: FUNCTION_KEYS }, (_, at) => `F${at + 1}`).map((name) => [name, name])),
};

/** Keys the browser keeps while the world has the keys: full screen and the developer tools. */
const BROWSER_KEYS = new Set(['F11', 'F12']);
/** Keys the browser keeps with Ctrl or the command key: reload, and copy, paste and cut. */
const BROWSER_SHORTCUTS = new Set(['R', 'C', 'V', 'X']);
/**
 * Keys the browser keeps with Ctrl or the command key and Shift: the hard
 * reload, and the developer tools, the inspector and the console.
 */
const BROWSER_SHIFT_SHORTCUTS = new Set(['R', 'I', 'C', 'J']);
/** Keys a Mac browser keeps with the command and the option keys: the developer tools, the console and the inspector. */
const MAC_OPTION_SHORTCUTS = new Set(['I', 'J', 'C']);
const MAC_PLATFORM = /Mac|iPhone|iPad|iPod/;

/** The egui name of `word` in `table`; null when it has none. */
function named(table: Record<string, string>, word: string): string | null {
  return Object.hasOwn(table, word) ? table[word] : null;
}

/**
 * The egui name of the key of `event`: by the key it gives, else by where
 * it sits, as egui-winit reads it. Null for a key egui does not know.
 */
export function keyName(event: KeyboardEvent): string | null {
  return named(EGUI_KEY, event.key) ?? named(EGUI_CODE, event.code);
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

/** The attribute that marks the field of the chat line, whose keys the view reads its own way. */
export const CHAT_ATTRIBUTE = 'data-chat';

/** Which field has the keys: none, the chat line, or another field. */
type FieldFocus = 'none' | 'chat' | 'other';

function focusOf(target: EventTarget | null): FieldFocus {
  if (!isField(target)) return 'none';
  return (target as HTMLElement).hasAttribute(CHAT_ATTRIBUTE) ? 'chat' : 'other';
}

/** True when `target` is a field that types the keys itself. */
export function isField(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement);
}

/** True when the browser keeps `key` for itself even while the world has the keys. */
function browserKeeps(key: string, held: Mods): boolean {
  const command = held.ctrl || held.command;
  // The command key without Ctrl is only a Mac's: elsewhere Ctrl+Alt stays the world's.
  const macOption = held.command && !held.ctrl && held.alt;
  if (macOption) return MAC_OPTION_SHORTCUTS.has(key);
  return BROWSER_KEYS.has(key) || (command && (held.shift ? BROWSER_SHIFT_SHORTCUTS : BROWSER_SHORTCUTS).has(key));
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
 * the keys and when it lets them go. While no field has the keys, the
 * browser does not act on a key the world reads (F1, F5, Alt+D, ...),
 * save the few it keeps (`browserKeeps`). Every held key comes up when the
 * page loses the keyboard. Gives the function that stops it.
 */
export function attachKeys(target: Window, send: (event: InputEvent) => void): () => void {
  const held = new Set<string>();
  let inField: FieldFocus = 'none';
  const focus = (field: FieldFocus) => {
    if (field === inField) return;
    inField = field;
    send({ kind: 'Focus', chat_focused: field === 'chat', other_field_focused: field === 'other' });
  };
  const focusIn = (event: FocusEvent) => focus(focusOf(event.target));
  const focusOut = (event: FocusEvent) => focus(focusOf(event.relatedTarget));
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
    if (!typing && !browserKeeps(key, heldMods)) event.preventDefault();
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
