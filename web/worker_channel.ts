import {
    EVENT_KEY_LEN,
    EVENT_SIZE,
    HEADER_SIZE,
    INPUT_DATA_INDEX,
    INPUT_LEN_INDEX,
    INPUT_MAX_BYTES,
    INPUT_READY_INDEX,
    KEY_EVENT_CAPACITY,
    type KeyEvent,
    lock,
    NUM_KEY_EVENTS_INDEX,
    readKey,
    SLEEP_INDEX,
    STOP_INDEX,
    TOTAL_INT32S,
    unlock,
    type WorkerMessage,
} from "./ui_channel.ts";

export class WorkerChannel {
    private buffer: Int32Array;

    constructor(capacity: number = KEY_EVENT_CAPACITY) {
        const minInt32s = HEADER_SIZE + EVENT_SIZE * capacity + 2 +
            Math.ceil(INPUT_MAX_BYTES / 4);
        const byteLength = Math.max(minInt32s, TOTAL_INT32S) * 4;
        this.buffer = new Int32Array(new SharedArrayBuffer(byteLength));
    }

    getBuffer(): SharedArrayBuffer {
        return this.buffer.buffer as SharedArrayBuffer;
    }

    checkInterrupt(): boolean {
        return Atomics.exchange(this.buffer, STOP_INDEX, 0) !== 0;
    }

    sleep(ms: bigint): void {
        Atomics.wait(this.buffer, SLEEP_INDEX, 0, Number(ms));
    }

    waitForInput(): string {
        workerPost({ cmd: "input" });
        Atomics.wait(this.buffer, INPUT_READY_INDEX, 0);
        const len = this.buffer[INPUT_LEN_INDEX];
        const byteView = new Uint8Array(
            this.buffer.buffer,
            INPUT_DATA_INDEX * 4,
            len,
        );
        const text = new TextDecoder().decode(byteView.slice(0, len));
        Atomics.store(this.buffer, INPUT_READY_INDEX, 0);
        Atomics.store(this.buffer, INPUT_LEN_INDEX, 0);
        return text;
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

    ready(hadErrors = false, version = ""): void {
        workerPost({
            cmd: "ready",
            buffer: this.buffer.buffer as SharedArrayBuffer,
            hadErrors,
            version,
        });
    }
    error(data: string): void {
        workerPost({ cmd: "error", data });
    }
    progress(data: number): void {
        workerPost({ cmd: "progress", data });
    }
    write(fd: number, data: string): void {
        workerPost({ cmd: "write", fd, data });
    }
    formatted(data: string): void {
        workerPost({ cmd: "formatted", data });
    }
    svg(data: string): void {
        workerPost({ cmd: "svg", data });
    }
}

function workerPost(msg: WorkerMessage): void {
    // deno-lint-ignore no-explicit-any
    (self as any).postMessage(msg);
}
