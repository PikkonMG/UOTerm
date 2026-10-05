import { useState } from 'preact/hooks';
import { isClick, useWindowDrag } from './drag';
import { Note } from './Note';
import type { MapItemData, Point, Send } from './types';
import { WishField } from './WishField';

/** The main mouse button. */
const PRIMARY = 0;

/** A pin on its way to a new place. */
interface Dragged {
  pin: number;
  at: Point;
}

/** A point of the page in the points of the land, which the panel layer may scale. */
function onLand(at: { clientX: number; clientY: number }, land: HTMLElement | null, size: Point): Point {
  const box = land?.getBoundingClientRect();
  const scale = box && box.width > 0 ? box.width / size.x : 1;
  const point = { x: (at.clientX - (box?.left ?? 0)) / scale, y: (at.clientY - (box?.top ?? 0)) / scale };
  return { x: Math.min(Math.max(point.x, 0), size.x), y: Math.min(Math.max(point.y, 0), size.y) };
}

/**
 * A map item: the land between its corners, its pins with the course line
 * from one to the next, and its buttons. While the player plots, a click
 * on the land puts a pin, a pin drags to a new place, and a double click
 * takes it off.
 */
export function MapItem({ data, send }: { data: MapItemData; send: Send }) {
  const [dragged, setDragged] = useState<Dragged | null>(null);
  const [missing, setMissing] = useState(false);
  const follow = useWindowDrag();
  const spots = data.pins.map((pin, at) => (dragged?.pin === at ? dragged.at : pin.at));
  const course = spots.map((spot) => `${spot.x},${spot.y}`).join(' ');
  const drag = (event: PointerEvent, pin: number) => {
    if (!data.plotting || event.button !== PRIMARY) return;
    event.stopPropagation();
    const land = (event.currentTarget as HTMLElement).closest('.map-item-land') as HTMLElement | null;
    const from = { x: event.clientX, y: event.clientY };
    let moved = false;
    const move = (moving: PointerEvent) => {
      if (!moved && isClick(from, { x: moving.clientX, y: moving.clientY })) return;
      moved = true;
      setDragged({ pin, at: onLand(moving, land, data.land) });
    };
    const up = (released: PointerEvent) => {
      setDragged(null);
      if (moved) send({ move_pin: { pin, ...onLand(released, land, data.land) } });
    };
    follow(move, up);
  };
  return (
    <div class="map-item">
      <div class="map-item-paper" style={{ width: `${data.paper.x}px`, height: `${data.paper.y}px`, padding: `${data.edge}px` }}>
        <div
          class="map-item-land"
          title={data.plotting ? data.pin_hint : undefined}
          style={{ width: `${data.land.x}px`, height: `${data.land.y}px` }}
          onClick={(event) => {
            if (data.plotting && !dragged) send({ pin: onLand(event, event.currentTarget as HTMLElement, data.land) });
          }}
        >
          {missing ? <p class="faint small radar-missing">{data.no_files}</p> : <img class="map-item-picture" alt="" src={data.path} onError={() => setMissing(true)} />}
          <svg class="radar-marks" viewBox={`0 0 ${data.land.x} ${data.land.y}`}>
            <polyline points={course} fill="none" stroke="var(--text)" stroke-width={data.course_width} />
          </svg>
          {spots.map((spot, at) => (
            <span
              class="map-pin"
              key={at}
              title={data.plotting ? data.pin_move_hint : undefined}
              style={{
                left: `${spot.x - data.pin_radius}px`,
                top: `${spot.y - data.pin_radius}px`,
                width: `${data.pin_radius * 2}px`,
                height: `${data.pin_radius * 2}px`,
                borderWidth: `${data.pin_ring}px`,
              }}
              onPointerDown={(event) => drag(event, at)}
              onClick={(event) => event.stopPropagation()}
              onDblClick={(event) => {
                event.stopPropagation();
                if (data.plotting) send({ remove_pin: at });
              }}
            >
              <span class="small map-pin-number">{data.pins[at]?.number}</span>
            </span>
          ))}
        </div>
      </div>
      {data.live && (
        <>
          <WishField hint={data.wish_hint} words={data.mark} send={(wish) => send({ wish })} />
          <div class="button-row">
            {data.buttons.map((button, at) => (
              <button type="button" class={`button${button.waiting ? ' waiting' : ''}`} key={button.words} onClick={() => send({ button: at })}>
                {button.words}
              </button>
            ))}
          </div>
        </>
      )}
      <Note note={data.note} />
    </div>
  );
}
