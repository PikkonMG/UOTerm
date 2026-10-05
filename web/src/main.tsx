import { render } from 'preact';
import { useEffect, useState } from 'preact/hooks';
import './app.css';
import { loadView, type GameProfile } from './game';
import { api, SESSIONS_PATH, TokenNeeded, whenTokenNeeded, wordsOf, type Sessions } from './net/api';
import { SESSION_ENDED } from './net/live';
import { login } from './net/login';
import type { CreationWords } from './screens/creation_model';
import { Game } from './screens/Game';
import { LoginScreens } from './screens/LoginScreens';
import { profilePath, readPlace, sessionSearch, soundFontPath, type CharacterPlace, type LoginRules, type LoginWords } from './screens/login_state';
import { Token } from './screens/Token';
import './theme.css';
import { CharacterList, creationWords, CreationView, loginFault, loginWords, noRoomNote, savedDetail, saveName } from './wasm/uoterm_web.js';

const APP_ROOT = 'app';
const NO_SUCH_SESSION = 'That session is not running.';
/** Before the words of a fault that stopped the game; a reload opens the session again. */
const GAME_FAILED = 'The game stopped (reload the page to go on):';

/** The words and the rules of the screens before the game, from the view. */
interface LoginParts {
  words: LoginWords;
  creationWords: CreationWords;
  rules: LoginRules;
}

type Screen =
  | { kind: 'checking' }
  | ({ kind: 'login'; note: string | null } & LoginParts)
  | { kind: 'game'; session: string; profile: GameProfile }
  | { kind: 'fault'; words: string };

const CHECKING: Screen = { kind: 'checking' };

/** The rules of the login screens, as the view runs them. */
const VIEW_RULES: LoginRules = {
  fault: (host, port, account, password) => loginFault(host, port, account, password) ?? null,
  saveName,
  detail: savedDetail,
  noRoomNote: (names, listFlags) => noRoomNote(JSON.stringify(names), listFlags) ?? null,
  characterList: () => new CharacterList(),
};

/** The login screens, with the words of why the page shows them, if any. */
const loginScreen = (note: string | null): Screen => ({
  kind: 'login',
  note,
  words: loginWords(),
  creationWords: creationWords(),
  rules: VIEW_RULES,
});

/** The game of `session`, with the profile of its character (the default profile with none). */
async function gameOf(session: string, place: CharacterPlace | null): Promise<Screen> {
  const path = profilePath(place);
  const value = await api<unknown>(path);
  return { kind: 'game', session, profile: { path, value, soundFont: soundFontPath(place) } };
}

const faultOf = (error: unknown): Screen => ({ kind: 'fault', words: wordsOf(error) });

/**
 * Asks the API whether the page may call it, then opens the session the
 * address names when it runs, or the login. A call that wants the token
 * brings the token screen up (through `whenTokenNeeded`); its acceptance
 * asks again.
 */
async function firstScreen(): Promise<Screen | null> {
  try {
    const { sessions } = await api<Sessions>(SESSIONS_PATH);
    await loadView();
    const place = readPlace(new URLSearchParams(location.search));
    if (place === null) return loginScreen(null);
    if (!sessions.includes(place.session)) return { kind: 'fault', words: NO_SUCH_SESSION };
    return await gameOf(place.session, place.character);
  } catch (error) {
    if (error instanceof TokenNeeded) return null;
    return faultOf(error);
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
    if (screen.kind === 'checking' || screen.kind === 'fault') void check();
  };

  // The address names the session and its character, so a reload opens the same game.
  const ready = (session: string, place: CharacterPlace | null) => {
    history.replaceState(null, '', sessionSearch(session, place));
    gameOf(session, place).then(setScreen, (error: unknown) => setScreen(faultOf(error)));
  };

  // A session that ended leaves the game: the login again, with the words why; a reload no longer names it.
  const ended = () => {
    history.replaceState(null, '', location.pathname);
    setScreen(loginScreen(SESSION_ENDED));
  };

  return (
    <>
      <Body
        screen={screen}
        onReady={ready}
        onEnded={ended}
        onFault={(words) => setScreen({ kind: 'fault', words: `${GAME_FAILED} ${words}` })}
      />
      {tokenWanted && <Token onAccepted={accepted} />}
    </>
  );
}

interface BodyProps {
  screen: Screen;
  onReady(session: string, place: CharacterPlace | null): void;
  onEnded(): void;
  onFault(words: string): void;
}

function Body({ screen, onReady, onEnded, onFault }: BodyProps) {
  switch (screen.kind) {
    case 'checking':
      return null;
    case 'login':
      return (
        <LoginScreens
          words={screen.words}
          creationWords={screen.creationWords}
          rules={screen.rules}
          start={login}
          newCreation={(version, choices) => new CreationView(version, JSON.stringify(choices))}
          onReady={onReady}
          firstNote={screen.note}
        />
      );
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
