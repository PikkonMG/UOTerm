/**
 * The tooltips of the things on the panels: the mouse resting on a thing
 * tells the view, which asks the shard for its words and gives the
 * tooltip back with the panel data.
 */

import type { TipKey } from './types';

/** Tells the view what the mouse rests on, or that it left. */
export type Hover = (tip: TipKey | null) => void;

/** The handlers that tell the view the mouse rests on a thing with `tip`. */
export function hoverOn(tip: TipKey, hover?: Hover) {
  if (!hover) return {};
  return {
    onPointerEnter: () => hover(tip),
    onPointerLeave: () => hover(null),
  };
}
