import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Trade } from '../../src/panels/Trade';
import type { TradeData } from '../../src/panels/types';
import { label } from './fixtures';

const BOX = 1073742592;
const SWORD = 1073742593;
const RING = 1073742594;
const data: TradeData = {
  live: true,
  sides: [
    { head: { words: 'You: not yet', color: 'var(--text-faint)' }, mine: true, items: [{ serial: SWORD, picture: null, amount: null, hover: label('sword') }] },
    { head: { words: 'Ann: accepted', color: 'var(--hits-poisoned)' }, mine: false, items: [{ serial: RING, picture: null, amount: '2', hover: label('ring') }] },
  ],
  coins: [
    { label: 'Gold', platinum: false, words: '0', owned: 'of 500' },
    { label: 'Platinum', platinum: true, words: '0', owned: 'of 0' },
  ],
  theirs: [
    { label: 'Gold', value: '250' },
    { label: 'Platinum', value: '0' },
  ],
  accept: { words: 'Accept', color: 'var(--goal)' },
  cancel: 'Cancel',
  zone: { into: BOX },
};

describe('Trade', () => {
  it('offers_gold_accepts_and_cancels', () => {
    const send = vi.fn();
    const { getByText, getAllByRole } = render(<Trade data={data} send={send} />);
    fireEvent.input(getAllByRole('textbox')[0], { target: { value: '900' } });
    expect(send).toHaveBeenLastCalledWith({ gold: '900' });
    fireEvent.input(getAllByRole('textbox')[1], { target: { value: '1' } });
    expect(send).toHaveBeenLastCalledWith({ platinum: '1' });
    fireEvent.click(getByText('Accept'));
    expect(send).toHaveBeenLastCalledWith({ accept: true });
    fireEvent.click(getByText('Cancel'));
    expect(send).toHaveBeenLastCalledWith({ cancel: true });
    expect(getByText('250')).toBeTruthy();
  });

  it('clicks_and_uses_a_traded_item_and_takes_drops_on_its_side', () => {
    const send = vi.fn();
    const { getByLabelText, container } = render(<Trade data={data} send={send} />);
    fireEvent.click(getByLabelText('ring'));
    expect(send).toHaveBeenLastCalledWith({ click: RING });
    fireEvent.dblClick(getByLabelText('ring'));
    expect(send).toHaveBeenLastCalledWith({ double: RING });
    expect(container.querySelector('.trade-window')?.getAttribute('data-zone')).toBe(JSON.stringify({ into: BOX }));
  });
});
