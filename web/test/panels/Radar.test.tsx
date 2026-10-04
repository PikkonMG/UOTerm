import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Radar } from '../../src/panels/Radar';
import type { RadarData } from '../../src/panels/types';

const data: RadarData = {
  side: { x: 190, y: 190 },
  land: { path: '/v1/map/near/0/1000/1000', side: 256, matrix: [3, 3, -3, 3, 95, -673] },
  marks: [
    { kind: 'dot', at: { x: 95, y: 95 }, radius: 4, color: 'var(--self-figure)' },
    { kind: 'words', at: { x: 100, y: 95 }, top: false, words: 'Bank', color: 'var(--waiting)' },
  ],
  no_files: 'The radar needs the client files.',
  hint: 'Wheel: zoom. Double-click: larger or smaller.',
};

describe('Radar', () => {
  it('lays_the_land_by_the_matrix_of_the_view_and_draws_the_marks', () => {
    const { container, getByText } = render(<Radar data={data} send={vi.fn()} />);
    const land = container.querySelector('img') as HTMLImageElement;
    expect(land.getAttribute('src')).toBe(data.land.path);
    expect(land.style.transform).toBe('matrix(3, 3, -3, 3, 95, -673)');
    expect(container.querySelector('circle')?.getAttribute('fill')).toBe('var(--self-figure)');
    expect(getByText('Bank')).toBeTruthy();
  });

  it('zooms_with_the_wheel_and_grows_on_a_double_click', () => {
    const send = vi.fn();
    const { container } = render(<Radar data={data} send={send} />);
    const field = container.querySelector('.radar-field') as HTMLElement;
    fireEvent.wheel(field, { deltaY: -200, deltaMode: 0 });
    expect(send).toHaveBeenCalledWith({ wheel: 2 });
    fireEvent.dblClick(field);
    expect(send).toHaveBeenLastCalledWith({ double: true });
  });

  it('shows_the_words_when_the_land_does_not_come', () => {
    const { container, getByText } = render(<Radar data={data} send={vi.fn()} />);
    fireEvent.error(container.querySelector('img') as HTMLImageElement);
    expect(getByText(data.no_files)).toBeTruthy();
  });
});
