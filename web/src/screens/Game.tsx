import { useEffect, useRef, useState } from 'preact/hooks';
import { startGame, type GameHandle, type GameProfile, type Shown } from '../game';
import { Panels } from '../panels/Panels';

interface GameProps {
  session: string;
  profile: GameProfile;
  onEnded: () => void;
  /** The game stopped on a fault: its words. */
  onFault: (words: string) => void;
}

/** The world of a running session, over the whole page, with the Modern panels over it. */
export function Game({ session, profile, onEnded, onFault }: GameProps) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const game = useRef<GameHandle | null>(null);
  const [shown, setShown] = useState<Shown | null>(null);

  useEffect(() => {
    if (!canvas.current) return;
    const started = startGame(session, canvas.current, profile, setShown);
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
      {shown && handle && (
        <Panels data={shown.panels} plates={shown.plates} floats={shown.floats} send={handle.panel} input={handle.input} covered={handle.covered} />
      )}
    </>
  );
}
