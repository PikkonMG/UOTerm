import { render } from '@testing-library/preact';
import { describe, expect, it } from 'vitest';
import { Plates } from '../../src/panels/Plates';

describe('Plates', () => {
  it('places_each_name_and_the_words_over_heads_where_the_view_laid_them', () => {
    const plates = [
      {
        area: { min: { x: 10, y: 20 }, max: { x: 70, y: 40 } },
        name_at: { x: 14, y: 22 },
        name: 'an orc',
        name_color: [244, 72, 128, 255] as [number, number, number, number],
        bar: null,
      },
    ];
    const floats = [{ words: 'hail', x: 100, y: 50, color: [102, 102, 102, 102] as [number, number, number, number], alpha: 0.5, number: false }];
    const { getByText } = render(<Plates plates={plates} floats={floats} scale={1} />);
    const name = getByText('an orc');
    expect(name.style.color).toBe('rgb(244, 72, 128)');
    expect(name.parentElement?.style.left).toBe('10px');
    const hail = getByText('hail');
    expect(hail.style.color).toBe('rgb(255, 255, 255)');
    expect(hail.style.opacity).toBe('0.2');
  });
});
