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
 * proportions kept, and grown no more than the view allows, or than `most`
 * when the panel says (a grid by its Containers page).
 */
export function Picture({ picture, words, most }: { picture: string | null; words?: string; most?: number }) {
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
    const grows = most ?? mostScale;
    target.style.maxWidth = `${bitmap.width * grows}px`;
    target.style.maxHeight = `${bitmap.height * grows}px`;
    if (target.dataset.drawn === picture) return;
    target.width = bitmap.width;
    target.height = bitmap.height;
    target.getContext('2d')?.drawImage(bitmap, 0, 0);
    target.dataset.drawn = picture ?? undefined;
  });
  return <canvas ref={canvas} class="picture" role={words ? 'img' : undefined} aria-label={words} />;
}

/**
 * A picture laid side by side over a box of `size`, as a tiled gump
 * picture or the sides of a gump frame: drawn once its pixels came.
 */
export function TiledPicture({ picture, size }: { picture: string; size: { x: number; y: number } }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [, cameAgain] = useState(0);
  useEffect(() => whenPixels(picture, () => cameAgain((times) => times + 1)), [picture]);
  useLayoutEffect(() => {
    const target = canvas.current;
    const bitmap = pixelsOf(picture);
    if (!target || !bitmap) return;
    target.width = Math.round(size.x);
    target.height = Math.round(size.y);
    const context = target.getContext('2d');
    const pattern = context?.createPattern(bitmap, 'repeat');
    if (!context || !pattern) return;
    context.fillStyle = pattern;
    context.fillRect(0, 0, target.width, target.height);
  });
  return <canvas ref={canvas} class="picture" />;
}
