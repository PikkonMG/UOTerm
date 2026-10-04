/**
 * The drags of the panels: an item, a slot for the hotbar, or a row pulled
 * out of the near list. A press becomes a drag once it moves as far as a
 * click may (the view's `clickDistance()`, as egui counts it); where the
 * button comes up, the zone of the panel under it (`data-zone`, which the
 * panel data names) tells the view where the thing landed, and
 * `Desk::landing` of the view decides what that does.
 */

import type { DropZone, Point } from './types';

/** The attribute of an element a carried thing lands on, as JSON. */
export const ZONE_ATTRIBUTE = 'data-zone';
/** The attribute of the element of a panel, with its name. */
export const PANEL_ATTRIBUTE = 'data-panel';

let dragDistance = 0;
/** A drag of the panels put a thing on the mouse, and its button is still down. */
let carrying = false;

/** Sets how far a press moves before it drags (`clickDistance()` of the view). */
export function setDragDistance(points: number): void {
  dragDistance = points;
}

/**
 * True while a drag the panels started carries a thing: the page drops it
 * where the button comes up, even before the view's data shows it carried.
 */
export function isCarrying(): boolean {
  return carrying;
}

export interface DragSteps {
  /** The drag puts a thing on the mouse, which the page drops. */
  carries?: boolean;
  /** The press moved far enough: the drag starts. */
  started?(): void;
  /** The button came up after the drag started, at this point of the page. */
  dropped?(at: Point): void;
}

/** True when a press at `from` that came up at `to` moved no farther than a click may. */
export function isClick(from: Point, to: Point): boolean {
  return Math.hypot(to.x - from.x, to.y - from.y) <= dragDistance;
}

/** Follows a press at `from` until its button comes up. */
export function followDrag(from: { clientX: number; clientY: number }, steps: DragSteps): void {
  let started = false;
  const move = (event: MouseEvent) => {
    if (started || Math.hypot(event.clientX - from.clientX, event.clientY - from.clientY) <= dragDistance) return;
    started = true;
    carrying = Boolean(steps.carries);
    steps.started?.();
  };
  const up = (event: MouseEvent) => {
    window.removeEventListener('pointermove', move);
    window.removeEventListener('pointerup', up);
    if (!started) return;
    steps.dropped?.({ x: event.clientX, y: event.clientY });
    carrying = false;
  };
  window.addEventListener('pointermove', move);
  window.addEventListener('pointerup', up);
}

/** What lies under a drop: the zone of a panel, and whether it is on a panel at all. */
export function dropTarget(under: Element | null): { zone: DropZone | null; on_panel: boolean } {
  const zone = under?.closest(`[${ZONE_ATTRIBUTE}]`)?.getAttribute(ZONE_ATTRIBUTE);
  return {
    zone: zone ? (JSON.parse(zone) as DropZone) : null,
    on_panel: Boolean(under?.closest(`[${PANEL_ATTRIBUTE}]`)),
  };
}
