import type { ComponentChildren } from 'preact';
import { useState } from 'preact/hooks';
import { PANEL_ATTRIBUTE } from './drag';
import type { Colored, FrameHints, Place, Send } from './types';

interface FrameProps {
  title: string;
  /** The whole panel, unfolded, in the points of the panel layer. */
  area: Place;
  send: Send;
  /** The name of the panel in its events. */
  panel?: string;
  title_color?: string | null;
  aside?: Colored | null;
  edge?: string | null;
  locked?: boolean;
  folded?: boolean;
  foldable?: boolean;
  closable?: boolean;
  sizable?: boolean;
  /** The UI scale: a move of the mouse is this many times the move of the panel. */
  scale?: number;
  hints?: FrameHints;
  children?: ComponentChildren;
  /** What shows at the foot even while the panel is folded, as the chat line of the journal. */
  foot?: ComponentChildren;
}

/** A drag of the title or of the corner on its way. */
interface Drag {
  sizing: boolean;
  from: { x: number; y: number };
  by: { x: number; y: number };
}

/** The place a drag gives: moved by its title, or sized by its corner. */
function dragged(area: Place, drag: Drag, scale: number): Place {
  const dx = drag.by.x / scale;
  const dy = drag.by.y / scale;
  return drag.sizing ? { ...area, w: area.w + dx, h: area.h + dy } : { ...area, x: area.x + dx, y: area.y + dy };
}

/** A padlock: shut when locked, open when free. */
function Padlock({ locked }: { locked: boolean }) {
  return (
    <svg viewBox="0 0 16 16" class="mark-art" aria-hidden="true">
      <path d={locked ? 'M4.5 8 V5.5 a3.5 3.5 0 0 1 7 0 V8' : 'M4.5 8 V5.5 a3.5 3.5 0 0 1 7 0 V6'} />
      <rect x="3" y="8" width="10" height="7" rx="1.5" />
    </svg>
  );
}

/** A chevron that points down on a folded panel and up on an open one. */
function Chevron({ folded }: { folded: boolean }) {
  return (
    <svg viewBox="0 0 16 16" class="mark-art" aria-hidden="true">
      <path d={folded ? 'M4 6 L8 10 L12 6' : 'M4 10 L8 6 L12 10'} />
    </svg>
  );
}

function Cross() {
  return (
    <svg viewBox="0 0 16 16" class="mark-art" aria-hidden="true">
      <path d="M4 4 L12 12 M12 4 L4 12" />
    </svg>
  );
}

/**
 * The frame of every Modern panel the player moves: the glass, the title
 * that drags it, the corner that sizes it, and the lock, fold and close
 * marks. It reports what the player did; the view keeps the place by its
 * rules and gives it back.
 */
export function Frame(props: FrameProps) {
  const { title, area, send, locked = false, folded = false, scale = 1, hints } = props;
  const [drag, setDrag] = useState<Drag | null>(null);
  const shown = drag ? dragged(area, drag, scale) : area;

  const begin = (event: PointerEvent, sizing: boolean) => {
    if (locked || event.button > 0) return;
    event.preventDefault();
    const from = { x: event.clientX, y: event.clientY };
    let last: Drag = { sizing, from, by: { x: 0, y: 0 } };
    setDrag(last);
    const move = (moved: PointerEvent) => {
      last = { sizing, from, by: { x: moved.clientX - from.x, y: moved.clientY - from.y } };
      setDrag(last);
    };
    const up = (released: PointerEvent) => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      last = { sizing, from, by: { x: released.clientX - from.x, y: released.clientY - from.y } };
      setDrag(null);
      if (last.by.x !== 0 || last.by.y !== 0) send({ place: dragged(area, last, scale) });
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };

  const style = {
    left: `${shown.x}px`,
    top: `${shown.y}px`,
    width: `${shown.w}px`,
    height: folded ? undefined : `${shown.h}px`,
    '--edge': props.edge ?? undefined,
  };
  return (
    <section
      class={`frame panel${title ? ' titled' : ''}${folded ? ' folded' : ''}${locked ? ' locked' : ''}`}
      style={style}
      {...{ [PANEL_ATTRIBUTE]: props.panel ?? title }}
    >
      <header class="frame-title" title={locked ? undefined : hints?.drag} onPointerDown={(event) => begin(event, false)} onDblClick={() => !locked && send({ reset: true })}>
        <span class="frame-words" style={{ color: props.title_color ?? undefined }}>
          {title}
        </span>
        {props.aside && (
          <span class="frame-aside" style={{ color: props.aside.color }}>
            {props.aside.words}
          </span>
        )}
      </header>
      <div class="frame-marks">
        {props.foldable && (
          <button type="button" class="mark" aria-label={hints?.fold} title={hints?.fold} onClick={() => send({ fold: !folded })}>
            <Chevron folded={folded} />
          </button>
        )}
        <button type="button" class={`mark${locked ? ' on' : ''}`} aria-label={hints?.lock} title={hints?.lock} onClick={() => send({ lock: !locked })}>
          <Padlock locked={locked} />
        </button>
        {props.closable && (
          <button type="button" class="mark" aria-label={hints?.close} title={hints?.close} onClick={() => send({ close: true })}>
            <Cross />
          </button>
        )}
      </div>
      {!folded && <div class="frame-body">{props.children}</div>}
      {props.foot && <div class="frame-foot">{props.foot}</div>}
      {props.sizable && !folded && !locked && (
        <div class="frame-grip" role="separator" aria-label={hints?.size} title={hints?.size} onPointerDown={(event) => begin(event, true)} />
      )}
    </section>
  );
}
