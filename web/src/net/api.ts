/** Calls of the UOTerm API. The page is served by the API, so every path is same-origin. */

const STATUS_UNAUTHORIZED = 401;
const STATUS_NO_CONTENT = 204;
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

/** The options of a call that sends `body` as JSON. */
export function jsonInit(method: string, body: unknown): RequestInit {
  return { method, headers: { 'content-type': JSON_TYPE }, body: JSON.stringify(body) };
}

/**
 * The JSON answer of `path`. Throws `TokenNeeded` when the API wants the
 * token, and `ApiFailed` for any other refusal. An answer with no content
 * gives `undefined`.
 */
export async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, init);
  if (response.status === STATUS_UNAUTHORIZED) throw new TokenNeeded();
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
    await api<void>(TOKEN_PATH, jsonInit('POST', { token }));
    return true;
  } catch (error) {
    if (error instanceof TokenNeeded) return false;
    throw error;
  }
}

/** The `error` words of a refusal, else its status text. */
async function errorWords(response: Response): Promise<string> {
  const text = await response.text();
  try {
    const { error } = JSON.parse(text) as { error?: unknown };
    if (typeof error === 'string') return error;
  } catch {
    // Not JSON: the status says it.
  }
  return response.statusText || `status ${response.status}`;
}

/** The WebSocket address of `path` on the API that served the page. */
export function socketUrl(path: string): string {
  const scheme = location.protocol === SECURE_PAGE_SCHEME ? SECURE_SOCKET_SCHEME : SOCKET_SCHEME;
  return `${scheme}//${location.host}${path}`;
}
