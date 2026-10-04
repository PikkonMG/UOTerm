import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { whenTokenNeeded } from '../src/net/api';
import { BACKOFF_MS } from '../src/net/backoff';
import {
  ANSWER_LATE,
  ANSWER_WAIT_MS,
  LINK_LOST,
  LINK_LOST_WAITING,
  LiveLink,
  LOSSES_BEFORE_TOKEN_CHECK,
  type LiveHandlers,
} from '../src/net/live';
import { FakeSocket } from './fake_socket';

const SESSION = 's1';
const CALL_ID = 3;
const ACT_ID = 4;

function handlers() {
  return {
    frame: vi.fn<LiveHandlers['frame']>(),
    answer: vi.fn<LiveHandlers['answer']>(),
    ended: vi.fn<LiveHandlers['ended']>(),
    state: vi.fn<LiveHandlers['state']>(),
  };
}

beforeEach(() => {
  vi.useFakeTimers();
  FakeSocket.install();
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('LiveLink', () => {
  it('reconnects_after_a_drop_and_reports_lost_then_open', async () => {
    const states: string[] = [];
    const link = new LiveLink(SESSION, { frame() {}, answer() {}, ended() {}, state: (s) => states.push(s) });
    FakeSocket.last().open();
    FakeSocket.last().drop();
    await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]);
    FakeSocket.last().open();
    expect(states).toEqual(['open', 'lost', 'open']);
    link.close();
  });

  it('opens_the_live_path_of_the_session', () => {
    const link = new LiveLink(SESSION, handlers());
    expect(FakeSocket.last().url).toMatch(/^ws:\/\/[^/]+\/v1\/sessions\/s1\/live$/);
    link.close();
  });

  it('waits_longer_after_each_failed_try_and_repeats_the_last_wait', async () => {
    const link = new LiveLink(SESSION, handlers());
    for (const wait of [...BACKOFF_MS, BACKOFF_MS[BACKOFF_MS.length - 1]]) {
      const made = FakeSocket.made.length;
      FakeSocket.last().drop();
      await vi.advanceTimersByTimeAsync(wait - 1);
      expect(FakeSocket.made.length).toBe(made);
      await vi.advanceTimersByTimeAsync(1);
      expect(FakeSocket.made.length).toBe(made + 1);
    }
    link.close();
  });

  it('gives_frames_and_answers_to_the_handlers', () => {
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    FakeSocket.last().open();
    link.send({ kind: 'call', id: CALL_ID, tool: 'observe', args: {} });
    FakeSocket.last().receive({ kind: 'frame', watch: { tick: 1 } });
    FakeSocket.last().receive({ kind: 'answer', id: CALL_ID, ok: true, result: { seen: 2 } });
    expect(on.frame).toHaveBeenCalledWith({ tick: 1 });
    expect(on.answer).toHaveBeenCalledWith(CALL_ID, true, { seen: 2 });
    expect(FakeSocket.last().sentJson()).toEqual([{ kind: 'call', id: CALL_ID, tool: 'observe', args: {} }]);
    link.close();
  });

  it('gives_the_words_of_a_refused_answer', () => {
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    FakeSocket.last().open();
    link.send({ kind: 'act', id: ACT_ID, calls: [{ tool: 'use', args: {} }] });
    FakeSocket.last().receive({ kind: 'answer', id: ACT_ID, ok: false, result: null, error: 'too many acts wait' });
    expect(on.answer).toHaveBeenCalledWith(ACT_ID, false, { error: 'too many acts wait' });
    link.close();
  });

  it('fails_an_answer_that_does_not_come_in_time_and_drops_it_when_late', async () => {
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    FakeSocket.last().open();
    link.send({ kind: 'act', id: ACT_ID, calls: [{ tool: 'use', args: {} }] });
    await vi.advanceTimersByTimeAsync(ANSWER_WAIT_MS);
    expect(on.answer).toHaveBeenCalledWith(ACT_ID, false, { error: ANSWER_LATE });
    FakeSocket.last().receive({ kind: 'answer', id: ACT_ID, ok: true, result: null });
    expect(on.answer).toHaveBeenCalledTimes(1);
    link.close();
  });

  it('an_answer_in_time_is_given_once', async () => {
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    FakeSocket.last().open();
    link.send({ kind: 'call', id: CALL_ID, tool: 'observe', args: {} });
    FakeSocket.last().receive({ kind: 'answer', id: CALL_ID, ok: true, result: null });
    await vi.advanceTimersByTimeAsync(ANSWER_WAIT_MS);
    expect(on.answer).toHaveBeenCalledTimes(1);
    link.close();
  });

  it('fails_the_waiting_answers_when_the_link_drops', () => {
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    FakeSocket.last().open();
    link.send({ kind: 'call', id: CALL_ID, tool: 'observe', args: {} });
    FakeSocket.last().drop();
    expect(on.answer).toHaveBeenCalledWith(CALL_ID, false, { error: LINK_LOST_WAITING });
    link.close();
  });

  it('fails_a_call_at_once_while_the_link_is_lost', () => {
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    link.send({ kind: 'act', id: ACT_ID, calls: [] });
    expect(on.answer).toHaveBeenCalledWith(ACT_ID, false, { error: LINK_LOST });
    expect(FakeSocket.last().sent).toEqual([]);
    link.close();
  });

  it('sends_the_size_again_after_a_reconnect', async () => {
    const link = new LiveLink(SESSION, handlers());
    FakeSocket.last().open();
    link.send({ kind: 'size', size: 30 });
    FakeSocket.last().drop();
    await vi.advanceTimersByTimeAsync(BACKOFF_MS[0]);
    FakeSocket.last().open();
    expect(FakeSocket.last().sentJson()).toEqual([{ kind: 'size', size: 30 }]);
    link.close();
  });

  it('does_not_reopen_an_ended_session', async () => {
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    FakeSocket.last().open();
    FakeSocket.last().receive({ kind: 'ended' });
    await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1] * 2);
    expect(on.ended).toHaveBeenCalledTimes(1);
    expect(FakeSocket.made.length).toBe(1);
    expect(on.state).not.toHaveBeenCalledWith('lost');
    link.close();
  });

  it('drops_a_message_that_does_not_read', () => {
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    FakeSocket.last().open();
    FakeSocket.last().onmessage?.(new MessageEvent('message', { data: '{"kind": "fra' }));
    FakeSocket.last().receive({ kind: 'frame', watch: { tick: 2 } });
    expect(on.frame).toHaveBeenCalledTimes(1);
    expect(on.frame).toHaveBeenCalledWith({ tick: 2 });
    link.close();
  });

  it('asks_for_the_token_once_after_tries_in_a_row_fail', async () => {
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(new Response('{"error":"unauthorized"}', { status: 401 }));
    const tokenNeeded = vi.fn();
    const stop = whenTokenNeeded(tokenNeeded);
    const link = new LiveLink(SESSION, handlers());
    for (let tries = 1; tries < LOSSES_BEFORE_TOKEN_CHECK; tries += 1) {
      FakeSocket.last().drop();
      await vi.advanceTimersByTimeAsync(BACKOFF_MS[tries - 1]);
    }
    expect(fetchSpy).not.toHaveBeenCalled();
    FakeSocket.last().drop();
    await vi.waitFor(() => expect(tokenNeeded).toHaveBeenCalledTimes(1));
    expect(fetchSpy).toHaveBeenCalledWith('/v1/sessions', undefined);
    await vi.advanceTimersByTimeAsync(BACKOFF_MS[LOSSES_BEFORE_TOKEN_CHECK - 1]);
    FakeSocket.last().drop();
    await vi.advanceTimersByTimeAsync(0);
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    stop();
    link.close();
  });

  /** Drops the link as often as it takes for the page to ask after its session. */
  async function dropUntilAsked() {
    for (let tries = 1; tries <= LOSSES_BEFORE_TOKEN_CHECK; tries += 1) {
      FakeSocket.last().drop();
      if (tries < LOSSES_BEFORE_TOKEN_CHECK) await vi.advanceTimersByTimeAsync(BACKOFF_MS[tries - 1]);
    }
  }

  it('ends_the_link_when_its_session_is_gone', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(JSON.stringify({ sessions: ['s2'] })));
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    await dropUntilAsked();
    await vi.waitFor(() => expect(on.ended).toHaveBeenCalledOnce());
    const made = FakeSocket.made.length;
    await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1] * 4);
    expect(FakeSocket.made.length).toBe(made);
    link.close();
  });

  it('keeps_trying_while_its_session_runs', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(JSON.stringify({ sessions: [SESSION] })));
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    await dropUntilAsked();
    await vi.advanceTimersByTimeAsync(BACKOFF_MS[LOSSES_BEFORE_TOKEN_CHECK - 1]);
    FakeSocket.last().open();
    expect(on.ended).not.toHaveBeenCalled();
    expect(on.state).toHaveBeenLastCalledWith('open');
    link.close();
  });

  it('does_not_reopen_after_close', async () => {
    const link = new LiveLink(SESSION, handlers());
    FakeSocket.last().open();
    link.close();
    await vi.advanceTimersByTimeAsync(BACKOFF_MS[BACKOFF_MS.length - 1] * 2);
    expect(FakeSocket.made.length).toBe(1);
  });
});
