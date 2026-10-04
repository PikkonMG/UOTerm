/** Calls of the UOTerm API. The page is served by the API, so every path is same-origin. */

export const STATUS_UNAUTHORIZED = 401;
const STATUS_NO_CONTENT = 204;
export const METHOD_POST = 'POST';
export const METHOD_PUT = 'PUT';
/** The running sessions. Any screen may call it to learn whether the API wants its token. */
export const SESSIONS_PATH = '/v1/sessions';
/** Where the page trades the token for the cookie that carries it. */
const TOKEN_PATH = '/v1/web/token';
const JSON_TYPE = 'application/json';
const SECURE_PAGE_SCHEME = 'https:';
const SOCKET_SCHEME = 'ws:';
const SECURE_SOCKET_SCHEME = 'wss:';

/** The API wants the token: the page shows the token screen. */
export class TokenNeeded extends Error {
  constructor() {
    super('the API wants its token');
    this.name = 'TokenNeeded';
  }
}

/** The API refused a call: its status, and the words of its `error`. */
export class ApiFailed extends Error {
  constructor(
    readonly status: number,
    words: string,
  ) {
    super(words);
    this.name = 'ApiFailed';
  }
}

/** The listeners of one happening: each hears it until it stops. */
function happening() {
  const listeners = new Set<() => void>();
  return {
    listen(listener: () => void): () => void {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    tell(): void {
      for (const listener of [...listeners]) listener();
    },
  };
}

const tokenNeeded = happening();
const tokenGiven = happening();

/**
 * Calls `listener` each time the API wants its token, whoever learned it:
 * a call of `api`, the art feed, or the live link. Gives the function that
 * stops the calls.
 */
export function whenTokenNeeded(listener: () => void): () => void {
  return tokenNeeded.listen(listener);
}

/** Tells the page that the API wants its token. */
export function raiseTokenNeeded(): void {
  tokenNeeded.tell();
}

/**
 * Calls `listener` each time the API took a token, so what waited for it
 * goes on. Gives the function that stops the calls.
 */
export function whenTokenGiven(listener: () => void): () => void {
  return tokenGiven.listen(listener);
}

/** The options of a call that sends `body` as JSON. */
export function jsonInit(method: string, body: unknown): RequestInit {
  return { method, headers: { 'content-type': JSON_TYPE }, body: JSON.stringify(body) };
}

/**
 * The JSON answer of `path`. When the API wants the token it raises
 * `TokenNeeded` to the page and throws it; any other refusal throws
 * `ApiFailed`. An answer with no content gives `undefined`.
 */
export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, init);
  if (response.status === STATUS_UNAUTHORIZED) {
    raiseTokenNeeded();
    throw new TokenNeeded();
  }
  if (!response.ok) throw new ApiFailed(response.status, await errorWords(response));
  if (response.status === STATUS_NO_CONTENT) return undefined as T;
  return (await response.json()) as T;
}

/**
 * Gives the token to the API, which answers with the cookie the browser
 * shows on every later call, the live link too. False for a wrong token.
 */
export async function giveToken(token: string): Promise<boolean> {
  try {
    await api<void>(TOKEN_PATH, jsonInit(METHOD_POST, { token }));
    tokenGiven.tell();
    return true;
  } catch (error) {
    if (error instanceof TokenNeeded) return false;
    throw error;
  }
}

/** The WebSocket address of `path` on the API that served the page. */
export function socketUrl(path: string): string {
  const scheme = location.protocol === SECURE_PAGE_SCHEME ? SECURE_SOCKET_SCHEME : SOCKET_SCHEME;
  return `${scheme}//${location.host}${path}`;
}

/** A message of a link read from its JSON text; undefined when it does not read. */
export function readMessage<T>(data: unknown): T | undefined {
  if (typeof data !== 'string') return undefined;
  try {
    return JSON.parse(data) as T;
  } catch {
    return undefined;
  }
}

/** The `error` words of a refusal, else its status text. */
async function errorWords(response: Response): Promise<string> {
  const { error } = readMessage<{ error?: unknown }>(await response.text()) ?? {};
  return typeof error === 'string' ? error : response.statusText || `status ${response.status}`;
}
