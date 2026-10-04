import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Paperdoll } from '../../src/panels/Paperdoll';
import type { PaperdollData } from '../../src/panels/types';
import { label } from './fixtures';

const SWORD = 1073741904;
const doll: PaperdollData = {
  live: true,
  figure: null,
  out_of_sight: null,
  health: 0.5,
  rows: [{ serial: SWORD, picture: null, words: 'one handed', hover: label('') }],
  nothing: null,
  dresses: true,
  buttons: ['Status', 'Virtues'],
  close: 'Close',
  zone: 'wear',
};

describe('Paperdoll', () => {
  it('uses_names_worn_items_and_presses_its_buttons', () => {
    const send = vi.fn();
    const { getByText, container } = render(<Paperdoll data={doll} send={send} />);
    fireEvent.click(getByText('one handed'));
    expect(send).toHaveBeenLastCalledWith({ click: SWORD });
    fireEvent.dblClick(getByText('one handed'));
    expect(send).toHaveBeenLastCalledWith({ double: SWORD });
    fireEvent.click(getByText('Virtues'));
    expect(send).toHaveBeenLastCalledWith({ button: 1 });
    fireEvent.click(getByText('Close'));
    expect(send).toHaveBeenLastCalledWith({ close: true });
    expect(container.querySelector('.paperdoll')?.getAttribute('data-zone')).toBe('"wear"');
  });

  it('says_when_the_mobile_is_out_of_sight', () => {
    const { getByText } = render(<Paperdoll data={{ ...doll, out_of_sight: 'Out of sight.', rows: [] }} send={vi.fn()} />);
    expect(getByText('Out of sight.')).toBeTruthy();
  });
});
