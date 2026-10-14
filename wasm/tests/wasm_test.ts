import { assertEquals, assertMatch } from "jsr:@std/assert";
import { FakeRoom, type Step } from "./room.ts";
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
  run(
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
  repl_ready?(
    repl: number,
    ptr: number,
    len: number,
    cursor: number,
  ): number;
  string_allocate(size: number): number;
  string_deallocate(ptr: number, size: number): void;
  format(ptr: number, len: number): number;
  cstr_deallocate(ptr: number): void;
  version(): number;
  font_face(i: number): number;
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

function makeEnv(
  getBuffer: () => ArrayBufferLike,
  svgs: string[],
  interruptAfter = Infinity,
): WebAssembly.ModuleImports {
  let interruptCount = 0;

  return {
    check_interrupt: (): number => {
      interruptCount++;
      return interruptCount >= interruptAfter ? 1 : 0;
    },
    draw_svg: (ptr: number, len: number): void => {
      const b = new Uint8Array(getBuffer() as ArrayBuffer);
      svgs.push(decoder.decode(b.slice(ptr, ptr + len)));
    },
  };
}

// --- WASM loader ---

interface WasmContext {
  exports: WasmExports;
  stdout: string[];
  stderr: string[];
  svgs: string[];
  // The frames of the worlds, which go to the room of a session.
  room: FakeRoom;
}

interface LoadOptions {
  bigint?: boolean;
  interruptAfter?: number;
  // What the player does in a world.
  steps?: Step[];
  nickname?: string;
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

  // The engine reads the time through WASI, and a wait moves it on, so a
  // world runs its ticks without burning wall-clock time.
  const room = new FakeRoom(options.steps, options.nickname);

  const wasi = makeWasi({
    getBuffer: () => exports.memory.buffer,
    write: (fd, text) => {
      if (fd === 2) stderr.push(text);
      else stdout.push(text);
    },
    env: ["RUST_BACKTRACE=1"],
    now: () => room.ms,
    advance: (until) => room.advance(until),
    session: room.pipe,
  });

  const env = makeEnv(
    () => exports.memory.buffer,
    svgs,
    options.interruptAfter,
  );

  const instance = await WebAssembly.instantiate(module, {
    env,
    wasi_snapshot_preview1: wasi,
  });

  exports = instance.exports as unknown as WasmExports;

  return { exports, stdout, stderr, svgs, room };
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
): {
  result: number;
  stdout: string;
  stderr: string;
  svgs: string[];
  frames: Uint8Array[];
} {
  ctx.stdout.length = 0;
  ctx.stderr.length = 0;
  ctx.svgs.length = 0;
  ctx.room.frames.length = 0;
  const [ptr, len] = encodeString(ctx.exports, input);
  const result = ctx.exports.repl_run(ctx.repl, ptr, len);
  ctx.exports.string_deallocate(ptr, len);
  return {
    result,
    stdout: ctx.stdout.join(""),
    stderr: ctx.stderr.join(""),
    svgs: [...ctx.svgs],
    frames: [...ctx.room.frames],
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

// A world with a handler for every kind of key event but keydown. What it
// prints says which handler ran, and for which key.
const KEY_HANDLERS = `import sgleam/image
import sgleam/stroke
import sgleam/world

pub fn main() {
  world.create(0, draw)
  |> world.on_key_press(handler("press"))
  |> world.on_key_up(handler("up"))
  |> world.stop_when(fn(handled) { handled == 2 })
  |> world.run()
}

pub fn draw(_handled: Int) -> image.Image {
  image.circle(20, stroke.red)
}

fn handler(name: String) -> fn(Int, world.Key) -> Int {
  fn(handled, key) {
    echo key as name
    handled + 1
  }
}
`;

// Every name `world` knows, in the order it lists them.
const KEY_NAMES = [
  "ArrowLeft",
  "ArrowRight",
  "ArrowUp",
  "ArrowDown",
  "PageUp",
  "PageDown",
  "Home",
  "End",
  "Backspace",
  "Tab",
  "Enter",
  "Escape",
  "Delete",
  "Insert",
  "F1",
  "F2",
  "F3",
  "F4",
  "F5",
  "F6",
  "F7",
  "F8",
  "F9",
  "F10",
  "F11",
  "F12",
  "CapsLock",
  "NumLock",
  "ScrollLock",
  "PrintScreen",
  "Pause",
  "Shift",
  "Control",
  "Alt",
  "Meta",
];

// A world that prints the key of every keydown it is given, and stops once it
// has seen `count` of them.
function keyNamesProgram(count: number): string {
  return `import sgleam/image
import sgleam/stroke
import sgleam/world

pub fn main() {
  world.create(0, draw)
  |> world.on_key_down(fn(seen, key) {
    echo key
    seen + 1
  })
  |> world.stop_when(fn(seen) { seen == ${count} })
  |> world.run()
}

pub fn draw(_seen: Int) -> image.Image {
  image.circle(20, stroke.red)
}
`;
}

const ANIMATE = `import sgleam/image
import sgleam/stroke
import sgleam/world

pub fn main() {
  world.animate(fn(frame) {
    echo frame as "frame"
    image.circle(10 + frame, stroke.red)
  })
}
`;

const MULTIPLAYER_COUNTER = `import sgleam/fill
import sgleam/image
import sgleam/mworld

pub fn main() {
  mworld.create(0, min: 1, max: 1, to_image: draw)
  |> mworld.on_pad_down(fn(count, player, input) {
    echo #(mworld.player_index(player), mworld.player_nickname(player), input)
    count + 1
  })
  |> mworld.stop_when(fn(count) { count == 1 })
  |> mworld.run()
}

fn draw(count: Int, player: mworld.Player) -> image.Image {
  echo #(mworld.player_index(player), mworld.player_nickname(player), count)
  image.square(20 + count, fill.red)
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

Deno.test("font_face gives the faces of the frames", async () => {
  const { exports } = await loadWasm();
  const families = new Set<string>();
  let i = 0;
  for (let ptr; (ptr = exports.font_face(i)) !== 0; i++) {
    const [bytes, bytesLen, family, familyLen, weight] = new Uint32Array(
      exports.memory.buffer,
      ptr,
      6,
    );
    // A TrueType file starts with version 1.0.
    assertEquals(
      [...new Uint8Array(exports.memory.buffer, bytes, 4)],
      [0, 1, 0, 0],
    );
    assertEquals(bytesLen > 0, true);
    assertEquals(weight === 400 || weight === 700, true);
    families.add(
      decoder.decode(new Uint8Array(exports.memory.buffer, family, familyLen)),
    );
  }
  // Four faces a family.
  assertEquals(i, families.size * 4);
  assertEquals(i > 0, true);
});

Deno.test("repl smoke test", async () => {
  const ctx = await newRepl();
  const r = run(ctx, "1 + 2");
  assertEquals(r.result, REPL_OK, `stderr:\n${r.stderr}\nstdout:\n${r.stdout}`);
  assertEquals(r.stdout, "3\n");
  destroy(ctx);
});

Deno.test("an import of a pasted input is in scope for its definitions", async () => {
  const ctx = await newRepl();
  const r = run(
    ctx,
    "import gleam/int\n\npub fn f(n) {\n  int.to_string(n)\n}\n\nf(5)\n",
  );
  assertEquals(r.result, REPL_OK, `stderr:\n${r.stderr}\nstdout:\n${r.stdout}`);
  assertEquals(r.stdout, '"5"\n');
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
  const r = run(
    ctx,
    "import sgleam/image\nimport sgleam/stroke\nimage.circle(30, stroke.red)",
  );
  assertEquals(r.result, REPL_OK);
  assertEquals(
    r.svgs.length > 0,
    true,
    `expected SVG, got none. stdout: ${r.stdout}`,
  );
  assertEquals(r.svgs[0].includes("<svg"), true, "expected SVG content");
  destroy(ctx);
});

Deno.test("repl draws an image with one more column and row", async () => {
  const ctx = await newRepl();
  const r = run(
    ctx,
    "import sgleam/image\nimport sgleam/stroke\nimage.rectangle(40, 20, stroke.black)",
  );
  assertEquals(r.result, REPL_OK);
  assertEquals(r.svgs[0].includes('width="41" height="21"'), true, r.svgs[0]);
  destroy(ctx);
});

Deno.test("wedge image produces SVG", async () => {
  const ctx = await newRepl();
  const r = run(
    ctx,
    "import sgleam/image\nimport sgleam/fill\nimage.wedge(40, 90, fill.red)",
  );
  assertEquals(r.result, REPL_OK);
  assertEquals(r.svgs.length > 0, true, "expected SVG output");
  destroy(ctx);
});

Deno.test("add_curve renders", async () => {
  const ctx = await newRepl();
  const r = run(
    ctx,
    "import sgleam/image\nimport sgleam/stroke\nimage.add_curve(image.rectangle(100, 100, stroke.black), 20, 20, 0, 0.333, 80, 80, 0, 0.333, stroke.red)",
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

Deno.test("world.run shows its frames in a session", async () => {
  const ctx = await newRepl(MOVE_SQUARE, { interruptAfter: 50 });
  assertEquals(ctx.repl !== 0, true);
  const r = run(ctx, "main()");
  assertEquals(r.frames.length > 0, true, "expected frames from world.run");
  assertEquals(r.svgs.length, 0, "a world draws no svg");
  // check_interrupt fires here, so stderr says "Interrupted."
  destroy(ctx);
});

Deno.test("mworld fixes the player and maps keys to pad input", async () => {
  const ctx = await newRepl(MULTIPLAYER_COUNTER, {
    interruptAfter: 200_000,
    nickname: "Ana",
    steps: [{ type: "down", key: "ArrowRight" }],
  });
  const r = run(ctx, "main()");
  assertEquals(r.result, REPL_OK, `stderr:\n${r.stderr}`);
  assertEquals(
    r.stdout.includes('#(1, "Ana", Right)'),
    true,
    `expected the fixed player and pad input in:\n${r.stdout}`,
  );
  assertEquals(
    r.stdout.includes('#(1, "Ana", 1)'),
    true,
    `expected the per-player final view in:\n${r.stdout}`,
  );
  assertEquals(r.frames.length, 2, `stderr:\n${r.stderr}`);
  destroy(ctx);
});

// The events a handler is not for go by untouched: this world has no
// on_key_down, so the keydown it is given reaches none of its handlers.
Deno.test("on_key_press and on_key_up take their own events", async () => {
  const ctx = await newRepl(KEY_HANDLERS, {
    interruptAfter: 200_000,
    steps: [
      { type: "down", key: "ArrowLeft" },
      { type: "press", key: "a" },
      { type: "up", key: "Enter" },
    ],
  });
  const r = run(ctx, "main()");
  // The last line is the repl echoing what main() gave back.
  assertEquals(
    r.stdout,
    'user.gleam:19 press\nChar("a")\nuser.gleam:19 up\nEnter\nNil\n',
  );
  // The first state and one per handled event.
  assertEquals(r.frames.length, 3, `stderr:\n${r.stderr}`);
  destroy(ctx);
});

Deno.test("a close ends world.run", async () => {
  const ctx = await newRepl(KEY_HANDLERS, {
    interruptAfter: 200_000,
    steps: [{ type: "press", key: "a" }, "close"],
  });
  const r = run(ctx, "main()");
  assertEquals(r.stdout, 'user.gleam:19 press\nChar("a")\nNil\n');
  destroy(ctx);
});

Deno.test("a close ends world.animate", async () => {
  const ctx = await newRepl(ANIMATE, {
    interruptAfter: 200_000,
    steps: [{ type: "press", key: "a" }, "close"],
  });
  const r = run(ctx, "main()");
  // The key goes unused, so the first frame is the only one.
  assertEquals(r.stdout, "user.gleam:7 frame\n0\nNil\n");
  assertEquals(r.frames.length, 1, `stderr:\n${r.stderr}`);
  destroy(ctx);
});

Deno.test("every key name decodes to its Key", async () => {
  const keys = [...KEY_NAMES, "a"];
  const ctx = await newRepl(keyNamesProgram(keys.length), {
    interruptAfter: 200_000,
    steps: keys.map((key) => ({ type: "down", key })),
  });
  const r = run(ctx, "main()");
  const expected = [...KEY_NAMES, 'Char("a")']
    .map((k) => `user.gleam:8\n${k}\n`)
    .join("") + "Nil\n";
  assertEquals(r.stdout, expected, `stderr:\n${r.stderr}`);
  destroy(ctx);
});

// animate has no stop of its own: it draws until it is interrupted.
Deno.test("animate draws a frame per number", async () => {
  const ctx = await newRepl(ANIMATE, { interruptAfter: 20 });
  const r = run(ctx, "main()");
  assertEquals(
    r.stdout.startsWith(
      [0, 1, 2].map((n) => `user.gleam:7 frame\n${n}\n`).join(""),
    ),
    true,
    `stdout:\n${r.stdout}`,
  );
  assertEquals(r.frames.length >= 3, true, `got ${r.frames.length} frames`);
  // The frame is what the image is drawn from, so no two frames are alike.
  assertEquals(
    r.frames[0].join() !== r.frames[1].join(),
    true,
    "expected the frames to differ",
  );
  destroy(ctx);
});

// --- Completion ---

Deno.test("repl_complete returns candidates", async () => {
  const ctx = await newRepl();
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

Deno.test("repl_complete counts in bytes, not chars", async () => {
  const ctx = await newRepl();
  run(ctx, "fn my_func() { 1 }");
  // The break char before the word takes three bytes, and the one before
  // that takes two, so a `start` counted in chars would land inside them.
  const input = 'let x = "ol\u00e1\u2026my_f';
  const [ptr, len] = encodeString(ctx.exports, input);
  const resultPtr = ctx.exports.repl_complete!(ctx.repl, ptr, len, len);
  ctx.exports.string_deallocate(ptr, len);
  assertEquals(resultPtr !== 0, true, "repl_complete should return non-null");
  const result = readCstr(ctx.exports, resultPtr);
  ctx.exports.cstr_deallocate(resultPtr);
  assertEquals(
    result.startsWith("c 16\n"),
    true,
    `word should start at byte 16, got: ${result}`,
  );
  assertEquals(
    result.includes("my_func"),
    true,
    `should include my_func, got: ${result}`,
  );
  destroy(ctx);
});

Deno.test("repl_complete keeps the space a candidate ends with", async () => {
  const ctx = await newRepl();
  // The completion offers `:type` with the space that goes after it, and a
  // line is what tells one candidate from the next, so the space arrives.
  const input = ":ty";
  const [ptr, len] = encodeString(ctx.exports, input);
  const resultPtr = ctx.exports.repl_complete!(ctx.repl, ptr, len, len);
  ctx.exports.string_deallocate(ptr, len);
  assertEquals(resultPtr !== 0, true, "repl_complete should return non-null");
  const result = readCstr(ctx.exports, resultPtr);
  ctx.exports.cstr_deallocate(resultPtr);
  assertEquals(result, "c 0\n:type ", `got: ${result}`);
  destroy(ctx);
});

Deno.test("repl_complete survives a cursor inside a char", async () => {
  const ctx = await newRepl();
  // Byte 12 is the middle of the `\u00e1`, which is where a host counting in
  // UTF-16 puts the cursor at the end of this line.
  const input = 'let x = "ol\u00e1';
  const [ptr, len] = encodeString(ctx.exports, input);
  const resultPtr = ctx.exports.repl_complete!(ctx.repl, ptr, len, 12);
  ctx.exports.string_deallocate(ptr, len);
  if (resultPtr !== 0) ctx.exports.cstr_deallocate(resultPtr);
  // The module is still there to answer, which is the whole of the question.
  assertEquals(
    run(ctx, "1 + 1").result,
    0,
    "repl should still run after completing",
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

// --- Ready ---

Deno.test("repl_ready says what the input still needs", async () => {
  const ctx = await newRepl();
  const cases: [string, number][] = [
    // Finished: run it.
    ["1 + 2", -1],
    ["pub fn f() {\n  1\n}", -1],
    // Waiting on a value, with no bracket open.
    ["let x =", 0],
    // A level per bracket left open, whatever the bracket is.
    ["case x {", 2],
    ["io.println(", 2],
    ["let x = [1,", 2],
    ["pub fn f() {\n  case x {", 4],
    // A bracket the input only mentions opens nothing.
    ["1 + 1 // {", -1],
    // The way out of an input that will not close. With nothing open, a
    // blank line runs it and the error it earns is the engine's to give.
    ["let x =\n", -1],
    // With a bracket open it is a blank line inside the input -- how the
    // user writes a function with two statements.
    ["pub fn f() {\n  let x = 1\n", 2],
  ];
  for (const [input, expected] of cases) {
    const [ptr, len] = encodeString(ctx.exports, input);
    const result = ctx.exports.repl_ready!(ctx.repl, ptr, len, len);
    ctx.exports.string_deallocate(ptr, len);
    assertEquals(result, expected, JSON.stringify(input));
  }
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

// --- Interrupt ---

Deno.test("check_interrupt stops infinite recursion", async () => {
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
  // The world has no on_tick, so frames are produced only when key
  // handlers run. Feed ArrowRight events (move clamps, so the square
  // sits at the right edge after a few presses without triggering
  // stop_when). Each press renders, so N events => N frames.
  const steps = Array.from({ length: 150 }, (): Step => ({
    type: "down",
    key: "ArrowRight",
  }));
  const ctx = await newRepl(MOVE_SQUARE, {
    interruptAfter: 15_000,
    steps,
  });
  assertEquals(ctx.repl !== 0, true);
  const r = run(ctx, "main()");
  assertEquals(
    r.frames.length > 100,
    true,
    `expected many frames, got ${r.frames.length}`,
  );
  destroy(ctx);
});

Deno.test("panic handler intercepts Rust panics", async () => {
  const ctx = await newRepl(); // init is called
  try {
    // Calling string_deallocate with null pointer triggers a panic via assert!(!ptr.is_null())
    ctx.exports.string_deallocate(0, 10);
  } catch (e) {
    // The WASM module execution will abort on panic.
  }
  const stderr = ctx.stderr.join("");
  assertEquals(
    stderr.includes("Fatal compiler bug") ||
      stderr.includes("assertion failed"),
    true,
    `expected formatted panic message in stderr, got: ${stderr}`,
  );
  destroy(ctx);
});

// The cli reads a line at a time, so an input holding several items only
// reaches the repl through this api, which takes the whole of it at once.
Deno.test("pub on a line of its own", async () => {
  for (
    const input of [
      "pub\nfn f() { 1 }\nf()",
      "pub\ttype T {\n  A\n}\nA",
      "pub\nconst c = 1\nc",
      "@internal\npub fn g() { 2 }\ng()",
    ]
  ) {
    const ctx = await newRepl();
    const r = run(ctx, input);
    assertEquals(r.stderr, "", `${input} wrote to stderr`);
    assertEquals(r.stdout.length > 0, true, `${input} produced nothing`);
    destroy(ctx);
  }
});

Deno.test("run prints the String of smain as it is", async () => {
  const ctx = await loadWasm();
  const [codePtr, codeLen] = encodeString(
    ctx.exports,
    'pub fn smain() { "a\\nb" }',
  );
  const [cfgPtr, cfgLen] = encodeString(ctx.exports, "bigint=true");
  const result = ctx.exports.run(codePtr, codeLen, cfgPtr, cfgLen);
  assertEquals(result, 0);
  assertEquals(ctx.stdout.join(""), "a\nb\n");
});
