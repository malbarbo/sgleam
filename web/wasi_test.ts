import { assertEquals } from "jsr:@std/assert";
import { makeWasi } from "./wasi.ts";

const WASI_ESUCCESS = 0;
const encoder = new TextEncoder();
const decoder = new TextDecoder();

function createWasi(
    options: { env?: string[]; args?: string[] } = {},
): { wasi: ReturnType<typeof makeWasi>; mem: ArrayBuffer } {
    const mem = new ArrayBuffer(4096);
    const wasi = makeWasi({
        getBuffer: () => mem,
        write: () => {},
        env: options.env,
        args: options.args,
    });
    return { wasi, mem };
}

// Read a null-terminated UTF-8 string from memory at ptr.
function readCstr(mem: ArrayBuffer, ptr: number): string {
    const bytes = new Uint8Array(mem);
    let end = ptr;
    while (bytes[end] !== 0) end++;
    return decoder.decode(bytes.slice(ptr, end));
}

// --- environ_sizes_get ---

Deno.test("environ_sizes_get ascii", () => {
    const { wasi, mem } = createWasi({ env: ["A=1", "BB=22"] });
    const dv = new DataView(mem);
    assertEquals(wasi.environ_sizes_get(0, 4), WASI_ESUCCESS);
    assertEquals(dv.getInt32(0, true), 2); // count
    // "A=1" (3 bytes + null) + "BB=22" (5 bytes + null) = 10
    assertEquals(dv.getInt32(4, true), 10);
});

Deno.test("environ_sizes_get utf8 multibyte", () => {
    const { wasi, mem } = createWasi({ env: ["KEY=café"] });
    const dv = new DataView(mem);
    assertEquals(wasi.environ_sizes_get(0, 4), WASI_ESUCCESS);
    assertEquals(dv.getInt32(0, true), 1);
    // "KEY=café" is 10 UTF-8 bytes (é = 2 bytes) + 1 null = 11
    assertEquals(dv.getInt32(4, true), encoder.encode("KEY=café").length + 1);
});

// --- environ_get ---

Deno.test("environ_get ascii", () => {
    const { wasi, mem } = createWasi({ env: ["A=1", "BB=22"] });
    // Pointers area at offset 100, buffer area at offset 200.
    assertEquals(wasi.environ_get(100, 200), WASI_ESUCCESS);
    const dv = new DataView(mem);
    const ptr0 = dv.getInt32(100, true);
    const ptr1 = dv.getInt32(104, true);
    assertEquals(readCstr(mem, ptr0), "A=1");
    assertEquals(readCstr(mem, ptr1), "BB=22");
});

Deno.test("environ_get utf8 multibyte", () => {
    const { wasi, mem } = createWasi({ env: ["KEY=café"] });
    assertEquals(wasi.environ_get(100, 200), WASI_ESUCCESS);
    const dv = new DataView(mem);
    const ptr0 = dv.getInt32(100, true);
    assertEquals(readCstr(mem, ptr0), "KEY=café");
});

// --- args_sizes_get ---

Deno.test("args_sizes_get ascii", () => {
    const { wasi, mem } = createWasi({ args: ["hello", "world"] });
    const dv = new DataView(mem);
    assertEquals(wasi.args_sizes_get(0, 4), WASI_ESUCCESS);
    assertEquals(dv.getInt32(0, true), 2);
    // "hello" (5+1) + "world" (5+1) = 12
    assertEquals(dv.getInt32(4, true), 12);
});

Deno.test("args_sizes_get utf8 multibyte", () => {
    const { wasi, mem } = createWasi({ args: ["café"] });
    const dv = new DataView(mem);
    assertEquals(wasi.args_sizes_get(0, 4), WASI_ESUCCESS);
    assertEquals(dv.getInt32(0, true), 1);
    // "café" is 5 UTF-8 bytes + 1 null = 6
    assertEquals(dv.getInt32(4, true), encoder.encode("café").length + 1);
});

// --- args_get ---

Deno.test("args_get utf8 multibyte", () => {
    const { wasi, mem } = createWasi({ args: ["café"] });
    assertEquals(wasi.args_get(100, 200), WASI_ESUCCESS);
    const dv = new DataView(mem);
    const ptr0 = dv.getInt32(100, true);
    assertEquals(readCstr(mem, ptr0), "café");
});
