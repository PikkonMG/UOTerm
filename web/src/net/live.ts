/**
 * The live link of one session: the pictures of the session come in, the
 * tool calls of the page go out. A link that breaks opens again, waiting a
 * little longer after each failed try.
 */

import { socketUrl } from './api';

/** The waits between tries to open a lost link. The last one repeats. */
export const RECONNECT_MS = [250, 500, 1000, 2000, 4000];
/**
 * How long a call or an act may wait for its answer. The server drops an
 * answer when the page reads too slowly, and an act may wait behind four
 * others of four steps each (about 10 s), so this leaves room.
 */
export const ANSWER_WAIT_MS = 30_000;
export const ANSWER_LATE = 'no answer came in time';
export const LINK_LOST = 'the link to UOTerm is lost';
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
    socket.onmessage = (event: MessageEvent) => this.read(JSON.parse(event.data as string) as LiveIn);
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
        this.stop();
        this.failWaiting(SESSION_ENDED);
        this.handlers.ended();
        break;
    }
  }

  /** The link broke: the answers it carried will not come; try again. */
  private lost(): void {
    if (this.done) return;
    this.failWaiting(LINK_LOST);
    if (this.reportedOpen) {
      this.reportedOpen = false;
      this.handlers.state('lost');
    }
    const wait = RECONNECT_MS[Math.min(this.tries, RECONNECT_MS.length - 1)];
    this.tries += 1;
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
