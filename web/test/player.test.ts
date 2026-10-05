import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Player, resumeOnGesture, type AudioOut } from '../src/audio/player';
import { asContext, FAKE_RATE, FakeAudioContext } from './fake_audio';

const SOUND = 3;
const OTHER_SOUND = 4;
const TRACK = 9;
const FONT_PATH = '/v1/soundfont?shard=s&character=Mara';
const SOUND_BYTES = 8;
const MIDI_TYPE = 'audio/midi';
const MP3_TYPE = 'audio/mpeg';
const REPEATS = 'x-uoterm-repeats';
const STATUS_NOT_FOUND = 404;

const effect = (voice: number, sound: number, volume: number, replace: number | null = null): AudioOut => ({ kind: 'Effect', voice, sound, volume, replace });
const music = (soundFont: string | null = null): AudioOut => ({ kind: 'Music', track: TRACK, volume: 0.4, sound_font: soundFont });

/** Lets the fetches and the decoding of this turn finish. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

let context: FakeAudioContext;
let ended: number[];
let rendered: { midi: Uint8Array; font: Uint8Array; rate: number }[];
let fetched: string[];
/** The answers of the server by path; a path not here is not found. */
let answers: Map<string, () => Response>;

function player(): Player {
  return new Player(asContext(context), {
    ended: (voice) => ended.push(voice),
    soundFontPath: FONT_PATH,
    renderMidi: (midi, font, rate) => {
      rendered.push({ midi, font, rate });
      return new Float32Array([0.1, -0.1, 0.2, -0.2]);
    },
  });
}

beforeEach(async () => {
  context = new FakeAudioContext();
  await context.resume();
  ended = [];
  rendered = [];
  fetched = [];
  answers = new Map([
    [`/v1/sound/${SOUND}`, () => new Response(new Uint8Array(SOUND_BYTES))],
    [`/v1/sound/${OTHER_SOUND}`, () => new Response(new Uint8Array(SOUND_BYTES))],
  ]);
  vi.spyOn(globalThis, 'fetch').mockImplementation((input) => {
    const path = String(input);
    fetched.push(path);
    const answer = answers.get(path);
    return Promise.resolve(answer ? answer() : new Response(null, { status: STATUS_NOT_FOUND }));
  });
});

afterEach(() => vi.restoreAllMocks());

describe('player', () => {
  it('fetches_a_sound_one_time_and_plays_it_at_its_volume', async () => {
    const sounds = player();
    sounds.play([effect(0, SOUND, 0.5)]);
    await settle();
    expect(fetched).toEqual([`/v1/sound/${SOUND}`]);
    expect(context.playing()).toHaveLength(1);
    expect(context.playing()[0].output?.gain.value).toBe(0.5);
    sounds.play([effect(1, SOUND, 0.5)]);
    await settle();
    expect(fetched).toEqual([`/v1/sound/${SOUND}`]);
    expect(context.playing()).toHaveLength(2);
  });

  it('tells_the_view_when_a_voice_ended', async () => {
    const sounds = player();
    sounds.play([effect(7, SOUND, 1)]);
    await settle();
    context.playing()[0].end();
    expect(ended).toEqual([7]);
  });

  it('stops_the_voice_a_new_sound_replaces', async () => {
    const sounds = player();
    sounds.play([effect(0, SOUND, 1)]);
    await settle();
    sounds.play([effect(1, OTHER_SOUND, 1, 0)]);
    await settle();
    expect(context.playing()).toHaveLength(1);
    expect(context.sources[0].stopped).toBe(true);
  });

  it('a_sound_the_server_lacks_ends_its_voice', async () => {
    const sounds = player();
    sounds.play([effect(5, 99, 1)]);
    await settle();
    expect(context.playing()).toHaveLength(0);
    expect(ended).toEqual([5]);
  });

  it('a_suspended_context_plays_no_effect', async () => {
    const sounds = player();
    context.state = 'suspended';
    sounds.play([effect(2, SOUND, 1)]);
    await settle();
    expect(context.playing()).toHaveLength(0);
    expect(ended).toEqual([2]);
  });

  it('sets_the_volumes_the_view_tells', async () => {
    const sounds = player();
    sounds.play([effect(0, SOUND, 1), { kind: 'Rain', sound: OTHER_SOUND, volume: 0.2 }]);
    await settle();
    sounds.play([{ kind: 'Volumes', voices: [{ voice: 0, volume: 0 }], music: 0, rain: 0 }]);
    for (const source of context.playing()) expect(source.output?.gain.value).toBe(0);
  });

  it('loops_the_rain_until_it_stops', async () => {
    const sounds = player();
    sounds.play([{ kind: 'Rain', sound: SOUND, volume: 0.2 }]);
    await settle();
    expect(context.playing()).toHaveLength(1);
    expect(context.playing()[0].loop).toBe(true);
    sounds.play([{ kind: 'RainStop' }]);
    expect(context.playing()).toHaveLength(0);
  });

  it('plays_an_mp3_track_and_loops_it_when_it_repeats', async () => {
    answers.set(`/v1/music/${TRACK}`, () => new Response(new Uint8Array(SOUND_BYTES), { headers: { 'content-type': MP3_TYPE, [REPEATS]: 'true' } }));
    const sounds = player();
    sounds.play([music()]);
    await settle();
    expect(fetched).toEqual([`/v1/music/${TRACK}`]);
    expect(context.playing()).toHaveLength(1);
    expect(context.playing()[0].loop).toBe(true);
    expect(context.playing()[0].output?.gain.value).toBe(0.4);
    sounds.play([{ kind: 'MusicStop' }]);
    expect(context.playing()).toHaveLength(0);
  });

  it('plays_a_midi_track_with_the_sound_font', async () => {
    answers.set(FONT_PATH, () => new Response(new Uint8Array([1, 2])));
    answers.set(`/v1/music/${TRACK}?midi=true`, () => new Response(new Uint8Array([3]), { headers: { 'content-type': MIDI_TYPE, [REPEATS]: 'false' } }));
    const sounds = player();
    sounds.play([music('font.sf2')]);
    await settle();
    await settle();
    expect(rendered).toHaveLength(1);
    expect(rendered[0].rate).toBe(FAKE_RATE);
    expect([...rendered[0].font]).toEqual([1, 2]);
    const [source] = context.playing();
    expect(source.loop).toBe(false);
    expect(source.buffer?.numberOfChannels).toBe(2);
    expect([...(source.buffer?.channels[1] ?? [])]).toEqual([-0.1, -0.2].map(Math.fround));
  });

  it('a_track_asked_while_another_loads_plays_alone', async () => {
    answers.set(`/v1/music/${TRACK}`, () => new Response(new Uint8Array(SOUND_BYTES), { headers: { 'content-type': MP3_TYPE } }));
    const sounds = player();
    sounds.play([music(), { kind: 'MusicStop' }]);
    await settle();
    expect(context.playing()).toHaveLength(0);
  });
});

describe('resume_on_gesture', () => {
  it('resumes_the_context_on_the_first_press_and_stops_listening', () => {
    const fake = new FakeAudioContext();
    const resume = vi.spyOn(fake, 'resume');
    resumeOnGesture(window, asContext(fake));
    window.dispatchEvent(new Event('keydown'));
    window.dispatchEvent(new Event('pointerdown'));
    expect(resume).toHaveBeenCalledTimes(1);
  });
});
