import type { LootData, Send } from './types';

/** The corpses near the character, nearest first: each one opens or loots with a click. */
export function Loot({ data, send }: { data: LootData; send: Send }) {
  return (
    <div class="loot">
      {data.none && <p class="faint">{data.none}</p>}
      {data.rows.map((row) => (
        <div class="loot-row" key={row.serial}>
          <span class="loot-name">{row.name}</span>
          <span class="number small dim">{row.state}</span>
          {data.live && (
            <>
              <button type="button" class="button segment small" onClick={() => send({ open: row.serial })}>
                {data.open}
              </button>
              <button type="button" class="button segment small goal-words" onClick={() => send({ loot: row.serial })}>
                {data.loot}
              </button>
            </>
          )}
        </div>
      ))}
      {data.loot_all && (
        <button type="button" class="button segment goal-words" onClick={() => send({ loot_all: true })}>
          {data.loot_all}
        </button>
      )}
    </div>
  );
}
