import { afterEach, describe, expect, it, vi } from 'vitest';
import type { InputEvent } from '../src/input/events';
import { attachKeys, CHAT_ATTRIBUTE, keyName, mods } from '../src/input/keys';

const press = (key: string, code: string, init: KeyboardEventInit = {}) => new KeyboardEvent('keydown', { key, code, ...init });

const NO_MODS = { ctrl: false, alt: false, shift: false, command: false };

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

describe('keyName', () => {
  it('maps_the_logical_key_not_the_key_position', () => {
    expect(keyName(press('a', 'KeyQ'))).toBe('A');
  });
  it('maps_function_and_arrow_keys_to_egui_names', () => {
    expect(keyName(press('F1', 'F1'))).toBe('F1');
    expect(keyName(press('F35', 'F35'))).toBe('F35');
    expect(keyName(press('ArrowUp', 'ArrowUp'))).toBe('Up');
    expect(keyName(press(' ', 'Space'))).toBe('Space');
    expect(keyName(press('Escape', 'Escape'))).toBe('Escape');
  });
  it('maps_digits_and_numpad_digits_to_the_digit_names', () => {
    expect(keyName(press('1', 'Digit1'))).toBe('1');
    expect(keyName(press('7', 'Numpad7'))).toBe('7');
  });
  it('maps_a_numpad_key_without_num_lock_to_the_key_it_gives', () => {
    expect(keyName(press('ArrowDown', 'Numpad2'))).toBe('Down');
  });
  it('maps_punctuation_to_egui_names', () => {
    expect(keyName(press(',', 'Comma'))).toBe('Comma');
    expect(keyName(press('!', 'Digit1', { shiftKey: true }))).toBe('Exclamationmark');
    expect(keyName(press('`', 'Backquote'))).toBe('Backtick');
  });
  it('falls_back_to_the_key_position_when_the_key_gives_no_egui_name', () => {
    expect(keyName(press('&', 'Digit1'))).toBe('1');
    expect(keyName(press('é', 'Digit2'))).toBe('2');
    expect(keyName(press('ф', 'KeyA'))).toBe('A');
    expect(keyName(press('Unidentified', 'NumpadEnter'))).toBe('Enter');
    expect(keyName(press('a', 'KeyQ'))).toBe('A');
  });
  it('ignores_keys_egui_does_not_know', () => {
    expect(keyName(press('Dead', 'IntlBackslash'))).toBeNull();
    expect(keyName(press('ContextMenu', 'ContextMenu'))).toBeNull();
    expect(keyName(press('constructor', 'toString'))).toBeNull();
  });
});

describe('mods', () => {
  it('reads_ctrl_as_command_off_mac', () => {
    expect(mods(press('a', 'KeyA', { ctrlKey: true })).command).toBe(true);
  });
  it('reads_the_command_key_as_command_on_mac', () => {
    vi.stubGlobal('navigator', { ...navigator, platform: 'MacIntel' });
    expect(mods(press('a', 'KeyA', { ctrlKey: true }))).toEqual({ ...NO_MODS, ctrl: true });
    expect(mods(press('a', 'KeyA', { metaKey: true })).command).toBe(true);
  });
});

describe('attachKeys', () => {
  it('sends_the_text_then_the_key_and_its_release', () => {
    const events: InputEvent[] = [];
    const detach = attachKeys(window, (event) => events.push(event));
    window.dispatchEvent(press('a', 'KeyA'));
    window.dispatchEvent(new KeyboardEvent('keyup', { key: 'a', code: 'KeyA' }));
    detach();
    expect(events).toEqual([
      { kind: 'Text', text: 'a' },
      { kind: 'Key', key: 'A', mods: NO_MODS, pressed: true, repeat: false },
      { kind: 'Key', key: 'A', mods: NO_MODS, pressed: false, repeat: false },
    ]);
  });

  it('lets_go_of_every_held_key_when_the_page_loses_the_keyboard', () => {
    const events: InputEvent[] = [];
    const detach = attachKeys(window, (event) => events.push(event));
    window.dispatchEvent(press('ArrowUp', 'ArrowUp'));
    window.dispatchEvent(new Event('blur'));
    detach();
    expect(events.at(-1)).toEqual({ kind: 'Key', key: 'Up', mods: NO_MODS, pressed: false, repeat: false });
  });

  it('keeps_tab_from_the_browser_and_types_no_text_into_the_world_from_a_field', () => {
    const events: InputEvent[] = [];
    const detach = attachKeys(window, (event) => events.push(event));
    const tab = press('Tab', 'Tab', { cancelable: true });
    window.dispatchEvent(tab);
    const field = document.createElement('input');
    document.body.append(field);
    const typed = new KeyboardEvent('keydown', { key: 'b', code: 'KeyB', bubbles: true, cancelable: true });
    field.dispatchEvent(typed);
    detach();
    expect(tab.defaultPrevented).toBe(true);
    expect(typed.defaultPrevented).toBe(false);
    expect(events).not.toContainEqual({ kind: 'Text', text: 'b' });
    expect(events).toContainEqual(expect.objectContaining({ kind: 'Key', key: 'B', pressed: true }));
  });

  it('tells_the_view_when_a_field_takes_the_keys_and_when_it_lets_them_go', () => {
    const events: InputEvent[] = [];
    const detach = attachKeys(window, (event) => events.push(event));
    const field = document.createElement('input');
    document.body.append(field);
    field.focus();
    field.blur();
    detach();
    expect(events).toEqual([
      { kind: 'Focus', chat_focused: false, other_field_focused: true },
      { kind: 'Focus', chat_focused: false, other_field_focused: false },
    ]);
  });

  it('tells_the_view_when_the_chat_line_has_the_keys', () => {
    const events: InputEvent[] = [];
    const detach = attachKeys(window, (event) => events.push(event));
    const chat = document.createElement('input');
    chat.setAttribute(CHAT_ATTRIBUTE, '');
    const other = document.createElement('input');
    document.body.append(chat, other);
    chat.focus();
    other.focus();
    chat.focus();
    detach();
    expect(events).toEqual([
      { kind: 'Focus', chat_focused: true, other_field_focused: false },
      { kind: 'Focus', chat_focused: false, other_field_focused: true },
      { kind: 'Focus', chat_focused: true, other_field_focused: false },
    ]);
  });

  it('lets_go_of_a_key_by_the_same_name_it_went_down_with', () => {
    const events: InputEvent[] = [];
    const detach = attachKeys(window, (event) => events.push(event));
    window.dispatchEvent(press('&', 'Digit1'));
    window.dispatchEvent(new KeyboardEvent('keyup', { key: '&', code: 'Digit1' }));
    detach();
    expect(events.filter((event) => event.kind === 'Key').map((event) => event.kind === 'Key' && event.key)).toEqual(['1', '1']);
  });

  it('keeps_every_key_of_the_world_from_the_browser', () => {
    const detach = attachKeys(window, () => {});
    const kept = [press('F1', 'F1'), press('F3', 'F3'), press('F5', 'F5'), press('d', 'KeyD', { altKey: true }), press('Tab', 'Tab')].map((event) => {
      const cancelable = new KeyboardEvent('keydown', { key: event.key, code: event.code, altKey: event.altKey, cancelable: true });
      window.dispatchEvent(cancelable);
      return cancelable.defaultPrevented;
    });
    detach();
    expect(kept).toEqual([true, true, true, true, true]);
  });

  it('leaves_the_browser_its_own_keys', () => {
    const detach = attachKeys(window, () => {});
    const keys: KeyboardEventInit[] = [
      { key: 'F11', code: 'F11' },
      { key: 'F12', code: 'F12' },
      { key: 'r', code: 'KeyR', ctrlKey: true },
      { key: 'I', code: 'KeyI', ctrlKey: true, shiftKey: true },
      { key: 'c', code: 'KeyC', ctrlKey: true },
      { key: 'v', code: 'KeyV', ctrlKey: true },
      { key: 'x', code: 'KeyX', ctrlKey: true },
    ];
    const prevented = keys.map((init) => {
      const event = new KeyboardEvent('keydown', { ...init, cancelable: true });
      window.dispatchEvent(event);
      return event.defaultPrevented;
    });
    detach();
    expect(prevented).toEqual(keys.map(() => false));
  });

  it('leaves_the_browser_its_hard_reload_and_developer_tools', () => {
    const detach = attachKeys(window, () => {});
    const keys: KeyboardEventInit[] = [
      { key: 'R', code: 'KeyR', ctrlKey: true, shiftKey: true },
      { key: 'C', code: 'KeyC', ctrlKey: true, shiftKey: true },
      { key: 'J', code: 'KeyJ', ctrlKey: true, shiftKey: true },
    ];
    const prevented = keys.map((init) => {
      const event = new KeyboardEvent('keydown', { ...init, cancelable: true });
      window.dispatchEvent(event);
      return event.defaultPrevented;
    });
    detach();
    expect(prevented).toEqual(keys.map(() => false));
  });

  it('leaves_the_mac_browser_its_developer_tools_and_keeps_ctrl_alt_for_the_world', () => {
    vi.stubGlobal('navigator', { ...navigator, platform: 'MacIntel' });
    const detach = attachKeys(window, () => {});
    const devTools = new KeyboardEvent('keydown', { key: 'ˆ', code: 'KeyI', metaKey: true, altKey: true, cancelable: true });
    window.dispatchEvent(devTools);
    vi.unstubAllGlobals();
    const worldChord = new KeyboardEvent('keydown', { key: 'i', code: 'KeyI', ctrlKey: true, altKey: true, cancelable: true });
    window.dispatchEvent(worldChord);
    detach();
    expect(devTools.defaultPrevented).toBe(false);
    expect(worldChord.defaultPrevented).toBe(true);
  });

  it('sends_no_text_for_a_key_held_with_ctrl', () => {
    const events: InputEvent[] = [];
    const detach = attachKeys(window, (event) => events.push(event));
    window.dispatchEvent(press('c', 'KeyC', { ctrlKey: true }));
    detach();
    expect(events).toEqual([{ kind: 'Key', key: 'C', mods: { ...NO_MODS, ctrl: true, command: true }, pressed: true, repeat: false }]);
  });
});
