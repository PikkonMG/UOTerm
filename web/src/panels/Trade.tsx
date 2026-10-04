import { followDrag, ZONE_ATTRIBUTE } from './drag';
import { hoverOn, type Hover } from './hover';
import { Picture } from './Picture';
import type { Send, TradeData, TradedItem } from './types';

const PRIMARY = 0;

/** One item of a trade: a click names it, a double click uses it, and the character drags his own back. */
function Traded({ item, mine, live, send, hover }: { item: TradedItem; mine: boolean; live: boolean; send: Send; hover?: Hover }) {
  return (
    <div
      class="trade-cell"
      role="img"
      aria-label={item.hover.words}
      {...hoverOn(item.hover, hover)}
      onClick={() => send({ click: item.serial })}
      onDblClick={() => send({ double: item.serial })}
      onPointerDown={(event) => {
        if (mine && live && event.button === PRIMARY) followDrag(event, { carries: true, started: () => send({ drag: item.serial }) });
      }}
    >
      <Picture picture={item.picture} />
      {item.amount && <span class="cell-amount number small shadowed">{item.amount}</span>}
    </div>
  );
}

/**
 * A trade with another player: each side with whether it accepted and
 * its items, the gold and platinum the character offers (held by the view
 * to what he has) and the other's offer, and Accept and Cancel. An item
 * dropped anywhere on it goes on the character's side.
 */
export function Trade({ data, send, hover }: { data: TradeData; send: Send; hover?: Hover }) {
  return (
    <div class="trade-window" {...{ [ZONE_ATTRIBUTE]: JSON.stringify(data.zone) }}>
      <div class="trade-sides">
        {data.sides.map((side, at) => (
          <div class="trade-side" key={at}>
            <p style={{ color: side.head.color }}>{side.head.words}</p>
            <div class="trade-cells">
              {side.items.map((item) => (
                <Traded item={item} mine={side.mine} live={data.live} send={send} hover={hover} key={item.serial} />
              ))}
            </div>
            {side.mine
              ? data.coins.map((coin) => (
                  <label class="coin-row" key={coin.label}>
                    <span class="dim">{coin.label}</span>
                    <input
                      class="field number"
                      value={coin.words}
                      disabled={!data.live}
                      onInput={(event) => send({ [coin.platinum ? 'platinum' : 'gold']: event.currentTarget.value })}
                    />
                    <span class="number small faint">{coin.owned}</span>
                  </label>
                ))
              : data.theirs.map((coin) => (
                  <p class="coin-row" key={coin.label}>
                    <span class="dim">{coin.label}</span>
                    <span class="number waiting">{coin.value}</span>
                  </p>
                ))}
          </div>
        ))}
      </div>
      {data.accept && (
        <div class="button-row">
          <button type="button" class="button" style={{ color: data.accept.color }} onClick={() => send({ accept: true })}>
            {data.accept.words}
          </button>
          {data.cancel && (
            <button type="button" class="button alarm" onClick={() => send({ cancel: true })}>
              {data.cancel}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
