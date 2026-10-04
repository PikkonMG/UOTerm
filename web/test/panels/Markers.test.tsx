import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Markers } from '../../src/panels/Markers';
import type { MarkersData } from '../../src/panels/types';

const data: MarkersData = {
  files: [
    { words: 'towns', chosen: true },
    { words: 'camps', chosen: false },
  ],
  search: '',
  search_hint: 'search the markers',
  rows: [{ at: 3, words: 'Bank  1434, 1699  ', buttons: ['Go'] }],
  nothing: null,
};

describe('Markers', () => {
  it('turns_its_files_searches_and_goes_to_a_marker', () => {
    const send = vi.fn();
    const { getByText, getByPlaceholderText } = render(<Markers data={data} send={send} />);
    fireEvent.click(getByText('camps'));
    expect(send).toHaveBeenLastCalledWith({ file: 1 });
    fireEvent.input(getByPlaceholderText('search the markers'), { target: { value: 'ba' } });
    expect(send).toHaveBeenLastCalledWith({ search: 'ba' });
    fireEvent.click(getByText('Go'));
    expect(send).toHaveBeenLastCalledWith({ row: { at: 3, button: 0 } });
  });
});
