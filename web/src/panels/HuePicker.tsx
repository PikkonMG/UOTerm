import type { HueGridData, Send } from './types';

/** The words of a cell of the grid, for its button. */
const cellWords = (at: number) => `hue ${at + 1}`;

/**
 * The grid of hues of every panel that picks one: a click picks a cell,
 * and the slider under it shifts the shade of the whole grid. The view
 * gives the color of each cell.
 */
export function HuePicker({ data, live, send }: { data: HueGridData; live: boolean; send: Send }) {
  return (
    <div class="hue-picker">
      <div class="hue-cells" style={{ gridTemplateColumns: `repeat(${data.columns}, 1fr)` }}>
        {data.cells.map((color, at) => (
          <button
            type="button"
            class={`hue-cell${at === data.chosen ? ' chosen' : ''}`}
            aria-label={cellWords(at)}
            key={at}
            style={{ background: color }}
            disabled={!live}
            onClick={() => send({ cell: at })}
          />
        ))}
      </div>
      <label class="points">
        <span class="dim small">{data.shade_words}</span>
        <input type="range" min={data.shade_least} max={data.shade_most} value={data.shade} disabled={!live} onInput={(event) => send({ shade: Number(event.currentTarget.value) })} />
      </label>
    </div>
  );
}
