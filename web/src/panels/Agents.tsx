import { Picture } from './Picture';
import type { AgentWindowData, Send } from './types';

const ENTER = 'Enter';

/**
 * One agent window, the list of them, or the ignore list: the rows the
 * view lays, each with its buttons, the pictures of the bag items, and a
 * field whose words the view keeps.
 */
export function Agents({ data, send }: { data: AgentWindowData; send: Send }) {
  return (
    <div class="agents">
      {data.rows.map((row, at) => {
        switch (row.kind) {
          case 'words':
            return (
              <p class="small" key={at} style={{ color: row.words.color }}>
                {row.words.words}
              </p>
            );
          case 'labeled':
            return (
              <div class="agent-row" key={at}>
                <span class="small agent-words">{row.words}</span>
                {row.buttons.map((button, place) => (
                  <button type="button" class="button small" key={place} style={{ color: button.color, width: `${row.width}px` }} onClick={() => send({ press: { row: at, at: place } })}>
                    {button.words}
                  </button>
                ))}
              </div>
            );
          case 'buttons':
            return (
              <div class="agent-row" key={at}>
                {row.buttons.map((button, place) => (
                  <button type="button" class="button small agent-wide" key={place} style={{ color: button.color }} onClick={() => send({ press: { row: at, at: place } })}>
                    {button.words}
                  </button>
                ))}
              </div>
            );
          case 'pictures':
            return (
              <div class="agent-pictures" key={at}>
                {row.pictures.map((picture, place) => (
                  <button type="button" class="cell-art agent-picture" key={place} title={picture.name} onClick={() => send({ picture: { row: at, at: place } })}>
                    <Picture picture={picture.picture} words={picture.name} />
                  </button>
                ))}
              </div>
            );
          case 'field':
            return (
              <div class="agent-row" key={at}>
                <input
                  class="field"
                  placeholder={row.hint}
                  value={row.words}
                  onInput={(event) => send({ typing: event.currentTarget.value })}
                  onKeyDown={(event) => {
                    if (event.key === ENTER) send({ submit: at });
                  }}
                />
                <button type="button" class="button small goal" style={{ width: `${row.width}px` }} onClick={() => send({ submit: at })}>
                  {row.button}
                </button>
              </div>
            );
        }
      })}
    </div>
  );
}
