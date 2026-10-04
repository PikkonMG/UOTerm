import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Macros } from '../../src/panels/Macros';
import type { MacrosData } from '../../src/panels/types';

const editor: MacrosData = {
  place: { x: 100, y: 50, w: 820, h: 520 },
  title: 'Macros',
  status: 'running',
  names: [
    { words: 'heal', chosen: true },
    { words: 'mine', chosen: false },
  ],
  name: 'heal',
  lines: 'bandageself\n',
  wish: '',
  name_hint: 'Macro name',
  lines_hint: 'One command on each line.',
  wish_hint: 'Say the next step in plain words',
  add_line: { words: 'Add line', color: 'var(--goal)' },
  buttons: [
    { words: 'New', color: 'var(--text)', enabled: true },
    { words: 'Save', color: 'var(--text-faint)', enabled: false },
  ],
  note: { words: 'Jev looks for the hotkey...', failed: false },
};

describe('Macros', () => {
  it('picks_a_macro_keeps_its_fields_in_the_view_and_presses_the_buttons', () => {
    const send = vi.fn();
    const { getByText, getByPlaceholderText } = render(<Macros data={editor} send={send} />);
    fireEvent.click(getByText('mine'));
    expect(send).toHaveBeenLastCalledWith({ pick: 1 });
    fireEvent.input(getByPlaceholderText('Macro name'), { target: { value: 'heal2' } });
    expect(send).toHaveBeenLastCalledWith({ name: 'heal2' });
    fireEvent.input(getByPlaceholderText('Say the next step in plain words'), { target: { value: 'drink a cure' } });
    expect(send).toHaveBeenLastCalledWith({ wish: 'drink a cure' });
    fireEvent.click(getByText('Add line'));
    expect(send).toHaveBeenLastCalledWith({ add_line: true });
    fireEvent.click(getByText('New'));
    expect(send).toHaveBeenLastCalledWith({ button: 0 });
    expect((getByText('Save') as HTMLButtonElement).disabled).toBe(true);
    expect(getByText('Jev looks for the hotkey...')).toBeTruthy();
    expect(getByText('heal').closest('[data-panel]')?.getAttribute('data-panel')).toBe('macros');
  });
});
