import { afterEach, describe, expect, it, vi } from 'vitest';
import { dropTarget, followDrag, setDragDistance } from '../../src/panels/drag';

afterEach(() => {
  document.body.innerHTML = '';
});

describe('drag', () => {
  it('starts_a_drag_only_past_the_distance_and_tells_where_it_ended', () => {
    setDragDistance(6);
    const started = vi.fn();
    const dropped = vi.fn();
    followDrag({ clientX: 0, clientY: 0 }, { started, dropped });
    window.dispatchEvent(new MouseEvent('pointermove', { clientX: 3, clientY: 0 }));
    expect(started).not.toHaveBeenCalled();
    window.dispatchEvent(new MouseEvent('pointermove', { clientX: 30, clientY: 0 }));
    expect(started).toHaveBeenCalledTimes(1);
    window.dispatchEvent(new MouseEvent('pointerup', { clientX: 30, clientY: 4 }));
    expect(dropped).toHaveBeenCalledWith({ x: 30, y: 4 });
  });

  it('a_press_that_does_not_move_is_no_drag', () => {
    setDragDistance(6);
    const dropped = vi.fn();
    followDrag({ clientX: 0, clientY: 0 }, { dropped });
    window.dispatchEvent(new MouseEvent('pointerup', { clientX: 1, clientY: 1 }));
    expect(dropped).not.toHaveBeenCalled();
  });

  it('finds_the_zone_and_the_panel_under_a_drop', () => {
    document.body.innerHTML = '<div data-panel="hotbar"><div data-zone=\'{"slot":3}\'><span id="cell">3</span></div></div><div id="map"></div>';
    const cell = document.getElementById('cell') as HTMLElement;
    expect(dropTarget(cell)).toEqual({ zone: { slot: 3 }, on_panel: true });
    expect(dropTarget(document.getElementById('map'))).toEqual({ zone: null, on_panel: false });
  });
});
