import { render } from 'preact';
import { act } from 'preact/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { LoginFailed, type LoginAsk, type LoginForm, type LoginReply } from '../src/net/login';
import { LoginScreens, type StartLogin } from '../src/screens/LoginScreens';
import { CREATE, CREATION_WORDS, namePageView } from './fake_creation';
import { LOGIN_WORDS, rules } from './fake_login';

const VERSION = '7.0.102.3';
const CHOICES = { towns: [], features: 0, list_flags: 0 };
const NAMES = ['Mara', ''];
const LIST: LoginAsk = { kind: 'Characters', names: NAMES, refused: null, choices: CHOICES };
const NO_LOGINS = { host: '127.0.0.1', port: 2593, logins: [] };

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = '';
});

/** A login that asks for the character list, keeps each reply, and gives session `s1`. */
function listThenReady(replies: LoginReply[]): StartLogin {
  return async (_form: LoginForm, onAsk) => {
    replies.push(await onAsk(LIST, VERSION));
    return 's1';
  };
}

async function showForm(
  start: StartLogin,
  onReady = vi.fn(),
  newCreation = vi.fn(),
  firstNote: string | null = null,
  logins: unknown = NO_LOGINS,
): Promise<HTMLElement> {
  vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(JSON.stringify(logins)));
  const root = document.createElement('div');
  document.body.append(root);
  render(
    <LoginScreens
      words={LOGIN_WORDS}
      creationWords={CREATION_WORDS}
      rules={rules()}
      start={start}
      newCreation={newCreation}
      onReady={onReady}
      firstNote={firstNote}
    />,
    root,
  );
  await vi.waitFor(() => expect(root.querySelector('form')).not.toBeNull());
  return root;
}

function button(root: HTMLElement, words: string): HTMLButtonElement {
  const found = [...root.querySelectorAll('button')].find((each) => each.textContent === words);
  if (!found) throw new Error(`no button ${words}`);
  return found;
}

async function connect(root: HTMLElement) {
  await act(async () => button(root, LOGIN_WORDS.connect).click());
}

describe('LoginScreens', () => {
  it('plays_the_character_clicked_with_the_profile_of_its_shard', async () => {
    const replies: LoginReply[] = [];
    const onReady = vi.fn();
    const root = await showForm(listThenReady(replies), onReady);
    await connect(root);
    await act(async () => button(root, 'Mara').click());
    await vi.waitFor(() => expect(onReady).toHaveBeenCalled());
    expect(replies).toEqual([{ kind: 'Request', request: { Play: 0 } }]);
    expect(onReady).toHaveBeenCalledWith('s1', { shard: '127.0.0.1:2593', character: 'Mara' });
  });

  it('makes_a_new_character_with_the_version_of_the_login_and_plays_it', async () => {
    const replies: LoginReply[] = [];
    const onReady = vi.fn();
    const maker = namePageView();
    const newCreation = vi.fn(() => maker);
    const root = await showForm(listThenReady(replies), onReady, newCreation);
    await connect(root);
    await act(async () => button(root, LOGIN_WORDS.make).click());
    expect(newCreation).toHaveBeenCalledWith(VERSION, CHOICES);
    const name = root.querySelector<HTMLInputElement>('input.name-field');
    act(() => {
      if (!name) throw new Error('no name field');
      name.value = 'Lia';
      name.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await act(async () => button(root, CREATE).click());
    await vi.waitFor(() => expect(onReady).toHaveBeenCalled());
    expect(maker.wish).toHaveBeenCalledWith(JSON.stringify(NAMES));
    expect(replies[0]).toMatchObject({ kind: 'Request', request: { Make: { name: 'Lia' } } });
    expect(onReady).toHaveBeenCalledWith('s1', { shard: '127.0.0.1:2593', character: 'Lia' });
    expect(maker.free).toHaveBeenCalledOnce();
  });

  it('brings_the_form_back_with_the_words_of_a_failed_login', async () => {
    const start: StartLogin = () => Promise.reject(new LoginFailed('bad password'));
    const root = await showForm(start);
    await connect(root);
    await vi.waitFor(() => expect(root.textContent).toContain('bad password'));
    expect(root.querySelector('form')).not.toBeNull();
  });

  it('shows_why_the_page_came_back_to_the_login', async () => {
    const root = await showForm(listThenReady([]), vi.fn(), vi.fn(), 'the session ended');
    expect(root.textContent).toContain('the session ended');
  });

  it('keeps_the_form_and_the_saved_login_after_a_failed_login_but_not_the_password', async () => {
    const cedric = { ...NO_LOGINS, logins: [{ name: 'cedric', host: 'play.example.com', port: 2594, account: 'acct2', shard: '', character: '', encryption: 'osi', era: 't2a', version: '5.0.9.1' }] };
    const tries: LoginForm[] = [];
    const start: StartLogin = (form) => {
      tries.push(form);
      return Promise.reject(new LoginFailed('bad password'));
    };
    const root = await showForm(start, vi.fn(), vi.fn(), null, cedric);
    const field = (label: string) => {
      const id = [...root.querySelectorAll('label')].find((each) => each.textContent === label)?.htmlFor;
      const found = id ? root.querySelector<HTMLInputElement>(`#${id}`) : null;
      if (!found) throw new Error(`no field ${label}`);
      return found;
    };
    act(() => root.querySelector<HTMLButtonElement>('.saved-login')?.click());
    act(() => {
      field('Password').value = 'first';
      field('Password').dispatchEvent(new Event('input', { bubbles: true }));
    });
    await connect(root);
    await vi.waitFor(() => expect(root.textContent).toContain('bad password'));
    expect(field('Host').value).toBe('play.example.com');
    expect(field('Account').value).toBe('acct2');
    expect(field('Password').value).toBe('');
    act(() => {
      field('Password').value = 'second';
      field('Password').dispatchEvent(new Event('input', { bubbles: true }));
    });
    await connect(root);
    await vi.waitFor(() => expect(tries.length).toBe(2));
    expect(tries[1]).toMatchObject({ host: 'play.example.com', account: 'acct2', password: 'second', encryption: 'osi', era: 't2a', version: '5.0.9.1' });
  });
});
