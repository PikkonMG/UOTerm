import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PNG_SIGNATURE, takeScreenshot } from '../src/screenshot';

const FILE = 'Screenshot_2026-10-04_12-00-00.png';
const STATUS_CREATED = 201;
const STATUS_FAILED = 500;

let drawn: unknown[];
let posted: { path: string; body: Uint8Array }[];

beforeEach(() => {
  drawn = [];
  posted = [];
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockImplementation(() => ({ drawImage: (picture: unknown) => drawn.push(picture) }) as unknown as RenderingContext);
  vi.spyOn(HTMLCanvasElement.prototype, 'toBlob').mockImplementation((done) => done(new Blob([new Uint8Array([...PNG_SIGNATURE, 0])])));
  vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
    posted.push({ path: String(input), body: new Uint8Array(await (init?.body as Blob).arrayBuffer()) });
    return new Response(JSON.stringify({ file: FILE }), { status: STATUS_CREATED });
  });
});

afterEach(() => vi.restoreAllMocks());

describe('screenshot', () => {
  it('posts_one_png_of_the_world_and_the_panels_and_gives_its_file', async () => {
    const world = document.createElement('canvas');
    const overlay = document.createElement('div');
    const panels = { width: 1, height: 1 } as HTMLImageElement;
    const file = await takeScreenshot(world, overlay, () => Promise.resolve(panels));
    expect(file).toBe(FILE);
    expect(posted).toHaveLength(1);
    expect(posted[0].path).toBe('/v1/screenshots');
    expect([...posted[0].body.slice(0, PNG_SIGNATURE.length)]).toEqual([...PNG_SIGNATURE]);
    expect(drawn[0]).toBe(world);
    expect(drawn.at(-1)).toBe(panels);
  });

  it('keeps_the_world_alone_when_the_panels_cannot_be_drawn', async () => {
    const world = document.createElement('canvas');
    await takeScreenshot(world, document.createElement('div'), () => Promise.resolve(null));
    expect(drawn).toEqual([world]);
  });

  it('fails_with_the_words_of_the_server', async () => {
    vi.mocked(fetch).mockResolvedValue(new Response(JSON.stringify({ error: 'disk full' }), { status: STATUS_FAILED }));
    const taking = takeScreenshot(document.createElement('canvas'), document.createElement('div'), () => Promise.resolve(null));
    await expect(taking).rejects.toThrow('disk full');
  });
});
