import { assertEquals } from "jsr:@std/assert";
import { UIChannel, WorkerMessage } from "./ui_channel.ts";

const STDERR = 2;
const DIST = new URL("../dist/", import.meta.url).href;

function makeWorker(): [Worker, UIChannel] {
    const worker = new Worker(
        `${DIST}worker.js?wasm=sgleam.wasm`,
        { type: "module" },
    );
    return [worker, new UIChannel(worker)];
}

interface RunResult {
    stdout: string[];
    stderr: string[];
    svgs: string[];
    error: string | null;
}

function loadCode(code: string, timeoutMs = 5000): Promise<RunResult> {
    return runWorker(code, "load", timeoutMs);
}

function runExpr(expr: string, timeoutMs = 5000): Promise<RunResult> {
    return runWorker(expr, "run", timeoutMs);
}

function runWorker(
    data: string,
    mode: "load" | "run",
    timeoutMs: number,
): Promise<RunResult> {
    return new Promise((resolve, reject) => {
        const [worker, channel] = makeWorker();
        let initialized = false;
        const stdout: string[] = [];
        const stderr: string[] = [];
        const svgs: string[] = [];

        const timeout = setTimeout(() => {
            channel.stop();
            setTimeout(() => {
                worker.terminate();
                resolve({ stdout, stderr, svgs, error: null });
            }, 500);
        }, timeoutMs);

        worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
            const msg = event.data;
            if (msg.cmd === "ready") {
                if (!initialized) {
                    initialized = true;
                    channel.setBuffer(msg.buffer);
                    if (mode === "load") {
                        channel.load(data);
                    } else {
                        channel.run(data);
                    }
                } else {
                    clearTimeout(timeout);
                    worker.terminate();
                    resolve({ stdout, stderr, svgs, error: null });
                }
            } else if (msg.cmd === "write") {
                if (msg.fd === STDERR) {
                    stderr.push(msg.data);
                } else {
                    stdout.push(msg.data);
                }
            } else if (msg.cmd === "svg") {
                svgs.push(msg.data);
            } else if (msg.cmd === "error") {
                clearTimeout(timeout);
                worker.terminate();
                resolve({ stdout, stderr, svgs, error: msg.data });
            }
        };

        worker.onerror = (e) => {
            clearTimeout(timeout);
            worker.terminate();
            reject(new Error(`Worker crashed: ${e.message}`));
        };
    });
}

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

Deno.test("move_square loads without crash", async () => {
    const result = await loadCode(MOVE_SQUARE, 8000);
    assertEquals(
        result.error,
        null,
        `Worker error: ${result.error}\nstderr: ${result.stderr.join("")}`,
    );
    assertEquals(
        result.stderr.length,
        0,
        `Unexpected stderr: ${result.stderr.join("")}`,
    );
});

// Regression: sgleam.sleep(1) must use BigInt (1n) for WASM i64 compat.
// Without this, world.run() crashes with RuntimeError: unreachable
// (signature_mismatch:sleep).
Deno.test("world.run does not crash (sleep bigint regression)", async () => {
    const result = await new Promise<RunResult>((resolve, reject) => {
        const [worker, channel] = makeWorker();
        let readyCount = 0;
        const stderr: string[] = [];
        const svgs: string[] = [];

        const timeout = setTimeout(() => {
            channel.stop();
            setTimeout(() => {
                worker.terminate();
                resolve({ stdout: [], stderr, svgs, error: null });
            }, 500);
        }, 10_000);

        worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
            const msg = event.data;
            if (msg.cmd === "ready") {
                readyCount++;
                if (readyCount === 1) {
                    channel.setBuffer(msg.buffer);
                    channel.load(MOVE_SQUARE);
                } else if (readyCount === 2) {
                    // load finished, now run main()
                    channel.run("main()");
                } else {
                    // run finished (or repl reloaded after crash)
                    clearTimeout(timeout);
                    worker.terminate();
                    resolve({ stdout: [], stderr, svgs, error: null });
                }
            } else if (msg.cmd === "write") {
                if (msg.fd === 2) stderr.push(msg.data);
            } else if (msg.cmd === "svg") {
                svgs.push(msg.data);
            } else if (msg.cmd === "error") {
                clearTimeout(timeout);
                worker.terminate();
                resolve({ stdout: [], stderr, svgs, error: msg.data });
            }
        };

        worker.onerror = (e) => {
            clearTimeout(timeout);
            worker.terminate();
            reject(new Error(`Worker crashed: ${e.message}`));
        };
    });
    assertEquals(
        result.stderr.length,
        0,
        `Crashed! stderr: ${result.stderr.join("")}`,
    );
});

// This test lets world.run() loop without interruption to check for stack overflow.
// It should survive 30 seconds without crashing.
Deno.test("move_square survives long run", async () => {
    const result = await new Promise<RunResult>((resolve, reject) => {
        const [worker, channel] = makeWorker();
        let initialized = false;
        const stderr: string[] = [];
        const svgs: string[] = [];
        let svgCount = 0;

        // No stop signal — let it run until crash or timeout
        const timeout = setTimeout(() => {
            channel.stop();
            setTimeout(() => {
                worker.terminate();
                resolve({
                    stdout: [`rendered ${svgCount} frames`],
                    stderr,
                    svgs: [],
                    error: null,
                });
            }, 500);
        }, 30_000);

        worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
            const msg = event.data;
            if (msg.cmd === "ready") {
                if (!initialized) {
                    initialized = true;
                    channel.setBuffer(msg.buffer);
                    channel.load(MOVE_SQUARE);
                }
                // Don't resolve on second ready — world.run blocks forever
            } else if (msg.cmd === "write") {
                if (msg.fd === STDERR) {
                    stderr.push(msg.data);
                    // If stderr contains "stackoverflow", it crashed
                    if (msg.data.toLowerCase().includes("stack overflow")) {
                        clearTimeout(timeout);
                        worker.terminate();
                        resolve({
                            stdout: [`crashed after ${svgCount} frames`],
                            stderr,
                            svgs: [],
                            error: "stack overflow",
                        });
                    }
                }
            } else if (msg.cmd === "svg") {
                svgCount++;
            } else if (msg.cmd === "error") {
                clearTimeout(timeout);
                worker.terminate();
                resolve({
                    stdout: [`error after ${svgCount} frames`],
                    stderr,
                    svgs: [],
                    error: msg.data,
                });
            }
        };

        worker.onerror = (e) => {
            clearTimeout(timeout);
            worker.terminate();
            resolve({
                stdout: [`crashed after ${svgCount} frames`],
                stderr,
                svgs: [],
                error: `Worker crash: ${e.message}`,
            });
        };
    });
    assertEquals(
        result.error,
        null,
        `Crashed: ${result.error}\nframes: ${result.stdout.join("")}\nstderr: ${
            result.stderr.join("")
        }`,
    );
});

Deno.test("circle image renders in REPL", async () => {
    const result = await runExpr(
        "import sgleam/stroke\nimage.circle(30, stroke.red)",
    );
    assertEquals(result.error, null, `Worker error: ${result.error}`);
    assertEquals(
        result.stderr.length,
        0,
        `stderr: ${result.stderr.join("")}`,
    );
    assertEquals(
        result.svgs.length > 0,
        true,
        `Expected SVG output, got none. stdout: ${result.stdout.join("")}`,
    );
});

Deno.test("wedge image renders in REPL", async () => {
    const result = await runExpr(
        "import sgleam/fill\nimage.wedge(40, 90, fill.red)",
    );
    assertEquals(result.error, null, `Worker error: ${result.error}`);
    assertEquals(
        result.stderr.length,
        0,
        `stderr: ${result.stderr.join("")}`,
    );
    assertEquals(
        result.svgs.length > 0,
        true,
        `Expected SVG output, got none. stdout: ${result.stdout.join("")}`,
    );
});

Deno.test("add_curve renders in REPL", async () => {
    const result = await runExpr(
        "import sgleam/stroke\nimage.add_curve(image.rectangle(100, 100, stroke.black), 20, 20, 0, 0.333, 80, 80, 0, 0.333, stroke.red)",
    );
    assertEquals(result.error, null, `Worker error: ${result.error}`);
    assertEquals(
        result.stderr.length,
        0,
        `stderr: ${result.stderr.join("")}`,
    );
});
