// Shared buffer layout (Int32Array indices):
//   0: STOP_INDEX            - interrupt flag (main → worker)
//   1: SLEEP_INDEX           - sleep/wake signal (main → worker)
//   2: KEY_EVENTS_LOCK_INDEX - spinlock for key event queue (0 = unlocked, 1 = locked)
//   3: NUM_KEY_EVENTS_INDEX  - number of events queued
//   4+: event slots (EVENT_SIZE int32s each)

export const STOP_INDEX = 0;
export const SLEEP_INDEX = 1;
export const KEY_EVENTS_LOCK_INDEX = 2;
export const NUM_KEY_EVENTS_INDEX = 3;
export const HEADER_SIZE = 4;
export const EVENT_KEY_LEN = 12;
export const EVENT_SIZE = 1 + EVENT_KEY_LEN + 5;

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

export type UIMessage =
    | { cmd: "load"; data: string }
    | { cmd: "run"; data: string }
    | { cmd: "format"; data: string };

export type WorkerMessage =
    | { cmd: "ready"; buffer: SharedArrayBuffer; hadErrors: boolean }
    | { cmd: "error"; data: string }
    | { cmd: "progress"; data: number }
    | { cmd: "write"; fd: number; data: string }
    | { cmd: "formatted"; data: string }
    | { cmd: "svg"; data: string };

interface Postable {
    postMessage(msg: UIMessage): void;
}

export class UIChannel {
    private buffer: Int32Array | null = null;
    private readonly worker: Postable;

    constructor(worker: Postable) {
        this.worker = worker;
    }

    setBuffer(buf: SharedArrayBuffer): void {
        this.buffer = new Int32Array(buf);
    }

    load(data: string): void {
        this.worker.postMessage({ cmd: "load", data });
    }
    run(data: string): void {
        this.worker.postMessage({ cmd: "run", data });
    }
    format(data: string): void {
        this.worker.postMessage({ cmd: "format", data });
    }

    // stop() writes directly to the shared buffer instead of using postMessage,
    // because the worker blocks on Atomics.wait() during sleep and cannot process
    // messages while waiting. The atomic store wakes it immediately via notify.
    stop(): void {
        const buf = this.buffer!;
        Atomics.store(buf, STOP_INDEX, 1);
        Atomics.notify(buf, SLEEP_INDEX, 1);
    }

    enqueueKeyEvent(event: KeyEvent): boolean {
        const buf = this.buffer!;
        lock(buf);
        try {
            const count = buf[NUM_KEY_EVENTS_INDEX];
            const capacity = Math.floor(
                (buf.length - HEADER_SIZE) / EVENT_SIZE,
            );
            if (count >= capacity) {
                return false;
            }
            const offset = HEADER_SIZE + count * EVENT_SIZE;
            buf[offset] = event.type;
            writeKey(buf, offset + 1, event.key);
            buf[offset + 1 + EVENT_KEY_LEN + 0] = event.alt ? 1 : 0;
            buf[offset + 1 + EVENT_KEY_LEN + 1] = event.ctrl ? 1 : 0;
            buf[offset + 1 + EVENT_KEY_LEN + 2] = event.shift ? 1 : 0;
            buf[offset + 1 + EVENT_KEY_LEN + 3] = event.meta ? 1 : 0;
            buf[offset + 1 + EVENT_KEY_LEN + 4] = event.repeat ? 1 : 0;
            buf[NUM_KEY_EVENTS_INDEX] = count + 1;
            return true;
        } finally {
            unlock(buf);
        }
    }
}

// --- Helpers shared with worker_channel.ts ---

export function lock(mem: Int32Array): void {
    while (Atomics.compareExchange(mem, KEY_EVENTS_LOCK_INDEX, 0, 1) !== 0) {}
}

export function unlock(mem: Int32Array): void {
    Atomics.store(mem, KEY_EVENTS_LOCK_INDEX, 0);
}

export function writeKey(mem: Int32Array, offset: number, key: string): void {
    for (let i = 0; i < EVENT_KEY_LEN; i++) {
        mem[offset + i] = i < key.length ? key.codePointAt(i)! : 0;
    }
}

export function readKey(mem: Int32Array, offset: number): string {
    const chars: number[] = [];
    for (let i = 0; i < EVENT_KEY_LEN; i++) {
        const code = mem[offset + i];
        if (code === 0) break;
        chars.push(code);
    }
    return String.fromCodePoint(...chars);
}
