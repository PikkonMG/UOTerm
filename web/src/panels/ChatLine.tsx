import { useEffect, useRef } from 'preact/hooks';
import type { InputEvent } from '../input/events';
import { CHAT_ATTRIBUTE } from '../input/keys';
import type { ChatData, Send } from './types';

interface ChatLineProps {
  data: ChatData;
  send: Send;
  /** Gives the view an input event: the words of the field. */
  input(event: InputEvent): void;
}

/**
 * The chat line: the mode button, the field, and Pin for a command. The
 * field types its words itself and gives them to the view; Enter and
 * Escape go to the view as keys, which decide what the line does. The view
 * asks the field to take the keys or let them go.
 */
export function ChatLine({ data, send, input }: ChatLineProps) {
  const field = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (data.focus === 'take') field.current?.focus();
    if (data.focus === 'leave') field.current?.blur();
  }, [data.focus]);

  useEffect(() => {
    if (!data.paste || !navigator.clipboard) return;
    navigator.clipboard.readText().then(
      (pasted) => input({ kind: 'ChatWords', text: `${field.current?.value ?? ''}${pasted}` }),
      () => {},
    );
  }, [data.paste]);

  if (data.hidden) return null;
  if (!data.live) return <p class="chat-line faint">{data.idle_words}</p>;
  return (
    <div class="chat-line">
      <button type="button" class="button chat-mode" onClick={() => send({ mode: true })}>
        {data.mode_words}
      </button>
      <input
        ref={field}
        class={`field chat-field${data.open ? ' open' : ''}`}
        {...{ [CHAT_ATTRIBUTE]: '' }}
        value={data.text}
        placeholder={data.hint}
        onInput={(event) => input({ kind: 'ChatWords', text: event.currentTarget.value })}
      />
      {data.pin_words && (
        <button type="button" class="button" onClick={() => send({ pin: true })}>
          {data.pin_words}
        </button>
      )}
    </div>
  );
}
