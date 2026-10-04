import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import type { Hover } from './hover';
import { hoverOn } from './hover';
import { PANEL_ATTRIBUTE } from './drag';
import { Picture, TiledPicture } from './Picture';
import type { GumpPicture, GumpPieceData, HtmlLine, Point, Send, ShardGumpData } from './types';

/** The main mouse button. */
const PRIMARY = 0;
/** The controls of a gump take their own presses: a press on them does not drag it. */
const CONTROLS = 'button, input, .gump-html';

const at = (point: Point, size: Point) => ({ left: `${point.x}px`, top: `${point.y}px`, width: `${size.x}px`, height: `${size.y}px` });

/** One picture of a gump at its place: stretched to its box, or laid side by side. */
function Laid({ picture }: { picture: GumpPicture }) {
  return (
    <div class="gump-piece" style={at(picture.at, picture.size)}>
      {picture.tiled ? <TiledPicture picture={picture.picture} size={picture.size} /> : <Picture picture={picture.picture} />}
    </div>
  );
}

/** Lines of HTML words of the shard, each run of one look as text. */
function HtmlLines({ lines, height }: { lines: HtmlLine[]; height: number }) {
  return (
    <>
      {lines.map((line, row) => (
        <div class="gump-html-line" key={row} style={{ paddingLeft: `${line.left}px`, height: `${height}px` }}>
          {line.spans.map((span, at) => (
            <span
              key={at}
              style={{
                color: span.color,
                fontSize: `${span.size}px`,
                fontWeight: span.bold ? 'bold' : undefined,
                fontStyle: span.italic ? 'italic' : undefined,
                textDecoration: span.underline ? 'underline' : undefined,
              }}
            >
              {span.words}
            </span>
          ))}
        </div>
      ))}
    </>
  );
}

/** A button of gump pictures: its pressed picture shows while the button is down. */
function GumpButton({ piece, send, hover }: { piece: Extract<GumpPieceData, { kind: 'button' }>; send: Send; hover?: Hover }) {
  const [down, setDown] = useState(false);
  const shown = down && piece.pressed ? piece.pressed : piece.normal;
  return (
    <button
      type="button"
      class="gump-piece gump-button"
      data-piece={piece.piece}
      style={{ ...at(piece.at, piece.size), opacity: piece.alpha }}
      onPointerDown={() => setDown(true)}
      onPointerUp={() => setDown(false)}
      onPointerLeave={() => setDown(false)}
      onClick={() => send({ button: piece.piece })}
      {...(piece.tip ? hoverOn(piece.tip, hover) : {})}
    >
      <Picture picture={shown} />
      {piece.item && (
        <span class="gump-piece" style={at({ x: piece.item.at.x - piece.at.x, y: piece.item.at.y - piece.at.y }, piece.item.size)}>
          <Picture picture={piece.item.picture} />
        </span>
      )}
    </button>
  );
}

/** A field of a gump: the view keeps its words and their most. */
function GumpEntry({ piece, send }: { piece: Extract<GumpPieceData, { kind: 'entry' }>; send: Send }) {
  const field = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    if (piece.focus) field.current?.focus();
  }, [piece.focus]);
  return (
    <input
      ref={field}
      class="gump-piece gump-entry"
      style={{ ...at(piece.at, piece.size), color: piece.color, opacity: piece.alpha }}
      value={piece.words}
      maxLength={piece.most ?? undefined}
      onInput={(event) => send({ field: { id: piece.id, words: event.currentTarget.value } })}
    />
  );
}

function Piece({ piece, send, hover }: { piece: GumpPieceData; send: Send; hover?: Hover }) {
  switch (piece.kind) {
    case 'pictures':
      return (
        <div class="gump-layer" style={{ opacity: piece.alpha }} {...(piece.tip ? hoverOn(piece.tip, hover) : {})}>
          {piece.pictures.map((picture, place) => (
            <Laid picture={picture} key={place} />
          ))}
        </div>
      );
    case 'html':
      return (
        <div class="gump-layer" style={{ opacity: piece.alpha }} {...(piece.tip ? hoverOn(piece.tip, hover) : {})}>
          {piece.paper.map((picture, place) => (
            <Laid picture={picture} key={place} />
          ))}
          <div
            class={`gump-piece gump-html${piece.scroll ? ' scrolls' : ''}`}
            style={{ ...at(piece.at, piece.size), padding: `${piece.pad}px`, background: piece.background ?? undefined }}
          >
            <div style={{ width: `${piece.width}px` }}>
              <HtmlLines lines={piece.lines} height={piece.line_height} />
            </div>
          </div>
        </div>
      );
    case 'button':
      return <GumpButton piece={piece} send={send} hover={hover} />;
    case 'choice':
      return (
        <button
          type="button"
          class="gump-piece gump-button"
          data-piece={piece.piece}
          style={{ ...at(piece.at, piece.size), opacity: piece.alpha }}
          onClick={() => send({ tick: piece.piece })}
          {...(piece.tip ? hoverOn(piece.tip, hover) : {})}
        >
          <Picture picture={piece.picture} />
        </button>
      );
    case 'entry':
      return <GumpEntry piece={piece} send={send} />;
    case 'veil':
      return <div class="gump-piece" style={{ ...at(piece.at, piece.size), background: piece.color }} />;
  }
}

/**
 * A gump of the shard in the layout its maker gave it: each piece at its
 * place in gump pixels. A drag of its background moves it; a right click
 * answers it with no button, when the view lets it.
 */
export function ShardGump({ data, scale, send, hover }: { data: ShardGumpData; scale: number; send: Send; hover?: Hover }) {
  const [moved, setMoved] = useState<Point | null>(null);
  const shown = moved ?? data.at;
  const begin = (event: PointerEvent) => {
    const target = event.target as HTMLElement | null;
    if (!data.movable || event.button !== PRIMARY || target?.closest(CONTROLS)) return;
    const from = { x: event.clientX, y: event.clientY };
    let last = data.at;
    const move = (moving: PointerEvent) => {
      last = { x: data.at.x + (moving.clientX - from.x) / scale, y: data.at.y + (moving.clientY - from.y) / scale };
      setMoved(last);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      setMoved(null);
      send({ place: last });
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  return (
    <section
      class="shard-gump"
      style={at(shown, data.size)}
      {...{ [PANEL_ATTRIBUTE]: data.panel }}
      onPointerDown={begin}
      onContextMenu={(event) => {
        event.preventDefault();
        if (data.closable) send({ close: true });
      }}
    >
      {data.pieces.map((piece, place) => (
        <Piece piece={piece} send={send} hover={hover} key={place} />
      ))}
    </section>
  );
}
