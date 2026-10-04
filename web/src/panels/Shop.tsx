import { hoverOn, type Hover } from './hover';
import { Picture } from './Picture';
import type { Send, ShopData } from './types';

/**
 * The list of a shopkeeper with its cart: a click names a good, a double
 * click takes one more and with Shift all, and the steps count it; the
 * foot has the total, the gold, and the buttons that deal, clear and close.
 */
export function Shop({ data, send, hover }: { data: ShopData; send: Send; hover?: Hover }) {
  return (
    <div class="shop">
      <div class="shop-goods">
        {data.goods.map((good) => (
          <div
            class="shop-good"
            key={good.serial}
            {...hoverOn(good.hover, hover)}
            onClick={() => send({ click: good.serial })}
            onDblClick={(event) => send({ take: good.serial, all: event.shiftKey })}
          >
            <span class="cell-art">
              <Picture picture={good.picture} />
            </span>
            <span class="shop-name">{good.name}</span>
            <span class="number small faint">{good.left}</span>
            <span class="number waiting">{good.price}</span>
            {data.live && (
              <span class="shop-steps">
                <button
                  type="button"
                  class="button segment small"
                  onClick={(event) => {
                    event.stopPropagation();
                    send({ step: { serial: good.serial, up: false, big: event.shiftKey } });
                  }}
                >
                  {data.step_down}
                </button>
                <span class="number shop-count" style={{ color: good.count.color }}>
                  {good.count.words}
                </span>
                <button
                  type="button"
                  class="button segment small"
                  onClick={(event) => {
                    event.stopPropagation();
                    send({ step: { serial: good.serial, up: true, big: event.shiftKey } });
                  }}
                >
                  {data.step_up}
                </button>
              </span>
            )}
          </div>
        ))}
      </div>
      <div class="shop-totals">
        {data.gold && <span class="number waiting">{data.gold}</span>}
        <span class="number shop-total">{data.total}</span>
      </div>
      {data.live && (
        <div class="button-row">
          <button type="button" class="button goal" onClick={() => send({ deal: true })}>
            {data.deal}
          </button>
          <button type="button" class="button" onClick={() => send({ clear: true })}>
            {data.clear}
          </button>
          <button type="button" class="button dim" onClick={() => send({ close: true })}>
            {data.close}
          </button>
        </div>
      )}
    </div>
  );
}
