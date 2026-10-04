import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Agents } from '../../src/panels/Agents';
import type { AgentWindowData } from '../../src/panels/types';

const window: AgentWindowData = {
  rows: [
    { kind: 'words', words: { words: 'Loot is off.', color: 'var(--text-faint)' } },
    { kind: 'labeled', words: 'Loot', buttons: [{ words: 'On', color: 'var(--goal)' }], width: 64 },
    { kind: 'pictures', pictures: [{ picture: null, name: 'bandage' }] },
    { kind: 'field', hint: 'name', button: 'Add', width: 64, words: 'Bob' },
  ],
};

describe('Agents', () => {
  it('presses_buttons_and_pictures_by_their_row_and_keeps_the_field_in_the_view', () => {
    const send = vi.fn();
    const { getByText, getByTitle, getByPlaceholderText } = render(<Agents data={window} send={send} />);
    fireEvent.click(getByText('On'));
    expect(send).toHaveBeenLastCalledWith({ press: { row: 1, at: 0 } });
    fireEvent.click(getByTitle('bandage'));
    expect(send).toHaveBeenLastCalledWith({ picture: { row: 2, at: 0 } });
    const field = getByPlaceholderText('name') as HTMLInputElement;
    expect(field.value).toBe('Bob');
    fireEvent.input(field, { target: { value: 'Bobby' } });
    expect(send).toHaveBeenLastCalledWith({ typing: 'Bobby' });
    fireEvent.keyDown(field, { key: 'Enter' });
    expect(send).toHaveBeenLastCalledWith({ submit: 3 });
  });

  it('shows_the_words_of_the_session_as_text', () => {
    const { getByText } = render(<Agents data={{ rows: [{ kind: 'words', words: { words: '<b>x</b>', color: 'var(--text)' } }] }} send={vi.fn()} />);
    expect(getByText('<b>x</b>')).toBeTruthy();
  });
});
