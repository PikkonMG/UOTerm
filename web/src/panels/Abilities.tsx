import { followDrag } from './drag';
import { hoverOn, type Hover } from './hover';
import { Picture } from './Picture';
import type { AbilitiesData, Colored, RacialData, Send } from './types';

const PRIMARY = 0;

/** An icon that uses its ability on a click and drags it toward the hotbar. */
function AbilityIcon({ picture, side, at, send, live }: { picture: string; side: number; at: number; send: Send; live: boolean }) {
  return (
    <span
      class="cell-art ability-icon"
      style={{ width: `${side}px`, height: `${side}px` }}
      onClick={() => live && send({ use: at })}
      onPointerDown={(event) => {
        if (live && event.button === PRIMARY) followDrag(event, { carries: true, started: () => send({ drag: at }) });
      }}
    >
      <Picture picture={picture} />
    </span>
  );
}

/** The two buttons beside an ability: its use, and Pin. */
function Pair({ buttons, at, send }: { buttons: Colored[]; at: number; send: Send }) {
  if (buttons.length === 0) return null;
  const [use, pin] = buttons;
  return (
    <div class="button-row">
      <button type="button" class="button small" style={{ color: use.color }} onClick={() => send({ use: at })}>
        {use.words}
      </button>
      <button type="button" class="button small" style={{ color: pin.color }} onClick={() => send({ pin: at })}>
        {pin.words}
      </button>
    </div>
  );
}

/**
 * The combat panel: the primary and the secondary ability of the weapon in
 * hand (a click arms one or lets it go; Pin and a drag put it on the
 * hotbar), then every weapon ability with the weapons that have it.
 */
export function Abilities({ data, send, hover }: { data: AbilitiesData; send: Send; hover?: Hover }) {
  return (
    <div class="abilities">
      {data.slots.map((slot, at) => (
        <div class="ability-card" key={at} {...hoverOn(slot.hover, hover)}>
          <AbilityIcon picture={slot.picture} side={data.icon} at={at} send={send} live={slot.buttons.length > 0} />
          <div class="ability-words">
            <p class="detail">
              <span>{slot.words}</span>
              {slot.armed && (
                <span class="small" style={{ color: slot.armed.color }}>
                  {slot.armed.words}
                </span>
              )}
            </p>
            <Pair buttons={slot.buttons} at={at} send={send} />
          </div>
        </div>
      ))}
      <p class="title small goal-words">{data.all_title}</p>
      <div class="ability-rows">
        {data.rows.map((row, at) => (
          <div class="ability-row" key={at} {...hoverOn(row.hover, hover)}>
            <span class="cell-art">
              <Picture picture={row.picture} />
            </span>
            <span class="small" style={{ color: row.name.color }}>
              {row.name.words}
            </span>
            {row.slot && <span class="small goal-words">{row.slot}</span>}
          </div>
        ))}
      </div>
    </div>
  );
}

/** The racial panel: each ability of the race, Passive, or Use and Pin for the one that acts. */
export function Racial({ data, send, hover }: { data: RacialData; send: Send; hover?: Hover }) {
  return (
    <div class="abilities">
      {data.none && <p class="faint small">{data.none}</p>}
      {data.rows.map((row, at) => (
        <div class="ability-card" key={at} {...(row.hover ? hoverOn(row.hover, hover) : {})}>
          <AbilityIcon picture={row.picture} side={data.icon} at={at} send={send} live={row.buttons.length > 0} />
          <div class="ability-words">
            <p>{row.name}</p>
            {row.passive && <p class="faint small">{row.passive}</p>}
            <Pair buttons={row.buttons} at={at} send={send} />
          </div>
        </div>
      ))}
    </div>
  );
}
