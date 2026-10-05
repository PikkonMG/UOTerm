import type { PlacedPlate, PlacedWords } from './types';

const FULL = 255;

/** A color of the view (premultiplied RGBA bytes) as CSS, and its alpha. */
function plain([r, g, b, a]: [number, number, number, number]): { color: string; alpha: number } {
  if (a === 0) return { color: 'transparent', alpha: 0 };
  const channel = (value: number) => Math.min(FULL, Math.round((value * FULL) / a));
  return { color: `rgb(${channel(r)}, ${channel(g)}, ${channel(b)})`, alpha: a / FULL };
}

const box = (area: { min: { x: number; y: number }; max: { x: number; y: number } }) => ({
  left: `${area.min.x}px`,
  top: `${area.min.y}px`,
  width: `${area.max.x - area.min.x}px`,
  height: `${area.max.y - area.min.y}px`,
});

/**
 * The words over the world: the name plates and the words over heads,
 * each where the view laid it out, in the points of the view, which the
 * UI `scale` grows.
 */
export function Plates({ plates, floats, scale }: { plates: PlacedPlate[]; floats: PlacedWords[]; scale: number }) {
  return (
    <div class="world-words" style={{ '--ui-scale': scale }}>
      {plates.map((plate, at) => {
        const name = plain(plate.name_color);
        return (
          <div class="plate" key={at} style={box(plate.area)}>
            <span class="plate-name" style={{ left: `${plate.name_at.x - plate.area.min.x}px`, top: `${plate.name_at.y - plate.area.min.y}px`, color: name.color, opacity: name.alpha }}>
              {plate.name}
            </span>
            {plate.bar && (
              <>
                <span class="plate-bar plate-bar-back" style={{ ...box(plate.bar.back), left: `${plate.bar.back.min.x - plate.area.min.x}px`, top: `${plate.bar.back.min.y - plate.area.min.y}px` }} />
                <span
                  class="plate-bar"
                  style={{ ...box(plate.bar.fill), left: `${plate.bar.fill.min.x - plate.area.min.x}px`, top: `${plate.bar.fill.min.y - plate.area.min.y}px`, background: plain(plate.bar.color).color }}
                />
              </>
            )}
          </div>
        );
      })}
      {floats.map((float, at) => {
        const words = plain(float.color);
        return (
          <span class={`float shadowed${float.number ? ' number' : ''}`} key={at} style={{ left: `${float.x}px`, top: `${float.y}px`, color: words.color, opacity: words.alpha * float.alpha }}>
            {float.words}
          </span>
        );
      })}
    </div>
  );
}
