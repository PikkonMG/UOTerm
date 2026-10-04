import { DetailRow } from './Rows';
import type { PackData } from './types';

/** The pack: the gold beside the heading, then the weight, the clock and the lists. */
export function Pack({ data }: { data: PackData }) {
  return (
    <div class="rows">
      <div class="pack-heading">
        <h2 class="title">{data.heading}</h2>
        <span class="number" style={{ color: data.gold.color }}>
          {data.gold.words}
        </span>
        <span class="dim">{data.gold_words}</span>
      </div>
      {data.rows.map((detail, at) => (
        <DetailRow detail={detail} key={at} />
      ))}
    </div>
  );
}
