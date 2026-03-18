import { assertEquals, assertMatch } from "jsr:@std/assert";
import { UIChannel, WorkerMessage } from "./ui_channel.ts";

const STDERR = 2;

function makeWorker(): [Worker, UIChannel] {
    const worker = new Worker(
        new URL("./worker.js?wasm=sgleam.wasm", import.meta.url).href,
        { type: "module" },
    );
    return [worker, new UIChannel(worker)];
}

Deno.test("repl smoke test", async () => {
    return new Promise<void>((resolve, reject) => {
        const [worker, channel] = makeWorker();
        let initialized = false;

        worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
            const data = event.data;
            if (data.cmd === "ready") {
                if (!initialized) {
                    initialized = true;
                    channel.setBuffer(data.buffer);
                    channel.run("1 + 2");
                }
            } else if (data.cmd === "write") {
                if (data.fd === STDERR) return;
                assertEquals(data.data, "3\n");
                worker.terminate();
                resolve();
            } else if (data.cmd === "error") {
                reject(new Error(`Worker error: ${data.data}`));
            }
        };
    });
});

Deno.test("multiple runs", async () => {
    return new Promise<void>((resolve, reject) => {
        const [worker, channel] = makeWorker();
        let readyCount = 0;

        worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
            const data = event.data;
            if (data.cmd === "ready") {
                readyCount++;
                if (readyCount === 1) {
                    channel.setBuffer(data.buffer);
                    channel.run("1 + 2");
                } else if (readyCount === 2) {
                    channel.run("10 + 20");
                }
            } else if (data.cmd === "write") {
                if (data.fd === STDERR) return;
                if (readyCount === 1) {
                    assertEquals(data.data, "3\n");
                } else if (readyCount === 2) {
                    assertEquals(data.data, "30\n");
                    worker.terminate();
                    resolve();
                }
            } else if (data.cmd === "error") {
                reject(new Error(`Worker error: ${data.data}`));
            }
        };
    });
});

Deno.test("error output contains ansi codes", async () => {
    return new Promise<void>((resolve, reject) => {
        const [worker, channel] = makeWorker();
        let initialized = false;

        worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
            const data = event.data;
            if (data.cmd === "ready") {
                if (!initialized) {
                    initialized = true;
                    channel.setBuffer(data.buffer);
                    channel.run("unknown_variable");
                }
            } else if (data.cmd === "write") {
                if (data.fd === STDERR) {
                    assertMatch(data.data, /\x1b\[/);
                    worker.terminate();
                    resolve();
                }
            } else if (data.cmd === "error") {
                reject(new Error(`Worker error: ${data.data}`));
            }
        };
    });
});

Deno.test("load with errors sets hadErrors true", async () => {
    const code = "pub fn f(x) { x + 1 }\npub fn g() { unknown }";
    return new Promise<void>((resolve, reject) => {
        const [worker, channel] = makeWorker();
        let readyCount = 0;

        worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
            const data = event.data;
            if (data.cmd === "ready") {
                readyCount++;
                if (readyCount === 1) {
                    channel.setBuffer(data.buffer);
                    channel.load(code);
                } else {
                    assertEquals(data.hadErrors, true);
                    worker.terminate();
                    resolve();
                }
            } else if (data.cmd === "error") {
                reject(new Error(`Worker error: ${data.data}`));
            }
        };
    });
});

Deno.test("load without errors sets hadErrors false", async () => {
    const code = "pub fn f(x: Int) -> Int { x + 1 }";
    return new Promise<void>((resolve, reject) => {
        const [worker, channel] = makeWorker();
        let readyCount = 0;

        worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
            const data = event.data;
            if (data.cmd === "ready") {
                readyCount++;
                if (readyCount === 1) {
                    channel.setBuffer(data.buffer);
                    channel.load(code);
                } else {
                    assertEquals(data.hadErrors, false);
                    worker.terminate();
                    resolve();
                }
            } else if (data.cmd === "error") {
                reject(new Error(`Worker error: ${data.data}`));
            }
        };
    });
});

Deno.test("load with examples", async () => {
    const code = `import sgleam/check

pub fn add(a: Int, b: Int) -> Int {
  a + b
}

pub fn add_examples() {
  check.eq(add(1, 2), 3)
  check.eq(add(0, 0), 0)
}
`;
    return new Promise<void>((resolve, reject) => {
        const [worker, channel] = makeWorker();
        let initialized = false;
        const outputs: string[] = [];

        worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
            const data = event.data;
            if (data.cmd === "ready") {
                if (!initialized) {
                    initialized = true;
                    channel.setBuffer(data.buffer);
                    channel.load(code);
                } else {
                    assertEquals(outputs, [
                        "Running tests...\n",
                        "2 tests, 2 success(es), 0 failure(s) and 0 error(s).\n",
                    ]);
                    worker.terminate();
                    resolve();
                }
            } else if (data.cmd === "write") {
                if (data.fd === STDERR) {
                    reject(new Error(`Unexpected stderr: ${data.data}`));
                } else {
                    outputs.push(data.data);
                }
            } else if (data.cmd === "error") {
                reject(new Error(`Worker error: ${data.data}`));
            }
        };
    });
});
