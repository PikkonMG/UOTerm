import { render } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import './app.css';
import { api, SESSIONS_PATH, TokenNeeded, whenTokenNeeded } from './net/api';
import { Token } from './screens/Token';
import './theme.css';

const APP_ROOT = 'app';

type Screen = { kind: 'checking' } | { kind: 'token' } | { kind: 'ready' } | { kind: 'fault'; words: string };

const TOKEN_SCREEN: Screen = { kind: 'token' };

function App() {
  const [screen, setScreen] = useState<Screen>({ kind: 'checking' });

  /** Asks the API whether the page may call it; a call that wants the token shows the token screen. */
  const check = () =>
    api(SESSIONS_PATH).then(
      () => setScreen({ kind: 'ready' }),
      (error: unknown) => {
        if (!(error instanceof TokenNeeded)) {
          setScreen({ kind: 'fault', words: error instanceof Error ? error.message : String(error) });
        }
      },
    );

  useEffect(() => {
    const stop = whenTokenNeeded(() => setScreen(TOKEN_SCREEN));
    void check();
    return stop;
  }, []);

  switch (screen.kind) {
    case 'checking':
      return null;
    case 'token':
      return <Token onAccepted={() => void check()} />;
    case 'ready':
      return <p class="panel screen">UOTerm answers.</p>;
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
