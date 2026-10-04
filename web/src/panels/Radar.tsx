import { useState } from 'preact/hooks';
import { wheelNotches } from '../input/pointer';
import type { MarkData, RadarData, Send } from './types';

/** One mark of a map, as an SVG shape where the view laid it. */
export function Mark({ mark }: { mark: MarkData }) {
  switch (mark.kind) {
    case 'line':
      return <line x1={mark.from.x} y1={mark.from.y} x2={mark.to.x} y2={mark.to.y} stroke={mark.color} stroke-width={mark.width} />;
    case 'outline':
      return <polygon points={mark.points.map((point) => `${point.x},${point.y}`).join(' ')} fill="none" stroke={mark.color} stroke-width={mark.width} />;
    case 'words':
      return (
        <text x={mark.at.x} y={mark.at.y} fill={mark.color} class="mark-words" dominant-baseline={mark.top ? 'hanging' : 'middle'}>
          {mark.words}
        </text>
      );
    case 'dot':
      return <circle cx={mark.at.x} cy={mark.at.y} r={mark.radius} fill={mark.color} />;
    case 'square':
      return <rect x={mark.at.x - mark.side / 2} y={mark.at.y - mark.side / 2} width={mark.side} height={mark.side} fill={mark.color} />;
    case 'ring':
      return <circle cx={mark.at.x} cy={mark.at.y} r={mark.radius} fill="none" stroke={mark.color} stroke-width={mark.width} />;
    case 'health_bar':
      return (
        <g>
          <rect x={mark.track.x} y={mark.track.y} width={mark.track.w} height={mark.track.h} fill={mark.back} />
          <rect x={mark.track.x} y={mark.track.y} width={mark.track.w * mark.share} height={mark.track.h} fill={mark.color} />
        </g>
      );
  }
}

/**
 * The radar: the picture of the land round the character, laid on the
 * field by the matrix of the view, turned as the play field is, with the
 * marks over it. The wheel zooms; a double click makes it large or small.
 */
export function Radar({ data, send }: { data: RadarData; send: Send }) {
  const [missing, setMissing] = useState<string | null>(null);
  const [a, b, c, d, e, f] = data.land.matrix;
  return (
    <div
      class="radar-field"
      title={data.hint}
      style={{ width: `${data.side.x}px`, height: `${data.side.y}px` }}
      onWheel={(event) => {
        event.preventDefault();
        send({ wheel: wheelNotches(event) });
      }}
      onDblClick={() => send({ double: true })}
    >
      {missing === data.land.path ? (
        <p class="faint small radar-missing">{data.no_files}</p>
      ) : (
        <img
          class="radar-land"
          alt=""
          src={data.land.path}
          width={data.land.side}
          height={data.land.side}
          style={{ transform: `matrix(${a}, ${b}, ${c}, ${d}, ${e}, ${f})` }}
          onError={() => setMissing(data.land.path)}
        />
      )}
      <svg class="radar-marks" viewBox={`0 0 ${data.side.x} ${data.side.y}`}>
        {data.marks.map((mark, at) => (
          <Mark mark={mark} key={at} />
        ))}
      </svg>
    </div>
  );
}
