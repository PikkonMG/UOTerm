import { useLayoutEffect, useRef } from 'preact/hooks';
import { pixelsOf } from '../net/art';

/**
 * A picture of the art the view asked for, by its key in the page's cache
 * (`pixelsOf`): drawn once its pixels came, at their own size, which the
 * CSS shrinks to fit its cell.
 */
export function Picture({ picture, words }: { picture: string | null; words?: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  useLayoutEffect(() => {
    const target = canvas.current;
    const bitmap = picture ? pixelsOf(picture) : undefined;
    if (!target || !bitmap || target.dataset.drawn === picture) return;
    target.width = bitmap.width;
    target.height = bitmap.height;
    target.getContext('2d')?.drawImage(bitmap, 0, 0);
    target.dataset.drawn = picture ?? undefined;
  });
  return <canvas ref={canvas} class="picture" role={words ? 'img' : undefined} aria-label={words} />;
}
