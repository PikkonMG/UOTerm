import { useLayoutEffect, useRef } from 'preact/hooks';
import type { EntryData, Send } from './types';

const ENTER = 'Enter';
const ESCAPE = 'Escape';

/**
 * The dialog for words the shard waits for: its description and the field
 * that keeps the rules of the shard (the view keeps them). The field takes
 * the keys once as the dialog opens; Enter or Okay answers, and Escape or
 * Cancel says no when the shard lets the player.
 */
export function Entry({ data, send }: { data: EntryData; send: Send }) {
  const field = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    if (!data.focus || !field.current) return;
    field.current.focus();
    send({ focused: true });
  }, [data.focus]);
  return (
    <div class="entry">
      <p class="dim">{data.description}</p>
      <input
        ref={field}
        class="field"
        placeholder={data.hint}
        value={data.words}
        disabled={!data.live}
        onInput={(event) => send({ words: event.currentTarget.value })}
        onKeyDown={(event) => {
          if (event.key === ENTER) send({ okay: true });
          else if (event.key === ESCAPE && data.cancel) send({ cancel: true });
        }}
      />
      {data.take_control && <p class="faint">{data.take_control}</p>}
      {data.okay && (
        <div class="button-row">
          <button type="button" class="button goal" onClick={() => send({ okay: true })}>
            {data.okay}
          </button>
          {data.cancel && (
            <button type="button" class="button dim" onClick={() => send({ cancel: true })}>
              {data.cancel}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
