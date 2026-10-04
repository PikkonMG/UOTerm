import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Board } from '../../src/panels/Board';
import type { BoardData } from '../../src/panels/types';

const board: BoardData = {
  live: true,
  posts: [
    { serial: 51, subject: 'Ore', poster: 'Ann  Day 1', indent: 0, reading: false },
    { serial: 52, subject: 'Re: Ore', poster: 'Bob  Day 2', indent: 14, reading: true },
  ],
  text: 'I buy.',
  subject: '',
  body: '',
  subject_hint: 'Subject',
  text_hint: 'Your message',
  post: 'Post',
  reply: 'Reply',
  remove: 'Remove',
  close: 'Close',
  paper: 'rgba(226, 214, 184, 1)',
  ink: 'rgba(46, 36, 24, 1)',
};

describe('Board', () => {
  it('reads_writes_and_posts', () => {
    const send = vi.fn();
    const { getByText, getByPlaceholderText } = render(<Board data={board} send={send} />);
    fireEvent.click(getByText('Ore'));
    expect(send).toHaveBeenLastCalledWith({ read: 51 });
    fireEvent.input(getByPlaceholderText('Subject'), { target: { value: 'Hi' } });
    expect(send).toHaveBeenLastCalledWith({ subject: 'Hi' });
    fireEvent.input(getByPlaceholderText('Your message'), { target: { value: 'Sold.' } });
    expect(send).toHaveBeenLastCalledWith({ text: 'Sold.' });
    for (const [words, action] of [['Post', 'post'], ['Reply', 'reply'], ['Remove', 'remove'], ['Close', 'close']]) {
      fireEvent.click(getByText(words));
      expect(send).toHaveBeenLastCalledWith({ [action]: true });
    }
    expect(getByText('I buy.')).toBeTruthy();
  });
});
