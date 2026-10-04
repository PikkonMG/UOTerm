import { render } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import './app.css';
import { DEFAULT_PROFILE_PATH, loadView, type GameProfile } from './game';
import { api, SESSIONS_PATH, TokenNeeded, whenTokenNeeded } from './net/api';
import { SESSION_ENDED } from './net/live';
import { Game } from './screens/Game';
import { Token } from './screens/Token';
import './theme.css';

const APP_ROOT = 'app';
/** The query that opens a running session, so a reload of the page needs no new login: `?session=<id>`. */
const SESSION_QUERY = 'session';
const NO_SUCH_SESSION = 'That session is not running.';
/** Before the words of a fault that stopped the game; a reload opens the session again. */
const GAME_FAILED = 'The game stopped (reload the page to go on):';

type Screen =
  | { kind: 'checking' }
  | { kind: 'ready' }
  | { kind: 'game'; session: string; profile: GameProfile }
  | { kind: 'fault'; words: string };

const CHECKING: Screen = { kind: 'checking' };

/** The running sessions, as `GET /v1/sessions` gives them. */
interface Sessions {
  sessions: string[];
}

/**
 * Asks the API whether the page may call it, and opens the session the
 * query names when it runs. A call that wants the token brings the token
 * screen up (through `whenTokenNeeded`); its acceptance asks again.
 */
async function firstScreen(): Promise<Screen | null> {
  try {
    const { sessions } = await api<Sessions>(SESSIONS_PATH);
    const session = new URLSearchParams(location.search).get(SESSION_QUERY);
    if (session === null) return { kind: 'ready' };
    if (!sessions.includes(session)) return { kind: 'fault', words: NO_SUCH_SESSION };
    const value = await api<unknown>(DEFAULT_PROFILE_PATH);
    await loadView();
    return { kind: 'game', session, profile: { path: DEFAULT_PROFILE_PATH, value } };
  } catch (error) {
    if (error instanceof TokenNeeded) return null;
    return { kind: 'fault', words: error instanceof Error ? error.message : String(error) };
  }
}

function App() {
  const [screen, setScreen] = useState<Screen>(CHECKING);
  const [tokenWanted, setTokenWanted] = useState(false);

  const check = () =>
    firstScreen().then((next) => {
      if (next) setScreen(next);
    });

  useEffect(() => {
    const stop = whenTokenNeeded(() => setTokenWanted(true));
    void check();
    return stop;
  }, []);

  // A game keeps running under the token screen: what waited for the token goes on once it is given.
  const accepted = () => {
    setTokenWanted(false);
    if (screen.kind !== 'game') void check();
  };

  return (
    <>
      <Body
        screen={screen}
        onEnded={() => setScreen({ kind: 'fault', words: SESSION_ENDED })}
        onFault={(words) => setScreen({ kind: 'fault', words: `${GAME_FAILED} ${words}` })}
      />
      {tokenWanted && <Token onAccepted={accepted} />}
    </>
  );
}

function Body({ screen, onEnded, onFault }: { screen: Screen; onEnded: () => void; onFault: (words: string) => void }) {
  switch (screen.kind) {
    case 'checking':
      return null;
    case 'ready':
      return <p class="panel screen">UOTerm answers.</p>;
    case 'game':
      return <Game session={screen.session} profile={screen.profile} onEnded={onEnded} onFault={onFault} />;
    case 'fault':
      return (
        <p class="panel screen fault" role="alert">
          {screen.words}
        </p>
      );
  }
}

const root = document.getElementById(APP_ROOT);
if (root) render(<App />, root);
