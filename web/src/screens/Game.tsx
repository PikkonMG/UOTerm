import { useEffect, useRef } from 'preact/hooks';
import { startGame, type GameProfile } from '../game';

/** The world of a running session, over the whole page. */
export function Game({ session, profile, onEnded }: { session: string; profile: GameProfile; onEnded: () => void }) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!canvas.current) return;
    const game = startGame(session, canvas.current, profile);
    void game.ended.then(onEnded);
    return () => game.stop();
  }, [session]);

  return <canvas ref={canvas} class="world" />;
}
