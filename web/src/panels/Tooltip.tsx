import type { Point, TooltipData } from './types';

/**
 * The tooltip of the thing under the mouse, beside the mouse: the name in
 * the title face, the other lines dim, and what a click does.
 */
export function Tooltip({ data, at }: { data: TooltipData; at: Point }) {
  const [name, ...rest] = data.lines;
  return (
    <div class="tooltip" style={{ left: `${at.x}px`, top: `${at.y}px` }}>
      {name && <p class="tip-name">{name}</p>}
      {rest.map((line, index) => (
        <p class="tip-line dim" key={index}>
          {line}
        </p>
      ))}
      {data.footer && <p class="tip-footer goal-words">{data.footer}</p>}
    </div>
  );
}
