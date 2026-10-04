import type { Send } from './types';

/**
 * The key chord or the controller buttons of a macro: a press waits for
 * the next key or buttons, which the view takes from its own input (the
 * keys by their egui names, the buttons from the controller), as the
 * Options of the Rust window do. Escape lets it go.
 */
export function KeyCapture({ words, at, pad, send }: { words: string; at: number; pad: boolean; send: Send }) {
  return (
    <button type="button" class="button small key-capture" onClick={() => send({ capture: { at, pad } })}>
      {words}
    </button>
  );
}
