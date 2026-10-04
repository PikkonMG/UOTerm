import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { OldMenu } from '../../src/panels/OldMenu';

describe('OldMenu', () => {
  it('picks_an_answer_and_cancels', () => {
    const send = vi.fn();
    const data = { live: true, entries: [{ picture: null, name: 'dagger' }, { picture: null, name: 'sword' }], cancel: 'Cancel' };
    const { getByText } = render(<OldMenu data={data} send={send} />);
    fireEvent.click(getByText('sword'));
    expect(send).toHaveBeenLastCalledWith({ pick: 1 });
    fireEvent.click(getByText('Cancel'));
    expect(send).toHaveBeenLastCalledWith({ cancel: true });
  });
});
