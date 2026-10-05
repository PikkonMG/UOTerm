import { useEffect, useRef, useState } from 'preact/hooks';
import { api, jsonInit, METHOD_PUT, wordsOf } from '../net/api';
import { AskReplaced, type CharacterChoices, type LoginAsk, type LoginForm, type LoginReply } from '../net/login';
import { Characters } from './Characters';
import { Creation } from './Creation';
import type { CreationMaker, CreationWords } from './creation_model';
import { Login, type BlankLogin, type KeptLogin, type SavedForm, type SavedLogin } from './Login';
import { characterOf, make, nextScreen, pick, shardOf, type CharacterPlace, type LoginRules, type LoginWords } from './login_state';
import { Picking } from './Picking';

const LOGINS_PATH = '/v1/logins';

/** The saved logins as `GET /v1/logins` gives them. */
interface Logins extends BlankLogin {
  logins: SavedLogin[];
}

/** Logs in with a form and answers its questions (the `login` of `net/login`). */
export type StartLogin = (form: LoginForm, onAsk: (ask: LoginAsk, version: string) => Promise<LoginReply>) => Promise<string>;

interface LoginScreensProps {
  words: LoginWords;
  creationWords: CreationWords;
  rules: LoginRules;
  start: StartLogin;
  /** A new character for a login that speaks `version`, by the choices of its account. */
  newCreation(version: string, choices: CharacterChoices): CreationMaker;
  /** The character is in the world: the session, and whose profile it plays with. */
  onReady(session: string, place: CharacterPlace | null): void;
  /** Why the page came back to the login, such as a session that ended. */
  firstNote?: string | null;
}

/** The character list as the login asked it. */
interface Listed {
  names: string[];
  refused: string | null;
  choices: CharacterChoices;
  version: string;
  /** Counts the questions, so a list asked again starts with no delete asked. */
  asked: number;
}

type Stage =
  | { kind: 'form' }
  | { kind: 'connecting' }
  | { kind: 'picking'; names: string[] }
  | { kind: 'characters'; listed: Listed }
  | { kind: 'creating'; listed: Listed; maker: CreationMaker };

const FORM: Stage = { kind: 'form' };
const CONNECTING: Stage = { kind: 'connecting' };

/**
 * The screens before the game: the login form, the shards, the
 * characters, and the making of a new one. The login link asks; each
 * screen answers; the character in the world gives its session.
 */
export function LoginScreens({ words, creationWords, rules, start, newCreation, onReady, firstNote = null }: LoginScreensProps) {
  const [logins, setLogins] = useState<Logins | null>(null);
  const [stage, setStage] = useState<Stage>(FORM);
  const [note, setNote] = useState<string | null>(firstNote);
  /** The form of the last login, shown again when it fails; its password is empty. */
  const [kept, setKept] = useState<KeptLogin | null>(null);
  /** The answer the open question of the login waits for. */
  const answer = useRef<{ resolve(reply: LoginReply): void; reject(error: AskReplaced): void } | null>(null);
  /** The character the replies play, to name the profile of the game. */
  const chosen = useRef<string | null>(null);
  const asked = useRef(0);

  useEffect(() => {
    api<Logins>(LOGINS_PATH).then(setLogins, (error: unknown) => {
      setNote(wordsOf(error));
    });
  }, []);

  const onAsk = (ask: LoginAsk, version: string) =>
    new Promise<LoginReply>((resolve, reject) => {
      answer.current?.reject(new AskReplaced());
      answer.current = { resolve, reject };
      asked.current += 1;
      if (nextScreen(ask) === 'picking') setStage({ kind: 'picking', names: ask.names });
      else if (ask.kind === 'Characters') {
        const { names, refused, choices } = ask;
        setStage({ kind: 'characters', listed: { names, refused, choices, version, asked: asked.current } });
      }
    });

  const reply = (sent: LoginReply, names: string[]) => {
    if (sent.kind === 'Request') chosen.current = characterOf(sent, names);
    answer.current?.resolve(sent);
    answer.current = null;
    setStage(CONNECTING);
  };

  const connect = (form: LoginForm, left: KeptLogin) => {
    setKept(left);
    chosen.current = form.character ?? null;
    setNote(null);
    setStage(CONNECTING);
    start(form, onAsk).then(
      (session) => {
        const character = chosen.current;
        onReady(session, character ? { shard: shardOf(form.host, form.port), character } : null);
      },
      (error: unknown) => {
        answer.current = null;
        setStage(FORM);
        setNote(wordsOf(error));
      },
    );
  };

  const save = async (name: string, form: SavedForm): Promise<SavedLogin[]> => {
    const saved = await api<Logins>(`${LOGINS_PATH}/${encodeURIComponent(name)}`, jsonInit(METHOD_PUT, form));
    setLogins(saved);
    return saved.logins;
  };

  switch (stage.kind) {
    case 'form':
      if (!logins) return note ? <p class="panel screen fault">{note}</p> : null;
      return (
        <Login saved={logins.logins} blank={logins} kept={kept} words={words} rules={rules} note={note} onConnect={connect} onSave={save} />
      );
    case 'connecting':
      return <p class="panel screen waiting">{words.connecting}</p>;
    case 'picking':
      return <Picking title={words.pick_shard} names={stage.names} onPick={(index) => reply(pick(index), [])} />;
    case 'characters': {
      const { listed } = stage;
      return (
        <Characters
          key={listed.asked}
          names={listed.names}
          refused={listed.refused}
          noRoom={rules.noRoomNote(listed.names, listed.choices.list_flags)}
          newList={rules.characterList}
          words={words}
          onReply={(sent) => reply(sent, listed.names)}
          onMake={() => setStage({ kind: 'creating', listed, maker: newCreation(listed.version, listed.choices) })}
        />
      );
    }
    case 'creating': {
      const { listed, maker } = stage;
      return (
        <CreationStage
          maker={maker}
          words={creationWords}
          onLeave={() => setStage({ kind: 'characters', listed })}
          onFinish={() => reply(make(maker.wish(JSON.stringify(listed.names))), listed.names)}
        />
      );
    }
  }
}

interface CreationStageProps {
  maker: CreationMaker;
  words: CreationWords;
  onLeave(): void;
  onFinish(): void;
}

/** The creation screen, which frees its view once it is gone, so nothing calls a freed view. */
function CreationStage({ maker, words, onLeave, onFinish }: CreationStageProps) {
  useEffect(() => () => maker.free(), [maker]);
  return <Creation model={maker} words={words} onLeave={onLeave} onFinish={onFinish} />;
}
