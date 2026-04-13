import { assertEquals, assertMatch } from "jsr:@std/assert";
import { makeWasi } from "./wasi.ts";

// --- WASM exports interface ---

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
  repl_complete?(
    repl: number,
    ptr: number,
    len: number,
    cursor_pos: number,
  ): number;
  repl_stop(): void;
  string_allocate(size: number): number;
  string_deallocate(ptr: number, size: number): void;
  format(ptr: number, len: number): number;
  cstr_deallocate(ptr: number): void;
  version(): number;
}

// --- Memory helpers ---

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function encodeString(exports: WasmExports, str: string): [number, number] {
  const encoded = encoder.encode(str);
  const ptr = exports.string_allocate(encoded.length);
  new Uint8Array(exports.memory.buffer, ptr, encoded.length).set(encoded);
  return [ptr, encoded.length];
}

function readCstr(exports: WasmExports, ptr: number): string {
  const buffer = new Uint8Array(exports.memory.buffer);
  let end = ptr;
  while (buffer[end] !== 0) end++;
  return decoder.decode(buffer.slice(ptr, end));
}

// --- Mock env ---

const KEYDOWN = 1;
const KEYNONE = 3;

interface EnvKeyEvent {
  type: number;
  key: string;
  alt?: boolean;
  ctrl?: boolean;
  shift?: boolean;
  meta?: boolean;
  repeat?: boolean;
}

interface EnvOptions {
  interruptAfter?: number;
  keyEvents?: EnvKeyEvent[];
}

function makeEnv(
  getBuffer: () => ArrayBufferLike,
  svgs: string[],
  options: EnvOptions = {},
): WebAssembly.ModuleImports {
  let interruptCount = 0;
  const interruptAfter = options.interruptAfter ?? Infinity;
  const keyEvents = [...(options.keyEvents ?? [])];

  return {
    check_interrupt: (): number => {
      interruptCount++;
      return interruptCount >= interruptAfter ? 1 : 0;
    },
    sleep: (_ms: bigint): void => {},
    draw_svg: (ptr: number, len: number): void => {
      const b = new Uint8Array(getBuffer() as ArrayBuffer);
      svgs.push(decoder.decode(b.slice(ptr, ptr + len)));
    },
    get_key_event: (ptr: number, len: number, mods: number): number => {
      const event = keyEvents.shift();
      if (!event) {
        return KEYNONE;
      }
      const b = new Uint8Array(getBuffer() as ArrayBuffer);
      const key = encoder.encode(event.key);
      const writeStart = Math.max(0, ptr);
      const writeEnd = Math.min(b.length, ptr + Math.max(0, len));
      if (writeStart < writeEnd) {
        b.fill(0, writeStart, writeEnd);
        const keyLen = Math.min(key.length, writeEnd - writeStart);
        for (let i = 0; i < keyLen; i++) {
          b[writeStart + i] = key[i];
        }
      }
      if (mods >= 0 && mods + 4 < b.length) {
        b[mods + 0] = event.alt ? 1 : 0;
        b[mods + 1] = event.ctrl ? 1 : 0;
        b[mods + 2] = event.shift ? 1 : 0;
        b[mods + 3] = event.meta ? 1 : 0;
        b[mods + 4] = event.repeat ? 1 : 0;
      }
      return event.type;
    },
    text_width: (): number => 10,
    text_height: (): number => 16,
    text_x_offset: (): number => -5,
    text_y_offset: (): number => 12,
    load_bitmap_fetch: (): number => 0,
    load_bitmap_width: (): number => 0,
    load_bitmap_height: (): number => 0,
    load_bitmap_data: (): number => 0,
  };
}

// --- WASM loader ---

interface WasmContext {
  exports: WasmExports;
  stdout: string[];
  stderr: string[];
  svgs: string[];
}

interface LoadOptions {
  bigint?: boolean;
  interruptAfter?: number;
  keyEvents?: EnvKeyEvent[];
}

async function loadWasm(options: LoadOptions = {}): Promise<WasmContext> {
  const stdout: string[] = [];
  const stderr: string[] = [];
  const svgs: string[] = [];

  const wasmPath = new URL(
    "../../target/wasm32-wasip1/release-small/sgleam.wasm",
    import.meta.url,
  ).pathname;
  const wasmBytes = await Deno.readFile(wasmPath);
  const module = await WebAssembly.compile(wasmBytes);

  let exports: WasmExports;

  const wasi = makeWasi({
    getBuffer: () => exports.memory.buffer,
    write: (fd, text) => {
      if (fd === 2) stderr.push(text);
      else stdout.push(text);
    },
    env: ["RUST_BACKTRACE=1"],
  });

  const env = makeEnv(
    () => exports.memory.buffer,
    svgs,
    {
      interruptAfter: options.interruptAfter,
      keyEvents: options.keyEvents,
    },
  );

  const instance = await WebAssembly.instantiate(module, {
    env,
    wasi_snapshot_preview1: wasi,
  });

  exports = instance.exports as unknown as WasmExports;

  return { exports, stdout, stderr, svgs };
}

// --- Helper ---

interface ReplContext extends WasmContext {
  repl: number;
}

async function newRepl(
  source = "",
  options: LoadOptions = {},
): Promise<ReplContext> {
  const ctx = await loadWasm(options);
  const [codePtr, codeLen] = encodeString(ctx.exports, source);
  const config = options.bigint !== false ? "bigint=true" : "";
  const [cfgPtr, cfgLen] = encodeString(ctx.exports, config);
  const repl = ctx.exports.repl_new(codePtr, codeLen, cfgPtr, cfgLen);
  ctx.exports.string_deallocate(codePtr, codeLen);
  ctx.exports.string_deallocate(cfgPtr, cfgLen);
  return { ...ctx, repl };
}

function run(
  ctx: ReplContext,
  input: string,
): { result: number; stdout: string; stderr: string; svgs: string[] } {
  ctx.stdout.length = 0;
  ctx.stderr.length = 0;
  ctx.svgs.length = 0;
  const [ptr, len] = encodeString(ctx.exports, input);
  const result = ctx.exports.repl_run(ctx.repl, ptr, len);
  ctx.exports.string_deallocate(ptr, len);
  return {
    result,
    stdout: ctx.stdout.join(""),
    stderr: ctx.stderr.join(""),
    svgs: [...ctx.svgs],
  };
}

function destroy(ctx: ReplContext): void {
  ctx.exports.repl_destroy(ctx.repl);
}

// --- Constants ---

const REPL_OK = 0;
const REPL_ERROR = 1;
const REPL_QUIT = 2;

// --- World program used in multiple tests ---

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

// --- Tests ---

Deno.test("version returns non-empty string", async () => {
  const ctx = await loadWasm();
  const ptr = ctx.exports.version();
  const ver = readCstr(ctx.exports, ptr);
  ctx.exports.cstr_deallocate(ptr);
  assertEquals(ver.length > 0, true, "version should be non-empty");
});

Deno.test("repl smoke test", async () => {
  const ctx = await newRepl();
  const r = run(ctx, "1 + 2");
  assertEquals(r.result, REPL_OK, `stderr:\n${r.stderr}\nstdout:\n${r.stdout}`);
  assertEquals(r.stdout, "3\n");
  destroy(ctx);
});

Deno.test("stepper renders UI in stdout", async () => {
  const ctx = await newRepl("", {
    keyEvents: [{ type: KEYDOWN, key: "q" }],
  });
  const r = run(ctx, ":stepper case 1 == 0 { True -> 10 False -> 20 }");
  assertEquals(r.result, REPL_OK, `stderr:\n${r.stderr}\nstdout:\n${r.stdout}`);
  assertEquals(r.stdout.includes("\x1b[2J\x1b[H"), true, "expected clear screen ANSI");
  assertEquals(r.stdout.includes("Stepper - Step 1"), true, "expected UI text");
  destroy(ctx);
});

Deno.test(":quit returns quit status", async () => {
  const ctx = await newRepl();
  const r = run(ctx, ":quit");
  assertEquals(r.result, REPL_QUIT);
  destroy(ctx);
});

Deno.test("multiple runs", async () => {
  const ctx = await newRepl();
  const r1 = run(ctx, "1 + 2");
  assertEquals(r1.stdout, "3\n");
  const r2 = run(ctx, "10 + 20");
  assertEquals(r2.stdout, "30\n");
  destroy(ctx);
});

Deno.test("error output contains ansi codes", async () => {
  const ctx = await newRepl();
  const r = run(ctx, "unknown_variable");
  assertEquals(r.result, REPL_ERROR);
  assertMatch(r.stderr, /\x1b\[/, "expected ANSI codes in error output");
  destroy(ctx);
});

Deno.test("load with errors returns null", async () => {
  const ctx = await loadWasm();
  const source = "pub fn f(x) { x + 1 }\npub fn g() { unknown }";
  const [codePtr, codeLen] = encodeString(ctx.exports, source);
  const [cfgPtr, cfgLen] = encodeString(ctx.exports, "");
  const repl = ctx.exports.repl_new(codePtr, codeLen, cfgPtr, cfgLen);
  ctx.exports.string_deallocate(codePtr, codeLen);
  ctx.exports.string_deallocate(cfgPtr, cfgLen);
  assertEquals(repl, 0, "repl_new should return null for code with errors");
});

Deno.test("load without errors", async () => {
  const ctx = await newRepl("pub fn f(x: Int) -> Int { x + 1 }");
  assertEquals(ctx.repl !== 0, true, "repl_new should return non-null");
  const r = run(ctx, "f(10)");
  assertEquals(r.result, REPL_OK);
  assertEquals(r.stdout, "11\n");
  destroy(ctx);
});

Deno.test("load with examples", async () => {
  const source = `import sgleam/check

pub fn add(a: Int, b: Int) -> Int {
  a + b
}

pub fn add_examples() {
  check.eq(add(1, 2), 3)
  check.eq(add(0, 0), 0)
}
`;
  const ctx = await loadWasm();
  const [codePtr, codeLen] = encodeString(ctx.exports, source);
  const [cfgPtr, cfgLen] = encodeString(ctx.exports, "");
  const repl = ctx.exports.repl_new(codePtr, codeLen, cfgPtr, cfgLen);
  ctx.exports.string_deallocate(codePtr, codeLen);
  ctx.exports.string_deallocate(cfgPtr, cfgLen);
  assertEquals(repl !== 0, true);
  const stdout = ctx.stdout.join("");
  assertEquals(
    stdout.includes("2 tests"),
    true,
    `expected test output, got: ${stdout}`,
  );
  ctx.exports.repl_destroy(repl);
});

Deno.test("circle image produces SVG", async () => {
  const ctx = await newRepl();
  const r = run(ctx, "import sgleam/stroke\nimage.circle(30, stroke.red)");
  assertEquals(r.result, REPL_OK);
  assertEquals(
    r.svgs.length > 0,
    true,
    `expected SVG, got none. stdout: ${r.stdout}`,
  );
  assertEquals(r.svgs[0].includes("<svg"), true, "expected SVG content");
  destroy(ctx);
});

Deno.test("wedge image produces SVG", async () => {
  const ctx = await newRepl();
  const r = run(ctx, "import sgleam/fill\nimage.wedge(40, 90, fill.red)");
  assertEquals(r.result, REPL_OK);
  assertEquals(r.svgs.length > 0, true, "expected SVG output");
  destroy(ctx);
});

Deno.test("add_curve renders", async () => {
  const ctx = await newRepl();
  const r = run(
    ctx,
    "import sgleam/stroke\nimage.add_curve(image.rectangle(100, 100, stroke.black), 20, 20, 0, 0.333, 80, 80, 0, 0.333, stroke.red)",
  );
  assertEquals(r.result, REPL_OK);
  assertEquals(r.stderr, "", `unexpected stderr: ${r.stderr}`);
  destroy(ctx);
});

Deno.test("format code", async () => {
  const ctx = await loadWasm();
  const input = "pub fn f( x : Int ) { x+1 }";
  const [ptr, len] = encodeString(ctx.exports, input);
  const resultPtr = ctx.exports.format(ptr, len);
  ctx.exports.string_deallocate(ptr, len);
  assertEquals(resultPtr !== 0, true, "format should return non-null");
  const formatted = readCstr(ctx.exports, resultPtr);
  ctx.exports.cstr_deallocate(resultPtr);
  assertEquals(
    formatted.includes("pub fn f"),
    true,
    "should contain formatted function",
  );
});

Deno.test("sleep works with BigInt", async () => {
  const ctx = await newRepl();
  const r = run(ctx, "import sgleam/system\nsystem.sleep(10)");
  assertEquals(r.result, REPL_OK);
  assertEquals(r.stderr, "", `unexpected stderr: ${r.stderr}`);
  destroy(ctx);
});

Deno.test("sleep works with computed BigInt value", async () => {
  const ctx = await newRepl();
  const r = run(
    ctx,
    "import sgleam/system\nlet ms = 25 + 25\nsystem.sleep(ms)",
  );
  assertEquals(r.result, REPL_OK);
  assertEquals(r.stderr, "", `unexpected stderr: ${r.stderr}`);
  destroy(ctx);
});

Deno.test("sleep works with Number (bigint disabled)", async () => {
  const ctx = await newRepl("", { bigint: false });
  const r = run(ctx, "import sgleam/system\nsystem.sleep(50)");
  assertEquals(r.result, REPL_OK);
  assertEquals(r.stderr, "", `unexpected stderr: ${r.stderr}`);
  destroy(ctx);
});

Deno.test("move_square loads without crash", async () => {
  const ctx = await newRepl(MOVE_SQUARE);
  assertEquals(ctx.repl !== 0, true, "repl_new should return non-null");
  assertEquals(
    ctx.stderr.join(""),
    "",
    `unexpected stderr: ${ctx.stderr.join("")}`,
  );
  destroy(ctx);
});

// Regression: the WASM import for sleep must not collide with the POSIX
// sleep symbol.  The wasm32-wasip1 linker replaces unresolved "sleep" with
// a signature_mismatch stub that traps.  We renamed the import to
// "sgleam_sleep" to avoid this.
Deno.test("world.run does not crash (sleep regression)", async () => {
  const ctx = await newRepl(MOVE_SQUARE, { interruptAfter: 50 });
  assertEquals(ctx.repl !== 0, true);
  const r = run(ctx, "main()");
  assertEquals(r.svgs.length > 0, true, "expected SVG frames from world.run");
  // "Interrupted." in stderr is expected when check_interrupt triggers
  destroy(ctx);
});

// --- Completion ---

Deno.test("repl_complete returns candidates", async () => {
  const ctx = await newRepl();
  // Define a function, then complete on its prefix
  run(ctx, "fn my_func() { 1 }");
  const input = "my_f";
  const [ptr, len] = encodeString(ctx.exports, input);
  const resultPtr = ctx.exports.repl_complete!(ctx.repl, ptr, len, len);
  ctx.exports.string_deallocate(ptr, len);
  assertEquals(resultPtr !== 0, true, "repl_complete should return non-null");
  const result = readCstr(ctx.exports, resultPtr);
  ctx.exports.cstr_deallocate(resultPtr);
  assertEquals(result.startsWith("c "), true, "should start with 'c '");
  assertEquals(
    result.includes("my_func"),
    true,
    `should include my_func, got: ${result}`,
  );
  destroy(ctx);
});

Deno.test("repl_complete returns null for no match", async () => {
  const ctx = await newRepl();
  const input = "zzz_no_match";
  const [ptr, len] = encodeString(ctx.exports, input);
  const resultPtr = ctx.exports.repl_complete!(ctx.repl, ptr, len, len);
  ctx.exports.string_deallocate(ptr, len);
  assertEquals(resultPtr, 0, "repl_complete should return null for no match");
  destroy(ctx);
});

Deno.test("repl_complete returns null for empty prefix", async () => {
  const ctx = await newRepl();
  const input = "";
  const [ptr, len] = encodeString(ctx.exports, input);
  const resultPtr = ctx.exports.repl_complete!(ctx.repl, ptr, len, 0);
  ctx.exports.string_deallocate(ptr, len);
  assertEquals(
    resultPtr,
    0,
    "repl_complete should return null for empty input",
  );
  destroy(ctx);
});

// --- Config ---

Deno.test("repl_new config bigint=true enables BigInt", async () => {
  const ctx = await newRepl("", { bigint: true });
  const r = run(ctx, "1 + 2");
  assertEquals(r.result, REPL_OK);
  assertEquals(r.stdout, "3\n");
  destroy(ctx);
});

Deno.test("repl_new config bigint=false disables BigInt", async () => {
  const ctx = await newRepl("", { bigint: false });
  const r = run(ctx, "1 + 2");
  assertEquals(r.result, REPL_OK);
  assertEquals(r.stdout, "3\n");
  destroy(ctx);
});

// --- Stop ---

Deno.test("repl_stop interrupts infinite recursion", async () => {
  const ctx = await newRepl("", { interruptAfter: 100 });
  const r = run(ctx, "fn f() { f() } f()");
  assertEquals(r.result, REPL_ERROR);
  assertEquals(
    r.stderr.includes("Interrupted"),
    true,
    `expected 'Interrupted' in stderr, got: ${r.stderr}`,
  );
  destroy(ctx);
});

// --- Stress ---

Deno.test("move_square survives many frames", async () => {
  const ctx = await newRepl(MOVE_SQUARE, { interruptAfter: 50_000 });
  assertEquals(ctx.repl !== 0, true);
  const r = run(ctx, "main()");
  assertEquals(
    r.svgs.length > 100,
    true,
    `expected many SVG frames, got ${r.svgs.length}`,
  );
  destroy(ctx);
});
