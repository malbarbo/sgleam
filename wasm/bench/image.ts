// Times the cases of engine/benches/image on the WASM binary, as the
// browser runs them. The binary compiles one module, so each case goes in
// together with the bench helper. `deno run --allow-read wasm/bench/image.ts
// scene` runs only the cases whose name holds `scene`.

import { makeWasi } from "../tests/wasi.ts";

interface WasmExports {
  memory: WebAssembly.Memory;
  repl_new(
    code_ptr: number,
    code_len: number,
    config_ptr: number,
    config_len: number,
  ): number;
  repl_run(repl: number, ptr: number, len: number): number;
  repl_destroy(repl: number): void;
  string_allocate(size: number): number;
  string_deallocate(ptr: number, size: number): void;
}

const encoder = new TextEncoder();
const cases = new URL("../../engine/benches/image/", import.meta.url);
const wasm = new URL(
  "../../target/wasm32-wasip1/release-small/sgleam.wasm",
  import.meta.url,
);

function write(fd: number, text: string) {
  const out = fd === 2 ? Deno.stderr : Deno.stdout;
  const bytes = encoder.encode(text);
  let written = 0;
  while (written < bytes.length) {
    written += out.writeSync(bytes.subarray(written));
  }
}

function encode(exports: WasmExports, text: string): [number, number] {
  const bytes = encoder.encode(text);
  const ptr = exports.string_allocate(bytes.length);
  new Uint8Array(exports.memory.buffer, ptr, bytes.length).set(bytes);
  return [ptr, bytes.length];
}

// The case and the helper as one module. The imports of both go to the top
// once each, and `bench.run` becomes `run`.
function merge(helper: string, source: string): string {
  const imports = new Set<string>();
  const body: string[] = [];
  for (const text of [helper, source.replaceAll(/\bbench\./g, "")]) {
    for (const line of text.split("\n")) {
      if (line === "import bench") continue;
      if (line.startsWith("import ")) imports.add(line);
      else body.push(line);
    }
  }
  return [...imports, ...body].join("\n");
}

async function run(module: WebAssembly.Module, source: string) {
  let exports!: WasmExports;
  const wasi = makeWasi({
    getBuffer: () => exports.memory.buffer,
    // The REPL shows the `Nil` that `main()` gives.
    write: (fd, text) => text === "Nil\n" || write(fd, text),
    readStdin: () => "",
    args: ["sgleam", "run"],
    env: [],
    isTty: false,
  });
  const env = {
    check_interrupt: () => 0,
    sleep: () => {},
    draw_svg: () => {},
    wait_event: () => 4,
  };
  const instance = await WebAssembly.instantiate(module, {
    env,
    wasi_snapshot_preview1: wasi,
  });
  exports = instance.exports as unknown as WasmExports;

  const [code, codeLen] = encode(exports, source);
  const [config, configLen] = encode(exports, "bigint=true");
  const repl = exports.repl_new(code, codeLen, config, configLen);
  if (repl === 0) return;
  const [main, mainLen] = encode(exports, "main()");
  exports.repl_run(repl, main, mainLen);
}

const filter = Deno.args;
const helper = await Deno.readTextFile(new URL("bench.gleam", cases));
const names = [...Deno.readDirSync(cases)]
  .map((entry) => entry.name)
  .filter((name) => name.endsWith(".gleam") && name !== "bench.gleam")
  .filter((name) => filter.length === 0 || filter.some((f) => name.includes(f)))
  .sort();
const module = await WebAssembly.compile(await Deno.readFile(wasm));

for (const name of names) {
  console.log(`# ${name}`);
  const source = await Deno.readTextFile(new URL(name, cases));
  try {
    await run(module, merge(helper, source));
  } catch (err) {
    // A stack overflow in WASM is a trap, which ends the instance.
    console.log(`${name} trapped: ${err}`);
  }
}
