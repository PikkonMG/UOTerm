import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { startGame } from '../src/game';
import { FakeAudioContext } from './fake_audio';
import { FakeSocket } from './fake_socket';

/** What the fakes of the view and the renderer did, and how the renderer behaves. */
const made = vi.hoisted(() => ({
  viewsFreed: 0,
  /** The session each view was told its pictures are of. */
  sessions: [] as string[],
  rendererFails: false,
  disposeFails: false,
}));

vi.mock('../src/wasm/uoterm_web.js', () => ({
  default: () => Promise.resolve(),
  artMostScale: () => 1,
  atlasSide: () => 1,
  clickDistance: () => 1,
  renderMidi: () => new Float32Array(0),
  wheelPointsPerNotch: () => 1,
  whiteSide: () => 1,
  WebView: class {
    setSession(session: string): void {
      made.sessions.push(session);
    }
    setTextMeasure(): void {}
    setBodyMeasure(): void {}
    setFullscreen(): void {}
    setOrdersOn(): void {}
    free(): void {
      made.viewsFreed += 1;
    }
  },
}));

vi.mock('../src/world/renderer', () => ({
  WorldRenderer: class {
    constructor() {
      if (made.rendererFails) throw new Error('no WebGL');
    }
    dispose(): void {
      if (made.disposeFails) throw new Error('the renderer would not go');
    }
  },
}));

const SESSION = 's1';
const PROFILE = { path: '/v1/profiles/default', value: {}, soundFont: '/v1/sound-font' };
const SHOWS = { panels: vi.fn(), words: vi.fn() };

/** Starts a game on a new canvas. */
function start() {
  return startGame(SESSION, document.createElement('canvas'), document.createElement('div'), PROFILE, SHOWS);
}

beforeEach(() => {
  made.viewsFreed = 0;
  made.sessions = [];
  made.rendererFails = false;
  made.disposeFails = false;
  FakeSocket.install();
  vi.stubGlobal('AudioContext', FakeAudioContext);
  vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"on":false}'));
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('startGame', () => {
  it('takes_down_what_it_made_and_throws_when_the_renderer_fails', () => {
    made.rendererFails = true;
    const listened = vi.spyOn(window, 'addEventListener');
    expect(start).toThrow('no WebGL');
    expect(made.sessions).toEqual([SESSION]);
    expect(made.viewsFreed).toBe(1);
    expect(FakeSocket.made).toHaveLength(0);
    expect(listened).not.toHaveBeenCalled();
  });

  it('ends_the_game_when_the_session_ends_even_if_a_part_will_not_go', async () => {
    made.disposeFails = true;
    const logged = vi.spyOn(console, 'error').mockImplementation(() => {});
    const game = start();
    FakeSocket.last().open();
    FakeSocket.last().receive({ kind: 'ended' });
    await expect(game.ended).resolves.toBeUndefined();
    expect(made.viewsFreed).toBe(1);
    expect(logged).toHaveBeenCalled();
  });

  it('stops_without_a_throw_when_a_part_will_not_go', () => {
    made.disposeFails = true;
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const game = start();
    expect(() => game.stop()).not.toThrow();
    expect(made.viewsFreed).toBe(1);
  });
});
