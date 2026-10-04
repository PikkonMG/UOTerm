import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Book } from '../../src/panels/Book';
import type { BookData } from '../../src/panels/types';

const book: BookData = {
  writing: true,
  by: null,
  title: 'Tales',
  author: 'Ann',
  title_hint: 'The title of the book',
  author_hint: 'The author',
  pages: [
    { number: '1', words: 'Once\nupon', caret: null },
    { number: '2', words: '', caret: null },
  ],
  lines: 10,
  edits: 0,
  turns: ['First', 'Back', 'Next', 'Last'],
  save: 'Save',
  close: 'Close',
  paper: 'rgba(226, 214, 184, 1)',
  ink: 'rgba(46, 36, 24, 1)',
};

describe('Book', () => {
  it('writes_pages_and_the_cover_and_turns', () => {
    const send = vi.fn();
    const { getAllByRole, getByPlaceholderText, getByText } = render(<Book data={book} send={send} />);
    const right = getAllByRole('textbox').filter((field) => field.tagName === 'TEXTAREA')[1] as HTMLTextAreaElement;
    fireEvent.input(right, { target: { value: 'Hi', selectionStart: 2 } });
    expect(send).toHaveBeenLastCalledWith({ page: { side: 1, words: 'Hi', caret: 2 } });
    fireEvent.input(getByPlaceholderText('The author'), { target: { value: 'Mara' } });
    expect(send).toHaveBeenLastCalledWith({ cover: { title: 'Tales', author: 'Mara' } });
    fireEvent.click(getByText('Next'));
    expect(send).toHaveBeenLastCalledWith({ turn: 2 });
    fireEvent.click(getByText('Save'));
    expect(send).toHaveBeenLastCalledWith({ save: true });
    fireEvent.click(getByText('Close'));
    expect(send).toHaveBeenLastCalledWith({ close: true });
  });

  it('shows_a_sealed_book_as_words_on_paper', () => {
    const { getByText, queryByRole } = render(<Book data={{ ...book, writing: false, by: 'by Ann', save: null }} send={vi.fn()} />);
    expect(getByText('by Ann')).toBeTruthy();
    expect(queryByRole('textbox')).toBeNull();
    expect(getByText(/Once/)).toBeTruthy();
  });
});
