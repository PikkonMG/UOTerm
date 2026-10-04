import { fireEvent, render } from '@testing-library/preact';
import { describe, expect, it, vi } from 'vitest';
import { ChatLine } from '../../src/panels/ChatLine';
import type { ChatData } from '../../src/panels/types';

const data: ChatData = {
  text: '',
  open: true,
  hidden: false,
  live: true,
  idle_words: 'Take control to talk.',
  mode_words: 'Do',
  hint: 'A command. Press Enter.',
  pin_words: 'Pin',
  strip: null,
};

describe('ChatLine', () => {
  it('gives_the_view_the_words_of_its_field_and_marks_it_as_the_chat_line', () => {
    const input = vi.fn();
    const { getByPlaceholderText } = render(<ChatLine data={data} send={vi.fn()} input={input} />);
    const field = getByPlaceholderText('A command. Press Enter.') as HTMLInputElement;
    expect(field.hasAttribute('data-chat')).toBe(true);
    fireEvent.input(field, { target: { value: 'hail' } });
    expect(input).toHaveBeenCalledWith({ kind: 'ChatWords', text: 'hail' });
  });

  it('turns_its_mode_and_pins_a_command', () => {
    const send = vi.fn();
    const { getByText } = render(<ChatLine data={data} send={send} input={vi.fn()} />);
    fireEvent.click(getByText('Do'));
    expect(send).toHaveBeenCalledWith({ mode: true });
    fireEvent.click(getByText('Pin'));
    expect(send).toHaveBeenLastCalledWith({ pin: true });
  });

  it('says_to_take_control_while_the_agent_has_the_character', () => {
    const { getByText, queryByRole } = render(<ChatLine data={{ ...data, live: false }} send={vi.fn()} input={vi.fn()} />);
    expect(getByText('Take control to talk.')).toBeTruthy();
    expect(queryByRole('textbox')).toBeNull();
  });
});
