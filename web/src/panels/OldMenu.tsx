import { Picture } from './Picture';
import type { OldMenuData, Send } from './types';

/** The old-style menu of the shard: a click on an answer picks it; Cancel says none. */
export function OldMenu({ data, send }: { data: OldMenuData; send: Send }) {
  return (
    <div class="old-menu">
      {data.entries.map((entry, at) => (
        <button type="button" class="button row menu-entry" key={at} disabled={!data.live} onClick={() => send({ pick: at })}>
          {entry.picture && (
            <span class="cell-art">
              <Picture picture={entry.picture} />
            </span>
          )}
          <span>{entry.name}</span>
        </button>
      ))}
      {data.cancel && (
        <button type="button" class="button dim" onClick={() => send({ cancel: true })}>
          {data.cancel}
        </button>
      )}
    </div>
  );
}
