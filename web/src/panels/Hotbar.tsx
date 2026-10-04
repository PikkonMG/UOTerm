import { useEffect, useRef } from 'preact/hooks';
import { ZONE_ATTRIBUTE } from './drag';
import { hoverOn, type Hover } from './hover';
import { Picture } from './Picture';
import type { HotbarData, PickerData, Send } from './types';

interface HotbarProps {
  data: HotbarData;
  /** The choices of the empty slot whose picker is open. */
  picker: PickerData | null;
  send: Send;
  hover?: Hover;
}

/**
 * The hotbar: ten slots in a row, each pressed by a click or its key and
 * emptied by a right click. A click on an empty slot opens its picker. Each
 * slot takes an item, a skill or a spell dropped on it.
 */
export function Hotbar({ data, picker, send, hover }: HotbarProps) {
  const bar = useRef<HTMLDivElement>(null);
  const picking = data.picking !== null;

  // A click away from the bar and its picker shuts the picker; one on the bar picks another slot.
  useEffect(() => {
    if (!picking) return;
    const away = (event: PointerEvent) => {
      if (!bar.current?.contains(event.target as Node)) send({ close_picker: true });
    };
    window.addEventListener('pointerdown', away);
    return () => window.removeEventListener('pointerdown', away);
  }, [picking]);

  return (
    <div class="hotbar" ref={bar}>
      {picker && data.picking !== null && (
        <div class="picker panel">
          <h2 class="heading">{picker.title}</h2>
          {picker.no_macros && <p class="faint small">{picker.no_macros}</p>}
          <div class="picker-choices">
            {picker.choices.map((words, at) => (
              <button type="button" class="button" key={at} onClick={() => send({ choose: at })}>
                {words}
              </button>
            ))}
          </div>
        </div>
      )}
      <div class="slots">
        {data.slots.map((slot, at) => (
          <button
            type="button"
            key={at}
            class={`slot${data.picking === at ? ' picking' : ''}`}
            {...{ [ZONE_ATTRIBUTE]: JSON.stringify({ slot: at }) }}
            {...hoverOn(slot.hover, hover)}
            onClick={() => send(slot.words ? { press: at } : { pick: at })}
            onContextMenu={(event) => {
              event.preventDefault();
              if (slot.words) send({ clear: at });
            }}
          >
            <span class="slot-key">{slot.key}</span>
            {slot.picture ? <Picture picture={slot.picture} words={slot.words} /> : slot.words && <span class="slot-words">{slot.words}</span>}
          </button>
        ))}
      </div>
    </div>
  );
}
