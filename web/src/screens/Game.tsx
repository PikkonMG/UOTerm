import { useEffect, useRef, useState } from 'preact/hooks';
import type { WorldWords } from '../frame_loop';
import { startGame, type GameHandle, type GameProfile } from '../game';
import { Panels } from '../panels/Panels';
import { Plates } from '../panels/Plates';
import type { PanelData } from '../panels/types';

interface GameProps {
  session: string;
  profile: GameProfile;
  onEnded: () => void;
  /** The game stopped on a fault: its words. */
  onFault: (words: string) => void;
}

const NO_WORDS: WorldWords = { plates: [], floats: [] };

/**
 * The world of a running session, over the whole page, with the words over
 * it and the Modern panels over them. Each draws again only when the view
 * gave it something new.
 */
export function Game({ session, profile, onEnded, onFault }: GameProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const overlay = useRef<HTMLDivElement>(null);
  const game = useRef<GameHandle | null>(null);
  const [panels, setPanels] = useState<PanelData | null>(null);
  const [words, setWords] = useState<WorldWords>(NO_WORDS);

  useEffect(() => {
    if (!canvas.current || !overlay.current) return;
    const started = startGame(session, canvas.current, overlay.current, profile, { panels: setPanels, words: setWords });
    game.current = started;
    started.ended.then(onEnded, (error: unknown) => onFault(error instanceof Error ? error.message : String(error)));
    return () => {
      game.current = null;
      started.stop();
    };
  }, [session]);

  const handle = game.current;
  return (
    <>
      <canvas ref={canvas} class="world" />
      <div ref={overlay}>
        <Plates plates={words.plates} floats={words.floats} scale={panels?.look.ui_scale ?? 1} />
        {panels && handle && <Panels data={panels} send={handle.panel} input={handle.input} covered={handle.covered} />}
      </div>
    </>
  );
}
