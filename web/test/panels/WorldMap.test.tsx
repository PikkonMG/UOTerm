import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { WorldMap } from '../../src/panels/WorldMap';
import type { WorldMapData } from '../../src/panels/types';

const data: WorldMapData = {
  live: true,
  view_words: 'World',
  goto_hint: 'x y',
  go: 'Look',
  walk: 'Walk',
  place: '1000, 1000',
  buttons: [
    { words: 'Redraw', hint: 'Draw again.' },
    { words: 'Reload', hint: 'Read again.' },
    { words: 'Markers', hint: 'The markers.' },
  ],
  side: { x: 400, y: 300 },
  land: null,
  tiles: [{ path: '/v1/map-picture/0/0/0?drawn=0', place: { x: -10, y: -20, w: 256, h: 256 } }],
  marks: [{ kind: 'words', at: { x: 5, y: 5 }, top: true, words: 'Bank', color: 'var(--waiting)' }],
  no_files: 'The map needs the client files.',
  hint: 'Click: walk there.',
  pan: '',
  mouse: '1001, 1002',
  zone: 'Britain',
  note: null,
};

describe('WorldMap', () => {
  it('turns_its_view_looks_walks_and_presses_its_buttons', () => {
    const send = vi.fn();
    const { getByText, getByPlaceholderText } = render(<WorldMap data={data} send={send} />);
    fireEvent.click(getByText('World'));
    expect(send).toHaveBeenLastCalledWith({ view: true });
    fireEvent.input(getByPlaceholderText('x y'), { target: { value: '1 2' } });
    fireEvent.click(getByText('Look'));
    expect(send).toHaveBeenLastCalledWith({ goto: { words: '1 2', walk: false } });
    fireEvent.click(getByText('Walk'));
    expect(send).toHaveBeenLastCalledWith({ goto: { words: '1 2', walk: true } });
    fireEvent.click(getByText('Markers'));
    expect(send).toHaveBeenLastCalledWith({ button: 2 });
    expect(getByText('Bank')).toBeTruthy();
    expect(getByText('Britain')).toBeTruthy();
  });

  it('lays_the_tiles_and_sends_clicks_and_the_wheel_in_points_of_the_field', () => {
    const send = vi.fn();
    const { container } = render(<WorldMap data={data} send={send} />);
    const tile = container.querySelector('img') as HTMLImageElement;
    expect(tile.getAttribute('src')).toBe(data.tiles[0].path);
    expect(tile.style.left).toBe('-10px');
    const field = container.querySelector('.map-field') as HTMLElement;
    fireEvent.pointerDown(field, { button: 0, clientX: 30, clientY: 40 });
    fireEvent.pointerUp(field, { button: 0, clientX: 30, clientY: 40, ctrlKey: true });
    expect(send).toHaveBeenLastCalledWith({ click: { x: 30, y: 40, ctrl: true } });
    fireEvent.wheel(field, { deltaY: -200, deltaMode: 0 });
    expect(send).toHaveBeenLastCalledWith({ wheel: 2 });
  });
});
