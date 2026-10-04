import { ZONE_ATTRIBUTE } from './drag';
import { hoverOn, type Hover } from './hover';
import { Picture } from './Picture';
import type { CountersData, Send } from './types';

const SECONDARY = 2;

/**
 * The counter bar: a cell for each item the Counters page names, with how
 * many the pack holds. A click or a double click uses one (the view knows
 * which by the options), a right click with Alt empties a cell, an item
 * dropped on a cell counts from then on, and the switch fixes the cells.
 */
export function Counters({ data, send, hover }: { data: CountersData; send: Send; hover?: Hover }) {
  const side = `${data.side}px`;
  return (
    <div class="counters">
      <button type="button" class="button counters-fixed small" title={data.fixed_hint} style={{ color: data.fixed.color }} onClick={() => send({ fixed: true })}>
        {data.fixed.words}
      </button>
      <div class="counter-cells" style={{ gridTemplateColumns: `repeat(${data.columns}, ${side})`, gap: `${data.gap}px` }}>
        {data.cells.map((cell, at) => (
          <div
            class={`counter-cell${cell.flashing ? ' flashing' : ''}`}
            key={at}
            style={{ width: side, height: side }}
            {...(cell.zone ? { [ZONE_ATTRIBUTE]: JSON.stringify(cell.zone) } : {})}
            {...hoverOn(cell.hover, hover)}
            onClick={() => send({ click: at })}
            onDblClick={() => send({ double: at })}
            onContextMenu={(event) => {
              event.preventDefault();
              if (event.button === SECONDARY) send({ menu: { cell: at, alt: event.altKey } });
            }}
          >
            <Picture picture={cell.picture} />
            {cell.amount && (
              <span class="cell-amount small number shadowed" style={{ color: cell.amount.color }}>
                {cell.amount.words}
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
