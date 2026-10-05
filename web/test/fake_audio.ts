/**
 * A Web Audio context with no sound device, for the tests: it keeps the
 * sources it started and the gains they go through, decodes any bytes to
 * a buffer of their length, and is suspended until resumed.
 */

/** The sample rate of the fake context. */
export const FAKE_RATE = 48_000;

export class FakeParam {
  value = 1;
}

export class FakeGain {
  readonly gain = new FakeParam();
  connected: unknown = null;
  connect(to: unknown): unknown {
    this.connected = to;
    return to;
  }
  disconnect(): void {
    this.connected = null;
  }
}

export class FakeSource {
  buffer: FakeBuffer | null = null;
  loop = false;
  started = false;
  stopped = false;
  output: FakeGain | null = null;
  onended: (() => void) | null = null;
  connect(to: FakeGain): FakeGain {
    this.output = to;
    return to;
  }
  start(): void {
    this.started = true;
  }
  stop(): void {
    this.stopped = true;
  }
  /** The sound came to its end, as the browser tells it. */
  end(): void {
    this.onended?.();
  }
}

export class FakeBuffer {
  readonly channels: Float32Array[];
  constructor(
    readonly numberOfChannels: number,
    readonly length: number,
    readonly sampleRate: number,
  ) {
    this.channels = Array.from({ length: numberOfChannels }, () => new Float32Array(length));
  }
  copyToChannel(samples: Float32Array, channel: number): void {
    this.channels[channel].set(samples);
  }
}

export class FakeAudioContext {
  readonly destination = {};
  readonly sampleRate = FAKE_RATE;
  state: 'suspended' | 'running' | 'closed' = 'suspended';
  readonly sources: FakeSource[] = [];
  readonly decoded: ArrayBuffer[] = [];

  createGain(): FakeGain {
    return new FakeGain();
  }
  createBufferSource(): FakeSource {
    const source = new FakeSource();
    this.sources.push(source);
    return source;
  }
  createBuffer(channels: number, length: number, rate: number): FakeBuffer {
    return new FakeBuffer(channels, length, rate);
  }
  decodeAudioData(bytes: ArrayBuffer): Promise<FakeBuffer> {
    this.decoded.push(bytes);
    return Promise.resolve(new FakeBuffer(1, bytes.byteLength, this.sampleRate));
  }
  resume(): Promise<void> {
    this.state = 'running';
    return Promise.resolve();
  }
  close(): Promise<void> {
    this.state = 'closed';
    return Promise.resolve();
  }
  /** The sources that started and have not stopped. */
  playing(): FakeSource[] {
    return this.sources.filter((source) => source.started && !source.stopped);
  }
}

/** The context as the page's code takes it. */
export const asContext = (fake: FakeAudioContext) => fake as unknown as AudioContext;
