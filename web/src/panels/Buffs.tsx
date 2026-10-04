import { hoverOn, type Hover } from './hover';
import { Picture } from './Picture';
import type { BuffsData } from './types';

/** The buff bar: each buff's icon, or its short name with no picture, and the time it has left. */
export function Buffs({ data, hover }: { data: BuffsData; hover?: Hover }) {
  const side = { width: `${data.side}px`, height: `${data.side}px` };
  return (
    <div class="buffs">
      {data.icons.map((icon, at) => (
        <div class="buff" key={at} {...hoverOn(icon.hover, hover)}>
          <span class="cell-art buff-icon" style={side}>
            {icon.picture ? <Picture picture={icon.picture} /> : <span class="small">{icon.short}</span>}
          </span>
          {icon.time && (
            <span class="small number" style={{ color: icon.time.color }}>
              {icon.time.words}
            </span>
          )}
        </div>
      ))}
    </div>
  );
}
