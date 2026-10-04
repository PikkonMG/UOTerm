import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Loot } from '../../src/panels/Loot';
import type { LootData } from '../../src/panels/types';

const CORPSE = 1073742336;
const data: LootData = {
  live: true,
  rows: [{ serial: CORPSE, name: 'a corpse', state: 'shut  2' }],
  none: null,
  open: 'Open',
  loot: 'Loot',
  loot_all: 'Loot all in reach',
};

describe('Loot', () => {
  it('opens_loots_and_loots_all_near', () => {
    const send = vi.fn();
    const { getByText } = render(<Loot data={data} send={send} />);
    expect(getByText(/^shut/)).toBeTruthy();
    fireEvent.click(getByText('Open'));
    expect(send).toHaveBeenLastCalledWith({ open: CORPSE });
    fireEvent.click(getByText('Loot'));
    expect(send).toHaveBeenLastCalledWith({ loot: CORPSE });
    fireEvent.click(getByText('Loot all in reach'));
    expect(send).toHaveBeenLastCalledWith({ loot_all: true });
  });

  it('says_no_corpse_is_near_and_has_no_buttons_without_control', () => {
    const { getByText, queryByText } = render(<Loot data={{ ...data, live: false, rows: [], none: 'No corpse near.', loot_all: null }} send={vi.fn()} />);
    expect(getByText('No corpse near.')).toBeTruthy();
    expect(queryByText('Open')).toBeNull();
  });
});
