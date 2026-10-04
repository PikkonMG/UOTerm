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
    expect(send).toHaveBeenCalledWith({ click: 0 });
  });

  it('clears_a_slot_on_a_right_click_and_sends_a_click_of_an_empty_one', () => {
    const send = vi.fn();
    const { getByText } = render(<Hotbar data={data} picker={null} send={send} />);
    fireEvent.contextMenu(getByText('Heal'));
    expect(send).toHaveBeenCalledWith({ clear: 0 });
    fireEvent.click(getByText('2'));
    expect(send).toHaveBeenLastCalledWith({ click: 1 });
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

  it('clears_a_picture_whose_slot_was_emptied', () => {
    const shown: HotbarData = { ...data, slots: [{ ...data.slots[0], picture: 'kept' }] };
    const { container, rerender } = render(<Hotbar data={shown} picker={null} send={vi.fn()} />);
    const canvas = container.querySelector('canvas') as HTMLCanvasElement;
    canvas.dataset.drawn = 'kept';
    canvas.width = 4;
    rerender(<Hotbar data={{ ...shown, slots: [{ ...shown.slots[0], picture: 'other' }] }} picker={null} send={vi.fn()} />);
    expect(canvas.width).toBe(0);
    expect(canvas.dataset.drawn).toBeUndefined();
  });
});
