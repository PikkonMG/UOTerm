import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ANSWER_LATE, ANSWER_WAIT_MS, LINK_LOST, LiveLink, RECONNECT_MS, type LiveHandlers } from '../src/net/live';
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
});

describe('LiveLink', () => {
  it('reconnects_after_a_drop_and_reports_lost_then_open', async () => {
    const states: string[] = [];
    const link = new LiveLink(SESSION, { frame() {}, answer() {}, ended() {}, state: (s) => states.push(s) });
    FakeSocket.last().open();
    FakeSocket.last().drop();
    await vi.advanceTimersByTimeAsync(RECONNECT_MS[0]);
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
    for (const wait of [...RECONNECT_MS, RECONNECT_MS[RECONNECT_MS.length - 1]]) {
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
    expect(on.answer).toHaveBeenCalledWith(CALL_ID, false, { error: LINK_LOST });
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
    await vi.advanceTimersByTimeAsync(RECONNECT_MS[0]);
    FakeSocket.last().open();
    expect(FakeSocket.last().sentJson()).toEqual([{ kind: 'size', size: 30 }]);
    link.close();
  });

  it('does_not_reopen_an_ended_session', async () => {
    const on = handlers();
    const link = new LiveLink(SESSION, on);
    FakeSocket.last().open();
    FakeSocket.last().receive({ kind: 'ended' });
    await vi.advanceTimersByTimeAsync(RECONNECT_MS[RECONNECT_MS.length - 1] * 2);
    expect(on.ended).toHaveBeenCalledTimes(1);
    expect(FakeSocket.made.length).toBe(1);
    expect(on.state).not.toHaveBeenCalledWith('lost');
    link.close();
  });

  it('does_not_reopen_after_close', async () => {
    const link = new LiveLink(SESSION, handlers());
    FakeSocket.last().open();
    link.close();
    await vi.advanceTimersByTimeAsync(RECONNECT_MS[RECONNECT_MS.length - 1] * 2);
    expect(FakeSocket.made.length).toBe(1);
  });
});
