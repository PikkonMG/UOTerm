import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Ring } from '../../src/panels/Ring';

const data = {
  center: { x: 300, y: 300 },
  name: 'an orc',
  lines: [
    { words: 'Look', enabled: true, at: { x: 300, y: 204 } },
    { words: 'Trade', enabled: false, at: { x: 396, y: 300 } },
  ],
};

describe('Ring', () => {
  it('picks_a_line_and_a_click_away_closes', () => {
    const send = vi.fn();
    const { getByText, getByTestId } = render(<Ring data={data} send={send} />);
    fireEvent.click(getByText('Look'));
    expect(send).toHaveBeenCalledWith({ pick: 0 });
    fireEvent.click(getByText('Trade'));
    expect(send).toHaveBeenCalledTimes(1);
    fireEvent.pointerDown(getByTestId('ring-away'));
    expect(send).toHaveBeenLastCalledWith({ close: true });
  });
});
