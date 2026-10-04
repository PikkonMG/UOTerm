import { Picture } from './Picture';
import type { RaceData, Send } from './types';

/** The words of a hue of the palette, for its button. */
const hueWords = (at: number) => `hue ${at + 1}`;

/**
 * The race change: the hair and beard styles in lists, the figure of the
 * new looks, the color of each part with the palette of the one the
 * player picks, and the buttons that send the looks or keep the old ones.
 */
export function Race({ data, send }: { data: RaceData; send: Send }) {
  return (
    <div class="race">
      <div class="race-styles">
        {data.styles.map((list, at) => (
          <label class="race-part" key={list.label}>
            <span class="dim">{list.label}</span>
            <select class="field" value={String(list.chosen)} disabled={!data.live} onChange={(event) => send({ style: { list: at, at: Number(event.currentTarget.value) } })}>
              {list.choices.map((words, choice) => (
                <option value={String(choice)} key={choice}>
                  {words}
                </option>
              ))}
            </select>
          </label>
        ))}
      </div>
      <div class="race-figure">
        <Picture picture={data.figure} />
      </div>
      <div class="race-paints">
        {data.paints.map((paint, at) => (
          <div class="race-part" key={paint.label}>
            <span class="dim">{paint.label}</span>
            <button
              type="button"
              class={`swatch${paint.picking ? ' chosen' : ''}`}
              aria-label={paint.label}
              title={data.hint}
              style={{ background: paint.color }}
              disabled={!data.live}
              onClick={() => send({ paint: at })}
            />
          </div>
        ))}
        {data.palette && (
          <div class="race-palette" style={{ gridTemplateColumns: `repeat(${data.palette.columns}, 1fr)` }}>
            {data.palette.hues.map((color, at) => (
              <button
                type="button"
                class={`hue-cell${at === data.palette?.chosen ? ' chosen' : ''}`}
                aria-label={hueWords(at)}
                key={at}
                style={{ background: color }}
                onClick={() => send({ hue: at })}
              />
            ))}
          </div>
        )}
      </div>
      {data.change && (
        <div class="button-row race-foot">
          <button type="button" class="button goal" onClick={() => send({ change: true })}>
            {data.change}
          </button>
          <button type="button" class="button dim" onClick={() => send({ keep: true })}>
            {data.keep}
          </button>
        </div>
      )}
    </div>
  );
}
