import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Profile } from '../../src/panels/Profile';
import type { ProfileData } from '../../src/panels/types';

const MARKUP = '<i>me</i>';

describe('Profile', () => {
  it('shows_the_words_as_text_writes_and_closes', () => {
    const send = vi.fn();
    const data: ProfileData = { title: 'the Brave', words: MARKUP, writing: null, hint: 'What you say', write: 'Write', close: 'Close' };
    const { getByText, container, rerender } = render(<Profile data={data} send={send} />);
    expect(container.querySelector('i')).toBeNull();
    expect(container.textContent).toContain(MARKUP);
    fireEvent.click(getByText('Write'));
    expect(send).toHaveBeenLastCalledWith({ write: true });
    rerender(<Profile data={{ ...data, writing: 'Hi', write: 'Save' }} send={send} />);
    const field = container.querySelector('textarea') as HTMLTextAreaElement;
    fireEvent.input(field, { target: { value: 'Hello' } });
    expect(send).toHaveBeenLastCalledWith({ words: 'Hello' });
    fireEvent.click(getByText('Close'));
    expect(send).toHaveBeenLastCalledWith({ close: true });
  });
});
