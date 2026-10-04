/**
 * Feeds the view the pictures, tables and measures it asks for. The view
 * names each want once; the feed fetches it, with at most `ART_PARALLEL`
 * requests in flight, and gives the answer back.
 *
 * Browsers do not cache the answers of `POST /v1/art`, so the feed keeps
 * the pixels of each picture by the key the view gave, until the view
 * forgets the picture.
 */

import { jsonInit } from './api';

export const ART_PARALLEL = 8;
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

/** The pixels of each picture the view has, by its key. */
const pictures = new Map<string, ImageBitmap>();

/** The pixels of the picture of `key`, once they came. */
export function pixelsOf(key: string): ImageBitmap | undefined {
  return pictures.get(key);
}

export class ArtFeed {
  /** Requests that wait for a place, in the order the view named them. */
  private readonly waiting: (() => Promise<void>)[] = [];
  private inFlight = 0;

  constructor(private readonly view: FeedView) {}

  /** Takes what the view wants now and starts what has a place. Call it once a frame. */
  pump(): void {
    for (const key of this.view.artForgotten()) forget(key);
    for (const path of this.view.dataWanted()) this.waiting.push(() => this.getData(path));
    for (const post of this.view.postsWanted()) this.waiting.push(() => this.post(post));
    for (const want of this.view.artWanted()) this.waiting.push(() => this.getArt(want));
    this.startWaiting();
  }

  private startWaiting(): void {
    while (this.inFlight < ART_PARALLEL) {
      const request = this.waiting.shift();
      if (!request) return;
      this.inFlight += 1;
      void request().finally(() => {
        this.inFlight -= 1;
        this.startWaiting();
      });
    }
  }

  private async getArt(want: ArtWant): Promise<void> {
    const arrived = await fetchPicture(want.request);
    if (!arrived) {
      this.view.artMissing(want.key);
      return;
    }
    forget(want.key);
    pictures.set(want.key, arrived.picture);
    const [anchorX, anchorY] = arrived.anchor;
    this.view.artArrived(want.key, arrived.picture.width, arrived.picture.height, anchorX, anchorY);
  }

  private async getData(path: string): Promise<void> {
    const text = await fetchText(path);
    if (text === undefined) this.view.dataMissing(path);
    else this.view.dataArrived(path, text);
  }

  private async post(want: PostWant): Promise<void> {
    const text = await fetchText(want.path, jsonInit('POST', want.body));
    if (text === undefined) this.view.postMissing(want.key);
    else this.view.postArrived(want.key, text);
  }
}

/** Lets the pixels of `key` go. */
function forget(key: string): void {
  pictures.get(key)?.close();
  pictures.delete(key);
}

/** The decoded picture of `request` and its anchor; undefined when the server has none or it does not decode. */
async function fetchPicture(request: unknown): Promise<{ picture: ImageBitmap; anchor: [number, number] } | undefined> {
  try {
    const response = await fetch(ART_PATH, jsonInit('POST', request));
    if (!response.ok) return undefined;
    const picture = await createImageBitmap(await response.blob());
    return { picture, anchor: anchorOf(response.headers.get(ANCHOR_HEADER)) };
  } catch {
    return undefined;
  }
}

/** The body of `path` as text; undefined when the call fails. */
async function fetchText(path: string, init?: RequestInit): Promise<string | undefined> {
  try {
    const response = await fetch(path, init);
    return response.ok ? await response.text() : undefined;
  } catch {
    return undefined;
  }
}

/** The anchor of an `x,y` header; a missing or bad part is 0. */
function anchorOf(header: string | null): [number, number] {
  const [x, y] = (header ?? '').split(ANCHOR_SEPARATOR).map(Number);
  return [Number.isFinite(x) ? x : NO_ANCHOR, Number.isFinite(y) ? y : NO_ANCHOR];
}
