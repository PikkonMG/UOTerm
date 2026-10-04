import { PANEL_ATTRIBUTE } from './drag';
import type { QuestArrowData, Send } from './types';

/**
 * The box round the quest arrow the view draws over the world: a click
 * with the left or the right button tells the shard, while the view lets
 * it. The map takes no click there.
 */
export function QuestArrow({ data, send }: { data: QuestArrowData; send: Send }) {
  const { place } = data;
  return (
    <div
      class="quest-arrow"
      title={data.hint ?? undefined}
      style={{ left: `${place.x}px`, top: `${place.y}px`, width: `${place.w}px`, height: `${place.h}px` }}
      {...{ [PANEL_ATTRIBUTE]: 'quest_arrow' }}
      onClick={() => send({ click: { right: false } })}
      onContextMenu={(event) => {
        event.preventDefault();
        send({ click: { right: true } });
      }}
    />
  );
}
