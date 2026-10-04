import { render } from 'preact';
import { act } from 'preact/test-utils';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LoginForm } from '../src/net/login';
import { Characters } from '../src/screens/Characters';
import { Login, type KeptLogin, type SavedLogin } from '../src/screens/Login';
import { leave, play, remove } from '../src/screens/login_state';
import { Picking } from '../src/screens/Picking';
import { LOGIN_WORDS as WORDS, rules } from './fake_login';

const CEDRIC: SavedLogin = {
  name: 'cedric',
  host: 'play.example.com',
  port: 2594,
  account: 'acct2',
  shard: 'Britannia',
  character: 'Cedric',
  encryption: 'osi',
  era: null,
  version: null,
};

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = '';
});

function mount(node: preact.ComponentChild): HTMLElement {
  const root = document.createElement('div');
  document.body.append(root);
  act(() => render(node, root));
  return root;
}

function button(root: HTMLElement, words: string): HTMLButtonElement {
  const found = [...root.querySelectorAll('button')].filter((each) => each.textContent === words);
  if (found.length === 0) throw new Error(`no button ${words}`);
  return found[0];
}

function fieldOf(root: HTMLElement, label: string): HTMLInputElement {
  const id = [...root.querySelectorAll('label')].find((each) => each.textContent === label)?.htmlFor;
  const field = id ? root.querySelector<HTMLInputElement>(`#${id}`) : null;
  if (!field) throw new Error(`no field ${label}`);
  return field;
}

function type(field: HTMLInputElement, words: string) {
  act(() => {
    field.value = words;
    field.dispatchEvent(new Event('input', { bubbles: true }));
  });
}

describe('Login', () => {
  it('has_the_fields_of_the_window_in_its_order_and_hides_the_password', () => {
    const root = mount(
      <Login kept={null} saved={[]} blank={{ host: '127.0.0.1', port: 2593 }} words={WORDS} rules={rules()} note={null} onConnect={vi.fn()} onSave={vi.fn()} />,
    );
    const labels = [...root.querySelectorAll('label')].map((each) => each.textContent);
    expect(labels.slice(0, WORDS.labels.length)).toEqual(WORDS.labels);
    expect(fieldOf(root, 'Password').type).toBe('password');
    expect(fieldOf(root, 'Host').value).toBe('127.0.0.1');
    expect(fieldOf(root, 'Port').value).toBe('2593');
    expect(root.textContent).toContain(WORDS.no_saved);
  });

  it('connects_with_the_form_and_keeps_the_password_nowhere', () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem');
    const onConnect = vi.fn<(form: LoginForm, kept: KeptLogin) => void>();
    const root = mount(
      <Login kept={null} saved={[CEDRIC]} blank={{ host: '', port: 2593 }} words={WORDS} rules={rules()} note={null} onConnect={onConnect} onSave={vi.fn()} />,
    );
    act(() => root.querySelector<HTMLButtonElement>('.saved-login')?.click());
    expect(root.textContent).toContain('acct2 @ play.example.com:2594');
    expect(fieldOf(root, 'Account').value).toBe('acct2');
    expect(fieldOf(root, 'Password').value).toBe('');
    type(fieldOf(root, 'Password'), 'hunter2');
    act(() => button(root, WORDS.connect).click());
    expect(onConnect.mock.calls[0][0]).toEqual({
      host: 'play.example.com',
      port: 2594,
      account: 'acct2',
      password: 'hunter2',
      shard: 'Britannia',
      character: 'Cedric',
      encryption: 'osi',
      era: null,
      version: null,
    });
    const [, kept] = onConnect.mock.calls[0];
    expect(kept.picked).toBe(CEDRIC);
    expect(kept.fields).not.toContain('hunter2');
    expect(setItem).not.toHaveBeenCalled();
    expect(location.search).not.toContain('hunter2');
  });

  it('shows_the_fault_of_a_form_and_does_not_connect', () => {
    const onConnect = vi.fn();
    const root = mount(
      <Login kept={null} saved={[]} blank={{ host: '', port: 2593 }} words={WORDS} rules={rules('Type the host.')} note={null} onConnect={onConnect} onSave={vi.fn()} />,
    );
    act(() => button(root, WORDS.connect).click());
    expect(root.textContent).toContain('Type the host.');
    expect(onConnect).not.toHaveBeenCalled();
  });

  it('saves_the_form_under_a_name_without_the_password', async () => {
    const onSave = vi.fn().mockResolvedValue([CEDRIC]);
    const root = mount(
      <Login kept={null} saved={[]} blank={{ host: '10.0.0.7', port: 2593 }} words={WORDS} rules={rules()} note={null} onConnect={vi.fn()} onSave={onSave} />,
    );
    type(fieldOf(root, 'Account'), 'mara');
    type(fieldOf(root, 'Password'), 'hunter2');
    act(() => button(root, WORDS.save_login).click());
    expect(fieldOf(root, 'Save as').value).toBe('mara@10.0.0.7');
    await act(async () => button(root, WORDS.save).click());
    expect(onSave).toHaveBeenCalledOnce();
    const [name, form] = onSave.mock.calls[0];
    expect(name).toBe('mara@10.0.0.7');
    expect(JSON.stringify(form)).not.toContain('hunter2');
    expect(root.textContent).toContain(`${WORDS.saved_as} mara@10.0.0.7.`);
  });
});

describe('Picking', () => {
  it('picks_the_shard_clicked', () => {
    const onPick = vi.fn();
    const root = mount(<Picking title={WORDS.pick_shard} names={['Atlantic', 'Pacific']} onPick={onPick} />);
    act(() => button(root, 'Pacific').click());
    expect(onPick).toHaveBeenCalledWith(1);
  });
});

describe('Characters', () => {
  const names = ['Mara', '', 'Cedric'];

  it('plays_a_slot_and_asks_twice_before_a_delete', () => {
    const onReply = vi.fn();
    const root = mount(<Characters names={names} refused={null} room={true} words={WORDS} onReply={onReply} onMake={vi.fn()} />);
    act(() => button(root, WORDS.empty_slot).click());
    expect(onReply).not.toHaveBeenCalled();
    const deletes = () => [...root.querySelectorAll('button')].filter((each) => each.textContent?.startsWith(WORDS.delete));
    expect(deletes().length).toBe(2);
    act(() => deletes()[1].click());
    expect(onReply).not.toHaveBeenCalled();
    expect(button(root, WORDS.delete_sure)).toBeTruthy();
    act(() => button(root, WORDS.delete_sure).click());
    expect(onReply).toHaveBeenCalledWith(remove(2));
    act(() => button(root, 'Mara').click());
    expect(onReply).toHaveBeenLastCalledWith(play(0));
    act(() => button(root, WORDS.leave).click());
    expect(onReply).toHaveBeenLastCalledWith(leave());
  });

  it('shows_every_slot_the_refusal_and_whether_a_new_character_has_room', () => {
    const onMake = vi.fn();
    const full = mount(<Characters names={names} refused="That name is taken." room={false} words={WORDS} onReply={vi.fn()} onMake={onMake} />);
    expect(full.textContent).toContain('That name is taken.');
    const slots = [...full.querySelectorAll('button')].filter((each) => each.textContent === WORDS.empty_slot);
    expect(slots.length).toBe(WORDS.character_slots - 2);
    act(() => button(full, WORDS.make).click());
    expect(onMake).not.toHaveBeenCalled();
    expect(full.textContent).toContain(WORDS.no_room);
    document.body.innerHTML = '';
    const open = mount(<Characters names={names} refused={null} room={true} words={WORDS} onReply={vi.fn()} onMake={onMake} />);
    act(() => button(open, WORDS.make).click());
    expect(onMake).toHaveBeenCalledOnce();
  });
});
