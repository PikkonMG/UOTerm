import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Tip } from '../../src/panels/Tip';

describe('Tip', () => {
  it('shows_the_words_and_asks_for_another_tip', () => {
    const send = vi.fn();
    const { getByText } = render(<Tip data={{ words: '<b>Hail</b>', previous: 'Previous', next: 'Next' }} send={send} />);
    expect(getByText('<b>Hail</b>')).toBeTruthy();
    fireEvent.click(getByText('Previous'));
    expect(send).toHaveBeenLastCalledWith({ previous: true });
    fireEvent.click(getByText('Next'));
    expect(send).toHaveBeenLastCalledWith({ next: true });
  });
});
