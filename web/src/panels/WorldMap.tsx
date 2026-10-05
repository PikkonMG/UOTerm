import { useRef, useState } from 'preact/hooks';
import { wheelNotches } from '../input/pointer';
import { isClick, useWindowDrag } from './drag';
import { Note } from './Note';
import { Mark } from './Radar';
import type { Point, Send, WorldMapData } from './types';

/** The main mouse button. */
const PRIMARY = 0;

/** A point of the page in the points of the field, which the panel layer may scale. */
function onField(event: { clientX: number; clientY: number; currentTarget: EventTarget | null }, side: Point): Point {
  const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
  const scale = box.width > 0 ? box.width / side.x : 1;
  return { x: (event.clientX - box.left) / scale, y: (event.clientY - box.top) / scale };
}

/**
 * The world map: the go-to box and the view button, the buttons of the
 * markers, and the field with the land, the tiles of the whole world and
 * the marks the view laid. A click on the field walks or targets there; a
 * drag moves the whole-world view when the World Map page lets it; the
 * wheel zooms.
 */
export function WorldMap({ data, send }: { data: WorldMapData; send: Send }) {
  const [goto, setGoto] = useState('');
  const [missing, setMissing] = useState<Set<string>>(new Set());
  /** Where the main button went down on the field, until it comes up. */
  const pressedAt = useRef<Point | null>(null);
  const follow = useWindowDrag();
  const miss = (path: string) => setMissing((known) => new Set(known).add(path));
  const shownPaths = [...(data.land ? [data.land.path] : []), ...data.tiles.map((tile) => tile.path)];
  const noLand = shownPaths.length > 0 && shownPaths.every((path) => missing.has(path));
  const press = (event: PointerEvent) => {
    if (event.button !== PRIMARY) return;
    let last = { x: event.clientX, y: event.clientY };
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const scale = box.width > 0 ? box.width / data.side.x : 1;
    const from = last;
    pressedAt.current = from;
    const move = (moving: PointerEvent) => {
      const now = { x: moving.clientX, y: moving.clientY };
      if (isClick(from, now)) return;
      send({ drag: { x: (now.x - last.x) / scale, y: (now.y - last.y) / scale } });
      last = now;
    };
    follow(move);
  };
  const release = (event: PointerEvent) => {
    const from = pressedAt.current;
    pressedAt.current = null;
    if (event.button !== PRIMARY || !from || !isClick(from, { x: event.clientX, y: event.clientY })) return;
    send({ click: { ...onField(event, data.side), ctrl: event.ctrlKey || event.metaKey } });
  };
  const [a, b, c, d, e, f] = data.land?.matrix ?? [];
  return (
    <div class="world-map">
      <div class="button-row map-tools">
        {data.place && <span class="small dim map-place">{data.place}</span>}
        <input
          class="field"
          placeholder={data.goto_hint}
          value={goto}
          onInput={(event) => setGoto(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') send({ goto: { words: goto, walk: false } });
          }}
        />
        <button type="button" class="button" onClick={() => send({ goto: { words: goto, walk: false } })}>
          {data.go}
        </button>
        {data.walk && (
          <button type="button" class="button goal" onClick={() => send({ goto: { words: goto, walk: true } })}>
            {data.walk}
          </button>
        )}
        <button type="button" class="button" onClick={() => send({ view: true })}>
          {data.view_words}
        </button>
      </div>
      <div class="button-row map-tools">
        {data.buttons.map((button, at) => (
          <button type="button" class="button" key={button.words} title={button.hint} onClick={() => send({ button: at })}>
            {button.words}
          </button>
        ))}
      </div>
      <div
        class="map-field"
        title={[data.hint, data.pan].filter(Boolean).join('  ')}
        style={{ width: `${data.side.x}px`, height: `${data.side.y}px` }}
        onPointerDown={press}
        onPointerUp={release}
        onPointerMove={(event) => send({ hover: onField(event, data.side) })}
        onPointerLeave={() => send({ hover: null })}
        onWheel={(event) => {
          event.preventDefault();
          send({ wheel: wheelNotches(event) });
        }}
      >
        {data.land && (
          <img
            class="radar-land"
            alt=""
            src={data.land.path}
            width={data.land.side}
            height={data.land.side}
            style={{ transform: `matrix(${a}, ${b}, ${c}, ${d}, ${e}, ${f})` }}
            onError={() => miss(data.land?.path ?? '')}
          />
        )}
        {data.tiles.map((tile) => (
          <img
            class="map-tile"
            alt=""
            key={tile.path}
            src={tile.path}
            style={{ left: `${tile.place.x}px`, top: `${tile.place.y}px`, width: `${tile.place.w}px`, height: `${tile.place.h}px` }}
            onError={() => miss(tile.path)}
          />
        ))}
        {noLand && <p class="faint small radar-missing">{data.no_files}</p>}
        <svg class="radar-marks" viewBox={`0 0 ${data.side.x} ${data.side.y}`}>
          {data.marks.map((mark, at) => (
            <Mark mark={mark} key={at} />
          ))}
        </svg>
        {data.mouse && <span class="small map-mouse">{data.mouse}</span>}
        {data.zone && <span class="small map-zone">{data.zone}</span>}
      </div>
      <Note note={data.note} />
    </div>
  );
}
