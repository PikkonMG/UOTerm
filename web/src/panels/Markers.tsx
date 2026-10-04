import type { MarkersData, Send } from './types';

/**
 * The markers manager: a tab for each marker file, a search, and the
 * markers of the open file that hold its words, each with its buttons.
 * The files are read-only here: a marker is added and changed in the
 * UOTerm window.
 */
export function Markers({ data, send }: { data: MarkersData; send: Send }) {
  return (
    <div class="markers">
      <div class="button-row">
        {data.files.map((file, at) => (
          <button type="button" class={`button${file.chosen ? ' chosen' : ''}`} key={file.words} onClick={() => send({ file: at })}>
            {file.words}
          </button>
        ))}
      </div>
      <input class="field" placeholder={data.search_hint} value={data.search} onInput={(event) => send({ search: event.currentTarget.value })} />
      <div class="marker-rows">
        {data.nothing && <p class="faint">{data.nothing}</p>}
        {data.rows.map((row) => (
          <div class="marker-row" key={row.at}>
            <span>{row.words}</span>
            {row.buttons.map((words, button) => (
              <button type="button" class="button goal" key={words} onClick={() => send({ row: { at: row.at, button } })}>
                {words}
              </button>
            ))}
          </div>
        ))}
      </div>
      <p class="small faint">{data.read_only}</p>
    </div>
  );
}
