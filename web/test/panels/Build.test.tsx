import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Build } from '../../src/panels/Build';
import type { BuildData } from '../../src/panels/types';

const data: BuildData = {
  kinds: [
    { words: 'Wall', chosen: true },
    { words: 'Roof', chosen: false },
  ],
  styles: [{ words: 'Stone', chosen: true }],
  no_parts: null,
  pieces: [
    { picture: '5', chosen: true },
    { picture: '6', chosen: false },
  ],
  wish_hint: 'Say the part',
  find: 'Find',
  buttons: {
    remove: { words: 'Remove', chosen: false },
    changes: ['Clear', 'Leave'],
    kept: ['Backup'],
    pick: { words: 'Pick', chosen: false },
    pick_tip: 'Click a part',
    floor_words: 'Floor',
    floors: [
      { words: '1', chosen: true },
      { words: '2', chosen: false },
    ],
    storeys: [{ words: 'Storey 1', look: 1, tip: 'how it shows' }],
  },
  counts: [{ words: 'Components 3/90', alarm: false }],
  counts_tip: 'Components | Fixtures',
  note: { words: 'Jev looks', failed: false },
};

describe('Build', () => {
  it('sends_each_pick_and_each_step_as_a_raw_fact', () => {
    const send = vi.fn();
    const { getByText, getByPlaceholderText, container } = render(<Build data={data} send={send} />);
    const steps: [string, Record<string, unknown>][] = [
      ['Roof', { kind: 1 }],
      ['Stone', { style: 0 }],
      ['Remove', { remove: true }],
      ['Clear', { change: 0 }],
      ['Leave', { change: 1 }],
      ['Backup', { keep: 0 }],
      ['Pick', { pick: true }],
      ['2', { floor: 2 }],
      ['Storey 1', { storey: 0 }],
    ];
    for (const [words, action] of steps) {
      fireEvent.click(getByText(words));
      expect(send).toHaveBeenLastCalledWith(action);
    }
    fireEvent.click(container.querySelectorAll('.build-piece')[1] as HTMLElement);
    expect(send).toHaveBeenLastCalledWith({ piece: 1 });
    const wish = getByPlaceholderText('Say the part');
    fireEvent.input(wish, { target: { value: 'a roof' } });
    fireEvent.click(getByText('Find'));
    expect(send).toHaveBeenLastCalledWith({ wish: 'a roof' });
    expect(getByText('Components 3/90')).toBeTruthy();
    expect(getByText('Jev looks')).toBeTruthy();
  });

  it('shows_no_steps_without_control', () => {
    const { queryByText } = render(<Build data={{ ...data, buttons: null }} send={vi.fn()} />);
    expect(queryByText('Remove')).toBeNull();
  });
});
