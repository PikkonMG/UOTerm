import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Grid } from '../../src/panels/Grid';
import type { GridData } from '../../src/panels/types';
import { label } from './fixtures';

const BAG = 1073741825;
const HATCHET = 1073741840;
const NO_MODS = { ctrl: false, alt: false, shift: false, command: false };

const grid: GridData = {
  serial: BAG,
  live: true,
  glass: 'var(--glass)',
  glass_opacity: 0.8,
  search: '',
  search_hint: 'search',
  count: '1',
  favorite: { words: 'Fav', color: 'var(--text-dim)', hint: 'Make this bag the favorite bag.' },
  loot_all: null,
  loot_bag: null,
  columns: 2,
  side: 46,
  gap: 4,
  art_scale: 1,
  cells: [
    {
      slot: 0,
      locked: true,
      item: {
        serial: HATCHET,
        picture: null,
        alpha: 1,
        amount: { words: '3', color: 'var(--text)' },
        mark: null,
        chosen: false,
        slider: 0.5,
        hover: { ...label('hatchet'), in_grid: true },
        zone: { into: HATCHET },
      },
    },
    { slot: 1, locked: false, item: null },
  ],
  strip: { buttons: ['Move here', 'Clear'], count: '1' },
  zone: { into: BAG },
};

describe('Grid', () => {
  it('sends_the_raw_facts_of_a_press_on_a_cell', () => {
    const send = vi.fn();
    const { getByText, container } = render(<Grid data={grid} send={send} />);
    const cell = getByText('3').closest('.grid-cell') as Element;
    fireEvent.click(cell, { detail: 1, shiftKey: true });
    expect(send).toHaveBeenLastCalledWith({ cell: { slot: 0, count: 1, mods: { ...NO_MODS, shift: true } } });
    fireEvent.dblClick(cell, { detail: 2 });
    expect(send).toHaveBeenLastCalledWith({ cell: { slot: 0, count: 2, double: true, mods: NO_MODS } });
    fireEvent.contextMenu(cell, { clientX: 7, clientY: 9 });
    expect(send).toHaveBeenLastCalledWith({ cell: { slot: 0, count: 1, secondary: true, x: 7, y: 9, mods: NO_MODS } });
    const empty = container.querySelectorAll('.grid-cell')[1];
    fireEvent.click(empty, { detail: 1 });
    expect(send).toHaveBeenLastCalledWith({ cell: { slot: 1, count: 1, mods: NO_MODS } });
  });

  it('searches_marks_its_zones_and_moves_the_chosen_items', () => {
    const send = vi.fn();
    const { getByPlaceholderText, getByText, container } = render(<Grid data={grid} send={send} />);
    fireEvent.input(getByPlaceholderText('search'), { target: { value: 'axe' } });
    expect(send).toHaveBeenLastCalledWith({ search: 'axe' });
    fireEvent.click(getByText('Fav'));
    expect(send).toHaveBeenLastCalledWith({ favorite: true });
    fireEvent.click(getByText('Clear'));
    expect(send).toHaveBeenLastCalledWith({ strip: 1 });
    expect(container.querySelector('.grid')?.getAttribute('data-zone')).toBe(JSON.stringify({ into: BAG }));
    expect(getByText('3').closest('.grid-cell')?.getAttribute('data-zone')).toBe(JSON.stringify({ into: HATCHET }));
    expect(container.querySelector('.lock-dot')).not.toBeNull();
  });

  it('sets_how_many_a_click_grabs_by_the_bar_of_a_pile', () => {
    const send = vi.fn();
    const { container } = render(<Grid data={grid} send={send} />);
    const bar = container.querySelector('.pile-bar') as HTMLElement;
    bar.getBoundingClientRect = () => ({ left: 0, width: 40, top: 0, height: 6, right: 40, bottom: 6, x: 0, y: 0, toJSON: () => ({}) });
    fireEvent.pointerDown(bar, { clientX: 10, button: 0 });
    expect(send).toHaveBeenLastCalledWith({ amount: { slot: 0, share: 0.25 } });
  });
});
