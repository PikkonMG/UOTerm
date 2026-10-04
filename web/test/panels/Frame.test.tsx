import { act, fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Frame } from '../../src/panels/Frame';

const AREA = { x: 10, y: 10, w: 300, h: 200 };
const HINTS = { drag: 'Drag: move.', lock: 'Lock it.', close: 'Close.', size: 'Drag: size.', fold: 'Fold it.' };

describe('Frame', () => {
  it('reports_a_title_drag_as_a_new_place', () => {
    const send = vi.fn();
    const { getByText } = render(
      <Frame title="Journal" area={AREA} send={send}>
        <p>x</p>
      </Frame>,
    );
    const title = getByText('Journal');
    fireEvent.pointerDown(title, { clientX: 20, clientY: 15 });
    fireEvent.pointerMove(window, { clientX: 70, clientY: 45 });
    fireEvent.pointerUp(window, { clientX: 70, clientY: 45 });
    expect(send).toHaveBeenLastCalledWith({ place: { x: 60, y: 40, w: 300, h: 200 } });
  });

  it('moves_by_the_points_of_the_panels_when_they_are_scaled', () => {
    const send = vi.fn();
    const { getByText } = render(
      <Frame title="Journal" area={AREA} scale={2} send={send}>
        <p>x</p>
      </Frame>,
    );
    fireEvent.pointerDown(getByText('Journal'), { clientX: 0, clientY: 0 });
    fireEvent.pointerUp(window, { clientX: 40, clientY: 20 });
    expect(send).toHaveBeenLastCalledWith({ place: { x: 30, y: 20, w: 300, h: 200 } });
  });

  it('does_not_move_while_locked_and_puts_the_panel_back_on_a_double_click', () => {
    const send = vi.fn();
    const { getByText } = render(
      <Frame title="Journal" area={AREA} locked send={send}>
        <p>x</p>
      </Frame>,
    );
    fireEvent.pointerDown(getByText('Journal'), { clientX: 0, clientY: 0 });
    fireEvent.pointerUp(window, { clientX: 40, clientY: 20 });
    expect(send).not.toHaveBeenCalled();
    const free = vi.fn();
    const again = render(
      <Frame title="Radar" area={AREA} send={free}>
        <p>y</p>
      </Frame>,
    );
    fireEvent.dblClick(again.getByText('Radar'));
    expect(free).toHaveBeenCalledWith({ reset: true });
  });

  it('sizes_by_its_corner_and_reports_lock_fold_and_close', () => {
    const send = vi.fn();
    const { getByLabelText } = render(
      <Frame title="Journal" area={AREA} hints={HINTS} sizable foldable closable send={send}>
        <p>x</p>
      </Frame>,
    );
    fireEvent.pointerDown(getByLabelText(HINTS.size), { clientX: 0, clientY: 0 });
    fireEvent.pointerUp(window, { clientX: 20, clientY: 30 });
    expect(send).toHaveBeenLastCalledWith({ place: { x: 10, y: 10, w: 320, h: 230 } });
    fireEvent.click(getByLabelText(HINTS.lock));
    expect(send).toHaveBeenLastCalledWith({ lock: true });
    fireEvent.click(getByLabelText(HINTS.fold));
    expect(send).toHaveBeenLastCalledWith({ fold: true });
    fireEvent.click(getByLabelText(HINTS.close));
    expect(send).toHaveBeenLastCalledWith({ close: true });
  });

  it('shows_only_its_title_and_foot_when_folded', () => {
    const { queryByText } = render(
      <Frame title="Journal" area={AREA} folded foldable send={vi.fn()} foot={<p>chat</p>}>
        <p>lines</p>
      </Frame>,
    );
    expect(queryByText('lines')).toBeNull();
    expect(queryByText('chat')).not.toBeNull();
  });

  it('stops_following_a_drag_when_it_goes', () => {
    const send = vi.fn();
    const { getByText, unmount } = render(
      <Frame title="Journal" area={AREA} send={send}>
        <p>x</p>
      </Frame>,
    );
    const removed = vi.spyOn(window, 'removeEventListener');
    fireEvent.pointerDown(getByText('Journal'), { clientX: 0, clientY: 0 });
    act(() => {
      unmount();
    });
    expect(removed).toHaveBeenCalledWith('pointerup', expect.any(Function));
    fireEvent.pointerUp(window, { clientX: 40, clientY: 20 });
    expect(send).not.toHaveBeenCalled();
  });
});
