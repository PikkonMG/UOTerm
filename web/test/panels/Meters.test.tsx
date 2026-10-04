import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Dps } from '../../src/panels/Dps';
import { Durability } from '../../src/panels/Durability';
import { PartyInvite } from '../../src/panels/PartyInvite';
import { Stats } from '../../src/panels/Stats';

describe('Dps', () => {
  it('presses_the_buttons_of_the_meter_by_their_place_and_lists_the_mobiles', () => {
    const send = vi.fn();
    const data = {
      buttons: ['Start', 'Pause', 'Stop'],
      per_second: { words: '2.5 per second', color: 'var(--goal)' },
      total: '10  4s',
      rows: [{ name: 'a rat', words: '10  2.5/s' }],
      none: null,
    };
    const { getByText } = render(<Dps data={data} send={send} />);
    fireEvent.click(getByText('Stop'));
    expect(send).toHaveBeenLastCalledWith({ button: 2 });
    expect(getByText('a rat')).toBeTruthy();
  });
});

describe('Durability and Stats', () => {
  it('show_the_wear_and_flip_the_words_on_a_double_click', () => {
    const wear = render(<Durability data={{ art: 30, rows: [{ picture: null, name: 'leather tunic', words: { words: '5 / 40', color: 'var(--alarm)' }, fill: 0.125 }], none: null }} />);
    expect(wear.getByText('5 / 40').style.color).toBe('var(--alarm)');
    const send = vi.fn();
    const stats = render(<Stats data={{ words: { words: 'Ping: 20 ms', color: 'var(--hits-poisoned)' }, hint: 'Double-click: more or less.' }} send={send} />);
    fireEvent.dblClick(stats.getByText('Ping: 20 ms'));
    expect(send).toHaveBeenLastCalledWith({ double: true });
  });
});

describe('PartyInvite', () => {
  it('answers_an_invite_and_shows_no_answer_without_control', () => {
    const send = vi.fn();
    const words = 'Bob has invited you to join a party.';
    const data = { words, accept: { words: 'Accept', color: 'var(--goal)' }, decline: { words: 'Decline', color: 'var(--alarm)' } };
    const { getByText, rerender, queryByText } = render(<PartyInvite data={data} send={send} />);
    fireEvent.click(getByText('Decline'));
    expect(send).toHaveBeenLastCalledWith({ decline: true });
    rerender(<PartyInvite data={{ words, accept: null, decline: null }} send={send} />);
    expect(queryByText('Accept')).toBeNull();
  });
});
