import { assertEquals } from "jsr:@std/assert";
import {
    KEYDOWN,
    type KeyEvent,
    KEYPRESS,
    KEYUP,
    UIChannel,
} from "./ui_channel.ts";
import { WorkerChannel } from "./worker_channel.ts";

function makeChannels(capacity = 4): [UIChannel, WorkerChannel] {
    const worker = new WorkerChannel(capacity);
    const ui = new UIChannel({ postMessage() {} });
    ui.setBuffer(worker.getBuffer());
    return [ui, worker];
}

function event(overrides: Partial<KeyEvent> = {}): KeyEvent {
    return {
        type: KEYPRESS,
        key: "a",
        alt: false,
        ctrl: false,
        shift: false,
        meta: false,
        repeat: false,
        ...overrides,
    };
}

Deno.test("dequeue from empty buffer returns null", () => {
    const [, worker] = makeChannels();
    assertEquals(worker.dequeueKeyEvent(), null);
});

Deno.test("enqueue then dequeue returns the same event", () => {
    const [ui, worker] = makeChannels();
    const e = event();
    assertEquals(ui.enqueueKeyEvent(e), true);
    assertEquals(worker.dequeueKeyEvent(), e);
    assertEquals(worker.dequeueKeyEvent(), null);
});

Deno.test("events are dequeued in FIFO order", () => {
    const [ui, worker] = makeChannels();
    const a = event({ type: KEYDOWN, key: "a" });
    const b = event({ type: KEYUP, key: "b" });
    ui.enqueueKeyEvent(a);
    ui.enqueueKeyEvent(b);
    assertEquals(worker.dequeueKeyEvent(), a);
    assertEquals(worker.dequeueKeyEvent(), b);
    assertEquals(worker.dequeueKeyEvent(), null);
});

Deno.test("modifier flags are preserved", () => {
    const [ui, worker] = makeChannels();
    const e = event({
        type: KEYDOWN,
        key: "x",
        alt: true,
        ctrl: true,
        shift: true,
        meta: true,
        repeat: true,
    });
    ui.enqueueKeyEvent(e);
    assertEquals(worker.dequeueKeyEvent(), e);
});

Deno.test("enqueue returns false when buffer is full", () => {
    const [ui] = makeChannels(2);
    const e = event();
    assertEquals(ui.enqueueKeyEvent(e), true);
    assertEquals(ui.enqueueKeyEvent(e), true);
    assertEquals(ui.enqueueKeyEvent(e), false);
});

Deno.test("buffer can be reused after dequeue", () => {
    const [ui, worker] = makeChannels(1);
    const e = event({ key: "z" });
    ui.enqueueKeyEvent(e);
    worker.dequeueKeyEvent();
    assertEquals(ui.enqueueKeyEvent(e), true);
    assertEquals(worker.dequeueKeyEvent(), e);
});

Deno.test("multi-character key is preserved", () => {
    const [ui, worker] = makeChannels();
    const e = event({ type: KEYDOWN, key: "Enter" });
    ui.enqueueKeyEvent(e);
    assertEquals(worker.dequeueKeyEvent(), e);
});

Deno.test("all key event types round-trip", () => {
    for (const type of [KEYPRESS, KEYDOWN, KEYUP]) {
        const [ui, worker] = makeChannels();
        ui.enqueueKeyEvent(event({ type, key: "A" }));
        assertEquals(worker.dequeueKeyEvent()?.type, type);
    }
});

Deno.test("checkInterrupt returns false when stop has not been called", () => {
    const [, worker] = makeChannels();
    assertEquals(worker.checkInterrupt(), false);
});

Deno.test("checkInterrupt returns true after stop", () => {
    const [ui, worker] = makeChannels();
    ui.stop();
    assertEquals(worker.checkInterrupt(), true);
});

Deno.test("checkInterrupt clears the flag", () => {
    const [ui, worker] = makeChannels();
    ui.stop();
    worker.checkInterrupt();
    assertEquals(worker.checkInterrupt(), false);
});
