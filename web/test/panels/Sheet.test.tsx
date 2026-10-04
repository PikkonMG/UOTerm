import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Sheet } from '../../src/panels/Sheet';
import type { SheetData } from '../../src/panels/types';
import { label } from './fixtures';

const SWORD = 1073741904;
const tabs = [
  { words: 'Character', chosen: true },
  { words: 'Skills', chosen: false },
  { words: 'Spells', chosen: false },
  { words: 'Party', chosen: false },
];
const character: SheetData = {
  tabs,
  live: true,
  character: {
    views: [
      { words: 'Worn', chosen: true },
      { words: 'Status', chosen: false },
    ],
    worn: {
      doll: null,
      name: 'Mara',
      stats: [{ stat: 0, name: 'Strength', value: '50', lock: 0, hover: label('Strength') }],
      facts: [{ words: 'Gold', value: '120' }],
      worn_words: 'Worn (1)',
      rows: [{ serial: SWORD, layer: 1, picture: null, words: 'one handed', wear: null, take_off: 'x', hover: label('') }],
      nothing: null,
      zone: 'wear',
      wear: { hint: 'Say what to wear', button: 'Wear', on: true, note: null },
    },
    status: null,
  },
  skills: null,
  spells: null,
  party: null,
};

describe('Sheet', () => {
  it('turns_its_tabs_and_views', () => {
    const send = vi.fn();
    const { getByText } = render(<Sheet data={character} send={send} />);
    fireEvent.click(getByText('Skills'));
    expect(send).toHaveBeenCalledWith({ tab: 1 });
    fireEvent.click(getByText('Status'));
    expect(send).toHaveBeenLastCalledWith({ view: 1 });
  });

  it('takes_off_uses_and_names_a_worn_item_and_turns_a_stat_lock', () => {
    const send = vi.fn();
    const { getByText, getByLabelText } = render(<Sheet data={character} send={send} />);
    fireEvent.click(getByText('x'));
    expect(send).toHaveBeenCalledWith({ take_off: 1 });
    fireEvent.dblClick(getByText('one handed'));
    expect(send).toHaveBeenCalledWith({ worn_double: SWORD });
    fireEvent.click(getByText('one handed'));
    expect(send).toHaveBeenCalledWith({ worn_click: SWORD });
    fireEvent.click(getByLabelText('Strength'));
    expect(send).toHaveBeenLastCalledWith({ stat_lock: 0 });
  });

  it('asks_jev_what_to_wear_and_takes_a_drop_on_the_figure', () => {
    const send = vi.fn();
    const { getByPlaceholderText, getByText, container } = render(<Sheet data={character} send={send} />);
    fireEvent.input(getByPlaceholderText('Say what to wear'), { target: { value: 'my sword' } });
    fireEvent.click(getByText('Wear'));
    expect(send).toHaveBeenLastCalledWith({ wear: 'my sword' });
    expect(container.querySelector('[data-zone]')?.getAttribute('data-zone')).toBe('"wear"');
  });

  it('uses_and_pins_a_skill_and_sorts_its_table', () => {
    const send = vi.fn();
    const skills: SheetData = {
      ...character,
      character: null,
      skills: {
        sums: 'Real 50.0   Base 50.0',
        grouped: false,
        columns: [
          { words: 'Name', chosen: true, descending: false },
          { words: 'Real', chosen: false, descending: false },
        ],
        new_group: null,
        reset: null,
        reset_ask: null,
        rows: [{ kind: 'skill', id: 21, name: 'Hiding', values: ['50.0'], lock: 0, buttons: ['Use', 'Pin'], hover: label('Hiding') }],
      },
    };
    const { getByText } = render(<Sheet data={skills} send={send} />);
    fireEvent.click(getByText('Use'));
    expect(send).toHaveBeenCalledWith({ use_skill: 21 });
    fireEvent.click(getByText('Pin'));
    expect(send).toHaveBeenCalledWith({ pin_skill: 21 });
    fireEvent.click(getByText('Real'));
    expect(send).toHaveBeenLastCalledWith({ sort: 1 });
  });

  it('reads_casts_and_pins_a_spell', () => {
    const send = vi.fn();
    const spells: SheetData = {
      ...character,
      character: null,
      spells: {
        books: [{ words: 'Chivalry', chosen: true }],
        no_book: null,
        empty: null,
        list: [{ id: 201, name: 'Cleanse by Fire', icon: null, chosen: true, hover: label('Cleanse by Fire') }],
        assign: null,
        detail: { id: 201, icon: null, name: 'Cleanse by Fire', group: null, power: 'Expor Flamus', lines: [], buttons: ['Cast', 'Pin'] },
        pick: null,
      },
    };
    const { getAllByText, getByText } = render(<Sheet data={spells} send={send} />);
    fireEvent.click(getAllByText('Cleanse by Fire')[0]);
    expect(send).toHaveBeenCalledWith({ spell: 201 });
    fireEvent.click(getByText('Cast'));
    expect(send).toHaveBeenCalledWith({ cast: 201 });
    fireEvent.click(getByText('Pin'));
    expect(send).toHaveBeenLastCalledWith({ pin_spell: 201 });
  });

  it('tells_a_member_and_kicks_from_the_party_tab', () => {
    const send = vi.fn();
    const party: SheetData = {
      ...character,
      character: null,
      party: {
        invite: null,
        loot: 'Party loots: no',
        leave: 'Disband the party',
        add: 'Add member',
        rows: [
          {
            kind: 'place',
            number: '2.',
            member: { serial: 2, name: 'Bob', pools: [0.8, 0, 0], chosen: false, tell: 'Tell', kick: 'Kick', hover: label('Bob') },
            empty: 'Empty',
          },
        ],
        tell: { hint: 'Tell the party', say: 'Say' },
      },
    };
    const { getByText, getByPlaceholderText } = render(<Sheet data={party} send={send} />);
    fireEvent.click(getByText('Tell'));
    expect(send).toHaveBeenCalledWith({ tell_to: 2 });
    fireEvent.click(getByText('Kick'));
    expect(send).toHaveBeenCalledWith({ kick: 2 });
    fireEvent.input(getByPlaceholderText('Tell the party'), { target: { value: 'heal me' } });
    fireEvent.click(getByText('Say'));
    expect(send).toHaveBeenLastCalledWith({ say: 'heal me' });
  });
});
