import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Buffs } from '../../src/panels/Buffs';
import { Cast, Cooldowns } from '../../src/panels/Combat';
import { Counters } from '../../src/panels/Counters';
import { InfoBar } from '../../src/panels/InfoBar';
import type { CountersData } from '../../src/panels/types';
import { label } from './fixtures';

const counters: CountersData = {
  side: 40,
  gap: 4,
  columns: 2,
  cells: [
    { picture: null, amount: { words: '12', color: 'var(--text)' }, flashing: true, hover: label('Bandages'), zone: { counter: 0 } },
    { picture: null, amount: null, flashing: false, hover: label('Drop an item here to count it.'), zone: { counter: 1 } },
  ],
  fixed: { words: 'Fixed', color: 'var(--text-faint)' },
  fixed_hint: 'Fixed cells take no dropped items and do not empty.',
};

describe('Counters', () => {
  it('sends_clicks_double_clicks_and_alt_right_clicks_by_cell_and_names_its_drop_zones', () => {
    const send = vi.fn();
    const { getByText, container } = render(<Counters data={counters} send={send} />);
    const cell = container.querySelectorAll('.counter-cell')[0];
    fireEvent.click(cell);
    expect(send).toHaveBeenLastCalledWith({ click: 0 });
    fireEvent.dblClick(cell);
    expect(send).toHaveBeenLastCalledWith({ double: 0 });
    fireEvent.contextMenu(cell, { button: 2, altKey: true });
    expect(send).toHaveBeenLastCalledWith({ menu: { cell: 0, alt: true } });
    fireEvent.click(getByText('Fixed'));
    expect(send).toHaveBeenLastCalledWith({ fixed: true });
    expect(container.querySelectorAll('.counter-cell')[1].getAttribute('data-zone')).toBe('{"counter":1}');
    expect(cell.className).toContain('flashing');
    expect(getByText('12').style.color).toBe('var(--text)');
  });
});

describe('Buffs, cooldowns, cast and info bar', () => {
  it('show_their_words_in_the_colors_the_view_gives', () => {
    const buffs = render(<Buffs data={{ side: 34, icons: [{ picture: null, short: 'Prot', time: { words: '12s', color: 'var(--alarm)' }, hover: label('Protection') }] }} />);
    expect(buffs.getByText('Prot')).toBeTruthy();
    expect(buffs.getByText('12s').style.color).toBe('var(--alarm)');
    const bars = render(<Cooldowns data={{ bars: [{ label: 'Wither', seconds: '3.5s', fill: 0.5, color: 'rgba(1, 2, 3, 1)' }] }} />);
    expect(bars.getByText('3.5s')).toBeTruthy();
    const cast = render(<Cast data={{ name: { words: 'Clumsy', color: 'var(--alarm)' }, aside: '2 in range (12)', fill: 0.25 }} />);
    expect(cast.getByText('2 in range (12)')).toBeTruthy();
    const info = render(<InfoBar data={{ parts: [{ label: { words: 'Hits', color: 'var(--text)' }, words: { words: '50/50', color: 'var(--goal)' }, fill: 1, bar_color: 'var(--goal)' }] }} />);
    expect(info.getByText('50/50').style.color).toBe('var(--goal)');
    expect(info.container.querySelector('.bar-fill')).toBeTruthy();
  });
});
