import { WorkerChannel } from "./worker_channel.ts";

// --- Types ---

interface WasmExports {
    memory: WebAssembly.Memory;
    repl_new(ptr: number, len: number): number;
    repl_run(repl: number, ptr: number, len: number): boolean;
    repl_destroy(repl: number): void;
    repl_stop(): void;
    string_allocate(size: number): number;
    string_deallocate(ptr: number): void;
    format(ptr: number, len: number): number;
    cstr_deallocate(ptr: number): void;
    use_bigint?(flag: boolean): void;
}

// --- Constants ---

const STDOUT = 1;
const STDERR = 2;
const IS_DENO = "Deno" in globalThis;

const WASI_ESUCCESS = 0;
const WASI_EINVAL = 28;
const WASI_ENOSYS = 52;
const WASI_EBADF = 8;

// --- Repl session ---

class ReplSession {
    private readonly exports: WasmExports;
    private readonly ptr: number;

    constructor(exports: WasmExports, input: string) {
        this.exports = exports;
        const [ptr, len] = encodeString(exports, input);
        this.ptr = exports.repl_new(ptr, len);
        exports.string_deallocate(ptr);
    }

    // Returns true if the user typed :quit.
    run(input: string): boolean {
        const [ptr, len] = encodeString(this.exports, input);
        try {
            return this.exports.repl_run(this.ptr, ptr, len);
        } finally {
            this.exports.string_deallocate(ptr);
        }
    }

    stop(): void {
        this.exports.repl_stop();
    }

    destroy(): void {
        this.exports.repl_destroy(this.ptr);
    }
}

// --- Worker ---

class Worker {
    private wasmModule!: WebAssembly.Module;
    private exports!: WasmExports;
    private session: ReplSession | null = null;
    private channel = new WorkerChannel();

    constructor() {
        this.loadWasm();
    }

    private getBuffer(): ArrayBuffer {
        return this.exports.memory.buffer;
    }

    private makeSgleamEnv() {
        return {
            sgleam_check_interrupt: (): number => {
                return this.channel.checkInterrupt() ? 1 : 0;
            },
            sgleam_sleep: (ms: bigint): void => {
                this.channel.sleep(ms);
            },
            sgleam_draw_svg: (ptr: number, len: number): void => {
                const buf = new Uint8Array(this.getBuffer());
                this.channel.svg(
                    new TextDecoder().decode(buf.slice(ptr, ptr + len)),
                );
            },
            sgleam_get_key_event: (
                ptr: number,
                len: number,
                mods: number,
            ): number => {
                const event = this.channel.dequeueKeyEvent();
                if (event === null) {
                    return 3;
                }
                const buf = new Uint8Array(this.getBuffer());
                const encoded = new TextEncoder().encode(event.key);
                buf.set(encoded.subarray(0, len), ptr);
                buf.fill(0, ptr + encoded.length, ptr + len);
                buf[mods + 0] = event.alt ? 1 : 0;
                buf[mods + 1] = event.ctrl ? 1 : 0;
                buf[mods + 2] = event.shift ? 1 : 0;
                buf[mods + 3] = event.meta ? 1 : 0;
                buf[mods + 4] = event.repeat ? 1 : 0;
                return event.type;
            },
            sgleam_text_height: (
                text: number,
                textLen: number,
                font: number,
                fontLen: number,
                size: number,
            ): number => {
                if (IS_DENO) {
                    return fontLen;
                }
                const buf = new Uint8Array(this.getBuffer());
                const jtext = new TextDecoder().decode(
                    buf.slice(text, text + textLen),
                );
                const jfont = new TextDecoder().decode(
                    buf.slice(font, font + fontLen),
                );
                // deno-lint-ignore no-undef
                const offscreen = new OffscreenCanvas(1, 1);
                const ctx = offscreen.getContext("2d")!;
                ctx.font = `${size}px ${jfont}`;
                const metrics = ctx.measureText(jtext);
                // TODO: why actual doesnt work?
                return metrics.fontBoundingBoxAscent +
                    metrics.fontBoundingBoxDescent;
            },
            sgleam_text_width: (
                text: number,
                textLen: number,
                font: number,
                fontLen: number,
                size: number,
            ): number => {
                if (IS_DENO) {
                    return 0.6 * fontLen * textLen;
                }
                const buf = new Uint8Array(this.getBuffer());
                const jtext = new TextDecoder().decode(
                    buf.slice(text, text + textLen),
                );
                const jfont = new TextDecoder().decode(
                    buf.slice(font, font + fontLen),
                );
                // deno-lint-ignore no-undef
                const offscreen = new OffscreenCanvas(1, 1);
                const ctx = offscreen.getContext("2d")!;
                ctx.font = `${size}px ${jfont}`;
                return ctx.measureText(jtext).width;
            },
        };
    }

    private makeWasi(args: string[], env: string[]) {
        const decoder = new TextDecoder();
        const encoder = new TextEncoder();

        return {
            clock_time_get: (
                clockId: number,
                _precision: bigint,
                timePtr: number,
            ): number => {
                try {
                    const dataView = new DataView(this.getBuffer());
                    let timestamp: bigint;
                    switch (clockId) {
                        case 0: // CLOCK_REALTIME
                            timestamp = BigInt(Date.now()) * 1_000_000n;
                            break;
                        case 1:
                        case 2:
                        case 3:
                            if (
                                typeof performance !== "undefined" &&
                                typeof performance.now === "function"
                            ) {
                                timestamp = BigInt(
                                    Math.round(performance.now() * 1_000_000),
                                );
                            } else {
                                timestamp = BigInt(Date.now()) * 1_000_000n;
                            }
                            break;
                        default:
                            return WASI_EINVAL;
                    }
                    dataView.setBigUint64(timePtr, timestamp, true);
                    return WASI_ESUCCESS;
                } catch {
                    console.error("clock_time_get failed");
                    return WASI_ENOSYS;
                }
            },
            environ_get: (
                environPtr: number,
                environBufPtr: number,
            ): number => {
                try {
                    const dataView = new DataView(this.getBuffer());
                    const envPtrs: number[] = [];
                    let currentBufPtr = environBufPtr;
                    for (const envVar of env) {
                        envPtrs.push(currentBufPtr);
                        // FIXME: what about encoding?
                        for (let i = 0; i < envVar.length; i++) {
                            dataView.setUint8(
                                currentBufPtr++,
                                envVar.charCodeAt(i),
                            );
                        }
                        dataView.setUint8(currentBufPtr++, 0); // null terminator
                    }
                    for (let i = 0; i < envPtrs.length; i++) {
                        dataView.setInt32(environPtr + i * 4, envPtrs[i], true);
                    }
                    dataView.setInt32(environPtr + envPtrs.length * 4, 0, true); // array terminator
                    return WASI_ESUCCESS;
                } catch {
                    console.error("environ_get failed");
                    return WASI_ENOSYS;
                }
            },
            environ_sizes_get: (
                environCountPtr: number,
                environBufSizePtr: number,
            ): number => {
                try {
                    const dataView = new DataView(this.getBuffer());
                    let environBufSize = 0;
                    for (const envVar of env) {
                        environBufSize += envVar.length + 1;
                    }
                    dataView.setInt32(environCountPtr, env.length, true);
                    dataView.setInt32(environBufSizePtr, environBufSize, true);
                    return WASI_ESUCCESS;
                } catch {
                    console.error("environ_sizes_get failed");
                    return WASI_ENOSYS;
                }
            },
            proc_exit: (): number => {
                return 0;
            },
            fd_write: (
                fd: number,
                iovsPtr: number,
                iovsLen: number,
                nwrittenPtr: number,
            ): number => {
                if (fd !== STDOUT && fd !== STDERR) {
                    console.error("fd_write: unsupported file descriptor:", fd);
                    return WASI_EBADF;
                }
                try {
                    const dataView = new DataView(this.getBuffer());
                    let totalBytesWritten = 0;
                    for (let i = 0; i < iovsLen; i++) {
                        const iovPtr = iovsPtr + i * 8; // iovec is 8 bytes (ptr + len)
                        const bufPtr = dataView.getInt32(iovPtr, true);
                        const bufLen = dataView.getInt32(iovPtr + 4, true);
                        const buf = new Uint8Array(
                            this.getBuffer(),
                            bufPtr,
                            bufLen,
                        );
                        const text = decoder.decode(buf);
                        this.channel.write(fd, text);
                        totalBytesWritten += bufLen;
                    }
                    dataView.setInt32(nwrittenPtr, totalBytesWritten, true);
                    return WASI_ESUCCESS;
                } catch {
                    console.error("fd_write failed");
                    return WASI_ENOSYS;
                }
            },
            fd_seek: (): number => 0,
            fd_read: (): number => 0,
            fd_close: (): number => 0,
            fd_fdstat_get: (fd: number, statPtr: number): number => {
                if (fd === STDOUT || fd === STDERR) {
                    // Zero the entire fdstat struct (24 bytes) then set fs_filetype.
                    // isatty() checks fs_filetype == 2 AND (fs_rights_base[0] & 0x24) == 0,
                    // so uninitialized stack memory in fs_rights_base would make it return false.
                    const mem = new Uint8Array(this.getBuffer());
                    mem.fill(0, statPtr, statPtr + 24);
                    mem[statPtr] = 2; // WASI_FILETYPE_CHARACTER_DEVICE
                }
                return WASI_ESUCCESS;
            },
            args_sizes_get: (
                argcPtr: number,
                argvBufSizePtr: number,
            ): number => {
                try {
                    let argvBufSize = 0;
                    for (const arg of args) {
                        argvBufSize += arg.length + 1;
                    }
                    const dataView = new DataView(this.getBuffer());
                    dataView.setInt32(argcPtr, args.length, true);
                    dataView.setInt32(argvBufSizePtr, argvBufSize, true);
                    return WASI_ESUCCESS;
                } catch {
                    console.error("args_sizes_get failed");
                    return WASI_ENOSYS;
                }
            },
            args_get: (argvPtr: number, argvBuf: number): number => {
                try {
                    let offset = 0;
                    const argPointers: number[] = [];
                    const byteView = new Uint8Array(this.getBuffer());
                    for (const arg of args) {
                        const encodedArg = encoder.encode(arg);
                        argPointers.push(argvBuf + offset);
                        byteView.set(encodedArg, argvBuf + offset);
                        offset += encodedArg.length;
                        byteView[argvBuf + offset] = 0; // null terminator
                        offset++;
                    }
                    const dataView = new DataView(this.getBuffer());
                    for (let i = 0; i < argPointers.length; i++) {
                        dataView.setInt32(
                            argvPtr + i * 4,
                            argPointers[i],
                            true,
                        );
                    }
                    dataView.setInt32(argvPtr + args.length * 4, 0, true);
                    return WASI_ESUCCESS;
                } catch {
                    console.error("args_get failed");
                    return WASI_ENOSYS;
                }
            },
            random_get: (buf: number, len: number): number => {
                try {
                    const buffer = new Uint8Array(this.getBuffer(), buf, len);
                    for (let i = 0; i < buffer.length; i++) {
                        buffer[i] = Math.floor(Math.random() * 256);
                    }
                    return WASI_ESUCCESS;
                } catch {
                    console.error("random_get failed");
                    return WASI_ENOSYS;
                }
            },
            path_open: (): number => {
                console.error("path_open");
                return WASI_ENOSYS;
            },
            path_create_directory: (): number => {
                return WASI_ENOSYS;
            },
            path_filestat_get: (): number => {
                return WASI_ENOSYS;
            },
            path_readlink: (): number => {
                return WASI_ENOSYS;
            },
            fd_filestat_get: (): number => {
                console.error("fd_filestat_get");
                return WASI_ENOSYS;
            },
            fd_prestat_get: (): number => {
                console.error("fd_prestat_get");
                return WASI_ENOSYS;
            },
            fd_prestat_dir_name: (): number => {
                console.error("fd_prestat_dir_name");
                return WASI_ENOSYS;
            },
        };
    }

    private async loadWasm(): Promise<void> {
        try {
            const response = await fetch("sgleam.wasm");
            if (!response.ok) {
                this.channel.error(
                    `Error loading sgleam.wasm: ${response.status}`,
                );
                return;
            }
            const total = parseInt(
                response.headers.get("Content-Length") ?? "0",
            );
            const reader = response.body!.getReader();
            const chunks: Uint8Array[] = [];
            let loaded = 0;
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                chunks.push(value);
                loaded += value.length;
                if (total) {
                    this.channel.progress((loaded / total) * 100);
                }
            }
            const wasmBytes = new Uint8Array(loaded);
            let offset = 0;
            for (const chunk of chunks) {
                wasmBytes.set(chunk, offset);
                offset += chunk.length;
            }
            await this.instantiateWasm(
                await WebAssembly.compile(wasmBytes.buffer),
            );
        } catch (error) {
            this.channel.error(`${error}`);
        }
    }

    private async instantiateWasm(
        wasmModule: WebAssembly.Module,
    ): Promise<void> {
        this.wasmModule = wasmModule;
        const instance = await WebAssembly.instantiate(wasmModule, {
            env: this.makeSgleamEnv(),
            wasi_snapshot_preview1: this.makeWasi([], [
                "RUST_BACKTRACE=1",
            ]),
        });
        this.exports = instance.exports as unknown as WasmExports;
        this.exports.use_bigint?.(true);
        self.onmessage = (e) => this.processMsg(e);
        this.initRepl("");
    }

    processMsg(event: MessageEvent): void {
        const data = event.data;
        switch (data.cmd) {
            case "run":
                this.runRepl(data.data);
                break;
            case "format":
                this.formatRepl(data.data);
                break;
            case "load":
                this.initRepl(data.data);
                break;
            case "stop":
                this.session?.stop();
                break;
            default:
                console.log(`${event}`);
        }
    }

    initRepl(input: string): void {
        this.session?.destroy();
        this.session = new ReplSession(this.exports, input);
        this.channel.ready();
    }

    async runRepl(input: string): Promise<void> {
        try {
            if (this.session!.run(input)) {
                // :quit
                this.channel.write(STDOUT, "Reloading the repl.");
                this.initRepl("");
            } else {
                this.channel.ready();
            }
        } catch (err) {
            console.log(err);
            this.channel.write(
                STDERR,
                "Execution error (probably a stackoverflow). Reloading the repl.",
            );
            this.session = null;
            await this.instantiateWasm(this.wasmModule);
        }
    }

    formatRepl(input: string): void {
        const [ptr, len] = encodeString(this.exports, input);
        const r = this.exports.format(ptr, len);
        this.exports.string_deallocate(ptr);
        if (r !== 0) {
            this.channel.formatted(readCstr(this.exports, r));
            this.exports.cstr_deallocate(r);
        } else {
            this.channel.ready();
        }
    }
}

// --- Memory helpers ---

function encodeString(exports: WasmExports, str: string): [number, number] {
    const encoded = new TextEncoder().encode(str);
    const ptr = exports.string_allocate(encoded.length);
    new Uint8Array(exports.memory.buffer, ptr, encoded.length).set(encoded);
    return [ptr, encoded.length];
}

function readCstr(exports: WasmExports, ptr: number): string {
    const buffer = new Uint8Array(exports.memory.buffer);
    let end = ptr;
    while (buffer[end] !== 0) end++;
    return new TextDecoder().decode(buffer.slice(ptr, end));
}

// --- Init ---

new Worker();
