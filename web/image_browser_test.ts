import { assertEquals } from "jsr:@std/assert";

function findChromium(): string {
    const env = Deno.env.get("CHROME_BIN");
    if (env) return env;
    for (const name of ["chromium", "chromium-browser", "google-chrome"]) {
        try {
            const r = new Deno.Command("which", {
                args: [name],
                stdout: "piped",
                stderr: "null",
            }).outputSync();
            if (r.success) return name;
        } catch { /* not found */ }
    }
    throw new Error("No chromium/chrome binary found");
}

const DIST = new URL("../dist/", import.meta.url).pathname;

const MOVE_SQUARE = `import gleam/int
import sgleam/fill
import sgleam/image
import sgleam/stroke
import sgleam/style
import sgleam/world
import sgleam/xplace
import sgleam/yplace

pub fn main() {
  world.create(Pos(lines / 2, columns / 2), draw)
  |> world.on_key_down(move)
  |> world.stop_when(fn(p) { p.line == 0 && p.column == 0 })
  |> world.run()
}

const lines = 9
const columns = 11
const size = 30

pub type Pos {
  Pos(line: Int, column: Int)
}

pub fn draw(p: Pos) -> image.Image {
  image.empty_scene(size * columns, size * lines)
  |> image.place_image_align(
    size * p.column,
    size * p.line,
    xplace.Left,
    yplace.Top,
    image.square(size, [fill.red, stroke.black] |> style.join),
  )
}

pub fn move(p: Pos, key: world.Key) -> Pos {
  let p = case key {
    world.ArrowLeft -> Pos(..p, column: p.column - 1)
    world.ArrowRight -> Pos(..p, column: p.column + 1)
    world.ArrowDown -> Pos(..p, line: p.line + 1)
    world.ArrowUp -> Pos(..p, line: p.line - 1)
    _ -> p
  }
  Pos(int.clamp(p.line, 0, lines - 1), int.clamp(p.column, 0, columns - 1))
}
`;

function makeTestHtml(gleamCode: string): string {
    const escaped = JSON.stringify(gleamCode);
    return `<!DOCTYPE html>
<html><head><title>WAITING</title></head>
<body>
<script type="module">
const STOP_INDEX = 0;
const SLEEP_INDEX = 1;
const INPUT_READY_INDEX = 184;

try {
    const worker = new Worker("worker.js?wasm=sgleam.wasm", { type: "module" });
    let initialized = false;
    let svgCount = 0;
    const errors = [];
    const stderr = [];
    let buf = null;

    worker.onmessage = (event) => {
        const data = event.data;
        if (data.cmd === "ready") {
            if (!initialized) {
                initialized = true;
                buf = new Int32Array(data.buffer);
                worker.postMessage({ cmd: "load", data: ${escaped} });
            } else {
                // load finished, now run main()
                worker.postMessage({ cmd: "run", data: "main()" });
            }
        } else if (data.cmd === "write") {
            if (data.fd === 2) {
                stderr.push(data.data);
            }
        } else if (data.cmd === "svg") {
            svgCount++;
            // After getting some frames, report success and stop
            if (svgCount >= 50) {
                Atomics.store(buf, STOP_INDEX, 1);
                Atomics.notify(buf, SLEEP_INDEX, 1);
                Atomics.notify(buf, INPUT_READY_INDEX, 1);
                document.title = JSON.stringify({
                    ok: true, svgCount, errors, stderr,
                });
            }
        } else if (data.cmd === "error") {
            errors.push(data.data);
            document.title = JSON.stringify({
                ok: false, svgCount, errors, stderr,
            });
        }
    };

    worker.onerror = (e) => {
        document.title = JSON.stringify({
            ok: false, svgCount,
            errors: ["worker crash: " + e.message],
            stderr,
        });
    };

    // Safety timeout
    setTimeout(() => {
        if (document.title === "WAITING") {
            document.title = JSON.stringify({
                ok: svgCount > 0, svgCount, errors, stderr,
            });
        }
        if (buf) {
            Atomics.store(buf, STOP_INDEX, 1);
            Atomics.notify(buf, SLEEP_INDEX, 1);
            Atomics.notify(buf, INPUT_READY_INDEX, 1);
        }
    }, 20000);
} catch (e) {
    document.title = JSON.stringify({
        ok: false, svgCount: 0,
        errors: ["init error: " + e.message], stderr: [],
    });
}
</script>
</body></html>`;
}

interface Result {
    ok: boolean;
    svgCount: number;
    errors: string[];
    stderr: string[];
}

async function runInBrowser(gleamCode: string): Promise<Result> {
    const html = makeTestHtml(gleamCode);

    // Serve dist/ with COOP/COEP headers
    const ac = new AbortController();
    let port = 0;
    const server = Deno.serve(
        {
            signal: ac.signal,
            port: 0,
            onListen: (addr) => {
                port = addr.port;
            },
        },
        async (req) => {
            const url = new URL(req.url);
            const headers = {
                "cross-origin-opener-policy": "same-origin",
                "cross-origin-embedder-policy": "require-corp",
            };
            if (url.pathname === "/") {
                return new Response(html, {
                    headers: { "content-type": "text/html", ...headers },
                });
            }
            try {
                const file = await Deno.readFile(
                    DIST + url.pathname.slice(1),
                );
                const ct = url.pathname.endsWith(".js")
                    ? "application/javascript"
                    : url.pathname.endsWith(".wasm")
                    ? "application/wasm"
                    : "application/octet-stream";
                return new Response(file, {
                    headers: { "content-type": ct, ...headers },
                });
            } catch {
                return new Response("Not found", { status: 404 });
            }
        },
    );
    while (port === 0) await new Promise((r) => setTimeout(r, 50));

    // Launch Chrome with remote debugging
    const debugPort = 9222 + Math.floor(Math.random() * 1000);
    const proc = new Deno.Command(findChromium(), {
        args: [
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            `--remote-debugging-port=${debugPort}`,
            `http://localhost:${port}/`,
        ],
        stdout: "null",
        stderr: "null",
    }).spawn();

    try {
        // Wait for Chrome to start and poll title via CDP
        await new Promise((r) => setTimeout(r, 1000));
        const deadline = Date.now() + 30_000;

        while (Date.now() < deadline) {
            try {
                const resp = await fetch(
                    `http://localhost:${debugPort}/json`,
                );
                const targets = await resp.json();
                const page = targets.find((t: { type: string }) =>
                    t.type === "page"
                );
                if (page) {
                    // Use CDP to get the title
                    const ws = new WebSocket(page.webSocketDebuggerUrl);
                    const result = await new Promise<Result>(
                        (resolve) => {
                            let id = 1;
                            ws.onopen = () => {
                                ws.send(JSON.stringify({
                                    id: id++,
                                    method: "Runtime.evaluate",
                                    params: {
                                        expression: "document.title",
                                    },
                                }));
                            };
                            ws.onmessage = (e) => {
                                const msg = JSON.parse(e.data);
                                if (msg.result?.result?.value) {
                                    const title = msg.result.result.value;
                                    if (title !== "WAITING") {
                                        ws.close();
                                        resolve(JSON.parse(title));
                                        return;
                                    }
                                }
                                // Poll again
                                setTimeout(() => {
                                    ws.send(JSON.stringify({
                                        id: id++,
                                        method: "Runtime.evaluate",
                                        params: {
                                            expression: "document.title",
                                        },
                                    }));
                                }, 500);
                            };
                        },
                    );
                    return result;
                }
            } catch {
                // CDP not ready yet
            }
            await new Promise((r) => setTimeout(r, 500));
        }

        return {
            ok: false,
            svgCount: 0,
            errors: ["Timed out waiting for Chrome"],
            stderr: [],
        };
    } finally {
        proc.kill();
        ac.abort();
        await server.finished;
    }
}

Deno.test({
    name: "move_square in headless chrome",
    async fn() {
        const result = await runInBrowser(MOVE_SQUARE);
        console.log("Result:", JSON.stringify(result, null, 2));
        assertEquals(
            result.ok,
            true,
            `Crashed after ${result.svgCount} frames.\nErrors: ${
                result.errors.join("\n")
            }\nStderr: ${result.stderr.join("")}`,
        );
    },
    sanitizeOps: false,
    sanitizeResources: false,
});
