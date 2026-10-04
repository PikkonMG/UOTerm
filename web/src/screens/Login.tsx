import { useState } from 'preact/hooks';
import type { LoginForm } from '../net/login';
import type { LoginRules, LoginWords } from './login_state';

type Encryption = NonNullable<LoginForm['encryption']>;

/** One saved login, as `GET /v1/logins` lists it. It never holds a password. */
export interface SavedLogin {
  name: string;
  host: string;
  port: number;
  account: string;
  shard: string;
  character: string;
  encryption: Encryption;
  era: string | null;
  version: string | null;
}

/** Where a login with no saved login goes: the server of the config file. */
export interface BlankLogin {
  host: string;
  port: number;
}

/** A form as `PUT /v1/logins/{name}` saves it: no password. */
export interface SavedForm {
  host: string;
  port: number;
  account: string;
  shard: string;
  character: string;
  encryption: Encryption;
}

interface LoginProps {
  saved: SavedLogin[];
  blank: BlankLogin;
  words: LoginWords;
  rules: LoginRules;
  /** Words of the last login that failed. */
  note: string | null;
  onConnect(form: LoginForm): void;
  /** Saves the form under a name; gives the saved logins after, which come back as `saved`. */
  onSave(name: string, form: SavedForm): Promise<SavedLogin[]>;
}

/** The text of the fields, in the order of `LoginWords.labels`. */
type Fields = [host: string, port: string, account: string, password: string, shard: string, character: string];

const HOST = 0;
const PORT = 1;
const ACCOUNT = 2;
const PASSWORD = 3;
const SHARD = 4;
const CHARACTER = 5;
const FIELD_ID = 'login-field-';
const SAVE_NAME_ID = 'login-save-name';
/** The browser offers its own saved password for the password field, and keeps nothing else. */
const AUTOCOMPLETE: Record<number, string> = { [ACCOUNT]: 'username', [PASSWORD]: 'current-password' };
const NO_AUTOCOMPLETE = 'off';

const blankFields = (blank: BlankLogin): Fields => [blank.host, String(blank.port), '', '', '', ''];
const savedFields = (saved: SavedLogin): Fields => [saved.host, String(saved.port), saved.account, '', saved.shard, saved.character];
const named = (words: string): string | null => words.trim() || null;

/**
 * The login form, with the saved logins at its side: the fields of the
 * window's login screen in its order. The password is typed each time,
 * sent once with the login, and kept nowhere; a saved login never has it.
 */
export function Login({ saved, blank, words, rules, note, onConnect, onSave }: LoginProps) {
  const [fields, setFields] = useState<Fields>(() => blankFields(blank));
  const [encryption, setEncryption] = useState<Encryption>(words.encryptions[0][0] as Encryption);
  const [picked, setPicked] = useState<SavedLogin | null>(null);
  const [saveName, setSaveName] = useState<string | null>(null);
  const [fault, setFault] = useState<string | null>(null);
  const [told, setTold] = useState<string | null>(null);

  const setField = (at: number, value: string) => setFields((now) => now.map((old, place) => (place === at ? value : old)) as Fields);

  /** The field of `at`, but its type: the password field hides what it holds. */
  const fieldProps = (at: number) => ({
    id: `${FIELD_ID}${at}`,
    class: 'field',
    autocomplete: AUTOCOMPLETE[at] ?? NO_AUTOCOMPLETE,
    value: fields[at],
    onInput: (event: Event) => setField(at, (event.currentTarget as HTMLInputElement).value),
  });

  const pickSaved = (login: SavedLogin) => {
    setFields(savedFields(login));
    setEncryption(login.encryption);
    setPicked(login);
    setSaveName(null);
    setFault(null);
    setTold(null);
    document.getElementById(`${FIELD_ID}${PASSWORD}`)?.focus();
  };

  const connect = (event: SubmitEvent) => {
    event.preventDefault();
    const refusal = rules.fault(fields[HOST], fields[PORT], fields[ACCOUNT], fields[PASSWORD]);
    setFault(refusal);
    if (refusal !== null) return;
    onConnect({
      host: fields[HOST].trim(),
      port: Number(fields[PORT].trim()),
      account: fields[ACCOUNT].trim(),
      password: fields[PASSWORD],
      shard: named(fields[SHARD]),
      character: named(fields[CHARACTER]),
      encryption,
      era: picked?.era ?? null,
      version: picked?.version ?? null,
    });
  };

  const save = async () => {
    const name = saveName?.trim() ?? '';
    const refusal = rules.fault(fields[HOST], fields[PORT], fields[ACCOUNT], null);
    setFault(refusal);
    if (refusal !== null) return;
    const form: SavedForm = {
      host: fields[HOST].trim(),
      port: Number(fields[PORT].trim()),
      account: fields[ACCOUNT].trim(),
      shard: fields[SHARD].trim(),
      character: fields[CHARACTER].trim(),
      encryption,
    };
    try {
      const after = await onSave(name, form);
      setPicked(after.find((login) => login.name === name) ?? picked);
      setSaveName(null);
      setTold(`${words.saved_as} ${name}.`);
    } catch (error) {
      setFault(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <form class="panel screen login" onSubmit={connect}>
      <h1 class="title">{words.title}</h1>
      <div class="login-body">
        <section class="saved-logins">
          <h2 class="label">{words.saved}</h2>
          {saved.length === 0 && <p class="faint">{words.no_saved}</p>}
          {saved.map((login) => (
            <button
              type="button"
              key={login.name}
              class={`saved-login${picked?.name === login.name ? ' chosen' : ''}`}
              onClick={() => pickSaved(login)}
            >
              <span class="saved-name">{login.name}</span>
              <span class="saved-detail">{rules.detail(login.account, login.host, login.port)}</span>
            </button>
          ))}
        </section>
        <section class="login-fields">
          {words.labels.map((label, at) => (
            <div class="field-row" key={label}>
              <label class="label" for={`${FIELD_ID}${at}`}>
                {label}
              </label>
              {at === PASSWORD ? <input type="password" {...fieldProps(at)} /> : <input type="text" {...fieldProps(at)} />}
            </div>
          ))}
          <div class="field-row">
            <span class="label">{words.encryption}</span>
            <div class="choices">
              {words.encryptions.map(([mode, label]) => (
                <button
                  type="button"
                  key={mode}
                  class={`button${encryption === mode ? ' chosen' : ''}`}
                  aria-pressed={encryption === mode}
                  onClick={() => setEncryption(mode as Encryption)}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>
          {saveName !== null && (
            <div class="field-row">
              <label class="label" for={SAVE_NAME_ID}>
                {words.save_as}
              </label>
              <input id={SAVE_NAME_ID} class="field" value={saveName} onInput={(event) => setSaveName(event.currentTarget.value)} />
              <button type="button" class="button goal" onClick={() => void save()}>
                {words.save}
              </button>
              <button type="button" class="button" onClick={() => setSaveName(null)}>
                {words.cancel}
              </button>
            </div>
          )}
          <div class="button-row">
            <button type="submit" class="button goal">
              {words.connect}
            </button>
            <button type="button" class="button" onClick={() => setSaveName(picked?.name ?? rules.saveName(fields[ACCOUNT], fields[HOST]))}>
              {words.save_login}
            </button>
            <span class="faint">{words.not_saved}</span>
          </div>
        </section>
      </div>
      {(fault ?? note) && (
        <p class="fault" role="alert">
          {fault ?? note}
        </p>
      )}
      {told && !fault && <p class="told">{told}</p>}
    </form>
  );
}
