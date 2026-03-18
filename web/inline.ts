// Build script: inlines JS (with worker.js embedded) into HTML files
// to produce single-file dist outputs.
//
// Usage: deno run --allow-read --allow-write web/inline.ts

const workerJs = await Deno.readTextFile("dist/worker.js");

// Shared worker embedding logic
const workerLiteral = "`" + workerJs.replaceAll("\\", "\\\\").replaceAll(
    "`",
    "\\`",
).replaceAll("$", "\\$") + "`";

const workerReplacement = [
    `const __wasmUrl = new URL("sgleam.wasm", location.href).href;`,
    `const __workerCode = ${workerLiteral}.replace('new URL(import.meta.url).searchParams.get("wasm") ?? "sgleam.wasm"', JSON.stringify(__wasmUrl));`,
    `const worker = new Worker(URL.createObjectURL(new Blob([__workerCode], { type: "application/javascript" })));`,
].join("\n    ");

function embedWorker(js: string): string {
    return js.replace(
        `const worker = new Worker("worker.js", {\n      type: "module"\n    });`,
        workerReplacement,
    );
}

// --- Playground (index.html) ---

let playgroundHtml = await Deno.readTextFile("web/sgleam.html");
let sgleamJs = embedWorker(await Deno.readTextFile("dist/sgleam.js"));
const codeflaskJs = await Deno.readTextFile("build/codeflask.min.js");

playgroundHtml = playgroundHtml.replace(
    /    <script src="https:\/\/unpkg\.com\/codeflask\/build\/codeflask\.min\.js"><\/script>/,
    `    <script>${codeflaskJs}</script>`,
);

playgroundHtml = playgroundHtml.replace(
    /    <script type="module" src="sgleam\.js"><\/script>/,
    `    <script type="module">${sgleamJs}</script>`,
);

await Deno.writeTextFile("dist/index.html", playgroundHtml);
console.log("Written dist/index.html");

// --- Player (player.html) ---

let playerHtml = await Deno.readTextFile("web/player.html");
let playerJs = embedWorker(await Deno.readTextFile("dist/player.js"));

playerHtml = playerHtml.replace(
    /    <script type="module" src="player\.js"><\/script>/,
    `    <script type="module">${playerJs}</script>`,
);

await Deno.writeTextFile("dist/player.html", playerHtml);
console.log("Written dist/player.html");
