import { followDrag, ZONE_ATTRIBUTE } from './drag';
import { hoverOn, type Hover } from './hover';
import { ShareBar } from './Rows';
import type { NearData, Send } from './types';

/**
 * The near list: every mobile in view, its notoriety, its hits and how
 * far it is. A click targets it while the shard waits, a double click
 * attacks it in war and uses it in peace, a right click opens its ring,
 * a row dragged out opens a bar of its own, and an item dropped on a row
 * goes to the mobile.
 */
export function Near({ data, send, hover }: { data: NearData; send: Send; hover?: Hover }) {
  if (data.nobody) return <p class="faint">{data.nobody}</p>;
  return (
    <div class="near-rows">
      {data.rows.map((row) => (
        <div
          class="near-row"
          key={row.serial}
          {...{ [ZONE_ATTRIBUTE]: JSON.stringify(row.zone) }}
          {...hoverOn(row.hover, hover)}
          onClick={() => send({ click: row.serial })}
          onDblClick={() => send({ double: row.serial })}
          onContextMenu={(event) => {
            event.preventDefault();
            send({ menu: { serial: row.serial, x: event.clientX, y: event.clientY } });
          }}
          onPointerDown={(event) => {
            if (event.button === 0) followDrag(event, { dropped: (at) => send({ pull: { serial: row.serial, ...at } }) });
          }}
        >
          <span class="dot" style={{ background: row.color }} />
          <span class="near-name" style={{ color: row.name.color }}>
            {row.name.words}
          </span>
          {row.title && <span class="faint near-title">{row.title}</span>}
          {row.hits !== null && <ShareBar fill={row.hits} color={row.color} />}
          <span class="number dim">{row.distance}</span>
        </div>
      ))}
    </div>
  );
}
