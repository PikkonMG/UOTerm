import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Panels } from '../../src/panels/Panels';
import type { PanelData } from '../../src/panels/types';
import { frameAt, label } from './fixtures';

const HINTS = { drag: 'Drag: move.', lock: 'Lock it.', close: 'Close.', size: 'Drag: size.', fold: 'Fold it.' };

function data(): PanelData {
  return {
    look: { ui_scale: 1, opacity: 1, font: null },
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
    grids: [],
    loot: null,
    split: null,
    shop: null,
    trades: [],
    old_menu: null,
    book: null,
    board: null,
    paperdoll: null,
    entry: null,
    race: null,
    tip: null,
    dye: null,
    gumps: [],
    build: null,
    channels: null,
    world_map: null,
    markers: null,
    marker_box: null,
    map_items: [],
    profile: null,
    quest_arrow: null,
    buffs: null,
    cooldowns: null,
    cast: null,
    counters: null,
    info_bar: null,
    dps: null,
    durability: null,
    net_stats: null,
    debug: null,
    abilities: null,
    racial: null,
    invite: null,
    agents: [],
    macros: null,
    options: null,
    color_picker: null,
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
      strip: { x: 900, y: 700, w: 400, h: 58 },
    },
    tooltip: null,
    carried: null,
  };
}

describe('Panels', () => {
  it('lays_each_panel_at_its_place_and_sends_its_actions_by_its_name', () => {
    const send = vi.fn();
    const { getByText } = render(<Panels data={data()} send={send} input={vi.fn()} covered={vi.fn()} />);
    const frame = getByText('Hotbar').closest('[data-panel]') as HTMLElement;
    expect(frame.style.left).toBe('400px');
    expect(frame.style.top).toBe('600px');
    fireEvent.click(getByText('Heal'));
    expect(send).toHaveBeenCalledWith('hotbar', { click: 0 });
  });

  it('grows_the_panels_by_the_ui_scale_and_tells_where_they_lie', () => {
    const covered = vi.fn();
    const scaled = { ...data(), look: { ui_scale: 2, opacity: 0.5, font: null } };
    const { container } = render(<Panels data={scaled} send={vi.fn()} input={vi.fn()} covered={covered} />);
    const layer = container.querySelector('.panels') as HTMLElement;
    expect(layer.style.getPropertyValue('--ui-scale')).toBe('2');
    expect(layer.style.getPropertyValue('--panel-opacity')).toBe('0.5');
    expect(covered).toHaveBeenCalled();
  });

  it('drops_the_carried_item_on_the_zone_under_the_button', () => {
    const send = vi.fn();
    const carried = { ...data(), carried: { picture: null, words: 'logs', alpha: 0.85 } };
    const { getByText } = render(<Panels data={carried} send={send} input={vi.fn()} covered={vi.fn()} />);
    const cell = getByText('Heal');
    const at = { clientX: 5, clientY: 6 };
    document.elementFromPoint = () => cell;
    fireEvent.pointerUp(window, at);
    expect(send).toHaveBeenLastCalledWith('desk', { drop: { x: 5, y: 6, zone: { slot: 0 }, on_panel: true, shift: false } });
  });

  it('asks_for_the_tooltip_of_a_thing_the_mouse_rests_on', () => {
    const send = vi.fn();
    const { getByText } = render(<Panels data={data()} send={send} input={vi.fn()} covered={vi.fn()} />);
    const slot = getByText('Heal').closest('button') as HTMLElement;
    fireEvent.pointerEnter(slot);
    expect(send).toHaveBeenLastCalledWith('tips', { over: label('Heal') });
    fireEvent.pointerLeave(slot);
    expect(send).toHaveBeenLastCalledWith('tips', { over: null });
  });

  it('tells_the_view_where_the_panels_lie_again_when_the_scale_or_the_bar_changes', () => {
    // A box as the page draws it: as tall as its buttons, grown by the UI scale.
    vi.spyOn(Element.prototype, 'getBoundingClientRect').mockImplementation(function (this: Element) {
      const scale = Number((this.closest('.panels') as HTMLElement | null)?.style.getPropertyValue('--ui-scale') || 1);
      const height = Math.max(this.querySelectorAll('button').length, 1) * 10 * scale;
      return { left: 0, top: 0, right: 100 * scale, bottom: height, width: 100 * scale, height, x: 0, y: 0, toJSON: () => ({}) } as DOMRect;
    });
    const covered = vi.fn();
    const bar = {
      place: { x: 0, y: 0, w: 880, h: 60 },
      location: { numbers: '1, 2, 0', map_words: 'map', map: '0', faces_words: 'faces', facing: '' },
      folded: false,
      launcher_words: 'Panels',
      launcher_shows: false,
      status: null,
      buttons: ['Take control'],
    };
    const first = { ...data(), hotbar: null, bar };
    const { rerender } = render(<Panels data={first} send={vi.fn()} input={vi.fn()} covered={covered} />);
    const bottoms = () => covered.mock.calls.at(-1)?.[0].map((area: { max: { y: number } }) => area.max.y);
    const before = bottoms();
    rerender(<Panels data={{ ...first, bar: { ...bar, buttons: ['Bag', 'Sheet', 'War', 'Stop'] } }} send={vi.fn()} input={vi.fn()} covered={covered} />);
    const taller = bottoms();
    expect(taller[0]).toBeGreaterThan(before[0]);
    rerender(<Panels data={{ ...first, look: { ui_scale: 2, opacity: 1, font: null } }} send={vi.fn()} input={vi.fn()} covered={covered} />);
    expect(covered.mock.calls.at(-1)?.[0][0].max.x).toBe(200);
    vi.restoreAllMocks();
  });

  it('names_the_action_of_a_grid_by_its_container', () => {
    const send = vi.fn();
    const panels = data();
    panels.grids = [
      {
        frame: frameAt('grid:9', 'Backpack', { x: 10, y: 10, w: 200, h: 200 }),
        body: {
          serial: 9,
          live: true,
          glass: 'var(--glass)',
          glass_opacity: 1,
          search: '',
          search_hint: 'search',
          count: '0',
          favorite: { words: 'Fav', color: 'var(--text-dim)', hint: 'Fav' },
          loot_all: null,
          loot_bag: null,
          columns: 1,
          side: 46,
          gap: 4,
          art_scale: 1,
          cells: [],
          strip: null,
          zone: { into: 9 },
        },
      },
    ];
    const { getByText } = render(<Panels data={panels} send={send} input={vi.fn()} covered={vi.fn()} />);
    fireEvent.click(getByText('Fav'));
    expect(send).toHaveBeenCalledWith('grid:9', { favorite: true });
  });

  it('names_the_actions_of_a_gump_and_a_map_item_by_their_names', () => {
    const send = vi.fn();
    const panels = data();
    panels.gumps = [
      {
        panel: 'gump:77',
        at: { x: 50, y: 60 },
        size: { x: 100, y: 100 },
        movable: true,
        closable: true,
        live: true,
        pieces: [{ kind: 'button', piece: 4, at: { x: 0, y: 0 }, size: { x: 20, y: 20 }, normal: '1', pressed: null, item: null, alpha: 1, tip: null }],
      },
    ];
    panels.profile = {
      frame: frameAt('profile', 'Mara', { x: 300, y: 10, w: 200, h: 200 }),
      body: { title: '', words: 'Hi', writing: null, hint: '', write: null, close: 'Close' },
    };
    const { container, getByText } = render(<Panels data={panels} send={send} input={vi.fn()} covered={vi.fn()} />);
    const gump = container.querySelector('[data-panel="gump:77"]') as HTMLElement;
    expect(gump.style.left).toBe('50px');
    fireEvent.click(container.querySelector('[data-piece="4"]') as HTMLElement);
    expect(send).toHaveBeenCalledWith('gump:77', { button: 4 });
    fireEvent.click(getByText('Close'));
    expect(send).toHaveBeenCalledWith('profile', { close: true });
  });
});
