// Build script: inlines CodeFlask, sgleam.js (with worker.js embedded) into
// sgleam.html to produce a single dist/index.html file.
//
// Usage: deno run --allow-read --allow-write web/inline.ts

const [htmlPath, sgleamJsPath, workerJsPath, codeflaskPath, outputPath] = [
    "web/sgleam.html",
    "dist/sgleam.js",
    "dist/worker.js",
    "build/codeflask.min.js",
    "dist/index.html",
];

let html = await Deno.readTextFile(htmlPath);
let sgleamJs = await Deno.readTextFile(sgleamJsPath);
const workerJs = await Deno.readTextFile(workerJsPath);
const codeflaskJs = await Deno.readTextFile(codeflaskPath);

// Embed worker.js as a blob URL inside sgleam.js.
// The main thread computes the absolute wasm URL (since blob workers can't
// resolve relative paths) and patches it into the worker code before creating
// the blob.
const workerLiteral = "`" + workerJs.replaceAll("\\", "\\\\").replaceAll(
    "`",
    "\\`",
).replaceAll("$", "\\$") + "`";

const workerReplacement = [
    `const __wasmUrl = new URL("sgleam.wasm", location.href).href;`,
    `const __workerCode = ${workerLiteral}.replace('new URL(import.meta.url).searchParams.get("wasm") ?? "sgleam.wasm"', JSON.stringify(__wasmUrl));`,
    `const worker = new Worker(URL.createObjectURL(new Blob([__workerCode], { type: "application/javascript" })));`,
].join("\n    ");

sgleamJs = sgleamJs.replace(
    `const worker = new Worker("worker.js", {\n      type: "module"\n    });`,
    workerReplacement,
);

// Replace CodeFlask CDN script with inline
html = html.replace(
    /    <script src="https:\/\/unpkg\.com\/codeflask\/build\/codeflask\.min\.js"><\/script>/,
    `    <script>${codeflaskJs}</script>`,
);

// Replace sgleam.js module script with inline
html = html.replace(
    /    <script type="module" src="sgleam\.js"><\/script>/,
    `    <script type="module">${sgleamJs}</script>`,
);

await Deno.writeTextFile(outputPath, html);
console.log(`Written ${outputPath}`);
