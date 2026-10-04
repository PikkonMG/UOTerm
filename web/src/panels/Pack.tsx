import { DetailRow } from './Rows';
import type { PackData } from './types';

/** The pack: the heading, then the weight with the gold at its right end, the clock and the lists. */
export function Pack({ data }: { data: PackData }) {
  const [first, ...rest] = data.rows;
  return (
    <div class="rows">
      <h2 class="title">{data.heading}</h2>
      {first && (
        <div class="pack-first">
          <DetailRow detail={first} />
          <span class="number" style={{ color: data.gold.color }}>
            {data.gold.words}
          </span>
          <span class="dim">{data.gold_words}</span>
        </div>
      )}
      {rest.map((detail, at) => (
        <DetailRow detail={detail} key={at} />
      ))}
    </div>
  );
}
