import type { ProfileData, Send } from './types';

/**
 * The profile of a character: the title, what the shard and the owner
 * wrote, as text; while the owner writes, a field for his words. Write
 * opens the words, Save sends them.
 */
export function Profile({ data, send }: { data: ProfileData; send: Send }) {
  return (
    <div class="profile">
      <p class="dim">{data.title}</p>
      {data.writing === null ? (
        <p class="profile-words">{data.words}</p>
      ) : (
        <textarea class="field profile-words" placeholder={data.hint} value={data.writing} onInput={(event) => send({ words: event.currentTarget.value })} />
      )}
      <div class="button-row">
        {data.write && (
          <button type="button" class="button goal" onClick={() => send({ write: true })}>
            {data.write}
          </button>
        )}
        <button type="button" class="button dim" onClick={() => send({ close: true })}>
          {data.close}
        </button>
      </div>
    </div>
  );
}
