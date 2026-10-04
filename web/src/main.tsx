import { render } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import './app.css';
import { api, TokenNeeded } from './net/api';
import { Token } from './screens/Token';
import './theme.css';

/** A call every screen may make: it fails with `TokenNeeded` when the API wants its token. */
const SESSIONS_PATH = '/v1/sessions';
const APP_ROOT = 'app';

type Screen = { kind: 'checking' } | { kind: 'token' } | { kind: 'ready' } | { kind: 'fault'; words: string };

function App() {
  const [screen, setScreen] = useState<Screen>({ kind: 'checking' });

  const check = () =>
    api(SESSIONS_PATH).then(
      () => setScreen({ kind: 'ready' }),
      (error: unknown) =>
        setScreen(
          error instanceof TokenNeeded
            ? { kind: 'token' }
            : { kind: 'fault', words: error instanceof Error ? error.message : String(error) },
        ),
    );

  useEffect(() => {
    void check();
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
