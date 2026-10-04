import type { RingData, Send } from './types';

/**
 * The ring round a right click: the acts of the window, then the lines of
 * the shard's menu, each where the view laid it. A click away shuts it;
 * the map takes no input while it is open.
 */
export function Ring({ data, send }: { data: RingData; send: Send }) {
  return (
    <div class="ring">
      <div class="ring-away" data-testid="ring-away" onPointerDown={() => send({ close: true })} />
      <span class="ring-hub" style={{ left: `${data.center.x}px`, top: `${data.center.y}px` }} />
      <span class="ring-name shadowed" style={{ left: `${data.center.x}px`, top: `${data.center.y}px` }}>
        {data.name}
      </span>
      {data.lines.map((line, at) => (
        <button
          type="button"
          key={at}
          class={`ring-line${line.enabled ? '' : ' faint'}`}
          disabled={!line.enabled}
          style={{ left: `${line.at.x}px`, top: `${line.at.y}px` }}
          onClick={() => line.enabled && send({ pick: at })}
        >
          {line.words}
        </button>
      ))}
    </div>
  );
}
