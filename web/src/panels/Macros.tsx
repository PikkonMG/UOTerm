import { PANEL_ATTRIBUTE } from './drag';
import { Note } from './Note';
import type { MacrosData, Send } from './types';

const ENTER = 'Enter';

/**
 * The macro editor in the middle of the room: the macros of the scripts
 * folder, the name and the lines of the one being edited, the field of
 * plain words for Jev, and the buttons. What each does is the view's.
 */
export function Macros({ data, send }: { data: MacrosData; send: Send }) {
  const place = data.place;
  return (
    <section
      class="panel macros"
      style={{ left: `${place.x}px`, top: `${place.y}px`, width: `${place.w}px`, height: `${place.h}px` }}
      {...{ [PANEL_ATTRIBUTE]: 'macros' }}
    >
      <div class="macros-head">
        <h1 class="title">{data.title}</h1>
        <span class="small number dim">{data.status}</span>
      </div>
      <div class="macros-body">
        <div class="macros-list">
          {data.names.map((name, at) => (
            <button type="button" class={`button row${name.chosen ? ' chosen' : ''}`} key={at} onClick={() => send({ pick: at })}>
              {name.words}
            </button>
          ))}
        </div>
        <div class="macros-edit">
          <input class="field" placeholder={data.name_hint} value={data.name} onInput={(event) => send({ name: event.currentTarget.value })} />
          <textarea class="field number macros-lines" placeholder={data.lines_hint} value={data.lines} onInput={(event) => send({ lines: event.currentTarget.value })} />
          <div class="wish-row">
            <input
              class="field"
              placeholder={data.wish_hint}
              value={data.wish}
              onInput={(event) => send({ wish: event.currentTarget.value })}
              onKeyDown={(event) => {
                if (event.key === ENTER) send({ add_line: true });
              }}
            />
            <button type="button" class="button" style={{ color: data.add_line.color }} onClick={() => send({ add_line: true })}>
              {data.add_line.words}
            </button>
          </div>
          <div class="button-row">
            {data.buttons.map((button, at) => (
              <button type="button" class="button" key={at} disabled={!button.enabled} style={{ color: button.color }} onClick={() => send({ button: at })}>
                {button.words}
              </button>
            ))}
          </div>
        </div>
      </div>
      <Note note={data.note} />
    </section>
  );
}
