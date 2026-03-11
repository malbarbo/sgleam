import {
    EVENT_KEY_LEN,
    EVENT_SIZE,
    HEADER_SIZE,
    type KeyEvent,
    lock,
    NUM_KEY_EVENTS_INDEX,
    readKey,
    SLEEP_INDEX,
    STOP_INDEX,
    unlock,
    type WorkerMessage,
} from "./ui_channel.ts";

export class WorkerChannel {
    private buffer: Int32Array;

    constructor(capacity: number = 10) {
        const byteLength = (HEADER_SIZE + EVENT_SIZE * capacity) * 4;
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

    ready(): void {
        workerPost({
            cmd: "ready",
            buffer: this.buffer.buffer as SharedArrayBuffer,
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
