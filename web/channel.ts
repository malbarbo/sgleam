// Shared buffer layout (Int32Array indices):
//   0: STOP_INDEX            - interrupt flag (main → worker)
//   1: SLEEP_INDEX           - sleep/wake signal (main → worker)
//   2: KEY_EVENTS_LOCK_INDEX - spinlock for key event queue (0 = unlocked, 1 = locked)
//   3: NUM_KEY_EVENTS_INDEX  - number of events queued
//   4+: event slots (EVENT_SIZE int32s each)

const STOP_INDEX = 0;
const SLEEP_INDEX = 1;
const KEY_EVENTS_LOCK_INDEX = 2;
const NUM_KEY_EVENTS_INDEX = 3;
const HEADER_SIZE = 4;
const EVENT_KEY_LEN = 12;
const EVENT_SIZE = 1 + EVENT_KEY_LEN + 5;

export const KEYPRESS = 0;
export const KEYDOWN = 1;
export const KEYUP = 2;

export interface KeyEvent {
  type: number;
  key: string;
  alt: boolean;
  ctrl: boolean;
  shift: boolean;
  meta: boolean;
  repeat: boolean;
}

export type WorkerMessage =
  | { cmd: "ready" }
  | { cmd: "error"; data: string }
  | { cmd: "progress"; data: number }
  | { cmd: "output"; fd: number; data: string }
  | { cmd: "format"; data: string }
  | { cmd: "svg"; data: string };

// --- UI channel (main thread side) ---

export class UIChannel {
  private buffer: Int32Array;

  constructor(capacity: number = 10) {
    const byteLength = (HEADER_SIZE + EVENT_SIZE * capacity) * 4;
    this.buffer = new Int32Array(new SharedArrayBuffer(byteLength));
  }

  getBuffer(): SharedArrayBuffer {
    return this.buffer.buffer as SharedArrayBuffer;
  }

  stop(): void {
    Atomics.store(this.buffer, STOP_INDEX, 1);
    Atomics.notify(this.buffer, SLEEP_INDEX, 1);
  }

  enqueueKeyEvent(event: KeyEvent): boolean {
    lock(this.buffer);
    try {
      const count = this.buffer[NUM_KEY_EVENTS_INDEX];
      const capacity = Math.floor((this.buffer.length - HEADER_SIZE) / EVENT_SIZE);
      if (count >= capacity) {
        return false;
      }
      const offset = HEADER_SIZE + count * EVENT_SIZE;
      this.buffer[offset] = event.type;
      writeKey(this.buffer, offset + 1, event.key);
      this.buffer[offset + 1 + EVENT_KEY_LEN + 0] = event.alt ? 1 : 0;
      this.buffer[offset + 1 + EVENT_KEY_LEN + 1] = event.ctrl ? 1 : 0;
      this.buffer[offset + 1 + EVENT_KEY_LEN + 2] = event.shift ? 1 : 0;
      this.buffer[offset + 1 + EVENT_KEY_LEN + 3] = event.meta ? 1 : 0;
      this.buffer[offset + 1 + EVENT_KEY_LEN + 4] = event.repeat ? 1 : 0;
      this.buffer[NUM_KEY_EVENTS_INDEX] = count + 1;
      return true;
    } finally {
      unlock(this.buffer);
    }
  }
}

// --- Worker channel (worker side) ---

export class WorkerChannel {
  private buffer!: Int32Array;

  init(sharedBuffer: SharedArrayBuffer): void {
    this.buffer = new Int32Array(sharedBuffer);
  }

  checkInterrupt(): boolean {
    return Atomics.exchange(this.buffer, STOP_INDEX, 0) !== 0;
  }

  sleep(ms: bigint): void {
    Atomics.wait(this.buffer, SLEEP_INDEX, 0, Number(ms));
  }

  dequeueKeyEvent(): KeyEvent | null {
    lock(this.buffer);
    try {
      const count = this.buffer[NUM_KEY_EVENTS_INDEX];
      if (count === 0) {
        return null;
      }
      const type = this.buffer[HEADER_SIZE];
      const key = readKey(this.buffer, HEADER_SIZE + 1);
      const alt = !!this.buffer[HEADER_SIZE + 1 + EVENT_KEY_LEN + 0];
      const ctrl = !!this.buffer[HEADER_SIZE + 1 + EVENT_KEY_LEN + 1];
      const shift = !!this.buffer[HEADER_SIZE + 1 + EVENT_KEY_LEN + 2];
      const meta = !!this.buffer[HEADER_SIZE + 1 + EVENT_KEY_LEN + 3];
      const repeat = !!this.buffer[HEADER_SIZE + 1 + EVENT_KEY_LEN + 4];
      const remaining = (count - 1) * EVENT_SIZE;
      this.buffer.copyWithin(
        HEADER_SIZE,
        HEADER_SIZE + EVENT_SIZE,
        HEADER_SIZE + EVENT_SIZE + remaining,
      );
      this.buffer.fill(
        0,
        HEADER_SIZE + remaining,
        HEADER_SIZE + remaining + EVENT_SIZE,
      );
      this.buffer[NUM_KEY_EVENTS_INDEX] = count - 1;
      return { type, key, alt, ctrl, shift, meta, repeat };
    } finally {
      unlock(this.buffer);
    }
  }

  ready(): void { workerPost({ cmd: "ready" }); }
  error(data: string): void { workerPost({ cmd: "error", data }); }
  progress(data: number): void { workerPost({ cmd: "progress", data }); }
  write(fd: number, data: string): void { workerPost({ cmd: "output", fd, data }); }
  format(data: string): void { workerPost({ cmd: "format", data }); }
  svg(data: string): void { workerPost({ cmd: "svg", data }); }
}

function workerPost(msg: WorkerMessage): void {
  // deno-lint-ignore no-explicit-any
  (self as any).postMessage(msg);
}

// --- Helpers ---

function lock(mem: Int32Array): void {
  while (Atomics.compareExchange(mem, KEY_EVENTS_LOCK_INDEX, 0, 1) !== 0) {}
}

function unlock(mem: Int32Array): void {
  Atomics.store(mem, KEY_EVENTS_LOCK_INDEX, 0);
}

function writeKey(mem: Int32Array, offset: number, key: string): void {
  for (let i = 0; i < EVENT_KEY_LEN; i++) {
    mem[offset + i] = i < key.length ? key.codePointAt(i)! : 0;
  }
}

function readKey(mem: Int32Array, offset: number): string {
  const chars: number[] = [];
  for (let i = 0; i < EVENT_KEY_LEN; i++) {
    const code = mem[offset + i];
    if (code === 0) break;
    chars.push(code);
  }
  return String.fromCodePoint(...chars);
}
