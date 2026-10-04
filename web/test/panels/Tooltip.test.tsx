import { render } from '@testing-library/preact';
import { describe, expect, it } from 'vitest';
import { Tooltip } from '../../src/panels/Tooltip';

describe('Tooltip', () => {
  it('shows_the_lines_and_what_a_click_does_beside_the_mouse', () => {
    const { getByText, container } = render(<Tooltip data={{ lines: ['a hatchet', 'Weight: 4'], footer: 'Click: use.' }} at={{ x: 50, y: 60 }} />);
    expect(getByText('a hatchet').classList.contains('tip-name')).toBe(true);
    expect(getByText('Click: use.')).toBeTruthy();
    const tip = container.firstElementChild as HTMLElement;
    expect(tip.style.left).toBe('50px');
  });
});
