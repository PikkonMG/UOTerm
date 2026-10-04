import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { MapItem } from '../../src/panels/MapItem';
import type { MapItemData } from '../../src/panels/types';

const data: MapItemData = {
  live: true,
  path: '/v1/map-item/1/1000/1200/1400/1600',
  paper: { x: 200, y: 200 },
  land: { x: 196, y: 196 },
  edge: 2,
  pins: [
    { number: '1', at: { x: 40, y: 90 } },
    { number: '2', at: { x: 100, y: 20 } },
  ],
  pin_radius: 4,
  pin_ring: 1.5,
  course_width: 1.5,
  plotting: true,
  pin_hint: 'Click: put a pin here.',
  pin_move_hint: 'Drag: move the pin.',
  no_files: 'The picture needs the client files.',
  wish_hint: 'Say the place',
  mark: 'Mark',
  buttons: [
    { words: 'Clear pins', waiting: false },
    { words: 'Stop plotting', waiting: true },
    { words: 'Close', waiting: false },
  ],
  note: null,
};

describe('MapItem', () => {
  it('puts_moves_and_takes_off_pins_and_presses_its_buttons', () => {
    const send = vi.fn();
    const { container, getByText, getByPlaceholderText } = render(<MapItem data={data} send={send} />);
    expect((container.querySelector('img') as HTMLImageElement).getAttribute('src')).toBe(data.path);
    const land = container.querySelector('.map-item-land') as HTMLElement;
    fireEvent.click(land, { clientX: 50, clientY: 60 });
    expect(send).toHaveBeenLastCalledWith({ pin: { x: 50, y: 60 } });
    const pins = container.querySelectorAll('.map-pin');
    fireEvent.dblClick(pins[0] as Element);
    expect(send).toHaveBeenLastCalledWith({ remove_pin: 0 });
    fireEvent.pointerDown(pins[1] as Element, { button: 0, clientX: 100, clientY: 20 });
    fireEvent.pointerMove(window, { clientX: 120, clientY: 30 });
    fireEvent.pointerUp(window, { clientX: 120, clientY: 30 });
    expect(send).toHaveBeenLastCalledWith({ move_pin: { pin: 1, x: 120, y: 30 } });
    fireEvent.click(getByText('Stop plotting'));
    expect(send).toHaveBeenLastCalledWith({ button: 1 });
    fireEvent.input(getByPlaceholderText('Say the place'), { target: { value: 'bank' } });
    fireEvent.click(getByText('Mark'));
    expect(send).toHaveBeenLastCalledWith({ wish: 'bank' });
  });
});
