import type { Send, TipData } from './types';

/** The tip of the day or the notice of the shard, as words; a tip asks for the one before or after it. */
export function Tip({ data, send }: { data: TipData; send: Send }) {
  return (
    <div class="notice">
      <div class="notice-words">{data.words}</div>
      {data.previous && (
        <div class="button-row">
          <button type="button" class="button" onClick={() => send({ previous: true })}>
            {data.previous}
          </button>
          <button type="button" class="button" onClick={() => send({ next: true })}>
            {data.next}
          </button>
        </div>
      )}
    </div>
  );
}
