import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Dye } from '../../src/panels/Dye';
import { HuePicker } from '../../src/panels/HuePicker';
import type { DyeData, HueGridData } from '../../src/panels/types';

const grid: HueGridData = {
  columns: 2,
  cells: ['rgba(1, 1, 1, 1)', 'rgba(2, 2, 2, 1)', 'rgba(3, 3, 3, 1)', 'rgba(4, 4, 4, 1)'],
  chosen: 1,
  shade: 1,
  shade_least: 0,
  shade_most: 4,
  shade_words: 'Shade',
};

describe('HuePicker', () => {
  it('picks_a_cell_and_shifts_the_shade', () => {
    const send = vi.fn();
    const { getAllByRole, getByRole } = render(<HuePicker data={grid} live send={send} />);
    const cells = getAllByRole('button');
    expect(cells[1].classList.contains('chosen')).toBe(true);
    fireEvent.click(cells[2]);
    expect(send).toHaveBeenLastCalledWith({ cell: 2 });
    fireEvent.input(getByRole('slider'), { target: { value: '3' } });
    expect(send).toHaveBeenLastCalledWith({ shade: 3 });
  });
});

describe('Dye', () => {
  it('sends_the_hue_and_starts_the_eyedropper', () => {
    const send = vi.fn();
    const data: DyeData = { live: true, grid, tub: null, hue: '7', okay: 'Okay', eyedropper: { words: 'Eyedropper', color: 'var(--text)' } };
    const { getByText } = render(<Dye data={data} send={send} />);
    expect(getByText('7')).toBeTruthy();
    fireEvent.click(getByText('Okay'));
    expect(send).toHaveBeenLastCalledWith({ okay: true });
    fireEvent.click(getByText('Eyedropper'));
    expect(send).toHaveBeenLastCalledWith({ eyedropper: true });
  });
});
