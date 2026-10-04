import { afterEach, describe, expect, it, vi } from 'vitest';
import { giveToken, whenTokenNeeded } from '../src/net/api';
import { ArtFeed, ART_PARALLEL, FETCH_TRIES, pixelsOf, type FeedView } from '../src/net/art';
import { backoffWait, LONGEST_BACKOFF_MS } from '../src/net/backoff';

type FakeView = { [K in keyof FeedView]: ReturnType<typeof vi.fn> };

/** A view that wants `art`, `data` and `posts` once, then nothing. */
function viewWanting(
  art: { key: string; request: unknown }[],
  data: string[] = [],
  posts: { key: string; path: string; body: unknown }[] = [],
): FakeView {
  return {
    artWanted: vi.fn().mockReturnValueOnce(art).mockReturnValue([]),
    dataWanted: vi.fn().mockReturnValueOnce(data).mockReturnValue([]),
    postsWanted: vi.fn().mockReturnValueOnce(posts).mockReturnValue([]),
    artForgotten: vi.fn().mockReturnValue([]),
    artArrived: vi.fn(),
    artMissing: vi.fn(),
    dataArrived: vi.fn(),
    dataMissing: vi.fn(),
    postArrived: vi.fn(),
    postMissing: vi.fn(),
  };
}

/** A decoded picture of the size the test names. */
function bitmap(width: number, height: number) {
  return { width, height, close: vi.fn() };
}

/** The time a request that never reaches the server takes to be given up. */
const ALL_RETRY_WAITS_MS = Array.from({ length: FETCH_TRIES - 1 }, (_, retry) => backoffWait(retry)).reduce((a, b) => a + b, 0);

afterEach(() => {
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe('ArtFeed', () => {
  it('keeps_at_most_the_parallel_limit_in_flight', () => {
    const wanted = Array.from({ length: 20 }, (_, i) => ({ key: String(i), request: { kind: 'Land', land_id: i, hue: 0 } }));
    const view = viewWanting(wanted);
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockReturnValue(new Promise(() => {}));
    new ArtFeed(view as never).pump();
    expect(fetchSpy).toHaveBeenCalledTimes(ART_PARALLEL);
  });

  it('marks_a_404_as_missing', async () => {
    const view = viewWanting([{ key: '9', request: { kind: 'Item', graphic: 9 } }]);
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(null, { status: 404 }));
    const feed = new ArtFeed(view as never);
    feed.pump();
    await vi.waitFor(() => expect(view.artMissing).toHaveBeenCalledWith('9'));
  });

  it('posts_the_request_and_keeps_the_pixels_with_their_anchor', async () => {
    const request = { kind: 'Item', graphic: 7 };
    const view = viewWanting([{ key: 'item-7', request }]);
    const picture = bitmap(10, 20);
    vi.stubGlobal('createImageBitmap', vi.fn().mockResolvedValue(picture));
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValue(new Response(new Uint8Array([1]), { status: 200, headers: { 'x-uoterm-anchor': '3,-4' } }));
    new ArtFeed(view as never).pump();
    await vi.waitFor(() => expect(view.artArrived).toHaveBeenCalledWith('item-7', 10, 20, 3, -4));
    expect(pixelsOf('item-7')).toBe(picture);
    expect(fetchSpy).toHaveBeenCalledWith('/v1/art', expect.objectContaining({ method: 'POST', body: JSON.stringify(request) }));
  });

  it('drops_the_pixels_the_view_forgot', async () => {
    const view = viewWanting([{ key: 'item-8', request: { kind: 'Item', graphic: 8 } }]);
    const picture = bitmap(1, 1);
    vi.stubGlobal('createImageBitmap', vi.fn().mockResolvedValue(picture));
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(new Uint8Array([1]), { status: 200, headers: { 'x-uoterm-anchor': '0,0' } }));
    const feed = new ArtFeed(view as never);
    feed.pump();
    await vi.waitFor(() => expect(pixelsOf('item-8')).toBe(picture));
    view.artForgotten.mockReturnValueOnce(['item-8']);
    feed.pump();
    expect(pixelsOf('item-8')).toBeUndefined();
    expect(picture.close).toHaveBeenCalled();
  });

  it('marks_a_picture_that_does_not_decode_as_missing', async () => {
    const view = viewWanting([{ key: 'bad', request: { kind: 'Item', graphic: 1 } }]);
    vi.stubGlobal('createImageBitmap', vi.fn().mockRejectedValue(new Error('not a picture')));
    vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response(new Uint8Array([1]), { status: 200 }));
    new ArtFeed(view as never).pump();
    await vi.waitFor(() => expect(view.artMissing).toHaveBeenCalledWith('bad'));
  });

  it('gives_data_answers_as_text_and_failures_as_missing', async () => {
    const view = viewWanting([], ['/v1/data/tiledata', '/v1/map/0/9999/9999']);
    vi.spyOn(globalThis, 'fetch').mockImplementation((path) =>
      Promise.resolve(path === '/v1/data/tiledata' ? new Response('{"land":[]}') : new Response(null, { status: 404 })),
    );
    new ArtFeed(view as never).pump();
    await vi.waitFor(() => expect(view.dataArrived).toHaveBeenCalledWith('/v1/data/tiledata', '{"land":[]}'));
    await vi.waitFor(() => expect(view.dataMissing).toHaveBeenCalledWith('/v1/map/0/9999/9999'));
  });

  it('posts_the_bodies_the_view_wants', async () => {
    const body = { text: 'Hail', look: 'plate' };
    const view = viewWanting([], [], [{ key: '5', path: '/v1/text/measure', body }]);
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValue(new Response('{"lines":["Hail"],"line_height":14}'));
    new ArtFeed(view as never).pump();
    await vi.waitFor(() => expect(view.postArrived).toHaveBeenCalledWith('5', '{"lines":["Hail"],"line_height":14}'));
    expect(fetchSpy).toHaveBeenCalledWith('/v1/text/measure', expect.objectContaining({ method: 'POST', body: JSON.stringify(body) }));
  });

  it('marks_a_post_that_never_reaches_the_server_as_missing_after_its_tries', async () => {
    vi.useFakeTimers();
    const view = viewWanting([], [], [{ key: '6', path: '/v1/map/live', body: {} }]);
    const fetchSpy = vi.spyOn(globalThis, 'fetch').mockRejectedValue(new TypeError('network'));
    new ArtFeed(view as never).pump();
    await vi.advanceTimersByTimeAsync(ALL_RETRY_WAITS_MS - 1);
    expect(view.postMissing).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(view.postMissing).toHaveBeenCalledWith('6');
    expect(fetchSpy).toHaveBeenCalledTimes(FETCH_TRIES);
  });

  it('tries_data_again_when_the_server_was_out_of_reach', async () => {
    vi.useFakeTimers();
    const view = viewWanting([], ['/v1/data/seasons']);
    vi.spyOn(globalThis, 'fetch')
      .mockRejectedValueOnce(new TypeError('network'))
      .mockResolvedValue(new Response('{"seasons":[]}'));
    new ArtFeed(view as never).pump();
    await vi.advanceTimersByTimeAsync(backoffWait(0));
    expect(view.dataArrived).toHaveBeenCalledWith('/v1/data/seasons', '{"seasons":[]}');
    expect(view.dataMissing).not.toHaveBeenCalled();
  });

  it('tries_a_picture_again_when_the_server_was_out_of_reach', async () => {
    vi.useFakeTimers();
    const view = viewWanting([{ key: 'again', request: { kind: 'Item', graphic: 2 } }]);
    vi.stubGlobal('createImageBitmap', vi.fn().mockResolvedValue(bitmap(4, 5)));
    vi.spyOn(globalThis, 'fetch')
      .mockRejectedValueOnce(new TypeError('network'))
      .mockResolvedValue(new Response(new Uint8Array([1]), { status: 200, headers: { 'x-uoterm-anchor': '1,2' } }));
    new ArtFeed(view as never).pump();
    await vi.advanceTimersByTimeAsync(backoffWait(0));
    expect(view.artArrived).toHaveBeenCalledWith('again', 4, 5, 1, 2);
    expect(view.artMissing).not.toHaveBeenCalled();
  });

  it('asks_for_the_token_and_waits_for_it_before_it_tries_again', async () => {
    vi.useFakeTimers();
    const tokenNeeded = vi.fn();
    const stop = whenTokenNeeded(tokenNeeded);
    const view = viewWanting([], ['/v1/data/cliloc']);
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockResolvedValueOnce(new Response('{"error":"unauthorized"}', { status: 401 }))
      .mockResolvedValueOnce(new Response(null, { status: 204 }))
      .mockResolvedValue(new Response('{"1":"a"}'));
    new ArtFeed(view as never).pump();
    await vi.advanceTimersByTimeAsync(0);
    expect(tokenNeeded).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(LONGEST_BACKOFF_MS * FETCH_TRIES);
    expect(fetchSpy).toHaveBeenCalledTimes(1);
    expect(view.dataMissing).not.toHaveBeenCalled();
    await giveToken('right');
    await vi.advanceTimersByTimeAsync(0);
    expect(view.dataArrived).toHaveBeenCalledWith('/v1/data/cliloc', '{"1":"a"}');
    stop();
  });

  it('stops_its_tries_and_tells_the_view_nothing_once_closed', async () => {
    vi.useFakeTimers();
    const view = viewWanting([], ['/v1/data/seasons', '/v1/data/tiledata']);
    let answer: (response: Response) => void = () => {};
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockRejectedValueOnce(new TypeError('network'))
      .mockReturnValueOnce(new Promise((resolve) => (answer = resolve)));
    const feed = new ArtFeed(view as never);
    feed.pump();
    await vi.advanceTimersByTimeAsync(0);
    feed.close();
    answer(new Response('{"land":[]}'));
    await vi.advanceTimersByTimeAsync(LONGEST_BACKOFF_MS * FETCH_TRIES);
    feed.pump();
    expect(fetchSpy).toHaveBeenCalledTimes(2);
    expect(view.dataArrived).not.toHaveBeenCalled();
    expect(view.dataMissing).not.toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('starts_a_waiting_request_when_one_ends', async () => {
    const wanted = Array.from({ length: ART_PARALLEL + 1 }, (_, i) => ({ key: `w${i}`, request: { kind: 'Item', graphic: i } }));
    const view = viewWanting(wanted);
    let endFirst: (answer: Response) => void = () => {};
    const fetchSpy = vi
      .spyOn(globalThis, 'fetch')
      .mockReturnValueOnce(new Promise((resolve) => (endFirst = resolve)))
      .mockReturnValue(new Promise(() => {}));
    new ArtFeed(view as never).pump();
    expect(fetchSpy).toHaveBeenCalledTimes(ART_PARALLEL);
    endFirst(new Response(null, { status: 404 }));
    await vi.waitFor(() => expect(fetchSpy).toHaveBeenCalledTimes(ART_PARALLEL + 1));
  });
});
