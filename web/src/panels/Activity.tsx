import { DetailRow } from './Rows';
import type { ActivityData } from './types';

/** What the agent does: the goal as the heading, the details under it. */
export function Activity({ data }: { data: ActivityData }) {
  return (
    <div class="rows">
      <h2 class="title">{data.heading}</h2>
      {data.idle && (
        <p class="detail" style={{ color: data.idle.color }}>
          {data.idle.words}
        </p>
      )}
      {data.details.map((detail, at) => (
        <DetailRow detail={detail} key={at} />
      ))}
    </div>
  );
}
