import { assertEquals } from "jsr:@std/assert";
import {
    computeTextHeight,
    computeTextWidth,
    computeTextXOffset,
    computeTextYOffset,
} from "./env.ts";

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

// Runs a script in headless chromium and returns the document title.
async function runInBrowser(js: string): Promise<string> {
    const html =
        `<canvas id="c" width="500" height="500"></canvas><script>${js}</script>`;
    const file = await Deno.makeTempFile({ suffix: ".html" });
    await Deno.writeTextFile(file, html);
    try {
        const cmd = new Deno.Command(findChromium(), {
            args: [
                "--headless",
                "--disable-gpu",
                "--no-sandbox",
                "--virtual-time-budget=5000",
                "--dump-dom",
                `file://${file}`,
            ],
            stdout: "piped",
            stderr: "null",
        });
        const output = await cmd.output();
        const dom = new TextDecoder().decode(output.stdout);
        const start = dom.indexOf("<title>") + 7;
        const end = dom.indexOf("</title>");
        return dom.slice(start, end);
    } finally {
        await Deno.remove(file);
    }
}

// Build the browser-side JS that uses the same formulas as env.ts.
// We inline the exported functions so the browser executes identical logic.
const BROWSER_SCRIPT = `
const computeTextWidth = ${computeTextWidth.toString()};
const computeTextHeight = ${computeTextHeight.toString()};
const computeTextXOffset = ${computeTextXOffset.toString()};
const computeTextYOffset = ${computeTextYOffset.toString()};

function getMetrics(text, size, font) {
    const c = new OffscreenCanvas(1, 1);
    const ctx = c.getContext("2d");
    ctx.font = size + "px " + font;
    const m = ctx.measureText(text);
    return {
        width: computeTextWidth(m),
        height: computeTextHeight(m),
        x_offset: computeTextXOffset(m),
        y_offset: computeTextYOffset(m),
    };
}

function makeSvg(text, size, font) {
    const m = getMetrics(text, size, font);
    return {
        metrics: m,
        svg: '<svg xmlns="http://www.w3.org/2000/svg" width="' + m.width + '" height="' + m.height + '">'
            + '<text dominant-baseline="alphabetic" text-anchor="start"'
            + ' x="' + m.x_offset + '" y="' + m.y_offset + '"'
            + ' font-family="' + font + '" font-size="' + size + '"'
            + ' transform="translate(' + (m.width/2) + ',' + (m.height/2) + ')"'
            + ' fill="black">' + text + '</text></svg>',
    };
}

function checkBounds(label, text, size, font) {
    const { svg, metrics } = makeSvg(text, size, font);
    const w = Math.ceil(metrics.width) + 4;
    const h = Math.ceil(metrics.height) + 4;
    const canvas = document.getElementById("c");
    canvas.width = w; canvas.height = h;
    const ctx = canvas.getContext("2d");
    ctx.clearRect(0, 0, w, h);
    const img = new Image();
    img.onload = () => {
        ctx.drawImage(img, 0, 0);
        const data = ctx.getImageData(0, 0, w, h).data;
        let minX = w, maxX = 0, minY = h, maxY = 0;
        for (let y = 0; y < h; y++)
            for (let x = 0; x < w; x++)
                if (data[(y * w + x) * 4 + 3] > 10) {
                    minX = Math.min(minX, x); maxX = Math.max(maxX, x);
                    minY = Math.min(minY, y); maxY = Math.max(maxY, y);
                }
        document.title += JSON.stringify({
            label, svgW: Math.round(metrics.width), svgH: Math.round(metrics.height),
            minX, minY, maxX, maxY,
        }) + "|||";
    };
    img.src = "data:image/svg+xml," + encodeURIComponent(svg);
}
`;

interface BoundsResult {
    label: string;
    svgW: number;
    svgH: number;
    minX: number;
    minY: number;
    maxX: number;
    maxY: number;
}

async function getTextBounds(
    cases: { label: string; text: string; size: number; font: string }[],
): Promise<BoundsResult[]> {
    const calls = cases.map((c, i) =>
        `setTimeout(() => checkBounds("${c.label}", "${c.text}", ${c.size}, "${c.font}"), ${
            i * 200 + 100
        });`
    ).join("\n");
    const title = await runInBrowser(BROWSER_SCRIPT + calls);
    return title.split("|||").filter((s) => s.trim()).map((s) => JSON.parse(s));
}

Deno.test("text bounding boxes start at origin", async () => {
    const results = await getTextBounds([
        { label: "hello", text: "hello", size: 30, font: "sans-serif" },
        { label: "a", text: "a", size: 30, font: "sans-serif" },
        { label: "Ap", text: "Áp", size: 30, font: "sans-serif" },
        { label: "p", text: "p", size: 30, font: "sans-serif" },
        {
            label: "Testing",
            text: "Testing text",
            size: 16,
            font: "sans-serif",
        },
    ]);
    for (const r of results) {
        assertEquals(
            r.minX <= 1 && r.minY <= 1,
            true,
            `${r.label}: expected pixels near (0,0) but got (${r.minX},${r.minY}), svg=${r.svgW}x${r.svgH}`,
        );
    }
});
