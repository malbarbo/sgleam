import { assertEquals } from "jsr:@std/assert";
import { UIChannel } from "./channel.js";

const STDERR = 2;

Deno.test("repl smoke test", async () => {
    return new Promise((resolve, reject) => {
        const repl = new Worker(
            new URL("./repl.js", import.meta.url).href,
            { type: "module" }
        );

        let first = true;
        const sharedBuffer = new UIChannel(10).getBuffer();

        repl.onmessage = (event) => {
            const data = event.data;
            if (data.cmd == 'ready') {
                if (first) {
                    first = false;
                    repl.postMessage({ cmd: 'init', data: sharedBuffer });
                    repl.postMessage({ cmd: 'run', data: "1 + 2" });
                }
            } else if (data.cmd == 'output') {
                if (data.fd === STDERR) return; // ignore compiler warnings
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
