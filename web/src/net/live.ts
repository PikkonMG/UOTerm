/**
 * The live link of one session: the pictures of the session come in, the
 * tool calls of the page go out. A link that breaks opens again, waiting a
 * little longer after each failed try, while its session runs.
 */

import { api, readMessage, SESSIONS_PATH, socketUrl } from './api';
import { backoffWait } from './backoff';

// The limits of the server (crates/uoterm-runtime: `TOOL_CALL_TIMEOUT` of
// a session, `ACT_STEP_GAP_MS`, and `MAX_ACT_STEPS` and `ACT_PLACES` of
// the live link). They bound how long an answer can take.
/** The longest one tool call takes before the session gives up on it. */
const TOOL_CALL_MS = 8_000;
const ACT_STEP_GAP_MS = 650;
const MAX_ACT_STEPS = 4;
/** The act that runs and the four that may wait before it. */
const ACT_PLACES = 5;
/** The longest act: each step at its longest, with the gaps between them. */
const LONGEST_ACT_MS = MAX_ACT_STEPS * TOOL_CALL_MS + (MAX_ACT_STEPS - 1) * ACT_STEP_GAP_MS;
/** Room for the answer to reach the page (the server's send timeout). */
const ANSWER_TRAVEL_MS = 5_000;

/**
 * How long a call or an act waits for its answer: an act that waits
 * behind every other place, each at its longest, then runs at its longest
 * (about 175 s). Past it the server has dropped the answer, which it does
 * when the page reads too slowly.
 */
export const ANSWER_WAIT_MS = ACT_PLACES * LONGEST_ACT_MS + ANSWER_TRAVEL_MS;
/**
 * Failed tries to open the link in a row before the page asks the API
 * whether it wants its token and whether the session still runs; it asks
 * again after each as many more.
 */
export const LOSSES_BEFORE_TOKEN_CHECK = 3;
export const ANSWER_LATE = 'no answer came in time';
/** A call or an act sent while the link is lost; it never left the page. */
export const LINK_LOST = 'the link to UOTerm is lost';
/** The link broke while an answer was on its way; the server finishes an act it started. */
export const LINK_LOST_WAITING = 'the link was lost; the act may have run';
export const SESSION_ENDED = 'the session ended';

export type LinkState = 'open' | 'lost';

/** One tool call: the name of the tool and its arguments. */
export interface PageCall {
  tool: string;
  args: unknown;
}

/** What the page sends on the link. */
export type LiveOut =
  | ({ kind: 'call'; id: number } & PageCall)
  | { kind: 'act'; id: number; calls: PageCall[] }
  | { kind: 'size'; size: number };

/** The running sessions, as `GET /v1/sessions` gives them. */
interface Sessions {
  sessions: string[];
}

/** What the server sends on the link. */
export type LiveIn =
  | { kind: 'frame'; watch: unknown }
  | { kind: 'answer'; id: number; ok: boolean; result: unknown; error?: string }
  | { kind: 'ended' };

export interface LiveHandlers {
  /** A new picture of the session: the value of the `watch` tool. */
  frame(watch: unknown): void;
  /** The answer of a call or an act. A failure gives `{error: words}`. */
  answer(id: number, ok: boolean, result: unknown): void;
  /** The session is over; the link does not open again. */
  ended(): void;
  state(s: LinkState): void;
}

type Timer = ReturnType<typeof setTimeout>;

export class LiveLink {
  private socket: WebSocket;
  /** Failed tries since the link was last open. */
  private tries = 0;
  /** True from an open link until it is reported lost. */
  private reportedOpen = false;
  /** True once the page closed the link or the session ended. */
  private done = false;
  private retry: Timer | undefined;
  /** The timers of the calls and acts that wait for their answer, by id. */
  private readonly waiting = new Map<number, Timer>();
  /** The last size the page asked for, sent again on each new link. */
  private size: LiveOut | undefined;

  constructor(
    private readonly session: string,
    private readonly handlers: LiveHandlers,
  ) {
    this.socket = this.connect();
  }

  /**
   * Sends a message. A call or an act on a lost link fails at once; one
   * whose answer does not come in `ANSWER_WAIT_MS` fails then.
   */
  send(message: LiveOut): void {
    if (message.kind === 'size') {
      this.size = message;
      if (this.isOpen()) this.socket.send(JSON.stringify(message));
      return;
    }
    if (!this.isOpen()) {
      this.handlers.answer(message.id, false, { error: LINK_LOST });
      return;
    }
    const late = setTimeout(() => this.settle(message.id, false, { error: ANSWER_LATE }), ANSWER_WAIT_MS);
    this.waiting.set(message.id, late);
    this.socket.send(JSON.stringify(message));
  }

  /** Closes the link for good. Answers still on their way are let go. */
  close(): void {
    this.stop();
    for (const timer of this.waiting.values()) clearTimeout(timer);
    this.waiting.clear();
  }

  private connect(): WebSocket {
    const socket = new WebSocket(socketUrl(`/v1/sessions/${encodeURIComponent(this.session)}/live`));
    socket.onopen = () => {
      this.tries = 0;
      this.reportedOpen = true;
      this.handlers.state('open');
      if (this.size) socket.send(JSON.stringify(this.size));
    };
    socket.onmessage = (event: MessageEvent) => {
      const message = readMessage<LiveIn>(event.data);
      if (message) this.read(message);
    };
    socket.onclose = () => this.lost();
    return socket;
  }

  private read(message: LiveIn): void {
    switch (message.kind) {
      case 'frame':
        this.handlers.frame(message.watch);
        break;
      case 'answer':
        this.settle(message.id, message.ok, message.ok || message.error === undefined ? message.result : { error: message.error });
        break;
      case 'ended':
        this.end();
        break;
    }
  }

  /** The session is over: no more tries, and the page is told. */
  private end(): void {
    this.stop();
    this.failWaiting(SESSION_ENDED);
    this.handlers.ended();
  }

  /**
   * Asks the API for its sessions: a refusal for want of the token raises
   * it to the page; a session no longer listed ends the link, since the
   * link of a session that is gone never opens.
   */
  private checkSession(): void {
    api<Sessions>(SESSIONS_PATH).then(
      ({ sessions }) => {
        if (!this.done && !sessions.includes(this.session)) this.end();
      },
      () => undefined,
    );
  }

  /**
   * The link broke, or did not open: the answers it carried will not come;
   * try again. A link the API refuses shows no reason to the page, so after
   * a few tries in a row a call asks whether the API wants its token (the
   * call raises it to the page) and whether the session still runs.
   */
  private lost(): void {
    if (this.done) return;
    this.failWaiting(LINK_LOST_WAITING);
    if (this.reportedOpen) {
      this.reportedOpen = false;
      this.handlers.state('lost');
    }
    const wait = backoffWait(this.tries);
    this.tries += 1;
    if (this.tries % LOSSES_BEFORE_TOKEN_CHECK === 0) this.checkSession();
    this.retry = setTimeout(() => (this.socket = this.connect()), wait);
  }

  private settle(id: number, ok: boolean, result: unknown): void {
    const timer = this.waiting.get(id);
    if (timer === undefined) return;
    clearTimeout(timer);
    this.waiting.delete(id);
    this.handlers.answer(id, ok, result);
  }

  private failWaiting(words: string): void {
    for (const id of [...this.waiting.keys()]) this.settle(id, false, { error: words });
  }

  /** Stops the link without a report: no more tries, no more messages. */
  private stop(): void {
    this.done = true;
    clearTimeout(this.retry);
    this.socket.onopen = null;
    this.socket.onmessage = null;
    this.socket.onclose = null;
    this.socket.close();
  }

  private isOpen(): boolean {
    return this.socket.readyState === WebSocket.OPEN;
  }
}
