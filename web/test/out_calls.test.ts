import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LiveOut } from '../src/net/live';
import { sendOut } from '../src/out_calls';

const SESSION = 's1';
const PROFILE_PATH = '/v1/profiles/127.0.0.1:2593/Mara';

function places() {
  const send = vi.fn<(message: LiveOut) => void>();
  const answer = vi.fn<(id: number, ok: boolean, resultJson: string) => void>();
  return { session: SESSION, profilePath: PROFILE_PATH, link: { send }, send, answer };
}

afterEach(() => vi.restoreAllMocks());

describe('sendOut', () => {
  it('sends_an_act_as_one_act_message_and_a_read_as_a_call', () => {
    const out = places();
    const calls = [{ tool: 'walk', args: { dir: 'n' } }];
    sendOut(
      [
        { kind: 'Act', id: 1, act: { calls, words: 'Walk north' } },
        { kind: 'Read', id: 2, tool: 'properties', args: { serial: 5 } },
      ],
      out,
    );
    expect(out.send.mock.calls).toEqual([
      [{ kind: 'act', id: 1, calls }],
      [{ kind: 'call', id: 2, tool: 'properties', args: { serial: 5 } }],
    ]);
  });

  it('asks_jev_and_gives_the_answer_back', async () => {
    const out = places();
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"act":{"calls":[],"words":"x"}}'));
    sendOut([{ kind: 'Jev', id: 3, route: 'order', body: { words: 'go', frame: {} } }], out);
    await vi.waitFor(() => expect(out.answer).toHaveBeenCalledWith(3, true, '{"act":{"calls":[],"words":"x"}}'));
    expect(fetchSpy).toHaveBeenCalledWith('/v1/sessions/s1/jev/order', expect.objectContaining({ method: 'POST' }));
  });

  it('gives_a_refusal_of_jev_back_as_its_words', async () => {
    const out = places();
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"error":"Jev is off"}', { status: 409 }));
    sendOut([{ kind: 'Jev', id: 4, route: 'order', body: {} }], out);
    await vi.waitFor(() => expect(out.answer).toHaveBeenCalledWith(4, false, '{"error":"Jev is off"}'));
  });

  it('keeps_the_profile_and_the_kept_files', async () => {
    const out = places();
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 204 }));
    sendOut(
      [
        { kind: 'SaveProfile', profile: { general: {} } },
        { kind: 'SaveKept', name: 'watch-hotbar.toml', data: { bars: [] } },
      ],
      out,
    );
    await vi.waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(2));
    expect(fetchSpy).toHaveBeenCalledWith(PROFILE_PATH, expect.objectContaining({ method: 'PUT', body: '{"general":{}}' }));
    expect(fetchSpy).toHaveBeenCalledWith('/v1/kept/watch-hotbar.toml', expect.objectContaining({ method: 'PUT', body: '{"bars":[]}' }));
  });
});
