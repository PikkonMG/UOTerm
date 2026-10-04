import type { ControlBarData, Send } from './types';

/**
 * The control bar at the middle of the top: where the character is, the
 * button of the panel launcher and the arrow that folds the bar; under a
 * rule, what the human does and one row of buttons, which
 * `clicks::bar_buttons` of the view names.
 */
export function ControlBar({ data, send }: { data: ControlBarData; send: Send }) {
  const { location } = data;
  return (
    <div class="bar-body">
      <div class="bar-place">
        <button type="button" class={`button bar-launcher${data.launcher_shows ? ' chosen' : ''}`} onClick={() => send({ launcher: true })}>
          {data.launcher_words}
        </button>
        <span class="place-words">
          <span class="number">{location.numbers}</span>
          <span class="dim">{location.map_words}</span>
          <span class="number">{location.map}</span>
          {location.facing && (
            <>
              <span class="dim">{location.faces_words}</span>
              <span>{location.facing}</span>
            </>
          )}
        </span>
        <button type="button" class="fold-arrow" aria-expanded={!data.folded} onClick={() => send({ fold: true })}>
          <svg viewBox="0 0 16 16" class="mark-art" aria-hidden="true">
            <path d={data.folded ? 'M4 6 L8 10 L12 6' : 'M4 10 L8 6 L12 10'} />
          </svg>
        </button>
      </div>
      {!data.folded && (
        <>
          <hr class="rule" />
          {data.status && <p class="bar-status waiting">{data.status}</p>}
          <div class="bar-buttons">
            {data.buttons.map((words, at) => (
              <button type="button" class="button segment" key={at} onClick={() => send({ press: at })}>
                {words}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
