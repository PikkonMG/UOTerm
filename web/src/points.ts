/**
 * The points of the view. One point is as many CSS pixels as the UI scale
 * of the Video page (`uiScale()` of the view): the world, the words over
 * it and the panels count in points, so the whole window grows by the UI
 * scale as the Rust window does. Every place the page gives the view goes
 * through `toPoints`.
 */

import type { Point } from './input/pointer';

let cssPerPoint = 1;

/** Sets the UI scale of the view now. */
export function setPointScale(scale: number): void {
  cssPerPoint = scale;
}

/** The CSS pixels of one point. */
export function pointScale(): number {
  return cssPerPoint;
}

/** A place in CSS pixels, in points. */
export function toPoints(x: number, y: number): Point {
  return { x: x / cssPerPoint, y: y / cssPerPoint };
}
