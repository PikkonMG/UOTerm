import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Panels } from '../../src/panels/Panels';
import type { PanelData } from '../../src/panels/types';
import { frameAt, label } from './fixtures';

const HINTS = { drag: 'Drag: move.', lock: 'Lock it.', close: 'Close.', size: 'Drag: size.', fold: 'Fold it.' };

function data(): PanelData {
  return {
    look: { ui_scale: 1, opacity: 1 },
    hints: HINTS,
    title: 'UOTerm watch',
    waiting: null,
    alarm: 0,
    bar: null,
    launcher: null,
    activity: null,
    vitals: null,
    pack: null,
    near: null,
    bars: [],
    journal: null,
    radar: null,
    hotbar: {
      frame: frameAt('hotbar', 'Hotbar', { x: 400, y: 600, w: 340, h: 80 }),
      body: { slots: [{ words: 'Heal', key: '1', picture: null, tip: 'Heal', hover: label('Heal') }], picking: null },
    },
    picker: null,
    sheet: null,
    split: null,
    ring: null,
    report: null,
    question: null,
    chat: {
      text: '',
      open: false,
      hidden: false,
      live: true,
      idle_words: '',
      mode_words: 'Say',
      hint: 'Press Enter to chat.',
      pin_words: null,
      focus: null,
      paste: false,
      strip: { x: 900, y: 700, w: 400, h: 58 },
    },
    tooltip: null,
    carried: null,
  };
}

describe('Panels', () => {
  it('lays_each_panel_at_its_place_and_sends_its_actions_by_its_name', () => {
    const send = vi.fn();
    const { getByText } = render(<Panels data={data()} plates={[]} floats={[]} send={send} input={vi.fn()} covered={vi.fn()} />);
    const frame = getByText('Hotbar').closest('[data-panel]') as HTMLElement;
    expect(frame.style.left).toBe('400px');
    expect(frame.style.top).toBe('600px');
    fireEvent.click(getByText('Heal'));
    expect(send).toHaveBeenCalledWith('hotbar', { press: 0 });
  });

  it('grows_the_panels_by_the_ui_scale_and_tells_where_they_lie', () => {
    const covered = vi.fn();
    const scaled = { ...data(), look: { ui_scale: 2, opacity: 0.5 } };
    const { container } = render(<Panels data={scaled} plates={[]} floats={[]} send={vi.fn()} input={vi.fn()} covered={covered} />);
    const layer = container.querySelector('.panels') as HTMLElement;
    expect(layer.style.getPropertyValue('--ui-scale')).toBe('2');
    expect(layer.style.getPropertyValue('--panel-opacity')).toBe('0.5');
    expect(covered).toHaveBeenCalled();
  });

  it('drops_the_carried_item_on_the_zone_under_the_button', () => {
    const send = vi.fn();
    const carried = { ...data(), carried: { picture: null, words: 'logs', alpha: 0.85 } };
    const { getByText } = render(<Panels data={carried} plates={[]} floats={[]} send={send} input={vi.fn()} covered={vi.fn()} />);
    const cell = getByText('Heal');
    const at = { clientX: 5, clientY: 6 };
    document.elementFromPoint = () => cell;
    fireEvent.pointerUp(window, at);
    expect(send).toHaveBeenLastCalledWith('desk', { drop: { x: 5, y: 6, zone: { slot: 0 }, on_panel: true, shift: false } });
  });

  it('asks_for_the_tooltip_of_a_thing_the_mouse_rests_on', () => {
    const send = vi.fn();
    const { getByText } = render(<Panels data={data()} plates={[]} floats={[]} send={send} input={vi.fn()} covered={vi.fn()} />);
    const slot = getByText('Heal').closest('button') as HTMLElement;
    fireEvent.pointerEnter(slot);
    expect(send).toHaveBeenLastCalledWith('tips', { over: label('Heal') });
    fireEvent.pointerLeave(slot);
    expect(send).toHaveBeenLastCalledWith('tips', { over: null });
  });
});
