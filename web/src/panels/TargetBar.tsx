import { useState } from 'preact/hooks';
import { ZONE_ATTRIBUTE } from './drag';
import { ShareBar } from './Rows';
import type { HealthBarData, Send } from './types';

/**
 * A health bar of its own, as the target bar and the bars pulled out of
 * the near list are: hits, mana and stamina, Heal and Cure for a party
 * member, and a menu on a right click to target, rename a pet or close.
 */
export function TargetBar({ data, send }: { data: HealthBarData; send: Send }) {
  const [menu, setMenu] = useState(false);
  const [name, setName] = useState('');
  return (
    <div class="health" {...{ [ZONE_ATTRIBUTE]: JSON.stringify(data.zone) }}>
      <div
        class="health-lines"
        onClick={() => send({ click: true })}
        onDblClick={() => send({ double: true })}
        onContextMenu={(event) => {
          event.preventDefault();
          setMenu(!menu);
        }}
      >
        {data.lines.map((line, at) => (
          <ShareBar key={at} fill={line.share} color={line.color} />
        ))}
      </div>
      {data.party && (
        <div class="button-row">
          <button type="button" class="button goal-words" onClick={() => send({ heal: true })}>
            {data.party[0]}
          </button>
          <button type="button" class="button goal-words" onClick={() => send({ cure: true })}>
            {data.party[1]}
          </button>
        </div>
      )}
      {menu && (
        <div class="popup menu">
          {data.target && (
            <button type="button" class="button row" onClick={() => send({ target: true })}>
              {data.target}
            </button>
          )}
          {data.rename && (
            <div class="button-row">
              <input class="field" value={name} onInput={(event) => setName(event.currentTarget.value)} />
              <button
                type="button"
                class="button"
                onClick={() => {
                  send({ rename: name });
                  setMenu(false);
                }}
              >
                {data.rename}
              </button>
            </div>
          )}
          <button type="button" class="button row" onClick={() => send({ close: true })}>
            {data.close}
          </button>
        </div>
      )}
    </div>
  );
}
