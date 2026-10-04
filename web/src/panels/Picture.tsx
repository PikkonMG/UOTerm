import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { pixelsOf, whenPixels } from '../net/art';

let mostScale = 1;

/** Sets how many times its size a picture grows at most to fit its cell (`artMostScale()` of the view). */
export function setArtMostScale(scale: number): void {
  mostScale = scale;
}

/**
 * A picture of the art the view asked for, by its key in the page's cache
 * (`pixelsOf`): drawn once its pixels came, fitted to its cell with its
 * proportions kept, and grown no more than the view allows.
 */
export function Picture({ picture, words }: { picture: string | null; words?: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [, cameAgain] = useState(0);
  // Pixels that come later draw the picture, with no new panel data.
  useEffect(() => (picture ? whenPixels(picture, () => cameAgain((times) => times + 1)) : undefined), [picture]);
  useLayoutEffect(() => {
    const target = canvas.current;
    const bitmap = picture ? pixelsOf(picture) : undefined;
    if (!target) return;
    if (!bitmap) {
      // No picture, or one that has not come: the last one goes.
      if (target.dataset.drawn) {
        target.width = 0;
        target.height = 0;
        delete target.dataset.drawn;
      }
      return;
    }
    if (target.dataset.drawn === picture) return;
    target.width = bitmap.width;
    target.height = bitmap.height;
    target.style.maxWidth = `${bitmap.width * mostScale}px`;
    target.style.maxHeight = `${bitmap.height * mostScale}px`;
    target.getContext('2d')?.drawImage(bitmap, 0, 0);
    target.dataset.drawn = picture ?? undefined;
  });
  return <canvas ref={canvas} class="picture" role={words ? 'img' : undefined} aria-label={words} />;
}
