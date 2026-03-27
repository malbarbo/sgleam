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

function makeReplTestHtml(expr: string): string {
    const escaped = JSON.stringify(expr);
    return `<!DOCTYPE html>
<html><head><title>WAITING</title></head>
<body>
<script type="module">
try {
    const worker = new Worker("worker.js?wasm=sgleam.wasm", { type: "module" });
    let initialized = false;
    const errors = [];
    const stderr = [];
    const svgs = [];

    worker.onmessage = (event) => {
        const data = event.data;
        if (data.cmd === "ready") {
            if (!initialized) {
                initialized = true;
                worker.postMessage({ cmd: "run", data: ${escaped} });
            } else {
                document.title = JSON.stringify({
                    ok: errors.length === 0 && stderr.length === 0,
                    svgCount: svgs.length,
                    errors, stderr, svgs,
                });
            }
        } else if (data.cmd === "write") {
            if (data.fd === 2) stderr.push(data.data);
        } else if (data.cmd === "svg") {
            svgs.push(data.data);
        } else if (data.cmd === "error") {
            errors.push(data.data);
            document.title = JSON.stringify({
                ok: false, svgCount: svgs.length, errors, stderr, svgs,
            });
        }
    };

    worker.onerror = (e) => {
        document.title = JSON.stringify({
            ok: false, svgCount: 0,
            errors: ["worker crash: " + e.message], stderr: [], svgs: [],
        });
    };

    setTimeout(() => {
        if (document.title === "WAITING") {
            document.title = JSON.stringify({
                ok: false, svgCount: svgs.length,
                errors: ["timeout"], stderr, svgs,
            });
        }
    }, 15000);
} catch (e) {
    document.title = JSON.stringify({
        ok: false, svgCount: 0,
        errors: ["init error: " + e.message], stderr: [], svgs: [],
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
    svgs: string[];
}

async function runExprInBrowser(expr: string): Promise<Result> {
    const html = makeReplTestHtml(expr);
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
        await new Promise((r) => setTimeout(r, 1000));
        const deadline = Date.now() + 20_000;

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
            svgs: [],
        };
    } finally {
        proc.kill();
        ac.abort();
        await server.finished;
    }
}

Deno.test({
    name: "text renders in browser",
    sanitizeOps: false,
    sanitizeResources: false,
    async fn() {
        const result = await runExprInBrowser(
            'import sgleam/fill\nimage.text("Hello", 24, fill.olive)',
        );
        assertEquals(result.ok, true, `Failed: ${JSON.stringify(result)}`);
        assertEquals(
            result.svgCount,
            1,
            `Expected 1 SVG, got ${result.svgCount}`,
        );
        assertEquals(
            result.svgs[0]?.includes("font-style"),
            true,
            `SVG missing font-style: ${result.svgs[0]}`,
        );
    },
});

Deno.test({
    name: "bold italic text renders in browser",
    sanitizeOps: false,
    sanitizeResources: false,
    async fn() {
        const result = await runExprInBrowser(
            'import sgleam/fill\nimport sgleam/font.{Bold, Font, Italic}\nimage.text_font("Test", Font(..font.default(), size: 20.0, font_style: Italic, font_weight: Bold), fill.black)',
        );
        assertEquals(result.ok, true, `Failed: ${JSON.stringify(result)}`);
        assertEquals(
            result.svgCount,
            1,
            `Expected 1 SVG, got ${result.svgCount}`,
        );
        assertEquals(
            result.svgs[0]?.includes('font-weight="bold"'),
            true,
            `SVG missing bold: ${result.svgs[0]}`,
        );
        assertEquals(
            result.svgs[0]?.includes('font-style="italic"'),
            true,
            `SVG missing italic: ${result.svgs[0]}`,
        );
    },
});

Deno.test({
    name: "underline text renders in browser",
    sanitizeOps: false,
    sanitizeResources: false,
    async fn() {
        const result = await runExprInBrowser(
            'import sgleam/fill\nimport sgleam/font.{Font}\nimage.text_font("Link", Font(..font.default(), size: 18.0, underline: True), fill.blue)',
        );
        assertEquals(result.ok, true, `Failed: ${JSON.stringify(result)}`);
        assertEquals(
            result.svgCount,
            1,
            `Expected 1 SVG, got ${result.svgCount}`,
        );
        assertEquals(
            result.svgs[0]?.includes('text-decoration="underline"'),
            true,
            `SVG missing underline: ${result.svgs[0]}`,
        );
    },
});
