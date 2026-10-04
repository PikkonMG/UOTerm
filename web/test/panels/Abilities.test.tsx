import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Abilities, Racial } from '../../src/panels/Abilities';
import type { AbilitiesData } from '../../src/panels/types';
import { label } from './fixtures';

const buttons = [
  { words: 'Arm', color: 'var(--goal)' },
  { words: 'Pin', color: 'var(--text)' },
];
const abilities: AbilitiesData = {
  icon: 44,
  slots: [
    { picture: 'g1', words: 'Primary: Armor Ignore', armed: null, buttons, hover: label('Primary') },
    { picture: 'g2', words: 'Secondary: Bleed Attack', armed: { words: 'Armed', color: 'var(--alarm)' }, buttons, hover: label('Secondary') },
  ],
  all_title: 'Every weapon ability',
  rows: [{ picture: 'g3', name: { words: 'Armor Ignore', color: 'var(--goal)' }, slot: 'Primary', hover: label('Armor Ignore') }],
};

describe('Abilities', () => {
  it('arms_pins_and_lists_the_abilities', () => {
    const send = vi.fn();
    const { getAllByText, getByText } = render(<Abilities data={abilities} send={send} />);
    fireEvent.click(getAllByText('Arm')[1]);
    expect(send).toHaveBeenLastCalledWith({ use: 1 });
    fireEvent.click(getAllByText('Pin')[0]);
    expect(send).toHaveBeenLastCalledWith({ pin: 0 });
    expect(getByText('Armed').style.color).toBe('var(--alarm)');
    expect(getByText('Every weapon ability')).toBeTruthy();
  });

  it('acts_on_no_click_of_an_icon_without_control', () => {
    const send = vi.fn();
    const still = { ...abilities, slots: abilities.slots.map((slot) => ({ ...slot, buttons: [] })) };
    const { container } = render(<Abilities data={still} send={send} />);
    fireEvent.click(container.querySelector('.ability-icon') as Element);
    expect(send).not.toHaveBeenCalled();
  });
});

describe('Racial', () => {
  it('uses_the_flight_and_says_passive_or_no_race', () => {
    const send = vi.fn();
    const rows = [
      { picture: 'f', name: 'Flying', passive: null, buttons: [{ words: 'Use', color: 'var(--goal)' }, buttons[1]], hover: label('Flying') },
      { picture: 'b', name: 'Berserk', passive: 'Passive', buttons: [], hover: null },
    ];
    const { getByText, rerender } = render(<Racial data={{ icon: 44, rows, none: null }} send={send} />);
    fireEvent.click(getByText('Use'));
    expect(send).toHaveBeenLastCalledWith({ use: 0 });
    expect(getByText('Passive')).toBeTruthy();
    rerender(<Racial data={{ icon: 44, rows: [], none: 'The shard names no race for the character.' }} send={send} />);
    expect(getByText('The shard names no race for the character.')).toBeTruthy();
  });
});
