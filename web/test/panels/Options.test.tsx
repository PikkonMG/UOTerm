import { fireEvent, render, waitFor } from '@testing-library/preact';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ColorPicker } from '../../src/panels/ColorPicker';
import { KeyCapture } from '../../src/panels/KeyCapture';
import { Options } from '../../src/panels/Options';
import type { HueData, KeysData, OptionsData } from '../../src/panels/types';

const hue: HueData = { words: '0x0035', color: 'rgba(1, 2, 3, 1)', hint: 'Pick the color.' };
const keys: KeysData = {
  macros: [
    {
      name: 'heal',
      chord: 'Press a key',
      pad: 'No button',
      bound: false,
      open: true,
      steps: [{ action: 'Say', action_id: 'say', argument: 'hi', typed: true, hint: 'words', choices: [], shown: 'Pick' }],
    },
  ],
  groups: [{ label: 'Speech', actions: [{ id: 'say', label: 'Say' }] }],
  defaults: [{ title: 'Default keys', lines: ['Alt+P: Open: Paperdoll'] }],
  name_hint: 'macro name',
  clear: 'Clear',
  steps: 'Steps',
  up: 'Up',
  down: 'Down',
  add_step: 'Add step',
  add: 'Add macro',
};

const options = (rows: OptionsData['rows']): OptionsData => ({
  pages: [
    { words: 'General', chosen: true },
    { words: 'Sound', chosen: false },
  ],
  rows,
  foot: ['Cancel', 'Apply', 'Default', 'Okay', 'Save as default'].map((words) => ({ words, color: 'var(--text)' })),
  default_hint: 'Puts this page back to its defaults.',
  default_at: 2,
  remove: 'Remove',
});

afterEach(() => vi.restoreAllMocks());

describe('Options', () => {
  it('sends_each_change_of_a_row_by_its_label_and_the_buttons_of_the_foot', () => {
    const send = vi.fn();
    const rows: OptionsData['rows'] = [
      { section: 'Movement', label: 'Always run', control: { kind: 'toggle', on: false } },
      { section: null, label: 'Zoom', control: { kind: 'slider', min: 0.5, max: 2, step: 0.05, value: 1, words: 'x1.00' } },
      { section: null, label: 'Window mode', control: { kind: 'choice', labels: ['Windowed', 'Borderless', 'Full screen'], index: 0 } },
      { section: null, label: 'Speech hue', control: { kind: 'hue', hue } },
    ];
    const { getByText, getByLabelText, container } = render(<Options data={options(rows)} send={send} />);
    fireEvent.click(getByText('Sound'));
    expect(send).toHaveBeenLastCalledWith({ page: 1 });
    fireEvent.click(getByLabelText('Always run'));
    expect(send).toHaveBeenLastCalledWith({ set: { row: 'Always run', value: true } });
    fireEvent.input(container.querySelector('input[type=range]') as Element, { target: { value: '1.5' } });
    expect(send).toHaveBeenLastCalledWith({ set: { row: 'Zoom', value: 1.5 } });
    fireEvent.change(container.querySelector('select') as Element, { target: { value: '2' } });
    expect(send).toHaveBeenLastCalledWith({ set: { row: 'Window mode', value: 2 } });
    fireEvent.click(getByLabelText('Pick the color.'));
    expect(send).toHaveBeenLastCalledWith({ swatch: { row: 'Speech hue', at: 0 } });
    fireEvent.click(getByText('Apply'));
    expect(send).toHaveBeenLastCalledWith({ foot: 1 });
    expect(getByText('Default').title).toBe('Puts this page back to its defaults.');
    expect(getByText('Movement')).toBeTruthy();
  });

  it('edits_the_macros_steps_and_waits_for_a_key', () => {
    const send = vi.fn();
    const rows: OptionsData['rows'] = [{ section: null, label: 'Macros', control: { kind: 'keys', ...keys } }];
    const { getByText, getByDisplayValue } = render(<Options data={options(rows)} send={send} />);
    expect(getByText('Press a key')).toBeTruthy();
    fireEvent.click(getByText('No button'));
    expect(send).toHaveBeenLastCalledWith({ capture: { at: 0, pad: true } });
    fireEvent.change(getByDisplayValue('hi'), { target: { value: 'hail' } });
    expect(send).toHaveBeenLastCalledWith({ step_argument: { at: 0, step: 0, words: 'hail' } });
    fireEvent.click(getByText('Up'));
    expect(send).toHaveBeenLastCalledWith({ step_move: { at: 0, step: 0, up: true } });
    fireEvent.click(getByText('Add macro'));
    expect(send).toHaveBeenLastCalledWith({ add: 'Macros' });
    expect(getByText('Alt+P: Open: Paperdoll')).toBeTruthy();
  });

  it('edits_an_entry_of_a_list_by_its_field', () => {
    const send = vi.fn();
    const rows: OptionsData['rows'] = [
      { section: null, label: 'Items', control: { kind: 'info_items', items: [{ label: 'Health', hue, data: 0 }], data_labels: ['Hits', 'Mana'], hint: 'label', add: 'Add item' } },
    ];
    const { getByDisplayValue, getByText } = render(<Options data={options(rows)} send={send} />);
    fireEvent.change(getByDisplayValue('Health'), { target: { value: 'HP' } });
    expect(send).toHaveBeenLastCalledWith({ field: { row: 'Items', at: 0, name: 'label', value: 'HP' } });
    fireEvent.change(getByDisplayValue('0x0035'), { target: { value: '0x0021' } });
    expect(send).toHaveBeenLastCalledWith({ field: { row: 'Items', at: 0, name: 'hue', value: '0x0021' } });
    fireEvent.click(getByText('Remove'));
    expect(send).toHaveBeenLastCalledWith({ remove: { row: 'Items', at: 0 } });
  });

  it('offers_the_player_fonts_of_the_server_for_the_font_file', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(JSON.stringify(['Avadonian.ttf']), { headers: { 'content-type': 'application/json' } }));
    const send = vi.fn();
    const rows: OptionsData['rows'] = [{ section: null, label: 'TrueType font file', control: { kind: 'file', words: '', hint: 'no file', fonts: true } }];
    const { getByText, container } = render(<Options data={options(rows)} send={send} />);
    await waitFor(() => expect(getByText('Avadonian.ttf')).toBeTruthy());
    fireEvent.change(container.querySelector('select') as Element, { target: { value: 'Avadonian.ttf' } });
    expect(send).toHaveBeenLastCalledWith({ set: { row: 'TrueType font file', value: 'Avadonian.ttf' } });
  });
});

describe('KeyCapture and ColorPicker', () => {
  it('asks_for_a_key_and_picks_a_hue', () => {
    const send = vi.fn();
    const key = render(<KeyCapture words="No key" at={2} pad={false} send={send} />);
    fireEvent.click(key.getByText('No key'));
    expect(send).toHaveBeenLastCalledWith({ capture: { at: 2, pad: false } });
    const grid = { columns: 2, cells: ['red', 'blue'], chosen: 0, shade: 1, shade_least: 0, shade_most: 4, shade_words: 'Shade' };
    const picker = render(
      <ColorPicker data={{ grid, color: 'red', words: '0x0035', eyedropper: { words: 'Eyedropper', color: 'var(--text)' }, okay: 'Okay', cancel: 'Cancel' }} send={send} />,
    );
    fireEvent.click(picker.getByLabelText('hue 2'));
    expect(send).toHaveBeenLastCalledWith({ cell: 1 });
    fireEvent.click(picker.getByText('Okay'));
    expect(send).toHaveBeenLastCalledWith({ okay: true });
  });
});
