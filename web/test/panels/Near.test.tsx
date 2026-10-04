import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { setDragDistance } from '../../src/panels/drag';
import { Near } from '../../src/panels/Near';
import { TargetBar } from '../../src/panels/TargetBar';
import type { HealthBarData, NearData } from '../../src/panels/types';
import { label } from './fixtures';

const ORC = 2817;
const near: NearData = {
  rows: [
    {
      serial: ORC,
      name: { words: 'an orc', color: 'var(--text)' },
      title: '',
      color: 'var(--noto-murderer)',
      hits: 0.5,
      distance: 3,
      hover: label('an orc'),
      zone: { into: ORC },
    },
  ],
  nobody: null,
};

describe('Near', () => {
  it('sends_the_clicks_of_a_row_as_the_near_list_of_the_window', () => {
    const send = vi.fn();
    const { getByText } = render(<Near data={near} send={send} />);
    const row = getByText('an orc');
    fireEvent.click(row);
    expect(send).toHaveBeenCalledWith({ click: ORC });
    fireEvent.dblClick(row);
    expect(send).toHaveBeenCalledWith({ double: ORC });
    fireEvent.contextMenu(row, { clientX: 30, clientY: 40 });
    expect(send).toHaveBeenLastCalledWith({ menu: { serial: ORC, x: 30, y: 40 } });
  });

  it('pulls_a_row_dragged_out_into_a_bar_where_it_is_let_go', () => {
    setDragDistance(6);
    const send = vi.fn();
    const { getByText } = render(<Near data={near} send={send} />);
    fireEvent.pointerDown(getByText('an orc'), { clientX: 10, clientY: 10, button: 0 });
    fireEvent.pointerMove(window, { clientX: 200, clientY: 300 });
    fireEvent.pointerUp(window, { clientX: 200, clientY: 300 });
    expect(send).toHaveBeenLastCalledWith({ pull: { serial: ORC, x: 200, y: 300 } });
  });
});

const bar: HealthBarData = {
  serial: ORC,
  lines: [{ share: 0.25, color: 'var(--hits)' }],
  party: ['Heal', 'Cure'],
  target: null,
  rename: null,
  close: 'Close bar',
  zone: { into: ORC },
};

describe('TargetBar', () => {
  it('casts_from_its_party_buttons_and_closes_from_its_menu', () => {
    const send = vi.fn();
    const { getByText, container } = render(<TargetBar data={bar} send={send} />);
    expect((container.querySelector('.bar-fill') as HTMLElement).style.width).toBe('25%');
    fireEvent.click(getByText('Heal'));
    expect(send).toHaveBeenCalledWith({ heal: true });
    fireEvent.contextMenu(container.querySelector('.health-lines') as HTMLElement);
    fireEvent.click(getByText('Close bar'));
    expect(send).toHaveBeenLastCalledWith({ close: true });
  });
});
