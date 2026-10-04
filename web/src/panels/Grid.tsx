import { mods } from '../input/keys';
import { followDrag, ZONE_ATTRIBUTE } from './drag';
import { hoverOn, type Hover } from './hover';
import { Picture } from './Picture';
import type { GridCell, GridData, Send, TitleButton } from './types';

const PERCENT = 100;
const PRIMARY = 0;

/** A button of the head of a grid, with its tip. */
function HeadButton({ button, pressed }: { button: TitleButton | null; pressed(): void }) {
  if (!button) return null;
  return (
    <button type="button" class="button segment small" style={{ color: button.color }} title={button.hint} onClick={pressed}>
      {button.words}
    </button>
  );
}

/** The bar at the foot of a pile of a grid loot: a press on it sets how many a click grabs. */
function PileBar({ slot, share, send }: { slot: number; share: number; send: Send }) {
  return (
    <span
      class="pile-bar"
      onPointerDown={(event) => {
        event.stopPropagation();
        const box = event.currentTarget.getBoundingClientRect();
        send({ amount: { slot, share: (event.clientX - box.left) / Math.max(box.width, 1) } });
      }}
    >
      <span class="pile-fill" style={{ width: `${share * PERCENT}%` }} />
    </span>
  );
}

/** One cell: a press sends its slot, the button, the clicks counted and the keys; the view decides. */
function Cell({ cell, data, send, hover }: { cell: GridCell; data: GridData; send: Send; hover?: Hover }) {
  const item = cell.item;
  const press = (event: MouseEvent, extra: Record<string, unknown>) => send({ cell: { slot: cell.slot, count: event.detail || 1, ...extra, mods: mods(event) } });
  return (
    <div
      class={`grid-cell${item?.chosen ? ' chosen' : ''}`}
      style={{ width: `${data.side}px`, height: `${data.side}px`, borderColor: item?.mark ?? undefined }}
      {...(item ? { [ZONE_ATTRIBUTE]: JSON.stringify(item.zone) } : {})}
      {...(item ? hoverOn(item.hover, hover) : {})}
      onClick={(event) => press(event, {})}
      onDblClick={(event) => press(event, { double: true })}
      onContextMenu={(event) => {
        event.preventDefault();
        press(event, { secondary: true, x: event.clientX, y: event.clientY });
      }}
      onPointerDown={(event) => {
        if (item && data.live && event.button === PRIMARY) followDrag(event, { carries: true, started: () => send({ drag: cell.slot }) });
      }}
    >
      {cell.locked && <span class="lock-dot" />}
      {item && (
        <>
          <span class="cell-picture" style={{ opacity: item.alpha }}>
            <Picture picture={item.picture} most={data.art_scale} />
          </span>
          {item.amount && (
            <span class="cell-amount number small shadowed" style={{ color: item.amount.color, opacity: item.alpha }}>
              {item.amount.words}
            </span>
          )}
          {item.slider !== null && <PileBar slot={cell.slot} share={item.slider} send={send} />}
        </>
      )}
    </div>
  );
}

/**
 * A grid container: its search, its count and the buttons of its head,
 * the cells of its items in the slots the view arranged, and the row that
 * moves the chosen items. An item dropped on it goes in; on a bag or a
 * pile of its kind, into that.
 */
export function Grid({ data, send, hover }: { data: GridData; send: Send; hover?: Hover }) {
  return (
    <div class="grid" {...{ [ZONE_ATTRIBUTE]: JSON.stringify(data.zone) }}>
      <div class="grid-head">
        <input class="field small grid-search" placeholder={data.search_hint} value={data.search} onInput={(event) => send({ search: event.currentTarget.value })} />
        <span class="number dim">{data.count}</span>
        <HeadButton button={data.favorite} pressed={() => send({ favorite: true })} />
        <HeadButton button={data.loot_all} pressed={() => send({ loot_all: true })} />
        <HeadButton button={data.loot_bag} pressed={() => send({ loot_bag: true })} />
      </div>
      <div class="grid-cells" style={{ gridTemplateColumns: `repeat(${data.columns}, ${data.side}px)` }}>
        {data.cells.map((cell) => (
          <Cell cell={cell} data={data} send={send} hover={hover} key={cell.slot} />
        ))}
      </div>
      {data.strip && (
        <div class="grid-strip">
          {data.strip.buttons.map((words, at) => (
            <button type="button" class="button segment small" key={at} onClick={() => send({ strip: at })}>
              {words}
            </button>
          ))}
          <span class="number waiting">{data.strip.count}</span>
        </div>
      )}
    </div>
  );
}
