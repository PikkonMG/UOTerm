import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Shop } from '../../src/panels/Shop';
import type { ShopData } from '../../src/panels/types';
import { label } from './fixtures';

const data: ShopData = {
  live: true,
  goods: [{ serial: 5, picture: null, name: 'Bandage', left: 'x100', price: '2 gp', count: { words: '0', color: 'var(--text-dim)' }, hover: label('Bandage') }],
  total: 'Total  0 gp',
  gold: 'Gold  1,000',
  deal: 'Buy',
  clear: 'Clear',
  close: 'Close',
  step_down: '-',
  step_up: '+',
};

describe('Shop', () => {
  it('sends_take_one_on_double_click_and_take_all_with_shift', () => {
    const send = vi.fn();
    const { getByText } = render(<Shop data={data} send={send} />);
    fireEvent.dblClick(getByText('Bandage'));
    expect(send).toHaveBeenLastCalledWith({ take: 5, all: false });
    fireEvent.dblClick(getByText('Bandage'), { shiftKey: true });
    expect(send).toHaveBeenLastCalledWith({ take: 5, all: true });
  });

  it('steps_names_and_deals', () => {
    const send = vi.fn();
    const { getByText } = render(<Shop data={data} send={send} />);
    fireEvent.click(getByText('+'), { shiftKey: true });
    expect(send).toHaveBeenLastCalledWith({ step: { serial: 5, up: true, big: true } });
    fireEvent.click(getByText('-'));
    expect(send).toHaveBeenLastCalledWith({ step: { serial: 5, up: false, big: false } });
    fireEvent.click(getByText('Bandage'));
    expect(send).toHaveBeenLastCalledWith({ click: 5 });
    fireEvent.click(getByText('Buy'));
    expect(send).toHaveBeenLastCalledWith({ deal: true });
    fireEvent.click(getByText('Clear'));
    expect(send).toHaveBeenLastCalledWith({ clear: true });
    fireEvent.click(getByText('Close'));
    expect(send).toHaveBeenLastCalledWith({ close: true });
    expect(getByText(/^Total/)).toBeTruthy();
  });
});
