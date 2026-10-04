import { afterEach, describe, expect, it, vi } from 'vitest';
import { api, ApiFailed, giveToken, jsonInit, readMessage, TokenNeeded, whenTokenGiven, whenTokenNeeded } from '../src/net/api';
import { BACKOFF_MS, backoffWait, LONGEST_BACKOFF_MS } from '../src/net/backoff';

afterEach(() => vi.restoreAllMocks());

describe('api', () => {
  it('asks_for_the_token_again_after_401', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"error":"unauthorized"}', { status: 401 }));
    await expect(api('/v1/sessions')).rejects.toBeInstanceOf(TokenNeeded);
  });

  it('returns_the_json_body', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"sessions":["s1"]}', { status: 200 }));
    await expect(api<{ sessions: string[] }>('/v1/sessions')).resolves.toEqual({ sessions: ['s1'] });
  });

  it('gives_the_status_and_the_words_of_a_refusal', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"error":"no such session"}', { status: 404 }));
    const failed = await api('/v1/sessions/s9').catch((error: unknown) => error);
    expect(failed).toBeInstanceOf(ApiFailed);
    expect(failed).toMatchObject({ status: 404, message: 'no such session' });
  });

  it('answers_nothing_for_no_content', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 204 }));
    await expect(api('/v1/kept/watch-hotbar.toml', jsonInit('PUT', {}))).resolves.toBeUndefined();
  });

  it('sends_a_body_as_json', () => {
    const init = jsonInit('POST', { token: 't' });
    expect(init.method).toBe('POST');
    expect(new Headers(init.headers).get('content-type')).toBe('application/json');
    expect(init.body).toBe('{"token":"t"}');
  });
});

describe('giveToken', () => {
  it('is_true_when_the_server_takes_the_token', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 204 }));
    await expect(giveToken('right')).resolves.toBe(true);
    expect(fetchSpy).toHaveBeenCalledWith('/v1/web/token', expect.objectContaining({ method: 'POST', body: '{"token":"right"}' }));
  });

  it('is_false_for_a_wrong_token', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"error":"unauthorized"}', { status: 401 }));
    await expect(giveToken('wrong')).resolves.toBe(false);
  });
});

describe('whenTokenNeeded', () => {
  it('hears_each_401_until_stopped', async () => {
    vi.spyOn(globalThis, 'fetch').mockImplementation(() =>
      Promise.resolve(new Response('{"error":"unauthorized"}', { status: 401 })),
    );
    const tokenNeeded = vi.fn();
    const stop = whenTokenNeeded(tokenNeeded);
    await api('/v1/sessions').catch(() => undefined);
    expect(tokenNeeded).toHaveBeenCalledTimes(1);
    stop();
    await api('/v1/sessions').catch(() => undefined);
    expect(tokenNeeded).toHaveBeenCalledTimes(1);
  });
});

describe('whenTokenGiven', () => {
  it('hears_a_token_the_server_took_and_not_a_wrong_one', async () => {
    const tokenGiven = vi.fn();
    const stop = whenTokenGiven(tokenGiven);
    vi.spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(new Response('{"error":"unauthorized"}', { status: 401 }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }));
    await giveToken('wrong');
    expect(tokenGiven).not.toHaveBeenCalled();
    await giveToken('right');
    expect(tokenGiven).toHaveBeenCalledTimes(1);
    stop();
  });
});

describe('readMessage', () => {
  it('reads_json_and_drops_what_does_not_read', () => {
    expect(readMessage('{"kind":"ended"}')).toEqual({ kind: 'ended' });
    expect(readMessage('{"kind":')).toBeUndefined();
    expect(readMessage(new ArrayBuffer(1))).toBeUndefined();
  });
});

describe('backoffWait', () => {
  it('grows_then_repeats_its_last_step', () => {
    expect(BACKOFF_MS.map((_, retry) => backoffWait(retry))).toEqual(BACKOFF_MS);
    expect(backoffWait(BACKOFF_MS.length + 3)).toBe(LONGEST_BACKOFF_MS);
  });
});
