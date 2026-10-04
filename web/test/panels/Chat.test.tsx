import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { Chat } from '../../src/panels/Chat';
import type { ChatPanelData } from '../../src/panels/types';

const MARKUP = '<b>hail</b>';

const data: ChatPanelData = {
  live: true,
  channels: {
    rows: [
      { name: 'General', picked: false, here: true, locked: null },
      { name: 'Guild', picked: true, here: false, locked: 'locked' },
    ],
    row_hint: 'Click: pick.',
    buttons: ['Join', 'Leave', 'Create'],
    asking: { label: 'Password:', hides: true, words: '', okay: 'Okay', cancel: 'Cancel' },
    lines: [{ who: 'Ann', words: MARKUP }],
    say_hint: 'Words for the channel',
    wish_hint: 'Say the channel',
    find: 'Find',
  },
  name_box: null,
  turn_on: null,
  note: null,
};

describe('Chat', () => {
  it('picks_joins_and_talks', () => {
    const send = vi.fn();
    const { getByText, getByPlaceholderText, container } = render(<Chat data={data} send={send} />);
    fireEvent.click(getByText('General'));
    expect(send).toHaveBeenLastCalledWith({ pick: 'General' });
    fireEvent.dblClick(getByText('General'));
    expect(send).toHaveBeenLastCalledWith({ join: 'General' });
    fireEvent.click(getByText('Leave'));
    expect(send).toHaveBeenLastCalledWith({ button: 1 });
    const password = container.querySelector('input[type="password"]') as HTMLInputElement;
    fireEvent.input(password, { target: { value: 'pw' } });
    expect(send).toHaveBeenLastCalledWith({ asking: 'pw' });
    fireEvent.click(getByText('Okay'));
    expect(send).toHaveBeenLastCalledWith({ answer: true });
    fireEvent.click(getByText('Cancel'));
    expect(send).toHaveBeenLastCalledWith({ answer: false });
    const say = getByPlaceholderText('Words for the channel') as HTMLInputElement;
    fireEvent.input(say, { target: { value: 'hi' } });
    fireEvent.keyDown(say, { key: 'Enter' });
    expect(send).toHaveBeenLastCalledWith({ say: 'hi' });
    const wish = getByPlaceholderText('Say the channel');
    fireEvent.input(wish, { target: { value: 'the guild' } });
    fireEvent.click(getByText('Find'));
    expect(send).toHaveBeenLastCalledWith({ wish: 'the guild' });
    expect(container.querySelector('b')).toBeNull();
    expect(container.textContent).toContain(MARKUP);
  });

  it('asks_for_the_chat_name_or_turns_the_chat_on', () => {
    const send = vi.fn();
    const named = render(<Chat data={{ ...data, channels: null, name_box: { words: 'Choose a name', most: 30, okay: 'Okay' } }} send={send} />);
    const field = named.container.querySelector('input') as HTMLInputElement;
    expect(field.maxLength).toBe(30);
    fireEvent.input(field, { target: { value: 'Mara' } });
    fireEvent.click(named.getByText('Okay'));
    expect(send).toHaveBeenLastCalledWith({ name: 'Mara' });
    const off = render(<Chat data={{ ...data, channels: null, turn_on: 'Turn the chat on' }} send={send} />);
    fireEvent.click(off.getByText('Turn the chat on'));
    expect(send).toHaveBeenLastCalledWith({ turn_on: true });
  });
});
