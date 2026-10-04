import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Entry } from '../../src/panels/Entry';
import type { EntryData } from '../../src/panels/types';

const entry: EntryData = {
  live: true,
  description: 'Name your pet',
  hint: 'Type the answer and press Enter.',
  words: '',
  focus: true,
  okay: 'Okay',
  cancel: 'Cancel',
  take_control: null,
};

describe('Entry', () => {
  it('takes_the_keys_once_types_and_answers', () => {
    const send = vi.fn();
    const { getByPlaceholderText, getByText } = render(<Entry data={entry} send={send} />);
    const field = getByPlaceholderText('Type the answer and press Enter.');
    expect(document.activeElement).toBe(field);
    expect(send).toHaveBeenCalledWith({ focused: true });
    fireEvent.input(field, { target: { value: 'Rex' } });
    expect(send).toHaveBeenLastCalledWith({ words: 'Rex' });
    fireEvent.keyDown(field, { key: 'Enter' });
    expect(send).toHaveBeenLastCalledWith({ okay: true });
    fireEvent.keyDown(field, { key: 'Escape' });
    expect(send).toHaveBeenLastCalledWith({ cancel: true });
    fireEvent.click(getByText('Okay'));
    expect(send).toHaveBeenLastCalledWith({ okay: true });
  });

  it('tells_the_agent_watcher_to_take_control', () => {
    const { getByText, queryByText } = render(<Entry data={{ ...entry, live: false, focus: false, okay: null, cancel: null, take_control: 'Take control to answer.' }} send={vi.fn()} />);
    expect(getByText('Take control to answer.')).toBeTruthy();
    expect(queryByText('Okay')).toBeNull();
  });
});
