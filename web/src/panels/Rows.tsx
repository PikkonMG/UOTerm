import type { Detail } from './types';

/** A dim label and its value in its color, one row of a panel on the map. */
export function DetailRow({ detail }: { detail: Detail }) {
  return (
    <p class="detail">
      <span class="dim">{detail.label}</span>
      <span style={{ color: detail.value.color }}>{detail.value.words}</span>
    </p>
  );
}

const PERCENT = 100;

/**
 * A bar: its track, the part lost a moment ago, and its fill, each a share
 * of the track. With no color the CSS of its place colors the fill.
 */
export function ShareBar({ fill, ghost, color, main = false }: { fill: number; ghost?: number; color?: string; main?: boolean }) {
  const percent = (share: number) => `${share * PERCENT}%`;
  return (
    <span class={`bar-track${main ? ' main' : ''}`}>
      {ghost !== undefined && <span class="bar-ghost" style={{ width: percent(ghost) }} />}
      <span class="bar-fill" style={{ width: percent(fill), background: color }} />
    </span>
  );
}
