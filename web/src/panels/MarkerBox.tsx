import type { MarkerBoxData, Send } from './types';

const ENTER = 'Enter';

/**
 * The box that adds a marker, or changes one of the player's own file:
 * its place, name, icon and color. Each change of a field goes to the
 * view, which checks the fields when the marker is kept and says why
 * they make none.
 */
export function MarkerBox({ data, send }: { data: MarkerBoxData; send: Send }) {
  const fields = { x: data.x, y: data.y, name: data.name, icon: data.icon, color: data.color };
  const typed = (changed: Partial<typeof fields>) => send({ fields: { ...fields, ...changed } });
  const [xWords, yWords, nameWords, iconWords, colorWords] = data.labels;
  const texts: [string, keyof typeof fields, string][] = [
    [xWords, 'x', ''],
    [yWords, 'y', ''],
    [nameWords, 'name', ''],
    [iconWords, 'icon', data.icon_hint],
  ];
  return (
    <div class="marker-box">
      {texts.map(([words, key, hint]) => (
        <label class="marker-field" key={key}>
          <span class="dim">{words}</span>
          <input
            class="field"
            aria-label={words}
            placeholder={hint}
            value={String(fields[key])}
            onInput={(event) => typed({ [key]: event.currentTarget.value })}
            onKeyDown={(event) => {
              if (event.key === ENTER) send({ submit: true });
            }}
          />
        </label>
      ))}
      <label class="marker-field">
        <span class="dim">{colorWords}</span>
        <select class="field" aria-label={colorWords} value={String(data.color)} onChange={(event) => typed({ color: Number(event.currentTarget.value) })}>
          {data.colors.map((color, at) => (
            <option value={String(at)} key={color}>
              {color}
            </option>
          ))}
        </select>
      </label>
      {data.error && <p class="small alarm">{data.error}</p>}
      <div class="button-row">
        <button type="button" class="button goal" onClick={() => send({ submit: true })}>
          {data.submit}
        </button>
        <button type="button" class="button dim" onClick={() => send({ cancel: true })}>
          {data.cancel}
        </button>
      </div>
    </div>
  );
}
