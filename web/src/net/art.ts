/**
 * Feeds the view the pictures, tables and measures it asks for. The view
 * names each want once; the feed fetches it, with at most `ART_PARALLEL`
 * requests in flight, and gives the answer back.
 *
 * The view takes "missing" as final, so only an answer of the server says
 * it: a request that does not reach the server is tried again, after the
 * waits of `BACKOFF_MS`, up to `FETCH_TRIES` times. A request the API
 * refuses for want of its token raises `TokenNeeded` to the page and is
 * tried again, every `LONGEST_BACKOFF_MS`, until the token is given.
 *
 * Browsers do not cache the answers of `POST /v1/art`, so the feed keeps
 * the pixels of each picture by the key the view gave, until the view
 * forgets the picture.
 */

import { jsonInit, METHOD_POST, raiseTokenNeeded, STATUS_UNAUTHORIZED } from './api';
import { backoffWait, LONGEST_BACKOFF_MS } from './backoff';

export const ART_PARALLEL = 8;
/** Tries of a request that does not reach the server, before it counts as missing. */
export const FETCH_TRIES = 4;
const ART_PATH = '/v1/art';
/** The point of a picture that goes on its tile: `x,y`. */
const ANCHOR_HEADER = 'x-uoterm-anchor';
const ANCHOR_SEPARATOR = ',';
const NO_ANCHOR = 0;

/** A picture to post to `/v1/art`: the request is an `ArtRequest`. */
export interface ArtWant {
  key: string;
  request: unknown;
}

/** A body to post to `path`. */
export interface PostWant {
  key: string;
  path: string;
  body: unknown;
}

/** The methods of the `WebView` the feed uses. */
export interface FeedView {
  artWanted(): ArtWant[];
  artArrived(key: string, width: number, height: number, anchorX: number, anchorY: number): void;
  artMissing(key: string): void;
  artForgotten(): string[];
  dataWanted(): string[];
  dataArrived(path: string, json: string): void;
  dataMissing(path: string): void;
  postsWanted(): PostWant[];
  postArrived(key: string, json: string): void;
  postMissing(key: string): void;
}

/** One request of the feed: what to fetch, and what to do with the answer. */
interface FeedRequest {
  path: string;
  init?: RequestInit;
  /** The server answered with an OK status and this body. */
  arrived(body: Blob, headers: Headers): Promise<void>;
  /** The server has none (any other status), the body does not read, or the server is out of reach. */
  missing(): void;
}

/** A request and the tries it failed to reach the server. */
interface Queued {
  request: FeedRequest;
  failedTries: number;
}

/** The pixels of each picture the view has, by its key. */
const pictures = new Map<string, ImageBitmap>();

/** The pixels of the picture of `key`, once they came. */
export function pixelsOf(key: string): ImageBitmap | undefined {
  return pictures.get(key);
}

export class ArtFeed {
  /** Requests that wait for a place, in the order the view named them. */
  private readonly waiting: Queued[] = [];
  private inFlight = 0;

  constructor(private readonly view: FeedView) {}

  /** Takes what the view wants now and starts what has a place. Call it once a frame. */
  pump(): void {
    for (const key of this.view.artForgotten()) forget(key);
    for (const path of this.view.dataWanted()) this.queue(this.dataRequest(path));
    for (const post of this.view.postsWanted()) this.queue(this.postRequest(post));
    for (const want of this.view.artWanted()) this.queue(this.artRequest(want));
    this.startWaiting();
  }

  private queue(request: FeedRequest, failedTries = 0): void {
    this.waiting.push({ request, failedTries });
  }

  private startWaiting(): void {
    while (this.inFlight < ART_PARALLEL) {
      const next = this.waiting.shift();
      if (!next) return;
      this.inFlight += 1;
      void this.run(next).finally(() => {
        this.inFlight -= 1;
        this.startWaiting();
      });
    }
  }

  private async run({ request, failedTries }: Queued): Promise<void> {
    let answer: { response: Response; body: Blob };
    try {
      const response = await fetch(request.path, request.init);
      answer = { response, body: await response.blob() };
    } catch {
      const tried = failedTries + 1;
      if (tried < FETCH_TRIES) this.later(request, tried, backoffWait(failedTries));
      else request.missing();
      return;
    }
    const { response, body } = answer;
    if (response.status === STATUS_UNAUTHORIZED) {
      raiseTokenNeeded();
      this.later(request, failedTries, LONGEST_BACKOFF_MS);
      return;
    }
    if (!response.ok) {
      request.missing();
      return;
    }
    try {
      await request.arrived(body, response.headers);
    } catch {
      request.missing();
    }
  }

  /** Queues `request` again after `wait`, without a place while it waits. */
  private later(request: FeedRequest, failedTries: number, wait: number): void {
    setTimeout(() => {
      this.queue(request, failedTries);
      this.startWaiting();
    }, wait);
  }

  private artRequest(want: ArtWant): FeedRequest {
    return {
      path: ART_PATH,
      init: jsonInit(METHOD_POST, want.request),
      arrived: async (body, headers) => {
        const picture = await createImageBitmap(body);
        forget(want.key);
        pictures.set(want.key, picture);
        const [anchorX, anchorY] = anchorOf(headers.get(ANCHOR_HEADER));
        this.view.artArrived(want.key, picture.width, picture.height, anchorX, anchorY);
      },
      missing: () => this.view.artMissing(want.key),
    };
  }

  private dataRequest(path: string): FeedRequest {
    return {
      path,
      arrived: async (body) => this.view.dataArrived(path, await body.text()),
      missing: () => this.view.dataMissing(path),
    };
  }

  private postRequest(want: PostWant): FeedRequest {
    return {
      path: want.path,
      init: jsonInit(METHOD_POST, want.body),
      arrived: async (body) => this.view.postArrived(want.key, await body.text()),
      missing: () => this.view.postMissing(want.key),
    };
  }
}

/** Lets the pixels of `key` go. */
function forget(key: string): void {
  pictures.get(key)?.close();
  pictures.delete(key);
}

/** The anchor of an `x,y` header; a missing or bad part is 0. */
function anchorOf(header: string | null): [number, number] {
  const [x, y] = (header ?? '').split(ANCHOR_SEPARATOR).map(Number);
  return [Number.isFinite(x) ? x : NO_ANCHOR, Number.isFinite(y) ? y : NO_ANCHOR];
}
