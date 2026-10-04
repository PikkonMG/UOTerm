import { Picture } from './Picture';
import { ShareBar } from './Rows';
import type { DurabilityData } from './types';

/** The worn items with a durability, the most worn first, each with a bar that turns to the alarm under the warning. */
export function Durability({ data }: { data: DurabilityData }) {
  const art = { width: `${data.art}px`, height: `${data.art}px` };
  return (
    <div class="durability">
      {data.none && <p class="faint small">{data.none}</p>}
      {data.rows.map((row, at) => (
        <div class="wear-row" key={at}>
          <span class="cell-art" style={art}>
            <Picture picture={row.picture} />
          </span>
          <div class="wear-words">
            <p class="detail small">
              <span>{row.name}</span>
              <span class="number" style={{ color: row.words.color }}>
                {row.words.words}
              </span>
            </p>
            <ShareBar fill={row.fill} color={row.words.color} />
          </div>
        </div>
      ))}
    </div>
  );
}
