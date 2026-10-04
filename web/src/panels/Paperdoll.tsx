import { followDrag, ZONE_ATTRIBUTE } from './drag';
import { hoverOn, type Hover } from './hover';
import { Picture } from './Picture';
import { ShareBar } from './Rows';
import type { PaperdollData, Send } from './types';

const PRIMARY = 0;

/**
 * The paperdoll of a mobile: his figure and health, what he wears (a
 * click names an item, a double click uses it, a drag takes it off a doll
 * the character dresses), and the buttons of the doll.
 */
export function Paperdoll({ data, send, hover }: { data: PaperdollData; send: Send; hover?: Hover }) {
  return (
    <div class="paperdoll" {...(data.zone ? { [ZONE_ATTRIBUTE]: JSON.stringify(data.zone) } : {})}>
      <div class="paperdoll-top">
        <div class="paperdoll-figure">
          {data.out_of_sight ? <span class="dim">{data.out_of_sight}</span> : <Picture picture={data.figure} />}
          {data.health !== null && <ShareBar fill={data.health} color="var(--hits)" />}
        </div>
        <div class="paperdoll-rows">
          {data.nothing && <p class="faint small">{data.nothing}</p>}
          {data.rows.map((row) => (
            <div
              class="worn-row"
              key={row.serial}
              {...hoverOn(row.hover, hover)}
              onClick={() => send({ click: row.serial })}
              onDblClick={() => send({ double: row.serial })}
              onPointerDown={(event) => {
                if (data.dresses && data.live && event.button === PRIMARY) followDrag(event, { carries: true, started: () => send({ drag: row.serial }) });
              }}
            >
              <span class="cell-art">
                <Picture picture={row.picture} />
              </span>
              <span class="dim small worn-words">{row.words}</span>
            </div>
          ))}
        </div>
      </div>
      <div class="button-row">
        {data.buttons.map((words, at) => (
          <button type="button" class="button" key={at} onClick={() => send({ button: at })}>
            {words}
          </button>
        ))}
        <button type="button" class="button dim" onClick={() => send({ close: true })}>
          {data.close}
        </button>
      </div>
    </div>
  );
}
