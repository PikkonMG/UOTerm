/**
 * The measures of words as the page draws them: the name plates in the
 * plate font (Barlow Condensed SemiBold at the plate size of the theme),
 * so the view lays the plates out round them, and the words of the panels
 * in the body font (Barlow at the body size), so the view breaks the
 * lines of a page of a book where the page draws them.
 */

const PLATE_SIZE = '--size-plate';
const BODY_SIZE = '--size-body';
const TITLE_WEIGHT = '--title-weight';
const PLATE_FONT = '{weight} {size} "Barlow Condensed"';
const BODY_FONT = '{size} "Barlow"';
const NOTHING: [number, number] = [0, 0];

type Measure = (text: string) => [number, number];

/** A measure of words in the font `font` at the size token `size`, or none where the page cannot measure. */
function measureIn(font: string, size: string): Measure {
  const context = document.createElement('canvas').getContext('2d');
  if (!context) return () => NOTHING;
  const tokens = getComputedStyle(document.documentElement);
  const token = (name: string) => tokens.getPropertyValue(name).trim();
  context.font = font.replace('{weight}', token(TITLE_WEIGHT)).replace('{size}', token(size));
  return (text) => {
    const metrics = context.measureText(text);
    return [metrics.width, metrics.fontBoundingBoxAscent + metrics.fontBoundingBoxDescent];
  };
}

/** A measure of words in the plate font. */
export function plateMeasure(): Measure {
  return measureIn(PLATE_FONT, PLATE_SIZE);
}

/** A measure of words in the body font of the panels. */
export function bodyMeasure(): Measure {
  return measureIn(BODY_FONT, BODY_SIZE);
}
