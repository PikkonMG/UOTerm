import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { ControlBar } from '../../src/panels/ControlBar';
import type { ControlBarData } from '../../src/panels/types';

const data: ControlBarData = {
  place: { x: 200, y: 16, w: 880, h: 100 },
  location: { numbers: '1000, 1000, 0', map_words: 'map', map: '0', faces_words: 'faces', facing: 'north' },
  folded: false,
  launcher_words: 'Panels',
  launcher_shows: false,
  status: null,
  buttons: ['Take control', 'Sheet', 'Quit'],
};

describe('ControlBar', () => {
  it('sends_the_words_of_the_button_pressed', () => {
    const send = vi.fn();
    const { getByText } = render(<ControlBar data={data} send={send} />);
    fireEvent.click(getByText('Take control'));
    expect(send).toHaveBeenCalledWith({ press: 'Take control' });
    fireEvent.click(getByText('Quit'));
    expect(send).toHaveBeenLastCalledWith({ press: 'Quit' });
  });

  it('shows_where_the_character_is_and_opens_the_launcher', () => {
    const send = vi.fn();
    const { getByText } = render(<ControlBar data={{ ...data, status: 'You have control.' }} send={send} />);
    expect(getByText('1000, 1000, 0')).toBeTruthy();
    expect(getByText('You have control.')).toBeTruthy();
    fireEvent.click(getByText('Panels'));
    expect(send).toHaveBeenCalledWith({ launcher: true });
  });

  it('folds_by_its_arrow', () => {
    const send = vi.fn();
    const { getByRole } = render(<ControlBar data={data} send={send} />);
    fireEvent.click(getByRole('button', { expanded: true }));
    expect(send).toHaveBeenCalledWith({ fold: true });
  });
});
