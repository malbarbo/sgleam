// Deno CLI wrapper around the sgleam WASM binary.
// Provides the same stdin/stdout interface as the native sgleam binary so the
// Rust integration tests in cli/tests/cli.rs can run both backends.

import { makeWasi } from "./tests/wasi.ts";

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
  format(ptr: number, len: number): number;
  cstr_deallocate(ptr: number): void;
  version(): number;
}

const REPL_OK = 0;
const REPL_ERROR = 1;
const REPL_QUIT = 2;

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

function writeOut(fd: number, text: string) {
  const bytes = encoder.encode(text);
  const out = fd === 2 ? Deno.stderr : Deno.stdout;
  let written = 0;
  while (written < bytes.length) {
    written += out.writeSync(bytes.subarray(written));
  }
}

async function readAllStdin(): Promise<string> {
  const chunks: Uint8Array[] = [];
  const reader = Deno.stdin.readable.getReader();
  try {
    while (true) {
      const { value, done } = await reader.read();
      if (done) break;
      chunks.push(value);
    }
  } finally {
    reader.releaseLock();
  }
  let total = 0;
  for (const c of chunks) total += c.length;
  const merged = new Uint8Array(total);
  let offset = 0;
  for (const c of chunks) {
    merged.set(c, offset);
    offset += c.length;
  }
  return decoder.decode(merged);
}

function wasmPath(): string {
  return new URL(
    "../target/wasm32-wasip1/release-small/sgleam.wasm",
    import.meta.url,
  ).pathname;
}

interface WasmCtx {
  exports: WasmExports;
  setStdinCursor(offset: number): void;
}

async function loadWasm(
  args: string[],
  stdinText: string,
): Promise<WasmCtx> {
  const bytes = await Deno.readFile(wasmPath());
  const module = await WebAssembly.compile(bytes);
  let exports: WasmExports;

  let stdinCursor = 0;

  const wasi = makeWasi({
    getBuffer: () => exports.memory.buffer,
    write: (fd, text) => writeOut(fd, text),
    readStdin: () => {
      const remaining = stdinText.slice(stdinCursor);
      stdinCursor = stdinText.length;
      return remaining;
    },
    args,
    env: [],
    isTty: false,
  });

  const env: WebAssembly.ModuleImports = {
    check_interrupt: (): number => 0,
    sleep: (_ms: bigint): void => {},
    draw_svg: (): void => {},
    get_key_event: (): number => 3, // EVENT_NONE
    text_width: (): number => 10,
    text_height: (): number => 16,
    text_x_offset: (): number => -5,
    text_y_offset: (): number => 12,
    load_bitmap_fetch: (): number => 0,
    load_bitmap_width: (): number => 0,
    load_bitmap_height: (): number => 0,
    load_bitmap_data: (): number => 0,
  };

  const instance = await WebAssembly.instantiate(module, {
    env,
    wasi_snapshot_preview1: wasi,
  });

  exports = instance.exports as unknown as WasmExports;
  return {
    exports,
    setStdinCursor: (offset: number) => {
      stdinCursor = offset;
    },
  };
}

interface Statement {
  text: string;
  endOffset: number;
}

// Splits piped stdin into statements with the same granularity as rustyline's
// continuation mode: accumulate lines until brackets/braces/parens are balanced
// (ignoring content inside string literals and line comments). `endOffset` is
// the position in the original input immediately after this statement's
// trailing newline, matching where rustyline would have consumed stdin up to.
function splitStatements(input: string): Statement[] {
  const statements: Statement[] = [];
  let depth = 0;
  let inString = false;
  let stringQuote = "";
  let stmtStart = 0;
  let i = 0;

  const emit = (end: number) => {
    const raw = input.slice(stmtStart, end);
    if (raw.trim().length > 0) {
      // Match rustyline: it returns one line (or a continuation block) without
      // the trailing newline; the REPL's line/column tracking uses this form.
      const text = raw.replace(/\n+$/, "");
      statements.push({ text, endOffset: end });
    }
    stmtStart = end;
    depth = 0;
  };

  while (i < input.length) {
    const c = input[i];
    if (inString) {
      if (c === "\\" && i + 1 < input.length) {
        i += 2;
        continue;
      }
      if (c === stringQuote) inString = false;
      i++;
      continue;
    }
    if (c === "/" && input[i + 1] === "/") {
      while (i < input.length && input[i] !== "\n") i++;
      continue;
    }
    if (c === '"') {
      inString = true;
      stringQuote = '"';
      i++;
      continue;
    }
    if (c === "{" || c === "(" || c === "[") {
      depth++;
      i++;
      continue;
    }
    if (c === "}" || c === ")" || c === "]") {
      depth--;
      i++;
      continue;
    }
    if (c === "\n") {
      i++;
      if (!inString && depth <= 0) emit(i);
      continue;
    }
    i++;
  }
  if (stmtStart < input.length) emit(input.length);
  return statements;
}

async function runFormat(): Promise<number> {
  const stdin = await readAllStdin();
  const ctx = await loadWasm(["sgleam", "format"], "");
  const { exports } = ctx;
  const [ptr, len] = encodeString(exports, stdin);
  const resultPtr = exports.format(ptr, len);
  exports.string_deallocate(ptr, len);
  if (resultPtr === 0) return 1;
  const formatted = readCstr(exports, resultPtr);
  exports.cstr_deallocate(resultPtr);
  writeOut(1, formatted);
  return 0;
}

function runReplStatements(
  ctx: WasmCtx,
  repl: number,
  input: string,
): number {
  const statements = splitStatements(input);
  let lastStatus = REPL_OK;
  for (const stmt of statements) {
    ctx.setStdinCursor(stmt.endOffset);
    const [ptr, len] = encodeString(ctx.exports, stmt.text);
    const status = ctx.exports.repl_run(repl, ptr, len);
    ctx.exports.string_deallocate(ptr, len);
    if (status === REPL_QUIT) {
      return REPL_QUIT;
    }
    if (status === REPL_ERROR) lastStatus = REPL_ERROR;
  }
  return lastStatus;
}

async function loadSource(file: string | null): Promise<string> {
  if (!file) return "";
  return await Deno.readTextFile(file);
}

async function runRepl(opts: {
  quiet: boolean;
  number: boolean;
  file: string | null;
}): Promise<number> {
  const args = ["sgleam", "repl"];
  if (opts.quiet) args.push("-q");
  if (opts.number) args.push("-n");
  if (opts.file) args.push(opts.file);

  const source = await loadSource(opts.file);
  const stdin = await readAllStdin();
  const ctx = await loadWasm(args, stdin);
  const { exports } = ctx;

  if (!opts.quiet) {
    const verPtr = exports.version();
    const ver = readCstr(exports, verPtr);
    exports.cstr_deallocate(verPtr);
    writeOut(
      1,
      `Welcome to ${ver}.\nType ctrl-d or ":quit" to exit.\n`,
    );
  }

  const config = opts.number ? "" : "bigint=true";
  const [codePtr, codeLen] = encodeString(exports, source);
  const [cfgPtr, cfgLen] = encodeString(exports, config);
  const repl = exports.repl_new(codePtr, codeLen, cfgPtr, cfgLen);
  exports.string_deallocate(codePtr, codeLen);
  exports.string_deallocate(cfgPtr, cfgLen);
  if (repl === 0) return 1;

  const status = runReplStatements(ctx, repl, stdin);
  exports.repl_destroy(repl);
  return status === REPL_ERROR ? 1 : 0;
}

async function runFile(opts: {
  number: boolean;
  file: string;
}): Promise<number> {
  const source = await loadSource(opts.file);
  const stdin = await readAllStdin();
  const ctx = await loadWasm(
    ["sgleam", "run", opts.file],
    stdin,
  );
  const { exports } = ctx;

  const config = opts.number ? "" : "bigint=true";
  const [codePtr, codeLen] = encodeString(exports, source);
  const [cfgPtr, cfgLen] = encodeString(exports, config);
  const repl = exports.repl_new(codePtr, codeLen, cfgPtr, cfgLen);
  exports.string_deallocate(codePtr, codeLen);
  exports.string_deallocate(cfgPtr, cfgLen);
  if (repl === 0) return 1;

  const [ptr, len] = encodeString(exports, "main()");
  const status = exports.repl_run(repl, ptr, len);
  exports.string_deallocate(ptr, len);
  exports.repl_destroy(repl);
  return status === REPL_ERROR ? 1 : 0;
}

interface ParsedArgs {
  command: "repl" | "format" | "run" | "welcome" | "unknown";
  quiet: boolean;
  number: boolean;
  file: string | null;
  unknown?: string;
}

function parseArgs(argv: string[]): ParsedArgs {
  if (argv.length === 0) {
    return { command: "welcome", quiet: false, number: false, file: null };
  }

  const first = argv[0];
  let command: ParsedArgs["command"];
  let rest: string[];
  if (first === "repl" || first === "format" || first === "run") {
    command = first;
    rest = argv.slice(1);
  } else if (first.startsWith("-")) {
    command = "unknown";
    rest = argv;
    return {
      command,
      quiet: false,
      number: false,
      file: null,
      unknown: first,
    };
  } else {
    // Treat bare file as `run FILE`
    command = "run";
    rest = argv;
  }

  let quiet = false;
  let number = false;
  let file: string | null = null;
  for (const arg of rest) {
    if (arg === "-q") quiet = true;
    else if (arg === "-n") number = true;
    else if (arg.startsWith("-")) {
      return {
        command: "unknown",
        quiet: false,
        number: false,
        file: null,
        unknown: arg,
      };
    } else if (file === null) file = arg;
    else {
      return {
        command: "unknown",
        quiet: false,
        number: false,
        file: null,
        unknown: arg,
      };
    }
  }

  return { command, quiet, number, file };
}

async function runWelcome(): Promise<number> {
  const { exports } = await loadWasm(["sgleam"], "");
  const verPtr = exports.version();
  const ver = readCstr(exports, verPtr);
  exports.cstr_deallocate(verPtr);
  writeOut(1, `Welcome to ${ver}.\nType ctrl-d or ":quit" to exit.\n`);
  return 0;
}

async function main(): Promise<number> {
  const parsed = parseArgs(Deno.args);
  switch (parsed.command) {
    case "welcome":
      return await runWelcome();
    case "format":
      return await runFormat();
    case "repl":
      return await runRepl({
        quiet: parsed.quiet,
        number: parsed.number,
        file: parsed.file,
      });
    case "run":
      if (!parsed.file) {
        writeOut(2, "error: `run` requires a FILE argument\n");
        return 1;
      }
      return await runFile({
        number: parsed.number,
        file: parsed.file,
      });
    case "unknown":
      writeOut(2, `error: unknown argument: ${parsed.unknown}\n`);
      return 1;
  }
}

Deno.exit(await main());
