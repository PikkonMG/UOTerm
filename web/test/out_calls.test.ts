import { afterEach, describe, expect, it, vi } from 'vitest';
import type { LiveOut } from '../src/net/live';
import type { InputEvent } from '../src/input/events';
import { CHAT_ATTRIBUTE } from '../src/input/keys';
import { sendOut } from '../src/out_calls';

const SESSION = 's1';
const PROFILE_PATH = '/v1/profiles/127.0.0.1:2593/Mara';

function places() {
  const send = vi.fn<(message: LiveOut) => void>();
  const answer = vi.fn<(id: number, ok: boolean, resultJson: string) => void>();
  const input = vi.fn<(event: InputEvent) => void>();
  return { session: SESSION, profilePath: PROFILE_PATH, link: { send }, send, answer, input };
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

  it('gives_the_player_a_text_file_to_keep', () => {
    const made = vi.fn<(blob: Blob) => string>().mockReturnValue('blob:journal');
    vi.stubGlobal('URL', { createObjectURL: made, revokeObjectURL: vi.fn() });
    const clicked = vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});
    sendOut([{ kind: 'Download', name: 'Mara-20261004-090507.txt', text: 'Bob: hail' }], places());
    expect(made).toHaveBeenCalledTimes(1);
    const link = clicked.mock.contexts[0] as HTMLAnchorElement;
    expect(link.download).toBe('Mara-20261004-090507.txt');
    expect(link.href).toBe('blob:journal');
    vi.unstubAllGlobals();
  });

  it('gives_the_chat_line_the_keys_and_takes_them_back_as_the_view_asks', () => {
    const field = document.createElement('input');
    field.setAttribute(CHAT_ATTRIBUTE, '');
    document.body.append(field);
    sendOut([{ kind: 'ChatFocus', take: true }], places());
    expect(document.activeElement).toBe(field);
    sendOut([{ kind: 'ChatFocus', take: false }], places());
    expect(document.activeElement).not.toBe(field);
    field.remove();
  });

  it('pastes_the_clipboard_into_the_chat_line', async () => {
    const field = document.createElement('input');
    field.setAttribute(CHAT_ATTRIBUTE, '');
    field.value = 'hail ';
    document.body.append(field);
    vi.stubGlobal('navigator', { clipboard: { readText: () => Promise.resolve('friend') } });
    const out = places();
    sendOut([{ kind: 'ChatPaste' }], out);
    await vi.waitFor(() => expect(out.input).toHaveBeenCalledWith({ kind: 'ChatWords', text: 'hail friend' }));
    vi.unstubAllGlobals();
    field.remove();
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

  it('keeps_the_default_profile_and_asks_for_the_full_screen_as_the_window_mode_says', async () => {
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 204 }));
    const asked = vi.fn(() => Promise.resolve());
    document.documentElement.requestFullscreen = asked;
    sendOut(
      [
        { kind: 'SaveDefaultProfile', profile: { general: {} } },
        { kind: 'Fullscreen', on: true },
      ],
      places(),
    );
    await vi.waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(1));
    expect(fetchSpy).toHaveBeenCalledWith('/v1/profiles/default', expect.objectContaining({ method: 'PUT' }));
    expect(asked).toHaveBeenCalledTimes(1);
  });
});
