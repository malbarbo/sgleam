let wasmModule;
let wasmExports;
let repl;
let stopBuffer;

async function loadAndInstantiateWasm() {
    wasmModule = await WebAssembly.compileStreaming(fetch('sgleam.wasm'));
    await instantiateWasm();
}

async function instantiateWasm() {
    var count = 0;
    const __WASI_ESUCCESS = 0;
    const __WASI_EINVAL = 28;
    const __WASI_ENOSYS = 52;
    const __WASI_EBADF = 8;
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();
    const args = [];
    const env = ['RUST_BACKTRACE=1'];
    const instance = await WebAssembly.instantiate(wasmModule, {
        env: {
            import_check_interrupt() {
                return Atomics.exchange(stopBuffer, 0);
            }
        },
        wasi_snapshot_preview1: {
            clock_time_get(clockId, _precision, timePtr) {
                try {
                    const dataView = new DataView(instance.exports.memory.buffer);
                    let timestamp;
                    switch (clockId) {
                        case 0: // __WASI_CLOCK_REALTIME
                            timestamp = BigInt(Date.now()) * 1_000_000n;
                            break;
                        case 1:
                        case 2:
                        case 3:
                            if (typeof performance !== 'undefined' && typeof performance.now === 'function') {
                                timestamp = BigInt(Math.round(performance.now() * 1_000_000));
                            } else {
                                timestamp = BigInt(Date.now()) * 1_000_000n;
                            }
                            break;
                        default:
                            return __WASI_EINVAL;
                    }

                    dataView.setBigUint64(timePtr, timestamp, true);

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('clock_time_get failed:', error);
                    return __WASI_ENOSYS;
                }
            },
            environ_get(environPtr, environBufPtr) {
                try {
                    const dataView = new DataView(instance.exports.memory.buffer);
                    const envPtrs = [];

                    let currentBufPtr = environBufPtr;
                    for (const envVar of env) {
                        envPtrs.push(currentBufPtr);
                        // FIXME: what about encoding?
                        for (let i = 0; i < envVar.length; i++) {
                            dataView.setUint8(currentBufPtr++, envVar.charCodeAt(i));
                        }
                        dataView.setUint8(currentBufPtr++, 0); // null terminator
                    }

                    for (let i = 0; i < envPtrs.length; i++) {
                        dataView.setInt32(environPtr + i * 4, envPtrs[i], true);
                    }
                    dataView.setInt32(environPtr + envPtrs.length * 4, 0, true); // array terminator.

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('environ_get failed:', error);
                    return __WASI_ENOSYS;
                }
            },
            environ_sizes_get(environCountPtr, environBufSizePtr) {
                try {
                    const dataView = new DataView(instance.exports.memory.buffer);

                    let environBufSize = 0;
                    for (const envVar of env) {
                        environBufSize += envVar.length + 1;
                    }

                    dataView.setInt32(environCountPtr, env.length, true);
                    dataView.setInt32(environBufSizePtr, environBufSize, true);

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('environ_sizes_get failed:', error);
                    return __WASI_ENOSYS;
                }
            },
            proc_exit() {
                console.trace(count++, arguments);
                return 0;
            },
            fd_write(fd, iovsPtr, iovsLen, nwrittenPtr) {
                if (fd !== 1 && fd !== 2) {
                    console.error('fd_write: unsupported file descriptor:', fd);
                    return __WASI_EBADF;
                }

                try {
                    const memory = instance.exports.memory;
                    const dataView = new DataView(memory.buffer);

                    let totalBytesWritten = 0;
                    for (let i = 0; i < iovsLen; i++) {
                        const iovPtr = iovsPtr + i * 8; // iovec structure is 8 bytes (ptr + len).
                        const bufPtr = dataView.getInt32(iovPtr, true);
                        const bufLen = dataView.getInt32(iovPtr + 4, true);
                        const buf = new Uint8Array(memory.buffer, bufPtr, bufLen);

                        postOutput(decoder.decode(buf));

                        totalBytesWritten += bufLen;
                    }

                    dataView.setInt32(nwrittenPtr, totalBytesWritten, true);

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('fd_write failed:', error);
                    return __WASI_ENOSYS;
                }
            },
            fd_seek() {
                console.trace(count++, arguments);
                return 0;
            },
            fd_read() {
                console.trace(count++, arguments);
                return 0;
            },
            fd_close() {
                console.trace(count++, arguments);
                return 0;
            },
            fd_fdstat_get() {
                // called by is_terminal
                return 0;
            },
            args_sizes_get(argc_ptr, argv_buf_size_ptr) {
                try {
                    let argv_buf_size = 0;
                    for (const arg of args) {
                        argv_buf_size += arg.length + 1;
                    }
                    const dataView = new DataView(instance.exports.memory.buffer);
                    dataView.setInt32(argc_ptr, args.length, true);
                    dataView.setInt32(argv_buf_size_ptr, argv_buf_size, true);
                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('args_sizes_get:', error);
                    return __WASI_ENOSYS;
                }
            },
            args_get(argv_ptr, argv_buf) {
                try {
                    let offset = 0;
                    const argPointers = [];
                    const byteView = new Uint8Array(instance.exports.memory.buffer);
                    for (const arg of simulatedArgs) {
                        const encodedArg = encoder.encode(arg);
                        argPointers.push(argv_buf + offset);
                        byteView.set(encodedArg, argv_buf + offset);
                        offset += encodedArg.length;
                        byteView[argv_buf + offset] = 0; // Null terminator
                        offset++;
                    }

                    const dataView = new DataView(instance.exports.memory.buffer);
                    for (let i = 0; i < argPointers.length; i++) {
                        dataView.setInt32(argv_ptr + i * 4, argPointers[i], true);
                    }
                    dataView.setInt32(argv_ptr + simulatedArgs.length * 4, 0, true);

                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('args_sizes_get:', error);
                    return __WASI_ENOSYS;
                }
            },
            random_get(buf, len) {
                try {
                    const buffer = new Uint8Array(instance.exports.memory.buffer, buf, len);
                    for (let i = 0; i < buffer.length; i++) {
                        buffer[i] = Math.floor(Math.random() * 256);
                    }
                    return __WASI_ESUCCESS;
                } catch (error) {
                    console.error('random_get:', error);
                    return __WASI_ENOSYS;
                }
            },
        }
    });
    wasmExports = instance.exports;
    wasmExports.use_bigint(true);
    self.onmessage = processMsg;
    initRepl('');
}

function postOutput(data) {
    self.postMessage({ cmd: 'output', data: data });
}

function postReady() {
    self.postMessage({ cmd: 'ready' });
}

function processMsg(event) {
    const data = event.data;
    if (data.cmd == 'init') {
        stopBuffer = new Int32Array(data.data);
    } else if (data.cmd == 'run') {
        runRepl(data.data);
    } else if (data.cmd == 'load') {
        initRepl(data.data);
    } else if (data.cmd == 'stop') {
        wasmExports.repl_stop();
    } else {
        console.log(`${event}`);
    }
}

function initRepl(input) {
    if (repl) {
        wasmExports.repl_destroy(repl);
        repl = null;
    }
    const [ptr, len] = createString(input);
    repl = wasmExports.repl_new(ptr, len);
    wasmExports.string_deallocate(ptr);
    postReady();
}

async function runRepl(input) {
    const [ptr, len] = createString(input);
    try {
        if (wasmExports.repl_run(repl, ptr, len)) {
            // :quit
            postOutput('Reloading the repl.');
            initRepl("");
        } else {
            postReady();
        }
    } catch (err) {
        postOutput('Execution error (probably a stackoverflow). Reloading the repl.');
        repl = null;
        await instantiateWasm();
    } finally {
        wasmExports.string_deallocate(ptr);
    }
}

function createString(str) {
    const encoded = new TextEncoder().encode(str);
    const ptr = wasmExports.string_allocate(encoded.length);
    const bytes = new Uint8Array(wasmExports.memory.buffer, ptr, encoded.length);
    bytes.set(encoded)
    return [ptr, encoded.length];
}

loadAndInstantiateWasm();