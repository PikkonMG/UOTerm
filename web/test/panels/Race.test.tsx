import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Race } from '../../src/panels/Race';
import type { RaceData } from '../../src/panels/types';

const race: RaceData = {
  live: true,
  styles: [{ label: 'Hair Style', choices: ['Bald', 'Long'], chosen: 0 }],
  figure: null,
  paints: [{ label: 'Skin Tone', color: 'rgba(1, 2, 3, 1)', picking: true }],
  hint: 'Click: pick from the palette.',
  palette: { columns: 2, hues: ['rgba(4, 5, 6, 1)', 'rgba(7, 8, 9, 1)'], chosen: 0 },
  change: 'Change',
  keep: 'Keep my looks',
};

describe('Race', () => {
  it('picks_styles_and_colors_and_sends_the_looks', () => {
    const send = vi.fn();
    const { getByRole, getByLabelText, getAllByRole, getByText } = render(<Race data={race} send={send} />);
    fireEvent.change(getByRole('combobox'), { target: { value: '1' } });
    expect(send).toHaveBeenLastCalledWith({ style: { list: 0, at: 1 } });
    fireEvent.click(getByLabelText('Skin Tone'));
    expect(send).toHaveBeenLastCalledWith({ paint: 0 });
    fireEvent.click(getAllByRole('button', { name: /hue/ })[1]);
    expect(send).toHaveBeenLastCalledWith({ hue: 1 });
    fireEvent.click(getByText('Change'));
    expect(send).toHaveBeenLastCalledWith({ change: true });
    fireEvent.click(getByText('Keep my looks'));
    expect(send).toHaveBeenLastCalledWith({ keep: true });
  });
});
