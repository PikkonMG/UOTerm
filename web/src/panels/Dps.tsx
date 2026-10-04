import type { DpsData, Send } from './types';

/** The damage meter: Start, Pause or Resume and Stop while the human has control, the damage each second, the whole, and each mobile. */
export function Dps({ data, send }: { data: DpsData; send: Send }) {
  return (
    <div class="dps">
      {data.buttons.length > 0 && (
        <div class="button-row">
          {data.buttons.map((words, at) => (
            <button type="button" class="button" key={at} onClick={() => send({ button: at })}>
              {words}
            </button>
          ))}
        </div>
      )}
      <p class="detail number">
        <span style={{ color: data.per_second.color }}>{data.per_second.words}</span>
        <span class="dim">{data.total}</span>
      </p>
      {data.none && <p class="faint small">{data.none}</p>}
      {data.rows.map((row, at) => (
        <p class="detail small" key={at}>
          <span>{row.name}</span>
          <span class="dim number">{row.words}</span>
        </p>
      ))}
    </div>
  );
}
