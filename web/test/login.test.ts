import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { login, LoginFailed, LOGIN_CLOSED, type LoginAsk, type LoginForm, type LoginReply } from '../src/net/login';
import { FakeSocket } from './fake_socket';

const FORM: LoginForm = { host: '127.0.0.1', port: 2593, account: 'test', password: 'pw', shard: null, character: 'Mara' };
const SHARD_ASK: LoginAsk = { kind: 'Shard', names: ['Atlantic', 'Pacific'] };
const PICK_SECOND: LoginReply = { kind: 'Pick', index: 1 };
const VERSION = '7.0.102.3';

beforeEach(() => FakeSocket.install());
afterEach(() => vi.unstubAllGlobals());

describe('login', () => {
  it('sends_the_form_answers_each_question_and_gives_the_session', async () => {
    const onAsk = vi.fn().mockResolvedValue(PICK_SECOND);
    const session = login(FORM, onAsk);
    const socket = FakeSocket.last();
    expect(socket.url).toMatch(/^ws:\/\/[^/]+\/v1\/login\/live$/);
    socket.open();
    socket.receive({ kind: 'ask', ask: SHARD_ASK, version: VERSION });
    await vi.waitFor(() => expect(socket.sent.length).toBe(2));
    socket.receive({ kind: 'ready', session: 's1' });
    await expect(session).resolves.toBe('s1');
    expect(onAsk).toHaveBeenCalledWith(SHARD_ASK, VERSION);
    expect(socket.sentJson()).toEqual([
      { kind: 'login', ...FORM },
      { kind: 'reply', reply: PICK_SECOND },
    ]);
  });

  it('rejects_with_the_words_of_the_failure', async () => {
    const session = login(FORM, vi.fn());
    FakeSocket.last().open();
    FakeSocket.last().receive({ kind: 'failed', words: 'bad password' });
    await expect(session).rejects.toEqual(new LoginFailed('bad password'));
  });

  it('rejects_when_the_link_closes_before_the_end', async () => {
    const session = login(FORM, vi.fn());
    FakeSocket.last().open();
    FakeSocket.last().drop();
    await expect(session).rejects.toEqual(new LoginFailed(LOGIN_CLOSED));
  });

  it('closes_the_link_when_the_page_cannot_answer', async () => {
    const session = login(FORM, vi.fn().mockRejectedValue(new LoginFailed('gone')));
    const socket = FakeSocket.last();
    socket.open();
    socket.receive({ kind: 'ask', ask: SHARD_ASK, version: VERSION });
    await expect(session).rejects.toEqual(new LoginFailed('gone'));
    expect(socket.readyState).toBe(FakeSocket.CLOSED);
  });
});
