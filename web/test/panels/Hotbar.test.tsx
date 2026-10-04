import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Hotbar } from '../../src/panels/Hotbar';
import type { HotbarData } from '../../src/panels/types';
import { label } from './fixtures';

const data: HotbarData = {
  slots: [
    { words: 'Heal', key: '1', picture: null, tip: 'Heal', hover: label('Heal') },
    { words: '', key: '2', picture: null, tip: '', hover: label('2') },
  ],
  picking: null,
};

describe('Hotbar', () => {
  it('sends_the_slot_index_on_click', () => {
    const send = vi.fn();
    const { getByText } = render(<Hotbar data={data} picker={null} send={send} />);
    fireEvent.click(getByText('Heal'));
    expect(send).toHaveBeenCalledWith({ press: 0 });
  });

  it('clears_a_slot_on_a_right_click_and_opens_the_picker_of_an_empty_one', () => {
    const send = vi.fn();
    const { getByText } = render(<Hotbar data={data} picker={null} send={send} />);
    fireEvent.contextMenu(getByText('Heal'));
    expect(send).toHaveBeenCalledWith({ clear: 0 });
    fireEvent.click(getByText('2'));
    expect(send).toHaveBeenLastCalledWith({ pick: 1 });
  });

  it('puts_a_choice_of_the_picker_on_the_slot', () => {
    const send = vi.fn();
    const picker = { title: 'Put on slot 2', choices: ['bow'], no_macros: null };
    const { getByText } = render(<Hotbar data={{ ...data, picking: 1 }} picker={picker} send={send} />);
    fireEvent.click(getByText('bow'));
    expect(send).toHaveBeenCalledWith({ choose: 0 });
  });

  it('shuts_the_picker_on_a_click_away_from_the_bar', () => {
    const send = vi.fn();
    const picker = { title: 'Put on slot 2', choices: ['bow'], no_macros: null };
    render(<Hotbar data={{ ...data, picking: 1 }} picker={picker} send={send} />);
    fireEvent.pointerDown(document.body);
    expect(send).toHaveBeenCalledWith({ close_picker: true });
  });

  it('names_each_slot_as_a_zone_for_a_drop', () => {
    const { getByText } = render(<Hotbar data={data} picker={null} send={vi.fn()} />);
    const cell = getByText('2').closest('[data-zone]');
    expect(cell?.getAttribute('data-zone')).toBe(JSON.stringify({ slot: 1 }));
  });
});
