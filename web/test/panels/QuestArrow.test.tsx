import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { QuestArrow } from '../../src/panels/QuestArrow';

describe('QuestArrow', () => {
  it('tells_the_shard_of_a_left_or_a_right_click', () => {
    const send = vi.fn();
    const { container } = render(<QuestArrow data={{ place: { x: 10, y: 20, w: 16, h: 16 }, hint: 'Click: tell the shard.' }} send={send} />);
    const arrow = container.querySelector('.quest-arrow') as HTMLElement;
    expect(arrow.style.left).toBe('10px');
    fireEvent.click(arrow);
    expect(send).toHaveBeenLastCalledWith({ click: { right: false } });
    fireEvent.contextMenu(arrow);
    expect(send).toHaveBeenLastCalledWith({ click: { right: true } });
  });
});
