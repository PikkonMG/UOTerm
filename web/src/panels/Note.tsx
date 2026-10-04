import type { NoteData } from './types';

/** Words under a panel for a while: what Jev does, or why he could not. */
export function Note({ note }: { note: NoteData | null }) {
  if (!note) return null;
  return <p class={`small note${note.failed ? ' alarm' : ' waiting'}`}>{note.words}</p>;
}
