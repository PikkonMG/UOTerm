import { useLayoutEffect, useRef } from 'preact/hooks';
import type { InputEvent } from '../input/events';
import { CHAT_ATTRIBUTE } from '../input/keys';
import type { ChatData, Send } from './types';

/** What the view last asked of the field of the chat line, until the field is there to do it. */
let focusAsked: boolean | null = null;

/** Takes the keys to the field, or lets them go; true when the field was there. */
function applyFocus(field: HTMLInputElement | null): boolean {
  if (!field || focusAsked === null) return field !== null;
  if (focusAsked) field.focus();
  else field.blur();
  focusAsked = null;
  return true;
}

/**
 * The view asks the field of the chat line to take the keys or let them
 * go. A field not drawn yet, as a line that opens in this frame, does it
 * when it is drawn.
 */
export function askChatFocus(take: boolean): void {
  focusAsked = take;
  applyFocus(document.querySelector<HTMLInputElement>(`[${CHAT_ATTRIBUTE}]`));
}

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
 * asks the field to take the keys or let them go with its out-calls.
 */
export function ChatLine({ data, send, input }: ChatLineProps) {
  const field = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    applyFocus(field.current);
  });

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
