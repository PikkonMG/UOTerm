import { useEffect, useRef } from 'preact/hooks';
import { startGame, type GameProfile } from '../game';

interface GameProps {
  session: string;
  profile: GameProfile;
  onEnded: () => void;
  /** The game stopped on a fault: its words. */
  onFault: (words: string) => void;
}

/** The world of a running session, over the whole page. */
export function Game({ session, profile, onEnded, onFault }: GameProps) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvas.current) return;
    const game = startGame(session, canvas.current, profile);
    game.ended.then(onEnded, (error: unknown) => onFault(error instanceof Error ? error.message : String(error)));
    return () => game.stop();
  }, [session]);

  return <canvas ref={canvas} class="world" />;
}
