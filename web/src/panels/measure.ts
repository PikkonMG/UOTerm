/**
 * The measure of the name plates as the page draws them: the width and the
 * height of words in the plate font (Barlow Condensed SemiBold at the
 * plate size of the theme), so the view lays the plates out round them.
 */

const PLATE_SIZE = '--size-plate';
const PLATE_FONT = '600 {size} "Barlow Condensed"';
const NOTHING: [number, number] = [0, 0];

/** A measure of words in the plate font, or none where the page cannot measure. */
export function plateMeasure(): (text: string) => [number, number] {
  const context = document.createElement('canvas').getContext('2d');
  if (!context) return () => NOTHING;
  const size = getComputedStyle(document.documentElement).getPropertyValue(PLATE_SIZE).trim();
  context.font = PLATE_FONT.replace('{size}', size);
  return (text) => {
    const metrics = context.measureText(text);
    return [metrics.width, metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent];
  };
}
