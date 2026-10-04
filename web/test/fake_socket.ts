import { vi } from 'vitest';

/** A stand-in for the browser `WebSocket` the tests drive by hand. */
export class FakeSocket {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;
  /** Every socket made since `install`, the newest last. */
  static made: FakeSocket[] = [];

  readyState = FakeSocket.CONNECTING;
  readonly sent: string[] = [];
  onopen: ((event: Event) => void) | null = null;
  onclose: ((event: CloseEvent) => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;

  constructor(readonly url: string) {
    FakeSocket.made.push(this);
  }

  /** Puts the fake in place of `WebSocket` until `vi.unstubAllGlobals`. */
  static install(): void {
    FakeSocket.made = [];
    vi.stubGlobal('WebSocket', FakeSocket);
  }

  static last(): FakeSocket {
    const socket = FakeSocket.made.at(-1);
    if (!socket) throw new Error('no socket was made');
    return socket;
  }

  /** The server took the socket. */
  open(): void {
    this.readyState = FakeSocket.OPEN;
    this.onopen?.(new Event('open'));
  }

  /** The link broke, or the server closed it. */
  drop(): void {
    this.readyState = FakeSocket.CLOSED;
    this.onclose?.(new Event('close') as CloseEvent);
  }

  /** The server sent `message` as JSON. */
  receive(message: unknown): void {
    this.onmessage?.(new MessageEvent('message', { data: JSON.stringify(message) }));
  }

  /** The messages the page sent, read back from JSON. */
  sentJson(): unknown[] {
    return this.sent.map((text) => JSON.parse(text) as unknown);
  }

  send(data: string): void {
    this.sent.push(data);
  }

  close(): void {
    if (this.readyState !== FakeSocket.CLOSED) this.drop();
  }
}
