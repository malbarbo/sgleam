import { assertEquals } from "jsr:@std/assert";

Deno.test("repl smoke test", async () => {
    return new Promise((resolve, reject) => {
        const repl = new Worker(
            new URL("./repl.js", import.meta.url).href,
            { type: "module" }
        );

        let first = true;
        let sharedBuffer = new SharedArrayBuffer(4);
        let buffer = new Int32Array(sharedBuffer);
        Atomics.store(buffer, 0, 0);

        repl.onmessage = (event) => {
            const data = event.data;
            if (data.cmd == 'ready') {
                if (first) {
                    first = false;
                    repl.postMessage({ cmd: 'init', data: sharedBuffer });
                    repl.postMessage({ cmd: 'run', data: "1 + 2" });
                }
            } else if (data.cmd == 'output') {
                assertEquals(data.data, "3\n");
                repl.terminate();
                resolve();
            } else {
                console.log(`${data.cmd}: ${data.data}`);
                reject();
            }
        }
    });
});