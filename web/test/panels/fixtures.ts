import type { FrameData, Place, TipKey } from '../../src/panels/types';

/** A tooltip of plain words. */
export const label = (words: string): TipKey => ({ serial: null, words, footer: '' });

/** The frame of a panel at `area`. */
export function frameAt(panel: string, title: string, area: Place): FrameData {
  return {
    panel,
    title,
    title_color: null,
    aside: null,
    edge: null,
    area,
    locked: false,
    folded: false,
    foldable: true,
    closable: true,
    sizable: true,
  };
}
