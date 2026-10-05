import { act, fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { ShardGump } from '../../src/panels/ShardGump';
import type { ShardGumpData } from '../../src/panels/types';

const MARKUP = '<script>alert(1)</script>';

const gump: ShardGumpData = {
  panel: 'gump:77',
  at: { x: 40, y: 30 },
  size: { x: 300, y: 200 },
  movable: true,
  closable: true,
  live: true,
  pieces: [
    { kind: 'pictures', pictures: [{ at: { x: 0, y: 0 }, size: { x: 300, y: 200 }, picture: '1', tiled: true }], alpha: 1, tip: null },
    { kind: 'choice', piece: 0, at: { x: 10, y: 10 }, size: { x: 20, y: 20 }, picture: '2', alpha: 1, tip: null },
    { kind: 'entry', id: 9, at: { x: 10, y: 40 }, size: { x: 100, y: 20 }, words: 'old', color: 'rgba(1, 2, 3, 1)', most: 4, focus: false, alpha: 1 },
    { kind: 'button', piece: 2, at: { x: 10, y: 70 }, size: { x: 30, y: 20 }, normal: '3', pressed: '4', item: null, alpha: 1, tip: { serial: null, words: 'Okay', footer: '' } },
    {
      kind: 'html',
      at: { x: 10, y: 100 },
      size: { x: 200, y: 60 },
      paper: [],
      pad: 0,
      width: 200,
      scroll: true,
      background: null,
      line_height: 18,
      lines: [
        {
          left: 0,
          spans: [
            { words: MARKUP, color: 'rgba(1, 1, 1, 1)', bold: false, italic: false, underline: false, size: 14 },
            { words: 'bold', color: 'rgba(1, 1, 1, 1)', bold: true, italic: false, underline: false, size: 14 },
          ],
        },
      ],
      alpha: 0.5,
      tip: null,
    },
    { kind: 'veil', at: { x: 0, y: 0 }, size: { x: 50, y: 50 }, color: 'rgba(0, 0, 0, 0.5)' },
  ],
};

describe('ShardGump', () => {
  it('presses_a_button_ticks_a_box_and_types_in_a_field', () => {
    const send = vi.fn();
    const { container, getByDisplayValue } = render(<ShardGump data={gump} scale={1} send={send} />);
    fireEvent.click(container.querySelector('[data-piece="2"]') as HTMLElement);
    expect(send).toHaveBeenLastCalledWith({ button: 2 });
    fireEvent.click(container.querySelector('[data-piece="0"]') as HTMLElement);
    expect(send).toHaveBeenLastCalledWith({ tick: 0 });
    const field = getByDisplayValue('old') as HTMLInputElement;
    expect(field.maxLength).toBe(4);
    fireEvent.input(field, { target: { value: 'new' } });
    expect(send).toHaveBeenLastCalledWith({ field: { id: 9, words: 'new' } });
  });

  it('answers_with_no_button_on_a_right_click_only_when_it_may', () => {
    const send = vi.fn();
    const { container, rerender } = render(<ShardGump data={gump} scale={1} send={send} />);
    const root = container.querySelector('.shard-gump') as HTMLElement;
    fireEvent.contextMenu(root);
    expect(send).toHaveBeenLastCalledWith({ close: true });
    send.mockClear();
    rerender(<ShardGump data={{ ...gump, closable: false }} scale={1} send={send} />);
    fireEvent.contextMenu(container.querySelector('.shard-gump') as HTMLElement);
    expect(send).not.toHaveBeenCalled();
  });

  it('moves_by_a_drag_of_its_background', () => {
    const send = vi.fn();
    const { container } = render(<ShardGump data={gump} scale={2} send={send} />);
    const root = container.querySelector('.shard-gump') as HTMLElement;
    fireEvent.pointerDown(root, { button: 0, clientX: 100, clientY: 100 });
    fireEvent.pointerMove(window, { clientX: 140, clientY: 120 });
    fireEvent.pointerUp(window, { clientX: 140, clientY: 120 });
    expect(send).toHaveBeenLastCalledWith({ place: { x: 60, y: 40 } });
  });

  it('shows_the_html_words_of_the_shard_as_text', () => {
    const { container } = render(<ShardGump data={gump} scale={1} send={vi.fn()} />);
    expect(container.querySelector('script')).toBeNull();
    expect(container.textContent).toContain(MARKUP);
    const bold = [...container.querySelectorAll('.gump-html span')].find((span) => span.textContent === 'bold') as HTMLElement;
    expect(bold.style.fontWeight).toBe('bold');
  });

  it('stops_following_a_drag_when_it_goes', () => {
    const send = vi.fn();
    const { container, unmount } = render(<ShardGump data={gump} scale={2} send={send} />);
    fireEvent.pointerDown(container.querySelector('.shard-gump') as HTMLElement, { button: 0, clientX: 100, clientY: 100 });
    act(() => {
      unmount();
    });
    fireEvent.pointerUp(window, { clientX: 140, clientY: 120 });
    expect(send).not.toHaveBeenCalled();
  });
});
